# Use the official Rust image as the base
FROM rust:1.70-slim AS builder

# Install required dependencies for building (OpenSSL, pkg-config, Node.js)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the frontend
WORKDIR /app/crypto-dash-frontend
RUN npm install
RUN npm run build

# Change to the backend directory
WORKDIR /app/crypto-dash-backend

# Build the application in release mode (specify the binary)
RUN cargo build --release --bin api

# Use a smaller runtime image
FROM debian:bullseye-slim

# Install runtime dependencies (if needed, e.g., for SSL)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the built binary from the builder stage
COPY --from=builder /app/crypto-dash-backend/target/release/api /usr/local/bin/app

# Copy the built frontend
COPY --from=builder /app/crypto-dash-frontend/out /usr/local/bin/static

# Expose the port your app runs on (e.g., 8080)
EXPOSE 8080

# Run the application
CMD ["./usr/local/bin/app"]

# To build and run the Docker container:
# docker build -t trading-dashboard .
# docker run -p 8080:8080 trading-dashboard