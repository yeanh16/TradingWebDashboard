'use client'

import { useEffect, useState } from 'react'
import { TickerTable } from '@/components/TickerTable'
import { ExchangeSelector } from '@/components/ExchangeSelector'
import { TickerSelector } from '@/components/TickerSelector'
import { LatencyBadge } from '@/components/LatencyBadge'
import { useWebSocket } from '@/lib/useWebSocket'
import { apiClient } from '@/lib/api'
import { Channel, SelectedTicker, SymbolResponse, SymbolInfo } from '@/lib/types'

interface Exchange {
  id: string
  name: string
  status: 'online' | 'offline' | 'maintenance'
}

const MOCK_EXCHANGES: Exchange[] = [
  { id: 'binance', name: 'Binance', status: 'online' },
  { id: 'bybit', name: 'Bybit', status: 'online' },
]

const DEFAULT_TICKERS: SelectedTicker[] = [
  { symbol: 'BTC-USDT', base: 'BTC', quote: 'USDT', exchange: 'binance', display_name: 'Bitcoin / USDT' },
  { symbol: 'ETH-USDT', base: 'ETH', quote: 'USDT', exchange: 'binance', display_name: 'Ethereum / USDT' },
]

export default function HomePage() {
  const [selectedExchanges, setSelectedExchanges] = useState<string[]>(['binance', 'bybit'])
  const [selectedTickers, setSelectedTickers] = useState<SelectedTicker[]>(DEFAULT_TICKERS)
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
        const meta = symbolMetadata[ticker.exchange]?.[ticker.symbol]
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
      .filter(ticker => selectedExchanges.includes(ticker.exchange))
      .map(ticker => ({
        channel_type: 'ticker' as const,
        exchange: ticker.exchange,
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
  }, [selectedExchanges, selectedTickers, wsState.connected])

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
          />
        </div>
        <div className="lg:col-span-9">
          <TickerTable 
            selectedExchanges={selectedExchanges} 
            selectedTickers={selectedTickers}
            tickers={tickers}
            wsConnected={wsState.connected}
          />
        </div>
      </div>
    </div>
  )
}
