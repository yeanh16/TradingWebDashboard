# Product: Multi-Exchange Crypto Monitoring Dashboard

A web application that connects to multiple cryptocurrency exchanges (e.g., Binance, Bybit, OKX, Coinbase) to display **live best bid/ask**, **depth order books**, and **price charts** for any selected symbols. Users can select **two or more coins for side-by-side comparison**, pin favorites, and configure update granularities. The system normalizes heterogeneous exchange data into a unified schema and streams real-time updates to the frontend.

---

## Core Goals

1. **Low-latency, high-throughput streaming** of ticker and order book data from multiple exchanges.
2. **Normalized market data model** across venues (unified symbols, precision, quote/base).
3. **Flexible comparison UI**: compare any N symbols across exchanges and timeframes.
4. **Robustness**: automatic reconnects, backoff, rate-limit awareness, and partial degradation.
5. **Extensibility**: easy to add new exchanges, instruments, analytics, and persistence.

---

## High-Level Architecture

```
                              +--------------------+
                              |  Frontend (TS)     |
   +---------------------+    |  React/Next.js     |
   | External Exchanges  |    |  Recharts/LWCharts |
   |  (WS/REST)          |    |                    |
   +----------+----------+    +----+----------+----+
              |                     |          ^
              | WS/REST             | WebSock. | HTTP
              v                     v          |
        +-----+---------------------+----------+------+
        |            Backend (Rust, Tokio)            |
        |                                              |
        |  [Exchange Adapters]  -> unified model       |
        |  [Symbol Mapper]      -> normalize           |
        |  [Stream Hub]         -> fan-out WS/SSE      |
        |  [Cache Layer]        -> in-memory + Redis?  |
        |  [REST API]           -> discovery, history  |
        |  [Auth (opt)]         -> API keys, roles     |
        +-----------------------+----------------------+
                                |
                                v
                        +---------------+
                        | Persistence   |
                        | Redis (opt)   |
                        | Postgres (opt)|
                        +---------------+
```

* **Backend (Rust):**

  * Runtime: **Tokio**
  * HTTP framework: **Axum** (or Actix; Axum is great with Tokio + Tower)
  * WebSockets (tokio-tungstenite), **Serde** for JSON, **Reqwest** for REST fallbacks
  * Optional caching: **DashMap** (in-memory), **Redis** cluster for cross-node fan-out
  * Feature flags per exchange
* **Frontend (TypeScript):**

  * Framework: **Next.js** (App Router) with React 18
  * State: **Zustand** (stream state) + **TanStack Query** (HTTP caching)
  * Charts: **TradingView Lightweight-Charts** or **Recharts** for simple series
  * Styling: **Tailwind CSS** + shadcn/ui components
  * Transport: **native WebSocket** (binary/json) or **SSE** as fallback

---

## Data Model (Unified)

### Entities

* **Exchange**: `id`, `name`, `rate_limits`, `ws_url`, `rest_url`
* **Symbol**: `exchange_symbol` (e.g., `BTCUSDT`), `base`, `quote`, `price_precision`, `qty_precision`, `min_qty`, `tick_size`
* **Ticker**: `{ ts, exchange, symbol, bid, ask, last, bidSize, askSize }`
* **OrderBookSnapshot**: `{ ts, exchange, symbol, bids: [PriceLevel], asks: [PriceLevel], checksum? }`
* **OrderBookDelta**: `{ ts, exchange, symbol, bidsUpserts: [PriceLevel], asksUpserts: [PriceLevel], deletes? }`
* **Trade** (future): `{ ts, price, qty, side }`

`PriceLevel = { p: number, q: number }` with standardized decimal scaling.

### Normalization Rules

* Consistent decimal precision using `rust_decimal`.
* Symbols mapped to `{base-quote}` canonical form (e.g., `BTC-USDT`) using exchange-specific adapters.
* Time in **UTC ms** (`i64`) everywhere.
* Depth modes: `L10`, `L50`, `L200` (configurable).

---

## Backend (Rust) – Services & APIs

### Services

1. **Exchange Adapters**

   * One adapter per venue: handles auth, subscribe/unsubscribe, heartbeats, pings, backoff, and message translation -> unified messages.
   * Supports:

     * WS subscriptions: tickers, partial book, diff updates.
     * REST fallbacks for snapshots and symbol metadata.
   * Files live under `crates/exchanges/<exchange_name>/`.

2. **Symbol Registry & Mapper**

   * Bootstraps symbol metadata via REST.
   * Maintains exchange-to-canonical symbol maps.
   * Exposes lookups and validation.

3. **Stream Hub**

   * Central pub/sub routing of normalized frames to connected clients.
   * Option A (simple): in-process channels (broadcast) + backpressure.
   * Option B (cluster): Redis Pub/Sub or NATS for horizontal scale.

4. **Cache Layer**

   * Latest ticker per (exchange, symbol).
   * Last N book snapshots/deltas per stream (ring buffer).
   * Configurable retention for lightweight historical charts (e.g., 24h minute bars via on-the-fly aggregation).

5. **Persistence (Optional)**

   * Postgres + Timescale for OHLCV aggregation and historical queries.
   * Kafka/NATS for durable stream ingestion.

### Public API (HTTP + WS)

* **HTTP (REST)**

  * `GET /api/exchanges` → list supported exchanges & status
  * `GET /api/symbols?exchange=binance` → symbols + metadata
  * `GET /api/ticker?exchange=binance&symbol=BTC-USDT` → latest ticker
  * `GET /api/orderbook/snapshot?exchange=bybit&symbol=ETH-USDT&depth=50`
  * `GET /api/ohlcv?exchange=binance&symbol=BTC-USDT&tf=1m&limit=500` *(if enabled)*
  * `GET /api/compare?symbols=BTC-USDT,ETH-USDT&exchanges=binance,bybit` *(server-side merge)*

* **WebSocket**

  * `GET /ws`
  * **Client → Server** (JSON):

    * `{"op":"subscribe","channels":[{"type":"ticker","exchange":"binance","symbol":"BTC-USDT"},{"type":"orderbook","exchange":"bybit","symbol":"ETH-USDT","depth":50}] }`
    * `{"op":"unsubscribe","channels":[...]}`
  * **Server → Client** frames (JSON):

    * `{"type":"ticker", "payload": Ticker}`
    * `{"type":"orderbook_snapshot", "payload": OrderBookSnapshot}`
    * `{"type":"orderbook_delta", "payload": OrderBookDelta}`
    * `{"type":"info","message":"connected|heartbeat|rate_limited"}`

* **SSE** (fallback): `/sse?channels=...` with channel query DSL.

### Non-Functional

* **Performance Targets**

  * Ticker fan-out p95 < 100ms (WS-in to WS-out).
  * Order book updates sustained at 10–50 msgs/sec/stream (configurable).
* **Resilience**

  * Exponential backoff (jitter), auto resubscribe.
  * Per-exchange rate-limiters.
  * Dead-letter/poison message metrics.
* **Security**

  * No user auth initially; add JWT or API keys later.
  * Secrets via `dotenv`/KMS (never committed).
* **Observability**

  * **tracing** crate with `tracing-subscriber` (JSON logs)
  * `/health` (liveness) and `/ready` (readiness)
  * Prometheus metrics via `axum-prometheus`

---

## Frontend (TypeScript) – UX & Views

### Key Views

1. **Markets Overview**

   * Exchange selector, symbol search, favorites.
   * Live tick table (bid/ask/last, 24h change, spread).
   * Color cues for upticks/downticks.

2. **Compare Mode**

   * Multi-select symbols across exchanges.
   * Side-by-side cards: **Live ticker**, **Mini depth chart**, **Sparkline**.
   * Unified time controls (1m, 5m, 1h, 1d).

3. **Instrument Detail**

   * Large candlestick chart with volume & VWAP overlay.
   * Live order book (aggregated by price step), trades tape (future).
   * Depth heatmap (optional).

4. **Settings**

   * Update frequency, depth level, theme, latency indicator.
   * Manage API keys (future), layout preferences.

### Component Highlights

* `WsProvider` (context) manages WS connection + auto-reconnect.
* `useStream(channels)` hook returns live data stores (Zustand).
* `OrderBookView` renders top N levels with delta highlighting.
* `CompareGrid` responsive grid for N instruments (2–8).
* `LatencyBadge` pings server, shows RTT.

---

## Project Structure (Monorepo Optional)

You can implement as a **polyrepo** (separate folders) or a **monorepo** via Turborepo. Below is a **two-repo** baseline with clear expansion points.

### Backend: `crypto-dash-backend/` (Rust, Cargo workspace)

```
crypto-dash-backend/
├─ Cargo.toml                      # workspace
├─ rust-toolchain.toml
├─ .env.example
├─ crates/
│  ├─ api/                         # Axum app, routes, WS, SSE
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ main.rs                # bootstrap, router, layers
│  │     ├─ routes/
│  │     │  ├─ exchanges.rs
│  │     │  ├─ symbols.rs
│  │     │  ├─ ticker.rs
│  │     │  ├─ orderbook.rs
│  │     │  ├─ ohlcv.rs            # feature gated
│  │     │  └─ health.rs
│  │     ├─ ws/
│  │     │  ├─ server.rs           # WS handshake, session mgmt
│  │     │  └─ frames.rs           # message schema (serde)
│  │     ├─ sse.rs
│  │     ├─ state.rs               # AppState: hubs, caches
│  │     ├─ metrics.rs
│  │     └─ error.rs
│  ├─ core/                        # shared domain & utils
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ model.rs               # Ticker, OrderBook*, Symbol
│  │     ├─ normalize.rs           # symbol normalization
│  │     ├─ time.rs
│  │     ├─ config.rs              # env, feature flags
│  │     └─ prelude.rs
│  ├─ stream-hub/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ hub.rs                 # broadcast channels
│  │     ├─ topics.rs              # topic keys
│  │     └─ backpressure.rs
│  ├─ cache/
│  │  ├─ Cargo.toml
│  │  └─ src/
│  │     ├─ mem.rs                 # DashMap, ring buffers
│  │     ├─ redis.rs               # optional feature
│  │     └─ ohlcv.rs               # aggregation utils
│  └─ exchanges/
│     ├─ binance/
│     │  ├─ Cargo.toml
│     │  └─ src/
│     │     ├─ ws.rs               # subscribe tickers/books
│     │     ├─ rest.rs             # symbols, snapshot
│     │     ├─ mapper.rs           # -> unified model
│     │     └─ types.rs            # exchange-native structs
│     ├─ bybit/
│     │  └─ ...
│     └─ common/
│        ├─ Cargo.toml
│        └─ src/
│           ├─ client.rs           # reqwest, ws client helpers
│           ├─ ratelimit.rs        # governor/LeakyBucket
│           ├─ retry.rs            # exponential backoff
│           └─ auth.rs             # signed REST (future)
├─ scripts/
│  ├─ dev_certs.sh
│  └─ run_local.sh
└─ tests/
   ├─ e2e_orderbook.rs
   └─ e2e_ticker.rs
```

**Backend Key Files (initial content intent):**

* `api/src/main.rs`: set up Axum router, add CORS, tracing, metrics. Start adapters on background tasks, inject handles into `AppState`.
* `api/src/ws/server.rs`: upgrades to WS, parses subscribe/unsubscribe, attaches to hub topics, serializes unified frames.
* `core/src/model.rs`: serde models; unit tests for round-trip.
* `exchanges/*/ws.rs`: exchange subscription builders, message parsing, heartbeats.
* `cache/mem.rs`: `DashMap<(Exchange,Symbol), Ticker>`, `OrderBookStore` with ring buffers.

### Frontend: `crypto-dash-frontend/` (Next.js + TS)

```
crypto-dash-frontend/
├─ package.json
├─ pnpm-lock.yaml
├─ next.config.mjs
├─ .env.local.example
├─ src/
│  ├─ app/
│  │  ├─ layout.tsx
│  │  ├─ page.tsx                   # Markets overview
│  │  ├─ compare/
│  │  │  └─ page.tsx                # Compare mode
│  │  └─ instrument/[symbol]/page.tsx
│  ├─ components/
│  │  ├─ TickerTable.tsx
│  │  ├─ OrderBookView.tsx
│  │  ├─ CandlesChart.tsx
│  │  ├─ CompareGrid.tsx
│  │  ├─ ExchangeSelector.tsx
│  │  ├─ SymbolSearch.tsx
│  │  ├─ LatencyBadge.tsx
│  │  └─ ThemeToggle.tsx
│  ├─ lib/
│  │  ├─ api.ts                     # REST helpers (fetch)
│  │  ├─ ws.ts                      # WS connect/retry
│  │  ├─ streams.ts                 # subscribe DSL
│  │  ├─ types.ts                   # Shared TS types
│  │  └─ format.ts                  # number/time utils
│  ├─ store/
│  │  ├─ useSymbols.ts
│  │  ├─ useTickers.ts
│  │  ├─ useOrderBooks.ts
│  │  └─ useSettings.ts
│  ├─ styles/
│  │  └─ globals.css
│  └─ hooks/
│     └─ useHeartbeat.ts
└─ public/
   └─ favicon.ico
```

**Frontend Key Files (initial content intent):**

* `lib/ws.ts`: single WS instance; exponential backoff; queue pending subs until open.
* `lib/streams.ts`: `subscribeTicker({exchange,symbol})` returns an unsubscribe fn; pushes data into Zustand stores.
* `components/CandlesChart.tsx`: wraps Lightweight-Charts; exposes `setData`, `update` handlers.

---

## Initialisation & Local Dev

### Backend

```bash
# prerequisites: Rust stable, just, Redis (optional)
cd crypto-dash-backend
cp .env.example .env   # set RUST_LOG, ENABLE_REDIS=false, EXCHANGES=binance,bybit
cargo run -p api
```

`.env.example` (suggested):

```
RUST_LOG=info,crypto=debug
BIND_ADDR=0.0.0.0:8080
ENABLE_REDIS=false
REDIS_URL=redis://127.0.0.1:6379
EXCHANGES=binance,bybit
BOOK_DEPTH_DEFAULT=50
```

### Frontend

```bash
cd crypto-dash-frontend
pnpm i
cp .env.local.example .env.local   # NEXT_PUBLIC_API_URL=http://localhost:8080
pnpm dev
```

`.env.local.example`:

```
NEXT_PUBLIC_API_URL=http://localhost:8080
```

---

## Exchange Adapter Pattern (How to add a venue)

1. Create `crates/exchanges/<venue>/` with `ws.rs`, `rest.rs`, `mapper.rs`, `types.rs`.
2. Implement `ExchangeAdapter` trait in `exchanges/common/`:

   ```rust
   #[async_trait]
   pub trait ExchangeAdapter: Send + Sync {
       fn id(&self) -> &'static str;
       async fn start(&self, hub: HubHandle, cache: CacheHandle, symbols: &[SymbolSpec]) -> Result<()>;
       async fn subscribe(&self, subs: &[Channel]) -> Result<()>;
       async fn unsubscribe(&self, subs: &[Channel]) -> Result<()>;
   }
   ```
3. Normalize all outbound frames to `core::model::*`.
4. Register adapter in `api/src/main.rs`.

---

## Testing Strategy

* **Unit tests**: model (serde), mappers (sample payloads), symbol normalization.
* **Integration tests**: spin up API with mock exchange servers (recorded WS fixtures).
* **E2E**: headless Playwright to verify UI displays live deltas and compare mode.
* **Load tests**: `vegeta`/`k6` for REST; a custom WS flooder for fan-out.

---

## Observability & Ops

* Metrics: message rates per exchange/channel, dropped frames, reconnect counts, fan-out lag, p95/99 encode times.
* Health:

  * `/health` returns process liveness.
  * `/ready` checks at least one active upstream WS and recent hub activity.
* Logs: structured JSON with trace IDs; sample field: `{"exchange":"binance","symbol":"BTC-USDT","event":"orderbook_delta","levels":50}`.

---

## Security & Compliance (Roadmap)

* **Tier 1**: No user auth; CORS restricted to your domains; rate-limit public WS.
* **Tier 2**: API keys + JWT (per-user layouts, favorites).
* **Tier 3**: RBAC, audit logs, signed user actions (if you add trading later).

---

## Performance Notes

* Use **binary WS frames** (MessagePack) if JSON becomes bottleneck (feature flag).
* Apply **price level aggregation** server-side for depth > L50 to reduce payload size.
* **Coalesce** low-priority deltas at 50–100ms cadence per symbol to smooth bursts.

---

## Backlog / Future Features (pre-wired)

* Trades tape with clustering by aggressor side.
* Derived metrics: spread %, book imbalance, microprice, volatility bands.
* Alerts: spread threshold, breakout, volume spike (SSE push + toasts).
* Historical persistence: OHLCV aggregation and replay.
* Layouts: multi-pane save/restore per user; shareable permalinks.
* Mobile responsive layout and PWA offline shell.
* Execute mock orders on a paper engine (strictly non-custodial, non-trading at first).

---

## Acceptance Criteria (MVP)

* Users can select **any N symbols** across supported exchanges and see:

  * Live **best bid/ask** updating in near real-time.
  * A **top-N order book** view with rolling deltas.
  * A **basic time-series price chart** that updates at least once per second.
* Compare mode renders **at least 2–4 symbols** smoothly on a laptop.
* Resilience: recovers automatically from WS disconnects within 5s (configurable).
