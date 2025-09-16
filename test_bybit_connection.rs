use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use url::Url;
use anyhow::Result;
use serde_json::json;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Bybit WebSocket connection...");
    
    // Test basic connection to Bybit WebSocket
    let url = "wss://stream.bybit.com/v5/public/spot";
    println!("Connecting to: {}", url);
    
    let url = Url::parse(url)?;
    let (ws_stream, response) = connect_async(url).await?;
    println!("Connected successfully! Status: {}", response.status());
    
    let (mut write, mut read) = ws_stream.split();
    
    // Send subscription for BTCUSDT ticker
    let subscription = json!({
        "op": "subscribe",
        "args": ["tickers.BTCUSDT"]
    });
    
    write.send(Message::Text(subscription.to_string())).await?;
    println!("Sent subscription: {}", subscription);
    
    // Listen for a few messages to verify connection works
    println!("Listening for messages (5 seconds)...");
    let mut message_count = 0;
    
    let listen_task = async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    message_count += 1;
                    let display_text = if text.len() > 150 { 
                        format!("{}...", &text[..150]) 
                    } else { 
                        text 
                    };
                    println!("Message {}: {}", message_count, display_text);
                    
                    if message_count >= 2 {
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
    
    match timeout(Duration::from_secs(5), listen_task).await {
        Ok(_) => println!("✅ Successfully received {} messages", message_count),
        Err(_) => println!("⚠️ Timeout waiting for messages"),
    }
    
    println!("✅ Bybit WebSocket test completed!");
    Ok(())
}