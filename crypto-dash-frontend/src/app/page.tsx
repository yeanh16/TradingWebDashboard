'use client'

import { useCallback, useEffect, useState } from 'react'
import { TickerTable } from '@/components/TickerTable'
import { ExchangeSelector } from '@/components/ExchangeSelector'
import { TickerSelector } from '@/components/TickerSelector'
import { MarketCharts } from '@/components/MarketCharts'
import { LatencyBadge } from '@/components/LatencyBadge'
import { useWebSocket } from '@/lib/useWebSocket'
import { apiClient, aiClient } from '@/lib/api'
import { Channel, SelectedTicker, SymbolInfo, MarketType, SymbolsPayload, AllowedQuotes } from '@/lib/types'

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
type QuoteSymbol = string
const DEFAULT_BASE_ASSET = 'BTC'

export default function HomePage() {
  const [selectedExchanges, setSelectedExchanges] = useState<string[]>(['binance', 'bybit'])
  const [selectedTickers, setSelectedTickers] = useState<SelectedTicker[]>([])
  const [selectedMarketType, setSelectedMarketType] = useState<MarketType>('spot')
  const [selectedQuoteSymbol, setSelectedQuoteSymbol] = useState<QuoteSymbol>('USDT')
  const [allowedQuotes, setAllowedQuotes] = useState<AllowedQuotes>({ spot: [], perpetual: [] })
  const [hasInitializedDefaults, setHasInitializedDefaults] = useState(false)
  const [aiSummary, setAiSummary] = useState('Click summarise to generate insights about your selected markets.')
  const [chartInterval, setChartInterval] = useState<string>('1m')
  const [symbolMetadata, setSymbolMetadata] = useState<Record<string, Record<string, SymbolInfo>>>(() => ({}))
  const [exchanges, setExchanges] = useState<Exchange[]>([])
  const [loading, setLoading] = useState(true)
  
  const handleSummarize = useCallback(async () => {
    const focusedSelections = selectedTickers
      .filter((ticker) =>
        selectedExchanges.includes(ticker.exchange) &&
        ticker.market_type === selectedMarketType &&
        ticker.quote === selectedQuoteSymbol
      )

    if (focusedSelections.length === 0) {
      setAiSummary('AI preview: select a market to generate a summary.')
      return
    }

    const groupedSymbols = focusedSelections.reduce<Record<string, string>>((acc, ticker) => {
      const key = ticker.base.toUpperCase()
      if (!acc[key]) {
        acc[key] = `${ticker.exchange}:${ticker.base}${ticker.quote}`
      }
      return acc
    }, {})

    const displayList = Object.keys(groupedSymbols)
    const requestSymbols = Object.values(groupedSymbols)

    setAiSummary('Generating AI insight...')

    try {
      const response = await aiClient.getInsights({
        symbols: requestSymbols,
        interval: chartInterval,
      })
      const summary = response.overview || `AI preview: monitoring ${displayList.join(', ')} (${selectedMarketType.toUpperCase()} / ${selectedQuoteSymbol}). Detailed insights coming soon.`
      setAiSummary(summary)
    } catch (error) {
      console.error('Failed to fetch AI insight', error)
      setAiSummary('AI service unavailable. Please try again later.')
    }
  }, [selectedTickers, selectedExchanges, selectedMarketType, selectedQuoteSymbol, chartInterval])
  
  const { state: wsState, tickers, subscribe, unsubscribe, clearError } = useWebSocket()

  useEffect(() => {
    const allowed = allowedQuotes[selectedMarketType] ?? []
    setSelectedQuoteSymbol((current) => {
      if (allowed.length === 0) {
        return current
      }

      return allowed.includes(current) ? current : allowed[0]
    })
  }, [allowedQuotes, selectedMarketType])

  const handleMarketTypeChange = (type: MarketType) => {
    setSelectedMarketType(type);
    setSelectedQuoteSymbol((current) => {
      const allowed = allowedQuotes[type] ?? [];
      if (allowed.length === 0) {
        return current;
      }
      return allowed.includes(current) ? current : allowed[0];
    });
  };

  useEffect(() => {
    if (selectedExchanges.length === 0) {
      setSymbolMetadata({})
      return
    }

    const loadMetadata = async () => {
      try {
        const response = await apiClient.getSymbols() as SymbolsPayload
        setAllowedQuotes(response.allowed_quotes)

        const metadataMap: Record<string, Record<string, SymbolInfo>> = {}

        response.exchanges.forEach((exchangeData) => {
          if (!selectedExchanges.includes(exchangeData.exchange)) {
            return
          }

          const symbolsMap: Record<string, SymbolInfo> = {}
          exchangeData.symbols.forEach((symbol) => {
            const key = `${symbol.symbol}::${symbol.market_type}`
            symbolsMap[key] = symbol
          })

          metadataMap[exchangeData.exchange] = symbolsMap
        })

        setSymbolMetadata(metadataMap)

        if (!hasInitializedDefaults) {
          const defaultEntries: SelectedTicker[] = []
          const allowedByMarket = response.allowed_quotes

          response.exchanges.forEach((exchangeData) => {
            if (!selectedExchanges.includes(exchangeData.exchange)) {
              return
            }

            exchangeData.symbols.forEach((symbol) => {
              if (symbol.base.toUpperCase() !== DEFAULT_BASE_ASSET) {
                return
              }

              const allowedForMarket = (allowedByMarket[symbol.market_type] ?? []).map((value) => value.toUpperCase())
              if (!allowedForMarket.includes(symbol.quote.toUpperCase())) {
                return
              }

              defaultEntries.push({
                symbol: symbol.symbol,
                base: symbol.base,
                quote: symbol.quote,
                exchange: exchangeData.exchange,
                market_type: symbol.market_type,
                display_name: symbol.display_name,
                price_precision: symbol.price_precision,
                tick_size: symbol.tick_size,
                min_qty: symbol.min_qty,
                step_size: symbol.step_size,
              })
            })
          })

          if (defaultEntries.length > 0) {
            defaultEntries.sort((a, b) => {
              if (a.exchange !== b.exchange) {
                return a.exchange.localeCompare(b.exchange)
              }
              if (a.market_type !== b.market_type) {
                return a.market_type.localeCompare(b.market_type)
              }
              return a.quote.localeCompare(b.quote)
            })

            setSelectedTickers((prev) => {
              if (prev.length > 0) {
                return prev
              }

              return defaultEntries
            })

            setHasInitializedDefaults(true)
          }
        }
      } catch (error) {
        console.error('Failed to load symbol metadata:', error)
      }
    }

    loadMetadata()
  }, [selectedExchanges, hasInitializedDefaults])

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
      .filter(ticker =>
        selectedExchanges.includes(ticker.exchange) &&
        ticker.market_type === selectedMarketType &&
        ticker.quote === selectedQuoteSymbol
      )
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
  }, [selectedExchanges, selectedTickers, selectedMarketType, selectedQuoteSymbol, wsState.connected, subscribe, unsubscribe])

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

      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-muted-foreground">Market Type</span>
          <div className="inline-flex rounded-md border border-border bg-background p-1">
            {MARKET_TYPES.map((type) => (
              <button
                key={type}
                onClick={() => handleMarketTypeChange(type)}
                className={`px-3 py-1 text-sm font-medium rounded-md transition-colors ${selectedMarketType === type ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {type === 'spot' ? 'Spot' : 'Perpetual'}
              </button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-muted-foreground">Quote</span>
          <div className="inline-flex flex-wrap gap-2 rounded-md border border-border bg-background p-1">
            {(allowedQuotes[selectedMarketType] ?? []).map((quote) => (
              <button
                key={quote}
                onClick={() => setSelectedQuoteSymbol(quote)}
                className={`px-3 py-1 text-sm font-medium rounded-md transition-colors ${selectedQuoteSymbol === quote ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {quote}
              </button>
            ))}
          </div>
        </div>
      </div>

      <section className="rounded-lg border bg-card p-6 space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-base font-semibold">AI Insight</h3>
          <button
            onClick={() => handleSummarize()}
            className="inline-flex items-center justify-center rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground shadow transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          >
            Summarise
          </button>
        </div>
        <p className="text-sm text-muted-foreground whitespace-pre-line">{aiSummary}</p>
      </section>

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
            activeQuoteSymbol={selectedQuoteSymbol}
            allowedQuotes={allowedQuotes}
          />
        </div>
        <div className="lg:col-span-9 space-y-6">
          <MarketCharts
            selectedTickers={selectedTickers}
            tickers={tickers}
            selectedExchanges={selectedExchanges}
            marketType={selectedMarketType}
            quoteSymbol={selectedQuoteSymbol}
            interval={chartInterval}
            onIntervalChange={setChartInterval}
          />
          <TickerTable 
            selectedExchanges={selectedExchanges} 
            selectedTickers={selectedTickers}
            tickers={tickers}
            wsConnected={wsState.connected}
            activeMarketType={selectedMarketType}
            activeQuoteSymbol={selectedQuoteSymbol}
          />
        </div>
      </div>
    </div>
  )
}
