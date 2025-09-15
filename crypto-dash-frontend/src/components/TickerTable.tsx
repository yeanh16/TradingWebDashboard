'use client'

import { useState, useEffect, useMemo } from 'react'
import { TrendingUp, TrendingDown, Wifi, WifiOff } from 'lucide-react'
import { Ticker, SelectedTicker } from '@/lib/types'

interface TickerData {
  symbol: string
  exchange: string
  bid: number
  ask: number
  last: number
  change24h: number
  volume24h: number
  spread: number
  lastUpdate?: Date
}

interface TickerTableProps {
  selectedExchanges: string[]
  selectedTickers: SelectedTicker[]
  tickers: Record<string, Ticker>
  wsConnected: boolean
}

// Mock data for demonstration when no live data is available - updated with current realistic prices
const MOCK_TICKERS: TickerData[] = [
  {
    symbol: 'BTC-USDT',
    exchange: 'binance',
    bid: 110250.50,
    ask: 110251.75,
    last: 110251.00,
    change24h: 2.45,
    volume24h: 125430000,
    spread: 0.003,
  },
  {
    symbol: 'ETH-USDT', 
    exchange: 'binance',
    bid: 4150.25,
    ask: 4150.95,
    last: 4150.60,
    change24h: -1.23,
    volume24h: 85670000,
    spread: 0.026,
  },
]

export function TickerTable({ selectedExchanges, selectedTickers, tickers, wsConnected }: TickerTableProps) {
  const [loading, setLoading] = useState(true)
  const [priceChanges, setPriceChanges] = useState<Record<string, 'up' | 'down' | null>>({})

  // Convert live ticker data to display format
  const displayTickers = useMemo(() => {
    const result: TickerData[] = []
    
    // Map through selected tickers and try to find live data for them
    selectedTickers.forEach(selectedTicker => {
      // Only show tickers for selected exchanges
      if (!selectedExchanges.includes(selectedTicker.exchange)) {
        return
      }

      // Try to find live ticker data
      const tickerKey = Object.keys(tickers).find(key => {
        const ticker = tickers[key]
        return ticker.exchange === selectedTicker.exchange &&
               ticker.symbol.base === selectedTicker.base &&
               ticker.symbol.quote === selectedTicker.quote
      })

      if (tickerKey) {
        const ticker = tickers[tickerKey]
        const spread = ticker.ask > 0 && ticker.bid > 0 
          ? (ticker.ask - ticker.bid) / ticker.ask * 100 
          : 0
          
        result.push({
          symbol: `${ticker.symbol.base}-${ticker.symbol.quote}`,
          exchange: ticker.exchange,
          bid: ticker.bid,
          ask: ticker.ask,
          last: ticker.last,
          change24h: 0, // We don't have 24h change in the current data structure
          volume24h: 0, // We don't have volume in the current data structure
          spread,
          lastUpdate: new Date(ticker.timestamp),
        })
      } else if (!wsConnected) {
        // Show mock data for selected ticker when not connected
        const mockTicker = MOCK_TICKERS.find(mock => 
          mock.symbol === selectedTicker.symbol && mock.exchange === selectedTicker.exchange
        )
        if (mockTicker) {
          result.push(mockTicker)
        } else {
          // Create a placeholder mock ticker
          result.push({
            symbol: selectedTicker.symbol,
            exchange: selectedTicker.exchange,
            bid: 0,
            ask: 0,
            last: 0,
            change24h: 0,
            volume24h: 0,
            spread: 0,
          })
        }
      }
    })
    
    return result
  }, [tickers, selectedExchanges, selectedTickers, wsConnected])

  // Track price changes for visual feedback
  useEffect(() => {
    const newChanges: Record<string, 'up' | 'down' | null> = {}
    
    Object.entries(tickers).forEach(([key, ticker]) => {
      const currentPrice = ticker.last
      // In a real app, you'd compare with previous price
      // For now, we'll randomly simulate price movements for demo
      newChanges[key] = Math.random() > 0.5 ? 'up' : 'down'
    })
    
    setPriceChanges(newChanges)
    
    // Clear price change indicators after 2 seconds
    const timeout = setTimeout(() => {
      setPriceChanges({})
    }, 2000)
    
    return () => clearTimeout(timeout)
  }, [tickers])

  useEffect(() => {
    // Simulate initial loading
    const timer = setTimeout(() => setLoading(false), 300)
    return () => clearTimeout(timer)
  }, [])

  const formatPrice = (price: number, decimals?: number) => {
    const precision = decimals ?? 2;
    return price.toLocaleString('en-US', {
      minimumFractionDigits: precision,
      maximumFractionDigits: precision,
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

  const getPriceClassName = (tickerKey: string) => {
    const change = priceChanges[tickerKey]
    if (change === 'up') return 'bg-green-100 dark:bg-green-900/30'
    if (change === 'down') return 'bg-red-100 dark:bg-red-900/30'
    return ''
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

  return (
    <div className="rounded-lg border bg-card">
      <div className="p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold">Live Market Data</h3>
          <div className="flex items-center space-x-2 text-sm">
            {wsConnected ? (
              <>
                <Wifi className="w-4 h-4 text-green-600" />
                <span className="text-green-600">Live</span>
              </>
            ) : (
              <>
                <WifiOff className="w-4 h-4 text-orange-600" />
                <span className="text-orange-600">Demo Mode</span>
              </>
            )}
          </div>
        </div>
        
        {displayTickers.length === 0 ? (
          <div className="text-center text-muted-foreground py-8">
            No tickers available for selected exchanges
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 text-sm font-medium text-muted-foreground">Symbol</th>
                  <th className="text-left py-2 text-sm font-medium text-muted-foreground">Exchange</th>
                  <th className="text-right py-2 text-sm font-medium text-muted-foreground">Last Price</th>
                  <th className="text-right py-2 text-sm font-medium text-muted-foreground">Bid</th>
                  <th className="text-right py-2 text-sm font-medium text-muted-foreground">Ask</th>
                  <th className="text-right py-2 text-sm font-medium text-muted-foreground">Spread</th>
                  <th className="text-right py-2 text-sm font-medium text-muted-foreground">Last Update</th>
                </tr>
              </thead>
              <tbody>
                {displayTickers.map((ticker) => {
                  const tickerKey = `${ticker.exchange}_${ticker.symbol.replace('-', '')}`
                  
                  // Find the selected ticker that matches this display ticker to get metadata
                  const selectedTicker = selectedTickers.find(st => 
                    st.symbol === ticker.symbol && st.exchange === ticker.exchange
                  )
                  
                  // Use price precision from metadata or default to 2
                  const pricePrecision = selectedTicker?.price_precision ?? 2
                  
                  return (
                    <tr 
                      key={tickerKey} 
                      className={`border-b border-border hover:bg-accent/50 transition-all duration-200 ${getPriceClassName(tickerKey)}`}
                    >
                      <td className="py-3">
                        <div className="font-medium">{ticker.symbol}</div>
                      </td>
                      <td className="py-3">
                        <div className="text-sm text-muted-foreground capitalize">
                          {ticker.exchange}
                        </div>
                      </td>
                      <td className="py-3 text-right font-mono">
                        ${formatPrice(ticker.last, pricePrecision)}
                      </td>
                      <td className="py-3 text-right font-mono text-green-600">
                        ${formatPrice(ticker.bid, pricePrecision)}
                      </td>
                      <td className="py-3 text-right font-mono text-red-600">
                        ${formatPrice(ticker.ask, pricePrecision)}
                      </td>
                      <td className="py-3 text-right font-mono text-sm text-muted-foreground">
                        {ticker.spread.toFixed(3)}%
                      </td>
                      <td className="py-3 text-right text-xs text-muted-foreground">
                        {ticker.lastUpdate 
                          ? ticker.lastUpdate.toLocaleTimeString()
                          : wsConnected ? 'Live' : 'Mock'
                        }
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}