use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BybitTicker {
    pub symbol: String,
    #[serde(rename = "tickDirection")]
    pub tick_direction: Option<String>,
    #[serde(rename = "price24hPcnt")]
    pub price24h_pcnt: Option<String>,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "prevPrice24h")]
    pub prev_price_24h: Option<String>,
    #[serde(rename = "highPrice24h")]
    pub high_price_24h: Option<String>,
    #[serde(rename = "lowPrice24h")]
    pub low_price_24h: Option<String>,
    #[serde(rename = "prevPrice1h")]
    pub prev_price_1h: Option<String>,
    #[serde(rename = "markPrice")]
    pub mark_price: Option<String>,
    #[serde(rename = "indexPrice")]
    pub index_price: Option<String>,
    #[serde(rename = "openInterest")]
    pub open_interest: Option<String>,
    #[serde(rename = "openInterestValue")]
    pub open_interest_value: Option<String>,
    #[serde(rename = "turnover24h")]
    pub turnover_24h: Option<String>,
    #[serde(rename = "volume24h")]
    pub volume_24h: Option<String>,
    #[serde(rename = "nextFundingTime")]
    pub next_funding_time: Option<String>,
    #[serde(rename = "fundingRate")]
    pub funding_rate: Option<String>,
    #[serde(rename = "bid1Price")]
    pub bid1_price: Option<String>,
    #[serde(rename = "bid1Size")]
    pub bid1_size: Option<String>,
    #[serde(rename = "ask1Price")]
    pub ask1_price: Option<String>,
    #[serde(rename = "ask1Size")]
    pub ask1_size: Option<String>,
    #[serde(rename = "bidPrice")]
    pub bid_price: Option<String>,
    #[serde(rename = "bidSize")]
    pub bid_size: Option<String>,
    #[serde(rename = "askPrice")]
    pub ask_price: Option<String>,
    #[serde(rename = "askSize")]
    pub ask_size: Option<String>,
    #[serde(rename = "basisRate")]
    pub basis_rate: Option<String>,
    #[serde(rename = "deliveryFeeRate")]
    pub delivery_fee_rate: Option<String>,
    #[serde(rename = "predictedDeliveryPrice")]
    pub predicted_delivery_price: Option<String>,
    #[serde(rename = "preOpenPrice")]
    pub pre_open_price: Option<String>,
    #[serde(rename = "preQty")]
    pub pre_qty: Option<String>,
    #[serde(rename = "curPreListingPhase")]
    pub cur_pre_listing_phase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BybitTickerPayload {
    Single(BybitTicker),
    Multiple(Vec<BybitTicker>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BybitMessage {
    Ticker {
        topic: String,
        ts: u64,
        #[serde(rename = "type")]
        message_type: String,
        data: BybitTickerPayload,
        #[serde(default)]
        cs: Option<u64>,
    },
    Subscription {
        success: bool,
        #[serde(rename = "ret_msg")]
        ret_msg: String,
    },
}

impl BybitTickerPayload {
    pub fn into_vec(self) -> Vec<BybitTicker> {
        match self {
            BybitTickerPayload::Single(ticker) => vec![ticker],
            BybitTickerPayload::Multiple(list) => list,
        }
    }
}
