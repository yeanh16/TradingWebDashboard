use serde::{Deserialize, Serialize};

/// Binance ticker response (24hr statistics stream)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]

pub struct BinanceTicker {
    #[serde(default)]
    pub e: Option<String>, // event type (e.g. 24hrTicker)
    #[serde(rename = "E", default)]
    pub event_time: Option<i64>,
    #[serde(default)]
    pub s: String, // symbol
    #[serde(default)]
    pub p: Option<String>, // price change
    #[serde(rename = "P", default)]
    pub price_change_percent: Option<String>,
    #[serde(default)]
    pub w: Option<String>, // weighted average price
    #[serde(default)]
    pub x: Option<String>, // first trade price
    #[serde(default)]
    pub c: Option<String>, // last price
    #[serde(rename = "Q", default)]
    pub last_qty: Option<String>,
    #[serde(default)]
    pub b: Option<String>, // Best bid price
    #[serde(rename = "B", default)]
    pub best_bid_qty: Option<String>, // Best bid qty
    #[serde(default)]
    pub a: Option<String>, // Best ask price
    #[serde(rename = "A", default)]
    pub best_ask_qty: Option<String>, // Best ask qty
    #[serde(default)]
    pub o: Option<String>, // open price
    #[serde(default)]
    pub h: Option<String>, // high price
    #[serde(default)]
    pub l: Option<String>, // low price
    #[serde(default)]
    pub v: Option<String>, // base asset volume
    #[serde(default)]
    pub q: Option<String>, // quote asset volume
    #[serde(rename = "O", default)]
    pub statistics_open_time: Option<i64>,
    #[serde(rename = "C", default)]
    pub statistics_close_time: Option<i64>,
    #[serde(rename = "F", default)]
    pub first_trade_id: Option<i64>,
    #[serde(rename = "L", default)]
    pub last_trade_id: Option<i64>,
    #[serde(default)]
    pub n: Option<i64>, // total number of trades
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
    StreamTicker {
        stream: String,
        data: BinanceTicker,
    },
    DirectTicker(BinanceTicker),
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

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_parse_24hr_ticker_message() {
        let raw_message = r#"{"e":"24hrTicker","E":1757888604019,"s":"BTCUSDT","p":"-21.48000000","P":"-0.019","w":"115669.75585612","x":"115853.45000000","c":"115831.96000000","Q":"0.00832000","b":"115831.96000000","B":"0.20337000","a":"115831.97000000","A":"12.85848000","o":"115853.44000000","h":"116165.19000000","l":"115141.80000000","v":"6348.13563000","q":"734287298.46364070","O":1757802204009,"C":1757888604009,"F":5231695487,"L":5232837353,"n":1141867}"#;

        let parsed: BinanceStreamMessage =
            serde_json::from_str(raw_message).expect("Failed to parse 24hr ticker message");

        match parsed {
            BinanceStreamMessage::DirectTicker(ticker) => {
                assert_eq!(ticker.e.as_deref(), Some("24hrTicker"));
                assert_eq!(ticker.s, "BTCUSDT");
                assert_eq!(ticker.c, "115831.96000000");
                assert_eq!(ticker.b, "115831.96000000");
                assert_eq!(ticker.a, "115831.97000000");
                assert_eq!(ticker.event_time, Some(1757888604019));
            }
            _ => panic!("Expected DirectTicker variant"),
        }
    }

    #[test]

    fn test_parse_eth_24hr_ticker_message() {
        let raw_message = r#"{"e":"24hrTicker","E":1757888604019,"s":"ETHUSDT","p":"-41.22000000","P":"-0.885","w":"4629.13557655","x":"4658.65000000","c":"4617.43000000","Q":"0.22000000","b":"4617.42000000","B":"0.00240000","a":"4617.43000000","A":"112.71210000","o":"4658.65000000","h":"4692.36000000","l":"4576.89000000","v":"278119.69690000","q":"1287453783.46015200","O":1757802204004,"C":1757888604004,"F":2846212739,"L":2848733731,"n":2520993}"#;

        let parsed: BinanceStreamMessage =
            serde_json::from_str(raw_message).expect("Failed to parse ETH 24hr ticker message");

        match parsed {
            BinanceStreamMessage::DirectTicker(ticker) => {
                assert_eq!(ticker.e.as_deref(), Some("24hrTicker"));
                assert_eq!(ticker.s, "ETHUSDT");
                assert_eq!(ticker.c, "4617.43000000");
                assert_eq!(ticker.b, "4617.42000000");
                assert_eq!(ticker.a, "4617.43000000");
                assert_eq!(ticker.event_time, Some(1757888604019));
            }
            _ => panic!("Expected DirectTicker variant"),
        }
    }

    #[test]

    fn test_parse_regular_ticker_message() {
        let raw_message = r#"{"stream":"btcusdt@ticker","data":{"s":"BTCUSDT","c":"50000.00","b":"49999.00","a":"50001.00","B":"1.0","A":"2.0","E":1234567890}}"#;

        let parsed: BinanceStreamMessage =
            serde_json::from_str(raw_message).expect("Failed to parse regular ticker message");

        match parsed {
            BinanceStreamMessage::StreamTicker { stream, data } => {
                assert_eq!(stream, "btcusdt@ticker");
                assert_eq!(data.s, "BTCUSDT");
                assert_eq!(data.c, "50000.00");
                assert_eq!(data.event_time, Some(1234567890));
            }
            _ => panic!("Expected StreamTicker variant"),
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

        // Verify they are parsed as DirectTicker

        match btc_parsed {
            BinanceStreamMessage::DirectTicker(ticker) => {
                assert_eq!(ticker.e.as_deref(), Some("24hrTicker"));

                assert_eq!(ticker.s, "BTCUSDT");

                assert_eq!(ticker.c, "115831.96000000");
            }

            _ => panic!("Expected DirectTicker variant for BTC"),
        }

        match eth_parsed {
            BinanceStreamMessage::DirectTicker(ticker) => {
                assert_eq!(ticker.e.as_deref(), Some("24hrTicker"));

                assert_eq!(ticker.s, "ETHUSDT");

                assert_eq!(ticker.c, "4617.43000000");
            }

            _ => panic!("Expected DirectTicker variant for ETH"),
        }
    }
}
