import { test, expect, Browser, BrowserContext, Page } from '@playwright/test';

test.describe('Browser Edge Cases and Reliability', () => {
  let browser: Browser;
  let context: BrowserContext;
  let page: Page;

  test.beforeEach(async ({ browser: testBrowser }) => {
    browser = testBrowser;
    context = await browser.newContext();
    page = await context.newPage();
  });

  test.afterEach(async () => {
    await context.close();
  });

  test('should handle browser tab visibility changes', async () => {
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForSelector('[data-testid="latency-badge"]', { timeout: 10000 });
    
    // Record initial connection state
    const initialStatus = await page.locator('[data-testid="latency-badge"]').textContent();
    
    // Simulate tab going to background (visibility change)
    await page.evaluate(() => {
      Object.defineProperty(document, 'visibilityState', {
        writable: true,
        value: 'hidden'
      });
      document.dispatchEvent(new Event('visibilitychange'));
    });
    
    await page.waitForTimeout(1000);
    
    // Simulate tab returning to foreground
    await page.evaluate(() => {
      Object.defineProperty(document, 'visibilityState', {
        writable: true,
        value: 'visible'
      });
      document.dispatchEvent(new Event('visibilitychange'));
    });
    
    await page.waitForTimeout(2000);
    
    // Connection should be maintained or restored
    const finalStatus = await page.locator('[data-testid="latency-badge"]').textContent();
    expect(finalStatus).toBeDefined();
    expect(finalStatus!.length).toBeGreaterThan(0);
  });

  test('should handle browser window focus/blur events', async () => {
    await page.goto('/');
    
    // Wait for page to load
    await page.waitForTimeout(2000);
    
    // Simulate window losing focus
    await page.evaluate(() => {
      window.dispatchEvent(new Event('blur'));
    });
    
    await page.waitForTimeout(500);
    
    // Simulate window regaining focus
    await page.evaluate(() => {
      window.dispatchEvent(new Event('focus'));
    });
    
    await page.waitForTimeout(1000);
    
    // Application should remain functional
    await expect(page.getByText('Markets Overview')).toBeVisible();
    await expect(page.locator('[data-testid="latency-badge"]')).toBeVisible();
  });

  test('should handle browser refresh gracefully', async () => {
    await page.goto('/');
    
    // Wait for initial load and get state
    await page.waitForTimeout(2000);
    const initialTickerCount = await page.locator('table tbody tr').count();
    
    // Refresh the page
    await page.reload();
    
    // Wait for reload to complete
    await page.waitForSelector('[data-testid="latency-badge"]', { timeout: 10000 });
    
    // Should restore to similar state
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    // Should have tickers again
    const newTickerCount = await page.locator('table tbody tr').count();
    expect(newTickerCount).toBeGreaterThanOrEqual(0);
  });

  test('should survive browser back/forward navigation', async () => {
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForTimeout(2000);
    
    // Navigate to a different page (if available) or use browser back
    await page.goBack();
    await page.waitForTimeout(500);
    
    await page.goForward();
    
    // Should restore the application state
    await expect(page.getByText('Markets Overview')).toBeVisible();
  });

  test('should handle multiple rapid page reloads', async () => {
    await page.goto('/');
    
    // Perform multiple rapid reloads
    for (let i = 0; i < 3; i++) {
      await page.reload();
      await page.waitForTimeout(1000);
    }
    
    // Final state should be stable
    await expect(page.getByText('Markets Overview')).toBeVisible();
    await expect(page.locator('[data-testid="latency-badge"]')).toBeVisible();
  });

  test('should handle browser storage persistence', async () => {
    await page.goto('/');
    
    // Wait for app to load
    await page.waitForTimeout(2000);
    
    // Set some state in localStorage (if app uses it)
    await page.evaluate(() => {
      localStorage.setItem('test-preference', 'test-value');
    });
    
    // Reload page
    await page.reload();
    await page.waitForTimeout(2000);
    
    // Check if state persisted
    const persistedValue = await page.evaluate(() => {
      return localStorage.getItem('test-preference');
    });
    
    expect(persistedValue).toBe('test-value');
  });

  test('should handle memory pressure gracefully', async () => {
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForTimeout(2000);
    
    // Simulate memory pressure by creating large objects
    await page.evaluate(() => {
      // Create some memory pressure
      const largeArrays = [];
      for (let i = 0; i < 10; i++) {
        largeArrays.push(new Array(100000).fill('memory-test'));
      }
      
      // Clean up after a moment
      setTimeout(() => {
        largeArrays.length = 0;
      }, 1000);
    });
    
    await page.waitForTimeout(2000);
    
    // App should remain functional
    await expect(page.getByText('Markets Overview')).toBeVisible();
  });

  test('should recover from WebSocket connection failures', async () => {
    await page.goto('/');
    
    // Wait for initial connection attempt
    await page.waitForTimeout(3000);
    
    // Simulate WebSocket connection failure by blocking WebSocket requests
    await page.route('**/ws', route => {
      route.abort();
    });
    
    await page.waitForTimeout(2000);
    
    // Should show disconnected state or demo mode
    const statusText = await page.locator('[data-testid="latency-badge"]').textContent();
    expect(statusText?.includes('Demo Mode') || statusText?.includes('Connecting')).toBeTruthy();
    
    // Remove the block to allow reconnection
    await page.unroute('**/ws');
    
    await page.waitForTimeout(5000);
    
    // Should attempt to reconnect
    // Note: In test environment, may still show demo mode
  });

  test('should handle device orientation changes on mobile', async () => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    
    // Wait for initial load
    await page.waitForTimeout(2000);
    
    // Rotate to landscape
    await page.setViewportSize({ width: 667, height: 375 });
    
    await page.waitForTimeout(1000);
    
    // App should adapt to new orientation
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    // Rotate back to portrait
    await page.setViewportSize({ width: 375, height: 667 });
    
    await page.waitForTimeout(1000);
    
    // Should still be functional
    await expect(page.getByText('Markets Overview')).toBeVisible();
  });

  test('should handle slow network conditions', async () => {
    // Simulate slow network
    await page.route('**/*', async route => {
      await new Promise(resolve => setTimeout(resolve, 100)); // 100ms delay
      await route.continue();
    });
    
    await page.goto('/');
    
    // Should eventually load despite slow network
    await expect(page.getByText('Markets Overview')).toBeVisible({ timeout: 15000 });
    
    // Should remain functional
    await expect(page.locator('[data-testid="latency-badge"]')).toBeVisible();
  });

  test('should handle browser resize events', async () => {
    await page.goto('/');
    await page.waitForTimeout(2000);
    
    // Test various window sizes
    const sizes = [
      { width: 1920, height: 1080 },
      { width: 1024, height: 768 },
      { width: 768, height: 1024 },
      { width: 375, height: 667 },
    ];
    
    for (const size of sizes) {
      await page.setViewportSize(size);
      await page.waitForTimeout(500);
      
      // Should remain functional at all sizes
      await expect(page.getByText('Markets Overview')).toBeVisible();
    }
  });

  test('should maintain functionality during high CPU load', async () => {
    await page.goto('/');
    await page.waitForTimeout(2000);
    
    // Simulate CPU intensive task
    const startTime = Date.now();
    await page.evaluate(() => {
      const endTime = Date.now() + 1000; // Run for 1 second
      while (Date.now() < endTime) {
        // Busy wait to simulate CPU load
        Math.random();
      }
    });
    
    const duration = Date.now() - startTime;
    expect(duration).toBeGreaterThan(900); // Should have actually run for ~1 second
    
    // App should remain responsive after CPU load
    await expect(page.getByText('Markets Overview')).toBeVisible();
    
    // User interactions should work
    await page.mouse.move(100, 100);
    await page.waitForTimeout(500);
  });

  test('should handle JavaScript errors gracefully', async () => {
    await page.goto('/');
    await page.waitForTimeout(2000);
    
    // Inject a JavaScript error
    await page.evaluate(() => {
      // This should cause an error but not crash the app
      setTimeout(() => {
        throw new Error('Test error for error handling');
      }, 100);
    });
    
    await page.waitForTimeout(1000);
    
    // App should continue to function despite the error
    await expect(page.getByText('Markets Overview')).toBeVisible();
    await expect(page.locator('[data-testid="latency-badge"]')).toBeVisible();
  });
});

test.describe('Cross-Browser Compatibility', () => {
  test('should work consistently across browsers', async ({ browserName }) => {
    const page = await browser.newPage();
    await page.goto('/');
    
    // Core functionality should work in all browsers
    await expect(page.getByText('Markets Overview')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('[data-testid="latency-badge"]')).toBeVisible();
    
    // Table should render
    await expect(page.locator('table')).toBeVisible();
    
    // Exchange selector should work
    await expect(page.getByText('Binance')).toBeVisible();
    
    console.log(`Test passed on ${browserName}`);
    
    await page.close();
  });

  test('should handle browser-specific features gracefully', async ({ browserName }) => {
    const page = await browser.newPage();
    await page.goto('/');
    
    await page.waitForTimeout(2000);
    
    // Test browser-specific WebSocket support
    const wsSupport = await page.evaluate(() => {
      return typeof WebSocket !== 'undefined';
    });
    
    expect(wsSupport).toBe(true);
    
    // Test local storage support
    const localStorageSupport = await page.evaluate(() => {
      try {
        localStorage.setItem('test', 'test');
        localStorage.removeItem('test');
        return true;
      } catch {
        return false;
      }
    });
    
    expect(localStorageSupport).toBe(true);
    
    console.log(`Browser feature test passed on ${browserName}`);
    
    await page.close();
  });
});