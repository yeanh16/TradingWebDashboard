// Common test utilities and helpers
use anyhow::Result;
use axum::Router;
use crypto_dash_api::state::AppState;
use crypto_dash_binance::BinanceAdapter;
use crypto_dash_bybit::BybitAdapter;
use crypto_dash_cache::MemoryCache;
use crypto_dash_stream_hub::StreamHub;
use crypto_dash_exchanges_common::ExchangeAdapter;
use std::sync::Arc;
use std::time::Duration;

/// Helper to create test app with all exchanges
pub async fn create_test_app() -> Result<(Router, Box<dyn FnOnce() + Send>)> {
    // Initialize core services
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await?;

    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;

    // Create app state with both adapters
    let mut app_state = AppState::new(hub_handle.clone(), cache_handle.clone());
    
    // Add Binance adapter
    let binance_adapter = Arc::new(BinanceAdapter::new());
    binance_adapter.start(hub_handle.clone(), cache_handle.clone()).await?;
    app_state.add_exchange(binance_adapter);
    
    // Add Bybit adapter
    let bybit_adapter = Arc::new(BybitAdapter::new());
    bybit_adapter.start(hub_handle, cache_handle).await?;
    app_state.add_exchange(bybit_adapter);

    // Create router with all routes
    let app = Router::new()
        .route("/health", axum::routing::get(crypto_dash_api::routes::health))
        .route("/ready", axum::routing::get(crypto_dash_api::routes::ready))
        .route("/api/exchanges", axum::routing::get(crypto_dash_api::routes::list_exchanges))
        .route("/api/symbols", axum::routing::get(crypto_dash_api::routes::list_symbols))
        .route("/ws", axum::routing::get(crypto_dash_api::ws::websocket_handler))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(app_state);

    let cleanup = Box::new(|| {
        // Cleanup code can go here if needed
    });

    Ok((app, cleanup))
}

/// Helper to create test server
pub async fn create_test_server(app: Router) -> tokio::net::TcpListener {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    tokio::net::TcpListener::bind(addr).await.unwrap()
}