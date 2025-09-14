use crate::types::{BinanceStreamMessage, BinanceTicker, BinanceOrderBook};
use crypto_dash_cache::CacheHandle;
use crypto_dash_core::{
    model::{
        Channel, ChannelType, ExchangeId, Symbol, Ticker, OrderBookSnapshot, 
        PriceLevel, StreamMessage
    },
    time::from_millis,
};
use crypto_dash_exchanges_common::{ExchangeAdapter, WsClient};
use crypto_dash_stream_hub::{HubHandle, Topic};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use std::str::FromStr;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info};

const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

pub struct BinanceAdapter {
    ws_client: Option<WsClient>,
    hub: Option<HubHandle>,
    cache: Option<CacheHandle>,
}

impl BinanceAdapter {
    pub fn new() -> Self {
        Self {
            ws_client: None,
            hub: None,
            cache: None,
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
        if let Some(cache) = &self.cache {
            cache.set_ticker(normalized_ticker.clone()).await;
        }

        // Publish to stream hub
        if let Some(hub) = &self.hub {
            let topic = Topic::ticker(self.id(), symbol);
            hub.publish(&topic, StreamMessage::Ticker(normalized_ticker)).await;
        }

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
        if let Some(cache) = &self.cache {
            cache.set_orderbook(normalized_orderbook.clone()).await;
        }

        // Publish to stream hub
        if let Some(hub) = &self.hub {
            let topic = Topic::orderbook(self.id(), symbol);
            hub.publish(&topic, StreamMessage::OrderBookSnapshot(normalized_orderbook)).await;
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
            let symbol_str = format!("{}{}", channel.symbol.base.to_lowercase(), channel.symbol.quote.to_lowercase());
            
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
}

#[async_trait]
impl ExchangeAdapter for BinanceAdapter {
    fn id(&self) -> ExchangeId {
        ExchangeId::from("binance")
    }

    async fn start(&self, _hub: HubHandle, _cache: CacheHandle) -> Result<()> {
        info!("Starting Binance adapter");
        
        // Store handles (in a real implementation, you'd use Arc<Mutex<>> or similar)
        // For this demo, we'll just log that it's started
        debug!("Binance adapter started with hub and cache handles");
        
        Ok(())
    }

    async fn subscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Subscribing to {} Binance channels", channels.len());
        
        // In a real implementation, this would connect to WebSocket and subscribe
        for channel in channels {
            debug!(
                "Would subscribe to Binance {} for {}/{}",
                match channel.channel_type {
                    ChannelType::Ticker => "ticker",
                    ChannelType::OrderBook => "orderbook",
                },
                channel.exchange.as_str(),
                channel.symbol.canonical()
            );
        }
        
        Ok(())
    }

    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Unsubscribing from {} Binance channels", channels.len());
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        // In a real implementation, check WebSocket connection status
        true
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping Binance adapter");
        Ok(())
    }
}

impl Default for BinanceAdapter {
    fn default() -> Self {
        Self::new()
    }
}