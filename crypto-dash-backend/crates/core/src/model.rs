use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Exchange identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExchangeId(pub String);

impl ExchangeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ExchangeId {
    fn from(s: &str) -> Self {
        ExchangeId(s.to_string())
    }
}

/// Normalized symbol representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub base: String,
    pub quote: String,
}

impl Symbol {
    pub fn new(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }

    pub fn canonical(&self) -> String {
        format!("{}-{}", self.base, self.quote)
    }
}

/// Market category for a given trading instrument
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    Spot,
    Perpetual,
}

impl Default for MarketType {
    fn default() -> Self {
        MarketType::Spot
    }
}

/// Exchange-specific symbol information (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub exchange_symbol: String,
    pub base: String,
    pub quote: String,
    pub price_precision: u8,
    pub qty_precision: u8,
    pub min_qty: Decimal,
    pub tick_size: Decimal,
}

/// Canonical symbol metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMeta {
    pub exchange: ExchangeId,
    pub market_type: MarketType,
    pub symbol: String,
    pub base: String,
    pub quote: String,
    pub price_precision: u32,
    pub tick_size: String, // Use string to preserve exact decimal representation
    pub min_qty: Decimal,
    pub step_size: Decimal,
    pub filters: Option<HashMap<String, String>>,
    pub info: serde_json::Value,
}

/// Price level in order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

impl PriceLevel {
    pub fn new(price: Decimal, quantity: Decimal) -> Self {
        Self { price, quantity }
    }
}

/// Live ticker data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    pub timestamp: DateTime<Utc>,
    pub exchange: ExchangeId,
    #[serde(default)]
    pub market_type: MarketType,
    pub symbol: Symbol,
    pub bid: Decimal,
    pub ask: Decimal,
    pub last: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
}

/// Candlestick data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candlestick {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

/// Order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub timestamp: DateTime<Utc>,
    pub exchange: ExchangeId,
    #[serde(default)]
    pub market_type: MarketType,
    pub symbol: Symbol,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub checksum: Option<String>,
}

/// Order book delta update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub timestamp: DateTime<Utc>,
    pub exchange: ExchangeId,
    #[serde(default)]
    pub market_type: MarketType,
    pub symbol: Symbol,
    pub bids_upserts: Vec<PriceLevel>,
    pub asks_upserts: Vec<PriceLevel>,
    pub deletes: Option<Vec<Decimal>>, // price levels to delete
}

/// Market data channel types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Ticker,
    OrderBook,
}

/// Subscription channel specification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Channel {
    pub channel_type: ChannelType,
    pub exchange: ExchangeId,
    #[serde(default)]
    pub market_type: MarketType,
    pub symbol: Symbol,
    pub depth: Option<u16>, // for order book channels
}

/// WebSocket message types sent to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "snake_case")]
pub enum StreamMessage {
    Ticker(Ticker),
    OrderBookSnapshot(OrderBookSnapshot),
    OrderBookDelta(OrderBookDelta),
    Info { message: String },
    Error { message: String },
}

/// WebSocket operations from clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
#[serde(rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe { channels: Vec<Channel> },
    Unsubscribe { channels: Vec<Channel> },
    Ping,
}

/// Exchange metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeInfo {
    pub id: ExchangeId,
    pub name: String,
    pub status: ExchangeStatus,
    pub rate_limits: HashMap<String, u32>,
    pub ws_url: String,
    pub rest_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExchangeStatus {
    Online,
    Offline,
    Maintenance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_canonical() {
        let symbol = Symbol::new("BTC", "USDT");
        assert_eq!(symbol.canonical(), "BTC-USDT");
    }

    #[test]
    fn test_price_level_creation() {
        let level = PriceLevel::new(Decimal::new(50000, 0), Decimal::new(1, 1));
        assert_eq!(level.price, Decimal::new(50000, 0));
        assert_eq!(level.quantity, Decimal::new(1, 1));
    }

    #[test]
    fn ticker_defaults_to_spot_market() {
        let ticker = Ticker {
            timestamp: Utc::now(),
            exchange: ExchangeId::from("binance"),
            market_type: MarketType::default(),
            symbol: Symbol::new("BTC", "USDT"),
            bid: Decimal::new(50000, 0),
            ask: Decimal::new(50010, 0),
            last: Decimal::new(50005, 0),
            bid_size: Decimal::new(1, 0),
            ask_size: Decimal::new(1, 0),
        };

        assert_eq!(ticker.market_type, MarketType::Spot);
    }
}
