import { renderHook, act, waitFor } from '@testing-library/react';
import { Server } from 'mock-socket';
import { useWebSocket } from '@/lib/useWebSocket';
import { Channel, StreamMessage, Ticker } from '@/lib/types';

// Mock the API client
jest.mock('@/lib/api', () => ({
  apiClient: {
    getWebSocketUrl: () => 'ws://localhost:8080/ws',
  },
}));

describe('useWebSocket', () => {
  let mockServer: Server;

  beforeEach(() => {
    mockServer = new Server('ws://localhost:8080/ws');
    jest.clearAllTimers();
    jest.useFakeTimers();
  });

  afterEach(() => {
    mockServer.stop();
    jest.clearAllTimers();
    jest.useRealTimers();
  });

  it('should initialize with default state', () => {
    const { result } = renderHook(() => useWebSocket());

    expect(result.current.state).toEqual({
      connected: false,
      reconnecting: false,
      latency: null,
      lastMessageTime: null,
      error: null,
    });
    expect(result.current.tickers).toEqual({});
  });

  it('should connect to WebSocket on mount', async () => {
    mockServer.on('connection', (socket) => {
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected to crypto-dash API',
      }));
    });

    const { result } = renderHook(() => useWebSocket());

    // Fast-forward timers to trigger connection
    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      expect(result.current.state.connected).toBe(true);
    });
  });

  it('should handle ticker messages', async () => {
    const mockTicker: Ticker = {
      timestamp: '2024-01-01T00:00:00Z',
      exchange: 'binance',
      symbol: { base: 'BTC', quote: 'USDT' },
      bid: 50000,
      ask: 50001,
      last: 50000.5,
      bid_size: 1.0,
      ask_size: 1.0,
    };

    mockServer.on('connection', (socket) => {
      socket.send(JSON.stringify({
        type: 'ticker',
        payload: mockTicker,
      }));
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      const tickerKey = 'binance_BTCUSDT';
      expect(result.current.tickers[tickerKey]).toEqual(mockTicker);
    });
  });

  it('should handle subscription requests', async () => {
    let receivedMessage: any = null;

    mockServer.on('connection', (socket) => {
      socket.on('message', (data) => {
        receivedMessage = JSON.parse(data as string);
      });
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      expect(result.current.state.connected).toBe(true);
    });

    const channels: Channel[] = [{
      channel_type: 'ticker',
      exchange: 'binance',
      symbol: { base: 'BTC', quote: 'USDT' },
    }];

    act(() => {
      result.current.subscribe(channels);
    });

    await waitFor(() => {
      expect(receivedMessage).toEqual({
        op: 'subscribe',
        channels,
      });
    });
  });

  it('should handle unsubscription requests', async () => {
    let receivedMessage: any = null;

    mockServer.on('connection', (socket) => {
      socket.on('message', (data) => {
        receivedMessage = JSON.parse(data as string);
      });
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      expect(result.current.state.connected).toBe(true);
    });

    const channels: Channel[] = [{
      channel_type: 'ticker',
      exchange: 'binance',
      symbol: { base: 'BTC', quote: 'USDT' },
    }];

    act(() => {
      result.current.unsubscribe(channels);
    });

    await waitFor(() => {
      expect(receivedMessage).toEqual({
        op: 'unsubscribe',
        channels,
      });
    });
  });

  it('should handle ping/pong for latency measurement', async () => {
    let pingReceived = false;

    mockServer.on('connection', (socket) => {
      socket.on('message', (data) => {
        const message = JSON.parse(data as string);
        if (message.op === 'ping') {
          pingReceived = true;
          socket.send(JSON.stringify({
            type: 'info',
            message: 'pong',
          }));
        }
      });
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      expect(result.current.state.connected).toBe(true);
    });

    // Trigger ping interval
    act(() => {
      jest.advanceTimersByTime(30000);
    });

    await waitFor(() => {
      expect(pingReceived).toBe(true);
      expect(result.current.state.latency).toBeGreaterThanOrEqual(0);
    });
  });

  it('should handle connection errors', async () => {
    mockServer.on('connection', (socket) => {
      socket.close({ code: 1006, reason: 'Connection failed' });
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      expect(result.current.state.connected).toBe(false);
    });
  });

  it('should attempt reconnection after disconnect', async () => {
    let connectionCount = 0;

    mockServer.on('connection', (socket) => {
      connectionCount++;
      if (connectionCount === 1) {
        // First connection - close immediately
        socket.close();
      } else {
        // Second connection - keep open
        socket.send(JSON.stringify({
          type: 'info',
          message: 'Reconnected',
        }));
      }
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    // Wait for initial connection and disconnect
    await waitFor(() => {
      expect(result.current.state.connected).toBe(false);
    });

    // Fast-forward to trigger reconnection
    act(() => {
      jest.advanceTimersByTime(3000);
    });

    await waitFor(() => {
      expect(result.current.state.connected).toBe(true);
      expect(connectionCount).toBe(2);
    });
  });

  it('should handle error messages', async () => {
    const errorMessage = 'Test error message';

    mockServer.on('connection', (socket) => {
      socket.send(JSON.stringify({
        type: 'error',
        message: errorMessage,
      }));
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    await waitFor(() => {
      expect(result.current.state.error).toBe(errorMessage);
    });
  });

  it('should clear errors when requested', async () => {
    const { result } = renderHook(() => useWebSocket());

    // Set an error state
    act(() => {
      result.current.state.error = 'Test error';
    });

    act(() => {
      result.current.clearError();
    });

    expect(result.current.state.error).toBeNull();
  });

  it('should handle malformed JSON messages gracefully', async () => {
    mockServer.on('connection', (socket) => {
      socket.send('invalid json');
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    // Should not crash and should maintain connection
    await waitFor(() => {
      expect(result.current.state.connected).toBe(true);
    });
  });

  it('should update lastMessageTime when receiving messages', async () => {
    const mockTicker: Ticker = {
      timestamp: '2024-01-01T00:00:00Z',
      exchange: 'binance',
      symbol: { base: 'BTC', quote: 'USDT' },
      bid: 50000,
      ask: 50001,
      last: 50000.5,
      bid_size: 1.0,
      ask_size: 1.0,
    };

    mockServer.on('connection', (socket) => {
      setTimeout(() => {
        socket.send(JSON.stringify({
          type: 'ticker',
          payload: mockTicker,
        }));
      }, 100);
    });

    const { result } = renderHook(() => useWebSocket());

    act(() => {
      jest.advanceTimersByTime(100);
    });

    const initialTime = result.current.state.lastMessageTime;

    act(() => {
      jest.advanceTimersByTime(200);
    });

    await waitFor(() => {
      expect(result.current.state.lastMessageTime).not.toBe(initialTime);
      expect(result.current.state.lastMessageTime).toBeInstanceOf(Date);
    });
  });
});