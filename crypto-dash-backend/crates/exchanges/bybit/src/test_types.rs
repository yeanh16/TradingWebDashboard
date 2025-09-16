use crypto_dash_exchanges_bybit::types::{BybitMessage, BybitTicker};

#[test]
fn test_bybit_ticker_parsing() {
    // This is the example message from the user
    let json_message = r#"{
    "topic": "tickers.SOLUSDT_SOL/USDT",
    "ts": 1744168585009,
    "type": "snapshot",
    "data": {
        "symbol": "SOLUSDT_SOL/USDT",
        "bidPrice": "20.3359",
        "bidSize": "1.7",
        "askPrice": "",
        "askSize": "",
        "lastPrice": "21.8182",
        "highPrice24h": "24.2356",
        "lowPrice24h": "-3",
        "prevPrice24h": "22.1468",
        "volume24h": "23309.9"
        }
    }"#;

    // Parse the message
    let parsed: BybitMessage = serde_json::from_str(json_message).unwrap();

    match parsed {
        BybitMessage::Ticker { topic, ts, message_type, data } => {
            assert_eq!(topic, "tickers.SOLUSDT_SOL/USDT");
            assert_eq!(ts, 1744168585009);
            assert_eq!(message_type, "snapshot");

            // Check the ticker data
            assert_eq!(data.symbol, "SOLUSDT_SOL/USDT");
            assert_eq!(data.last_price, "21.8182");
            assert_eq!(data.bid_price, "20.3359");
            assert_eq!(data.ask_price, "");
            assert_eq!(data.bid_size, "1.7");
            assert_eq!(data.ask_size, "");
            assert_eq!(data.high_price_24h, "24.2356");
            assert_eq!(data.low_price_24h, "-3");
            assert_eq!(data.prev_price_24h, "22.1468");
            assert_eq!(data.volume_24h, "23309.9");
        }
        _ => panic!("Expected Ticker message"),
    }

    println!("✅ BybitTicker type successfully parses the API message format!");
}
