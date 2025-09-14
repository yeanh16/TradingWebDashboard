use crate::types::{BybitMessage, BybitTicker};
use crypto_dash_cache::CacheHandle;
use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, Symbol, Ticker, StreamMessage};
use crypto_dash_exchanges_common::{ExchangeAdapter, WsClient, MockDataGenerator};
use crypto_dash_stream_hub::{HubHandle, Topic};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

const BYBIT_WS_URL: &str = "wss://stream.bybit.com/v5/public/spot";

#[derive(Clone)]
pub struct BybitAdapter {
    ws_client: Arc<Mutex<Option<WsClient>>>,
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
            BybitMessage::Ticker { topic: _, data } => {
                self.handle_ticker(data).await?;
            }
            BybitMessage::Subscription { success, ret_msg } => {
                if success {
                    debug!("Bybit subscription successful: {}", ret_msg);
                } else {
                    error!("Bybit subscription failed: {}", ret_msg);
                }
            }
        }
        Ok(())
    }

    async fn handle_ticker(&self, ticker: BybitTicker) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.symbol)?;
        let timestamp = crypto_dash_core::time::from_millis(ticker.ts as i64)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", ticker.ts))?;

        let normalized_ticker = Ticker {
            timestamp,
            exchange: self.id(),
            symbol: symbol.clone(),
            bid: Decimal::from_str(&ticker.bid_price)?,
            ask: Decimal::from_str(&ticker.ask_price)?,
            last: Decimal::from_str(&ticker.last_price)?,
            bid_size: Decimal::from_str(&ticker.bid_size)?,
            ask_size: Decimal::from_str(&ticker.ask_size)?,
        };

        // Cache the ticker
        if let Some(cache) = &*self.cache.lock().await {
            cache.set_ticker(normalized_ticker.clone()).await;
        }

        // Publish to stream hub
        if let Some(hub) = &*self.hub.lock().await {
            let topic = Topic::ticker(self.id(), symbol);
            hub.publish(&topic, StreamMessage::Ticker(normalized_ticker)).await;
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

    async fn listen_for_messages(&self) -> Result<()> {
        loop {
            let message = {
                let mut ws_guard = self.ws_client.lock().await;
                if let Some(ws_client) = ws_guard.as_mut() {
                    match ws_client.next_message().await? {
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
                    }
                } else {
                    error!("WebSocket client not initialized");
                    break;
                }
            };

            match serde_json::from_str::<BybitMessage>(&message) {
                Ok(stream_message) => {
                    if let Err(e) = self.handle_message(stream_message).await {
                        error!("Failed to handle Bybit message: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to parse Bybit message: {} - Raw: {}", e, message);
                }
            }
        }
        Ok(())
    }

    async fn try_real_connection(&self) -> Result<()> {
        // Initialize WebSocket client
        let ws_client = WsClient::new(BYBIT_WS_URL).connect().await?;
        *self.ws_client.lock().await = Some(ws_client);
        
        // Start listening for messages in a background task
        let adapter = self.clone();
        tokio::spawn(async move {
            if let Err(e) = adapter.listen_for_messages().await {
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
                warn!("Failed to connect to real Bybit WebSocket: {}. Falling back to mock data.", e);
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
        
        let mut ws_guard = self.ws_client.lock().await;
        if let Some(ws_client) = ws_guard.as_mut() {
            ws_client.send_text(&subscription).await?;
            debug!("Sent Bybit subscription: {}", subscription);
        } else {
            return Err(anyhow!("WebSocket client not connected"));
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
        if let Some(mut ws_client) = ws_guard.take() {
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