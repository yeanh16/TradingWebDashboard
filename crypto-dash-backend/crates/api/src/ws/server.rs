use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use crypto_dash_core::model::{ClientMessage, StreamMessage};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket upgrade handler
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    info!("WebSocket upgrade request received");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let session_id = Uuid::new_v4();
    info!("New WebSocket connection: {}", session_id);

    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    // Send welcome message
    let welcome = StreamMessage::Info {
        message: format!("Connected to crypto-dash API. Session: {}", session_id),
    };

    if let Ok(msg) = serde_json::to_string(&welcome) {
        let mut sender_guard = sender.lock().await;
        if sender_guard.send(Message::Text(msg)).await.is_err() {
            error!("Failed to send welcome message to {}", session_id);
            return;
        }
    }

    // Create a subscriber for stream hub messages
    let mut stream_receiver = state.hub.subscribe_all().await;

    // Spawn a task to forward stream hub messages to the WebSocket
    let ws_sender = Arc::clone(&sender);
    let forward_task = tokio::spawn(async move {
        loop {
            match stream_receiver.recv().await {
                Ok((topic, stream_msg)) => {
                    debug!("Forwarding stream message for topic: {:?}", topic);
                    if let Ok(msg_text) = serde_json::to_string(&stream_msg) {
                        let mut sender_guard = ws_sender.lock().await;
                        if sender_guard.send(Message::Text(msg_text)).await.is_err() {
                            debug!("Failed to forward stream message - client disconnected");
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving from stream hub: {}", e);
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received text message from {}: {}", session_id, text);

                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        debug!("Successfully parsed client message: {:?}", client_msg);
                        if let Err(e) = handle_client_message(client_msg, &state, &sender).await {
                            error!("Error handling client message: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Invalid message format from {}: {} - Raw: {}",
                            session_id, e, text
                        );
                        let error_msg = StreamMessage::Error {
                            message: format!("Invalid message format: {}", e),
                        };

                        if let Ok(msg_text) = serde_json::to_string(&error_msg) {
                            let mut sender_guard = sender.lock().await;
                            let _ = sender_guard.send(Message::Text(msg_text)).await;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed: {}", session_id);
                break;
            }
            Ok(Message::Ping(ping)) => {
                debug!("Received ping from {}", session_id);
                let mut sender_guard = sender.lock().await;
                if sender_guard.send(Message::Pong(ping)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong from {}", session_id);
            }
            Ok(Message::Binary(_)) => {
                warn!("Binary messages not supported");
            }
            Err(e) => {
                error!("WebSocket error for {}: {}", session_id, e);
                break;
            }
        }
    }

    // Cancel the forwarding task when WebSocket disconnects
    forward_task.abort();
    info!("WebSocket connection ended: {}", session_id);
}

/// Handle client messages
async fn handle_client_message(
    message: ClientMessage,
    state: &AppState,
    sender: &Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match message {
        ClientMessage::Subscribe { channels } => {
            debug!("Subscribe request for {} channels", channels.len());

            // Debug: Log the available exchanges
            debug!(
                "Available exchanges: {:?}",
                state.exchanges.keys().collect::<Vec<_>>()
            );

            // Group channels by exchange
            let mut exchanges_channels = std::collections::HashMap::new();
            for channel in &channels {
                let exchange_id = channel.exchange.as_str().to_string();
                debug!(
                    "Processing channel for exchange: '{}' (channel: {:?})",
                    exchange_id, channel
                );
                exchanges_channels
                    .entry(exchange_id)
                    .or_insert_with(Vec::new)
                    .push(channel.clone());
            }

            let num_exchanges = exchanges_channels.len();
            debug!(
                "Grouped into {} exchanges: {:?}",
                num_exchanges,
                exchanges_channels.keys().collect::<Vec<_>>()
            );

            // Subscribe to each exchange
            for (exchange_id, exchange_channels) in &exchanges_channels {
                debug!("Looking up exchange adapter for: '{}'", exchange_id);
                if let Some(adapter) = state.exchanges.get(exchange_id) {
                    debug!(
                        "Found adapter for '{}', subscribing to {} channels",
                        exchange_id,
                        exchange_channels.len()
                    );
                    match adapter.subscribe(exchange_channels).await {
                        Ok(()) => {
                            info!(
                                "Successfully subscribed to {} channels on {}",
                                exchange_channels.len(),
                                exchange_id
                            );
                        }
                        Err(e) => {
                            error!(
                                "Failed to subscribe to {} channels on {}: {}",
                                exchange_channels.len(),
                                exchange_id,
                                e
                            );
                        }
                    }
                } else {
                    warn!(
                        "Unknown exchange: '{}' (available: {:?})",
                        exchange_id,
                        state.exchanges.keys().collect::<Vec<_>>()
                    );
                }
            }

            let response = StreamMessage::Info {
                message: format!(
                    "Subscribed to {} channels across {} exchanges",
                    channels.len(),
                    num_exchanges
                ),
            };

            let msg_text = serde_json::to_string(&response)?;
            let mut sender_guard = sender.lock().await;
            sender_guard.send(Message::Text(msg_text)).await?;
        }
        ClientMessage::Unsubscribe { channels } => {
            debug!("Unsubscribe request for {} channels", channels.len());

            // Group channels by exchange
            let mut exchanges_channels = std::collections::HashMap::new();
            for channel in &channels {
                let exchange_id = channel.exchange.as_str().to_string();
                exchanges_channels
                    .entry(exchange_id)
                    .or_insert_with(Vec::new)
                    .push(channel.clone());
            }

            // Unsubscribe from each exchange
            for (exchange_id, exchange_channels) in exchanges_channels {
                if let Some(adapter) = state.exchanges.get(&exchange_id) {
                    debug!(
                        "Unsubscribing from {} channels on {}",
                        exchange_channels.len(),
                        exchange_id
                    );
                    if let Err(e) = adapter.unsubscribe(&exchange_channels).await {
                        error!(
                            "Failed to unsubscribe from {} channels on {}: {}",
                            exchange_channels.len(),
                            exchange_id,
                            e
                        );
                    }
                } else {
                    warn!("Unknown exchange: {}", exchange_id);
                }
            }

            let response = StreamMessage::Info {
                message: format!("Unsubscribed from {} channels", channels.len()),
            };

            let msg_text = serde_json::to_string(&response)?;
            let mut sender_guard = sender.lock().await;
            sender_guard.send(Message::Text(msg_text)).await?;
        }
        ClientMessage::Ping => {
            debug!("Ping received");

            let response = StreamMessage::Info {
                message: "Pong".to_string(),
            };

            let msg_text = serde_json::to_string(&response)?;
            let mut sender_guard = sender.lock().await;
            sender_guard.send(Message::Text(msg_text)).await?;
        }
    }

    Ok(())
}
