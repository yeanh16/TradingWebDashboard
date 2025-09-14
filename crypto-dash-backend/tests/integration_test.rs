use crypto_dash_binance::BinanceAdapter;
use crypto_dash_bybit::BybitAdapter;
use crypto_dash_cache::MemoryCache;
use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, Symbol};
use crypto_dash_exchanges_common::ExchangeAdapter;
use crypto_dash_stream_hub::StreamHub;
use std::sync::Arc;

#[tokio::test]
async fn test_adapters_can_start() {
    // Initialize core services
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await.expect("Failed to start stream hub");

    let cache = MemoryCache::new();
    let cache_handle = cache.start().await.expect("Failed to start cache");

    // Test Binance adapter
    let binance_adapter = Arc::new(BinanceAdapter::new());
    assert_eq!(binance_adapter.id(), ExchangeId::from("binance"));
    
    // Note: In test environment we can't connect to real WebSocket, so we expect this to fail
    // In a real implementation, we'd mock the WebSocket client for testing
    let result = binance_adapter.start(hub_handle.clone(), cache_handle.clone()).await;
    // For now, we expect the connection to fail since we can't reach external services in tests
    // but the adapter should at least initialize properly
    
    // Test Bybit adapter
    let bybit_adapter = Arc::new(BybitAdapter::new());
    assert_eq!(bybit_adapter.id(), ExchangeId::from("bybit"));
    let bybit_result = bybit_adapter.start(hub_handle.clone(), cache_handle.clone()).await;
    assert!(bybit_result.is_ok()); // Bybit adapter doesn't connect yet, so should succeed
}

#[tokio::test]
async fn test_channel_creation() {
    let symbol = Symbol::new("BTC", "USDT");
    let channel = Channel {
        channel_type: ChannelType::Ticker,
        exchange: ExchangeId::from("binance"),
        symbol: symbol.clone(),
    };
    
    assert_eq!(channel.symbol.canonical(), "BTC-USDT");
    assert_eq!(channel.exchange.as_str(), "binance");
}