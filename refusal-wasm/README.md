# refusal-wasm — the WASM contour of the Portable Refusal Algebra

№855 (issue #1680, wave 74 — W-C; ADR-0204 §3/§4). The facade compiles
the DEPENDENCY-FREE algebra source of the root crate
(`../src/refusal_algebra.rs`, included verbatim via `#[path]` — one
SSOT source, two compile targets) for `wasm32-unknown-unknown` and adds
only the JSON/C-ABI consumption boundary. No pull of the language
dependency tree — the facade is a leaf over `serde`/`serde_json` only.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo build -p refusal-wasm --target wasm32-unknown-unknown --release
# → target/wasm32-unknown-unknown/release/refusal_wasm.wasm
```

## Consume (host, no Rust toolchain)

```bash
node refusal-wasm/smoke.mjs target/wasm32-unknown-unknown/release/refusal_wasm.wasm
```

The C-ABI contract (`rw_pool_len` / `rw_alloc` / `rw_eval` / `rw_reset`)
and the JSON contract (`{"op":"join"|"meet"|"verdict","reasons":[...]}` →
`{"verdict":"allow"}` / `{"verdict":"deny","code":...,"cause":...}` /
`{"verdict":"error","detail":...}`) are documented in
[`src/lib.rs`](src/lib.rs) — the crate doc is the normative seam text;
the full spec page lands with №858 (`docs/spec/refusal-algebra.md`).

The reusable GitHub Action (`policy-check`, №856) consumes this facade
through the same boundary — a repository runs refusal policies through
the algebra in its own CI.

## Honest limits (ADR-0204 §6)

- The facade computes verdicts over the causes it is GIVEN — no IO, no
  network, no clock; the host decides what to do with a verdict (the
  Ledger stays a host-side carrier).
- The algebra is CLOSED over the frozen 14-word vocabulary (№392) and
  the frozen codes (№385/ADR-0169) — an unknown word is a loud `error`
  verdict, never a silent pass and never a new code.
- Publishing the artifact (npm/crate) is the owner's act — outside the
  naryad.
