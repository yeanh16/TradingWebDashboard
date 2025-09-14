'use client'

import { useEffect, useState } from 'react'
import { TickerTable } from '@/components/TickerTable'
import { ExchangeSelector } from '@/components/ExchangeSelector'
import { LatencyBadge } from '@/components/LatencyBadge'

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

  useEffect(() => {
    // Simulate loading exchanges from API
    const loadExchanges = async () => {
      try {
        // In a real app, this would be: await fetch('/api/exchanges')
        await new Promise(resolve => setTimeout(resolve, 500)) // Simulate API call
        setExchanges(MOCK_EXCHANGES)
      } catch (error) {
        console.error('Failed to load exchanges:', error)
        setExchanges(MOCK_EXCHANGES) // Fallback to mock data
      } finally {
        setLoading(false)
      }
    }

    loadExchanges()
  }, [])

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
        <LatencyBadge />
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
          <TickerTable selectedExchanges={selectedExchanges} />
        </div>
      </div>
    </div>
  )
}