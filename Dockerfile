FROM rust:1.78-slim AS builder

WORKDIR /app

# Dependency caching layer: copy manifests first
COPY Cargo.toml ./
# Generate lockfile for reproducible builds
RUN cargo generate-lockfile 2>/dev/null || true

# Create dummy source to pre-build dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -rf src

# Copy real source and build
COPY src/ src/
RUN touch src/main.rs && cargo build --release

# ── Runtime image ────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mlog /usr/local/bin/mlog
RUN chmod +x /usr/local/bin/mlog

WORKDIR /office

# Default: serve app.mlog. Override with: docker run mlog run /file.mlog
CMD ["mlog", "serve", "app.mlog"]
