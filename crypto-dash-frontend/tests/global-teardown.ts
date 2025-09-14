export default async function globalTeardown() {
  console.log('Tearing down global test environment...');
  
  // Clean up mock WebSocket server
  if (global.mockServer) {
    global.mockServer.stop();
  }
  
  console.log('Global test teardown complete');
}