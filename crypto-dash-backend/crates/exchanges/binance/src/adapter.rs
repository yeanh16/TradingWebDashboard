use crate::types::{BinanceOrderBook, BinanceStreamMessage, BinanceTicker};

use anyhow::{anyhow, Result};

use async_trait::async_trait;

use crypto_dash_cache::CacheHandle;

use crypto_dash_core::{
    model::{
        Channel, ChannelType, ExchangeId, MarketType, OrderBookSnapshot, PriceLevel, StreamMessage,
        Symbol, Ticker,
    },
    normalize::SymbolMapper,
    time::{from_millis, now, to_millis},
};

use crypto_dash_exchanges_common::{ExchangeAdapter, WsClient};

use crypto_dash_stream_hub::{HubHandle, Topic};

use rust_decimal::Decimal;

use std::collections::HashMap;
use std::str::FromStr;

use std::sync::Arc;

use tokio::sync::Mutex;

use tokio_tungstenite::tungstenite::Message;

use tracing::{debug, error, info, warn};

const BINANCE_SPOT_WS_URL: &str = "wss://stream.binance.com:9443/ws";
const BINANCE_PERP_WS_URL: &str = "wss://fstream.binance.com/ws";
const SUPPORTED_MARKETS: [MarketType; 2] = [MarketType::Spot, MarketType::Perpetual];

#[derive(Clone)]
pub struct BinanceAdapter {
    hub: Arc<Mutex<Option<HubHandle>>>,
    cache: Arc<Mutex<Option<CacheHandle>>>,
    ws_clients: Arc<Mutex<HashMap<MarketType, Option<Arc<WsClient>>>>>,
    symbol_mapper: SymbolMapper,
    // no mock generators or mock flags - production behavior only
}

impl BinanceAdapter {
    pub fn new() -> Self {
        let mut ws_clients = HashMap::new();
        for market in SUPPORTED_MARKETS {
            ws_clients.insert(market, None);
            // nothing to insert for mocks
        }

        Self {
            hub: Arc::new(Mutex::new(None)),
            cache: Arc::new(Mutex::new(None)),
            ws_clients: Arc::new(Mutex::new(ws_clients)),
            symbol_mapper: SymbolMapper::default(),
            // no mock state
        }
    }

    fn market_label(market_type: MarketType) -> &'static str {
        match market_type {
            MarketType::Spot => "spot",
            MarketType::Perpetual => "perpetual",
        }
    }

    // Mocks removed; always return false if asked
    async fn mock_enabled(&self, _market_type: MarketType) -> bool {
        false
    }

    async fn get_ws_client(&self, market_type: MarketType) -> Option<Arc<WsClient>> {
        let guard = self.ws_clients.lock().await;
        guard
            .get(&market_type)
            .and_then(|client| client.as_ref().cloned())
    }

    async fn set_ws_client(&self, market_type: MarketType, client: Option<Arc<WsClient>>) {
        let mut guard = self.ws_clients.lock().await;
        if let Some(entry) = guard.get_mut(&market_type) {
            *entry = client;
        }
    }

    async fn get_mock_generator(&self, _market_type: MarketType) -> Option<()> {
        None
    }

    async fn set_mock_enabled(&self, _market_type: MarketType, _enabled: bool) {
        // no-op: mocks removed
    }

    async fn handle_message(
        &self,
        market_type: MarketType,
        message: BinanceStreamMessage,
    ) -> Result<()> {
        match message {
            BinanceStreamMessage::StreamTicker { stream: _, data } => {
                self.handle_ticker(market_type, data).await?;
            }

            BinanceStreamMessage::DirectTicker(data) => {
                self.handle_ticker(market_type, data).await?;
            }

            BinanceStreamMessage::OrderBook { stream, data } => {
                self.handle_orderbook(market_type, &stream, data).await?;
            }

            BinanceStreamMessage::Error { error, .. } => {
                error!("Binance error: {} - {}", error.code, error.msg);
            }
        }

        Ok(())
    }

    async fn disconnect_if_no_subscribers(&self, topic: &Topic) -> Result<()> {
        let should_disconnect = {
            let hub_guard = self.hub.lock().await;

            if let Some(hub) = hub_guard.as_ref() {
                hub.global_subscriber_count() == 0 && hub.subscriber_count(topic) == 0
            } else {
                false
            }
        };

        if should_disconnect {
            let market_type = topic.market_type;
            let mut ws_guard = self.ws_clients.lock().await;

            if let Some(entry) = ws_guard.get_mut(&market_type) {
                if let Some(client) = entry.take() {
                    info!(
                        market = Self::market_label(market_type),
                        "Binance market disconnected due to no subscribers"
                    );

                    client.close().await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_ticker(&self, market_type: MarketType, ticker: BinanceTicker) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.s)?;

        let event_millis = ticker
            .event_time
            .or(ticker.statistics_close_time)
            .unwrap_or_else(|| to_millis(now()));

        let timestamp = from_millis(event_millis)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", event_millis))?;

        let bid_size = ticker.best_bid_qty.as_deref().unwrap_or("0");

        let ask_size = ticker.best_ask_qty.as_deref().unwrap_or("0");

        let last_price = Decimal::from_str(ticker.c.as_deref().unwrap_or("0"))?;

        let bid_price = ticker
            .b
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(Decimal::from_str)
            .transpose()?
            .unwrap_or_else(|| last_price.clone());

        let ask_price = ticker
            .a
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(Decimal::from_str)
            .transpose()?
            .unwrap_or_else(|| last_price.clone());

        let normalized_ticker = Ticker {
            timestamp,

            exchange: self.id(),
            market_type,

            symbol: symbol.clone(),

            bid: bid_price,

            ask: ask_price,

            last: last_price.clone(),

            bid_size: Decimal::from_str(bid_size)?,

            ask_size: Decimal::from_str(ask_size)?,
        };

        if let Some(cache) = &*self.cache.lock().await {
            cache.set_ticker(normalized_ticker.clone()).await;
        }

        let topic = Topic::ticker(self.id(), market_type, symbol);

        if let Some(hub) = &*self.hub.lock().await {
            hub.publish(&topic, StreamMessage::Ticker(normalized_ticker))
                .await;
        }

        self.disconnect_if_no_subscribers(&topic).await?;

        Ok(())
    }

    async fn handle_orderbook(
        &self,
        market_type: MarketType,
        stream: &str,
        orderbook: BinanceOrderBook,
    ) -> Result<()> {
        // Extract symbol from stream name (e.g., "btcusdt@depth")

        let symbol_str = stream.split('@').next().unwrap_or(stream).to_uppercase();

        let symbol = self.parse_symbol(&symbol_str)?;

        let timestamp = crypto_dash_core::time::now();

        let mut bids = Vec::new();

        for bid in orderbook.bids {
            if bid.len() >= 2 {
                bids.push(PriceLevel::new(
                    Decimal::from_str(&bid[0])?,
                    Decimal::from_str(&bid[1])?,
                ));
            }
        }

        let mut asks = Vec::new();

        for ask in orderbook.asks {
            if ask.len() >= 2 {
                asks.push(PriceLevel::new(
                    Decimal::from_str(&ask[0])?,
                    Decimal::from_str(&ask[1])?,
                ));
            }
        }

        let normalized_orderbook = OrderBookSnapshot {
            timestamp,

            exchange: self.id(),

            market_type,

            symbol: symbol.clone(),

            bids,

            asks,

            checksum: None,
        };

        if let Some(cache) = &*self.cache.lock().await {
            cache.set_orderbook(normalized_orderbook.clone()).await;
        }

        let topic = Topic::orderbook(self.id(), market_type, symbol);

        if let Some(hub) = &*self.hub.lock().await {
            hub.publish(
                &topic,
                StreamMessage::OrderBookSnapshot(normalized_orderbook),
            )
            .await;
        }

        self.disconnect_if_no_subscribers(&topic).await?;

        Ok(())
    }

    fn parse_symbol(&self, binance_symbol: &str) -> Result<Symbol> {
        // Use the symbol mapper for production-ready symbol normalization
        if let Some(symbol) = self.symbol_mapper.to_canonical(&self.id(), binance_symbol) {
            return Ok(symbol);
        }

        // Fallback to simple parsing for unmapped symbols
        if binance_symbol.ends_with("USDT") {
            let base = &binance_symbol[..binance_symbol.len() - 4];
            Ok(Symbol::new(base, "USDT"))
        } else if binance_symbol.ends_with("USDC") {
            let base = &binance_symbol[..binance_symbol.len() - 4];
            Ok(Symbol::new(base, "USDC"))
        } else if binance_symbol.ends_with("TUSD") {
            let base = &binance_symbol[..binance_symbol.len() - 4];
            Ok(Symbol::new(base, "TUSD"))
        } else if binance_symbol.ends_with("BTC") {
            let base = &binance_symbol[..binance_symbol.len() - 3];
            Ok(Symbol::new(base, "BTC"))
        } else if binance_symbol.ends_with("ETH") {
            let base = &binance_symbol[..binance_symbol.len() - 3];
            Ok(Symbol::new(base, "ETH"))
        } else {
            Err(anyhow!("Unsupported symbol format: {}", binance_symbol))
        }
    }

    fn streams_from_channels(&self, channels: &[Channel]) -> Vec<String> {
        let mut streams = Vec::new();

        for channel in channels {
            let symbol_str = format!(
                "{}{}",
                channel.symbol.base.to_lowercase(),
                channel.symbol.quote.to_lowercase()
            );

            match channel.channel_type {
                ChannelType::Ticker => {
                    streams.push(format!("{}@ticker", symbol_str));
                }

                ChannelType::OrderBook => {
                    let depth = channel.depth.unwrap_or(20);

                    streams.push(format!("{}@depth{}", symbol_str, depth));
                }
            }
        }

        streams
    }

    fn format_subscription(&self, channels: &[Channel]) -> Result<String> {
        let streams = self.streams_from_channels(channels);

        let subscription = serde_json::json!({

            "method": "SUBSCRIBE",

            "params": streams,

            "id": 1

        });

        Ok(subscription.to_string())
    }

    fn format_unsubscription(&self, channels: &[Channel]) -> Result<String> {
        let streams = self.streams_from_channels(channels);

        let unsubscription = serde_json::json!({

            "method": "UNSUBSCRIBE",

            "params": streams,

            "id": 1

        });

        Ok(unsubscription.to_string())
    }

    async fn listen_for_messages(
        &self,
        market_type: MarketType,
        ws_client: Arc<WsClient>,
    ) -> Result<()> {
        loop {
            let message = match ws_client.next_message().await? {
                Some(Message::Text(text)) => text,

                Some(Message::Close(_)) => {
                    warn!("Binance WebSocket connection closed");

                    break;
                }

                Some(_) => continue,

                None => {
                    warn!("Binance WebSocket stream ended");

                    break;
                }
            };

            match serde_json::from_str::<BinanceStreamMessage>(&message) {
                Ok(stream_message) => {
                    if let Err(e) = self.handle_message(market_type, stream_message).await {
                        error!("Failed to handle Binance message: {}", e);
                    }
                }

                Err(e) => {
                    debug!("Failed to parse Binance message: {} - Raw: {}", e, message);
                }
            }
        }

        let mut ws_guard = self.ws_clients.lock().await;

        if let Some(entry) = ws_guard.get_mut(&market_type) {
            if let Some(current) = entry.as_ref() {
                if Arc::ptr_eq(current, &ws_client) {
                    *entry = None;
                }
            }
        }

        Ok(())
    }

    async fn try_real_connection(&self, market_type: MarketType) -> Result<Arc<WsClient>> {
        let ws_url = match market_type {
            MarketType::Spot => BINANCE_SPOT_WS_URL,
            MarketType::Perpetual => BINANCE_PERP_WS_URL,
        };

        debug!(
            market = Self::market_label(market_type),
            "Attempting to connect to Binance WebSocket: {}", ws_url
        );

        let ws_client = Arc::new(WsClient::new(ws_url));

        ws_client.connect().await?;

        debug!(
            market = Self::market_label(market_type),
            "Binance WebSocket handshake successful"
        );

        self.set_ws_client(market_type, Some(ws_client.clone()))
            .await;

        let adapter = self.clone();
        let listener_client = ws_client.clone();
        let listener_market = market_type;

        tokio::spawn(async move {
            if let Err(e) = adapter
                .listen_for_messages(listener_market, listener_client)
                .await
            {
                error!(
                    market = BinanceAdapter::market_label(listener_market),
                    "Binance WebSocket listener error: {}", e
                );
            }
        });

        Ok(ws_client)
    }

    async fn start_mock_data(&self, _market_type: MarketType, _hub: HubHandle) -> Result<()> {
        // Mocks removed; nothing to do
        Ok(())
    }

    async fn ensure_connection(&self, market_type: MarketType) -> Result<Option<Arc<WsClient>>> {
        // Do not fallback to mocks; attempt a real connection and propagate errors
        if let Some(client) = self.get_ws_client(market_type).await {
            if client.is_connected() {
                return Ok(Some(client));
            }
        }

        match self.try_real_connection(market_type).await {
            Ok(client) => {
                self.set_mock_enabled(market_type, false).await;
                self.set_ws_client(market_type, Some(client.clone())).await;
                Ok(Some(client))
            }
            Err(err) => Err(err),
        }
    }
    async fn subscribe_internal(&self, channels: &[Channel]) -> Result<()> {
        info!("Subscribing to {} Binance channels", channels.len());

        if channels.is_empty() {
            debug!("No Binance channels to subscribe");
            return Ok(());
        }

        let mut by_market: HashMap<MarketType, Vec<Channel>> = HashMap::new();
        for channel in channels {
            by_market
                .entry(channel.market_type)
                .or_insert_with(Vec::new)
                .push(channel.clone());
        }

        for (market_type, market_channels) in by_market {
            if market_channels.is_empty() {
                continue;
            }

            let maybe_client = self.ensure_connection(market_type).await?;

            if maybe_client.is_none() {
                info!(
                    market = Self::market_label(market_type),
                    "Using mock data for Binance market - subscription acknowledged"
                );
                continue;
            }

            let subscription = self.format_subscription(&market_channels)?;
            if let Some(ws_client) = maybe_client {
                ws_client.send_text(&subscription).await?;
                debug!(
                    market = Self::market_label(market_type),
                    "Sent Binance subscription: {}", subscription
                );
            }
        }

        Ok(())
    }

    async fn unsubscribe_internal(&self, channels: &[Channel]) -> Result<()> {
        info!("Unsubscribing from {} Binance channels", channels.len());

        if channels.is_empty() {
            debug!("No Binance channels to unsubscribe");
            return Ok(());
        }

        let mut by_market: HashMap<MarketType, Vec<Channel>> = HashMap::new();
        for channel in channels {
            by_market
                .entry(channel.market_type)
                .or_insert_with(Vec::new)
                .push(channel.clone());
        }

        for (market_type, market_channels) in by_market {
            if market_channels.is_empty() {
                continue;
            }

            if self.mock_enabled(market_type).await {
                info!(
                    market = Self::market_label(market_type),
                    "Using mock data for Binance market - unsubscribe acknowledged"
                );
                continue;
            }

            let unsubscription = self.format_unsubscription(&market_channels)?;
            if let Some(ws_client) = self.get_ws_client(market_type).await {
                ws_client.send_text(&unsubscription).await?;
                debug!(
                    market = Self::market_label(market_type),
                    "Sent Binance unsubscription: {}", unsubscription
                );
            } else {
                return Err(anyhow!(
                    "WebSocket client not connected for Binance {} market",
                    Self::market_label(market_type)
                ));
            }
        }

        Ok(())
    }
}

#[async_trait]

impl ExchangeAdapter for BinanceAdapter {
    fn id(&self) -> ExchangeId {
        ExchangeId::from("binance")
    }

    async fn start(&self, hub: HubHandle, cache: CacheHandle) -> Result<()> {
        info!("Starting Binance adapter");

        *self.hub.lock().await = Some(hub.clone());
        *self.cache.lock().await = Some(cache.clone());

        debug!("Binance adapter initialized with hub and cache handles");

        Ok(())
    }

    async fn subscribe(&self, channels: &[Channel]) -> Result<()> {
        self.subscribe_internal(channels).await
    }

    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()> {
        self.unsubscribe_internal(channels).await
    }

    async fn is_connected(&self) -> bool {
        // Mocks removed; only real ws client connections indicate connectivity
        let ws_guard = self.ws_clients.lock().await;
        for client in ws_guard.values() {
            if let Some(client) = client {
                if client.is_connected() {
                    return true;
                }
            }
        }

        false
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping Binance adapter");

        let mut ws_guard = self.ws_clients.lock().await;

        for (market_type, client_opt) in ws_guard.iter_mut() {
            if let Some(client) = client_opt.take() {
                info!(
                    market = Self::market_label(*market_type),
                    "Closing Binance WebSocket connection"
                );
                client.close().await?;
            }
        }

        Ok(())
    }
}

impl Default for BinanceAdapter {
    fn default() -> Self {
        Self::new()
    }
}
