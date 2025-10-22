# ==== Builder stage ====
FROM rust:1.70-slim-bullseye AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release --bin api

# ==== Runtime stage ====
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates libssl1.1 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /usr/local/bin/backend

EXPOSE 8080

CMD ["/usr/local/bin/backend"]
