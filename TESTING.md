# Testing Guide for Crypto Trading Dashboard

This document provides comprehensive information about the testing infrastructure and how to run all types of tests for the Crypto Trading Dashboard application.

## Overview

Our testing strategy covers multiple layers and scenarios:

- **Backend Tests (Rust)**: Unit tests, integration tests, edge cases
- **Frontend Tests (TypeScript/React)**: Unit tests, component tests, integration tests
- **End-to-End Tests (Playwright)**: Browser functionality, cross-browser compatibility
- **Edge Case Testing**: Network failures, browser crashes, memory pressure
- **Performance Testing**: Load testing, concurrent users, real-time updates

## Test Structure

```
TradingWebDashboard/
├── crypto-dash-backend/
│   ├── tests/                          # Basic integration tests
│   ├── integration-tests/              # Comprehensive test suite
│   │   ├── tests/
│   │   │   ├── api_integration_tests.rs
│   │   │   ├── websocket_tests.rs
│   │   │   └── edge_case_tests.rs
│   │   └── src/common.rs              # Test utilities
│   └── crates/                        # Individual crate unit tests
│
└── crypto-dash-frontend/
    ├── tests/
    │   ├── unit/                      # Unit tests
    │   │   ├── api.test.ts
    │   │   ├── useWebSocket.test.ts
    │   │   └── TickerTable.test.tsx
    │   ├── integration/               # Integration tests
    │   │   └── full-app.test.tsx
    │   ├── e2e/                       # End-to-end tests
    │   │   ├── dashboard.spec.ts
    │   │   └── browser-edge-cases.spec.ts
    │   └── setup.ts                   # Test configuration
    ├── jest.config.js                 # Jest configuration
    └── playwright.config.ts           # Playwright configuration
```

## Running Tests

### Backend Tests (Rust)

```bash
cd crypto-dash-backend

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test package
cargo test -p crypto-dash-integration-tests

# Run tests with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Frontend Unit Tests (Jest)

```bash
cd crypto-dash-frontend

# Install dependencies
npm install

# Run all unit and integration tests
npm test

# Run tests in watch mode
npm run test:watch

# Run tests with coverage
npm run test:coverage

# Run only unit tests
npm run test:unit

# Run only integration tests
npm run test:integration
```

### End-to-End Tests (Playwright)

```bash
cd crypto-dash-frontend

# Install Playwright browsers (first time only)
npx playwright install

# Run all E2E tests
npm run test:e2e

# Run E2E tests with UI mode
npm run test:e2e:ui

# Run E2E tests on specific browser
npx playwright test --project=chromium
npx playwright test --project=firefox
npx playwright test --project=webkit
```

### All Tests

```bash
cd crypto-dash-frontend

# Run all tests (unit, integration, and E2E)
npm run test:all
```

## Test Categories

### 1. Backend Testing (Rust)

#### Unit Tests
- **Location**: Each crate's `src/` directory
- **Coverage**: Core models, data structures, business logic
- **Examples**: Symbol validation, ticker calculations, exchange adapters

#### Integration Tests
- **Location**: `crypto-dash-backend/integration-tests/`
- **Coverage**: API endpoints, WebSocket handling, service interactions

**API Integration Tests**:
- Health endpoints (`/health`, `/ready`)
- Exchange endpoints (`/api/exchanges`)
- Symbol endpoints (`/api/symbols`)
- CORS handling
- Error responses
- Rate limiting

**WebSocket Tests**:
- Connection lifecycle
- Subscription/unsubscription
- Message handling
- Error handling
- Concurrent connections

**Edge Case Tests**:
- High load scenarios
- Memory pressure
- Network failures
- Data consistency
- Resource cleanup

### 2. Frontend Testing (TypeScript/React)

#### Unit Tests
- **Framework**: Jest + React Testing Library
- **Coverage**: Hooks, utilities, API client

**WebSocket Hook Tests**:
```typescript
// tests/unit/useWebSocket.test.ts
- Connection establishment
- Message handling
- Reconnection logic
- Error handling
- Subscription management
```

**API Client Tests**:
```typescript
// tests/unit/api.test.ts
- HTTP requests
- Error handling
- Timeout scenarios
- Response parsing
```

#### Component Tests
- **Framework**: Jest + React Testing Library
- **Coverage**: React components, user interactions

**TickerTable Tests**:
```typescript
// tests/unit/TickerTable.test.tsx
- Data display
- Real-time updates
- Price formatting
- Filtering
- Empty states
```

#### Integration Tests
- **Framework**: Jest with mocked WebSocket
- **Coverage**: Full application flow

```typescript
// tests/integration/full-app.test.tsx
- WebSocket data integration
- Real-time updates
- Error handling
- State management
```

### 3. End-to-End Testing (Playwright)

#### Browser Functionality Tests
```typescript
// tests/e2e/dashboard.spec.ts
- Page loading
- Component visibility
- User interactions
- Data display
- Responsive design
```

#### Edge Case and Reliability Tests
```typescript
// tests/e2e/browser-edge-cases.spec.ts
- Tab visibility changes
- Browser refresh
- Network disconnection
- Memory pressure
- Device orientation
- Cross-browser compatibility
```

## Test Scenarios Covered

### Normal Use Cases ✅
- Application loading and initialization
- WebSocket connection establishment
- Real-time data display
- User interactions (exchange selection, ticker selection)
- Data filtering and formatting
- Responsive design across devices

### Browser Functionality ✅
- Page navigation and refresh
- Tab focus/blur events
- Browser window resize
- Mobile device orientation changes
- Browser back/forward navigation
- Local storage persistence

### Edge Cases ✅
- **Network Issues**:
  - WebSocket disconnection and reconnection
  - API failures and timeouts
  - Slow network conditions
  - Offline/online state changes

- **Browser Issues**:
  - Tab visibility changes
  - Memory pressure situations
  - JavaScript errors
  - Browser crashes and recovery

- **Data Issues**:
  - Malformed WebSocket messages
  - Invalid API responses
  - Empty data states
  - Rapid data updates

### Performance and Reliability ✅
- **Load Testing**:
  - Multiple concurrent connections
  - High-frequency data updates
  - Large data sets
  - Memory usage monitoring

- **Cross-Browser Compatibility**:
  - Chrome/Chromium
  - Firefox
  - Safari/WebKit
  - Mobile browsers

## Running Tests in CI/CD

### GitHub Actions Example

```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  backend-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run backend tests
        run: |
          cd crypto-dash-backend
          cargo test

  frontend-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - name: Install dependencies
        run: |
          cd crypto-dash-frontend
          npm install
      - name: Run unit tests
        run: |
          cd crypto-dash-frontend
          npm test
      - name: Run E2E tests
        run: |
          cd crypto-dash-frontend
          npx playwright install
          npm run test:e2e
```

## Test Configuration

### Jest Configuration
```javascript
// jest.config.js
module.exports = {
  testEnvironment: 'jsdom',
  setupFilesAfterEnv: ['<rootDir>/tests/setup.ts'],
  moduleNameMapping: {
    '^@/(.*)$': '<rootDir>/src/$1',
  },
  testMatch: [
    '<rootDir>/tests/unit/**/*.(test|spec).(js|ts|tsx)',
    '<rootDir>/tests/integration/**/*.(test|spec).(js|ts|tsx)',
  ],
  testPathIgnorePatterns: ['<rootDir>/tests/e2e/'],
}
```

### Playwright Configuration
```typescript
// playwright.config.ts
export default defineConfig({
  testDir: './tests/e2e',
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'Mobile Chrome', use: { ...devices['Pixel 5'] } },
  ],
  webServer: [
    {
      command: 'npm run dev',
      url: 'http://localhost:3000',
    },
    {
      command: 'cd ../crypto-dash-backend && cargo run -p api',
      url: 'http://localhost:8080/health',
    }
  ],
})
```

## Test Data and Mocking

### WebSocket Mocking
```typescript
// tests/setup.ts
import { Server } from 'mock-socket';

// Global WebSocket mock
global.WebSocket = require('mock-socket').WebSocket;

// Test server setup
global.mockServer = new Server('ws://localhost:8080/ws');
```

### API Mocking
```typescript
// Example API mock
jest.mock('@/lib/api', () => ({
  apiClient: {
    getExchanges: jest.fn(),
    getSymbols: jest.fn(),
    getWebSocketUrl: () => 'ws://localhost:8080/ws',
  },
}));
```

## Debugging Tests

### Backend Tests
```bash
# Run with debug output
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_websocket_connection_lifecycle

# Run with backtrace
RUST_BACKTRACE=1 cargo test
```

### Frontend Tests
```bash
# Run Jest in debug mode
node --inspect-brk node_modules/.bin/jest --runInBand

# Run Playwright in debug mode
npx playwright test --debug

# Run Playwright with headed browser
npx playwright test --headed
```

## Performance Benchmarks

Our tests ensure the application meets these performance criteria:

- **Frontend load time**: < 5 seconds
- **WebSocket connection**: < 2 seconds
- **Real-time updates**: < 100ms latency
- **Memory usage**: Stable under extended use
- **Cross-browser consistency**: 95%+ feature parity

## Contributing to Tests

When adding new features:

1. **Add unit tests** for new functions/components
2. **Update integration tests** for new API endpoints
3. **Add E2E tests** for new user workflows
4. **Test edge cases** and error conditions
5. **Verify cross-browser compatibility**

### Test Naming Conventions
- Unit tests: `[component/function].test.[ts|tsx]`
- Integration tests: `[feature].integration.test.[ts|tsx]`
- E2E tests: `[workflow].spec.ts`

### Test Structure
```typescript
describe('Component/Feature Name', () => {
  beforeEach(() => {
    // Setup
  });

  describe('normal operations', () => {
    test('should handle expected input', () => {
      // Test implementation
    });
  });

  describe('error conditions', () => {
    test('should handle invalid input', () => {
      // Error test implementation
    });
  });

  describe('edge cases', () => {
    test('should handle extreme conditions', () => {
      // Edge case implementation
    });
  });
});
```

This comprehensive testing infrastructure ensures the reliability, performance, and user experience of the Crypto Trading Dashboard across all supported browsers and usage scenarios.