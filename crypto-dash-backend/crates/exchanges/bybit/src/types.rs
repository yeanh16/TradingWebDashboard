use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BybitTicker {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "bidPrice", default)]
    pub bid_price: Option<String>,
    #[serde(rename = "askPrice", default)]
    pub ask_price: Option<String>,
    #[serde(rename = "bidSize", default)]
    pub bid_size: Option<String>,
    #[serde(rename = "askSize", default)]
    pub ask_size: Option<String>,
    #[serde(rename = "highPrice24h")]
    pub high_price_24h: String,
    #[serde(rename = "lowPrice24h")]
    pub low_price_24h: String,
    #[serde(rename = "prevPrice24h")]
    pub prev_price_24h: String,
    #[serde(rename = "volume24h")]
    pub volume_24h: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BybitMessage {
    Ticker {
        topic: String,
        ts: u64,
        #[serde(rename = "type")]
        message_type: String,
        data: BybitTicker,
    },
    Subscription {
        success: bool,
        #[serde(rename = "ret_msg")]
        ret_msg: String,
    },
}
