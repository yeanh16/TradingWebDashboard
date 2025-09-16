# Crypto Trading Dashboard

Monorepo for a real-time cryptocurrency market dashboard. A Rust backend ingests and normalizes exchange data and exposes REST + WebSocket APIs, while a Next.js frontend renders an interactive trading view.

## Highlights
- Axum-based Rust API that streams normalized ticker updates and Binance order-book snapshots over WebSockets.
- Exchange adapters for Binance and Bybit built on a shared trait with automatic reconnects and mock-data fallback.
- Exchange catalog service that fetches symbol metadata from upstream REST APIs, caches the results, and exposes them via `/api/symbols`.
- In-memory cache and publish/subscribe stream hub so multiple WebSocket clients can share upstream connections without duplication.
- Next.js 14 frontend with exchange and ticker selectors, latency monitoring, and a live markets table with graceful offline fallback.
- Batteries-included testing across cargo, Jest, and Playwright, plus a `run-tests.sh` helper to execute full suites locally.

## Architecture Overview
```
+------------------------------+        +----------------------------------------------+        +-----------------------------+
|      crypto-dash-frontend    |        |             crypto-dash-backend              |        |     External Exchanges      |
|------------------------------|        |----------------------------------------------|        |-----------------------------|
|  Next.js App Router (pages)  |        |  Axum API (REST + WebSocket gateway)         |        |  Binance WS / REST APIs     |
|  Exchange/Ticker components  |        |  Stream Hub (pub/sub topics)                 |        |  Bybit WS / REST APIs       |
|  useWebSocket hook           |<------>|  Memory Cache (latest tickers/order books)   |<-------+  Live feeds and metadata    |
|  apiClient (REST helper)     |  WS    |  Exchange Catalog (symbol metadata service)  |        |                             |
|  Zustand / React Query store |  REST  |  Exchange Adapters (Binance, Bybit)          |        |                             |
|  Tailwind-powered UI         |<------>|      - shared WS client and mock generator   |        |                             |
|------------------------------|        |----------------------------------------------|        |-----------------------------|
            ^                                      ^                 ^
            |                                      |                 |
            +---------------------------+----------+-----------------+
                                        |
                                        v
                         run-tests.sh / TESTING.md orchestration
```
The diagram highlights how the Next.js frontend consumes REST and WebSocket data exposed by the Axum API. Exchange adapters maintain upstream sessions (or fall back to deterministic mock data), publish normalized events into the stream hub, and persist snapshots in the cache, while the catalog fetches and caches symbol metadata. Frontend stores feed React components via Zustand and React Query.js frontend consumes REST and WebSocket data exposed by the Axum API. Adapters (Binance, Bybit) maintain upstream exchange connections or generate mock data, publish normalized events into the stream hub, and persist snapshots in the cache. The exchange catalog fetches and caches symbol metadata. Frontend data flows through Zustand/React Query into UI components.js UI talks to the Axum API over REST and WebSockets. Exchange adapters maintain upstream sessions (or fall back to deterministic mock data) and publish normalized events into the stream hub and cache, while the catalog fetches symbol metadata from exchange REST APIs.

## Repository Layout
```
TradingWebDashboard/
  crypto-dash-backend/          # Rust workspace
    crates/
      api/                      # HTTP + WebSocket service
      core/                     # Shared models, config, normalization helpers
      stream-hub/               # Broadcast hub for market data topics
      cache/                    # In-memory cache facade
      exchanges/
        common/                 # Exchange adapter trait, WS client, mock generator
        binance/                # Binance integration
        bybit/                  # Bybit integration
    integration-tests/          # Cross-crate integration suite
    tests/                      # High-level backend tests
    scripts/                    # Developer scripts (dev.sh, etc.)
  crypto-dash-frontend/         # Next.js 14 application
    src/app/                    # App Router entrypoint
    src/components/             # Dashboard UI building blocks
    src/lib/                    # API client + WebSocket hook
    tests/                      # Jest + Playwright suites
  run-tests.sh                  # Helper to run backend and frontend tests
  TESTING.md                    # Extended testing guide
```

## Getting Started

### Backend (Rust)
1. Install Rust (https://rustup.rs) and ensure `cargo` is on your `PATH`.
2. Copy the example environment file and adjust as needed:
   ```
   cd crypto-dash-backend
   cp .env.example .env
   ```
3. Run the API service:
   ```
   cargo run -p api
   ```
   The server listens on `http://localhost:8080` by default and serves both REST endpoints and the `/ws` WebSocket.

Useful commands:
```
cargo check          # fast validation
cargo test           # workspace tests
RUST_LOG=debug cargo run -p api
```

### Frontend (Next.js)
1. Install Node.js 18+.
2. Copy the example env file:
   ```
   cd crypto-dash-frontend
   cp .env.local.example .env.local
   ```
3. Install dependencies and start the dev server:
   ```
   npm install
   npm run dev
   ```
   The UI is served from `http://localhost:3000` and expects the backend at `http://localhost:8080` unless `NEXT_PUBLIC_API_URL` is overridden.

Other scripts:
```
npm run build        # production build
npm run lint         # ESLint
npm test             # Jest unit + integration tests
npm run test:e2e     # Playwright end-to-end tests
```

## Environment Configuration

Backend `.env` options (see `crypto-dash-backend/.env.example`):
- `BIND_ADDR`: host and port for the Axum server (default `0.0.0.0:8080`).
- `EXCHANGES`: comma-separated list of adapters to start (`binance,bybit`).
- `RUST_LOG`: tracing filter (default `info` with crate overrides).
- `ENABLE_REDIS` / `REDIS_URL`: reserved for the upcoming Redis cache integration.
- `BOOK_DEPTH_DEFAULT`: default order-book depth when requesting snapshots (Binance adapter honours this setting).

Frontend `.env.local`:
- `NEXT_PUBLIC_API_URL`: base URL for REST + WebSocket endpoints (defaults to `http://localhost:8080`).

## API Surface

REST endpoints (served from `http://localhost:8080`):
- `GET /health` - liveness status.
- `GET /ready` - readiness status including dependency health.
- `GET /api/exchanges` - active exchanges with connection status.
- `GET /api/symbols` - symbol metadata grouped by exchange. Use `?exchange=binance` to filter.
- `POST /api/symbols/refresh` - refresh metadata for all exchanges or a specific one via `?exchange=`.

WebSocket endpoint: `ws://localhost:8080/ws`

Subscribe to live channels by sending:
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

Server messages follow the `StreamMessage` enum:
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
Additional message types include `order_book_snapshot`, `order_book_delta`, `info`, and `error`. Clients can also send `{"op":"ping"}` and receive latency-aware `info` responses.

The backend caches the latest values so new subscribers receive updates without having to trigger new upstream connections. If an upstream exchange is unreachable, adapters fall back to deterministic mock data so local development can continue.

## Frontend Features

- Uses a shared `useWebSocket` hook for automatic reconnects, ping/pong latency tracking, and selective subscribe/unsubscribe per ticker.
- Displays exchange metadata and symbols fetched from the REST API with graceful degradation if the API is offline (defaults to a curated list).
- `ExchangeSelector`, `TickerSelector`, `TickerTable`, and `LatencyBadge` components compose the dashboard. The table highlights price movements and shows mock values whenever the WebSocket is disconnected.
- Styling through Tailwind CSS with responsive layouts; icons via `lucide-react`.

## Testing

Backend:
```
cd crypto-dash-backend
cargo test            # unit + integration tests
```

Frontend:
```
cd crypto-dash-frontend
npm test              # Jest unit + integration
npm run test:e2e      # Playwright (requires backend + frontend running)
```

All-in-one helper:
```
./run-tests.sh --help
./run-tests.sh                # backend + frontend unit tests
./run-tests.sh --all          # include E2E, performance placeholders, reports
```

See `TESTING.md` for deep-dives, coverage tips, and debugging commands.

## Additional Documentation

- `crypto-dash-backend/README.md` - backend-focused details.
- `crypto-dash-frontend/README.md` - frontend-specific notes.
- `TESTING.md` - comprehensive testing strategy.

## Roadmap

Current focus areas include:
- Exposing cached order-book snapshots and deltas to the frontend UI.
- Surfacing additional exchanges (OKX, Coinbase) through the adapter interface.
- Optional Redis backing store for distributed deployments.
- Enhanced frontend visualizations (depth charts, comparative views).

Contributions and feedback are welcome; open an issue or PR with ideas or improvements.







