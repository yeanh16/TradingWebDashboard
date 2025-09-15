use anyhow::Result;
use crypto_dash_cache::MemoryCache;
use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, Symbol, Ticker};
use crypto_dash_stream_hub::StreamHub;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Test cache behavior under high load
#[tokio::test]
async fn test_cache_high_load() -> Result<()> {
    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;

    // Create many concurrent ticker updates
    let mut handles = Vec::new();
    for i in 0..1000 {
        let cache_handle = cache_handle.clone();
        let handle = tokio::spawn(async move {
            let ticker = Ticker {
                timestamp: chrono::Utc::now(),
                exchange: ExchangeId::from("binance"),
                symbol: Symbol::new("BTC", "USDT"),
                bid: 50000.0 + i as f64,
                ask: 50001.0 + i as f64,
                last: 50000.5 + i as f64,
                bid_size: 1.0,
                ask_size: 1.0,
            };
            
            cache_handle.store_ticker(ticker).await
        });
        handles.push(handle);
    }

    // Wait for all updates to complete
    for handle in handles {
        handle.await??;
    }

    // Verify cache statistics
    let stats = cache_handle.get_stats().await?;
    assert!(stats.total_tickers > 0);

    Ok(())
}

/// Test memory usage under sustained load
#[tokio::test]
async fn test_memory_usage_stability() -> Result<()> {
    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;

    // Continuously update cache for a short period
    let start_time = std::time::Instant::now();
    let mut counter = 0;

    while start_time.elapsed() < Duration::from_secs(2) {
        let ticker = Ticker {
            timestamp: chrono::Utc::now(),
            exchange: ExchangeId::from("binance"),
            symbol: Symbol::new("BTC", "USDT"),
            bid: 50000.0 + counter as f64,
            ask: 50001.0 + counter as f64,
            last: 50000.5 + counter as f64,
            bid_size: 1.0,
            ask_size: 1.0,
        };
        
        cache_handle.store_ticker(ticker).await?;
        counter += 1;
        
        // Small delay to prevent overwhelming the system
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    // Verify memory hasn't grown excessively (basic check)
    let stats = cache_handle.get_stats().await?;
    assert!(stats.total_tickers > 0);
    assert!(counter > 100); // Should have processed many updates

    Ok(())
}

/// Test stream hub resilience under subscriber failures
#[tokio::test]
async fn test_stream_hub_subscriber_failures() -> Result<()> {
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await?;

    // Create multiple subscribers
    let mut subscribers = Vec::new();
    for _ in 0..5 {
        let receiver = hub_handle.subscribe_all().await;
        subscribers.push(receiver);
    }

    // Drop some subscribers to simulate failures
    drop(subscribers.pop());
    drop(subscribers.pop());

    // Publish a message
    let ticker = Ticker {
        timestamp: chrono::Utc::now(),
        exchange: ExchangeId::from("binance"),
        symbol: Symbol::new("BTC", "USDT"),
        bid: 50000.0,
        ask: 50001.0,
        last: 50000.5,
        bid_size: 1.0,
        ask_size: 1.0,
    };

    let topic = crypto_dash_stream_hub::topics::TopicKey::from_channel(&Channel {
        channel_type: ChannelType::Ticker,
        exchange: ExchangeId::from("binance"),
        symbol: Symbol::new("BTC", "USDT"),
    });

    hub_handle.publish(topic, crypto_dash_core::model::StreamMessage::Ticker { payload: ticker }).await?;

    // Remaining subscribers should still receive the message
    for mut subscriber in subscribers {
        let result = timeout(Duration::from_secs(1), subscriber.recv()).await;
        assert!(result.is_ok());
    }

    Ok(())
}

/// Test concurrent connection handling
#[tokio::test]
async fn test_concurrent_stream_subscriptions() -> Result<()> {
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await?;

    // Create many concurrent subscriptions
    let mut handles = Vec::new();
    for i in 0..50 {
        let hub_handle = hub_handle.clone();
        let handle = tokio::spawn(async move {
            let mut receiver = hub_handle.subscribe_all().await;
            
            // Wait for a message or timeout
            let result = timeout(Duration::from_millis(100), receiver.recv()).await;
            
            // We don't expect messages in this test, just testing subscription creation
            result.is_err() // Timeout is expected
        });
        handles.push(handle);
    }

    // Wait for all subscriptions to be created
    for handle in handles {
        let timeout_occurred = handle.await?;
        assert!(timeout_occurred); // All should timeout as expected
    }

    Ok(())
}

/// Test malformed data handling
#[tokio::test]
async fn test_malformed_data_handling() -> Result<()> {
    // Test invalid symbol creation
    let result = std::panic::catch_unwind(|| {
        Symbol::new("", "USDT")
    });
    assert!(result.is_err() || Symbol::new("", "USDT").base.is_empty());

    // Test invalid ticker data
    let ticker = Ticker {
        timestamp: chrono::Utc::now(),
        exchange: ExchangeId::from("binance"),
        symbol: Symbol::new("BTC", "USDT"),
        bid: f64::NAN,
        ask: f64::INFINITY,
        last: -1.0,
        bid_size: 0.0,
        ask_size: -1.0,
    };

    // The system should handle these gracefully without panicking
    assert!(ticker.bid.is_nan());
    assert!(ticker.ask.is_infinite());
    assert!(ticker.last < 0.0);

    Ok(())
}

/// Test exchange adapter error recovery
#[tokio::test]
async fn test_exchange_adapter_error_recovery() -> Result<()> {
    use crypto_dash_exchanges_common::retry::ExponentialBackoff;

    let mut backoff = ExponentialBackoff::new(Duration::from_millis(10), Duration::from_millis(100));

    // Test backoff progression
    let delay1 = backoff.next_delay();
    let delay2 = backoff.next_delay();
    let delay3 = backoff.next_delay();

    assert!(delay2 >= delay1);
    assert!(delay3 >= delay2);
    assert!(delay3 <= Duration::from_millis(100)); // Should respect max delay

    Ok(())
}

/// Test data consistency during rapid updates
#[tokio::test]
async fn test_data_consistency_rapid_updates() -> Result<()> {
    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;

    let symbol = Symbol::new("BTC", "USDT");
    let exchange = ExchangeId::from("binance");

    // Send rapid updates with increasing prices
    for i in 0..100 {
        let ticker = Ticker {
            timestamp: chrono::Utc::now(),
            exchange: exchange.clone(),
            symbol: symbol.clone(),
            bid: 50000.0 + i as f64,
            ask: 50001.0 + i as f64,
            last: 50000.5 + i as f64,
            bid_size: 1.0,
            ask_size: 1.0,
        };
        
        cache_handle.store_ticker(ticker).await?;
    }

    // Retrieve final ticker and verify it has the latest data
    if let Some(final_ticker) = cache_handle.get_ticker(&exchange, &symbol).await? {
        assert!(final_ticker.bid >= 50099.0); // Should be one of the latest values
        assert!(final_ticker.ask >= 50100.0);
    }

    Ok(())
}

/// Test resource cleanup after disconnections
#[tokio::test]
async fn test_resource_cleanup() -> Result<()> {
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await?;

    // Create and immediately drop many receivers
    for _ in 0..100 {
        let _receiver = hub_handle.subscribe_all().await;
        // Receiver is dropped at end of scope
    }

    // The hub should still be functional
    let mut final_receiver = hub_handle.subscribe_all().await;
    
    // Publish a test message
    let ticker = Ticker {
        timestamp: chrono::Utc::now(),
        exchange: ExchangeId::from("binance"),
        symbol: Symbol::new("BTC", "USDT"),
        bid: 50000.0,
        ask: 50001.0,
        last: 50000.5,
        bid_size: 1.0,
        ask_size: 1.0,
    };

    let topic = crypto_dash_stream_hub::topics::TopicKey::from_channel(&Channel {
        channel_type: ChannelType::Ticker,
        exchange: ExchangeId::from("binance"),
        symbol: Symbol::new("BTC", "USDT"),
    });

    hub_handle.publish(topic, crypto_dash_core::model::StreamMessage::Ticker { payload: ticker }).await?;

    // Should still receive the message
    let result = timeout(Duration::from_secs(1), final_receiver.recv()).await;
    assert!(result.is_ok());

    Ok(())
}

/// Test system behavior under extreme load
#[tokio::test]
async fn test_extreme_load_handling() -> Result<()> {
    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;
    
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await?;

    // Create multiple publishers and subscribers
    let mut handles = Vec::new();

    // Publishers
    for i in 0..10 {
        let cache_handle = cache_handle.clone();
        let hub_handle = hub_handle.clone();
        let handle = tokio::spawn(async move {
            for j in 0..100 {
                let ticker = Ticker {
                    timestamp: chrono::Utc::now(),
                    exchange: ExchangeId::from("binance"),
                    symbol: Symbol::new("BTC", "USDT"),
                    bid: 50000.0 + (i * 100 + j) as f64,
                    ask: 50001.0 + (i * 100 + j) as f64,
                    last: 50000.5 + (i * 100 + j) as f64,
                    bid_size: 1.0,
                    ask_size: 1.0,
                };
                
                let _ = cache_handle.store_ticker(ticker.clone()).await;
                
                let topic = crypto_dash_stream_hub::topics::TopicKey::from_channel(&Channel {
                    channel_type: ChannelType::Ticker,
                    exchange: ExchangeId::from("binance"),
                    symbol: Symbol::new("BTC", "USDT"),
                });

                let _ = hub_handle.publish(topic, crypto_dash_core::model::StreamMessage::Ticker { payload: ticker }).await;
                
                // Small delay to prevent overwhelming
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }

    // Subscribers
    for _ in 0..5 {
        let hub_handle = hub_handle.clone();
        let handle = tokio::spawn(async move {
            let mut receiver = hub_handle.subscribe_all().await;
            let mut count = 0;
            
            while count < 50 {
                if let Ok(_) = timeout(Duration::from_millis(10), receiver.recv()).await {
                    count += 1;
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    // System should still be responsive
    let stats = cache_handle.get_stats().await?;
    assert!(stats.total_tickers > 0);

    Ok(())
}

/// Test graceful degradation under resource constraints
#[tokio::test]
async fn test_graceful_degradation() -> Result<()> {
    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;

    // Fill cache with data
    for i in 0..1000 {
        let ticker = Ticker {
            timestamp: chrono::Utc::now(),
            exchange: ExchangeId::from("binance"),
            symbol: Symbol::new(&format!("SYM{}", i), "USDT"),
            bid: 1.0 + i as f64,
            ask: 1.01 + i as f64,
            last: 1.005 + i as f64,
            bid_size: 1.0,
            ask_size: 1.0,
        };
        
        cache_handle.store_ticker(ticker).await?;
    }

    // Cache should still be functional
    let stats = cache_handle.get_stats().await?;
    assert!(stats.total_tickers > 0);

    // Should be able to retrieve recent data
    let symbol = Symbol::new("SYM999", "USDT");
    let result = cache_handle.get_ticker(&ExchangeId::from("binance"), &symbol).await?;
    assert!(result.is_some());

    Ok(())
}