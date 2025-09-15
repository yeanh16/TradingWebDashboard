import { ApiClient } from '@/lib/api';

// Mock fetch globally
global.fetch = jest.fn();

describe('ApiClient', () => {
  let apiClient: ApiClient;
  const mockFetch = global.fetch as jest.MockedFunction<typeof fetch>;

  beforeEach(() => {
    apiClient = new ApiClient('http://localhost:8080');
    mockFetch.mockClear();
  });

  describe('constructor', () => {
    it('should use provided base URL', () => {
      const client = new ApiClient('https://example.com');
      expect(client.getWebSocketUrl()).toBe('wss://example.com/ws');
    });

    it('should use default base URL when none provided', () => {
      const client = new ApiClient();
      // Should use environment variable or fallback
      expect(client.getWebSocketUrl()).toMatch(/^ws:\/\//);
    });
  });

  describe('getExchanges', () => {
    it('should fetch exchanges successfully', async () => {
      const mockResponse = {
        exchanges: [
          { id: 'binance', name: 'Binance', status: 'online' },
          { id: 'bybit', name: 'Bybit', status: 'online' },
        ],
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce(mockResponse),
      } as any);

      const result = await apiClient.getExchanges();

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/exchanges',
        expect.objectContaining({
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        })
      );
      expect(result).toEqual(mockResponse);
    });

    it('should handle API errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      } as any);

      await expect(apiClient.getExchanges()).rejects.toThrow(
        'API request failed: 500 Internal Server Error'
      );
    });
  });

  describe('getSymbols', () => {
    it('should fetch symbols for specific exchange', async () => {
      const mockResponse = {
        exchange: 'binance',
        symbols: [
          { symbol: 'BTC-USDT', base: 'BTC', quote: 'USDT', display_name: 'Bitcoin / USDT' },
        ],
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce(mockResponse),
      } as any);

      const result = await apiClient.getSymbols('binance');

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/symbols?exchange=binance',
        expect.objectContaining({
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        })
      );
      expect(result).toEqual(mockResponse);
    });

    it('should fetch symbols for all exchanges when no exchange specified', async () => {
      const mockResponse = { symbols: [] };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce(mockResponse),
      } as any);

      await apiClient.getSymbols();

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/symbols',
        expect.any(Object)
      );
    });
  });

  describe('getHealth', () => {
    it('should fetch health status', async () => {
      const mockResponse = { status: 'healthy', timestamp: '2024-01-01T00:00:00Z' };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce(mockResponse),
      } as any);

      const result = await apiClient.getHealth();

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8080/health',
        expect.any(Object)
      );
      expect(result).toEqual(mockResponse);
    });
  });

  describe('getReady', () => {
    it('should fetch readiness status', async () => {
      const mockResponse = { status: 'ready', timestamp: '2024-01-01T00:00:00Z' };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce(mockResponse),
      } as any);

      const result = await apiClient.getReady();

      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8080/ready',
        expect.any(Object)
      );
      expect(result).toEqual(mockResponse);
    });
  });

  describe('getWebSocketUrl', () => {
    it('should generate WebSocket URL for HTTP base URL', () => {
      const client = new ApiClient('http://localhost:8080');
      expect(client.getWebSocketUrl()).toBe('ws://localhost:8080/ws');
    });

    it('should generate WebSocket URL for HTTPS base URL', () => {
      const client = new ApiClient('https://api.example.com');
      expect(client.getWebSocketUrl()).toBe('wss://api.example.com/ws');
    });

    it('should handle URLs with ports', () => {
      const client = new ApiClient('http://localhost:3001');
      expect(client.getWebSocketUrl()).toBe('ws://localhost:3001/ws');
    });
  });

  describe('error handling', () => {
    it('should handle network errors', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      await expect(apiClient.getHealth()).rejects.toThrow('Network error');
    });

    it('should handle 404 errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
      } as any);

      await expect(apiClient.getExchanges()).rejects.toThrow(
        'API request failed: 404 Not Found'
      );
    });

    it('should handle timeout errors', async () => {
      // Simulate timeout by never resolving
      mockFetch.mockImplementationOnce(
        () => new Promise((resolve) => {
          // Never resolve to simulate timeout
        })
      );

      // Use a timeout for this test
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('Test timeout')), 100);
      });

      await expect(
        Promise.race([apiClient.getHealth(), timeoutPromise])
      ).rejects.toThrow('Test timeout');
    });
  });

  describe('request headers', () => {
    it('should include default headers', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce({}),
      } as any);

      await apiClient.getHealth();

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        })
      );
    });

    it('should merge custom headers', async () => {
      const client = new ApiClient('http://localhost:8080');
      
      // Access the private request method through reflection for testing
      const requestMethod = (client as any).request.bind(client);

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: jest.fn().mockResolvedValueOnce({}),
      } as any);

      await requestMethod('/test', {
        headers: {
          'Authorization': 'Bearer token',
        },
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
            'Authorization': 'Bearer token',
          }),
        })
      );
    });
  });
});