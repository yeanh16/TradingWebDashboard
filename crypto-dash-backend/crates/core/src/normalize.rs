use crate::model::{ExchangeId, Symbol};
use anyhow::Result;
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
    pub fn add_mapping(
        &mut self,
        exchange: ExchangeId,
        exchange_symbol: String,
        canonical: Symbol,
    ) {
        self.exchange_to_canonical.insert(
            (exchange.clone(), exchange_symbol.clone()),
            canonical.clone(),
        );
        self.canonical_to_exchange
            .insert((exchange, canonical), exchange_symbol);
    }

    /// Convert exchange-specific symbol to canonical
    pub fn to_canonical(&self, exchange: &ExchangeId, exchange_symbol: &str) -> Option<Symbol> {
        self.exchange_to_canonical
            .get(&(exchange.clone(), exchange_symbol.to_string()))
            .cloned()
    }

    /// Convert canonical symbol to exchange-specific
    pub fn to_exchange(&self, exchange: &ExchangeId, canonical: &Symbol) -> Option<String> {
        self.canonical_to_exchange
            .get(&(exchange.clone(), canonical.clone()))
            .cloned()
    }

    /// Load default mappings for common exchanges
    pub fn load_defaults(&mut self) {
        // Binance mappings
        let binance = ExchangeId::from("binance");
        self.add_mapping(
            binance.clone(),
            "BTCUSDT".to_string(),
            Symbol::new("BTC", "USDT"),
        );
        self.add_mapping(
            binance.clone(),
            "ETHUSDT".to_string(),
            Symbol::new("ETH", "USDT"),
        );
        self.add_mapping(
            binance.clone(),
            "ADAUSDT".to_string(),
            Symbol::new("ADA", "USDT"),
        );
        self.add_mapping(
            binance.clone(),
            "SOLUSDT".to_string(),
            Symbol::new("SOL", "USDT"),
        );

        // Bybit mappings
        let bybit = ExchangeId::from("bybit");
        self.add_mapping(
            bybit.clone(),
            "BTCUSDT".to_string(),
            Symbol::new("BTC", "USDT"),
        );
        self.add_mapping(
            bybit.clone(),
            "ETHUSDT".to_string(),
            Symbol::new("ETH", "USDT"),
        );
        self.add_mapping(
            bybit.clone(),
            "ADAUSDT".to_string(),
            Symbol::new("ADA", "USDT"),
        );
        self.add_mapping(
            bybit.clone(),
            "SOLUSDT".to_string(),
            Symbol::new("SOL", "USDT"),
        );
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

/// Utility functions for symbol metadata normalization
pub fn precision_from_tick_size(tick_size: &str) -> Result<u32> {
    if tick_size == "0" || tick_size.is_empty() {
        return Ok(0);
    }

    if let Some(decimal_pos) = tick_size.find('.') {
        let decimal_part = &tick_size[decimal_pos + 1..];
        // Count trailing zeros to find the first non-zero digit
        let trailing_zeros = decimal_part.chars().take_while(|&c| c == '0').count();

        // Find the first non-zero digit after decimal point
        if let Some(first_non_zero) = decimal_part
            .chars()
            .skip(trailing_zeros)
            .find(|&c| c != '0')
        {
            if first_non_zero == '1' {
                // For tick sizes like "0.001", precision is the number of decimal places
                Ok(decimal_part.len() as u32)
            } else {
                // For tick sizes like "0.5", precision is position of first non-zero + 1
                Ok(trailing_zeros as u32 + 1)
            }
        } else {
            Ok(decimal_part.len() as u32)
        }
    } else {
        // No decimal point, precision is 0
        Ok(0)
    }
}

/// Normalize exchange symbol to canonical format
pub fn normalize_symbol(exchange_symbol: &str, exchange: &ExchangeId) -> Symbol {
    match exchange.as_str() {
        "binance" => {
            // Binance uses concatenated format like "BTCUSDT"
            // This is a simple heuristic - in practice you'd use exchange API data
            if exchange_symbol.ends_with("USDT") {
                let base = &exchange_symbol[..exchange_symbol.len() - 4];
                Symbol::new(base, "USDT")
            } else if exchange_symbol.ends_with("BTC") {
                let base = &exchange_symbol[..exchange_symbol.len() - 3];
                Symbol::new(base, "BTC")
            } else if exchange_symbol.ends_with("ETH") {
                let base = &exchange_symbol[..exchange_symbol.len() - 3];
                Symbol::new(base, "ETH")
            } else {
                // Fallback - try common patterns
                Symbol::new(exchange_symbol, "USDT")
            }
        }
        "bybit" => {
            // Bybit also uses concatenated format
            if exchange_symbol.ends_with("USDT") {
                let base = &exchange_symbol[..exchange_symbol.len() - 4];
                Symbol::new(base, "USDT")
            } else if exchange_symbol.ends_with("BTC") {
                let base = &exchange_symbol[..exchange_symbol.len() - 3];
                Symbol::new(base, "BTC")
            } else {
                Symbol::new(exchange_symbol, "USDT")
            }
        }
        _ => {
            // Default fallback
            Symbol::new(exchange_symbol, "USDT")
        }
    }
}

#[cfg(test)]
mod normalization_tests {
    use super::*;

    #[test]
    fn test_precision_from_tick_size() {
        assert_eq!(precision_from_tick_size("0.001").unwrap(), 3);
        assert_eq!(precision_from_tick_size("0.01").unwrap(), 2);
        assert_eq!(precision_from_tick_size("0.1").unwrap(), 1);
        assert_eq!(precision_from_tick_size("1").unwrap(), 0);
        assert_eq!(precision_from_tick_size("0.5").unwrap(), 1);
        assert_eq!(precision_from_tick_size("0.00001").unwrap(), 5);
    }

    #[test]
    fn test_normalize_symbol() {
        let binance = ExchangeId::from("binance");

        let symbol = normalize_symbol("BTCUSDT", &binance);
        assert_eq!(symbol.base, "BTC");
        assert_eq!(symbol.quote, "USDT");

        let symbol = normalize_symbol("ETHBTC", &binance);
        assert_eq!(symbol.base, "ETH");
        assert_eq!(symbol.quote, "BTC");
    }
}
