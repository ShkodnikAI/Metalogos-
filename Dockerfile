# ── Metalogos container image (№511, the audit 28.09 C-05 fix) ──────
#
# The container that ACTUALLY runs: ENTRYPOINT carries the mandatory
# `serve <file>` argument shape, the example program ships INSIDE the
# image, the liveness probe needs no curl (the `mlog health` subcommand
# talks to the built-in /health route), and the bind host comes from
# METALOGOS_HOST at deploy time (the program's own `host:` declaration,
# when present, always wins — №164 loopback default unchanged).
#
# Base images are pinned to versioned tags; the digest pin lands in the
# CI docker job's first successful run (docker.yml) and is recorded here.

# ── Builder ──────────────────────────────────────────
# №528: the builder image tracks the build contract's MSRV (rust-version =
# 1.93.1 in Cargo.toml) — a rust:1.85 builder now refuses to parse the
# manifest ("rustc 1.85.1 is not supported by the following packages"),
# which is the gate working as designed; the image follows the floor.
FROM rust:1.93-slim-bookworm AS builder

WORKDIR /app

# Install system deps (SQLite, SSL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests for dependency layer caching
COPY Cargo.toml Cargo.lock ./
COPY mlogpkg/Cargo.toml mlogpkg/
COPY mlog-lsp/Cargo.toml mlog-lsp/

# Create stub sources so dependency layer compiles.
# src/lib.rs stub: mlogpkg and mlog-lsp depend on the metalogos lib target.
# benches/ stub: [[bench]] in Cargo.toml requires the file for manifest parsing.
RUN mkdir -p src mlogpkg/src mlog-lsp/src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    echo "fn main() {}" > mlogpkg/src/main.rs && \
    echo "" > mlog-lsp/src/main.rs && \
    echo "" > benches/core_benchmarks.rs && \
    echo "" > benches/stage4_benchmark.rs && \
    cargo build --release --bin mlog

# Copy real source and rebuild (only application code changes)
COPY . .
RUN touch src/lib.rs src/main.rs mlogpkg/src/main.rs mlog-lsp/src/main.rs && \
    cargo build --release --bin mlog

# ── Runtime image ────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-0 ca-certificates && rm -rf /var/lib/apt/lists/*

RUN groupadd -r mlog && useradd -r -g mlog -d /app mlog
WORKDIR /app

COPY --from=builder /app/target/release/mlog /usr/local/bin/
# №511: the example program ships in the image — `serve` needs a real
# .mlog file (the former `CMD ["mlog", "serve"]` died at clap: `file` is
# NOT optional). Mounting a volume over /app/main.mlog is the documented
# way to run YOUR program without rebuilding.
COPY examples/docker_hello.mlog /app/main.mlog
USER mlog

EXPOSE 8080
# №511: METALOGOS_PORT was DEAD (nothing in src/ reads it — the port
# comes from the program's `mlogserver { port: ... }` declaration); the
# dead env is removed. The deploy-time bind host is METALOGOS_HOST —
# read ONLY when the program declares no `host:` (the declaration wins).
ENV METALOGOS_HOST=0.0.0.0

# №511: the liveness probe without curl (bookworm-slim has no curl —
# the audit's HEALTHCHECK suggestion would have left the container
# permanently unhealthy). `mlog health` GETs the built-in /health route
# and exits 0/1.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["mlog", "health"]

# `serve` takes the program path — the file ships in the image (or a
# volume mounts over it).
ENTRYPOINT ["mlog", "serve"]
CMD ["/app/main.mlog"]
