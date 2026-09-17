# ADR-0135: Semantic cache (cache_semantic) + LRU bound for the ADR-0047 cache

**Status:** Accepted
**Date:** 2026-09-12
**Naryad:** #273 (issue #309)
**Precedent:** ADR-0047 (LLM response cache — the foundation; the check order is preserved), ADR-0040 (EmbeddingBackend), #272/ADR-0134 (embed + vector infrastructure), FOIP-003 (token economy as the motivation)

## Context

The ADR-0047 cache is exact-match by hash (effective_prompt + input). Two documented limitations: (1) semantically identical queries with different wording — a miss (for FOSVED on the Render free tier this is a direct overpayment in tokens); (2) "In-memory cache grows without bound (no eviction policy beyond TTL)" — a line from the Consequences of ADR-0047 itself.

## Decision

### D1. LRU bound for the in-memory cache

The in-memory HashMap of the cache is bounded by `max_entries` (env `METALOGOS_LLM_CACHE_MAX`, default 1000; an invalid value — a loud warning on stderr and the default: an env typo must not disable the bound). Eviction by recency of USE (use = a get hit or an insert; a monotonic recency counter), not by creation age. Closes the ADR-0047 negative — a reference line has been added to its Consequences. The ADR-0047 surface (SQLite table `llm_cache`) is unchanged: `last_used` is an in-memory field.

### D2. Semantic mode — opt-in per pattern, layered on top of the exact hash

The learnable-pattern option `cache_semantic: true` + `cache_threshold: 0.92` (default, range (0, 1], parser-level validation — loud error). The ADR-0047 check order is preserved: few-shot → **exact hash** → semantic → LLM. On an exact-hash miss: `embed(input)` (the #272 SSOT manager — the same vectors as the `embed` builtin), a cosine scan over the SQLite table `llm_cache_semantic` (key, response, embedding BLOB, dim, created_at, ttl; created lazily), a hit when similarity ≥ threshold → the cached response is returned; a miss → LLM, the response + embedding are saved (INSERT OR REPLACE keyed by the exact-cache key).

### D3. Boundaries and anti-risks

- **Only with persist**: without `memory { persist: ... }`, enabling `cache_semantic` is a loud configuration error on the first call (in-memory vectors are deliberately not stored: memory/correctness). Without the `vec` feature — also a loud error (no embeddings).
- **False hits**: the default threshold is high (0.92); opt-in per pattern with a default of false — backward compatible with ADR-0047; a dim mismatch between stored rows and the query yields cosine 0.0 (below any threshold).
- **Observability**: a semantic hit is reflected in `llm_usage()` by a separate `cache_hits_semantic` counter (exact hits are not mixed in) and in the #276 traces as `cache: "semantic"`.
- **TTL takes precedence over semantics**: an expired semantic row does not hit (the same TTL semantics as the exact cache; expired rows are deleted during the scan).
- **Full-scan cosine, not vec0**: the cache table is small (hundreds of rows), a full scan with BLOB decode and scalar cosine is milliseconds; the #272 KNN infrastructure remains the foundation of memory Phase 4. The Decision is revisited if the cache tables grow.
- **TF-IDF drift**: IDF/dimension depend on the process's embed-call history (ADR-0040 boundary) — the similarity of paraphrases changes from measurement to measurement; the threshold is not a guaranteed quality metric, it is a cutoff. Honestly documented in REFERENCE and tests (the boundary values 0.919/0.921 are not reproducible against drift; the contract hit ⟺ sim ≥ threshold is covered by the pair "low threshold → hit" / "0.99 → miss").
- **VM boundary**: the cache loop of ADR-0047/0135 is the domain of the TW interpreter (the naryad Files list); VM::call_llm is a separate surface without a cache (the pre-registry boundary of ADR-0105), extension is out of scope for #273.

## Consequences

- Positive: semantic hits save tokens (FOIP-003); the ADR-0047 negative about unbounded growth is closed; observability separates exact/semantic hits; backward compatibility is complete (defaults false / 0.92 / no persist — ADR-0047 behavior is unchanged).
- Negative: a false hit is possible with overstated proximity of wording — mitigated by the high threshold and opt-in; a full table scan is O(n) per miss (acceptable at cache scales).
- Neutral: the semantic table lives in the same persist file as `llm_cache`/`memories` — one SQLite memory file (ADR-0116 is not violated).
