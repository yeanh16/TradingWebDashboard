'use client'

import { useEffect, useRef, useState } from 'react'
import type {
  CandlestickData,
  IChartApi,
  ISeriesApi,
  UTCTimestamp,
} from 'lightweight-charts'
import { apiClient } from '@/lib/api'
import type { Candle, SelectedTicker, Ticker } from '@/lib/types'

interface CandlestickChartProps {
  ticker: SelectedTicker
  interval: string
  liveTicker?: Ticker
}

const CHART_HEIGHT = 320
const DEFAULT_CANDLE_LIMIT = 500

const intervalToSeconds = (interval: string): number => {
  if (!interval) {
    return 60
  }

  const unit = interval.slice(-1)
  const value = Number(interval.slice(0, -1))
  if (!Number.isFinite(value) || value <= 0) {
    return 60
  }

  switch (unit) {
    case 'm':
    case 'M':
      return value * 60
    case 'h':
    case 'H':
      return value * 60 * 60
    case 'd':
    case 'D':
      return value * 60 * 60 * 24
    case 'w':
    case 'W':
      return value * 60 * 60 * 24 * 7
    default:
      return 60
  }
}

const parseValue = (value: string | number): number => {
  if (typeof value === 'number') {
    return value
  }
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

const toNumber = (value: number | string): number => {
  return typeof value === 'number' ? value : Number(value)
}

const resolveMinMove = (tickSize?: string, precision?: number): number | undefined => {
  if (tickSize) {
    const parsed = Number(tickSize)
    if (Number.isFinite(parsed) && parsed > 0) {
      return parsed
    }
  }

  if (typeof precision === 'number' && precision >= 0) {
    return Number((1 / Math.pow(10, precision)).toFixed(precision))
  }

  return undefined
}

const formatSymbolLabel = (ticker: SelectedTicker) => `${ticker.exchange.toUpperCase()} - ${ticker.base}/${ticker.quote}`

export function CandlestickChart({ ticker, liveTicker, interval }: CandlestickChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null)
  const lastCandleRef = useRef<CandlestickData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [cached, setCached] = useState(false)
  const [chartReady, setChartReady] = useState(false)
  const intervalSecondsRef = useRef(intervalToSeconds(interval))

  useEffect(() => {
    intervalSecondsRef.current = intervalToSeconds(interval)
  }, [interval])

  useEffect(() => {
    let disposed = false
    let resizeObserver: ResizeObserver | null = null

    const mountChart = async () => {
      const container = containerRef.current
      if (!container) {
        return
      }

      const { createChart, ColorType, CandlestickSeries } = await import('lightweight-charts')

      if (disposed || !containerRef.current) {
        return
      }

      const rootStyle = getComputedStyle(document.documentElement)
      const textColor = rootStyle.getPropertyValue('--foreground').trim() || '#e5e7eb'
      const gridColor = rootStyle.getPropertyValue('--muted').trim() || 'rgba(148, 163, 184, 0.2)'

      const chart = createChart(container, {
        width: container.clientWidth,
        height: CHART_HEIGHT,
        layout: {
          background: {
            type: ColorType.Solid,
            color: 'transparent',
          },
          textColor,
        },
        grid: {
          vertLines: { color: gridColor },
          horzLines: { color: gridColor },
        },
        rightPriceScale: {
          borderVisible: false,
        },
        timeScale: {
          borderVisible: false,
        },
        localization: {
          priceFormatter: (price: number) => price.toString(),
        },
      })

      const series = chart.addSeries(CandlestickSeries, {
        upColor: '#22c55e',
        downColor: '#ef4444',
        borderUpColor: '#16a34a',
        borderDownColor: '#dc2626',
        wickUpColor: '#16a34a',
        wickDownColor: '#dc2626',
      })

      chartRef.current = chart
      seriesRef.current = series
      setChartReady(true)

      resizeObserver = new ResizeObserver((entries) => {
        if (!chartRef.current) {
          return
        }

        for (const entry of entries) {
          const { width, height } = entry.contentRect
          chartRef.current.applyOptions({ width, height })
        }
      })

      resizeObserver.observe(container)
    }

    mountChart()

    return () => {
      disposed = true
      if (resizeObserver) {
        resizeObserver.disconnect()
        resizeObserver = null
      }
      setChartReady(false)
      chartRef.current?.remove()
      chartRef.current = null
      seriesRef.current = null
      lastCandleRef.current = null
    }
  }, [])

  useEffect(() => {
    if (!chartReady || !seriesRef.current) {
      return
    }

    let cancelled = false
    setLoading(true)
    setError(null)
    setCached(false)
    lastCandleRef.current = null

    const loadCandles = async () => {
      try {
        const symbolParam = `${ticker.base}${ticker.quote}`
        const response = await apiClient.getCandles({
          exchange: ticker.exchange,
          symbol: symbolParam,
          interval,
          limit: DEFAULT_CANDLE_LIMIT,
          market_type: ticker.market_type,
        })

        if (cancelled) {
          return
        }

        const candleData: CandlestickData[] = response.candles
          .map<CandlestickData>((candle: Candle) => ({
            time: Math.floor(new Date(candle.timestamp).getTime() / 1000) as UTCTimestamp,
            open: parseValue(candle.open),
            high: parseValue(candle.high),
            low: parseValue(candle.low),
            close: parseValue(candle.close),
          }))
          .filter((bar) =>
            Number.isFinite(bar.open) &&
            Number.isFinite(bar.high) &&
            Number.isFinite(bar.low) &&
            Number.isFinite(bar.close)
          )
          .sort((a, b) => (a.time as number) - (b.time as number))

        seriesRef.current?.setData(candleData)
        lastCandleRef.current = candleData.length > 0 ? candleData[candleData.length - 1] : null
        setCached(response.cached)
        chartRef.current?.timeScale().fitContent()
      } catch (err) {
        console.error('Failed to load candles', err)
        if (!cancelled) {
          setError('Unable to load historical data')
        }
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    }

    const minMove = resolveMinMove(ticker.tick_size, ticker.price_precision)
    if (minMove && seriesRef.current) {
      const derivedPrecision = Math.max(Math.round(-Math.log10(minMove)), 0)
      seriesRef.current.applyOptions({
        priceFormat: {
          type: 'price',
          precision: ticker.price_precision ?? derivedPrecision,
          minMove,
        },
      })
    } else {
      seriesRef.current?.applyOptions({
        priceFormat: {
          type: 'price',
          precision: ticker.price_precision ?? 2,
        },
      })
    }

    loadCandles()

    return () => {
      cancelled = true
    }
  }, [chartReady, ticker.exchange, ticker.base, ticker.quote, ticker.market_type, ticker.tick_size, ticker.price_precision, interval])

  useEffect(() => {
    if (!liveTicker || !seriesRef.current) {
      return
    }

    const price = toNumber(liveTicker.last)
    if (!Number.isFinite(price)) {
      return
    }

    const timestamp = new Date(liveTicker.timestamp).getTime()
    if (Number.isNaN(timestamp)) {
      return
    }

    const lastCandle = lastCandleRef.current
    if (!lastCandle) {
      return
    }

    const intervalSeconds = intervalSecondsRef.current
    const candleTimestamp = Math.floor(timestamp / 1000 / intervalSeconds) * intervalSeconds
    const candleTime = candleTimestamp as UTCTimestamp
    const lastTime = lastCandle.time as number

    if (candleTime < lastTime) {
      return
    }

    if (candleTime > lastTime) {
      const prevClose = toNumber(lastCandle.close ?? lastCandle.open)
      const open = Number.isFinite(prevClose) ? prevClose : price
      const high = Math.max(open, price)
      const low = Math.min(open, price)

      const newCandle: CandlestickData = {
        time: candleTime,
        open,
        high,
        low,
        close: price,
      }
      seriesRef.current.update(newCandle)
      lastCandleRef.current = newCandle
      return
    }

    const updated: CandlestickData = {
      time: candleTime,
      open: toNumber(lastCandle.open),
      high: Math.max(toNumber(lastCandle.high), price),
      low: Math.min(toNumber(lastCandle.low), price),
      close: price,
    }
    seriesRef.current.update(updated)
    lastCandleRef.current = updated
  }, [liveTicker])

  return (
    <div className="rounded-lg border bg-card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <div>
          <h3 className="text-sm font-semibold uppercase tracking-wide">{formatSymbolLabel(ticker)}</h3>
          <p className="text-xs text-muted-foreground">Interval: {interval.toUpperCase()} {cached ? '(cached)' : ''}</p>
        </div>
      </div>
      <div className="relative">
        <div ref={containerRef} className="h-[320px] w-full" />
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-background/60">
            <div className="text-sm text-muted-foreground">Loading candles...</div>
          </div>
        )}
        {error && !loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-background/80">
            <div className="text-sm text-red-500">{error}</div>
          </div>
        )}
      </div>
    </div>
  )
}
