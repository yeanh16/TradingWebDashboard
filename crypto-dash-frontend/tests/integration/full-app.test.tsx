import { render, screen, waitFor, act } from '@testing-library/react';
import { Server } from 'mock-socket';
import HomePage from '@/app/page';

// Mock the API module
const mockApiClient = {
  getExchanges: jest.fn(),
  getWebSocketUrl: () => 'ws://localhost:8080/ws',
};

jest.mock('@/lib/api', () => ({
  apiClient: mockApiClient,
}));

describe('Full Application Integration Tests', () => {
  let mockServer: Server;

  beforeEach(() => {
    mockServer = new Server('ws://localhost:8080/ws');
    jest.clearAllTimers();
    jest.useFakeTimers();
    
    // Reset mocks
    mockApiClient.getExchanges.mockClear();
  });

  afterEach(() => {
    mockServer.stop();
    jest.clearAllTimers();
    jest.useRealTimers();
  });

  test('should integrate WebSocket data with UI components', async () => {
    // Mock API response for exchanges
    mockApiClient.getExchanges.mockResolvedValue({
      exchanges: [
        { id: 'binance', name: 'Binance', status: 'online' },
        { id: 'bybit', name: 'Bybit', status: 'online' },
      ],
    });

    // Set up WebSocket server to send ticker data
    mockServer.on('connection', (socket) => {
      // Send connection confirmation
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected to crypto-dash API',
      }));

      // Listen for subscription requests
      socket.on('message', (data) => {
        const message = JSON.parse(data as string);
        
        if (message.op === 'subscribe') {
          // Send ticker data for subscribed channels
          message.channels.forEach((channel: any) => {
            if (channel.channel_type === 'ticker') {
              socket.send(JSON.stringify({
                type: 'ticker',
                payload: {
                  timestamp: new Date().toISOString(),
                  exchange: channel.exchange,
                  symbol: channel.symbol,
                  bid: 50000,
                  ask: 50001,
                  last: 50000.5,
                  bid_size: 1.0,
                  ask_size: 1.0,
                },
              }));
            }
          });

          // Send subscription confirmation
          socket.send(JSON.stringify({
            type: 'info',
            message: 'Subscribed to channels',
          }));
        }
      });
    });

    render(<HomePage />);

    // Wait for component to load
    act(() => {
      jest.advanceTimersByTime(500);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Wait for WebSocket connection and data
    act(() => {
      jest.advanceTimersByTime(1000);
    });

    await waitFor(() => {
      // Should show live data indicator when connected
      expect(screen.getByText('Live') || screen.getByText('Demo Mode')).toBeInTheDocument();
    });

    // Should display ticker data in the table
    await waitFor(() => {
      expect(screen.getByText('BTC-USDT')).toBeInTheDocument();
      expect(screen.getByText('$50,000.50')).toBeInTheDocument();
    });
  });

  test('should handle real-time updates and re-render correctly', async () => {
    mockApiClient.getExchanges.mockResolvedValue({
      exchanges: [{ id: 'binance', name: 'Binance', status: 'online' }],
    });

    let serverSocket: any;
    mockServer.on('connection', (socket) => {
      serverSocket = socket;
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected',
      }));
    });

    render(<HomePage />);

    act(() => {
      jest.advanceTimersByTime(500);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Send initial ticker data
    act(() => {
      serverSocket?.send(JSON.stringify({
        type: 'ticker',
        payload: {
          timestamp: new Date().toISOString(),
          exchange: 'binance',
          symbol: { base: 'BTC', quote: 'USDT' },
          bid: 50000,
          ask: 50001,
          last: 50000.5,
          bid_size: 1.0,
          ask_size: 1.0,
        },
      }));
    });

    await waitFor(() => {
      expect(screen.getByText('$50,000.50')).toBeInTheDocument();
    });

    // Send updated ticker data
    act(() => {
      serverSocket?.send(JSON.stringify({
        type: 'ticker',
        payload: {
          timestamp: new Date().toISOString(),
          exchange: 'binance',
          symbol: { base: 'BTC', quote: 'USDT' },
          bid: 51000,
          ask: 51001,
          last: 51000.5,
          bid_size: 1.0,
          ask_size: 1.0,
        },
      }));
    });

    // Should update to new price
    await waitFor(() => {
      expect(screen.getByText('$51,000.50')).toBeInTheDocument();
    });
  });

  test('should handle connection errors and show appropriate fallbacks', async () => {
    mockApiClient.getExchanges.mockRejectedValue(new Error('API Error'));

    // Set up server to close connection immediately
    mockServer.on('connection', (socket) => {
      socket.close();
    });

    render(<HomePage />);

    act(() => {
      jest.advanceTimersByTime(500);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Should fall back to demo mode or show error handling
    await waitFor(() => {
      expect(
        screen.getByText('Demo Mode') || 
        screen.getByText('Loading exchanges...') ||
        screen.getByText(/fallback/i)
      ).toBeInTheDocument();
    });
  });

  test('should handle subscription lifecycle correctly', async () => {
    mockApiClient.getExchanges.mockResolvedValue({
      exchanges: [{ id: 'binance', name: 'Binance', status: 'online' }],
    });

    const subscriptionMessages: any[] = [];
    mockServer.on('connection', (socket) => {
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected',
      }));

      socket.on('message', (data) => {
        const message = JSON.parse(data as string);
        subscriptionMessages.push(message);
        
        if (message.op === 'subscribe') {
          socket.send(JSON.stringify({
            type: 'info',
            message: 'Subscribed to channels',
          }));
        }
      });
    });

    const { unmount } = render(<HomePage />);

    act(() => {
      jest.advanceTimersByTime(1000);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Wait for subscription to happen
    act(() => {
      jest.advanceTimersByTime(2000);
    });

    // Should have sent subscription message
    await waitFor(() => {
      const subscribeMessage = subscriptionMessages.find(msg => msg.op === 'subscribe');
      expect(subscribeMessage).toBeDefined();
      expect(subscribeMessage?.channels).toBeDefined();
    });

    // Unmount component (should trigger unsubscribe)
    unmount();

    // Wait for unsubscribe
    act(() => {
      jest.advanceTimersByTime(500);
    });

    // Should have sent unsubscribe message
    const unsubscribeMessage = subscriptionMessages.find(msg => msg.op === 'unsubscribe');
    expect(unsubscribeMessage).toBeDefined();
  });

  test('should handle ping/pong latency measurement', async () => {
    mockApiClient.getExchanges.mockResolvedValue({
      exchanges: [{ id: 'binance', name: 'Binance', status: 'online' }],
    });

    let serverSocket: any;
    mockServer.on('connection', (socket) => {
      serverSocket = socket;
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected',
      }));

      socket.on('message', (data) => {
        const message = JSON.parse(data as string);
        if (message.op === 'ping') {
          // Respond with pong
          socket.send(JSON.stringify({
            type: 'info',
            message: 'pong',
          }));
        }
      });
    });

    render(<HomePage />);

    act(() => {
      jest.advanceTimersByTime(500);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Trigger ping interval (30 seconds)
    act(() => {
      jest.advanceTimersByTime(30000);
    });

    // Should show latency information
    await waitFor(() => {
      // Latency badge should be present and potentially show latency info
      expect(screen.getByTestId('latency-badge') || screen.getByText(/ms/)).toBeInTheDocument();
    });
  });

  test('should maintain state consistency across re-renders', async () => {
    mockApiClient.getExchanges.mockResolvedValue({
      exchanges: [
        { id: 'binance', name: 'Binance', status: 'online' },
        { id: 'bybit', name: 'Bybit', status: 'online' },
      ],
    });

    mockServer.on('connection', (socket) => {
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected',
      }));
    });

    const { rerender } = render(<HomePage />);

    act(() => {
      jest.advanceTimersByTime(500);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Get initial state
    const initialExchanges = screen.getAllByText(/binance|bybit/i);
    
    // Force re-render
    rerender(<HomePage />);

    // State should be maintained
    await waitFor(() => {
      const currentExchanges = screen.getAllByText(/binance|bybit/i);
      expect(currentExchanges.length).toBeGreaterThanOrEqual(initialExchanges.length);
    });
  });

  test('should handle multiple simultaneous data streams', async () => {
    mockApiClient.getExchanges.mockResolvedValue({
      exchanges: [{ id: 'binance', name: 'Binance', status: 'online' }],
    });

    let serverSocket: any;
    mockServer.on('connection', (socket) => {
      serverSocket = socket;
      socket.send(JSON.stringify({
        type: 'info',
        message: 'Connected',
      }));
    });

    render(<HomePage />);

    act(() => {
      jest.advanceTimersByTime(500);
    });

    await waitFor(() => {
      expect(screen.getByText('Markets Overview')).toBeInTheDocument();
    });

    // Send multiple ticker updates rapidly
    const tickers = [
      { symbol: { base: 'BTC', quote: 'USDT' }, last: 50000 },
      { symbol: { base: 'ETH', quote: 'USDT' }, last: 3000 },
      { symbol: { base: 'ADA', quote: 'USDT' }, last: 0.5 },
    ];

    tickers.forEach((ticker, index) => {
      act(() => {
        serverSocket?.send(JSON.stringify({
          type: 'ticker',
          payload: {
            timestamp: new Date().toISOString(),
            exchange: 'binance',
            symbol: ticker.symbol,
            bid: ticker.last - 0.5,
            ask: ticker.last + 0.5,
            last: ticker.last,
            bid_size: 1.0,
            ask_size: 1.0,
          },
        }));
      });
    });

    // Should handle all updates without issues
    await waitFor(() => {
      // At least one ticker should be displayed
      expect(screen.getByText(/\$[\d,]+\.\d{2}/)).toBeInTheDocument();
    });
  });
});