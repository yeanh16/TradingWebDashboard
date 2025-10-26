# Crypto Trading Dashboard

Live demo (Railway + Docker): https://tradingwebdashboard-production.up.railway.app/

This monorepo powers a real-time cryptocurrency dashboard delivered through three Dockerised microservices:

1. **Rust market data API** – ingests live exchange feeds, normalises candles/tickers, and exposes REST + WebSocket endpoints.
2. **Python AI insights service** – consumes the market API, enriches symbol data with technical indicators and narrative summaries using Google Gemini LLM AI API integration.
3. **Next.js frontend** – renders the trading UI and orchestrates calls to both backend services from the browser.

The services deploy independently (Railway, local Docker, or bare metal) while sharing a common domain model and environment conventions.

## Highlights
- Axum-based Rust API that streams normalised ticker updates, historical candles, and symbol metadata with WebSocket + REST interfaces.
- FastAPI microservice that derives insights (trend summaries, indicators) from market API candles and caches exchange metadata on demand.
- Exchange adapters for Binance and Bybit built on a shared trait with automatic reconnects and deterministic mock-data fallback.
- In-memory cache and publish/subscribe stream hub so multiple WebSocket clients can share upstream connections without duplication.
- Next.js 14 frontend with exchange/ticker selectors, latency monitoring, AI-powered insight panel, and graceful offline stubs.
- Batteries-included testing across Cargo, Pytest, and Jest/Playwright, plus a `run-tests.sh` helper for full-suite execution.

## Architecture Overview
```
                                     +------------------------------+
                                     |      crypto-dash-frontend    |
                                     |------------------------------|
                                     |  Next.js App (static export) |
               __ __________________ |  REST client + WS hook       |
               |                     |  Zustand / React Query store |
               |                      +-------+-------------+--------+
               | REST                                     |
               |                                          | REST + WebSocket
               v                                          v
+------------------------------+     +---------------------+--------------------+
|    crypto-dash-ai-backend    |     |        crypto-dash-backend               |
|------------------------------|     |------------------------------------------|
|  FastAPI insights service    |     |  Axum API (REST + WebSockets)            |
|  Technical indicator engine  |     |  Stream Hub (pub/sub topics)             |
|  Exchange metadata cache     |     |  Memory cache (tickers/order books)      |
+---------------+--------------+     |  Exchange adapters (Binance, Bybit)      |
                | REST (candles)     |  Catalog service (/api/symbols, candles) |
                +-------------------->+--------------------+--------------------+
                                                             |
                                                             | WebSocket / REST
                                                             v
                                              +-------------------------------+
                                              |     External Exchanges        |
                                              |  Binance & Bybit WS/REST APIs |
                                              +-------------------------------+
```
The frontend calls both backend services directly from the browser. The Rust API maintains upstream exchange sessions (or mock streams), caches snapshots, and exposes market data to clients. The Python AI service exclusively queries the market API for candles/metadata, computes indicators and summaries, and returns insight payloads rendered alongside the market data UI—without contacting external exchanges itself.

## Repository Layout
```
TradingWebDashboard/
  crypto-dash-backend/          # Rust workspace (market API + adapters)
    crates/api/                 # HTTP + WebSocket service
    crates/core/                # Shared models, config, normalisers
    crates/cache/               # In-memory cache facade
    crates/stream-hub/          # Broadcast hub for market data topics
    crates/exchanges/           # Common trait + Binance + Bybit adapters
    integration-tests/          # Cross-crate integration suite
  crypto-dash-ai-backend/       # FastAPI insights microservice
    app/                        # API routes, services, settings
    tests/                      # Pytest suites
  crypto-dash-frontend/         # Next.js 14 application
    src/app/                    # App Router entrypoint
    src/components/             # Dashboard UI building blocks
    src/lib/                    # API client + WebSocket hook
    tests/                      # Jest + Playwright suites
  run-tests.sh                  # Helper to run backend/frontend tests
  TESTING.md                    # Extended testing guide
```

## Getting Started

You can run each microservice locally via its native toolchain or build Docker images for parity with production.

### Rust Market API (`crypto-dash-backend`)
1. Install Rust (https://rustup.rs) and ensure `cargo` is on your `PATH`.
2. Copy the environment template:
   ```bash
   cd crypto-dash-backend
   cp .env.example .env
   ```
3. Start the API:
   ```bash
   cargo run -p api
   ```
   The server listens on `http://localhost:8080` and serves both REST endpoints and the `/ws` WebSocket.

Useful commands:
```
cargo check
cargo test
RUST_LOG=debug cargo run -p api
```

Docker:
```
docker build -t crypto-dash-backend -f crypto-dash-backend/Dockerfile crypto-dash-backend
docker run --rm -p 8080:8080 --name crypto-dash-backend crypto-dash-backend
```

### Python AI Insights (`crypto-dash-ai-backend`)
1. Install Python 3.11 and a virtual environment manager (or use the provided Dockerfile).
2. Copy environment variables:
   ```bash
   cd crypto-dash-ai-backend
   cp .env.example .env
   ```
3. Install dependencies and run the service:
   ```bash
   python -m venv .venv
   source .venv/bin/activate    # Windows: .venv\Scripts\activate
   pip install -r requirements.txt
   uvicorn app.main:app --host 0.0.0.0 --port 8000
   ```
   Set `AI_MARKET_API_BASE_URL` (default `http://localhost:8080`) so the insights service can reach the market API.

Docker:
```
docker build -t crypto-dash-ai -f crypto-dash-ai-backend/Dockerfile crypto-dash-ai-backend
docker run --rm -p 8000:8000 \
  -e AI_MARKET_API_BASE_URL=http://host.docker.internal:8080 \
  --name crypto-dash-ai crypto-dash-ai
```

### Frontend (`crypto-dash-frontend`)
1. Install Node.js 18+.
2. Copy environment variables:
   ```bash
   cd crypto-dash-frontend
   cp .env.local.example .env.local
   ```
3. Install dependencies and start dev mode:
   ```bash
   npm install
  npm run dev
   ```
   The UI is served from `http://localhost:3000`. Override `NEXT_PUBLIC_API_URL` and `NEXT_PUBLIC_AI_API_URL` as needed.

Docker (static export served by Nginx):
```
docker build -t crypto-dash-frontend -f crypto-dash-frontend/Dockerfile crypto-dash-frontend
docker run --rm -p 3000:80 --name crypto-dash-frontend crypto-dash-frontend
```

## Environment Configuration

**Market API (`crypto-dash-backend/.env.example`):**
- `BIND_ADDR` – host:port for Axum server (default `0.0.0.0:8080`).
- `EXCHANGES` – comma-separated adapters to start (`binance,bybit`).
- `RUST_LOG` – tracing filter (`info`, `debug`, etc.).
- `BOOK_DEPTH_DEFAULT`, `ORDER_BOOK_LIMITS_*` – optional snapshot tuning.
- `ENABLE_REDIS`, `REDIS_URL` – reserved for future distributed cache.

**AI Insights (`crypto-dash-ai-backend/.env.example`):**
- `AI_MARKET_API_BASE_URL` – URL of the market API service the AI should query.
- `AI_CORS_ORIGINS` – comma-delimited origins allowed for browser requests.
- `AI_DEFAULT_LIMIT`, `AI_HTTP_TIMEOUT_SECONDS` – request tuning knobs.

**Frontend (`crypto-dash-frontend/.env.local.example`):**
- `NEXT_PUBLIC_API_URL` – public market API base URL (REST + WebSocket).
- `NEXT_PUBLIC_AI_API_URL` – public AI insights base URL.

When deploying on Railway or another platform, configure each service independently, e.g. by setting the public URLs for browser-bound env vars and internal `.railway.internal` hosts for service-to-service calls.

## API Surface

### Market API (Rust/port 8080)
- `GET /health` – liveness probe.
- `GET /ready` – readiness including exchange status.
- `GET /api/exchanges` – active exchanges with connection diagnostics.
- `GET /api/symbols` – symbol metadata grouped by exchange (`?exchange=` to filter).
- `POST /api/symbols/refresh` – refresh metadata cache (optionally per exchange).
- `GET /api/candles` – OHLCV candles (`exchange`, `symbol`, `interval`, `limit` query params).
- WebSocket `ws://<host>/ws` – subscribe to `ticker`, `order_book_snapshot`, `order_book_delta`, etc. using `{ "op": "subscribe", "channels": [...] }` payloads.

The API caches the latest values so late subscribers receive immediate updates without new upstream connections. When an exchange is unavailable, adapters fall back to deterministic mock streams for development parity.

### AI Insights API (FastAPI/port 8000)
- `OPTIONS /insights` – CORS preflight helper for browsers.
- `GET /insights` – derive technical indicators and narrative summaries for symbols provided via `symbols`, `interval`, and optional `limit` query params.

The insights service uses the market API for candles and symbol metadata, then returns an `InsightsResponse` containing per-symbol indicators and a combined textual overview.

## Frontend Features

- `useWebSocket` hook manages subscriptions, reconnects, and ping/pong latency metrics for the market stream.
- REST client fetches exchange metadata, tickers, and AI insight payloads in parallel; React Query provides caching and automatic retries.
- Components (`ExchangeSelector`, `TickerSelector`, `TickerTable`, `InsightsPanel`, `LatencyBadge`, etc.) render the market view with AI summaries alongside recent price action.
- Tailwind CSS + `lucide-react` power the responsive UI; the app gracefully degrades to mock data when backends are unavailable.

## Testing

**Market API:**
```
cd crypto-dash-backend
cargo test
```

**AI Insights:**
```
cd crypto-dash-ai-backend
pytest
```

**Frontend:**
```
cd crypto-dash-frontend
npm test
npm run test:e2e   # requires the APIs running
```

**Full-suite helper:**
```
./run-tests.sh
./run-tests.sh --all   # include e2e + reports
```

See `TESTING.md` for deeper guidance, coverage examples, and debugging tips.

## Additional Documentation
- `crypto-dash-backend/README.md` – backend-specific design notes.
- `crypto-dash-ai-backend/README.md` – AI service details (coming soon).
- `crypto-dash-frontend/README.md` – frontend-specific notes.
- `TESTING.md` – comprehensive testing strategy across services.

## Roadmap
- Expose cached order-book snapshots and deltas in the frontend UI.
- Expand AI summarisation (multiple lookback windows, anomaly detection).
- Add additional exchange adapters (OKX, Coinbase, Kraken).
- Introduce optional Redis backbone for shared cache/state across replicas.
- Enhance dashboards with depth charts, comparative views, and alerting.

Contributions and feedback are welcome. Feel free to open issues, discussions, or pull requests with ideas or improvements.
