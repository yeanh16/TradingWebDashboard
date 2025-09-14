use crypto_dash_cache::CacheHandle;
use crypto_dash_core::model::ExchangeInfo;
use crypto_dash_exchanges_common::ExchangeAdapter;
use crypto_dash_stream_hub::HubHandle;
use std::collections::HashMap;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub hub: HubHandle,
    #[allow(dead_code)]
    pub cache: CacheHandle,
    pub exchanges: HashMap<String, Arc<dyn ExchangeAdapter>>,
}

impl AppState {
    pub fn new(hub: HubHandle, cache: CacheHandle) -> Self {
        Self {
            hub,
            cache,
            exchanges: HashMap::new(),
        }
    }

    pub fn add_exchange(&mut self, adapter: Arc<dyn ExchangeAdapter>) {
        let id = adapter.id().as_str().to_string();
        self.exchanges.insert(id, adapter);
    }

    pub async fn get_exchange_info(&self) -> Vec<ExchangeInfo> {
        let mut exchanges = Vec::new();
        
        for (id, adapter) in &self.exchanges {
            let info = ExchangeInfo {
                id: adapter.id(),
                name: id.clone(),
                status: if adapter.is_connected().await {
                    crypto_dash_core::model::ExchangeStatus::Online
                } else {
                    crypto_dash_core::model::ExchangeStatus::Offline
                },
                rate_limits: HashMap::new(),
                ws_url: "".to_string(),
                rest_url: "".to_string(),
            };
            exchanges.push(info);
        }
        
        exchanges
    }
}