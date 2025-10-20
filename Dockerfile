# ==== Builder stage for Rust backend ====
FROM rust:1.70-slim AS backend-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

WORKDIR /app/crypto-dash-backend
RUN cargo build --release --bin api

# ==== Builder stage for frontend ====
FROM node:18-bullseye-slim AS frontend-builder

WORKDIR /app
COPY . .

ARG NEXT_PUBLIC_API_URL
ARG NEXT_PUBLIC_AI_API_URL
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ENV NEXT_PUBLIC_AI_API_URL=${NEXT_PUBLIC_AI_API_URL}

WORKDIR /app/crypto-dash-frontend
RUN npm install && npm run build

# ==== Builder stage for Python AI service ====
FROM python:3.11-slim AS ai-builder

WORKDIR /app
COPY ./crypto-dash-ai-backend ./crypto-dash-ai-backend
RUN python -m venv /venv \
    && /venv/bin/pip install --upgrade pip \
    && /venv/bin/pip install -r crypto-dash-ai-backend/requirements.txt

# ==== Final runtime stage ====
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/crypto-dash-backend/target/release/api /usr/local/bin/backend
COPY --from=frontend-builder /app/crypto-dash-frontend/out /usr/local/bin/static
COPY --from=ai-builder /venv /venv
COPY --from=ai-builder /app/crypto-dash-ai-backend /srv/crypto-dash-ai-backend

ENV PATH="/venv/bin:${PATH}"
WORKDIR /srv/crypto-dash-ai-backend

EXPOSE 8080 8000

CMD ["/bin/sh", "-c", "/usr/local/bin/backend & python -m fastapi dev main.py --host 0.0.0.0 --port 8000"]
