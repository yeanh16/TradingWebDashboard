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
    pub symbols: Vec<SymbolMetaDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolMetaDto {
    pub symbol: String,
    pub base: String,
    pub quote: String,
    pub display_name: String,
    pub price_precision: u32,
    pub tick_size: String,
    pub min_qty: rust_decimal::Decimal,
    pub step_size: rust_decimal::Decimal,
}

// Legacy SymbolInfo for backwards compatibility
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
    // Try to get symbols from the catalog first
    let symbol_metas = state.get_symbol_meta(params.exchange.as_deref()).await;
    
    if !symbol_metas.is_empty() {
        // Group symbols by exchange
        let mut response_map: HashMap<String, Vec<SymbolMetaDto>> = HashMap::new();
        
        for meta in symbol_metas {
            let display_name = format!("{} / {}", meta.base, meta.quote);
            let symbol_key = format!("{}-{}", meta.base, meta.quote);
            
            let dto = SymbolMetaDto {
                symbol: symbol_key,
                base: meta.base,
                quote: meta.quote,
                display_name,
                price_precision: meta.price_precision,
                tick_size: meta.tick_size,
                min_qty: meta.min_qty,
                step_size: meta.step_size,
            };
            
            response_map.entry(meta.exchange.as_str().to_string())
                .or_insert_with(Vec::new)
                .push(dto);
        }
        
        let response: Vec<SymbolResponse> = response_map.into_iter()
            .map(|(exchange, symbols)| SymbolResponse { exchange, symbols })
            .collect();
            
        return Ok(Json(response));
    }
    
    // Fallback to curated list if no symbols are available
    let exchanges = state.get_exchange_info().await;
    let popular_symbols = get_popular_symbols();
    
    let mut response = Vec::new();
    
    if let Some(exchange_filter) = params.exchange {
        // Return symbols for specific exchange
        if let Some(symbols) = popular_symbols.get(&exchange_filter) {
            let symbol_dtos: Vec<SymbolMetaDto> = symbols.iter().map(|s| SymbolMetaDto {
                symbol: s.symbol.clone(),
                base: s.base.clone(),
                quote: s.quote.clone(),
                display_name: s.display_name.clone(),
                price_precision: get_price_precision(&s.base, &s.quote),
                tick_size: get_tick_size(&s.base, &s.quote),
                min_qty: rust_decimal::Decimal::new(1, 3), // 0.001
                step_size: rust_decimal::Decimal::new(1, 3), // 0.001
            }).collect();
            
            response.push(SymbolResponse {
                exchange: exchange_filter,
                symbols: symbol_dtos,
            });
        }
    } else {
        // Return symbols for all available exchanges
        for exchange in exchanges {
            if let Some(symbols) = popular_symbols.get(exchange.id.as_str()) {
                let symbol_dtos: Vec<SymbolMetaDto> = symbols.iter().map(|s| SymbolMetaDto {
                    symbol: s.symbol.clone(),
                    base: s.base.clone(),
                    quote: s.quote.clone(),
                    display_name: s.display_name.clone(),
                    price_precision: get_price_precision(&s.base, &s.quote),
                    tick_size: get_tick_size(&s.base, &s.quote),
                    min_qty: rust_decimal::Decimal::new(1, 3), // 0.001
                    step_size: rust_decimal::Decimal::new(1, 3), // 0.001
                }).collect();
                
                response.push(SymbolResponse {
                    exchange: exchange.id.as_str().to_string(),
                    symbols: symbol_dtos,
                });
            }
        }
    }
    
    Ok(Json(response))
}

/// POST /api/symbols/refresh - Refresh symbol metadata for an exchange
pub async fn refresh_symbols(
    Query(params): Query<SymbolsQuery>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match params.exchange {
        Some(exchange_name) => {
            match state.refresh_exchange_symbols(&exchange_name).await {
                Ok(_) => Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("Symbols refreshed for {}", exchange_name)
                }))),
                Err(e) => {
                    tracing::error!("Failed to refresh symbols for {}: {}", exchange_name, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        None => {
            // Refresh all exchanges
            let exchanges = state.get_exchange_info().await;
            for exchange in exchanges {
                if let Err(e) = state.refresh_exchange_symbols(exchange.id.as_str()).await {
                    tracing::warn!("Failed to refresh symbols for {}: {}", exchange.id.as_str(), e);
                }
            }
            
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Symbols refresh initiated for all exchanges"
            })))
        }
    }
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

/// Get appropriate price precision based on base and quote assets
fn get_price_precision(base: &str, quote: &str) -> u32 {
    match quote {
        "USDT" | "USDC" | "USD" | "BUSD" => {
            match base {
                // High-value coins get 2 decimal places
                "BTC" => 2,
                // Medium-value coins get 2-3 decimal places
                "ETH" | "BNB" | "SOL" | "ADA" | "DOT" | "AVAX" | "MATIC" | "LINK" | "UNI" => 2,
                // Lower-value coins get 4-6 decimal places
                "XRP" | "DOGE" | "SHIB" | "TRX" | "VET" | "HOT" => 4,
                // Very low-value coins get 6+ decimal places
                _ => 6,
            }
        },
        "BTC" => {
            // BTC pairs typically have 6-8 decimal places
            8
        },
        "ETH" => {
            // ETH pairs typically have 6-8 decimal places 
            6
        },
        _ => 6, // Default for other quote currencies
    }
}

/// Get appropriate tick size based on base and quote assets
fn get_tick_size(base: &str, quote: &str) -> String {
    match quote {
        "USDT" | "USDC" | "USD" | "BUSD" => {
            match base {
                "BTC" => "0.01".to_string(),
                "ETH" | "BNB" | "SOL" | "ADA" | "DOT" | "AVAX" | "MATIC" | "LINK" | "UNI" => "0.01".to_string(),
                "XRP" | "DOGE" | "SHIB" | "TRX" | "VET" | "HOT" => "0.0001".to_string(),
                _ => "0.000001".to_string(),
            }
        },
        "BTC" => "0.00000001".to_string(), // 1 satoshi
        "ETH" => "0.000001".to_string(),
        _ => "0.000001".to_string(),
    }
}