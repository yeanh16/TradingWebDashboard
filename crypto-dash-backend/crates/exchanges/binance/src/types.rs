use serde::{Deserialize, Serialize};

/// Binance ticker response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
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

/// Binance 24hr ticker statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Binance24hrTicker {
    pub e: String, // event type (should be "24hrTicker")
    #[serde(rename = "E")]
    pub event_time: i64,
    pub s: String, // symbol
    pub p: String, // price change
    #[serde(rename = "P")]
    pub price_change_percent: String,
    pub w: String, // weighted average price
    pub x: String, // first trade (F)-1 price (close price of the day before)
    pub c: String, // last price
    #[serde(rename = "Q")]
    pub last_qty: String,
    pub b: String, // best bid price
    #[serde(rename = "B")]
    pub best_bid_qty: String,
    pub a: String, // best ask price
    #[serde(rename = "A")]
    pub best_ask_qty: String,
    pub o: String, // open price
    pub h: String, // high price
    pub l: String, // low price
    pub v: String, // total traded base asset volume
    pub q: String, // total traded quote asset volume
    #[serde(rename = "O")]
    pub statistics_open_time: i64,
    #[serde(rename = "C")]
    pub statistics_close_time: i64,
    #[serde(rename = "F")]
    pub first_trade_id: i64,
    #[serde(rename = "L")]
    pub last_trade_id: i64,
    pub n: i64, // total number of trades
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
    DirectTicker24hr(Binance24hrTicker),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_24hr_ticker_message() {
        let raw_message = r#"{"e":"24hrTicker","E":1757888604019,"s":"BTCUSDT","p":"-21.48000000","P":"-0.019","w":"115669.75585612","x":"115853.45000000","c":"115831.96000000","Q":"0.00832000","b":"115831.96000000","B":"0.20337000","a":"115831.97000000","A":"12.85848000","o":"115853.44000000","h":"116165.19000000","l":"115141.80000000","v":"6348.13563000","q":"734287298.46364070","O":1757802204009,"C":1757888604009,"F":5231695487,"L":5232837353,"n":1141867}"#;

        let parsed: BinanceStreamMessage =
            serde_json::from_str(raw_message).expect("Failed to parse 24hr ticker message");

        match parsed {
            BinanceStreamMessage::DirectTicker24hr(ticker) => {
                assert_eq!(ticker.e, "24hrTicker");
                assert_eq!(ticker.s, "BTCUSDT");
                assert_eq!(ticker.c, "115831.96000000");
                assert_eq!(ticker.b, "115831.96000000");
                assert_eq!(ticker.a, "115831.97000000");
                assert_eq!(ticker.event_time, 1757888604019);
            }
            _ => panic!("Expected DirectTicker24hr variant"),
        }
    }

    #[test]
    fn test_parse_eth_24hr_ticker_message() {
        let raw_message = r#"{"e":"24hrTicker","E":1757888604019,"s":"ETHUSDT","p":"-41.22000000","P":"-0.885","w":"4629.13557655","x":"4658.65000000","c":"4617.43000000","Q":"0.22000000","b":"4617.42000000","B":"0.00240000","a":"4617.43000000","A":"112.71210000","o":"4658.65000000","h":"4692.36000000","l":"4576.89000000","v":"278119.69690000","q":"1287453783.46015200","O":1757802204004,"C":1757888604004,"F":2846212739,"L":2848733731,"n":2520993}"#;

        let parsed: BinanceStreamMessage =
            serde_json::from_str(raw_message).expect("Failed to parse ETH 24hr ticker message");

        match parsed {
            BinanceStreamMessage::DirectTicker24hr(ticker) => {
                assert_eq!(ticker.e, "24hrTicker");
                assert_eq!(ticker.s, "ETHUSDT");
                assert_eq!(ticker.c, "4617.43000000");
                assert_eq!(ticker.b, "4617.42000000");
                assert_eq!(ticker.a, "4617.43000000");
                assert_eq!(ticker.event_time, 1757888604019);
            }
            _ => panic!("Expected DirectTicker24hr variant"),
        }
    }

    #[test]
    fn test_parse_regular_ticker_message() {
        let raw_message = r#"{"stream":"btcusdt@ticker","data":{"s":"BTCUSDT","c":"50000.00","b":"49999.00","a":"50001.00","B":"1.0","A":"2.0","E":1234567890}}"#;

        let parsed: BinanceStreamMessage =
            serde_json::from_str(raw_message).expect("Failed to parse regular ticker message");

        match parsed {
            BinanceStreamMessage::Ticker { stream, data } => {
                assert_eq!(stream, "btcusdt@ticker");
                assert_eq!(data.s, "BTCUSDT");
                assert_eq!(data.c, "50000.00");
            }
            _ => panic!("Expected Ticker variant"),
        }
    }

    #[test]
    fn test_original_error_messages() {
        // Test the exact messages from the original error log
        let btc_message = r#"{"e":"24hrTicker","E":1757888604019,"s":"BTCUSDT","p":"-21.48000000","P":"-0.019","w":"115669.75585612","x":"115853.45000000","c":"115831.96000000","Q":"0.00832000","b":"115831.96000000","B":"0.20337000","a":"115831.97000000","A":"12.85848000","o":"115853.44000000","h":"116165.19000000","l":"115141.80000000","v":"6348.13563000","q":"734287298.46364070","O":1757802204009,"C":1757888604009,"F":5231695487,"L":5232837353,"n":1141867}"#;

        let eth_message = r#"{"e":"24hrTicker","E":1757888604019,"s":"ETHUSDT","p":"-41.22000000","P":"-0.885","w":"4629.13557655","x":"4658.65000000","c":"4617.43000000","Q":"0.22000000","b":"4617.42000000","B":"0.00240000","a":"4617.43000000","A":"112.71210000","o":"4658.65000000","h":"4692.36000000","l":"4576.89000000","v":"278119.69690000","q":"1287453783.46015200","O":1757802204004,"C":1757888604004,"F":2846212739,"L":2848733731,"n":2520993}"#;

        // Both messages should now parse successfully
        let btc_parsed: BinanceStreamMessage = serde_json::from_str(btc_message)
            .expect("Failed to parse BTC 24hr ticker from original error");
        let eth_parsed: BinanceStreamMessage = serde_json::from_str(eth_message)
            .expect("Failed to parse ETH 24hr ticker from original error");

        // Verify they are parsed as DirectTicker24hr
        match btc_parsed {
            BinanceStreamMessage::DirectTicker24hr(ticker) => {
                assert_eq!(ticker.e, "24hrTicker");
                assert_eq!(ticker.s, "BTCUSDT");
                assert_eq!(ticker.c, "115831.96000000");
            }
            _ => panic!("Expected DirectTicker24hr variant for BTC"),
        }

        match eth_parsed {
            BinanceStreamMessage::DirectTicker24hr(ticker) => {
                assert_eq!(ticker.e, "24hrTicker");
                assert_eq!(ticker.s, "ETHUSDT");
                assert_eq!(ticker.c, "4617.43000000");
            }
            _ => panic!("Expected DirectTicker24hr variant for ETH"),
        }
    }
}
