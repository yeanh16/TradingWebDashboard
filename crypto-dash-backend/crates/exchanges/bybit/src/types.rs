use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BybitTicker {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "bidPrice")]
    pub bid_price: String,
    #[serde(rename = "askPrice")]
    pub ask_price: String,
    #[serde(rename = "bidSize")]
    pub bid_size: String,
    #[serde(rename = "askSize")]
    pub ask_size: String,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BybitMessage {
    Ticker {
        topic: String,
        data: BybitTicker,
    },
    Subscription {
        success: bool,
        #[serde(rename = "ret_msg")]
        ret_msg: String,
    },
}