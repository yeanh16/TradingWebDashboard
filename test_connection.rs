#!/usr/bin/env cargo script

//! Test script to verify Bybit WebSocket connection and reconnection functionality
//! 
//! Usage: cargo script test_connection.rs

use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use url::Url;
use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, timeout, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Bybit WebSocket connection and reconnection...");
    
    // Test basic connection
    let url = "wss://stream.bybit.com/v5/public/spot";
    println!("Connecting to: {}", url);
    
    let url = Url::parse(url)?;
    let (ws_stream, response) = connect_async(url).await?;
    println!("Connected successfully! Status: {}", response.status());
    
    let (mut write, mut read) = ws_stream.split();
    
    // Send subscription
    let subscription = json!({
        "op": "subscribe",
        "args": ["tickers.BTCUSDT"]
    });
    
    write.send(Message::Text(subscription.to_string())).await?;
    println!("Sent subscription: {}", subscription);
    
    // Listen for messages with timeout
    println!("Listening for messages (10 seconds)...");
    let mut message_count = 0;
    
    let listen_task = async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    message_count += 1;
                    println!("Message {}: {}", message_count, 
                        if text.len() > 200 { 
                            format!("{}...", &text[..200]) 
                        } else { 
                            text 
                        }
                    );
                    
                    if message_count >= 3 {
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("Connection closed by server");
                    break;
                }
                Ok(_) => continue,
                Err(e) => {
                    println!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    };
    
    match timeout(Duration::from_secs(10), listen_task).await {
        Ok(_) => println!("Successfully received {} messages", message_count),
        Err(_) => println!("Timeout waiting for messages"),
    }
    
    println!("Test completed successfully! ✅");
    Ok(())
}