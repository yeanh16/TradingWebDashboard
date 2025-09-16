use crate::types::{Binance24hrTicker, BinanceOrderBook, BinanceStreamMessage, BinanceTicker};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crypto_dash_cache::CacheHandle;
use crypto_dash_core::{
    model::{
        Channel, ChannelType, ExchangeId, OrderBookSnapshot, PriceLevel, StreamMessage, Symbol,
        Ticker,
    },
    time::from_millis,
};
use crypto_dash_exchanges_common::{ExchangeAdapter, MockDataGenerator, WsClient};
use crypto_dash_stream_hub::{HubHandle, Topic};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

#[derive(Clone)]

pub struct BinanceAdapter {
    ws_client: Arc<Mutex<Option<Arc<WsClient>>>>,

    hub: Arc<Mutex<Option<HubHandle>>>,

    cache: Arc<Mutex<Option<CacheHandle>>>,

    mock_generator: Arc<Mutex<Option<MockDataGenerator>>>,

    use_mock_data: Arc<Mutex<bool>>,
}

impl BinanceAdapter {
    pub fn new() -> Self {
        Self {
            ws_client: Arc::new(Mutex::new(None)),

            hub: Arc::new(Mutex::new(None)),

            cache: Arc::new(Mutex::new(None)),

            mock_generator: Arc::new(Mutex::new(None)),

            use_mock_data: Arc::new(Mutex::new(false)),
        }
    }

    async fn handle_message(&self, message: BinanceStreamMessage) -> Result<()> {
        match message {
            BinanceStreamMessage::Ticker { stream: _, data } => {
                self.handle_ticker(data).await?;
            }

            BinanceStreamMessage::OrderBook { stream, data } => {
                self.handle_orderbook(&stream, data).await?;
            }

            BinanceStreamMessage::DirectTicker24hr(ticker_24hr) => {
                self.handle_24hr_ticker(ticker_24hr).await?;
            }

            BinanceStreamMessage::Error { error, .. } => {
                error!("Binance error: {} - {}", error.code, error.msg);
            }
        }

        Ok(())
    }

    async fn handle_ticker(&self, ticker: BinanceTicker) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.s)?;

        let timestamp = from_millis(ticker.event_time)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", ticker.event_time))?;

        let normalized_ticker = Ticker {
            timestamp,

            exchange: self.id(),

            symbol: symbol.clone(),

            bid: Decimal::from_str(&ticker.b)?,

            ask: Decimal::from_str(&ticker.a)?,

            last: Decimal::from_str(&ticker.c)?,

            bid_size: Decimal::from_str(&ticker.B)?,

            ask_size: Decimal::from_str(&ticker.A)?,
        };

        // Cache the ticker

        if let Some(cache) = &*self.cache.lock().await {
            cache.set_ticker(normalized_ticker.clone()).await;
        }

        // Publish to stream hub

        if let Some(hub) = &*self.hub.lock().await {
            let topic = Topic::ticker(self.id(), symbol);

            hub.publish(&topic, StreamMessage::Ticker(normalized_ticker))
                .await;
        }

        Ok(())
    }

    async fn handle_24hr_ticker(&self, ticker: Binance24hrTicker) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.s)?;

        let timestamp = from_millis(ticker.event_time)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", ticker.event_time))?;

        let normalized_ticker = Ticker {
            timestamp,

            exchange: self.id(),

            symbol: symbol.clone(),

            bid: Decimal::from_str(&ticker.b)?,

            ask: Decimal::from_str(&ticker.a)?,

            last: Decimal::from_str(&ticker.c)?,

            bid_size: Decimal::from_str(&ticker.best_bid_qty)?,

            ask_size: Decimal::from_str(&ticker.best_ask_qty)?,
        };

        // Cache the ticker

        if let Some(cache) = &*self.cache.lock().await {
            cache.set_ticker(normalized_ticker.clone()).await;
        }

        // Publish to stream hub

        if let Some(hub) = &*self.hub.lock().await {
            let topic = Topic::ticker(self.id(), symbol);

            hub.publish(&topic, StreamMessage::Ticker(normalized_ticker))
                .await;
        }

        debug!("Processed 24hr ticker for {}: last={}", ticker.s, ticker.c);

        Ok(())
    }

    async fn handle_orderbook(&self, stream: &str, orderbook: BinanceOrderBook) -> Result<()> {
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

            symbol: symbol.clone(),

            bids,

            asks,

            checksum: None,
        };

        // Cache the orderbook

        if let Some(cache) = &*self.cache.lock().await {
            cache.set_orderbook(normalized_orderbook.clone()).await;
        }

        // Publish to stream hub

        if let Some(hub) = &*self.hub.lock().await {
            let topic = Topic::orderbook(self.id(), symbol);

            hub.publish(
                &topic,
                StreamMessage::OrderBookSnapshot(normalized_orderbook),
            )
            .await;
        }

        Ok(())
    }

    fn parse_symbol(&self, binance_symbol: &str) -> Result<Symbol> {
        // Simple parsing - in production, this should use the symbol mapper

        if binance_symbol.ends_with("USDT") {
            let base = &binance_symbol[..binance_symbol.len() - 4];
            Ok(Symbol::new(base, "USDT"))
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

    fn format_subscription(&self, channels: &[Channel]) -> Result<String> {
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

        let subscription = serde_json::json!({

            "method": "SUBSCRIBE",

            "params": streams,

            "id": 1

        });

        Ok(subscription.to_string())
    }

    async fn listen_for_messages(&self, ws_client: Arc<WsClient>) -> Result<()> {
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
                    if let Err(e) = self.handle_message(stream_message).await {
                        error!("Failed to handle Binance message: {}", e);
                    }
                }

                Err(e) => {
                    debug!("Failed to parse Binance message: {} - Raw: {}", e, message);
                }
            }
        }

        let mut ws_guard = self.ws_client.lock().await;

        if let Some(current) = ws_guard.as_ref() {
            if Arc::ptr_eq(current, &ws_client) {
                *ws_guard = None;
            }
        }

        Ok(())
    }

    async fn try_real_connection(&self) -> Result<()> {
        debug!(
            "Attempting to connect to Binance WebSocket: {}",
            BINANCE_WS_URL
        );
        let ws_client = Arc::new(WsClient::new(BINANCE_WS_URL));
        ws_client.connect().await?;
        debug!("Binance WebSocket handshake successful");

        {
            let mut guard = self.ws_client.lock().await;
            *guard = Some(ws_client.clone());
        }

        let adapter = self.clone();
        let listener_client = ws_client.clone();

        tokio::spawn(async move {
            if let Err(e) = adapter.listen_for_messages(listener_client).await {
                error!("Binance WebSocket listener error: {}", e);
            }
        });

        Ok(())
    }

    async fn start_mock_data(&self, hub: HubHandle) -> Result<()> {
        info!("Starting Binance mock data generator");

        let mock_generator = MockDataGenerator::new(self.id(), hub);

        mock_generator.start().await;

        *self.mock_generator.lock().await = Some(mock_generator);

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

        // Store handles

        *self.hub.lock().await = Some(hub.clone());

        *self.cache.lock().await = Some(cache.clone());

        // Try to connect to real WebSocket first

        match self.try_real_connection().await {
            Ok(()) => {
                info!("Binance adapter connected to real WebSocket");

                *self.use_mock_data.lock().await = false;
            }

            Err(e) => {
                warn!(
                    "Failed to connect to real Binance WebSocket: {}. Falling back to mock data.",
                    e
                );

                self.start_mock_data(hub).await?;

                *self.use_mock_data.lock().await = true;
            }
        }

        debug!("Binance adapter started with hub and cache handles");

        Ok(())
    }

    async fn subscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Subscribing to {} Binance channels", channels.len());

        let use_mock = *self.use_mock_data.lock().await;

        if use_mock {
            info!("Using mock data for Binance - subscription request acknowledged");

            // Mock data generator is already running, just acknowledge the subscription

            return Ok(());
        }

        let subscription = self.format_subscription(channels)?;

        let ws_client = {
            let ws_guard = self.ws_client.lock().await;

            ws_guard.clone()
        };

        if let Some(ws_client) = ws_client {
            ws_client.send_text(&subscription).await?;

            debug!("Sent Binance subscription: {}", subscription);
        } else {
            return Err(anyhow!("WebSocket client not connected"));
        }

        Ok(())
    }

    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Unsubscribing from {} Binance channels", channels.len());

        Ok(())
    }

    async fn is_connected(&self) -> bool {
        let use_mock = *self.use_mock_data.lock().await;

        if use_mock {
            // Mock data is always "connected"

            return true;
        }

        let ws_guard = self.ws_client.lock().await;

        if let Some(ws_client) = ws_guard.as_ref() {
            ws_client.is_connected()
        } else {
            false
        }
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping Binance adapter");

        let mut ws_guard = self.ws_client.lock().await;

        if let Some(ws_client) = ws_guard.take() {
            ws_client.close().await?;
        }

        Ok(())
    }
}

impl Default for BinanceAdapter {
    fn default() -> Self {
        Self::new()
    }
}
