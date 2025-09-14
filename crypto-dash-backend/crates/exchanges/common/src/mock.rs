use crypto_dash_core::{
    model::{Symbol, Ticker, StreamMessage, ExchangeId},
    time::now,
};
use crypto_dash_stream_hub::{HubHandle, Topic};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::{interval};
use tracing::info;

/// Mock data generator for exchanges when real connections are not available
pub struct MockDataGenerator {
    exchange_id: ExchangeId,
    hub: HubHandle,
    symbols: Vec<Symbol>,
    base_prices: std::collections::HashMap<String, Decimal>,
}

impl MockDataGenerator {
    pub fn new(exchange_id: ExchangeId, hub: HubHandle) -> Self {
        let symbols = vec![
            Symbol::new("BTC", "USDT"),
            Symbol::new("ETH", "USDT"),
            Symbol::new("DOT", "USDT"),
            Symbol::new("ADA", "USDT"),
            Symbol::new("SOL", "USDT"),
            Symbol::new("MATIC", "USDT"),
            Symbol::new("AVAX", "USDT"),
            Symbol::new("LINK", "USDT"),
            Symbol::new("UNI", "USDT"),
            Symbol::new("XRP", "USDT"),
        ];

        let mut base_prices = std::collections::HashMap::new();
        base_prices.insert("BTC".to_string(), Decimal::from_str("43251.00").unwrap());
        base_prices.insert("ETH".to_string(), Decimal::from_str("2650.60").unwrap());
        base_prices.insert("DOT".to_string(), Decimal::from_str("7.85").unwrap());
        base_prices.insert("ADA".to_string(), Decimal::from_str("0.95").unwrap());
        base_prices.insert("SOL".to_string(), Decimal::from_str("240.00").unwrap());
        base_prices.insert("MATIC".to_string(), Decimal::from_str("0.55").unwrap());
        base_prices.insert("AVAX".to_string(), Decimal::from_str("42.50").unwrap());
        base_prices.insert("LINK".to_string(), Decimal::from_str("25.80").unwrap());
        base_prices.insert("UNI".to_string(), Decimal::from_str("12.45").unwrap());
        base_prices.insert("XRP".to_string(), Decimal::from_str("2.35").unwrap());

        Self {
            exchange_id,
            hub,
            symbols,
            base_prices,
        }
    }

    pub async fn start(&self) {
        info!("Starting mock data generator for exchange: {}", self.exchange_id.as_str());
        
        let mut interval = interval(Duration::from_millis(1000)); // Update every second
        let generator = self.clone();
        
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                generator.generate_mock_tickers().await;
            }
        });
    }

    async fn generate_mock_tickers(&self) {
        for symbol in &self.symbols {
            if let Some(base_price) = self.base_prices.get(&symbol.base) {
                let ticker = self.create_mock_ticker(symbol, *base_price);
                let topic = Topic::ticker(self.exchange_id.clone(), symbol.clone());
                
                self.hub.publish(&topic, StreamMessage::Ticker(ticker)).await;
            }
        }
    }

    fn create_mock_ticker(&self, symbol: &Symbol, base_price: Decimal) -> Ticker {
        // Generate realistic price variations (±2%)
        let variation = (rand::random::<f64>() - 0.5) * 0.04; // -2% to +2%
        let price_factor = Decimal::from_str(&(1.0 + variation).to_string()).unwrap_or(Decimal::ONE);
        let current_price = base_price * price_factor;
        
        // Generate realistic bid/ask spread (0.01% to 0.05%)
        let spread_factor = Decimal::from_str(&(0.0001 + rand::random::<f64>() * 0.0004).to_string()).unwrap();
        let spread = current_price * spread_factor;
        
        let bid = current_price - spread / Decimal::TWO;
        let ask = current_price + spread / Decimal::TWO;
        
        // Generate realistic volumes
        let bid_size = Decimal::from_str(&(0.1 + rand::random::<f64>() * 10.0).to_string()).unwrap();
        let ask_size = Decimal::from_str(&(0.1 + rand::random::<f64>() * 10.0).to_string()).unwrap();

        Ticker {
            timestamp: now(),
            exchange: self.exchange_id.clone(),
            symbol: symbol.clone(),
            bid,
            ask,
            last: current_price,
            bid_size,
            ask_size,
        }
    }
}

impl Clone for MockDataGenerator {
    fn clone(&self) -> Self {
        Self {
            exchange_id: self.exchange_id.clone(),
            hub: self.hub.clone(),
            symbols: self.symbols.clone(),
            base_prices: self.base_prices.clone(),
        }
    }
}