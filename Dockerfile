FROM rust:1.85-slim AS builder

WORKDIR /app

# Install system deps (SQLite, SSL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev pkg-config && rm -rf /var/lib/apt/lists/*

# Copy manifests for dependency layer caching
COPY Cargo.toml Cargo.lock ./
COPY mlogpkg/Cargo.toml mlogpkg/
COPY mlog-lsp/Cargo.toml mlog-lsp/

# Create stub sources so dependency layer compiles
RUN mkdir -p src mlogpkg/src mlog-lsp/src && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > mlogpkg/src/main.rs && \
    echo "fn main() {}" > mlog-lsp/src/main.rs && \
    cargo build --release 2>/dev/null || true

# Copy real source and rebuild (only application code changes)
COPY . .
RUN touch src/main.rs mlogpkg/src/main.rs mlog-lsp/src/main.rs && \
    cargo build --release

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