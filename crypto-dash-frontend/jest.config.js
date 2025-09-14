const nextJest = require('next/jest')

const createJestConfig = nextJest({
  // Provide the path to your Next.js app to load next.config.js and .env files
  dir: './',
})

// Add any custom config to be passed to Jest
const customJestConfig = {
  setupFilesAfterEnv: ['<rootDir>/tests/setup.ts'],
  testEnvironment: 'jest-environment-jsdom',
  testMatch: [
    '<rootDir>/tests/unit/**/*.(test|spec).(js|ts|tsx)',
    '<rootDir>/tests/integration/**/*.(test|spec).(js|ts|tsx)',
    '<rootDir>/src/**/__tests__/**/*.(test|spec).(js|ts|tsx)'
  ],
  testPathIgnorePatterns: [
    '<rootDir>/tests/e2e/',
    '<rootDir>/.next/',
    '<rootDir>/node_modules/'
  ],
  collectCoverageFrom: [
    'src/**/*.{js,jsx,ts,tsx}',
    '!src/**/*.d.ts',
    '!src/**/types.ts',
    '!src/app/layout.tsx',
    '!src/styles/**',
  ],
  coverageReporters: ['text', 'lcov', 'html'],
  coverageDirectory: 'coverage',
  moduleNameMapping: {
    '^@/(.*)$': '<rootDir>/src/$1',
  },
  testTimeout: 10000,
  globalSetup: '<rootDir>/tests/global-setup.ts',
  globalTeardown: '<rootDir>/tests/global-teardown.ts',
}

// createJestConfig is exported this way to ensure that next/jest can load the Next.js config which is async
module.exports = createJestConfig(customJestConfig)