import { Server } from 'mock-socket';

// Extend the global object to include mockServer
declare global {
  var mockServer: Server;
}

export default async function globalSetup() {
  // Set up any global test resources
  console.log('Setting up global test environment...');
  
  // Start mock WebSocket server for testing
  global.mockServer = new Server('ws://localhost:8080/ws');
  
  // Set environment variables for testing
  process.env.NEXT_PUBLIC_API_URL = 'http://localhost:8080';
  (process.env as any).NODE_ENV = 'test';
  
  console.log('Global test setup complete');
}