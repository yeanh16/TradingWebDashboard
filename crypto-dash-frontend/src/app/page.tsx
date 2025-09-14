'use client'

import { useEffect, useState } from 'react'
import { TickerTable } from '@/components/TickerTable'
import { ExchangeSelector } from '@/components/ExchangeSelector'
import { LatencyBadge } from '@/components/LatencyBadge'
import { useWebSocket } from '@/lib/useWebSocket'
import { apiClient } from '@/lib/api'
import { Channel } from '@/lib/types'

interface Exchange {
  id: string
  name: string
  status: 'online' | 'offline' | 'maintenance'
}

const MOCK_EXCHANGES: Exchange[] = [
  { id: 'binance', name: 'Binance', status: 'online' },
  { id: 'bybit', name: 'Bybit', status: 'online' },
]

export default function HomePage() {
  const [selectedExchanges, setSelectedExchanges] = useState<string[]>(['binance'])
  const [exchanges, setExchanges] = useState<Exchange[]>([])
  const [loading, setLoading] = useState(true)
  
  const { state: wsState, tickers, subscribe, unsubscribe, clearError } = useWebSocket()

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

  // Subscribe to ticker data when exchanges are selected
  useEffect(() => {
    if (!wsState.connected || selectedExchanges.length === 0) {
      return
    }

    // Define popular trading pairs to subscribe to
    const symbols = [
      { base: 'BTC', quote: 'USDT' },
      { base: 'ETH', quote: 'USDT' },
      { base: 'ADA', quote: 'USDT' },
      { base: 'SOL', quote: 'USDT' },
    ]

    const channels: Channel[] = selectedExchanges.flatMap(exchange =>
      symbols.map(symbol => ({
        channel_type: 'ticker' as const,
        exchange,
        symbol,
      }))
    )

    console.log('Subscribing to channels:', channels)
    subscribe(channels)

    return () => {
      console.log('Unsubscribing from channels:', channels)
      unsubscribe(channels)
    }
  }, [selectedExchanges, wsState.connected, subscribe, unsubscribe])

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

      <div className="grid gap-6 md:grid-cols-4">
        <div className="md:col-span-1">
          <ExchangeSelector
            exchanges={exchanges}
            selectedExchanges={selectedExchanges}
            onSelectionChange={setSelectedExchanges}
          />
        </div>
        <div className="md:col-span-3">
          <TickerTable 
            selectedExchanges={selectedExchanges} 
            tickers={tickers}
            wsConnected={wsState.connected}
          />
        </div>
      </div>
    </div>
  )
}