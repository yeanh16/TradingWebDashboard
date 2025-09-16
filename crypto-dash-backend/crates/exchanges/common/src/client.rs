use anyhow::{anyhow, Result};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, warn};
use url::Url;

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket client helper that supports concurrent send/receive operations
#[derive(Clone)]
pub struct WsClient {
    url: Arc<String>,
    writer: Arc<Mutex<Option<SplitSink<WsStream, Message>>>>,
    reader: Arc<Mutex<Option<SplitStream<WsStream>>>>,
    connected: Arc<AtomicBool>,
}

impl WsClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: Arc::new(url.into()),
            writer: Arc::new(Mutex::new(None)),
            reader: Arc::new(Mutex::new(None)),
            connected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Connect to the WebSocket
    pub async fn connect(&self) -> Result<()> {
        let url = Url::parse(self.url.as_str())?;
        debug!("Connecting to WebSocket: {}", self.url);

        let (stream, response) = connect_async(url).await?;
        debug!("WebSocket connected, status: {}", response.status());

        let (writer, reader) = stream.split();
        {
            let mut writer_guard = self.writer.lock().await;
            *writer_guard = Some(writer);
        }
        {
            let mut reader_guard = self.reader.lock().await;
            *reader_guard = Some(reader);
        }
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Send a message
    pub async fn send(&self, message: Message) -> Result<()> {
        let mut writer_guard = self.writer.lock().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.send(message).await?;
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    /// Send a text message
    pub async fn send_text(&self, text: impl Into<String>) -> Result<()> {
        self.send(Message::Text(text.into())).await
    }

    /// Send a JSON message
    pub async fn send_json<T: serde::Serialize>(&self, data: &T) -> Result<()> {
        let text = serde_json::to_string(data)?;
        self.send_text(text).await
    }

    /// Receive the next message
    pub async fn next_message(&self) -> Result<Option<Message>> {
        let mut reader_guard = self.reader.lock().await;
        if let Some(reader) = reader_guard.as_mut() {
            match reader.next().await {
                Some(Ok(message)) => Ok(Some(message)),
                Some(Err(e)) => {
                    self.connected.store(false, Ordering::SeqCst);
                    error!("WebSocket error: {}", e);
                    Err(e.into())
                }
                None => {
                    self.connected.store(false, Ordering::SeqCst);
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
        self.connected.load(Ordering::SeqCst)
    }

    /// Close the connection
    pub async fn close(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);
        {
            let mut writer_guard = self.writer.lock().await;
            if let Some(mut writer) = writer_guard.take() {
                writer.close().await?;
                debug!("WebSocket connection closed");
            }
        }
        {
            let mut reader_guard = self.reader.lock().await;
            *reader_guard = None;
        }
        Ok(())
    }
}

impl Drop for WsClient {
    fn drop(&mut self) {
        if self.connected.load(Ordering::SeqCst) {
            debug!("WebSocket client dropped, connection may still be open");
        }
    }
}
