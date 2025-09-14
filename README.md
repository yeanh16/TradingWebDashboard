# Crypto Trading Dashboard

A high-performance, real-time cryptocurrency trading dashboard that aggregates market data from multiple exchanges. Built with Rust backend for low-latency streaming and React frontend for responsive user experience.

## 🚀 Features

### Core Capabilities
- **Multi-Exchange Support**: Binance, Bybit (easily extensible)
- **Real-Time Streaming**: Sub-100ms WebSocket data distribution
- **Unified Data Model**: Normalized market data across all venues
- **Live Order Books**: Top-N level updates with delta highlighting
- **Ticker Monitoring**: Best bid/ask with 24h change tracking
- **Exchange Comparison**: Side-by-side market analysis

### Technical Highlights
- **Low Latency**: Rust/Tokio backend optimized for performance
- **Scalable Architecture**: Modular design with clean separation
- **Type Safety**: Full Rust + TypeScript implementation
- **Real-Time UI**: React with WebSocket integration
- **Responsive Design**: Mobile-friendly Tailwind CSS styling
- **Health Monitoring**: Built-in metrics and connection status

## 🏗️ Architecture

```
┌─────────────────┐     ┌──────────────────────────────────────┐     ┌─────────────────┐
│   Exchanges     │────▶│           Backend (Rust)             │────▶│  Frontend (TS)  │
│                 │     │                                      │     │                 │
│ • Binance       │     │ ┌─────────────┐ ┌──────────────────┐ │     │ • React/Next.js │
│ • Bybit         │     │ │ Exchange    │ │    Stream Hub    │ │     │ • WebSocket     │
│ • OKX (future)  │     │ │ Adapters    │ │   (Pub/Sub)      │ │     │ • Recharts      │
│ • Coinbase (f.) │     │ └─────────────┘ └──────────────────┘ │     │ • Tailwind CSS  │
│                 │     │ ┌─────────────┐ ┌──────────────────┐ │     │                 │
│                 │     │ │   Cache     │ │   HTTP API       │ │     │                 │
│                 │     │ │ (DashMap)   │ │   (Axum)         │ │     │                 │
│                 │     │ └─────────────┘ └──────────────────┘ │     │                 │
└─────────────────┘     └──────────────────────────────────────┘     └─────────────────┘
        │                                   │                                   │
    WebSocket                          HTTP + WebSocket                    HTTP + WebSocket
    REST APIs                          endpoints                          client connections
```

## 📁 Project Structure

```
TradingWebDashboard/
├── crypto-dash-backend/           # Rust backend
│   ├── crates/
│   │   ├── api/                   # HTTP server & WebSocket handlers
│   │   ├── core/                  # Domain models & utilities
│   │   ├── stream-hub/            # Real-time broadcasting
│   │   ├── cache/                 # In-memory data store
│   │   └── exchanges/             # Exchange adapters
│   │       ├── common/            # Shared utilities
│   │       ├── binance/           # Binance integration
│   │       └── bybit/             # Bybit integration
│   ├── scripts/                   # Development tools
│   └── README.md
│
├── crypto-dash-frontend/          # TypeScript frontend
│   ├── src/
│   │   ├── app/                   # Next.js pages
│   │   ├── components/            # React components
│   │   ├── lib/                   # API client & utilities
│   │   └── styles/                # Tailwind CSS
│   └── README.md
│
└── README.md                      # This file
```

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.70+ (`rustup`, `cargo`)
- **Node.js** 18+ & npm
- **Git**

### 1. Clone Repository

```bash
git clone <repository-url>
cd TradingWebDashboard
```

### 2. Start Backend

```bash
cd crypto-dash-backend
cp .env.example .env          # Configure as needed
cargo run -p api              # Starts on http://localhost:8080
```

### 3. Start Frontend (New Terminal)

```bash
cd crypto-dash-frontend
cp .env.local.example .env.local    # Configure as needed
npm install
npm run dev                         # Starts on http://localhost:3000
```

### 4. Access Dashboard

Open http://localhost:3000 in your browser and explore:

- **Markets Overview**: Live ticker data from selected exchanges
- **Exchange Selection**: Toggle between Binance, Bybit
- **Real-Time Updates**: Watch bid/ask prices update live
- **Connection Status**: Monitor WebSocket latency

## 🔧 Configuration

### Backend (.env)
```bash
RUST_LOG=info,crypto_dash=debug
BIND_ADDR=0.0.0.0:8080
EXCHANGES=binance,bybit
BOOK_DEPTH_DEFAULT=50
ENABLE_REDIS=false
```

### Frontend (.env.local)
```bash
NEXT_PUBLIC_API_URL=http://localhost:8080
```

## 📚 API Documentation

### REST Endpoints

- `GET /health` - Service health check
- `GET /ready` - Readiness probe  
- `GET /api/exchanges` - List supported exchanges

### WebSocket Protocol

**Connect**: `ws://localhost:8080/ws`

**Subscribe to channels**:
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

**Receive updates**:
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

## 🧪 Development

### Backend Development

```bash
cd crypto-dash-backend

# Fast compilation check
cargo check

# Run tests
cargo test

# Start with detailed logs
RUST_LOG=debug cargo run -p api

# Development script
./scripts/dev.sh
```

### Frontend Development

```bash
cd crypto-dash-frontend

# Start dev server with hot reload
npm run dev

# Type checking
npx tsc --noEmit

# Lint code
npm run lint

# Build for production
npm run build
```

## 🔮 Roadmap

### Phase 1 (MVP) ✅
- [x] Basic project structure
- [x] Rust backend with Axum
- [x] Exchange adapter framework
- [x] WebSocket streaming infrastructure
- [x] React frontend with real-time updates
- [x] Ticker data display

### Phase 2 (Enhanced)
- [ ] Live order book visualization
- [ ] WebSocket real exchange connections
- [ ] Symbol search and favorites
- [ ] Price charts with TradingView Lightweight Charts
- [ ] Compare mode for multiple symbols

### Phase 3 (Advanced)
- [ ] Historical data persistence
- [ ] Derived metrics (spread %, book imbalance)
- [ ] Alerts and notifications
- [ ] Advanced charting with indicators
- [ ] Portfolio tracking (view-only)

### Phase 4 (Scale)
- [ ] Redis pub/sub for horizontal scaling
- [ ] Kubernetes deployment configs
- [ ] Performance monitoring
- [ ] Load testing framework
- [ ] Additional exchanges (OKX, Coinbase, etc.)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) and [Tokio](https://tokio.rs/)
- UI powered by [Next.js](https://nextjs.org/) and [Tailwind CSS](https://tailwindcss.com/)
- Icons by [Lucide](https://lucide.dev/)
- Inspired by professional trading platforms

---

**Happy Trading!** 📈🚀
