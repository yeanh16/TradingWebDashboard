mod routes;
mod state;
mod ws;

use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use crypto_dash_binance::BinanceAdapter;
use crypto_dash_bybit::BybitAdapter;
use crypto_dash_cache::MemoryCache;
use crypto_dash_core::config::Config;
use crypto_dash_exchanges_common::ExchangeAdapter;
use crypto_dash_stream_hub::StreamHub;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "crypto_dash=debug,tower_http=debug,axum::rejection=trace".into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    info!("Starting crypto-dash API server on {}", config.bind_addr);
    info!("Enabled exchanges: {:?}", config.exchanges);

    // Initialize core services
    let stream_hub = StreamHub::new();
    let hub_handle = stream_hub.start().await?;

    let cache = MemoryCache::new();
    let cache_handle = cache.start().await?;

    // Create application state
    let mut app_state = AppState::new(hub_handle.clone(), cache_handle.clone());

    // Initialize exchange adapters
    for exchange_name in &config.exchanges {
        match exchange_name.as_str() {
            "binance" => {
                let adapter = Arc::new(BinanceAdapter::new());
                adapter.start(hub_handle.clone(), cache_handle.clone()).await?;
                app_state.add_exchange(adapter);
                info!("Initialized Binance adapter");
            }
            "bybit" => {
                let adapter = Arc::new(BybitAdapter::new());
                adapter.start(hub_handle.clone(), cache_handle.clone()).await?;
                app_state.add_exchange(adapter);
                info!("Initialized Bybit adapter");
            }
            _ => {
                tracing::warn!("Unknown exchange: {}", exchange_name);
            }
        }
    }

    // Build the application router
    let app = Router::new()
        // Health endpoints
        .route("/health", get(routes::health))
        .route("/ready", get(routes::ready))
        // API routes
        .route("/api/exchanges", get(routes::list_exchanges))
        // WebSocket endpoint
        .route("/ws", get(ws::websocket_handler))
        // Add middleware
        .layer(CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!("Server listening on {}", config.bind_addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}