'use client'

import { useState } from 'react'
import { Check } from 'lucide-react'

interface Exchange {
  id: string
  name: string
  status: 'online' | 'offline' | 'maintenance'
}

interface ExchangeSelectorProps {
  exchanges: Exchange[]
  selectedExchanges: string[]
  onSelectionChange: (selected: string[]) => void
}

export function ExchangeSelector({
  exchanges,
  selectedExchanges,
  onSelectionChange,
}: ExchangeSelectorProps) {
  const toggleExchange = (exchangeId: string) => {
    if (selectedExchanges.includes(exchangeId)) {
      onSelectionChange(selectedExchanges.filter(id => id !== exchangeId))
    } else {
      onSelectionChange([...selectedExchanges, exchangeId])
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online':
        return 'bg-green-500'
      case 'offline':
        return 'bg-red-500'
      case 'maintenance':
        return 'bg-yellow-500'
      default:
        return 'bg-gray-500'
    }
  }

  return (
    <div className="rounded-lg border bg-card p-6">
      <h3 className="text-lg font-semibold mb-4">Exchanges</h3>
      <div className="space-y-3">
        {exchanges.map((exchange) => (
          <div
            key={exchange.id}
            className="flex items-center space-x-3 cursor-pointer hover:bg-accent rounded-md p-2 transition-colors"
            onClick={() => toggleExchange(exchange.id)}
          >
            <div className="relative">
              <div
                className={`w-4 h-4 rounded border-2 flex items-center justify-center ${
                  selectedExchanges.includes(exchange.id)
                    ? 'bg-primary border-primary'
                    : 'border-border'
                }`}
              >
                {selectedExchanges.includes(exchange.id) && (
                  <Check className="w-3 h-3 text-primary-foreground" />
                )}
              </div>
            </div>
            <div className="flex-1">
              <div className="flex items-center space-x-2">
                <span className="text-sm font-medium">{exchange.name}</span>
                <div
                  className={`w-2 h-2 rounded-full ${getStatusColor(exchange.status)}`}
                  title={exchange.status}
                />
              </div>
              <div className="text-xs text-muted-foreground capitalize">
                {exchange.status}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}