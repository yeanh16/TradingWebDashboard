# Install dependencies separately for better caching
FROM node:18-bullseye-slim AS deps
WORKDIR /app
COPY package*.json ./
RUN npm ci

# Build Next.js application
FROM node:18-bullseye-slim AS builder
WORKDIR /app
ARG NEXT_PUBLIC_API_URL
ARG NEXT_PUBLIC_AI_API_URL
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ENV NEXT_PUBLIC_AI_API_URL=${NEXT_PUBLIC_AI_API_URL}
COPY package*.json ./
COPY --from=deps /app/node_modules ./node_modules
RUN mkdir -p public
COPY . .
RUN npm run build

# Runtime image serving static export
FROM nginx:1.25-alpine AS runner
WORKDIR /usr/share/nginx/html
COPY --from=builder /app/out ./

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
