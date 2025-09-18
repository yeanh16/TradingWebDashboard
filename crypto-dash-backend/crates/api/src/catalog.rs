use anyhow::{anyhow, Result};
use crypto_dash_cache::CacheHandle;
use crypto_dash_core::model::{ExchangeId, MarketType, SymbolMeta};
use crypto_dash_core::normalize::precision_from_tick_size;
use crypto_dash_exchanges_common::ExchangeAdapter;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Raw symbol data from Binance API
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BinanceSymbol {
    symbol: String,
    base_asset: String,
    quote_asset: String,
    base_asset_precision: u32,
    quote_precision: u32,
    filters: Vec<BinanceFilter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BinanceFilter {
    filter_type: String,
    tick_size: Option<String>,
    min_qty: Option<String>,
    step_size: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceExchangeInfo {
    symbols: Vec<BinanceSymbol>,
}

/// Raw symbol data from Bybit API
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BybitSymbol {
    symbol: String,
    base_coin: String,
    quote_coin: String,
    price_filter: Option<BybitPriceFilter>,
    lot_size_filter: Option<BybitLotSizeFilter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BybitPriceFilter {
    tick_size: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BybitLotSizeFilter {
    // Make these optional because Bybit responses may omit some fields
    // (different markets / versions sometimes return slightly different keys).
    min_order_qty: Option<String>,
    qty_step: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BybitResponse {
    result: BybitResult,
}

#[derive(Debug, Clone, Deserialize)]
struct BybitResult {
    list: Vec<BybitSymbol>,
}

/// Exchange catalog service for fetching and caching symbol metadata
pub struct ExchangeCatalog {
    cache: CacheHandle,
    client: Client,
    symbol_cache: Arc<RwLock<HashMap<String, Vec<SymbolMeta>>>>,
}

impl ExchangeCatalog {
    pub fn new(cache: CacheHandle) -> Self {
        Self {
            cache,
            client: Client::new(),
            symbol_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load symbol metadata for all exchanges
    pub async fn load_all(
        &self,
        exchanges: &HashMap<String, Arc<dyn ExchangeAdapter>>,
    ) -> Result<()> {
        info!("Loading symbol metadata for all exchanges");

        for (exchange_name, _adapter) in exchanges {
            if let Err(e) = self.load_exchange_symbols(exchange_name).await {
                error!("Failed to load symbols for {}: {}", exchange_name, e);
                // Try to load from cache
                if let Err(cache_err) = self.load_from_cache(exchange_name).await {
                    warn!(
                        "Failed to load symbols from cache for {}: {}",
                        exchange_name, cache_err
                    );
                    // Load fallback symbols
                    self.load_fallback_symbols(exchange_name).await;
                }
            }
        }

        Ok(())
    }

    /// Load symbol metadata for a specific exchange
    pub async fn load_exchange_symbols(&self, exchange_name: &str) -> Result<()> {
        info!("Loading symbols for exchange: {}", exchange_name);

        let symbols = match exchange_name {
            "binance" => self.fetch_binance_symbols().await?,
            "bybit" => self.fetch_bybit_symbols().await?,
            _ => return Err(anyhow!("Unsupported exchange: {}", exchange_name)),
        };

        // Store in memory cache
        {
            let mut cache = self.symbol_cache.write().await;
            cache.insert(exchange_name.to_string(), symbols.clone());
        }

        // Store in persistent cache
        let cache_key = format!("exchange_symbols_{}", exchange_name);
        if let Err(e) = self.cache.set(&cache_key, &symbols).await {
            warn!("Failed to cache symbols for {}: {}", exchange_name, e);
        }

        info!("Loaded {} symbols for {}", symbols.len(), exchange_name);
        Ok(())
    }

    /// Get symbols for specific exchange(s)
    pub async fn get_symbols(&self, exchange: Option<&str>) -> Vec<SymbolMeta> {
        let cache = self.symbol_cache.read().await;

        match exchange {
            Some(exchange_name) => cache.get(exchange_name).cloned().unwrap_or_default(),
            None => {
                let mut all_symbols = Vec::new();
                for symbols in cache.values() {
                    all_symbols.extend(symbols.clone());
                }
                all_symbols
            }
        }
    }

    /// Refresh symbols for a specific exchange
    pub async fn refresh_exchange(&self, exchange_name: &str) -> Result<()> {
        info!("Refreshing symbols for exchange: {}", exchange_name);
        self.load_exchange_symbols(exchange_name).await
    }

    async fn fetch_binance_symbols(&self) -> Result<Vec<SymbolMeta>> {
        let url = "https://api.binance.com/api/v3/exchangeInfo";
        let response = self.client.get(url).send().await?;
        let exchange_info: BinanceExchangeInfo = response.json().await?;

        let mut symbols = Vec::new();
        let exchange_id = ExchangeId::from("binance");

        for symbol in exchange_info.symbols {
            // Clone the symbol for serialization before moving parts
            let symbol_for_info = symbol.clone();

            // Find relevant filters
            let mut tick_size = "0.01".to_string();
            let mut min_qty = Decimal::from_str("0.001").unwrap_or_default();
            let mut step_size = Decimal::from_str("0.001").unwrap_or_default();
            let mut filters_map = HashMap::new();

            for filter in &symbol.filters {
                match filter.filter_type.as_str() {
                    "PRICE_FILTER" => {
                        if let Some(ts) = &filter.tick_size {
                            tick_size = ts.clone();
                        }
                    }
                    "LOT_SIZE" => {
                        if let Some(mq) = &filter.min_qty {
                            min_qty = Decimal::from_str(mq).unwrap_or_default();
                        }
                        if let Some(ss) = &filter.step_size {
                            step_size = Decimal::from_str(ss).unwrap_or_default();
                        }
                    }
                    _ => {}
                }

                // Store all filter info
                if let Ok(filter_json) = serde_json::to_string(&filter) {
                    filters_map.insert(filter.filter_type.clone(), filter_json);
                }
            }

            let price_precision = precision_from_tick_size(&tick_size).unwrap_or(2);

            let spot_meta = SymbolMeta {
                exchange: exchange_id.clone(),
                market_type: MarketType::Spot,
                symbol: symbol.symbol.clone(),
                base: symbol.base_asset.clone(),
                quote: symbol.quote_asset.clone(),
                price_precision,
                tick_size: tick_size.clone(),
                min_qty,
                step_size,
                filters: Some(filters_map.clone()),
                info: serde_json::to_value(&symbol_for_info).unwrap_or(Value::Null),
            };

            let mut perp_meta = spot_meta.clone();
            perp_meta.market_type = MarketType::Perpetual;

            symbols.push(spot_meta);
            symbols.push(perp_meta);
        }

        Ok(symbols)
    }

    async fn fetch_bybit_symbols(&self) -> Result<Vec<SymbolMeta>> {
        let url = "https://api.bybit.com/v5/market/instruments-info?category=spot";
        let response = self.client.get(url).send().await?;
        let bybit_response: BybitResponse = response.json().await?;

        let mut symbols = Vec::new();
        let exchange_id = ExchangeId::from("bybit");

        for symbol in bybit_response.result.list {
            // Clone the symbol for serialization before moving parts
            let symbol_for_info = symbol.clone();

            let tick_size = symbol
                .price_filter
                .as_ref()
                .map(|pf| pf.tick_size.clone())
                .unwrap_or_else(|| "0.01".to_string());

            let min_qty = symbol
                .lot_size_filter
                .as_ref()
                .and_then(|lsf| lsf.min_order_qty.as_ref())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_else(|| Decimal::from_str("0.001").unwrap());

            let step_size = symbol
                .lot_size_filter
                .as_ref()
                .and_then(|lsf| lsf.qty_step.as_ref())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_else(|| Decimal::from_str("0.001").unwrap());

            let price_precision = precision_from_tick_size(&tick_size).unwrap_or(2);

            let mut filters_map = HashMap::new();
            if let Some(pf) = &symbol.price_filter {
                if let Ok(filter_json) = serde_json::to_string(pf) {
                    filters_map.insert("PRICE_FILTER".to_string(), filter_json);
                }
            }
            if let Some(lsf) = &symbol.lot_size_filter {
                if let Ok(filter_json) = serde_json::to_string(lsf) {
                    filters_map.insert("LOT_SIZE".to_string(), filter_json);
                }
            }

            let spot_meta = SymbolMeta {
                exchange: exchange_id.clone(),
                market_type: MarketType::Spot,
                symbol: symbol.symbol.clone(),
                base: symbol.base_coin.clone(),
                quote: symbol.quote_coin.clone(),
                price_precision,
                tick_size: tick_size.clone(),
                min_qty,
                step_size,
                filters: Some(filters_map.clone()),
                info: serde_json::to_value(&symbol_for_info).unwrap_or(Value::Null),
            };

            let mut perp_meta = spot_meta.clone();
            perp_meta.market_type = MarketType::Perpetual;

            symbols.push(spot_meta);
            symbols.push(perp_meta);
        }

        Ok(symbols)
    }

    async fn load_from_cache(&self, exchange_name: &str) -> Result<()> {
        let cache_key = format!("exchange_symbols_{}", exchange_name);
        if let Ok(Some(symbols)) = self.cache.get::<Vec<SymbolMeta>>(&cache_key).await {
            let mut cache = self.symbol_cache.write().await;
            cache.insert(exchange_name.to_string(), symbols);
            info!("Loaded symbols for {} from cache", exchange_name);
            Ok(())
        } else {
            Err(anyhow!("No cached symbols found for {}", exchange_name))
        }
    }

    async fn load_fallback_symbols(&self, exchange_name: &str) {
        warn!("Loading fallback symbols for {}", exchange_name);

        // Create minimal fallback symbols
        let exchange_id = ExchangeId::from(exchange_name);
        let mut fallback_symbols = vec![
            SymbolMeta {
                exchange: exchange_id.clone(),
                market_type: MarketType::Spot,
                symbol: "BTCUSDT".to_string(),
                base: "BTC".to_string(),
                quote: "USDT".to_string(),
                price_precision: 2,
                tick_size: "0.01".to_string(),
                min_qty: Decimal::from_str("0.001").unwrap(),
                step_size: Decimal::from_str("0.001").unwrap(),
                filters: None,
                info: Value::Null,
            },
            SymbolMeta {
                exchange: exchange_id.clone(),
                market_type: MarketType::Spot,
                symbol: "ETHUSDT".to_string(),
                base: "ETH".to_string(),
                quote: "USDT".to_string(),
                price_precision: 2,
                tick_size: "0.01".to_string(),
                min_qty: Decimal::from_str("0.001").unwrap(),
                step_size: Decimal::from_str("0.001").unwrap(),
                filters: None,
                info: Value::Null,
            },
        ];

        let mut perp_entries: Vec<SymbolMeta> = fallback_symbols
            .iter()
            .map(|meta| {
                let mut clone = meta.clone();
                clone.market_type = MarketType::Perpetual;
                clone
            })
            .collect();
        fallback_symbols.append(&mut perp_entries);

        let mut cache = self.symbol_cache.write().await;
        cache.insert(exchange_name.to_string(), fallback_symbols);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_dash_cache::MemoryCache;

    #[tokio::test]
    async fn test_precision_calculation() {
        assert_eq!(precision_from_tick_size("0.001").unwrap(), 3);
        assert_eq!(precision_from_tick_size("0.01").unwrap(), 2);
        assert_eq!(precision_from_tick_size("1").unwrap(), 0);
    }

    #[tokio::test]
    async fn test_catalog_creation() {
        let cache = MemoryCache::new();
        let cache_handle = cache.start().await.unwrap();
        let catalog = ExchangeCatalog::new(cache_handle);

        // Test that empty symbols are returned initially
        let symbols = catalog.get_symbols(Some("binance")).await;
        assert!(symbols.is_empty());
    }
}
