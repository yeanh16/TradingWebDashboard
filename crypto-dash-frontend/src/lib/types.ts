export type MarketType = 'spot' | 'perpetual';

// Shared TypeScript types for the frontend

export interface ExchangeInfo {
  id: string
  name: string
  status: 'online' | 'offline' | 'maintenance'
  rate_limits?: Record<string, number>
  ws_url?: string
  rest_url?: string
}

export interface Symbol {
  base: string
  quote: string
}

export interface Ticker {
  timestamp: string
  exchange: string
  market_type: MarketType
  symbol: Symbol
  bid: number
  ask: number
  last: number
  bid_size: number
  ask_size: number
}

export interface PriceLevel {
  price: number
  quantity: number
}

export interface OrderBookSnapshot {
  timestamp: string
  exchange: string
  symbol: Symbol
  bids: PriceLevel[]
  asks: PriceLevel[]
  checksum?: string
}

export interface StreamMessage {
  type: 'ticker' | 'orderbook_snapshot' | 'orderbook_delta' | 'info' | 'error'
  payload?: any
  message?: string
}

export interface ClientMessage {
  op: 'subscribe' | 'unsubscribe' | 'ping'
  channels?: Channel[]
}

export interface Channel {
  channel_type: 'ticker' | 'orderbook'
  exchange: string
  market_type: MarketType
  symbol: Symbol
  depth?: number
}

export interface SymbolInfo {
  symbol: string
  base: string
  quote: string
  market_type: MarketType
  display_name: string
  price_precision?: number  // Optional for backwards compatibility
  tick_size?: string       // Optional for backwards compatibility
  min_qty?: number         // Optional for backwards compatibility
  step_size?: number       // Optional for backwards compatibility
}

export interface SymbolResponse {
  exchange: string
  symbols: SymbolInfo[]
}

export interface AllowedQuotes {
  spot: string[]
  perpetual: string[]
}

export interface SymbolsPayload {
  allowed_quotes: AllowedQuotes
  exchanges: SymbolResponse[]
}

export interface SelectedTicker {
  symbol: string
  base: string
  quote: string
  exchange: string
  market_type: MarketType
  display_name: string
  price_precision?: number
  tick_size?: string
  min_qty?: number
  step_size?: number
}