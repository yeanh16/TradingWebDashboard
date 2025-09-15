'use client'

import { useState, useEffect, useMemo } from 'react'
import { Search, X, Plus } from 'lucide-react'
import { apiClient } from '@/lib/api'
import { SymbolResponse, SymbolInfo, SelectedTicker } from '@/lib/types'

interface TickerSelectorProps {
  selectedExchanges: string[]
  selectedTickers: SelectedTicker[]
  onTickersChange: (tickers: SelectedTicker[]) => void
}

export function TickerSelector({ 
  selectedExchanges, 
  selectedTickers, 
  onTickersChange 
}: TickerSelectorProps) {
  const [availableSymbols, setAvailableSymbols] = useState<SymbolResponse[]>([])
  const [searchTerm, setSearchTerm] = useState('')
  const [showDropdown, setShowDropdown] = useState(false)
  const [loading, setLoading] = useState(false)

  // Load available symbols when exchanges change
  useEffect(() => {
    const loadSymbols = async () => {
      if (selectedExchanges.length === 0) {
        setAvailableSymbols([])
        return
      }

      setLoading(true)
      try {
        const response = await apiClient.getSymbols() as SymbolResponse[]
        const filteredSymbols = response.filter(item => 
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

  // Filter symbols based on search term
  const filteredSymbols = useMemo(() => {
    if (!searchTerm) return []
    
    const results: Array<SymbolInfo & { exchange: string }> = []
    availableSymbols.forEach(exchangeData => {
      exchangeData.symbols
        .filter(symbol => 
          symbol.symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
          symbol.display_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          symbol.base.toLowerCase().includes(searchTerm.toLowerCase())
        )
        .forEach(symbol => {
          results.push({ ...symbol, exchange: exchangeData.exchange })
        })
    })
    
    return results
  }, [availableSymbols, searchTerm])

  const addTicker = (symbol: SymbolInfo & { exchange: string }) => {
    const newTicker: SelectedTicker = {
      symbol: symbol.symbol,
      base: symbol.base,
      quote: symbol.quote,
      exchange: symbol.exchange,
      display_name: symbol.display_name,
      price_precision: symbol.price_precision,
      tick_size: symbol.tick_size,
      min_qty: symbol.min_qty,
      step_size: symbol.step_size,
    }

    // Check if ticker is already selected
    const isAlreadySelected = selectedTickers.some(
      ticker => ticker.symbol === newTicker.symbol && ticker.exchange === newTicker.exchange
    )

    if (!isAlreadySelected) {
      onTickersChange([...selectedTickers, newTicker])
    }
    
    setSearchTerm('')
    setShowDropdown(false)
  }

  const removeTicker = (index: number) => {
    const newTickers = selectedTickers.filter((_, i) => i !== index)
    onTickersChange(newTickers)
  }

  const isTickerSelected = (symbol: SymbolInfo & { exchange: string }) => {
    return selectedTickers.some(
      ticker => ticker.symbol === symbol.symbol && ticker.exchange === symbol.exchange
    )
  }

  return (
    <div className="rounded-lg border bg-card p-6" data-testid="ticker-selector">
      <h3 className="text-lg font-semibold mb-4">Selected Tickers</h3>
      
      {/* Selected tickers */}
      <div className="space-y-2 mb-4">
        {selectedTickers.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No tickers selected. Search below to add tickers.
          </p>
        ) : (
          selectedTickers.map((ticker, index) => (
            <div
              key={`${ticker.exchange}-${ticker.symbol}`}
              className="flex items-center justify-between bg-accent/50 rounded-md p-2"
            >
              <div className="flex-1">
                <div className="text-sm font-medium">{ticker.symbol}</div>
                <div className="text-xs text-muted-foreground capitalize">
                  {ticker.exchange} • {ticker.display_name}
                </div>
              </div>
              <button
                onClick={() => removeTicker(index)}
                className="text-muted-foreground hover:text-foreground transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          ))
        )}
      </div>

      {/* Search input */}
      <div className="relative">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground w-4 h-4" />
          <input
            type="text"
            placeholder="Search for tickers (e.g., BTC, ETH, bitcoin)..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onFocus={() => setShowDropdown(true)}
            onBlur={() => {
              // Delay hiding to allow clicks on dropdown items
              setTimeout(() => setShowDropdown(false), 200)
            }}
            className="w-full pl-10 pr-4 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            disabled={selectedExchanges.length === 0}
          />
        </div>

        {/* Dropdown */}
        {showDropdown && searchTerm && (
          <div className="absolute top-full left-0 right-0 mt-1 bg-background border border-border rounded-md shadow-lg z-50 max-h-60 overflow-y-auto">
            {loading ? (
              <div className="p-4 text-center text-muted-foreground">
                Loading symbols...
              </div>
            ) : filteredSymbols.length === 0 ? (
              <div className="p-4 text-center text-muted-foreground">
                No symbols found
              </div>
            ) : (
              filteredSymbols.slice(0, 20).map((symbol) => (
                <div
                  key={`${symbol.exchange}-${symbol.symbol}`}
                  className={`p-3 hover:bg-accent cursor-pointer border-b border-border last:border-b-0 ${
                    isTickerSelected(symbol) ? 'bg-accent/50' : ''
                  }`}
                  onClick={() => addTicker(symbol)}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <div className="font-medium text-sm">{symbol.symbol}</div>
                      <div className="text-xs text-muted-foreground">
                        {symbol.display_name}
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <span className="text-xs bg-muted px-2 py-1 rounded capitalize">
                        {symbol.exchange}
                      </span>
                      {isTickerSelected(symbol) ? (
                        <div className="text-green-600 text-xs">Added</div>
                      ) : (
                        <Plus className="w-4 h-4 text-muted-foreground" />
                      )}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {selectedExchanges.length === 0 && (
        <p className="text-xs text-muted-foreground mt-2">
          Please select at least one exchange to search for tickers.
        </p>
      )}
    </div>
  )
}