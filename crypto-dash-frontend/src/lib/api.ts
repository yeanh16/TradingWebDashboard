// API client utilities

import type { CandlesResponse, MarketType } from './types'

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

export class ApiClient {
  private baseUrl: string

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl
  }

  private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`
    
    const response = await fetch(url, {
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      ...options,
    })

    if (!response.ok) {
      throw new Error(`API request failed: ${response.status} ${response.statusText}`)
    }

    return response.json()
  }

  async getExchanges() {
    return this.request('/api/exchanges')
  }

  async getSymbols(exchange?: string) {
    const url = exchange ? `/api/symbols?exchange=${exchange}` : '/api/symbols'
    return this.request(url)
  }

  async getCandles(params: { exchange: string; symbol: string; interval: string; limit?: number; market_type?: MarketType }): Promise<CandlesResponse> {
    const searchParams = new URLSearchParams({
      exchange: params.exchange,
      symbol: params.symbol,
      interval: params.interval,
    })

    if (typeof params.limit === 'number') {
      searchParams.set('limit', params.limit.toString())
    }

    if (params.market_type) {
      searchParams.set('market_type', params.market_type)
    }

    const query = searchParams.toString()
    return this.request(`/api/candles?${query}`)
  }

  async getHealth() {
    return this.request('/health')
  }

  async getReady() {
    return this.request('/ready')
  }

  getWebSocketUrl(): string {
    const wsProtocol = this.baseUrl.startsWith('https') ? 'wss' : 'ws'
    const baseWithoutProtocol = this.baseUrl.replace(/^https?:\/\//, '')
    return `${wsProtocol}://${baseWithoutProtocol}/ws`
  }
}

export const apiClient = new ApiClient()
