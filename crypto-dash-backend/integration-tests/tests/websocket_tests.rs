use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    routing::get,
    Router,
};
use crypto_dash_core::model::{
    Channel, ChannelType, ClientMessage, ExchangeId, StreamMessage, Symbol,
};
use crypto_dash_integration_tests::create_test_app;
use futures::{SinkExt, StreamExt};
use serde_json;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};

/// Test WebSocket connection lifecycle
#[tokio::test]
async fn test_websocket_connection_lifecycle() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    // Test connection
    let ws_url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Should receive welcome message
    let welcome_msg = timeout(Duration::from_secs(5), ws_stream.next()).await??;
    match welcome_msg {
        Some(TungsteniteMessage::Text(text)) => {
            let parsed: StreamMessage = serde_json::from_str(&text)?;
            match parsed {
                StreamMessage::Info { message } => {
                    assert!(message.contains("Connected to crypto-dash API"));
                }
                _ => panic!("Expected info message, got {:?}", parsed),
            }
        }
        _ => panic!("Expected text message"),
    }

    // Test ping/pong
    let ping_msg = ClientMessage::Ping;
    let ping_text = serde_json::to_string(&ping_msg)?;
    ws_sink.send(TungsteniteMessage::Text(ping_text)).await?;

    let pong_msg = timeout(Duration::from_secs(5), ws_stream.next()).await??;
    match pong_msg {
        Some(TungsteniteMessage::Text(text)) => {
            let parsed: StreamMessage = serde_json::from_str(&text)?;
            match parsed {
                StreamMessage::Info { message } => {
                    assert_eq!(message, "Pong");
                }
                _ => panic!("Expected pong message, got {:?}", parsed),
            }
        }
        _ => panic!("Expected text message"),
    }

    // Close connection
    ws_sink.close().await?;

    Ok(())
}

/// Test subscription/unsubscription workflow
#[tokio::test]
async fn test_subscription_workflow() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let ws_url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Skip welcome message
    let _ = timeout(Duration::from_secs(1), ws_stream.next()).await?;

    // Test subscription
    let channel = Channel {
        channel_type: ChannelType::Ticker,
        exchange: ExchangeId::from("binance"),
        symbol: Symbol::new("BTC", "USDT"),
    };

    let subscribe_msg = ClientMessage::Subscribe {
        channels: vec![channel.clone()],
    };
    let subscribe_text = serde_json::to_string(&subscribe_msg)?;
    ws_sink
        .send(TungsteniteMessage::Text(subscribe_text))
        .await?;

    // Should get subscription confirmation
    let response = timeout(Duration::from_secs(5), ws_stream.next()).await??;
    match response {
        Some(TungsteniteMessage::Text(text)) => {
            let parsed: StreamMessage = serde_json::from_str(&text)?;
            match parsed {
                StreamMessage::Info { message } => {
                    assert!(message.contains("Subscribed to 1 channels"));
                }
                _ => panic!("Expected subscription confirmation, got {:?}", parsed),
            }
        }
        _ => panic!("Expected text message"),
    }

    // Test unsubscription
    let unsubscribe_msg = ClientMessage::Unsubscribe {
        channels: vec![channel],
    };
    let unsubscribe_text = serde_json::to_string(&unsubscribe_msg)?;
    ws_sink
        .send(TungsteniteMessage::Text(unsubscribe_text))
        .await?;

    // Should get unsubscription confirmation
    let response = timeout(Duration::from_secs(5), ws_stream.next()).await??;
    match response {
        Some(TungsteniteMessage::Text(text)) => {
            let parsed: StreamMessage = serde_json::from_str(&text)?;
            match parsed {
                StreamMessage::Info { message } => {
                    assert!(message.contains("Unsubscribed from 1 channels"));
                }
                _ => panic!("Expected unsubscription confirmation, got {:?}", parsed),
            }
        }
        _ => panic!("Expected text message"),
    }

    Ok(())
}

/// Test invalid message handling
#[tokio::test]
async fn test_invalid_message_handling() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let ws_url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Skip welcome message
    let _ = timeout(Duration::from_secs(1), ws_stream.next()).await?;

    // Send invalid JSON
    ws_sink
        .send(TungsteniteMessage::Text("invalid json".to_string()))
        .await?;

    // Should get error message
    let response = timeout(Duration::from_secs(5), ws_stream.next()).await??;
    match response {
        Some(TungsteniteMessage::Text(text)) => {
            let parsed: StreamMessage = serde_json::from_str(&text)?;
            match parsed {
                StreamMessage::Error { message } => {
                    assert!(message.contains("Invalid message format"));
                }
                _ => panic!("Expected error message, got {:?}", parsed),
            }
        }
        _ => panic!("Expected text message"),
    }

    Ok(())
}

/// Test multiple concurrent connections
#[tokio::test]
async fn test_concurrent_connections() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let ws_url = format!("ws://{}/ws", addr);

    // Create multiple connections
    let mut connections = Vec::new();
    for _ in 0..5 {
        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        // Skip welcome message
        let _ = timeout(Duration::from_secs(1), ws_stream.next()).await?;

        connections.push((ws_sink, ws_stream));
    }

    // Send ping to all connections
    for (ref mut sink, _) in &mut connections {
        let ping_msg = ClientMessage::Ping;
        let ping_text = serde_json::to_string(&ping_msg)?;
        sink.send(TungsteniteMessage::Text(ping_text)).await?;
    }

    // Verify all get pong responses
    for (_, ref mut stream) in &mut connections {
        let response = timeout(Duration::from_secs(5), stream.next()).await??;
        match response {
            Some(TungsteniteMessage::Text(text)) => {
                let parsed: StreamMessage = serde_json::from_str(&text)?;
                match parsed {
                    StreamMessage::Info { message } => {
                        assert_eq!(message, "Pong");
                    }
                    _ => panic!("Expected pong message, got {:?}", parsed),
                }
            }
            _ => panic!("Expected text message"),
        }
    }

    Ok(())
}

/// Test WebSocket binary message rejection
#[tokio::test]
async fn test_binary_message_rejection() -> Result<()> {
    let (app, _cleanup) = create_test_app().await?;
    let server = create_test_server(app).await;
    let addr = server.local_addr();

    let ws_url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Skip welcome message
    let _ = timeout(Duration::from_secs(1), ws_stream.next()).await?;

    // Send binary message
    ws_sink
        .send(TungsteniteMessage::Binary(vec![1, 2, 3, 4]))
        .await?;

    // Connection should remain open (binary messages are ignored)
    // Send a ping to verify connection is still active
    let ping_msg = ClientMessage::Ping;
    let ping_text = serde_json::to_string(&ping_msg)?;
    ws_sink.send(TungsteniteMessage::Text(ping_text)).await?;

    let response = timeout(Duration::from_secs(5), ws_stream.next()).await??;
    match response {
        Some(TungsteniteMessage::Text(text)) => {
            let parsed: StreamMessage = serde_json::from_str(&text)?;
            match parsed {
                StreamMessage::Info { message } => {
                    assert_eq!(message, "Pong");
                }
                _ => panic!("Expected pong message, got {:?}", parsed),
            }
        }
        _ => panic!("Expected text message"),
    }

    Ok(())
}

/// Helper to create test server
async fn create_test_server(app: Router) -> tokio::net::TcpListener {
    crypto_dash_integration_tests::create_test_server(app).await
}
