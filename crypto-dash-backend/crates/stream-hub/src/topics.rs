use crypto_dash_core::model::{Channel, ChannelType, ExchangeId, MarketType, Symbol};
use serde::{Deserialize, Serialize};

/// Topic key for routing messages in the stream hub
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic {
    pub channel_type: ChannelType,
    pub exchange: ExchangeId,
    pub market_type: MarketType,
    pub symbol: Symbol,
}

impl Topic {
    pub fn new(
        channel_type: ChannelType,
        exchange: ExchangeId,
        market_type: MarketType,
        symbol: Symbol,
    ) -> Self {
        Self {
            channel_type,
            exchange,
            market_type,
            symbol,
        }
    }

    /// Convert a Channel to a Topic
    pub fn from_channel(channel: &Channel) -> Self {
        Self {
            channel_type: channel.channel_type.clone(),
            exchange: channel.exchange.clone(),
            market_type: channel.market_type,
            symbol: channel.symbol.clone(),
        }
    }

    /// Create a ticker topic
    pub fn ticker(exchange: ExchangeId, market_type: MarketType, symbol: Symbol) -> Self {
        Self::new(ChannelType::Ticker, exchange, market_type, symbol)
    }

    /// Create an order book topic
    pub fn orderbook(exchange: ExchangeId, market_type: MarketType, symbol: Symbol) -> Self {
        Self::new(ChannelType::OrderBook, exchange, market_type, symbol)
    }

    /// Generate a string key for this topic
    pub fn key(&self) -> String {
        let channel_segment = match self.channel_type {
            ChannelType::Ticker => "ticker",
            ChannelType::OrderBook => "orderbook",
        };
        let market_segment = match self.market_type {
            MarketType::Spot => "spot",
            MarketType::Perpetual => "perpetual",
        };

        format!(
            "{}:{}:{}:{}",
            channel_segment,
            self.exchange.as_str(),
            market_segment,
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
            MarketType::Spot,
            Symbol::new("BTC", "USDT"),
        );

        assert_eq!(topic.key(), "ticker:binance:spot:BTC-USDT");
    }

    #[test]
    fn test_from_channel() {
        let channel = Channel {
            channel_type: ChannelType::OrderBook,
            exchange: ExchangeId::from("bybit"),
            market_type: MarketType::Perpetual,
            symbol: Symbol::new("ETH", "USDT"),
            depth: Some(50),
        };

        let topic = Topic::from_channel(&channel);
        assert_eq!(topic.channel_type, ChannelType::OrderBook);
        assert_eq!(topic.exchange.as_str(), "bybit");
        assert_eq!(topic.market_type, MarketType::Perpetual);
        assert_eq!(topic.symbol.canonical(), "ETH-USDT");
    }
}
