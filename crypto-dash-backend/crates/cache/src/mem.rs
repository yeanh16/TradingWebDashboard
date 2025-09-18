use crypto_dash_core::model::{ExchangeId, MarketType, OrderBookSnapshot, Symbol, Ticker};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// Cache key for ticker data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TickerKey {
    pub exchange: ExchangeId,
    pub market_type: MarketType,
    pub symbol: Symbol,
}

impl TickerKey {
    pub fn new(exchange: ExchangeId, market_type: MarketType, symbol: Symbol) -> Self {
        Self {
            exchange,
            market_type,
            symbol,
        }
    }
}

/// Cache key for order book data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderBookKey {
    pub exchange: ExchangeId,
    pub market_type: MarketType,
    pub symbol: Symbol,
}

impl OrderBookKey {
    pub fn new(exchange: ExchangeId, market_type: MarketType, symbol: Symbol) -> Self {
        Self {
            exchange,
            market_type,
            symbol,
        }
    }
}

/// Handle to interact with the cache
#[derive(Clone)]
pub struct CacheHandle {
    inner: Arc<MemoryCacheInner>,
}

impl CacheHandle {
    /// Store arbitrary data in the cache
    pub async fn set<T>(&self, key: &str, value: &T) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)?;
        self.inner.generic_data.insert(key.to_string(), serialized);
        debug!("Cached data for key: {}", key);
        Ok(())
    }

    /// Get arbitrary data from the cache
    pub async fn get<T>(&self, key: &str) -> anyhow::Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        if let Some(entry) = self.inner.generic_data.get(key) {
            let value: T = serde_json::from_str(&entry)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Store a ticker in the cache
    pub async fn set_ticker(&self, ticker: Ticker) {
        let key = TickerKey::new(
            ticker.exchange.clone(),
            ticker.market_type,
            ticker.symbol.clone(),
        );
        debug!(
            "Cached ticker for {}/{}",
            ticker.exchange.as_str(),
            ticker.symbol.canonical()
        );
        self.inner.tickers.insert(key, ticker);
    }

    /// Get a ticker from the cache
    pub async fn get_ticker(
        &self,
        exchange: &ExchangeId,
        market_type: MarketType,
        symbol: &Symbol,
    ) -> Option<Ticker> {
        let key = TickerKey::new(exchange.clone(), market_type, symbol.clone());
        self.inner
            .tickers
            .get(&key)
            .map(|entry| entry.value().clone())
    }

    /// Store an order book snapshot in the cache
    pub async fn set_orderbook(&self, orderbook: OrderBookSnapshot) {
        let key = OrderBookKey::new(
            orderbook.exchange.clone(),
            orderbook.market_type,
            orderbook.symbol.clone(),
        );
        debug!(
            "Cached orderbook for {}/{}",
            orderbook.exchange.as_str(),
            orderbook.symbol.canonical()
        );
        self.inner.orderbooks.insert(key, orderbook);
    }

    /// Get an order book snapshot from the cache
    pub async fn get_orderbook(
        &self,
        exchange: &ExchangeId,
        market_type: MarketType,
        symbol: &Symbol,
    ) -> Option<OrderBookSnapshot> {
        let key = OrderBookKey::new(exchange.clone(), market_type, symbol.clone());
        self.inner
            .orderbooks
            .get(&key)
            .map(|entry| entry.value().clone())
    }

    /// Get all cached tickers
    pub async fn get_all_tickers(&self) -> Vec<Ticker> {
        self.inner
            .tickers
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all cached order books
    pub async fn get_all_orderbooks(&self) -> Vec<OrderBookSnapshot> {
        self.inner
            .orderbooks
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Clear all cached data
    pub async fn clear(&self) {
        self.inner.tickers.clear();
        self.inner.orderbooks.clear();
        debug!("Cleared all cache data");
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            ticker_count: self.inner.tickers.len(),
            orderbook_count: self.inner.orderbooks.len(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub ticker_count: usize,
    pub orderbook_count: usize,
}

struct MemoryCacheInner {
    tickers: DashMap<TickerKey, Ticker>,
    orderbooks: DashMap<OrderBookKey, OrderBookSnapshot>,
    generic_data: DashMap<String, String>, // JSON serialized data
}

impl MemoryCacheInner {
    fn new() -> Self {
        Self {
            tickers: DashMap::new(),
            orderbooks: DashMap::new(),
            generic_data: DashMap::new(),
        }
    }
}

/// In-memory cache for market data
pub struct MemoryCache {
    inner: Arc<MemoryCacheInner>,
}

impl MemoryCache {
    /// Create a new memory cache
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MemoryCacheInner::new()),
        }
    }

    /// Get a handle to interact with the cache
    pub fn handle(&self) -> CacheHandle {
        CacheHandle {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Start the cache (currently just returns the handle)
    pub async fn start(self) -> anyhow::Result<CacheHandle> {
        debug!("Memory cache started");
        Ok(self.handle())
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_dash_core::time::now;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_ticker_cache() {
        let cache = MemoryCache::new();
        let handle = cache.handle();

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

        handle.set_ticker(ticker.clone()).await;

        let cached = handle
            .get_ticker(&ticker.exchange, MarketType::Spot, &ticker.symbol)
            .await;
        assert!(cached.is_some());

        let cached_ticker = cached.unwrap();
        assert_eq!(cached_ticker.bid, ticker.bid);
        assert_eq!(cached_ticker.ask, ticker.ask);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = MemoryCache::new();
        let handle = cache.handle();

        let stats = handle.stats().await;
        assert_eq!(stats.ticker_count, 0);
        assert_eq!(stats.orderbook_count, 0);

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

        handle.set_ticker(ticker).await;

        let stats = handle.stats().await;
        assert_eq!(stats.ticker_count, 1);
    }
}
