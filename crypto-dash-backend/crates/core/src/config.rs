use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind_addr: String,
    pub exchanges: Vec<String>,
    pub enable_redis: bool,
    pub redis_url: String,
    pub book_depth_default: u16,
    pub log_level: String,
    pub enable_real_connections: bool, // New flag for testing
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            exchanges: env::var("EXCHANGES")
                .unwrap_or_else(|_| "binance,bybit".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            enable_redis: env::var("ENABLE_REDIS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            book_depth_default: env::var("BOOK_DEPTH_DEFAULT")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            enable_real_connections: env::var("ENABLE_REAL_CONNECTIONS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8080".to_string(),
            exchanges: vec!["binance".to_string(), "bybit".to_string()],
            enable_redis: false,
            redis_url: "redis://127.0.0.1:6379".to_string(),
            book_depth_default: 50,
            log_level: "info".to_string(),
            enable_real_connections: true,
        }
    }
}