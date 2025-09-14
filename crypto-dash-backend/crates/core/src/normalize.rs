use crate::model::{ExchangeId, Symbol};
use std::collections::HashMap;

/// Symbol normalization utilities
pub struct SymbolMapper {
    /// Maps exchange-specific symbols to canonical symbols
    exchange_to_canonical: HashMap<(ExchangeId, String), Symbol>,
    /// Maps canonical symbols to exchange-specific symbols
    canonical_to_exchange: HashMap<(ExchangeId, Symbol), String>,
}

impl SymbolMapper {
    pub fn new() -> Self {
        Self {
            exchange_to_canonical: HashMap::new(),
            canonical_to_exchange: HashMap::new(),
        }
    }

    /// Add a symbol mapping
    pub fn add_mapping(&mut self, exchange: ExchangeId, exchange_symbol: String, canonical: Symbol) {
        self.exchange_to_canonical.insert((exchange.clone(), exchange_symbol.clone()), canonical.clone());
        self.canonical_to_exchange.insert((exchange, canonical), exchange_symbol);
    }

    /// Convert exchange-specific symbol to canonical
    pub fn to_canonical(&self, exchange: &ExchangeId, exchange_symbol: &str) -> Option<Symbol> {
        self.exchange_to_canonical.get(&(exchange.clone(), exchange_symbol.to_string())).cloned()
    }

    /// Convert canonical symbol to exchange-specific
    pub fn to_exchange(&self, exchange: &ExchangeId, canonical: &Symbol) -> Option<String> {
        self.canonical_to_exchange.get(&(exchange.clone(), canonical.clone())).cloned()
    }

    /// Load default mappings for common exchanges
    pub fn load_defaults(&mut self) {
        // Binance mappings
        let binance = ExchangeId::from("binance");
        self.add_mapping(binance.clone(), "BTCUSDT".to_string(), Symbol::new("BTC", "USDT"));
        self.add_mapping(binance.clone(), "ETHUSDT".to_string(), Symbol::new("ETH", "USDT"));
        self.add_mapping(binance.clone(), "ADAUSDT".to_string(), Symbol::new("ADA", "USDT"));
        self.add_mapping(binance.clone(), "SOLUSDT".to_string(), Symbol::new("SOL", "USDT"));

        // Bybit mappings
        let bybit = ExchangeId::from("bybit");
        self.add_mapping(bybit.clone(), "BTCUSDT".to_string(), Symbol::new("BTC", "USDT"));
        self.add_mapping(bybit.clone(), "ETHUSDT".to_string(), Symbol::new("ETH", "USDT"));
        self.add_mapping(bybit.clone(), "ADAUSDT".to_string(), Symbol::new("ADA", "USDT"));
        self.add_mapping(bybit.clone(), "SOLUSDT".to_string(), Symbol::new("SOL", "USDT"));
    }
}

impl Default for SymbolMapper {
    fn default() -> Self {
        let mut mapper = Self::new();
        mapper.load_defaults();
        mapper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_mapping() {
        let mut mapper = SymbolMapper::new();
        let exchange = ExchangeId::from("binance");
        let canonical = Symbol::new("BTC", "USDT");
        
        mapper.add_mapping(exchange.clone(), "BTCUSDT".to_string(), canonical.clone());
        
        assert_eq!(
            mapper.to_canonical(&exchange, "BTCUSDT"),
            Some(canonical.clone())
        );
        assert_eq!(
            mapper.to_exchange(&exchange, &canonical),
            Some("BTCUSDT".to_string())
        );
    }

    #[test]
    fn test_default_mappings() {
        let mapper = SymbolMapper::default();
        let binance = ExchangeId::from("binance");
        
        assert_eq!(
            mapper.to_canonical(&binance, "BTCUSDT"),
            Some(Symbol::new("BTC", "USDT"))
        );
    }
}