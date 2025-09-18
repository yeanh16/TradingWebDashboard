#[cfg(test)]
mod bybit_subscription_tests {
    use crate::{types::BybitMessage, BybitAdapter};
    use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, MarketType, Symbol};
    use crypto_dash_exchanges_common::ExchangeAdapter;

    #[tokio::test]
    async fn test_bybit_subscription_with_broken_connection() {
        // Create a Bybit adapter with no WebSocket connection (simulating broken connection)
        let adapter = BybitAdapter::new();

        // Create test channels
        let channels = vec![Channel {
            channel_type: ChannelType::Ticker,
            exchange: ExchangeId::from("bybit"),
            market_type: MarketType::Spot,
            symbol: Symbol::new("BTC", "USDT"),
            depth: None,
        }];

        // Test subscription with no connection (should not fail)
        let result = adapter.subscribe(&channels).await;

        // Should succeed (gracefully handles missing connection)
        assert!(
            result.is_ok(),
            "Subscription should succeed even with broken connection"
        );
    }

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
            BybitMessage::Ticker {
                topic,
                ts,
                message_type,
                data,
                ..
            } => {
                assert_eq!(topic, "tickers.SOLUSDT_SOL/USDT");
                assert_eq!(ts, 1744168585009);
                assert_eq!(message_type, "snapshot");

                let mut tickers = data.into_vec();
                assert_eq!(tickers.len(), 1);
                let ticker = tickers.remove(0);

                assert_eq!(ticker.symbol, "SOLUSDT_SOL/USDT");
                assert_eq!(ticker.last_price, "21.8182");
                assert_eq!(ticker.bid_price.as_deref(), Some("20.3359"));
                assert_eq!(ticker.ask_price.as_deref(), Some(""));
                assert_eq!(ticker.bid_size.as_deref(), Some("1.7"));
                assert_eq!(ticker.ask_size.as_deref(), Some(""));
                assert_eq!(ticker.high_price_24h.as_deref(), Some("24.2356"));
                assert_eq!(ticker.low_price_24h.as_deref(), Some("-3"));
                assert_eq!(ticker.prev_price_24h.as_deref(), Some("22.1468"));
                assert_eq!(ticker.volume_24h.as_deref(), Some("23309.9"));
            }
            _ => panic!("Expected Ticker message"),
        }

        println!("✅ BybitTicker type successfully parses the API message format!");
    }
}
