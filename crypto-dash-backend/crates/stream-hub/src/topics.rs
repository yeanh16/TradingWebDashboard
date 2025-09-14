use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, Symbol};
use serde::{Deserialize, Serialize};

/// Topic key for routing messages in the stream hub
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic {
    pub channel_type: ChannelType,
    pub exchange: ExchangeId,
    pub symbol: Symbol,
}

impl Topic {
    pub fn new(channel_type: ChannelType, exchange: ExchangeId, symbol: Symbol) -> Self {
        Self {
            channel_type,
            exchange,
            symbol,
        }
    }

    /// Convert a Channel to a Topic
    pub fn from_channel(channel: &Channel) -> Self {
        Self {
            channel_type: channel.channel_type.clone(),
            exchange: channel.exchange.clone(),
            symbol: channel.symbol.clone(),
        }
    }

    /// Create a ticker topic
    pub fn ticker(exchange: ExchangeId, symbol: Symbol) -> Self {
        Self::new(ChannelType::Ticker, exchange, symbol)
    }

    /// Create an order book topic
    pub fn orderbook(exchange: ExchangeId, symbol: Symbol) -> Self {
        Self::new(ChannelType::OrderBook, exchange, symbol)
    }

    /// Generate a string key for this topic
    pub fn key(&self) -> String {
        format!(
            "{}:{}:{}",
            match self.channel_type {
                ChannelType::Ticker => "ticker",
                ChannelType::OrderBook => "orderbook",
            },
            self.exchange.as_str(),
            self.symbol.canonical()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_key() {
        let topic = Topic::ticker(
            ExchangeId::from("binance"),
            Symbol::new("BTC", "USDT")
        );
        
        assert_eq!(topic.key(), "ticker:binance:BTC-USDT");
    }

    #[test]
    fn test_from_channel() {
        let channel = Channel {
            channel_type: ChannelType::OrderBook,
            exchange: ExchangeId::from("bybit"),
            symbol: Symbol::new("ETH", "USDT"),
            depth: Some(50),
        };

        let topic = Topic::from_channel(&channel);
        assert_eq!(topic.channel_type, ChannelType::OrderBook);
        assert_eq!(topic.exchange.as_str(), "bybit");
        assert_eq!(topic.symbol.canonical(), "ETH-USDT");
    }
}