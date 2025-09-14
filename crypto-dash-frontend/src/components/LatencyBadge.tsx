'use client'

import { useState, useEffect } from 'react'
import { Wifi, WifiOff } from 'lucide-react'

export function LatencyBadge() {
  const [latency, setLatency] = useState<number | null>(null)
  const [isConnected, setIsConnected] = useState(false)

  useEffect(() => {
    const measureLatency = async () => {
      try {
        const start = Date.now()
        // In a real app, this would ping the actual API
        await new Promise(resolve => setTimeout(resolve, Math.random() * 50 + 10))
        const end = Date.now()
        
        setLatency(end - start)
        setIsConnected(true)
      } catch {
        setIsConnected(false)
        setLatency(null)
      }
    }

    // Measure latency every 5 seconds
    const interval = setInterval(measureLatency, 5000)
    measureLatency() // Initial measurement

    return () => clearInterval(interval)
  }, [])

  const getLatencyColor = (latency: number) => {
    if (latency < 50) return 'text-green-600'
    if (latency < 100) return 'text-yellow-600'
    return 'text-red-600'
  }

  return (
    <div className="flex items-center space-x-2 text-sm">
      <div className="flex items-center space-x-1">
        {isConnected ? (
          <Wifi className="w-4 h-4 text-green-600" />
        ) : (
          <WifiOff className="w-4 h-4 text-red-600" />
        )}
        <span className="text-muted-foreground">
          {isConnected ? 'Connected' : 'Disconnected'}
        </span>
      </div>
      {latency !== null && (
        <div className="flex items-center space-x-1">
          <span className="text-muted-foreground">•</span>
          <span className={`font-mono ${getLatencyColor(latency)}`}>
            {latency}ms
          </span>
        </div>
      )}
    </div>
  )
}