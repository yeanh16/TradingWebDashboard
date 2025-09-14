use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct SymbolsQuery {
    exchange: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SymbolResponse {
    pub exchange: String,
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolInfo {
    pub symbol: String,
    pub base: String,
    pub quote: String,
    pub display_name: String,
}

/// GET /api/symbols - Get available trading symbols for exchanges
pub async fn list_symbols(
    Query(params): Query<SymbolsQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SymbolResponse>>, StatusCode> {
    let exchanges = state.get_exchange_info().await;
    
    // For now, provide a curated list of popular trading pairs
    // In a real implementation, this would come from exchange APIs
    let popular_symbols = get_popular_symbols();
    
    let mut response = Vec::new();
    
    if let Some(exchange_filter) = params.exchange {
        // Return symbols for specific exchange
        if let Some(symbols) = popular_symbols.get(&exchange_filter) {
            response.push(SymbolResponse {
                exchange: exchange_filter,
                symbols: symbols.clone(),
            });
        }
    } else {
        // Return symbols for all available exchanges
        for exchange in exchanges {
            if let Some(symbols) = popular_symbols.get(exchange.id.as_str()) {
                response.push(SymbolResponse {
                    exchange: exchange.id.as_str().to_string(),
                    symbols: symbols.clone(),
                });
            }
        }
    }
    
    Ok(Json(response))
}

fn get_popular_symbols() -> HashMap<String, Vec<SymbolInfo>> {
    let mut symbols = HashMap::new();
    
    // Popular trading pairs for both exchanges
    let common_pairs = vec![
        ("BTC", "USDT", "Bitcoin"),
        ("ETH", "USDT", "Ethereum"),
        ("ADA", "USDT", "Cardano"),
        ("SOL", "USDT", "Solana"),
        ("MATIC", "USDT", "Polygon"),
        ("DOT", "USDT", "Polkadot"),
        ("AVAX", "USDT", "Avalanche"),
        ("LINK", "USDT", "Chainlink"),
        ("UNI", "USDT", "Uniswap"),
        ("XRP", "USDT", "Ripple"),
        ("LTC", "USDT", "Litecoin"),
        ("BCH", "USDT", "Bitcoin Cash"),
        ("ATOM", "USDT", "Cosmos"),
        ("ICP", "USDT", "Internet Computer"),
        ("NEAR", "USDT", "NEAR Protocol"),
        ("ALGO", "USDT", "Algorand"),
        ("VET", "USDT", "VeChain"),
        ("MANA", "USDT", "Decentraland"),
        ("SAND", "USDT", "The Sandbox"),
        ("FTM", "USDT", "Fantom"),
        // BTC pairs
        ("ETH", "BTC", "Ethereum"),
        ("ADA", "BTC", "Cardano"),
        ("SOL", "BTC", "Solana"),
        ("LINK", "BTC", "Chainlink"),
        ("DOT", "BTC", "Polkadot"),
    ];
    
    let symbol_infos: Vec<SymbolInfo> = common_pairs
        .into_iter()
        .map(|(base, quote, name)| SymbolInfo {
            symbol: format!("{}-{}", base, quote),
            base: base.to_string(),
            quote: quote.to_string(),
            display_name: format!("{} / {}", name, quote),
        })
        .collect();
    
    // Both Binance and Bybit support these pairs
    symbols.insert("binance".to_string(), symbol_infos.clone());
    symbols.insert("bybit".to_string(), symbol_infos);
    
    symbols
}