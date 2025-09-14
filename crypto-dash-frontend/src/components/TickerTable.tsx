'use client'

import { useState, useEffect } from 'react'
import { TrendingUp, TrendingDown } from 'lucide-react'

interface TickerData {
  symbol: string
  exchange: string
  bid: number
  ask: number
  last: number
  change24h: number
  volume24h: number
  spread: number
}

interface TickerTableProps {
  selectedExchanges: string[]
}

// Mock data for demonstration
const MOCK_TICKERS: TickerData[] = [
  {
    symbol: 'BTC-USDT',
    exchange: 'binance',
    bid: 43250.50,
    ask: 43251.75,
    last: 43251.00,
    change24h: 2.45,
    volume24h: 125430000,
    spread: 0.003,
  },
  {
    symbol: 'ETH-USDT', 
    exchange: 'binance',
    bid: 2650.25,
    ask: 2650.95,
    last: 2650.60,
    change24h: -1.23,
    volume24h: 85670000,
    spread: 0.026,
  },
  {
    symbol: 'ADA-USDT',
    exchange: 'binance', 
    bid: 0.3845,
    ask: 0.3847,
    last: 0.3846,
    change24h: 5.67,
    volume24h: 45230000,
    spread: 0.052,
  },
  {
    symbol: 'BTC-USDT',
    exchange: 'bybit',
    bid: 43249.75,
    ask: 43252.25,
    last: 43250.50,
    change24h: 2.41,
    volume24h: 98760000,
    spread: 0.006,
  },
]

export function TickerTable({ selectedExchanges }: TickerTableProps) {
  const [tickers, setTickers] = useState<TickerData[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    // Simulate loading ticker data
    const loadTickers = async () => {
      setLoading(true)
      await new Promise(resolve => setTimeout(resolve, 300))
      
      // Filter tickers based on selected exchanges
      const filteredTickers = MOCK_TICKERS.filter(ticker =>
        selectedExchanges.includes(ticker.exchange)
      )
      
      setTickers(filteredTickers)
      setLoading(false)
    }

    loadTickers()
  }, [selectedExchanges])

  const formatPrice = (price: number, decimals = 2) => {
    return price.toLocaleString('en-US', {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    })
  }

  const formatVolume = (volume: number) => {
    if (volume >= 1000000) {
      return `${(volume / 1000000).toFixed(1)}M`
    }
    if (volume >= 1000) {
      return `${(volume / 1000).toFixed(1)}K`
    }
    return volume.toString()
  }

  if (loading) {
    return (
      <div className="rounded-lg border bg-card p-6">
        <div className="space-y-4">
          <div className="h-6 bg-muted rounded animate-pulse" />
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-12 bg-muted rounded animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  if (tickers.length === 0) {
    return (
      <div className="rounded-lg border bg-card p-6">
        <div className="text-center text-muted-foreground">
          No tickers available for selected exchanges
        </div>
      </div>
    )
  }

  return (
    <div className="rounded-lg border bg-card">
      <div className="p-6">
        <h3 className="text-lg font-semibold mb-4">Live Market Data</h3>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left py-2 text-sm font-medium text-muted-foreground">Symbol</th>
                <th className="text-left py-2 text-sm font-medium text-muted-foreground">Exchange</th>
                <th className="text-right py-2 text-sm font-medium text-muted-foreground">Last Price</th>
                <th className="text-right py-2 text-sm font-medium text-muted-foreground">24h Change</th>
                <th className="text-right py-2 text-sm font-medium text-muted-foreground">Bid</th>
                <th className="text-right py-2 text-sm font-medium text-muted-foreground">Ask</th>
                <th className="text-right py-2 text-sm font-medium text-muted-foreground">Spread</th>
                <th className="text-right py-2 text-sm font-medium text-muted-foreground">Volume (24h)</th>
              </tr>
            </thead>
            <tbody>
              {tickers.map((ticker, index) => (
                <tr key={`${ticker.exchange}-${ticker.symbol}`} className="border-b border-border hover:bg-accent/50 transition-colors">
                  <td className="py-3">
                    <div className="font-medium">{ticker.symbol}</div>
                  </td>
                  <td className="py-3">
                    <div className="text-sm text-muted-foreground capitalize">
                      {ticker.exchange}
                    </div>
                  </td>
                  <td className="py-3 text-right font-mono">
                    ${formatPrice(ticker.last)}
                  </td>
                  <td className="py-3 text-right">
                    <div className={`flex items-center justify-end space-x-1 ${
                      ticker.change24h >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}>
                      {ticker.change24h >= 0 ? (
                        <TrendingUp className="w-4 h-4" />
                      ) : (
                        <TrendingDown className="w-4 h-4" />
                      )}
                      <span className="font-mono">
                        {ticker.change24h >= 0 ? '+' : ''}{ticker.change24h.toFixed(2)}%
                      </span>
                    </div>
                  </td>
                  <td className="py-3 text-right font-mono text-green-600">
                    ${formatPrice(ticker.bid)}
                  </td>
                  <td className="py-3 text-right font-mono text-red-600">
                    ${formatPrice(ticker.ask)}
                  </td>
                  <td className="py-3 text-right font-mono text-sm text-muted-foreground">
                    {ticker.spread.toFixed(3)}%
                  </td>
                  <td className="py-3 text-right font-mono text-sm">
                    ${formatVolume(ticker.volume24h)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}