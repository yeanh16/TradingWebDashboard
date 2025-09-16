#[cfg(test)]
mod bybit_subscription_tests {
    use crate::{BybitAdapter, types::BybitMessage};
    use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, Symbol};
    use crypto_dash_exchanges_common::ExchangeAdapter;

    #[tokio::test]
    async fn test_bybit_subscription_with_broken_connection() {
        // Create a Bybit adapter with no WebSocket connection (simulating broken connection)
        let adapter = BybitAdapter::new();

        // Create test channels
        let channels = vec![Channel {
            channel_type: ChannelType::Ticker,
            exchange: ExchangeId::from("bybit"),
            symbol: Symbol::new("BTC", "USDT"),
            depth: None,
        }];

        // Test subscription with no connection (should not fail)
        let result = adapter.subscribe(&channels).await;

        // Should succeed (gracefully handles missing connection)
        assert!(result.is_ok(), "Subscription should succeed even with broken connection");
    }

    #[tokio::test]
    async fn test_bybit_adapter_initial_state() {
        // Test that adapter starts in correct state
        let adapter = BybitAdapter::new();
        
        // Should not be connected initially
        assert!(!adapter.is_connected().await, "Should not be connected initially");
        
        // Adapter should be ready for use
        assert_eq!(adapter.id().as_str(), "bybit", "Exchange ID should be bybit");
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

    #[test]
    fn test_bybit_subscription_success_message() {
        // Test parsing of subscription success response
        let success_message = r#"{
            "success": true,
            "ret_msg": "OK"
        }"#;

        let parsed: BybitMessage = serde_json::from_str(success_message).unwrap();
        match parsed {
            BybitMessage::Subscription { success, ret_msg } => {
                assert!(success, "Subscription should be successful");
                assert_eq!(ret_msg, "OK", "Success message should be OK");
            }
            _ => panic!("Expected Subscription message"),
        }

        println!("✅ Subscription success message parsed correctly!");
    }

    #[test]
    fn test_bybit_subscription_error_message() {
        // Test parsing of subscription error response
        let error_message = r#"{
            "success": false,
            "ret_msg": "Invalid channel"
        }"#;

        let parsed: BybitMessage = serde_json::from_str(error_message).unwrap();
        match parsed {
            BybitMessage::Subscription { success, ret_msg } => {
                assert!(!success, "Subscription should fail");
                assert_eq!(ret_msg, "Invalid channel", "Error message should match");
            }
            _ => panic!("Expected Subscription message"),
        }

        println!("✅ Subscription error message parsed correctly!");
    }
}