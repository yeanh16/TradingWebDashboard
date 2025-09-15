#[cfg(test)]
mod bybit_routing_tests {
    use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, Symbol, ClientMessage};
    use std::collections::HashMap;

    #[test]
    fn test_bybit_channel_exchange_id() {
        // Create a test channel for Bybit
        let channel = Channel {
            channel_type: ChannelType::Ticker,
            exchange: ExchangeId::from("bybit"),
            symbol: Symbol::new("BTC", "USDT"),
            depth: None,
        };

        // Test the exchange ID string conversion
        let exchange_id = channel.exchange.as_str().to_string();
        assert_eq!(exchange_id, "bybit");
        
        // Test serialization/deserialization
        let client_message = ClientMessage::Subscribe {
            channels: vec![channel.clone()]
        };

        let json = serde_json::to_string(&client_message).expect("Failed to serialize");
        println!("JSON: {}", json);
        
        let deserialized: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");
        
        match deserialized {
            ClientMessage::Subscribe { channels } => {
                assert_eq!(channels.len(), 1);
                let ch = &channels[0];
                assert_eq!(ch.exchange.as_str(), "bybit");
                assert_eq!(ch.symbol.base, "BTC");
                assert_eq!(ch.symbol.quote, "USDT");
            }
            _ => panic!("Expected Subscribe message"),
        }
    }

    #[test] 
    fn test_exchange_lookup_simulation() {
        // Simulate the exchange registration process
        let mut exchanges: HashMap<String, String> = HashMap::new();
        
        // Simulate how Bybit adapter gets registered
        let bybit_adapter_id = ExchangeId::from("bybit");
        let id = bybit_adapter_id.as_str().to_string();
        exchanges.insert(id.clone(), "bybit_adapter".to_string());
        
        println!("Registered exchanges: {:?}", exchanges.keys().collect::<Vec<_>>());
        
        // Simulate the lookup process
        let channel = Channel {
            channel_type: ChannelType::Ticker,
            exchange: ExchangeId::from("bybit"),
            symbol: Symbol::new("BTC", "USDT"),
            depth: None,
        };
        
        let exchange_id = channel.exchange.as_str().to_string();
        println!("Looking up exchange: '{}'", exchange_id);
        
        assert!(exchanges.contains_key(&exchange_id), "Exchange 'bybit' should be found");
    }
}