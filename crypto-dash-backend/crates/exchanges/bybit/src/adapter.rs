use crate::types::{BybitMessage, BybitTicker};

use anyhow::{anyhow, Result};

use async_trait::async_trait;

use crypto_dash_cache::CacheHandle;

use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, StreamMessage, Symbol, Ticker};

use crypto_dash_exchanges_common::{ExchangeAdapter, MockDataGenerator, WsClient};

use crypto_dash_stream_hub::{HubHandle, Topic};

use rust_decimal::Decimal;

use std::str::FromStr;

use std::sync::Arc;

use tokio::sync::Mutex;

use tokio_tungstenite::tungstenite::Message;

use tracing::{debug, error, info, warn};

const BYBIT_WS_URL: &str = "wss://stream.bybit.com/v5/public/spot";

#[derive(Clone)]

pub struct BybitAdapter {
    ws_client: Arc<Mutex<Option<Arc<WsClient>>>>,

    hub: Arc<Mutex<Option<HubHandle>>>,

    cache: Arc<Mutex<Option<CacheHandle>>>,

    mock_generator: Arc<Mutex<Option<MockDataGenerator>>>,

    use_mock_data: Arc<Mutex<bool>>,
}

impl BybitAdapter {
    pub fn new() -> Self {
        Self {
            ws_client: Arc::new(Mutex::new(None)),

            hub: Arc::new(Mutex::new(None)),

            cache: Arc::new(Mutex::new(None)),

            mock_generator: Arc::new(Mutex::new(None)),

            use_mock_data: Arc::new(Mutex::new(false)),
        }
    }

    async fn handle_message(&self, message: BybitMessage) -> Result<()> {
        match message {
            BybitMessage::Ticker {
                topic: _,

                ts,

                message_type: _,

                data,
            } => {
                self.handle_ticker(data, ts).await?;
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

    async fn handle_ticker(&self, ticker: BybitTicker, timestamp_ms: u64) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.symbol)?;
        let timestamp = crypto_dash_core::time::from_millis(timestamp_ms as i64)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp_ms))?;

        let bid_price = ticker.bid_price.as_deref().unwrap_or(&ticker.last_price);
        let ask_price = ticker.ask_price.as_deref().unwrap_or(&ticker.last_price);
        let bid_size = ticker.bid_size.as_deref().unwrap_or("0");
        let ask_size = ticker.ask_size.as_deref().unwrap_or("0");

        let normalized_ticker = Ticker {
            timestamp,
            exchange: self.id(),
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

        if let Some(hub) = &*self.hub.lock().await {
            let topic = Topic::ticker(self.id(), symbol);
            hub.publish(&topic, StreamMessage::Ticker(normalized_ticker))
                .await;
        }

        Ok(())
    }

    fn parse_symbol(&self, bybit_symbol: &str) -> Result<Symbol> {
        // Bybit uses format like BTCUSDT

        if bybit_symbol.ends_with("USDT") {
            let base = &bybit_symbol[..bybit_symbol.len() - 4];

            Ok(Symbol::new(base, "USDT"))
        } else if bybit_symbol.ends_with("USD") {
            let base = &bybit_symbol[..bybit_symbol.len() - 3];

            Ok(Symbol::new(base, "USD"))
        } else {
            Err(anyhow!("Unknown Bybit symbol format: {}", bybit_symbol))
        }
    }

    fn format_subscription(&self, channels: &[Channel]) -> Result<String> {
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

        let subscription = serde_json::json!({



            "op": "subscribe",



            "args": topics



        });

        Ok(subscription.to_string())
    }

    async fn listen_for_messages(&self, ws_client: Arc<WsClient>) -> Result<()> {
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

                    if let Err(e) = self.handle_message(stream_message).await {
                        error!("Failed to handle Bybit message: {}", e);
                    }
                }

                Err(e) => {
                    warn!("Failed to parse Bybit message: {} - Raw: {}", e, message);
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
        debug!("Attempting to connect to Bybit WebSocket: {}", BYBIT_WS_URL);

        let ws_client = Arc::new(WsClient::new(BYBIT_WS_URL));

        ws_client.connect().await?;

        debug!("Bybit WebSocket handshake successful");

        {
            let mut guard = self.ws_client.lock().await;

            *guard = Some(ws_client.clone());
        }

        let adapter = self.clone();

        let listener_client = ws_client.clone();

        tokio::spawn(async move {
            if let Err(e) = adapter.listen_for_messages(listener_client).await {
                error!("Bybit WebSocket listener error: {}", e);
            }
        });

        Ok(())
    }

    async fn start_mock_data(&self, hub: HubHandle) -> Result<()> {
        info!("Starting Bybit mock data generator");

        let mock_generator = MockDataGenerator::new(self.id(), hub);

        mock_generator.start().await;

        *self.mock_generator.lock().await = Some(mock_generator);

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

        // Store handles

        *self.hub.lock().await = Some(hub.clone());

        *self.cache.lock().await = Some(cache.clone());

        // Try to connect to real WebSocket first

        match self.try_real_connection().await {
            Ok(()) => {
                info!("Bybit adapter connected to real WebSocket");

                *self.use_mock_data.lock().await = false;
            }

            Err(e) => {
                warn!(
                    "Failed to connect to real Bybit WebSocket: {}. Falling back to mock data.",
                    e
                );

                self.start_mock_data(hub).await?;

                *self.use_mock_data.lock().await = true;
            }
        }

        debug!("Bybit adapter started with hub and cache handles");

        Ok(())
    }

    async fn subscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Subscribing to {} Bybit channels", channels.len());

        let use_mock = *self.use_mock_data.lock().await;

        if use_mock {
            info!("Using mock data for Bybit - subscription request acknowledged");
            // Mock data generator is already running, just acknowledge the subscription
            return Ok(());
        }

        let subscription = self.format_subscription(channels)?;
        info!("Bybit subscription message: {}", subscription);

        let ws_client = {
            let ws_guard = self.ws_client.lock().await;
            ws_guard.clone()
        };

        if let Some(ws_client) = ws_client {
            match ws_client.send_text(&subscription).await {
                Ok(()) => {
                    info!("Successfully sent Bybit subscription: {}", subscription);
                }
                Err(e) => {
                    error!(
                        "Failed to send Bybit subscription, connection may be broken: {}",
                        e
                    );
                    {
                        let mut ws_guard = self.ws_client.lock().await;
                        if let Some(current) = ws_guard.as_ref() {
                            if Arc::ptr_eq(current, &ws_client) {
                                *ws_guard = None;
                            }
                        }
                    }
                    warn!("Cleared broken Bybit WebSocket connection, switching to mock data");
                    // Switch to mock data as fallback
                    *self.use_mock_data.lock().await = true;
                    if let Some(hub) = &*self.hub.lock().await {
                        self.start_mock_data(hub.clone()).await?;
                        info!("Bybit: Switched to mock data due to connection failure");
                    }
                }
            }
        } else {
            warn!("Bybit WebSocket client not connected, switching to mock data");
            *self.use_mock_data.lock().await = true;
            if let Some(hub) = &*self.hub.lock().await {
                self.start_mock_data(hub.clone()).await?;
                info!("Bybit: Using mock data for subscription");
            }
        }

        Ok(())
    }

    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Unsubscribing from {} Bybit channels", channels.len());

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
        info!("Stopping Bybit adapter");

        let mut ws_guard = self.ws_client.lock().await;

        if let Some(ws_client) = ws_guard.take() {
            ws_client.close().await?;
        }

        Ok(())
    }
}

impl Default for BybitAdapter {
    fn default() -> Self {
        Self::new()
    }
}
