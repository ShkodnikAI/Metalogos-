FROM rust:1.85-slim AS builder

WORKDIR /app

# Install system deps (SQLite, SSL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev pkg-config && rm -rf /var/lib/apt/lists/*

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true

# Copy real source and build
COPY . .
RUN touch src/main.rs && cargo build --release

# ── Runtime image ────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-0 ca-certificates && rm -rf /var/lib/apt/lists/*

RUN groupadd -r mlog && useradd -r -g mlog -d /app mlog
WORKDIR /app

COPY --from=builder /app/target/release/mlog /usr/local/bin/
USER mlog

EXPOSE 8080
ENV METALOGOS_PORT=8080
CMD ["mlog", "serve"]