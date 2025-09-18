use crate::topics::Topic;
use crypto_dash_core::model::StreamMessage;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;
use uuid::Uuid;

const CHANNEL_CAPACITY: usize = 1000;

/// Handle to interact with the stream hub
#[derive(Clone)]
pub struct HubHandle {
    inner: Arc<StreamHubInner>,
}

impl HubHandle {
    /// Publish a message to a topic
    pub async fn publish(&self, topic: &Topic, message: StreamMessage) {
        self.inner.publish(topic, message).await;
    }

    /// Subscribe to a topic and get a receiver
    pub async fn subscribe(&self, topic: &Topic) -> SubscriberHandle {
        self.inner.subscribe(topic).await
    }

    /// Subscribe to all topics (for WebSocket forwarding)
    pub async fn subscribe_all(&self) -> GlobalSubscriberHandle {
        self.inner.subscribe_all().await
    }

    /// Get the number of active topics
    pub fn topic_count(&self) -> usize {
        self.inner.topics.len()
    }

    /// Get the number of subscribers for a topic
    /// Get the number of global subscribers
    pub fn global_subscriber_count(&self) -> usize {
        self.inner.global_sender.receiver_count()
    }

    pub fn subscriber_count(&self, topic: &Topic) -> usize {
        self.inner
            .topics
            .get(&topic.key())
            .map(|entry| entry.value().sender.receiver_count())
            .unwrap_or(0)
    }
}

/// Handle for a subscription to receive messages
pub struct SubscriberHandle {
    pub id: Uuid,
    pub topic: Topic,
    pub receiver: broadcast::Receiver<StreamMessage>,
}

impl SubscriberHandle {
    /// Receive the next message
    pub async fn recv(&mut self) -> Result<StreamMessage, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&mut self) -> Result<StreamMessage, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Handle for a global subscription to receive all messages
pub struct GlobalSubscriberHandle {
    pub id: Uuid,
    pub receiver: broadcast::Receiver<(Topic, StreamMessage)>,
}

impl GlobalSubscriberHandle {
    /// Receive the next message
    pub async fn recv(&mut self) -> Result<(Topic, StreamMessage), broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&mut self) -> Result<(Topic, StreamMessage), broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

struct TopicChannel {
    sender: broadcast::Sender<StreamMessage>,
}

struct StreamHubInner {
    topics: DashMap<String, TopicChannel>,
    global_sender: broadcast::Sender<(Topic, StreamMessage)>,
}

impl StreamHubInner {
    fn new() -> Self {
        let (global_sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            topics: DashMap::new(),
            global_sender,
        }
    }

    async fn publish(&self, topic: &Topic, message: StreamMessage) {
        let topic_key = topic.key();

        // Publish to specific topic subscribers
        if let Some(entry) = self.topics.get(&topic_key) {
            match entry.sender.send(message.clone()) {
                Ok(subscriber_count) => {
                    debug!(
                        topic = %topic_key,
                        subscribers = subscriber_count,
                        "Published message to topic"
                    );
                }
                Err(_) => {
                    debug!(topic = %topic_key, "No active subscribers for topic");
                }
            }
        }

        // Also publish to global subscribers (like WebSocket clients)
        match self.global_sender.send((topic.clone(), message)) {
            Ok(subscriber_count) => {
                debug!(
                    topic = %topic_key,
                    global_subscribers = subscriber_count,
                    "Published message to global subscribers"
                );
            }
            Err(_) => {
                debug!("No active global subscribers");
            }
        }
    }

    async fn subscribe(&self, topic: &Topic) -> SubscriberHandle {
        let topic_key = topic.key();

        let receiver = {
            let entry = self.topics.entry(topic_key.clone()).or_insert_with(|| {
                let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
                debug!(topic = %topic_key, "Created new topic channel");
                TopicChannel { sender }
            });

            entry.sender.subscribe()
        };

        let id = Uuid::new_v4();
        debug!(
            topic = %topic_key,
            subscriber_id = %id,
            "New subscriber"
        );

        SubscriberHandle {
            id,
            topic: topic.clone(),
            receiver,
        }
    }

    async fn subscribe_all(&self) -> GlobalSubscriberHandle {
        let id = Uuid::new_v4();
        let receiver = self.global_sender.subscribe();

        debug!(
            subscriber_id = %id,
            "New global subscriber"
        );

        GlobalSubscriberHandle { id, receiver }
    }
}

/// Central streaming hub for distributing real-time market data
pub struct StreamHub {
    inner: Arc<StreamHubInner>,
}

impl StreamHub {
    /// Create a new stream hub
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StreamHubInner::new()),
        }
    }

    /// Get a handle to interact with the hub
    pub fn handle(&self) -> HubHandle {
        HubHandle {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Start the hub (currently just returns the handle)
    pub async fn start(self) -> anyhow::Result<HubHandle> {
        debug!("Stream hub started");
        Ok(self.handle())
    }
}

impl Default for StreamHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_dash_core::model::{ExchangeId, MarketType, Symbol, Ticker};
    use crypto_dash_core::time::now;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_hub_publish_subscribe() {
        let hub = StreamHub::new();
        let handle = hub.handle();

        let topic = Topic::ticker(
            ExchangeId::from("binance"),
            MarketType::Spot,
            Symbol::new("BTC", "USDT"),
        );

        let mut subscriber = handle.subscribe(&topic).await;

        let ticker = Ticker {
            timestamp: now(),
            exchange: ExchangeId::from("binance"),
            market_type: MarketType::Spot,
            symbol: Symbol::new("BTC", "USDT"),
            bid: Decimal::new(50000, 0),
            ask: Decimal::new(50001, 0),
            last: Decimal::new(50000, 0),
            bid_size: Decimal::new(1, 0),
            ask_size: Decimal::new(1, 0),
        };

        handle
            .publish(&topic, StreamMessage::Ticker(ticker.clone()))
            .await;

        let received = subscriber.recv().await.unwrap();
        match received {
            StreamMessage::Ticker(received_ticker) => {
                assert_eq!(received_ticker.bid, ticker.bid);
                assert_eq!(received_ticker.ask, ticker.ask);
            }
            _ => panic!("Expected ticker message"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let hub = StreamHub::new();
        let handle = hub.handle();

        let topic = Topic::ticker(
            ExchangeId::from("binance"),
            MarketType::Spot,
            Symbol::new("BTC", "USDT"),
        );

        let mut sub1 = handle.subscribe(&topic).await;
        let mut sub2 = handle.subscribe(&topic).await;

        assert_eq!(handle.subscriber_count(&topic), 2);

        let ticker = Ticker {
            timestamp: now(),
            exchange: ExchangeId::from("binance"),
            market_type: MarketType::Spot,
            symbol: Symbol::new("BTC", "USDT"),
            bid: Decimal::new(50000, 0),
            ask: Decimal::new(50001, 0),
            last: Decimal::new(50000, 0),
            bid_size: Decimal::new(1, 0),
            ask_size: Decimal::new(1, 0),
        };

        handle.publish(&topic, StreamMessage::Ticker(ticker)).await;

        // Both subscribers should receive the message
        let _ = sub1.recv().await.unwrap();
        let _ = sub2.recv().await.unwrap();
    }
}
