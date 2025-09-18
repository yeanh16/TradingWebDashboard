'use client'

import { useEffect, useState } from 'react'
import { TickerTable } from '@/components/TickerTable'
import { ExchangeSelector } from '@/components/ExchangeSelector'
import { TickerSelector } from '@/components/TickerSelector'
import { LatencyBadge } from '@/components/LatencyBadge'
import { useWebSocket } from '@/lib/useWebSocket'
import { apiClient } from '@/lib/api'
import { Channel, SelectedTicker, SymbolResponse, SymbolInfo, MarketType } from '@/lib/types'

interface Exchange {
  id: string
  name: string
  status: 'online' | 'offline' | 'maintenance'
}

const MOCK_EXCHANGES: Exchange[] = [
  { id: 'binance', name: 'Binance', status: 'online' },
  { id: 'bybit', name: 'Bybit', status: 'online' },
]

const MARKET_TYPES: MarketType[] = ['spot', 'perpetual']

const DEFAULT_TICKERS: SelectedTicker[] = [
  { symbol: 'BTC-USDT', base: 'BTC', quote: 'USDT', exchange: 'binance', market_type: 'spot', display_name: 'Bitcoin / USDT' },
  { symbol: 'ETH-USDT', base: 'ETH', quote: 'USDT', exchange: 'binance', market_type: 'spot', display_name: 'Ethereum / USDT' },
]

export default function HomePage() {
  const [selectedExchanges, setSelectedExchanges] = useState<string[]>(['binance', 'bybit'])
  const [selectedTickers, setSelectedTickers] = useState<SelectedTicker[]>(DEFAULT_TICKERS)
  const [selectedMarketType, setSelectedMarketType] = useState<MarketType>('spot')
  const [symbolMetadata, setSymbolMetadata] = useState<Record<string, Record<string, SymbolInfo>>>(() => ({}))
  const [exchanges, setExchanges] = useState<Exchange[]>([])
  const [loading, setLoading] = useState(true)
  
  const { state: wsState, tickers, subscribe, unsubscribe, clearError } = useWebSocket()

  useEffect(() => {
    if (selectedExchanges.length === 0) {
      setSymbolMetadata({})
      return
    }

    const loadMetadata = async () => {
      try {
        const response = await apiClient.getSymbols() as SymbolResponse[]
        const metadataMap: Record<string, Record<string, SymbolInfo>> = {}

        response.forEach((exchangeData) => {
          if (!selectedExchanges.includes(exchangeData.exchange)) {
            return
          }

          const symbolsMap: Record<string, SymbolInfo> = {}
          exchangeData.symbols.forEach((symbol) => {
            symbolsMap[symbol.symbol] = symbol
          })

          metadataMap[exchangeData.exchange] = symbolsMap
        })

        setSymbolMetadata(metadataMap)
      } catch (error) {
        console.error('Failed to load symbol metadata:', error)
      }
    }

    loadMetadata()
  }, [selectedExchanges])

  useEffect(() => {
    // Load exchanges from API
    const loadExchanges = async () => {
      try {
        const response = await apiClient.getExchanges() as { exchanges?: Exchange[] }
        setExchanges(response.exchanges || MOCK_EXCHANGES)
      } catch (error) {
        console.error('Failed to load exchanges:', error)
        setExchanges(MOCK_EXCHANGES) // Fallback to mock data
      } finally {
        setLoading(false)
      }
    }

    loadExchanges()
  }, [])

  useEffect(() => {
    if (Object.keys(symbolMetadata).length === 0) {
      return
    }

    setSelectedTickers((prev) => {
      let updated = false
      const nextTickers: SelectedTicker[] = []

      prev.forEach((ticker) => {
        const meta = symbolMetadata[ticker.exchange]?.[`${ticker.symbol}::${ticker.market_type}`]
        if (!meta) {
          nextTickers.push(ticker)
          return
        }

        const nextTicker: SelectedTicker = { ...ticker }
        let changed = false

        if (meta.price_precision !== undefined && meta.price_precision !== ticker.price_precision) {
          nextTicker.price_precision = meta.price_precision
          changed = true
        }

        if (meta.tick_size !== undefined && meta.tick_size !== ticker.tick_size) {
          nextTicker.tick_size = meta.tick_size
          changed = true
        }

        if (meta.market_type && meta.market_type !== ticker.market_type) {
          nextTicker.market_type = meta.market_type
          changed = true
        }

        if (changed) {
          updated = true
        }

        nextTickers.push(nextTicker)
      })

      return updated ? nextTickers : prev
    })
  }, [symbolMetadata])

  // Subscribe to ticker data when exchanges or tickers are selected
  useEffect(() => {
    if (!wsState.connected || selectedTickers.length === 0) {
      return
    }

    // Create channels from selected tickers that match selected exchanges
    const channels: Channel[] = selectedTickers
      .filter(ticker => selectedExchanges.includes(ticker.exchange) && ticker.market_type === selectedMarketType)
      .map(ticker => ({
        channel_type: 'ticker' as const,
        exchange: ticker.exchange,
        market_type: ticker.market_type,
        symbol: {
          base: ticker.base,
          quote: ticker.quote,
        },
      }))

    if (channels.length > 0) {
      console.log('Subscribing to channels:', channels)
      subscribe(channels)

      return () => {
        console.log('Unsubscribing from channels:', channels)
        unsubscribe(channels)
      }
    }
  }, [selectedExchanges, selectedTickers, selectedMarketType, wsState.connected, subscribe, unsubscribe])

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-64">
        <div className="text-lg text-muted-foreground">Loading exchanges...</div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h2 className="text-3xl font-bold tracking-tight">Markets Overview</h2>
          <p className="text-muted-foreground">
            Real-time cryptocurrency market data from multiple exchanges
          </p>
        </div>
        <LatencyBadge wsState={wsState} onClearError={clearError} />
      </div>

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-muted-foreground">Market Type</span>
          <div className="inline-flex rounded-md border border-border bg-background p-1">
            {MARKET_TYPES.map((type) => (
              <button
                key={type}
                onClick={() => setSelectedMarketType(type)}
                className={`px-3 py-1 text-sm font-medium rounded-md transition-colors ${selectedMarketType === type ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {type === 'spot' ? 'Spot' : 'Perpetual'}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-12">
        <div className="lg:col-span-3 space-y-6">
          <ExchangeSelector
            exchanges={exchanges}
            selectedExchanges={selectedExchanges}
            onSelectionChange={setSelectedExchanges}
          />
          <TickerSelector
            selectedExchanges={selectedExchanges}
            selectedTickers={selectedTickers}
            onTickersChange={setSelectedTickers}
            activeMarketType={selectedMarketType}
          />
        </div>
        <div className="lg:col-span-9">
          <TickerTable 
            selectedExchanges={selectedExchanges} 
            selectedTickers={selectedTickers}
            tickers={tickers}
            wsConnected={wsState.connected}
            activeMarketType={selectedMarketType}
          />
        </div>
      </div>
    </div>
  )
}
