use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures::{SinkExt, StreamExt};
use url::Url;
use anyhow::{Result, anyhow};
use tracing::{debug, error, warn};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket client helper
pub struct WsClient {
    url: String,
    stream: Option<WsStream>,
}

impl WsClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            stream: None,
        }
    }

    /// Connect to the WebSocket
    pub async fn connect(mut self) -> Result<Self> {
        let url = Url::parse(&self.url)?;
        debug!("Connecting to WebSocket: {}", self.url);
        
        let (stream, response) = connect_async(url).await?;
        debug!("WebSocket connected, status: {}", response.status());
        
        self.stream = Some(stream);
        Ok(self)
    }

    /// Send a message
    pub async fn send(&mut self, message: Message) -> Result<()> {
        if let Some(stream) = &mut self.stream {
            stream.send(message).await?;
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
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

    /// Receive the next message
    pub async fn next_message(&mut self) -> Result<Option<Message>> {
        if let Some(stream) = &mut self.stream {
            match stream.next().await {
                Some(Ok(message)) => Ok(Some(message)),
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    Err(e.into())
                }
                None => {
                    warn!("WebSocket stream ended");
                    Ok(None)
                }
            }
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.close(None).await?;
            debug!("WebSocket connection closed");
        }
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