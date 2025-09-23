use crate::types::{BybitMessage, BybitTicker};

use anyhow::{anyhow, Result};

use async_trait::async_trait;

use crypto_dash_cache::CacheHandle;

use crypto_dash_core::model::{
    Channel, ChannelType, ExchangeId, MarketType, StreamMessage, Symbol, Ticker,
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

const BYBIT_SPOT_WS_URL: &str = "wss://stream.bybit.com/v5/public/spot";
const BYBIT_LINEAR_WS_URL: &str = "wss://stream.bybit.com/v5/public/linear";
const SUPPORTED_MARKETS: [MarketType; 2] = [MarketType::Spot, MarketType::Perpetual];

#[derive(Clone)]
pub struct BybitAdapter {
    ws_clients: Arc<Mutex<HashMap<MarketType, Option<Arc<WsClient>>>>>,

    hub: Arc<Mutex<Option<HubHandle>>>,

    cache: Arc<Mutex<Option<CacheHandle>>>,
}

impl BybitAdapter {
    pub fn new() -> Self {
        let mut ws_clients = HashMap::new();
    // no mock generators or mock flags - production behavior only

        for market in SUPPORTED_MARKETS {
            ws_clients.insert(market, None);
            // nothing to insert for mocks
        }

        Self {
            ws_clients: Arc::new(Mutex::new(ws_clients)),

            hub: Arc::new(Mutex::new(None)),

            cache: Arc::new(Mutex::new(None)),

            // no mock state
        }
    }

    fn market_label(market_type: MarketType) -> &'static str {
        match market_type {
            MarketType::Spot => "spot",
            MarketType::Perpetual => "perpetual",
        }
    }

    async fn mock_enabled(&self, market_type: MarketType) -> bool {
        // Mocks removed; always return false
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


    async fn handle_message(&self, market_type: MarketType, message: BybitMessage) -> Result<()> {
        match message {
            BybitMessage::Ticker { ts, data, .. } => {
                for ticker in data.into_vec() {
                    self.handle_ticker(market_type, ticker, ts).await?;
                }
            }

            BybitMessage::Subscription { success, ret_msg } => {
                if success {
                    info!("Bybit subscription successful: {}", ret_msg);

                    debug!(
                        "Bybit subscription response: success={}, message={}",
                        success, ret_msg
                    );
                } else {
                    error!("Bybit subscription failed: {}", ret_msg);

                    debug!(
                        "Bybit subscription response: success={}, message={}",
                        success, ret_msg
                    );
                }
            }
        }

        Ok(())
    }

    async fn handle_ticker(
        &self,
        market_type: MarketType,
        ticker: BybitTicker,
        timestamp_ms: u64,
    ) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.symbol)?;

        let timestamp = crypto_dash_core::time::from_millis(timestamp_ms as i64)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp_ms))?;

        let bid_price = ticker
            .bid1_price
            .as_deref()
            .filter(|v| !v.is_empty())
            .or_else(|| ticker.bid_price.as_deref().filter(|v| !v.is_empty()))
            .unwrap_or_else(|| ticker.last_price.as_str());

        let ask_price = ticker
            .ask1_price
            .as_deref()
            .filter(|v| !v.is_empty())
            .or_else(|| ticker.ask_price.as_deref().filter(|v| !v.is_empty()))
            .unwrap_or_else(|| ticker.last_price.as_str());

        let bid_size = ticker
            .bid1_size
            .as_deref()
            .filter(|v| !v.is_empty())
            .or_else(|| ticker.bid_size.as_deref().filter(|v| !v.is_empty()))
            .unwrap_or("0");

        let ask_size = ticker
            .ask1_size
            .as_deref()
            .filter(|v| !v.is_empty())
            .or_else(|| ticker.ask_size.as_deref().filter(|v| !v.is_empty()))
            .unwrap_or("0");

        let normalized_ticker = Ticker {
            timestamp,
            exchange: self.id(),
            market_type,
            symbol: symbol.clone(),
            bid: Decimal::from_str(bid_price)?,
            ask: Decimal::from_str(ask_price)?,
            last: Decimal::from_str(&ticker.last_price)?,
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

    async fn clear_ws_if_current(
        &self,
        market_type: MarketType,
        ws_client: &Arc<WsClient>,
    ) -> bool {
        let mut ws_guard = self.ws_clients.lock().await;
        if let Some(entry) = ws_guard.get_mut(&market_type) {
            if let Some(current) = entry.as_ref() {
                if Arc::ptr_eq(current, ws_client) {
                    *entry = None;
                    return true;
                }
            }
        }
        false
    }

    async fn reconnect_and_send(&self, market_type: MarketType, message: &str) -> Result<()> {
        match self.try_real_connection(market_type).await {
            Ok(_) => {
                info!(
                    market = Self::market_label(market_type),
                    "Bybit: Reconnected WebSocket, resending subscription"
                );
                if let Some(client) = self.get_ws_client(market_type).await {
                    client.send_text(message).await?;
                    info!(
                        market = Self::market_label(market_type),
                        "Bybit: Subscription sent after reconnect"
                    );
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Bybit WebSocket client unavailable after reconnect"
                    ))
                }
            }
            Err(e) => {
                warn!(market = Self::market_label(market_type), "Bybit reconnect failed: {}", e);
                // Do not fall back to mock data. Surface the error to callers so they
                // can handle it gracefully. The UI will decide when to retry.
                Err(e)
            }
        }
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
                        "Bybit market disconnected due to no subscribers"
                    );
                    client.close().await?;
                }
            }
        }

        Ok(())
    }

    fn parse_symbol(&self, bybit_symbol: &str) -> Result<Symbol> {
        // Bybit uses formats like BTCUSDT, BTCUSDT_PERP, or SOLUSDT_SOL/USDT
        let primary = bybit_symbol
            .split(|c| c == '_' || c == '.')
            .next()
            .unwrap_or(bybit_symbol);

        let sanitized = primary.replace('/', "");
        let upper = sanitized.to_uppercase();

        if let Some(base) = upper.strip_suffix("USDT") {
            Ok(Symbol::new(base, "USDT"))
        } else if let Some(base) = upper.strip_suffix("USDC") {
            Ok(Symbol::new(base, "USDC"))
        } else if let Some(base) = upper.strip_suffix("BUSD") {
            Ok(Symbol::new(base, "BUSD"))
        } else if let Some(base) = upper.strip_suffix("TUSD") {
            Ok(Symbol::new(base, "TUSD"))
        } else if let Some(base) = upper.strip_suffix("BTC") {
            Ok(Symbol::new(base, "BTC"))
        } else if let Some(base) = upper.strip_suffix("ETH") {
            Ok(Symbol::new(base, "ETH"))
        } else if let Some(base) = upper.strip_suffix("USD") {
            Ok(Symbol::new(base, "USD"))
        } else {
            Err(anyhow!("Unknown Bybit symbol format: {}", bybit_symbol))
        }
    }

    fn topics_from_channels(&self, channels: &[Channel]) -> Vec<String> {
        let mut topics = Vec::new();

        for channel in channels {
            match channel.channel_type {
                ChannelType::Ticker => {
                    let symbol = format!("{}{}", channel.symbol.base, channel.symbol.quote);

                    topics.push(format!("tickers.{}", symbol));
                }

                ChannelType::OrderBook => {
                    let symbol = format!("{}{}", channel.symbol.base, channel.symbol.quote);

                    topics.push(format!("orderbook.1.{}", symbol));
                }
            }
        }

        topics
    }

    fn format_subscription(&self, channels: &[Channel]) -> Result<String> {
        let topics = self.topics_from_channels(channels);

        let subscription = serde_json::json!({







            "op": "subscribe",







            "args": topics







        });

        Ok(subscription.to_string())
    }

    fn format_unsubscription(&self, channels: &[Channel]) -> Result<String> {
        let topics = self.topics_from_channels(channels);

        let unsubscription = serde_json::json!({







            "op": "unsubscribe",







            "args": topics







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
                    warn!("Bybit WebSocket connection closed");

                    break;
                }

                Some(_) => continue,

                None => {
                    warn!("Bybit WebSocket stream ended");

                    break;
                }
            };

            match serde_json::from_str::<BybitMessage>(&message) {
                Ok(stream_message) => {
                    debug!("Received Bybit message: {:?}", stream_message);

                    if let Err(e) = self.handle_message(market_type, stream_message).await {
                        error!("Failed to handle Bybit message: {}", e);
                    }
                }

                Err(e) => {
                    warn!("Failed to parse Bybit message: {} - Raw: {}", e, message);
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
            MarketType::Spot => BYBIT_SPOT_WS_URL,
            MarketType::Perpetual => BYBIT_LINEAR_WS_URL,
        };

        debug!(
            market = Self::market_label(market_type),
            "Attempting to connect to Bybit WebSocket: {}", ws_url
        );

        let ws_client = Arc::new(WsClient::new(ws_url));

        ws_client.connect().await?;

        debug!(
            market = Self::market_label(market_type),
            "Bybit WebSocket handshake successful"
        );

        self.set_ws_client(market_type, Some(ws_client.clone()))
            .await;
        self.set_mock_enabled(market_type, false).await;

        let adapter = self.clone();
        let listener_client = ws_client.clone();
        let listener_market = market_type;

        tokio::spawn(async move {
            if let Err(e) = adapter
                .listen_for_messages(listener_market, listener_client)
                .await
            {
                error!(
                    market = BybitAdapter::market_label(listener_market),
                    "Bybit WebSocket listener error: {}", e
                );
            }
        });

        Ok(ws_client)
    }

    async fn start_mock_data(&self, market_type: MarketType, hub: HubHandle) -> Result<()> {
        // Mocking removed; nothing to do
        Ok(())
    }
    async fn subscribe_internal(&self, channels: &[Channel]) -> Result<()> {
        info!("Subscribing to {} Bybit channels", channels.len());

        if channels.is_empty() {
            debug!("No Bybit channels to subscribe");
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

            // No mock behavior: attempt to send subscription or reconnect and return error to caller

            let subscription = self.format_subscription(&market_channels)?;
            info!(
                market = Self::market_label(market_type),
                "Bybit subscription message: {}", subscription
            );

            match self.get_ws_client(market_type).await {
                Some(ws_client) => match ws_client.send_text(&subscription).await {
                    Ok(()) => {
                        info!(
                            market = Self::market_label(market_type),
                            "Successfully sent Bybit subscription: {}", subscription
                        );
                    }
                    Err(e) => {
                        error!(
                            market = Self::market_label(market_type),
                            "Failed to send Bybit subscription, connection may be broken: {}", e
                        );

                        let _cleared = self.clear_ws_if_current(market_type, &ws_client).await;

                        // Attempt a reconnect/send once and propagate any error to caller
                        self.reconnect_and_send(market_type, &subscription).await?;
                    }
                },
                None => {
                    warn!(
                        market = Self::market_label(market_type),
                        "Bybit WebSocket client not connected, attempting to reconnect"
                    );
                    self.reconnect_and_send(market_type, &subscription).await?;
                }
            }
        }

        Ok(())
    }

    async fn unsubscribe_internal(&self, channels: &[Channel]) -> Result<()> {
        info!("Unsubscribing from {} Bybit channels", channels.len());

        if channels.is_empty() {
            debug!("No Bybit channels to unsubscribe");
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

            // No mock behavior for unsubscribes

            let unsubscription = self.format_unsubscription(&market_channels)?;
            info!(
                market = Self::market_label(market_type),
                "Bybit unsubscription message: {}", unsubscription
            );

            match self.get_ws_client(market_type).await {
                Some(ws_client) => match ws_client.send_text(&unsubscription).await {
                    Ok(()) => {
                        info!(
                            market = Self::market_label(market_type),
                            "Successfully sent Bybit unsubscription: {}", unsubscription
                        );
                    }
                    Err(e) => {
                        error!(
                            market = Self::market_label(market_type),
                            "Failed to send Bybit unsubscription: {}", e
                        );
                        self.clear_ws_if_current(market_type, &ws_client).await;
                    }
                },
                None => {
                    warn!(
                        market = Self::market_label(market_type),
                        "Bybit WebSocket client not connected, unable to unsubscribe"
                    );
                }
            }
        }

        Ok(())
    }
}

#[async_trait]

impl ExchangeAdapter for BybitAdapter {
    fn id(&self) -> ExchangeId {
        ExchangeId::from("bybit")
    }

    async fn start(&self, hub: HubHandle, cache: CacheHandle) -> Result<()> {
        info!("Starting Bybit adapter");

        *self.hub.lock().await = Some(hub.clone());
        *self.cache.lock().await = Some(cache.clone());

        debug!("Bybit adapter initialized with hub and cache handles");

        Ok(())
    }

    async fn subscribe(&self, channels: &[Channel]) -> Result<()> {
        self.subscribe_internal(channels).await
    }

    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()> {
        self.unsubscribe_internal(channels).await
    }

    async fn is_connected(&self) -> bool {
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
        info!("Stopping Bybit adapter");

        let mut ws_guard = self.ws_clients.lock().await;

        for (market_type, client_opt) in ws_guard.iter_mut() {
            if let Some(client) = client_opt.take() {
                info!(
                    market = Self::market_label(*market_type),
                    "Closing Bybit WebSocket connection"
                );
                client.close().await?;
            }
        }

        Ok(())
    }
}

impl Default for BybitAdapter {
    fn default() -> Self {
        Self::new()
    }
}
