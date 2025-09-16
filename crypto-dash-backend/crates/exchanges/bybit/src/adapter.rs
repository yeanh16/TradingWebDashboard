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
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
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
    pending_channels: Arc<Mutex<Vec<Channel>>>, // Store channels for resubscription
    reconnect_attempts: Arc<Mutex<u32>>,
}

impl BybitAdapter {
    pub fn new() -> Self {
        Self {
            ws_client: Arc::new(Mutex::new(None)),
            hub: Arc::new(Mutex::new(None)),
            cache: Arc::new(Mutex::new(None)),
            mock_generator: Arc::new(Mutex::new(None)),
            use_mock_data: Arc::new(Mutex::new(false)),
            pending_channels: Arc::new(Mutex::new(Vec::new())),
            reconnect_attempts: Arc::new(Mutex::new(0)),
        }
    }

    async fn handle_message(&self, message: BybitMessage) -> Result<()> {
        match message {
            BybitMessage::Ticker { topic: _, ts, message_type: _, data } => {
                self.handle_ticker(data, ts).await?;
            }
            BybitMessage::Subscription { success, ret_msg } => {
                if success {
                    info!("Bybit subscription successful: {}", ret_msg);
                    debug!("Bybit subscription response: success={}, message={}", success, ret_msg);
                } else {
                    error!("Bybit subscription failed: {}", ret_msg);
                    debug!("Bybit subscription response: success={}, message={}", success, ret_msg);
                }
            }
        }
        Ok(())
    }

    async fn handle_ticker(&self, ticker: BybitTicker, timestamp_ms: u64) -> Result<()> {
        let symbol = self.parse_symbol(&ticker.symbol)?;
        let timestamp = crypto_dash_core::time::from_millis(timestamp_ms as i64)
            .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp_ms))?;

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
                    // Check if connection is stale and needs reconnection
                    if ws_client.is_stale() {
                        warn!("Bybit WebSocket connection is stale, attempting reconnection");
                        drop(ws_guard); // Release lock before reconnection
                        if let Err(e) = self.handle_reconnection().await {
                            error!("Failed to reconnect Bybit WebSocket: {}", e);
                            break;
                        }
                        continue;
                    }
                    
                    match ws_client.next_message().await {
                        Ok(Some(Message::Text(text))) => text,
                        Ok(Some(Message::Close(_))) => {
                            warn!("Bybit WebSocket connection closed by server");
                            drop(ws_guard); // Release lock before reconnection
                            if let Err(e) = self.handle_reconnection().await {
                                error!("Failed to reconnect after server close: {}", e);
                                break;
                            }
                            continue;
                        }
                        Ok(Some(_)) => continue, // Skip non-text messages
                        Ok(None) => {
                            warn!("Bybit WebSocket stream ended");
                            drop(ws_guard); // Release lock before reconnection
                            if let Err(e) = self.handle_reconnection().await {
                                error!("Failed to reconnect after stream end: {}", e);
                                break;
                            }
                            continue;
                        }
                        Err(e) => {
                            error!("Bybit WebSocket error: {}", e);
                            drop(ws_guard); // Release lock before reconnection
                            if let Err(reconnect_err) = self.handle_reconnection().await {
                                error!("Failed to reconnect after error: {}", reconnect_err);
                                break;
                            }
                            continue;
                        }
                    }
                } else {
                    error!("WebSocket client not initialized");
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
        Ok(())
    }

    async fn try_real_connection(&self) -> Result<()> {
        // Initialize WebSocket client with ping/pong support
        let mut ws_client = WsClient::with_timeouts(
            BYBIT_WS_URL,
            Duration::from_secs(20), // Ping every 20 seconds  
            Duration::from_secs(60), // Connection timeout after 60 seconds
        );
        
        ws_client.connect().await?;
        *self.ws_client.lock().await = Some(ws_client);
        *self.reconnect_attempts.lock().await = 0; // Reset reconnect attempts on successful connection
        
        info!("Bybit WebSocket connection established successfully");
        Ok(())
    }

    async fn handle_reconnection(&self) -> Result<()> {
        let mut attempts = self.reconnect_attempts.lock().await;
        *attempts += 1;
        let current_attempts = *attempts;
        drop(attempts);

        info!("Bybit WebSocket reconnection attempt #{}", current_attempts);

        // Try direct connection 
        let max_attempts = 5;
        for attempt in 1..=max_attempts {
            match self.try_real_connection().await {
                Ok(()) => {
                    info!("Bybit WebSocket reconnected successfully on attempt {}", attempt);
                    *self.use_mock_data.lock().await = false;
                    
                    // Don't restart listener here to avoid recursive calls
                    // The existing listener will handle the new connection
                    
                    // Resubscribe to pending channels
                    let channels = self.pending_channels.lock().await.clone();
                    if !channels.is_empty() {
                        info!("Resubscribing to {} Bybit channels after reconnection", channels.len());
                        if let Err(e) = self.resubscribe_channels(&channels).await {
                            error!("Failed to resubscribe after reconnection: {}", e);
                        }
                    }
                    
                    return Ok(());
                }
                Err(e) => {
                    error!("Reconnection attempt {} failed: {}", attempt, e);
                    if attempt < max_attempts {
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempt.min(6))));
                        info!("Waiting {:?} before next reconnection attempt", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        // All attempts failed
        error!("Failed to reconnect Bybit WebSocket after {} attempts", max_attempts);
        
        // Fall back to mock data if reconnection fails
        warn!("Bybit: Falling back to mock data due to reconnection failure");
        *self.use_mock_data.lock().await = true;
        
        if let Some(hub) = &*self.hub.lock().await {
            self.start_mock_data(hub.clone()).await?;
        }
        
        Err(anyhow!("Failed to reconnect after {} attempts", max_attempts))
    }

    async fn resubscribe_channels(&self, channels: &[Channel]) -> Result<()> {
        let subscription = self.format_subscription(channels)?;
        info!("Bybit resubscription message: {}", subscription);
        
        let mut ws_guard = self.ws_client.lock().await;
        if let Some(ws_client) = ws_guard.as_mut() {
            ws_client.send_text(&subscription).await?;
            info!("Successfully resubscribed to Bybit channels");
        } else {
            warn!("Cannot resubscribe: WebSocket client not available");
        }
        
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
                // Start the message listener after successful connection
                let adapter = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = adapter.listen_for_messages().await {
                        error!("Bybit WebSocket listener error: {}", e);
                    }
                });
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
        
        // Store channels for potential resubscription
        *self.pending_channels.lock().await = channels.to_vec();
        
        let use_mock = *self.use_mock_data.lock().await;
        
        if use_mock {
            info!("Using mock data for Bybit - subscription request acknowledged");
            return Ok(());
        }
        
        let subscription = self.format_subscription(channels)?;
        info!("Bybit subscription message: {}", subscription);
        
        let mut ws_guard = self.ws_client.lock().await;
        if let Some(ws_client) = ws_guard.as_mut() {
            match ws_client.send_text(&subscription).await {
                Ok(()) => {
                    info!("Successfully sent Bybit subscription: {}", subscription);
                }
                Err(e) => {
                    error!("Failed to send Bybit subscription, attempting reconnection: {}", e);
                    // Clear the broken connection
                    *ws_guard = None;
                    drop(ws_guard);
                    
                    // Attempt reconnection
                    if let Err(reconnect_err) = self.handle_reconnection().await {
                        warn!("Reconnection failed, switching to mock data: {}", reconnect_err);
                        *self.use_mock_data.lock().await = true;
                        if let Some(hub) = &*self.hub.lock().await {
                            self.start_mock_data(hub.clone()).await?;
                            info!("Bybit: Switched to mock data due to connection failure");
                        }
                    }
                }
            }
        } else {
            warn!("Bybit WebSocket client not connected, attempting connection");
            drop(ws_guard);
            
            // Try to establish connection and subscribe
            match self.try_real_connection().await {
                Ok(()) => {
                    // Wait a bit for connection to stabilize
                    sleep(Duration::from_millis(500)).await;
                    
                    // Try subscription again
                    let mut ws_guard = self.ws_client.lock().await;
                    if let Some(ws_client) = ws_guard.as_mut() {
                        if let Err(e) = ws_client.send_text(&subscription).await {
                            error!("Failed to send subscription after reconnection: {}", e);
                        } else {
                            info!("Successfully subscribed after reconnection");
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to establish Bybit connection, switching to mock data: {}", e);
                    *self.use_mock_data.lock().await = true;
                    if let Some(hub) = &*self.hub.lock().await {
                        self.start_mock_data(hub.clone()).await?;
                        info!("Bybit: Using mock data for subscription");
                    }
                }
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