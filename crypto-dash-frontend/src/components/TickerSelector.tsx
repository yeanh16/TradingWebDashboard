'use client'

import { useState, useEffect, useMemo } from 'react'
import { Search, X, Plus } from 'lucide-react'
import { apiClient } from '@/lib/api'
import { SymbolResponse, SymbolInfo, SelectedTicker, MarketType } from '@/lib/types'

const MARKET_TYPE_LABEL: Record<MarketType, string> = {
  spot: 'Spot',
  perpetual: 'Perpetual',
}

const formatExchange = (value: string) => (value ? value.charAt(0).toUpperCase() + value.slice(1) : value)

interface TickerSelectorProps {
  selectedExchanges: string[]
  selectedTickers: SelectedTicker[]
  onTickersChange: (tickers: SelectedTicker[]) => void
  activeMarketType: MarketType
  activeQuoteSymbol: string
}

type SymbolOption = SymbolInfo & { exchange: string }

type GroupedSymbolOption = {
  key: string
  base: string
  displayName: string
  options: SymbolOption[]
}

type SelectedTickerGroup = {
  key: string
  base: string
  displayName: string
  entries: SelectedTicker[]
  exchanges: string[]
  quotes: string[]
  markets: MarketType[]
}

export function TickerSelector({
  selectedExchanges,
  selectedTickers,
  onTickersChange,
  activeMarketType,
  activeQuoteSymbol,
}: TickerSelectorProps) {
  const [availableSymbols, setAvailableSymbols] = useState<SymbolResponse[]>([])
  const [searchTerm, setSearchTerm] = useState('')
  const [showDropdown, setShowDropdown] = useState(false)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    const loadSymbols = async () => {
      if (selectedExchanges.length === 0) {
        setAvailableSymbols([])
        return
      }

      setLoading(true)
      try {
        const response = (await apiClient.getSymbols()) as SymbolResponse[]
        const filteredSymbols = response.filter((item) =>
          selectedExchanges.includes(item.exchange)
        )
        setAvailableSymbols(filteredSymbols)
      } catch (error) {
        console.error('Failed to load symbols:', error)
        setAvailableSymbols([])
      } finally {
        setLoading(false)
      }
    }

    loadSymbols()
  }, [selectedExchanges])

  const groupedSymbols = useMemo<GroupedSymbolOption[]>(() => {
    if (!searchTerm) {
      return []
    }

    const term = searchTerm.toLowerCase()
    const groups = new Map<string, GroupedSymbolOption>()

    availableSymbols.forEach((exchangeData) => {
      exchangeData.symbols
        .filter((symbol) => {
          const base = symbol.base.toLowerCase();
          const [displayBaseRaw] = symbol.display_name.split('/');
          const displayBase = (displayBaseRaw ?? symbol.base).trim().toLowerCase();

          const matchesBase = (
            base.includes(term) ||
            term.includes(base) ||
            displayBase.includes(term)
          );

          return matchesBase;
        })
        .forEach((symbol) => {
          const key = symbol.base.toUpperCase()
          if (!groups.has(key)) {
            const displayName = symbol.display_name.split(' / ')[0] ?? symbol.base
            groups.set(key, {
              key,
              base: symbol.base,
              displayName,
              options: [],
            })
          }

          groups.get(key)?.options.push({ ...symbol, exchange: exchangeData.exchange })
        })
    })

    const result = Array.from(groups.values())

    result.forEach((group) => {
      group.options.sort((a, b) => {
        if (a.exchange !== b.exchange) {
          return a.exchange.localeCompare(b.exchange)
        }
        if (a.market_type !== b.market_type) {
          return a.market_type.localeCompare(b.market_type)
        }
        return a.quote.localeCompare(b.quote)
      })
    })

    result.sort((a, b) => a.displayName.localeCompare(b.displayName))

    return result
  }, [availableSymbols, searchTerm])

  const selectedGroups = useMemo<SelectedTickerGroup[]>(() => {
    const map = new Map<string, SelectedTickerGroup>()

    selectedTickers.forEach((ticker) => {
      const key = ticker.base.toUpperCase()
      if (!map.has(key)) {
        const displayName = ticker.display_name?.split(' / ')[0] ?? ticker.base
        map.set(key, {
          key,
          base: ticker.base,
          displayName,
          entries: [],
          exchanges: [],
          quotes: [],
          markets: [],
        })
      }

      map.get(key)!.entries.push(ticker)
    })

    const result = Array.from(map.values())

    result.forEach((group) => {
      group.entries.sort((a, b) => {
        if (a.exchange !== b.exchange) {
          return a.exchange.localeCompare(b.exchange)
        }
        if (a.market_type !== b.market_type) {
          return a.market_type.localeCompare(b.market_type)
        }
        return a.quote.localeCompare(b.quote)
      })

      group.exchanges = Array.from(new Set(group.entries.map((entry) => entry.exchange))).sort((a, b) =>
        a.localeCompare(b)
      )
      group.quotes = Array.from(new Set(group.entries.map((entry) => entry.quote))).sort((a, b) =>
        a.localeCompare(b)
      )
      group.markets = Array.from(new Set(group.entries.map((entry) => entry.market_type))).sort((a, b) =>
        a.localeCompare(b)
      )
    })

    result.sort((a, b) => a.displayName.localeCompare(b.displayName))

    return result
  }, [selectedTickers])

  const isTickerSelected = (symbol: SymbolOption) => {
    return selectedTickers.some(
      (ticker) =>
        ticker.symbol === symbol.symbol &&
        ticker.exchange === symbol.exchange &&
        ticker.market_type === symbol.market_type
    )
  }

  const isGroupSelected = (group: GroupedSymbolOption) => group.options.every(isTickerSelected)

  const addCoinGroup = (group: GroupedSymbolOption) => {
    const additions = group.options.filter((option) => !isTickerSelected(option))

    if (additions.length === 0) {
      setSearchTerm('')
      setShowDropdown(false)
      return
    }

    const newTickers: SelectedTicker[] = additions.map((symbol) => ({
      symbol: symbol.symbol,
      base: symbol.base,
      quote: symbol.quote,
      exchange: symbol.exchange,
      market_type: symbol.market_type,
      display_name: symbol.display_name,
      price_precision: symbol.price_precision,
      tick_size: symbol.tick_size,
      min_qty: symbol.min_qty,
      step_size: symbol.step_size,
    }))

    onTickersChange([...selectedTickers, ...newTickers])
    setSearchTerm('')
    setShowDropdown(false)
  }

  const removeTickerGroup = (groupKey: string) => {
    const remaining = selectedTickers.filter((ticker) => ticker.base.toUpperCase() !== groupKey)
    onTickersChange(remaining)
  }

  return (
    <div className="rounded-lg border bg-card p-6" data-testid="ticker-selector">
      <h3 className="text-lg font-semibold mb-4">Selected Coins</h3>

      <div className="space-y-2 mb-4">
        {selectedGroups.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No coins selected. Search below to add coins.
          </p>
        ) : (
          selectedGroups.map((group) => {
            const summaryParts: string[] = []

            if (group.exchanges.length > 0) {
              const exchangeLabel = group.exchanges.map(formatExchange).join(', ')
              summaryParts.push(`Exchanges: ${exchangeLabel}`)
            }

            if (group.quotes.length > 0) {
              summaryParts.push(`Quotes: ${group.quotes.join(', ')}`)
            }

            if (group.markets.length > 0) {
              const marketLabel = group.markets.map((market) => MARKET_TYPE_LABEL[market]).join(', ')
              summaryParts.push(`Markets: ${marketLabel}`)
            }

            const summary = summaryParts.join(' | ')

            return (
              <div
                key={group.key}
                className="rounded-md border border-border bg-accent/40 p-3 space-y-2"
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="text-sm font-medium">{group.displayName}</div>
                    {summary && (
                      <div className="text-xs text-muted-foreground mt-1">{summary}</div>
                    )}
                  </div>
                  <button
                    onClick={() => removeTickerGroup(group.key)}
                    className="text-muted-foreground hover:text-foreground transition-colors"
                    aria-label={`Remove ${group.displayName}`}
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>

                <div className="flex flex-wrap gap-2">
                  {group.entries.map((entry) => {
                    const isActive =
                      entry.market_type === activeMarketType &&
                      entry.quote === activeQuoteSymbol

                    return (
                      <span
                        key={`${entry.exchange}-${entry.symbol}-${entry.market_type}`}
                        className={`inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs capitalize ${
                          isActive ? 'border-primary bg-primary/10 text-primary'
                            : 'border-border text-muted-foreground'
                        }`}
                      >
                        <span className="font-medium">{formatExchange(entry.exchange)}</span>
                        <span className="text-[11px] uppercase text-muted-foreground">{MARKET_TYPE_LABEL[entry.market_type].toUpperCase()}</span>
                        <span>{entry.quote}</span>
                      </span>
                    )
                  })}
                </div>
              </div>
            )
          })
        )}
      </div>

      <div className="relative">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground w-4 h-4" />
          <input
            type="text"
            placeholder="Search for coins (e.g., BTC, ETH, Bitcoin)..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onFocus={() => setShowDropdown(true)}
            onBlur={() => {
              setTimeout(() => setShowDropdown(false), 200)
            }}
            className="w-full pl-10 pr-4 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            disabled={selectedExchanges.length === 0}
          />
        </div>

        {showDropdown && searchTerm && (
          <div className="absolute top-full left-0 right-0 mt-1 bg-background border border-border rounded-md shadow-lg z-50 max-h-72 overflow-y-auto">
            {loading ? (
              <div className="p-4 text-center text-muted-foreground">Loading coins...</div>
            ) : groupedSymbols.length === 0 ? (
              <div className="p-4 text-center text-muted-foreground">No coins found</div>
            ) : (
              groupedSymbols.slice(0, 20).map((group) => {
                const exchanges = Array.from(new Set(group.options.map((option) => option.exchange))).sort((a, b) =>
                  a.localeCompare(b)
                )
                const quotes = Array.from(new Set(group.options.map((option) => option.quote))).sort((a, b) =>
                  a.localeCompare(b)
                )
                const markets = Array.from(new Set(group.options.map((option) => option.market_type))).sort((a, b) =>
                  a.localeCompare(b)
                )

                const summaryParts: string[] = []

                if (exchanges.length > 0) {
                  summaryParts.push(`Exchanges: ${exchanges.map(formatExchange).join(', ')}`)
                }

                if (quotes.length > 0) {
                  summaryParts.push(`Quotes: ${quotes.join(', ')}`)
                }

                if (markets.length > 0) {
                  summaryParts.push(
                    `Markets: ${markets.map((market) => MARKET_TYPE_LABEL[market]).join(', ')}`
                  )
                }

                const summary = summaryParts.join(' | ')
                const allSelected = isGroupSelected(group)

                return (
                  <button
                    key={group.key}
                    type="button"
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => addCoinGroup(group)}
                    disabled={allSelected}
                    className={`w-full text-left p-3 border-b border-border last:border-b-0 transition-colors ${
                      allSelected ? 'opacity-60 cursor-not-allowed' : 'hover:bg-accent/60'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <div className="font-medium text-sm">{group.displayName}</div>
                        {summary && (
                          <div className="text-xs text-muted-foreground mt-1">{summary}</div>
                        )}
                      </div>
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        {allSelected ? (
                          <span className="text-green-600 font-medium">Added</span>
                        ) : (
                          <>
                            <Plus className="w-3 h-3" />
                            <span>Add</span>
                          </>
                        )}
                      </div>
                    </div>
                  </button>
                )
              })
            )}
          </div>
        )}
      </div>

      {selectedExchanges.length === 0 && (
        <p className="text-xs text-muted-foreground mt-2">
          Please select at least one exchange to search for coins.
        </p>
      )}
    </div>
  )
}

