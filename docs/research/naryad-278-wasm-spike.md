# Naryad #278 — WASM Playground reconnaissance: dependency hygiene, core blockers, Go/No-Go

**Date:** 2026-09-12
**Dispatch:** #316, wave 4 (issue #314)
**Related:** `docs/refactoring-split-plan.md` (naryad #37 Block 2), ADR-0105 (VM experimental scope), #111 (feature gates), #271 item 4 (sqlite-vec under wasm)
**Verdict:** **No-Go** for the Playground in its current form → path to Go via dependency surgery (plan below).

## 1. What was fact-checked (main snapshot 26fd63ac, wave 4)

### 1.1. tokio hygiene — CONFIRMED by the use graph and LANDED on main

`tokio` was a hard dependency (`features = ["full"]`, not optional), although
only the following use it:

- `src/server.rs` (the module is already entirely `#[cfg(feature = "server")]`):
  `use tokio::sync::RwLock`, `tokio::spawn`, `tokio::net::TcpListener`,
  `tokio::task::spawn_blocking`, `tokio::sync::Mutex<rusqlite::Connection>`, `#[tokio::test]`;
- `src/main.rs` → `cmd_serve` (the function is already `#[cfg(feature = "server")]`,
  as is the `Commands::Serve` branch): the only `tokio::runtime::Builder`.

Outside the server stack (`grep 'tokio::' src/`): only a doc comment in
`builtins/http.rs`. Use-graph conclusion: **tokio is needed only by the `server` feature**.

Change on main (one line + a comment): `tokio` → `optional = true`,
`server = [..., "dep:tokio"]`. The core (parser/compiler/VM) and the `mlog` binary
build without it — `cargo check --bin mlog --no-default-features
--features "svg,chart,diagram,template,llm"` is green; the blocking job
`minimal-build` in CI now checks this configuration on every PR.
`mlog-lsp` is a separate crate with its own tokio, unaffected.

**Numbers:**

| Metric | Before (tokio hard) | After (tokio behind `server`) |
|---|---|---|
| Crates in the normal graph (default features) | 357 | 357 |
| Crates without server (`--no-default-features --features "svg,chart,diagram,template,llm"`) | — | **339 (−18)** |
| Removed from the graph | — | `axum`, `axum-core`, `axum-macros`, `matchit`, `http-body-util`, `serde_path_to_error`, `serde_urlencoded`, `tower-http`, `tokio-macros`, `tokio-util`, `h2`, `parking_lot(+core)`, `lock_api`, `signal-hook-registry`, `errno`, `fnv` |
| Clean debug lib build (this container, cold) | 153 s | 140 s (−13 s, ~8.5 %) |

**Honest caveat:** tokio ITSELF remained in the graph without server — it is pulled by
`reqwest` → `hyper` → `tokio` (the async client under the LLM/HTTP/voice builtins).
The hygiene removed the server stack and made the dependency explicit, but the full
removal of tokio from non-server builds is blocked by reqwest — see the blockers below. The timing
measurement is a cold debug `cargo build --lib` in the execution container (not a
GitHub-hosted runner; the baseline ~60 s from FEATURE_INTAKE §5 was measured in a different
configuration — absolute numbers must not be compared across machines, only the
delta on one machine: **−13 s / −18 crates**).

### 1.2. wasm32-unknown-unknown — actual list of core blockers

`rustup target add wasm32-unknown-unknown`; the check:

```
cargo check --lib --target wasm32-unknown-unknown --no-default-features
```

The target dependency graph under wasm (--no-default-features, core only +
hard dependencies): **303 crates**. The first hard stop — **build script
`openssl-sys v0.9.117`** (exit 101: no OpenSSL for the wasm target). The full
set of wasm-incompatible families in the target graph (via `cargo tree -i`
inversions, resolution facts + known target support):

| Family | Pulls in | Blocker reason |
|---|---|---|
| `openssl-sys` / `openssl` / `native-tls` | `imap`, `lettre`, directly metalogos | C-OpenSSL; build script fails on wasm (observed first stop) |
| `imap` (+`imap-proto`) | email builtins | native TLS stack |
| `lettre` | email builtins (SMTP) | native TLS stack |
| `reqwest` → `hyper`/`hyper-util`/`tokio`/`h2` | http/llm/voice builtins | reqwest does not support `wasm32-unknown-unknown` (only web targets via wasm-bindgen; no blocking client under wasm) |
| `rusqlite` / `libsqlite3-sys` | memory layer (memory_store, learnable cache persistence) | bundled C-sqlite does not build under wasm |
| `getrandom 0.4` | crypto crates | under `unknown-unknown` requires the wasm_js configuration; the default does not build |
| `socket2` / `libc` network primitives | tokio/hyper | no OS sockets under wasm |
| `tokio` | reqwest/hyper | partial under wasm (time/util without net); a blocker as part of the reqwest stack |

What is clean and wasm-compatible (this is the Playground core): `pest`/`pest_derive`
(parser), compiler, VM interpreter (ADR-0105 scope), ast/bytecode,
pure builtins (string/list/math/json/crypto primitives/calendar/time/encoding/svg/chart/diagram —
the latter generate strings without network).

## 2. Verdict: **No-Go** (for the Playground in its current form)

The Go criterion from the task statement — "the core builds, .wasm ≤ 5 MB gz, the demo
works". The core **does not build**: 8 blocker families on hard
dependencies, the first being `openssl-sys` (the observed build-script stop).
No .wasm size measurement was made — there is no build, nothing to measure.

## 3. Path to Go (plan of the next naryad, on top of #37 Block 2)

1. **Dependency surgery** (the main step, valuable even without wasm):
   - `imap` + `lettre` → optional behind the `email` feature (the email builtins
     are always registered; without the feature the handlers give a loud
     `FEATURE_DISABLED`-class error — convention #111);
   - `reqwest` → optional behind the `http` feature (http/llm/voice builtins);
   - `rusqlite` → optional behind the `memory` feature (an in-memory HashMap fallback
     or a loud failure — to be decided in a separate ADR);
   - `native-tls` goes away together with them; for wasm the network is excluded anyway
     (see item 2).
2. **Repeat wasm-check** after step 1: the expected remainder — `getrandom`
   (enable the `wasm_js` configuration for the crypto primitives) and trivia;
   core + pure builtins build.
3. **cdylib experiment**: a temporary `crate-type = ["cdylib"]` (in the spike,
   not on main), a `run_program` wrapper `textarea → run → output`, measurement
   of `.wasm` gz — the Go criterion is **< 5 MB gz**.
4. **Demo** on GitHub Pages (a static page, 3–5 examples from
   `examples/`, no backend) — a separate Playground-implementation naryad.

Until steps 1–3 are done, the priority is `docs/refactoring-split-plan.md`
(#37 Block 2): splitting the core structurally solves the wasm path too.
