# Crypto Trading Dashboard - Frontend

React/Next.js frontend for the crypto trading dashboard with real-time market data visualization.

## Features

- **Real-time updates**: Live market data via WebSocket connection
- **Multi-exchange view**: Compare data across different exchanges
- **Responsive design**: Tailwind CSS with dark/light mode support
- **Interactive components**: Sortable tables, exchange selection, latency monitoring
- **Type-safe**: Full TypeScript implementation

## Tech Stack

- **Framework**: Next.js 14 with App Router
- **Styling**: Tailwind CSS + shadcn/ui components
- **State Management**: Zustand (planned)
- **Charts**: Recharts (planned)
- **Icons**: Lucide React

## Quick Start

1. **Prerequisites**
   ```bash
   # Install Node.js 18+ and npm
   node --version
   npm --version
   ```

2. **Setup Environment**
   ```bash
   cd crypto-dash-frontend
   cp .env.local.example .env.local
   # Edit .env.local as needed
   ```

3. **Install Dependencies**
   ```bash
   npm install
   ```

4. **Run Development Server**
   ```bash
   npm run dev
   ```

The app will be available at `http://localhost:3000`.

## Available Scripts

```bash
npm run dev      # Start development server
npm run build    # Build for production  
npm run start    # Start production server
npm run lint     # Run ESLint
```

## Project Structure

```
src/
├── app/           # Next.js App Router pages
├── components/    # Reusable UI components
├── lib/           # Utilities and API client
├── store/         # State management (planned)
├── hooks/         # Custom React hooks (planned)
└── styles/        # Global CSS and Tailwind config
```

## Key Components

- **TickerTable**: Live market data display with real-time updates
- **ExchangeSelector**: Multi-select exchange picker
- **LatencyBadge**: Connection status and latency monitoring
- **OrderBookView**: Depth chart visualization (planned)
- **CompareGrid**: Side-by-side symbol comparison (planned)

## Configuration

Environment variables (`.env.local`):

```
NEXT_PUBLIC_API_URL=http://localhost:8080
```

## API Integration

The frontend connects to the Rust backend API:

- **REST API**: Exchange info, market data snapshots
- **WebSocket**: Real-time streaming updates
- **Health checks**: Connection monitoring

## Development

```bash
# Start with backend running
npm run dev

# Build and test production build
npm run build
npm run start

# Type checking
npx tsc --noEmit

# Linting
npm run lint
```

## Deployment

```bash
# Build for production
npm run build

# The built files will be in .next/
# Deploy to Vercel, Netlify, or any Node.js hosting
```