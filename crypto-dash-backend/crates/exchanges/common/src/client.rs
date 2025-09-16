use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures::{SinkExt, StreamExt};
use url::Url;
use anyhow::{Result, anyhow};
use tracing::{debug, error, warn, info};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket client helper with reconnection support
pub struct WsClient {
    url: String,
    stream: Option<WsStream>,
    last_ping: Option<Instant>,
    ping_interval: Duration,
    connection_timeout: Duration,
}

impl WsClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            stream: None,
            last_ping: None,
            ping_interval: Duration::from_secs(20), // Ping every 20 seconds
            connection_timeout: Duration::from_secs(60), // Consider connection dead after 60 seconds
        }
    }

    /// Create a new WebSocket client with custom ping interval and timeout
    pub fn with_timeouts(url: impl Into<String>, ping_interval: Duration, connection_timeout: Duration) -> Self {
        Self {
            url: url.into(),
            stream: None,
            last_ping: None,
            ping_interval,
            connection_timeout,
        }
    }

    /// Connect to the WebSocket
    pub async fn connect(&mut self) -> Result<()> {
        let url = Url::parse(&self.url)?;
        info!("Connecting to WebSocket: {}", self.url);
        
        let (stream, response) = connect_async(url).await?;
        info!("WebSocket connected successfully, status: {}", response.status());
        
        self.stream = Some(stream);
        self.last_ping = Some(Instant::now());
        Ok(())
    }

    /// Reconnect to the WebSocket with retry logic
    pub async fn reconnect(&mut self, max_attempts: u32) -> Result<()> {
        info!("Attempting to reconnect to WebSocket: {}", self.url);
        
        // Close existing connection if any
        if self.stream.is_some() {
            let _ = self.close().await;
        }
        
        let mut attempts = 0;
        while attempts < max_attempts {
            attempts += 1;
            
            match self.connect().await {
                Ok(()) => {
                    info!("WebSocket reconnected successfully after {} attempts", attempts);
                    return Ok(());
                }
                Err(e) => {
                    error!("Reconnection attempt {} failed: {}", attempts, e);
                    if attempts < max_attempts {
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempts.min(6))));
                        info!("Waiting {:?} before next reconnection attempt", delay);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(anyhow!("Failed to reconnect after {} attempts", max_attempts))
    }

    /// Send a message
    pub async fn send(&mut self, message: Message) -> Result<()> {
        if let Some(stream) = &mut self.stream {
            match stream.send(message).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("Failed to send WebSocket message: {}", e);
                    // Mark connection as broken
                    self.stream = None;
                    Err(e.into())
                }
            }
        } else {
            Err(anyhow!("Trying to work with closed connection"))
        }
    }

    /// Send a text message
    pub async fn send_text(&mut self, text: impl Into<String>) -> Result<()> {
        self.send(Message::Text(text.into())).await
    }

    /// Send a JSON message
    pub async fn send_json<T: serde::Serialize>(&mut self, data: &T) -> Result<()> {
        let text = serde_json::to_string(data)?;
        self.send_text(text).await
    }

    /// Receive the next message with timeout and ping handling
    pub async fn next_message(&mut self) -> Result<Option<Message>> {
        if let Some(stream) = &mut self.stream {
            // Check if we need to send a ping
            let needs_ping = if let Some(last_ping) = self.last_ping {
                last_ping.elapsed() > self.ping_interval
            } else {
                false
            };
            
            if needs_ping {
                if let Err(e) = stream.send(Message::Ping(vec![])).await {
                    error!("Failed to send ping: {}", e);
                    self.stream = None;
                    return Err(e.into());
                }
                debug!("Sent ping to keep connection alive");
                self.last_ping = Some(Instant::now());
            }
            
            match stream.next().await {
                Some(Ok(message)) => {
                    match &message {
                        Message::Pong(_) => {
                            debug!("Received pong from server");
                            self.last_ping = Some(Instant::now());
                        }
                        Message::Ping(data) => {
                            debug!("Received ping from server, sending pong");
                            if let Err(e) = stream.send(Message::Pong(data.clone())).await {
                                error!("Failed to respond to ping: {}", e);
                                self.stream = None;
                                return Err(e.into());
                            }
                        }
                        Message::Close(_) => {
                            info!("Received close frame from server");
                            self.stream = None;
                            return Ok(None);
                        }
                        _ => {}
                    }
                    Ok(Some(message))
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    // Mark connection as broken
                    self.stream = None;
                    Err(e.into())
                }
                None => {
                    warn!("WebSocket stream ended");
                    self.stream = None;
                    Ok(None)
                }
            }
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    /// Check if connected and healthy
    pub fn is_connected(&self) -> bool {
        if let Some(last_ping) = self.last_ping {
            self.stream.is_some() && last_ping.elapsed() < self.connection_timeout
        } else {
            self.stream.is_some()
        }
    }

    /// Check if connection is stale (needs reconnection)
    pub fn is_stale(&self) -> bool {
        if let Some(last_ping) = self.last_ping {
            last_ping.elapsed() > self.connection_timeout
        } else {
            false
        }
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            if let Err(e) = stream.close(None).await {
                warn!("Error closing WebSocket connection: {}", e);
            } else {
                debug!("WebSocket connection closed cleanly");
            }
        }
        self.last_ping = None;
        Ok(())
    }
}

impl Drop for WsClient {
    fn drop(&mut self) {
        if self.stream.is_some() {
            debug!("WebSocket client dropped, connection may still be open");
        }
    }
}