# ADR-0104: Cargo feature gating — measured binary impact

> **Status:** Accepted  
> **Date:** 2026-08-21  
> **Naryads:** №111 (feature flags), №111b (optional server deps), №112 (CI + docs)

## Context

Naryads №111 and №111b introduced Cargo features (`svg`, `chart`, `diagram`,
`template`, `llm`, `server`) and made `axum` / `tower` / `tower-http` / `hyper`
`optional = true` under `server`. The original expectation was a meaningful
reduction in binary size and build time when building without the HTTP server
stack.

## Decision

Document the **measured** impact rather than projected gains, and protect the
minimal configuration with a blocking CI job so regressions cannot return
silently.

## Measurement method

- Profile: `--release` only (dev sizes are not comparable)
- Isolation: `cargo clean` before each configuration
- Toolchain: rustc 1.98.0 (stable)
- Binary measured: `target/release/mlog` (byte size via `stat`)
- Configurations compared:
  - full: `cargo build --release --features full`
  - minimal: `cargo build --release --no-default-features --features "svg,chart,diagram,template"`

## Results (naryad №111b)

| Configuration | Wall time | Binary size |
|---|---:|---:|
| `--features full` | 318 s | 24 046 808 bytes (~22.94 MiB) |
| `--no-default-features --features "svg,chart,diagram,template"` | 345 s | 22 339 384 bytes (~21.31 MiB) |

| Metric | Value |
|---|---|
| Δ size | −1 707 424 bytes (~1.63 MiB, ≈ **7.1%** smaller) |
| Δ time | minimal was **+27 s** longer on a cold rebuild |

## Interpretation

The size win is **modest**. The bulk of the dependency tree (reqwest, rusqlite,
pdf crates, etc.) remains unconditional because those crates are used outside
`src/server.rs`. Optional server stack deps (`axum` and friends) account for
roughly the observed 1.6 MiB.

Cold wall-clock time for the minimal configuration was slightly **higher**, not
lower — consistent with less codegen parallelization and no reuse benefit from
server-stack artifacts on a fully cleaned tree. Feature gating is still
valuable for:

1. Correct compile-time exclusion of server code paths
2. Preventing silent breakage of `--no-default-features` builds (CI job
   `minimal-build`)
3. Future opt-in packaging where size budget matters more than cold-build time

This is **not** a large packaging win; treat claims of “much smaller binaries”
as incorrect unless further deps are proven exclusive and made optional.

## Consequences

- CI runs a blocking `minimal-build` job (naryad №112)
- Default features still enable all six flags so FOSVED release builds are unchanged
- Further size work requires auditing other “exclusive” crates the same way
  №111b audited `axum`/`tower*`/`hyper` — not by hope
