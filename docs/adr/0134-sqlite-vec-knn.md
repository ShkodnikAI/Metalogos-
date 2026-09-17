# ADR-0134: sqlite-vec as a KNN accelerator for semantic recall — spike #271 verdict: Go

**Status:** Accepted (verdict gate of dispatch #316: resolved by the implementer per the spike 2026-09-12, as the gate mechanics prescribe)
**Date:** 2026-09-12
**Naryad:** #271 (issue #307); spike report — `docs/research/naryad-271-sqlite-vec-spike.md`
**Precedent:** ADR-0104 (feature-gating with measured impact), ADR-0116 (SQLite as the carrier of memory state), FEATURE_INTAKE §4-C/§5, MEMORY_ROADMAP Phase 4

## Context

MEMORY_ROADMAP Phase 4 (L2 scenario grouping: `scenarios` / `scenario_members`, `group_scenarios()`, `recall_from_scenario()`) is designed around centroid embeddings and KNN. The current semantic recall implementation is a full scan: SELECT of all `memories` rows, decode of the `embedding BLOB` (LE f32), and the scalar `cosine_similarity` (`src/embeddings.rs`) in 4 places in `src/memory_store.rs`. At 100K records this is 112 ms per query (spike measurement, 2 vCPU Xeon) — at the boundary of interactivity and a growth brake for Phase 4.

sqlite-vec (asg017, MIT, crate `sqlite-vec 0.1.9` — the latest stable; 2.8M downloads) — the canonical SQLite accelerator for vector search: vec0 virtual table, ~100 KB of C source, static linking via cc. The #271 brief required a spike with objective Go criteria: binary delta < 2 MB, KNN 10K×384 < 50 ms, 3 OSes without manual flags. The spike code remains on the `naryad-271-sqlite-vec` branch (draft PR #334 is not merged); only this ADR and the report land in main.

## Decision

### D1. Go — sqlite-vec is adopted as the KNN engine for semantic recall

All criteria are met with margin: binary delta **0.15 MB** (probe measurement with real usage; criterion < 2 MB), KNN 10K×384 k=10 — **4.41 ms** vs 8.78 ms for the current path (criterion < 50 ms), inserts of ~81–84 thousand vectors/s, Linux/macOS/Windows build without manual flags (Linux locally + smoke; macOS/Windows — `--features portable --all-targets` jobs on the spike branch). Correctness: vec0 top-1 matches the full scalar scan at 1K/10K/100K (smoke + the benchmark's built-in verification). #272 implements `embed` / `vec_store` / `vec_search` on top of sqlite-vec.

### D2. Integration — static registration, no `load_extension`

The extension is registered as an auto-extension (`sqlite3_auto_extension` + `sqlite3_vec_init`) before the connection is opened; the rusqlite `load_extension` feature is not introduced. This rules out dynamic `.so`/`.dll` loading entirely — all vec0 code is statically linked into the binary, and there are no platform-specific extension-loading problems. Hybrid recall is preserved: BM25 stays on FTS5, vec0 replaces only the cosine half (the RRF merge is unchanged).

### D3. The `vec` feature — off-by-default, measured impact per ADR-0104

`vec = ["dep:sqlite-vec"]`, outside `default`/`full`. At the merge of #272 the feature is included in `portable` (the 0.15 MB delta allows it; cross-OS CI covers it on every run). The spike numbers are recorded in the FEATURES ledger following the ADR-0104 pattern: +0.15 MB binary, +1 dependency, ~2× on KNN.

### D4. Honest boundary: brute-force, not ANN; wasm boundary recorded

sqlite-vec 0.1.9 is a SIMD linear scan, not an ANN index; the gain is ~2× (SIMD C core + the scan inside SQLite without materializing the table in Rust). The browser path (`wasm32-unknown-unknown`) via rusqlite is impossible — there is no cc toolchain for wasm, and the build.rs of libsqlite3-sys has no branch for it (only the wasm32-wasip1 branch exists, requiring wasi-sdk); sqlite-vec itself is wasm-compatible and available to the Playground Go stack via SQLite-WASM (agreed with the #278 verdict). Revisit point: 100K+ records, or ANN/quantization appearing in upstream sqlite-vec.

## Consequences

- Positive: Phase 4 gets in-DB KNN without unloading the table; recall at 100K is 58 ms instead of 112 ms; memory stays a single SQLite file (ADR-0116 is not violated); +1 dependency (the FEATURE_INTAKE §4-C limit — 5, not exceeded).
- Negative/risks: a C dependency in the build tree (cc), as already accepted with libsqlite3-sys; the speed axis is capped by the brute-force nature of vec0 (deliberate, D4).
- Neutral: the spike code (bench + smoke) remains on the spike branch as a proof artifact; #272 moves the smoke-test contract into permanent CI when the feature is enabled.

## Finalization (naryad #272, 2026-09-12)

The Decision is implemented in the language: `embed` / `vec_store` / `vec_search` (`src/builtins/vector.rs`, category memory; see REFERENCE §4.23). Refinements fixed by the implementation:

- D2 refined: auto-extension registration is process-global and runs on the first vec builtin call; connections opened before it do not see vec0 (safe — the functions are not called through them).
- Tables are created with a **metadata column** `id` (vec0 0.1.9) — id is returned by the KNN query without a join and is `WHERE`-filterable in future naryads; the dimension is pinned in the service table `vec_meta` on the first write and re-checked on subsequent ones (loud error reporting both numbers — protection against model mixing, a #272 requirement).
- `embed` reuses the process-global SSOT `EmbeddingManager` instance (ADR-0040): TF-IDF by default (`dim = max(vocab, 256)`, deterministic for the same call sequence), OpenAI text-embedding-3-small (dim 1536) via env. dim/IDF drift from corpus growth is a documented boundary; the tables' dim gate is the guard.
- Cross-OS: `vec` is included in `portable` (D3) — macos/windows-check compile the C code on every run; the #272 test loop (roundtrip, KNN ordering, empty table, dim gate, sandbox, k limits, TW/VM crosscheck) is in blocking CI with the `vec` feature.
