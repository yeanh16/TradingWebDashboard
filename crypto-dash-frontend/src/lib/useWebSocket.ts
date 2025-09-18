import { useEffect, useRef, useState, useCallback } from 'react'
import { apiClient } from '@/lib/api'
import { StreamMessage, ClientMessage, Ticker, Channel } from '@/lib/types'

export interface WebSocketState {
  connected: boolean
  reconnecting: boolean
  latency: number | null
  lastMessageTime: Date | null
  error: string | null
}

export interface UseWebSocketReturn {
  state: WebSocketState
  tickers: Record<string, Ticker>
  subscribe: (channels: Channel[]) => void
  unsubscribe: (channels: Channel[]) => void
  clearError: () => void
}

export function useWebSocket(): UseWebSocketReturn {
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const pingIntervalRef = useRef<NodeJS.Timeout | null>(null)
  const lastPingRef = useRef<number | null>(null)
  
  const [state, setState] = useState<WebSocketState>({
    connected: false,
    reconnecting: false,
    latency: null,
    lastMessageTime: null,
    error: null,
  })
  
  const [tickers, setTickers] = useState<Record<string, Ticker>>({})
  
  const clearError = useCallback(() => {
    setState(prev => ({ ...prev, error: null }))
  }, [])
  
  const sendMessage = (message: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message))
    }
  }
  
  const subscribe = useCallback((channels: Channel[]) => {
    sendMessage({ op: 'subscribe', channels })
  }, [])
  
  const unsubscribe = useCallback((channels: Channel[]) => {
    sendMessage({ op: 'unsubscribe', channels })
  }, [])
  
  const startPingInterval = () => {
    if (pingIntervalRef.current) {
      clearInterval(pingIntervalRef.current)
    }
    
    pingIntervalRef.current = setInterval(() => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        lastPingRef.current = Date.now()
        sendMessage({ op: 'ping' })
      }
    }, 30000) // Ping every 30 seconds
  }
  
  const stopPingInterval = () => {
    if (pingIntervalRef.current) {
      clearInterval(pingIntervalRef.current)
      pingIntervalRef.current = null
    }
  }
  
  const connect = () => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return
    }
    
    setState(prev => ({ 
      ...prev, 
      reconnecting: prev.connected, 
      error: null 
    }))
    
    try {
      const wsUrl = apiClient.getWebSocketUrl()
      wsRef.current = new WebSocket(wsUrl)
      
      wsRef.current.onopen = () => {
        console.log('WebSocket connected')
        setState(prev => ({
          ...prev,
          connected: true,
          reconnecting: false,
          error: null,
        }))
        startPingInterval()
      }
      
      wsRef.current.onmessage = (event) => {
        const now = new Date()
        setState(prev => ({ ...prev, lastMessageTime: now }))
        
        try {
          const message: StreamMessage = JSON.parse(event.data)
          
          // Handle pong messages for latency calculation
          if (message.type === 'info' && message.message === 'pong') {
            if (lastPingRef.current) {
              const latency = Date.now() - lastPingRef.current
              setState(prev => ({ ...prev, latency }))
            }
            return
          }
          
          // Handle ticker updates
          if (message.type === 'ticker' && message.payload) {
            const ticker = message.payload as Ticker
            const tickerKey = `${ticker.exchange}_${ticker.market_type}_${ticker.symbol.base}${ticker.symbol.quote}`
            setTickers(prev => ({
              ...prev,
              [tickerKey]: ticker
            }))
          }
          
          // Handle other message types as needed
          if (message.type === 'error') {
            console.error('WebSocket error message:', message.message)
            setState(prev => ({ ...prev, error: message.message || 'Unknown error' }))
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error)
        }
      }
      
      wsRef.current.onclose = (event) => {
        console.log('WebSocket disconnected:', event.code, event.reason)
        setState(prev => ({
          ...prev,
          connected: false,
          reconnecting: false,
        }))
        stopPingInterval()
        
        // Auto-reconnect after 3 seconds unless it was a manual close
        if (event.code !== 1000) {
          if (reconnectTimeoutRef.current) {
            clearTimeout(reconnectTimeoutRef.current)
          }
          reconnectTimeoutRef.current = setTimeout(() => {
            console.log('Attempting to reconnect...')
            connect()
          }, 3000)
        }
      }
      
      wsRef.current.onerror = (error) => {
        console.error('WebSocket error:', error)
        setState(prev => ({
          ...prev,
          error: 'Connection error. Please check your network.',
        }))
      }
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error)
      setState(prev => ({
        ...prev,
        error: 'Failed to connect. Please try again.',
      }))
    }
  }
  
  const disconnect = () => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
    
    stopPingInterval()
    
    if (wsRef.current) {
      wsRef.current.close(1000, 'Manual disconnect')
      wsRef.current = null
    }
    
    setState({
      connected: false,
      reconnecting: false,
      latency: null,
      lastMessageTime: null,
      error: null,
    })
  }
  
  useEffect(() => {
    connect()
    
    return () => {
      disconnect()
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  
  return {
    state,
    tickers,
    subscribe,
    unsubscribe,
    clearError,
  }
}