#!/bin/bash

# Development script for crypto-dash-backend
# Usage: ./scripts/dev.sh

set -e

echo "🚀 Starting Crypto Trading Dashboard Backend..."

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | xargs)
    echo "✅ Environment variables loaded from .env"
else
    echo "⚠️  No .env file found, using defaults"
fi

# Build and run
echo "🔨 Building and starting the API server..."
RUST_LOG=${RUST_LOG:-"info,crypto_dash=debug"} cargo run -p api