# Multi-stage build for key-server
FROM rust:1.82 as builder

# Set the working directory
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build with release optimizations and limited parallelism to avoid memory issues
ENV CARGO_BUILD_JOBS=2
RUN cargo build --release --bin key-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false keyserver

# Copy the binary from builder stage  
COPY --from=builder /app/target/release/key-server /usr/local/bin/key-server

# Set ownership
RUN chown keyserver:keyserver /usr/local/bin/key-server

# Switch to non-root user
USER keyserver

# Expose port
EXPOSE 8080

# Run the application
CMD ["key-server"] 