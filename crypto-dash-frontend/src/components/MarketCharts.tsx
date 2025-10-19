'use client'

import { useMemo, useState } from 'react'
import { CandlestickChart } from '@/components/CandlestickChart'
import type { MarketType, SelectedTicker, Ticker } from '@/lib/types'

interface MarketChartsProps {
  selectedTickers: SelectedTicker[]
  tickers: Record<string, Ticker>
  selectedExchanges: string[]
  marketType: MarketType
  quoteSymbol: string
}

const INTERVAL_OPTIONS = ['1m', '5m', '15m', '1h', '4h', '1d']

const buildTickerKey = (ticker: SelectedTicker) => `${ticker.exchange}_${ticker.market_type}_${ticker.base}${ticker.quote}`

export function MarketCharts({ selectedTickers, tickers, selectedExchanges, marketType, quoteSymbol }: MarketChartsProps) {
  const [interval, setInterval] = useState<string>('1m')

  const visibleTickers = useMemo(() => {
    return selectedTickers.filter((ticker) =>
      selectedExchanges.includes(ticker.exchange) &&
      ticker.market_type === marketType &&
      ticker.quote === quoteSymbol
    )
  }, [selectedTickers, selectedExchanges, marketType, quoteSymbol])

  if (visibleTickers.length === 0) {
    return (
      <div className="rounded-lg border bg-card p-6 text-center text-sm text-muted-foreground">
        Select a symbol to view its chart
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-base font-semibold">Price Charts</h3>
        <div className="flex items-center gap-2 text-xs">
          <span className="text-muted-foreground">Interval</span>
          <div className="inline-flex rounded-md border border-border bg-background p-1">
            {INTERVAL_OPTIONS.map((option) => (
              <button
                key={option}
                onClick={() => setInterval(option)}
                className={`px-2 py-1 rounded-sm font-medium transition-colors ${interval === option ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {option.toUpperCase()}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        {visibleTickers.map((ticker) => {
          const liveTicker = tickers[buildTickerKey(ticker)]
          return (
            <CandlestickChart
              key={buildTickerKey(ticker)}
              ticker={ticker}
              interval={interval}
              liveTicker={liveTicker}
            />
          )
        })}
      </div>
    </div>
  )
}
