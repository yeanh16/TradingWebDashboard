use serde::{Deserialize, Serialize};

/// Binance ticker response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceTicker {
    pub s: String, // symbol
    pub c: String, // close price (last)
    pub b: String, // best bid price
    pub a: String, // best ask price
    pub B: String, // best bid quantity
    pub A: String, // best ask quantity
    #[serde(rename = "E")]
    pub event_time: i64,
}

/// Binance order book depth response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceOrderBook {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: i64,
    pub bids: Vec<[String; 2]>, // [price, quantity]
    pub asks: Vec<[String; 2]>, // [price, quantity]
}

/// Binance WebSocket stream message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BinanceStreamMessage {
    Ticker {
        stream: String,
        data: BinanceTicker,
    },
    OrderBook {
        stream: String,
        data: BinanceOrderBook,
    },
    Error {
        id: Option<i64>,
        error: BinanceError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceError {
    pub code: i32,
    pub msg: String,
}