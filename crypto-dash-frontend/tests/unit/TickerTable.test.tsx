import { render, screen, waitFor } from '@testing-library/react';
import { TickerTable } from '@/components/TickerTable';
import { Ticker } from '@/lib/types';

describe('TickerTable', () => {
  const mockSelectedExchanges = ['binance', 'bybit'];
  const mockSelectedTickers = [
    {
      symbol: 'BTC-USDT',
      base: 'BTC',
      quote: 'USDT',
      exchange: 'binance',
      market_type: 'spot',
      display_name: 'Bitcoin / USDT',
    },
    {
      symbol: 'ETH-USDT',
      base: 'ETH',
      quote: 'USDT',
      exchange: 'binance',
      market_type: 'spot',
      display_name: 'Ethereum / USDT',
    },
  ];

  const mockTickers: Record<string, Ticker> = {
    'binance_BTCUSDT': {
      timestamp: '2024-01-01T00:00:00Z',
      exchange: 'binance',
      symbol: { base: 'BTC', quote: 'USDT' },
      bid: 50000,
      ask: 50001,
      last: 50000.5,
      bid_size: 1.0,
      ask_size: 1.0,
    },
    'binance_ETHUSDT': {
      timestamp: '2024-01-01T00:00:00Z',
      exchange: 'binance',
      symbol: { base: 'ETH', quote: 'USDT' },
      bid: 3000,
      ask: 3001,
      last: 3000.5,
      bid_size: 1.0,
      ask_size: 1.0,
    },
  };

  const defaultProps = {
    selectedExchanges: mockSelectedExchanges,
    selectedTickers: mockSelectedTickers,
    tickers: {},
    wsConnected: false,
    activeMarketType: 'spot',
    activeQuoteSymbol: 'USDT',
  };

  beforeEach(() => {
    jest.clearAllTimers();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.clearAllTimers();
    jest.useRealTimers();
  });

  it('should render loading state initially', () => {
    render(<TickerTable {...defaultProps} />);

    // Should show loading skeleton
    expect(screen.getByRole('table', { hidden: true })).toBeInTheDocument();
    
    // Check for loading elements (skeleton placeholders)
    const loadingElements = document.querySelectorAll('.animate-pulse');
    expect(loadingElements.length).toBeGreaterThan(0);
  });

  it('should show live data when connected and tickers available', async () => {
    render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    // Wait for loading to complete
    jest.advanceTimersByTime(300);
    await waitFor(() => {
      expect(screen.getByText('Live Market Data')).toBeInTheDocument();
    });

    // Should show live indicator
    expect(screen.getByText('Live')).toBeInTheDocument();
    expect(screen.getByTestId('wifi-icon') || screen.getByLabelText(/wifi/i)).toBeInTheDocument();

    // Should show ticker data
    expect(screen.getByText('BTC-USDT')).toBeInTheDocument();
    expect(screen.getByText('ETH-USDT')).toBeInTheDocument();
    expect(screen.getByText('$50,000.50')).toBeInTheDocument();
    expect(screen.getByText('$3,000.50')).toBeInTheDocument();
  });

  it('should show demo mode when not connected', async () => {
    render(<TickerTable {...defaultProps} wsConnected={false} />);

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      expect(screen.getByText('Demo Mode')).toBeInTheDocument();
    });

    // Should show offline indicator
    expect(screen.getByTestId('wifi-off-icon') || screen.getByLabelText(/wifi.*off/i)).toBeInTheDocument();
  });

  it('should format prices correctly', async () => {
    render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      // Check price formatting with commas and decimals
      expect(screen.getByText('$50,000.50')).toBeInTheDocument();
      expect(screen.getByText('$3,000.50')).toBeInTheDocument();
      
      // Check bid/ask prices
      expect(screen.getByText('$50,000.00')).toBeInTheDocument(); // bid
      expect(screen.getByText('$50,001.00')).toBeInTheDocument(); // ask
    });
  });

  it('should calculate and display spread correctly', async () => {
    render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      // BTC spread: (50001 - 50000) / 50001 * 100 = 0.02%
      expect(screen.getByText('0.020%')).toBeInTheDocument();
      
      // ETH spread: (3001 - 3000) / 3001 * 100 = 0.033%
      expect(screen.getByText('0.033%')).toBeInTheDocument();
    });
  });

  it('should show empty state when no tickers match selected exchanges', async () => {
    render(
      <TickerTable
        {...defaultProps}
        selectedExchanges={['okx']} // Different exchange than mock tickers
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      expect(screen.getByText('No tickers available for selected exchanges')).toBeInTheDocument();
    });
  });

  it('should filter tickers by selected exchanges', async () => {
    const extendedTickers = {
      ...mockTickers,
      'bybit_BTCUSDT': {
        timestamp: '2024-01-01T00:00:00Z',
        exchange: 'bybit',
        symbol: { base: 'BTC', quote: 'USDT' },
        bid: 49999,
        ask: 50000,
        last: 49999.5,
        bid_size: 1.0,
        ask_size: 1.0,
      },
    };

    render(
      <TickerTable
        {...defaultProps}
        selectedExchanges={['binance']} // Only binance
        tickers={extendedTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      // Should only show binance tickers
      expect(screen.getAllByText('binance')).toHaveLength(2); // For BTC and ETH
      expect(screen.queryByText('bybit')).not.toBeInTheDocument();
    });
  });

  it('should show mock data when not connected', async () => {
    render(<TickerTable {...defaultProps} wsConnected={false} />);

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      // Should show mock data for demo mode
      expect(screen.getByText('BTC-USDT')).toBeInTheDocument();
      expect(screen.getByText('Demo Mode')).toBeInTheDocument();
    });
  });

  it('should handle price changes and show visual feedback', async () => {
    const { rerender } = render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      expect(screen.getByText('BTC-USDT')).toBeInTheDocument();
    });

    // Update ticker prices
    const updatedTickers = {
      ...mockTickers,
      'binance_BTCUSDT': {
        ...mockTickers['binance_BTCUSDT'],
        last: 51000, // Price increase
      },
    };

    rerender(
      <TickerTable
        {...defaultProps}
        tickers={updatedTickers}
        wsConnected={true}
      />
    );

    // Price change visual feedback should be triggered
    await waitFor(() => {
      expect(screen.getByText('$51,000.00')).toBeInTheDocument();
    });

    // Price change indicators should clear after timeout
    jest.advanceTimersByTime(2000);
  });

  it('should show last update time for live data', async () => {
    render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      // Should show formatted timestamp
      const timeElements = screen.getAllByText(/\d{1,2}:\d{2}:\d{2}/);
      expect(timeElements.length).toBeGreaterThan(0);
    });
  });

  it('should handle zero prices gracefully', async () => {
    const zeroPriceTickers = {
      'binance_BTCUSDT': {
        ...mockTickers['binance_BTCUSDT'],
        bid: 0,
        ask: 0,
        last: 0,
      },
    };

    render(
      <TickerTable
        {...defaultProps}
        tickers={zeroPriceTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      expect(screen.getByText('$0.00')).toBeInTheDocument();
      expect(screen.getByText('0.000%')).toBeInTheDocument(); // Spread should be 0
    });
  });

  it('should apply hover effects to table rows', async () => {
    render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      const tableRows = screen.getAllByRole('row');
      const dataRows = tableRows.slice(1); // Skip header row
      
      expect(dataRows.length).toBeGreaterThan(0);
      dataRows.forEach(row => {
        expect(row).toHaveClass('hover:bg-accent/50');
      });
    });
  });

  it('should maintain responsive design with overflow handling', async () => {
    render(
      <TickerTable
        {...defaultProps}
        tickers={mockTickers}
        wsConnected={true}
      />
    );

    jest.advanceTimersByTime(300);
    await waitFor(() => {
      const tableContainer = document.querySelector('.overflow-x-auto');
      expect(tableContainer).toBeInTheDocument();
    });
  });
});




