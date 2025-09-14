use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::get,
    Router,
};
use crypto_dash_core::model::{ExchangeInfo, SymbolResponse};
use reqwest;
use serde_json::Value;
use std::time::Duration;
use crypto_dash_integration_tests::create_test_app;

/// Test health endpoint
#[tokio::test]
async fn test_health_endpoint() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/health", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await?;
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].is_string());

    Ok(())
}

/// Test readiness endpoint
#[tokio::test]
async fn test_readiness_endpoint() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/ready", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await?;
    assert_eq!(body["status"], "ready");
    assert!(body["timestamp"].is_string());

    Ok(())
}

/// Test exchanges endpoint
#[tokio::test]
async fn test_exchanges_endpoint() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/exchanges", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: Value = response.json().await?;
    assert!(body["exchanges"].is_array());
    
    let exchanges = body["exchanges"].as_array().unwrap();
    assert!(!exchanges.is_empty());
    
    // Check that we have our test exchanges
    let exchange_ids: Vec<String> = exchanges.iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    
    assert!(exchange_ids.contains(&"binance".to_string()));
    assert!(exchange_ids.contains(&"bybit".to_string()));

    // Validate exchange structure
    for exchange in exchanges {
        assert!(exchange["id"].is_string());
        assert!(exchange["name"].is_string());
        assert!(exchange["status"].is_string());
    }

    Ok(())
}

/// Test symbols endpoint with valid exchange
#[tokio::test]
async fn test_symbols_endpoint_valid_exchange() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/symbols?exchange=binance", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let body: SymbolResponse = response.json().await?;
    assert_eq!(body.exchange, "binance");
    assert!(!body.symbols.is_empty());

    // Validate symbol structure
    for symbol in &body.symbols {
        assert!(!symbol.symbol.is_empty());
        assert!(!symbol.base.is_empty());
        assert!(!symbol.quote.is_empty());
        assert!(!symbol.display_name.is_empty());
    }

    Ok(())
}

/// Test symbols endpoint with invalid exchange
#[tokio::test]
async fn test_symbols_endpoint_invalid_exchange() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/symbols?exchange=nonexistent", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

/// Test symbols endpoint without exchange parameter
#[tokio::test]
async fn test_symbols_endpoint_missing_exchange() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/symbols", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

/// Test CORS headers
#[tokio::test]
async fn test_cors_headers() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .options(&format!("http://{}/api/exchanges", addr))
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert!(headers.contains_key("access-control-allow-methods"));

    Ok(())
}

/// Test rate limiting and error handling
#[tokio::test]
async fn test_concurrent_requests_handling() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    
    // Send multiple concurrent requests
    let mut handles = Vec::new();
    for _ in 0..10 {
        let client = client.clone();
        let url = format!("http://{}/health", addr);
        let handle = tokio::spawn(async move {
            client.get(&url).send().await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await??;
        assert_eq!(response.status(), StatusCode::OK);
    }

    Ok(())
}

/// Test malformed requests
#[tokio::test]
async fn test_malformed_requests() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();

    // Test invalid path
    let response = client
        .get(&format!("http://{}/nonexistent", addr))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Test invalid query parameters
    let response = client
        .get(&format!("http://{}/api/symbols?exchange=&invalid=value", addr))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

/// Test request timeout handling
#[tokio::test]
async fn test_request_timeout() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1)) // Very short timeout
        .build()?;

    // This should timeout quickly and demonstrate timeout handling
    let result = client
        .get(&format!("http://{}/health", addr))
        .send()
        .await;

    // The request might succeed if it's very fast, or timeout
    // Both outcomes are acceptable for this test
    match result {
        Ok(response) => {
            // If it succeeds, verify it's a valid response
            assert_eq!(response.status(), StatusCode::OK);
        }
        Err(e) => {
            // If it fails, it should be a timeout error
            assert!(e.is_timeout());
        }
    }

    Ok(())
}

/// Test content-type headers
#[tokio::test]
async fn test_content_type_headers() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://{}/api/exchanges", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str()?.contains("application/json"));

    Ok(())
}

/// Test large response handling
#[tokio::test]
async fn test_large_response_handling() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let client = reqwest::Client::new();
    
    // Request symbols which might be a large response
    let response = client
        .get(&format!("http://{}/api/symbols?exchange=binance", addr))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify we can parse the full response
    let _body: SymbolResponse = response.json().await?;
    
    Ok(())
}

/// Helper to create test server
async fn create_test_server(app: Router) -> tokio::net::TcpListener {
    crypto_dash_integration_tests::create_test_server(app).await
}