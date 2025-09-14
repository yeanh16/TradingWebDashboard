# Crypto Trading Dashboard - Backend

Rust-based backend for the crypto trading dashboard with real-time market data streaming.

## Features

- **Multi-exchange support**: Binance, Bybit (extensible architecture)
- **Real-time streaming**: WebSocket-based market data distribution
- **Normalized data model**: Unified format across all exchanges
- **Scalable architecture**: Tokio-based async runtime with modular design
- **Health monitoring**: Built-in health checks and metrics

## Architecture

```
Backend (Rust, Tokio)
├── api/           # HTTP server, WebSocket handlers, routes
├── core/          # Shared domain models, normalization, config
├── stream-hub/    # Real-time data broadcasting system  
├── cache/         # In-memory caching layer
└── exchanges/     # Exchange adapters
    ├── common/    # Shared exchange utilities
    ├── binance/   # Binance adapter
    └── bybit/     # Bybit adapter
```

## Quick Start

1. **Prerequisites**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Setup Environment**
   ```bash
   cd crypto-dash-backend
   cp .env.example .env
   # Edit .env as needed
   ```

3. **Run Development Server**
   ```bash
   cargo run -p api
   ```

The server will start on `http://localhost:8080` by default.

## API Endpoints

- **Health**: `GET /health`
- **Readiness**: `GET /ready`  
- **Exchanges**: `GET /api/exchanges`
- **WebSocket**: `GET /ws`

## Configuration

Environment variables (`.env`):

```
RUST_LOG=info,crypto_dash=debug
BIND_ADDR=0.0.0.0:8080
ENABLE_REDIS=false
REDIS_URL=redis://127.0.0.1:6379
EXCHANGES=binance,bybit
BOOK_DEPTH_DEFAULT=50
```

## Development

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Build release
cargo build --release

# Run with logs
RUST_LOG=debug cargo run -p api
```

## WebSocket Protocol

Connect to `/ws` and send JSON messages:

```json
{
  "op": "subscribe",
  "channels": [
    {
      "channel_type": "ticker",
      "exchange": "binance", 
      "symbol": {"base": "BTC", "quote": "USDT"}
    }
  ]
}
```

Server responses:
```json
{
  "type": "ticker",
  "payload": {
    "timestamp": "2024-01-01T00:00:00Z",
    "exchange": "binance",
    "symbol": {"base": "BTC", "quote": "USDT"},
    "bid": 43250.50,
    "ask": 43251.75,
    "last": 43251.00,
    "bid_size": 1.0,
    "ask_size": 1.0
  }
}
```