use crypto_dash_cache::CacheHandle;
use crypto_dash_core::model::{Channel, ExchangeId};
use crypto_dash_stream_hub::HubHandle;
use async_trait::async_trait;
use anyhow::Result;

/// Common interface for exchange adapters
#[async_trait]
pub trait ExchangeAdapter: Send + Sync {
    /// Get the exchange identifier
    fn id(&self) -> ExchangeId;

    /// Start the adapter with the given hub and cache handles
    async fn start(&self, hub: HubHandle, cache: CacheHandle) -> Result<()>;

    /// Subscribe to channels
    async fn subscribe(&self, channels: &[Channel]) -> Result<()>;

    /// Unsubscribe from channels
    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()>;

    /// Check if the adapter is connected
    async fn is_connected(&self) -> bool;

    /// Stop the adapter
    async fn stop(&self) -> Result<()>;
}