
use crypto_dash_exchanges_bybit::types::{BybitMessage, BybitTickerPayload};

#[test]
fn test_bybit_ticker_parsing() {
    // Example message using the newer linear futures schema
    let json_message = r#"{
        "topic": "tickers.BTCUSDT",
        "type": "snapshot",
        "ts": 1673272861686,
        "cs": 24987956059,
        "data": {
            "symbol": "BTCUSDT",
            "tickDirection": "PlusTick",
            "price24hPcnt": "0.017103",
            "lastPrice": "17216.00",
            "prevPrice24h": "16926.50",
            "highPrice24h": "17281.50",
            "lowPrice24h": "16915.00",
            "prevPrice1h": "17238.00",
            "markPrice": "17217.33",
            "indexPrice": "17227.36",
            "openInterest": "68744.761",
            "turnover24h": "1570383121.943499",
            "volume24h": "91705.276",
            "bid1Price": "17215.50",
            "bid1Size": "84.489",
            "ask1Price": "17216.00",
            "ask1Size": "83.020"
        }
    }"#;

    let parsed: BybitMessage = serde_json::from_str(json_message).unwrap();

    match parsed {
        BybitMessage::Ticker { topic, ts, message_type, data, cs } => {
            assert_eq!(topic, "tickers.BTCUSDT");
            assert_eq!(ts, 1673272861686);
            assert_eq!(message_type, "snapshot");
            assert_eq!(cs, Some(24987956059));

            let mut tickers = data.into_vec();
            assert_eq!(tickers.len(), 1);
            let ticker = tickers.remove(0);

            assert_eq!(ticker.symbol, "BTCUSDT");
            assert_eq!(ticker.last_price, "17216.00");
            assert_eq!(ticker.tick_direction.as_deref(), Some("PlusTick"));
            assert_eq!(ticker.price24h_pcnt.as_deref(), Some("0.017103"));
            assert_eq!(ticker.prev_price_24h.as_deref(), Some("16926.50"));
            assert_eq!(ticker.high_price_24h.as_deref(), Some("17281.50"));
            assert_eq!(ticker.low_price_24h.as_deref(), Some("16915.00"));
            assert_eq!(ticker.prev_price_1h.as_deref(), Some("17238.00"));
            assert_eq!(ticker.mark_price.as_deref(), Some("17217.33"));
            assert_eq!(ticker.index_price.as_deref(), Some("17227.36"));
            assert_eq!(ticker.open_interest.as_deref(), Some("68744.761"));
            assert_eq!(ticker.turnover_24h.as_deref(), Some("1570383121.943499"));
            assert_eq!(ticker.volume_24h.as_deref(), Some("91705.276"));
            assert_eq!(ticker.bid1_price.as_deref(), Some("17215.50"));
            assert_eq!(ticker.bid1_size.as_deref(), Some("84.489"));
            assert_eq!(ticker.ask1_price.as_deref(), Some("17216.00"));
            assert_eq!(ticker.ask1_size.as_deref(), Some("83.020"));
        }
        _ => panic!("Expected Ticker message"),
    }
}
