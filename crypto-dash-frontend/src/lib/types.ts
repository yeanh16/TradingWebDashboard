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
  symbol: Symbol
  depth?: number
}

export interface SymbolInfo {
  symbol: string
  base: string
  quote: string
  display_name: string
}

export interface SymbolResponse {
  exchange: string
  symbols: SymbolInfo[]
}