# Naryad #271 — sqlite-vec spike: bindings, size, platforms, benchmark, Go/No-Go

> **Status:** verdict **GO** (verdict gate of dispatch #316: decided by the spike performer, ADR post-hoc → ADR-0134)
> **Date:** 2026-09-12 · **Assignment:** issue #307 · **Base:** main `2afa719`
> **Spike branch:** `naryad-271-sqlite-vec` (`4bc40de`, draft PR #334 — never merged; only this document and ADR-0134 land in main)
> **Environment:** cargo/rustc 1.98.1, Linux x86_64, 2 vCPU Intel Xeon, criterion 0.8.2
> **Performer:** Super Z (agent) under the naryad contract AGENTS.md §8

## 1. Assignment and fact verification against the code (AGENTS.md §1)

All assignment facts confirmed against code `2afa719`:

- Metalogos memory is SQLite via `rusqlite 0.40.2` (Cargo.lock; `bundled` — a statically linked in-process C build of SQLite). The `memories` table stores the embedding as a `BLOB` of little-endian f32 (`SqliteStore::embedding_to_blob` / `blob_to_embedding`, `src/memory_store.rs:457/466`).
- Semantic recall — a full scan: `cosine_similarity` from `src/embeddings.rs:297` (scalar dot/norms, no SIMD) is applied in 4 places in `src/memory_store.rs` (lines 95, 198, 562, 765): the default TW path `recall_top_k`, `SqliteStore::recall_top_k`, `SqliteStore::recall`, the kg block. The real cost of a query = SELECT of all rows + decode of each BLOB + scalar cosine over each row.
- `sqlite-vec` was absent from `Cargo.toml` — confirmed.
- FEATURE_INTAKE §5: binary (Linux x86_64) ~6 MB (warning 8 MB, limit 12 MB); new dependencies per version: warning 2, limit 5.
- Crate chosen per the assignment: `sqlite-vec 0.1.9` (asg017, MIT; latest stable as of 2026-09-12; 2.8M downloads; the 0.1.10 alpha line is not used).

## 2. Compatibility: sqlite-vec + rusqlite 0.40 bundled

**Assignment fact-check confirmed: the feature `load_extension` is NOT needed.** The integration contract is static registration of the extension as an auto-extension, before opening the connection:

```rust
use rusqlite::ffi::sqlite3_auto_extension;
unsafe {
    sqlite3_auto_extension(Some(std::mem::transmute(
        sqlite_vec::sqlite3_vec_init as *const (),
    )));
}
// every NEW Connection (including open_in_memory) gets vec0
```

- Smoke test `tests/naryad_271_sqlite_vec_spike.rs` on rusqlite 0.40.2 bundled: **PASS** — `SELECT vec_version()` → `v0.1.9` (the extension loaded), `CREATE VIRTUAL TABLE ... USING vec0(embedding float[384] distance_metric=cosine)`, KNN `MATCH ? AND k = 5` returned k rows sorted by distance, and top-1 matched the full scalar scan.
- Static linking works: the cc crate compiles `sqlite-vec.c` into `libsqlite_vec0.a`, which is linked into the binary (see §4). Dynamic loading of `.so`/`.dll` is not used at all — this eliminates an entire class of platform-specific extension-loading problems.
- `distance_metric=cosine` is supported in vec0 and verified; on unit vectors the ranking is identical to L2 (verified with a diagnostic probe on both metrics — absolute distances differ, the order is the same), which gives compatibility with the existing cosine contract of `memory_store`.

## 3. KNN correctness

- Smoke: 1000 vectors dim=384 (deterministic PRNG), KNN k=5 — top-1 of vec0 == top-1 of the full scalar scan.
- Benchmark: built-in top-1 verification at both scales — `bf=0.815460, vec0=0.815460` (10K and 100K).
- Two bugs found during the spike were **in the spike's own code** (not in vec0) and were fixed; recorded as lessons:
  1. The generator seed `seed | 1` collapsed adjacent seeds (42 and 43) into one state — duplicate vectors resulted, and the "top-1 divergence" was a choice between two equal vectors. Diagnosis with a probe using direct distances showed `distance=0.00000000` for both rowids — **vec0 computed correctly from the very start**. Fix — a murmur3 finalizer for the seed.
  2. In the benchmark, the query for the full scan and the query for vec0 were generated from different PRNG streams — different queries were being compared. Fix — a single query vector for both paths.

## 4. Binary size

Method: a minimal probe crate with the same rusqlite 0.40 bundled as Metalogos; two release binaries on the same dep tree — one without vec and one with real use of the vec path (registration + vec0 + KNN; the linker discards unused code, hence the measurement is taken with actual use). The Metalogos release profile (default, no strip settings) was reproduced.

| Measurement | Bytes | MB |
|---|---|---|
| Base (rusqlite bundled, no vec) | 2 590 960 | 2.47 |
| + sqlite-vec (real use) | 2 740 280 | 2.61 |
| **Delta** | **149 320** | **0.15** |

- Compiled C: `sqlite-vec.o` 256 952 B (~251 KB), `libsqlite_vec0.a` 259 848 B (~254 KB) — the upstream claim "~100KB C" refers to the source; the compiled object is ~4× larger, and ~146 KB ends up in the binary after linking.
- Projection for `mlog` (currently ~6 MB): ≈ 6.15 MB — noticeably below the 8 MB warning threshold (FEATURE_INTAKE §5).
- Honesty caveat: the delta was measured on the probe binary, not on `mlog` itself; the addition is additive and independent of the rest of the binary, final confirmation — the release build of naryad #272.

**Go criterion "delta < 2 MB": met with a ~13× margin.**

## 5. Performance: KNN vs the current full scan

criterion 0.8.2, default parameters, release bench profile; three measurements per scale: the current path (BLOB→decode→scalar cosine), cosine without decode (lower bound of the current path), sqlite-vec KNN (vec0, cosine metric). k=10, dim=384.

| N vectors | full scan (current path) | cosine without decode | **sqlite-vec KNN** | speedup |
|---|---|---|---|---|
| 10 000 | 8.78 ms | 6.52 ms | **4.41 ms** | 1.99× |
| 100 000 | 111.6 ms | 82.0 ms | **57.6 ms** | 1.94× |

- Insert speed: **~81–84K vectors/s** (transaction, in-memory, float[384]) — the 100K corpus fills in ~1.2 s, batch memory backfill will not be a bottleneck.
- **Honest interpretation:** sqlite-vec 0.1.9 is also brute-force (a linear scan with a SIMD core in C), NOT an ANN index. The ~2× win comes from SIMD and from the scan running inside SQLite (no need to pull all rows and decode BLOBs into Rust). The decode part of the current path costs ~25% (8.78→6.52 ms at 10K), the math is the rest.
- Product significance: at 100K records the current recall is 112 ms per query (the edge of interactivity), vec0 — 58 ms; the main point — KNN moves INSIDE SQLite (the whole table is no longer materialized in Rust on every query), which is the architectural base of Phase 4 (scenario centroids, `recall_from_scenario`).

**Go criterion "KNN 10K×384 < 50 ms": met (4.4 ms, ~11× margin).**

## 6. Platforms

| Platform | Status | Evidence |
|---|---|---|
| Linux x86_64 | **VERIFIED** | locally: cc build + smoke + bench (this report) |
| macOS (arm64, github-runner) | **VERIFIED** | the `macos-check (non-blocking)` job on the spike branch: `cargo check --workspace --features portable --all-targets` — success (portable includes vec on the spike) |
| Windows (github-runner) | **VERIFIED** | same `windows-check (non-blocking)` job on the spike branch (`4bc40de`): `cargo check --workspace --features portable --all-targets` — success |
| wasm32-unknown-unknown (browser) | **NO-GO on the current stack** | see below |
| wasm32-wasip1 | path exists, not verified | requires wasi-sdk (clang-wasi); the build.rs of libsqlite3-sys 0.38.2 has a dedicated wasm32-wasi branch (`SQLITE_THREADSAFE=0`, mmap/getpid/signal emulations, optional wasm32-wasi-vfs) |

WASM facts:
- `cargo check --target wasm32-unknown-unknown` (probe crate rusqlite bundled + sqlite-vec): fails — cc-rs does not find a C toolchain for the wasm target; the build.rs of libsqlite3-sys 0.38.2 has no branch for wasm32-unknown-unknown. The browser path through rusqlite is impossible without rewriting the stack base — consistent with the verdict of naryad #278 (wasm spike: No-Go for the runtime in its current form, the Go path is a separate Go-stack Playground).
- sqlite-vec itself is wasm-compatible: upstream ships a wasm build for SQLite-WASM ("in the browser with WASM"). For the Playground Go path (naryad #278), in-browser memory is SQLite-WASM + sqlite-vec.wasm, a line separate from rusqlite.

**Go criterion "3 OSes without manual flags": met** (Linux locally; macOS/Windows — CI jobs on the spike branch, without a single manual flag: the cc build from the crate's build.rs).

## 7. Verdict: GO

| Criterion (assignment proposal) | Fact | Result |
|---|---|---|
| Binary delta < 2 MB | 0.15 MB | PASS (~13× margin) |
| KNN 10K×384 < 50 ms | 4.41 ms | PASS (~11× margin) |
| 3 OSes without manual flags | Linux locally + macOS/Windows spike CI jobs — all success | PASS |
| (extra) KNN correctness | top-1 == full scan at 1K/10K/100K | PASS |

Performer's decision under the mechanics of the verdict gate of dispatch #316: **GO** — naryad #272 (embed / vec_store / vec_search) is implemented on top of sqlite-vec; the ADR-0134 draft is attached to this PR.

## 8. Consequences for naryad #272 (draft, pending tasking in issue #308)

- Feature `vec = ["dep:sqlite-vec"]`, off-by-default; proposal: include it in `portable` at the merge of naryad #272 (the 0.15 MB delta allows it, cross-OS CI will cover it continuously — the candle/vision pattern, ADR-0104 measured impact).
- Integration: auto-extension registration at memory DB open; a vec0 embeddings table; the `mem_type` filter — via vec0 partition keys or post-filtering (decision of naryad #272); the RRF hybrid is preserved — BM25 stays on FTS5, vec0 replaces only the cosine half.
- Honest boundary: brute-force, not ANN; revisit point — 100K+ records or the appearance of ANN/quantization in upstream sqlite-vec (recorded in ADR-0134 as a revisit condition).

## 9. Spike artifacts (stay on the branch, not merged)

- `benches/naryad_271_vec_knn.rs` — criterion benchmark with built-in top-1 verification; without the `vec` feature it compiles to an empty stub (harness=false bench — a binary, main is required: E0601 of the spike's first CI run caught this, fix — main in the crate root, criterion_main! cannot be placed inside a module).
- `tests/naryad_271_sqlite_vec_spike.rs` — smoke test of the integration contract.
- `Cargo.toml` — sqlite-vec 0.1.9 optional, feature `vec`, temporarily in `portable` (SPIKE marker).
