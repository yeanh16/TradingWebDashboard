use crypto_dash_cache::CacheHandle;
use crypto_dash_core::model::{Channel, ChannelType, ExchangeId};
use crypto_dash_exchanges_common::ExchangeAdapter;
use crypto_dash_stream_hub::HubHandle;
use async_trait::async_trait;
use anyhow::Result;
use tracing::{debug, info};

pub struct BybitAdapter;

impl BybitAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExchangeAdapter for BybitAdapter {
    fn id(&self) -> ExchangeId {
        ExchangeId::from("bybit")
    }

    async fn start(&self, _hub: HubHandle, _cache: CacheHandle) -> Result<()> {
        info!("Starting Bybit adapter");
        debug!("Bybit adapter started with hub and cache handles");
        Ok(())
    }

    async fn subscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Subscribing to {} Bybit channels", channels.len());
        
        for channel in channels {
            debug!(
                "Would subscribe to Bybit {} for {}/{}",
                match channel.channel_type {
                    ChannelType::Ticker => "ticker",
                    ChannelType::OrderBook => "orderbook",
                },
                channel.exchange.as_str(),
                channel.symbol.canonical()
            );
        }
        
        Ok(())
    }

    async fn unsubscribe(&self, channels: &[Channel]) -> Result<()> {
        info!("Unsubscribing from {} Bybit channels", channels.len());
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping Bybit adapter");
        Ok(())
    }
}

impl Default for BybitAdapter {
    fn default() -> Self {
        Self::new()
    }
}