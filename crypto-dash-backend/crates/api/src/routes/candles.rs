use crate::state::AppState;
use anyhow::{anyhow, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use crypto_dash_core::model::{Candlestick, MarketType};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, warn};

const DEFAULT_CANDLE_LIMIT: usize = 200;
const MAX_CANDLE_LIMIT: usize = 1000;
const CACHE_TTL_SECONDS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct CandlesQuery {
    pub exchange: String,
    pub symbol: String,
    pub interval: String,
    pub limit: Option<usize>,
    pub market_type: Option<MarketType>,
}

#[derive(Debug, Serialize)]
pub struct CandlesResponse {
    pub exchange: String,
    pub symbol: String,
    pub market_type: MarketType,
    pub interval: String,
    pub limit: usize,
    pub candles: Vec<Candlestick>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedCandles {
    fetched_at: DateTime<Utc>,
    candles: Vec<Candlestick>,
}

pub async fn get_candles(
    State(state): State<AppState>,
    Query(params): Query<CandlesQuery>,
) -> Result<Json<CandlesResponse>, StatusCode> {
    let exchange = params.exchange.trim().to_lowercase();
    if exchange.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let market_type = params.market_type.unwrap_or(MarketType::Spot);

    let limit = params.limit.unwrap_or(DEFAULT_CANDLE_LIMIT);
    if limit == 0 || limit > MAX_CANDLE_LIMIT {
        return Err(StatusCode::BAD_REQUEST);
    }

    let interval = match CandleInterval::parse(params.interval.trim()) {
        Some(value) => value,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    let normalized_symbol = normalize_symbol(&params.symbol);
    if normalized_symbol.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let cache_key = format!(
        "candles:{}:{}:{}:{}:{}",
        exchange,
        market_label(market_type),
        normalized_symbol,
        interval.cache_key_fragment(),
        limit
    );

    let cache = state.cache.clone();
    if let Ok(Some(cached)) = cache.get::<CachedCandles>(&cache_key).await {
        if !is_stale(&cached) {
            return Ok(Json(CandlesResponse {
                exchange: exchange.clone(),
                symbol: normalized_symbol,
                market_type,
                interval: params.interval,
                limit,
                candles: cached.candles,
                cached: true,
            }));
        }
    }

    let client = state.http_client.clone();
    let candles = match fetch_exchange_candles(
        &client,
        &exchange,
        &normalized_symbol,
        &interval,
        limit,
        market_type,
    )
    .await
    {
        Ok(data) => data,
        Err(err) => {
            error!(
                exchange = %exchange,
                symbol = %normalized_symbol,
                interval = %params.interval,
                "Failed to fetch candles: {err:?}"
            );
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    let cached_payload = CachedCandles {
        fetched_at: Utc::now(),
        candles: candles.clone(),
    };

    if let Err(err) = cache.set(&cache_key, &cached_payload).await {
        warn!(
            exchange = %exchange,
            symbol = %normalized_symbol,
            interval = %params.interval,
            "Failed to cache candles: {err:?}"
        );
    }

    Ok(Json(CandlesResponse {
        exchange,
        symbol: normalized_symbol,
        market_type,
        interval: params.interval,
        limit,
        candles,
        cached: false,
    }))
}

#[derive(Debug, Clone)]
enum CandleInterval {
    Minutes(u32),
    Hours(u32),
    Days(u32),
    Weeks(u32),
    Months(u32),
}

impl CandleInterval {
    fn parse(value: &str) -> Option<Self> {
        if value.is_empty() {
            return None;
        }

        let trimmed = value.trim();
        let (number_part, unit) = trimmed.split_at(trimmed.len().saturating_sub(1));
        let unit_char = unit.chars().next()?;
        let magnitude: u32 = number_part.parse().ok()?;

        match unit_char {
            'm' => Some(Self::Minutes(magnitude)),
            'h' => Some(Self::Hours(magnitude)),
            'd' => Some(Self::Days(magnitude)),
            'w' => Some(Self::Weeks(magnitude)),
            'M' => Some(Self::Months(magnitude)),
            'H' => Some(Self::Hours(magnitude)),
            'D' => Some(Self::Days(magnitude)),
            'W' => Some(Self::Weeks(magnitude)),
            _ => None,
        }
    }

    fn cache_key_fragment(&self) -> String {
        match self {
            Self::Minutes(v) => format!("{}m", v),
            Self::Hours(v) => format!("{}h", v),
            Self::Days(v) => format!("{}d", v),
            Self::Weeks(v) => format!("{}w", v),
            Self::Months(v) => format!("{}M", v),
        }
    }

    fn to_binance_interval(&self) -> String {
        self.cache_key_fragment().to_lowercase()
    }

    fn to_bybit_interval(&self) -> String {
        match self {
            Self::Minutes(v) => v.to_string(),
            Self::Hours(v) => (v * 60).to_string(),
            Self::Days(v) => {
                if *v == 1 {
                    "D".to_string()
                } else {
                    (v * 1_440).to_string()
                }
            }
            Self::Weeks(v) => {
                if *v == 1 {
                    "W".to_string()
                } else {
                    (v * 10_080).to_string()
                }
            }
            Self::Months(v) => {
                if *v == 1 {
                    "M".to_string()
                } else {
                    (v * 43_200).to_string()
                }
            }
        }
    }
}

async fn fetch_exchange_candles(
    client: &Client,
    exchange: &str,
    symbol: &str,
    interval: &CandleInterval,
    limit: usize,
    market_type: MarketType,
) -> Result<Vec<Candlestick>> {
    match exchange {
        "binance" => fetch_binance_candles(client, symbol, interval, limit, market_type).await,
        "bybit" => fetch_bybit_candles(client, symbol, interval, limit, market_type).await,
        _ => Err(anyhow!("Unsupported exchange: {exchange}")),
    }
}

async fn fetch_binance_candles(
    client: &Client,
    symbol: &str,
    interval: &CandleInterval,
    limit: usize,
    market_type: MarketType,
) -> Result<Vec<Candlestick>> {
    let base_url = match market_type {
        MarketType::Spot => "https://api.binance.com/api/v3/klines",
        MarketType::Perpetual => "https://fapi.binance.com/fapi/v1/klines",
    };

    let response = client
        .get(base_url)
        .query(&[
            ("symbol", symbol),
            ("interval", &interval.to_binance_interval()),
            ("limit", &limit.to_string()),
        ])
        .send()
        .await?
        .error_for_status()?;

    let raw: Vec<Vec<serde_json::Value>> = response.json().await?;

    raw.into_iter()
        .map(|entry| parse_binance_entry(&entry))
        .collect()
}

fn parse_binance_entry(entry: &[serde_json::Value]) -> Result<Candlestick> {
    if entry.len() < 6 {
        return Err(anyhow!("Unexpected kline payload length"));
    }

    let open_time = entry[0]
        .as_i64()
        .ok_or_else(|| anyhow!("Missing open time"))?;

    let open = parse_decimal(&entry[1])?;
    let high = parse_decimal(&entry[2])?;
    let low = parse_decimal(&entry[3])?;
    let close = parse_decimal(&entry[4])?;
    let volume = parse_decimal(&entry[5])?;

    let timestamp = Utc
        .timestamp_millis_opt(open_time)
        .single()
        .ok_or_else(|| anyhow!("Invalid timestamp"))?;

    Ok(Candlestick {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

async fn fetch_bybit_candles(
    client: &Client,
    symbol: &str,
    interval: &CandleInterval,
    limit: usize,
    market_type: MarketType,
) -> Result<Vec<Candlestick>> {
    let url = "https://api.bybit.com/v5/market/kline";

    let category = match market_type {
        MarketType::Spot => "spot",
        MarketType::Perpetual => "linear",
    };

    let response = client
        .get(url)
        .query(&[
            ("category", category),
            ("symbol", symbol),
            ("interval", &interval.to_bybit_interval()),
            ("limit", &limit.to_string()),
        ])
        .send()
        .await?
        .error_for_status()?;

    let payload: BybitKlineResponse = response.json().await?;

    if payload.ret_code != 0 {
        return Err(anyhow!(
            "Bybit returned error {}: {}",
            payload.ret_code,
            payload.ret_msg
        ));
    }

    let result = payload
        .result
        .ok_or_else(|| anyhow!("Missing result in Bybit response"))?;

    result
        .list
        .into_iter()
        .map(|entry| parse_bybit_entry(&entry))
        .collect()
}

fn parse_bybit_entry(entry: &[String]) -> Result<Candlestick> {
    if entry.len() < 6 {
        return Err(anyhow!("Unexpected Bybit kline payload length"));
    }

    let open_time: i64 = entry[0].parse().map_err(|_| anyhow!("Invalid timestamp"))?;

    let timestamp = Utc
        .timestamp_millis_opt(open_time)
        .single()
        .ok_or_else(|| anyhow!("Invalid timestamp"))?;

    let open = Decimal::from_str(&entry[1])?;
    let high = Decimal::from_str(&entry[2])?;
    let low = Decimal::from_str(&entry[3])?;
    let close = Decimal::from_str(&entry[4])?;
    let volume = Decimal::from_str(&entry[5])?;

    Ok(Candlestick {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitKlineResponse {
    ret_code: i32,
    ret_msg: String,
    result: Option<BybitKlineResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitKlineResult {
    list: Vec<Vec<String>>,
}

fn parse_decimal(value: &serde_json::Value) -> Result<Decimal> {
    let text = value
        .as_str()
        .ok_or_else(|| anyhow!("Expected string for decimal"))?;
    Decimal::from_str(text).map_err(|err| anyhow!("Failed to parse decimal: {err}"))
}

fn normalize_symbol(symbol: &str) -> String {
    symbol
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn market_label(market_type: MarketType) -> &'static str {
    match market_type {
        MarketType::Spot => "spot",
        MarketType::Perpetual => "perpetual",
    }
}

fn is_stale(cached: &CachedCandles) -> bool {
    Utc::now().signed_duration_since(cached.fetched_at) > Duration::seconds(CACHE_TTL_SECONDS)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_binance_candles_returns_data() {
        let client = Client::new();
        let interval = CandleInterval::Minutes(1);
        let result = fetch_exchange_candles(
            &client,
            "binance",
            "BTCUSDT",
            &interval,
            5,
            MarketType::Spot,
        )
        .await
        .expect("failed to fetch binance candles");

        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn fetch_bybit_candles_returns_data() {
        let client = Client::new();
        let interval = CandleInterval::Minutes(1);
        let result =
            fetch_exchange_candles(&client, "bybit", "BTCUSDT", &interval, 5, MarketType::Spot)
                .await
                .expect("failed to fetch bybit candles");

        assert!(!result.is_empty());
    }
}
