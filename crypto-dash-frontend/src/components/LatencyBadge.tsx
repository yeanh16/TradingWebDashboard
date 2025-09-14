'use client'

import { Wifi, WifiOff, AlertCircle, X } from 'lucide-react'
import { WebSocketState } from '@/lib/useWebSocket'

interface LatencyBadgeProps {
  wsState: WebSocketState
  onClearError: () => void
}

export function LatencyBadge({ wsState, onClearError }: LatencyBadgeProps) {
  const getLatencyColor = (latency: number) => {
    if (latency < 50) return 'text-green-600'
    if (latency < 100) return 'text-yellow-600'
    return 'text-red-600'
  }

  const getConnectionStatus = () => {
    if (wsState.reconnecting) return { text: 'Reconnecting...', icon: Wifi, color: 'text-yellow-600' }
    if (wsState.connected) return { text: 'Connected', icon: Wifi, color: 'text-green-600' }
    return { text: 'Disconnected', icon: WifiOff, color: 'text-red-600' }
  }

  const status = getConnectionStatus()

  return (
    <div className="flex items-center space-x-4 text-sm">
      {/* Connection Status */}
      <div className="flex items-center space-x-2">
        <status.icon className={`w-4 h-4 ${status.color}`} />
        <span className={status.color}>
          {status.text}
        </span>
      </div>

      {/* Latency */}
      {wsState.latency !== null && (
        <div className="flex items-center space-x-1">
          <span className="text-muted-foreground">•</span>
          <span className={`font-mono ${getLatencyColor(wsState.latency)}`}>
            {wsState.latency}ms
          </span>
        </div>
      )}

      {/* Last Message Time */}
      {wsState.lastMessageTime && (
        <div className="flex items-center space-x-1">
          <span className="text-muted-foreground">•</span>
          <span className="text-muted-foreground text-xs">
            {wsState.lastMessageTime.toLocaleTimeString()}
          </span>
        </div>
      )}

      {/* Error Display */}
      {wsState.error && (
        <div className="flex items-center space-x-2 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 px-2 py-1 rounded-md">
          <AlertCircle className="w-4 h-4" />
          <span className="text-xs">{wsState.error}</span>
          <button
            onClick={onClearError}
            className="ml-1 hover:bg-red-100 dark:hover:bg-red-800/40 rounded p-0.5"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      )}
    </div>
  )
}