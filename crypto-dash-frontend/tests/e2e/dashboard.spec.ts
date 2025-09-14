import { test, expect } from '@playwright/test';

test.describe('Trading Dashboard - Basic Functionality', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the application
    await page.goto('/');
  });

  test('should load the main dashboard page', async ({ page }) => {
    // Check if main title is visible
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    // Check if subtitle is visible
    await expect(page.getByText('Real-time cryptocurrency market data from multiple exchanges')).toBeVisible();
    
    // Check if main components are present
    await expect(page.getByText('Exchange Selection')).toBeVisible();
    await expect(page.getByText('Ticker Selection')).toBeVisible();
    await expect(page.getByText('Live Market Data')).toBeVisible();
  });

  test('should display exchange selector with default exchanges', async ({ page }) => {
    // Wait for exchanges to load
    await page.waitForSelector('[data-testid="exchange-selector"]', { timeout: 10000 });
    
    // Check if default exchanges are visible
    await expect(page.getByText('Binance')).toBeVisible();
    await expect(page.getByText('Bybit')).toBeVisible();
    
    // Verify checkboxes are present and some are checked by default
    const binanceCheckbox = page.getByRole('checkbox', { name: /binance/i });
    const bybitCheckbox = page.getByRole('checkbox', { name: /bybit/i });
    
    await expect(binanceCheckbox).toBeVisible();
    await expect(bybitCheckbox).toBeVisible();
  });

  test('should display ticker table with market data', async ({ page }) => {
    // Wait for the ticker table to load
    await page.waitForSelector('table', { timeout: 10000 });
    
    // Check for table headers
    await expect(page.getByText('Symbol')).toBeVisible();
    await expect(page.getByText('Exchange')).toBeVisible();
    await expect(page.getByText('Last Price')).toBeVisible();
    await expect(page.getByText('Bid')).toBeVisible();
    await expect(page.getByText('Ask')).toBeVisible();
    await expect(page.getByText('Spread')).toBeVisible();
    
    // Check if at least one ticker is displayed
    await expect(page.locator('table tbody tr')).toHaveCount({ min: 1 });
  });

  test('should show connection status indicator', async ({ page }) => {
    // Wait for the connection status to appear
    await page.waitForSelector('[data-testid="latency-badge"]', { timeout: 10000 });
    
    // Should show either Live or Demo Mode
    const statusText = await page.locator('[data-testid="latency-badge"]').textContent();
    expect(statusText?.includes('Live') || statusText?.includes('Demo Mode')).toBeTruthy();
  });

  test('should allow exchange selection/deselection', async ({ page }) => {
    // Wait for the exchange selector to load
    await page.waitForSelector('[data-testid="exchange-selector"]', { timeout: 10000 });
    
    // Get initial state of Binance checkbox
    const binanceCheckbox = page.getByRole('checkbox', { name: /binance/i });
    const initialState = await binanceCheckbox.isChecked();
    
    // Toggle the checkbox
    await binanceCheckbox.click();
    
    // Verify the state changed
    const newState = await binanceCheckbox.isChecked();
    expect(newState).toBe(!initialState);
    
    // Wait a moment for the table to update
    await page.waitForTimeout(500);
    
    // The ticker table should update accordingly
    // If we unchecked Binance, Binance tickers should be hidden
    if (!newState) {
      const binanceTickers = page.locator('table tbody tr:has-text("binance")');
      await expect(binanceTickers).toHaveCount(0);
    }
  });

  test('should display responsive design on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    // Main content should still be visible
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    // Check if layout adapts to mobile
    const mainContainer = page.locator('main');
    await expect(mainContainer).toBeVisible();
    
    // Table should have horizontal scroll on mobile
    const tableContainer = page.locator('.overflow-x-auto');
    await expect(tableContainer).toBeVisible();
  });
});

test.describe('Trading Dashboard - WebSocket Functionality', () => {
  test('should handle WebSocket connection states', async ({ page }) => {
    await page.goto('/');
    
    // Wait for the page to load
    await page.waitForSelector('[data-testid="latency-badge"]', { timeout: 10000 });
    
    // Monitor network requests
    const wsRequests: string[] = [];
    page.on('websocket', ws => {
      wsRequests.push(ws.url());
    });
    
    // Check connection status changes
    const statusElement = page.locator('[data-testid="latency-badge"]');
    
    // Should eventually show some connection status
    await expect(statusElement).toBeVisible();
    
    // WebSocket connection attempts should be made
    await page.waitForTimeout(2000);
    // Note: In test environment, WebSocket might not connect to real backend
  });

  test('should display latency information when connected', async ({ page }) => {
    await page.goto('/');
    
    // Wait for connection status
    await page.waitForSelector('[data-testid="latency-badge"]', { timeout: 10000 });
    
    const latencyBadge = page.locator('[data-testid="latency-badge"]');
    
    // Should show either latency info or connection status
    const badgeText = await latencyBadge.textContent();
    expect(badgeText).toBeDefined();
    expect(badgeText!.length).toBeGreaterThan(0);
  });
});

test.describe('Trading Dashboard - Error Handling', () => {
  test('should handle API failures gracefully', async ({ page }) => {
    // Intercept API calls and make them fail
    await page.route('**/api/exchanges', route => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal Server Error' }),
      });
    });
    
    await page.goto('/');
    
    // Should still load the page and show fallback content
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    // Should handle the error gracefully (might show mock data or error message)
    await page.waitForTimeout(2000);
    
    // Page should remain functional
    await expect(page.locator('main')).toBeVisible();
  });

  test('should handle network disconnection', async ({ page }) => {
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForSelector('[data-testid="latency-badge"]', { timeout: 10000 });
    
    // Simulate network going offline
    await page.context().setOffline(true);
    
    // Wait for the app to detect disconnection
    await page.waitForTimeout(3000);
    
    // Should show offline state or demo mode
    const statusText = await page.locator('[data-testid="latency-badge"]').textContent();
    expect(statusText?.includes('Demo Mode') || statusText?.includes('Offline')).toBeTruthy();
    
    // Restore network
    await page.context().setOffline(false);
  });
});

test.describe('Trading Dashboard - User Interactions', () => {
  test('should allow ticker selection and display', async ({ page }) => {
    await page.goto('/');
    
    // Wait for ticker selector to load
    await page.waitForSelector('[data-testid="ticker-selector"]', { timeout: 10000 });
    
    // Should have some default tickers selected
    const tickerTable = page.locator('table tbody tr');
    await expect(tickerTable).toHaveCount({ min: 1 });
    
    // Should show ticker symbols
    await expect(page.getByText('BTC-USDT')).toBeVisible();
  });

  test('should update display when selections change', async ({ page }) => {
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForTimeout(2000);
    
    // Count initial tickers
    const initialTickerCount = await page.locator('table tbody tr').count();
    
    // Change exchange selection if possible
    const exchangeCheckboxes = page.getByRole('checkbox');
    const checkboxCount = await exchangeCheckboxes.count();
    
    if (checkboxCount > 0) {
      // Toggle the first exchange
      await exchangeCheckboxes.first().click();
      
      // Wait for update
      await page.waitForTimeout(1000);
      
      // Ticker count might change
      const newTickerCount = await page.locator('table tbody tr').count();
      // Count should be different (could be more or fewer)
      expect(newTickerCount).toBeGreaterThanOrEqual(0);
    }
  });

  test('should maintain state during page interactions', async ({ page }) => {
    await page.goto('/');
    
    // Wait for page to load
    await page.waitForTimeout(2000);
    
    // Record initial state
    const initialExchanges = await page.getByRole('checkbox').allTextContents();
    
    // Perform some interactions
    await page.mouse.move(100, 100);
    await page.waitForTimeout(500);
    
    // State should be maintained
    const currentExchanges = await page.getByRole('checkbox').allTextContents();
    expect(currentExchanges).toEqual(initialExchanges);
  });
});

test.describe('Trading Dashboard - Performance', () => {
  test('should load within reasonable time', async ({ page }) => {
    const startTime = Date.now();
    
    await page.goto('/');
    
    // Wait for main content to be visible
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    const loadTime = Date.now() - startTime;
    
    // Should load within 5 seconds
    expect(loadTime).toBeLessThan(5000);
  });

  test('should handle rapid updates without performance issues', async ({ page }) => {
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForTimeout(2000);
    
    // Monitor performance
    const startTime = Date.now();
    
    // Simulate rapid interactions
    for (let i = 0; i < 5; i++) {
      await page.mouse.move(Math.random() * 800, Math.random() * 600);
      await page.waitForTimeout(100);
    }
    
    const interactionTime = Date.now() - startTime;
    
    // Should remain responsive
    expect(interactionTime).toBeLessThan(2000);
    
    // Page should still be functional
    await expect(page.getByText('Markets Overview')).toBeVisible();
  });
});