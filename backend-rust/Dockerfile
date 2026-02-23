# Build stage
FROM rust:1.80-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies layer
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy real source code
COPY src src
COPY migrations migrations

# Touch main.rs to ensure it's recompiled
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL required by reqwest/sqlx TLS)
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder environment
COPY --from=builder /app/target/release/arbiagent-backend .

# Copy migrations (in case they are needed at runtime)
COPY migrations migrations

# Expose the application port
EXPOSE 8080

# Environment variables should be provided by Render
ENV PORT=8080

# Run the application
CMD ["./arbiagent-backend"]
