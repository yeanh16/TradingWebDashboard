use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use crypto_dash_core::model::{ClientMessage, StreamMessage};
use crate::state::AppState;
use futures::{sink::SinkExt, stream::StreamExt};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let session_id = Uuid::new_v4();
    info!("New WebSocket connection: {}", session_id);

    let (mut sender, mut receiver) = socket.split();

    // Send welcome message
    let welcome = StreamMessage::Info {
        message: format!("Connected to crypto-dash API. Session: {}", session_id),
    };
    
    if let Ok(msg) = serde_json::to_string(&welcome) {
        if sender.send(Message::Text(msg)).await.is_err() {
            error!("Failed to send welcome message to {}", session_id);
            return;
        }
    }

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received text message from {}: {}", session_id, text);
                
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        if let Err(e) = handle_client_message(client_msg, &state, &mut sender).await {
                            error!("Error handling client message: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Invalid message format from {}: {}", session_id, e);
                        let error_msg = StreamMessage::Error {
                            message: format!("Invalid message format: {}", e),
                        };
                        
                        if let Ok(msg_text) = serde_json::to_string(&error_msg) {
                            let _ = sender.send(Message::Text(msg_text)).await;
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
                if sender.send(Message::Pong(ping)).await.is_err() {
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

    info!("WebSocket connection ended: {}", session_id);
}

/// Handle client messages
async fn handle_client_message(
    message: ClientMessage,
    _state: &AppState,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match message {
        ClientMessage::Subscribe { channels } => {
            debug!("Subscribe request for {} channels", channels.len());
            
            // In a real implementation, subscribe to the requested channels
            for channel in &channels {
                debug!(
                    "Would subscribe to {} {} on {}",
                    channel.symbol.canonical(),
                    match channel.channel_type {
                        crypto_dash_core::model::ChannelType::Ticker => "ticker",
                        crypto_dash_core::model::ChannelType::OrderBook => "orderbook",
                    },
                    channel.exchange.as_str()
                );
            }

            let response = StreamMessage::Info {
                message: format!("Subscribed to {} channels", channels.len()),
            };
            
            let msg_text = serde_json::to_string(&response)?;
            sender.send(Message::Text(msg_text)).await?;
        }
        ClientMessage::Unsubscribe { channels } => {
            debug!("Unsubscribe request for {} channels", channels.len());
            
            let response = StreamMessage::Info {
                message: format!("Unsubscribed from {} channels", channels.len()),
            };
            
            let msg_text = serde_json::to_string(&response)?;
            sender.send(Message::Text(msg_text)).await?;
        }
        ClientMessage::Ping => {
            debug!("Ping received");
            
            let response = StreamMessage::Info {
                message: "Pong".to_string(),
            };
            
            let msg_text = serde_json::to_string(&response)?;
            sender.send(Message::Text(msg_text)).await?;
        }
    }

    Ok(())
}