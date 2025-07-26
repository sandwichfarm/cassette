# Build stage
FROM rust:1.75 as builder

WORKDIR /usr/src/cassette

# Copy the entire project
COPY . .

# Build the CLI
WORKDIR /usr/src/cassette/cli
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binary from builder
COPY --from=builder /usr/src/cassette/cli/target/release/cassette /usr/local/bin/cassette

# Create a non-root user
RUN useradd -m -u 1000 cassette
USER cassette

# Expose the default relay port
EXPOSE 8080

# Default command (can be overridden by docker-compose)
CMD ["cassette", "deck", "--relay", "--bind", "0.0.0.0:8080"]