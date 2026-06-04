# ADR-0040: Real Embeddings and Vector Recall (Phase 7.2)

## Status: Accepted

## Context

Phase 5 introduced memory (memorize/recall) with substring matching and knowledge graph
traversal. Recall used `entry.value.contains(&query)` for similarity — requiring exact word
overlap between stored facts and queries. This fails for semantic relationships:
- `memorize "the cat sat"` → `recall "feline resting"` → misses (no shared words)
- `memorize "user prefers spicy food"` → `recall "culinary preferences"` → misses

Phase 7.1 added real LLM backends but recall still used substring matching. A proper semantic
search system needs embedding vectors and cosine similarity.

## Decision

### 1. EmbeddingBackend Trait
Created `EmbeddingBackend` trait in `src/embeddings.rs`:
```rust
pub trait EmbeddingBackend: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
    fn similarity(&self, a: &[f32], b: &[f32]) -> f32;
    fn dimension(&self) -> usize;
}
```

Two implementations:
- **OpenAI**: `text-embedding-3-small` (1536 dimensions) via `POST /v1/embeddings`
- **TF-IDF fallback**: Bag-of-words with cosine similarity (no API needed)

### 2. TF-IDF Fallback (No API Required)
- Thread-safe via `Mutex<TfidfInner>` for interior mutability
- Vocabulary grows dynamically on every `embed()` call
- Tokenization: lowercase, alphanumeric split, filter single-char tokens
- **Smooth IDF**: `log((N+1)/(df+1)) + 1` — never zero, works for single-document corpora
- All vectors normalized to unit length for cosine similarity
- Minimum dimension: 256 (configurable via `TFIDF_EMBEDDING_DIM`)

### 3. OpenAI Embedding Provider
- `POST https://api.openai.com/v1/embeddings` with `text-embedding-3-small`
- 30-second timeout, 10-second connect timeout
- Configuration: `METALOGOS_EMBEDDING_PROVIDER=openai`, `METALOGOS_EMBEDDING_API_KEY`
- Graceful fallback to TF-IDF if API key missing or request fails

### 4. EmbeddingManager
Unified interface with environment-based backend selection:
- `METALOGOS_EMBEDDING_PROVIDER=openai` + API key → OpenAI
- Otherwise → TF-IDF fallback (default, safe for tests)

### 5. Memory Entry Embeddings
`MemoryEntry` now includes an `embedding: Vec<f32>` field:
- During `memorize`: `self.embedding_manager.embed(&value_str)` computes and stores the vector
- During `recall`: query is embedded, cosine similarity computed with all entries
- Score formula: `semantic_similarity × priority × decay`

### 6. Updated Recall Algorithm
```
score = cosine_similarity(query_embedding, entry_embedding) × entry.priority × exp(-decay_rate × age_days)
```
- Default `min_confidence` raised from 0.0 to 0.3 (filters noise)
- Fallback: if embeddings are empty, uses substring match (backward compatible)

### 7. Grammar Change
Removed `"recall"` from the `step_ident` blacklist in `grammar.pest`, allowing:
```mlog
flow Main { input: String = query -> recall -> output }
```

## Contract Tests (17 tests, all passing)

| Contract | Test |
|----------|------|
| memorize + recall shared words | `test_72_memorize_and_recall_shared_words` |
| No shared words → empty | `test_72_recall_fallback_no_shared_words` |
| Same text → similarity > 0.9 | `test_72_cosine_similarity_same_text` |
| Different text → low similarity | `test_72_cosine_similarity_different_text` |
| Manager defaults to TF-IDF | `test_72_embedding_manager_default_is_tfidf` |
| Embeddings non-empty on memorize | `test_72_embedding_stored_on_memorize` |
| Similarity exceeds threshold | `test_72_cosine_similarity_threshold` |
| Knowledge graph still works | `test_72_recall_with_knowledge_graph` |
| Partial overlap → intermediate | `test_72_tfidf_partial_overlap` |
| OpenAI requires API key | `test_72_openai_requires_api_key` |
| OpenAI dimension = 1536 | `test_72_openai_dimension` |
| TF-IDF unit norm | `test_72_tfidf_unit_norm` |
| Multiple entries → best match | `test_72_recall_best_match_among_multiple` |
| Empty memory → empty | `test_72_recall_empty_memory` |
| Cosine: identical | `test_72_cosine_similarity_identical` |
| Cosine: orthogonal | `test_72_cosine_similarity_orthogonal` |
| Cosine: empty | `test_72_cosine_similarity_empty` |

## Security Properties
- No API calls by default (TF-IDF is fully local)
- OpenAI API key stored only in environment, never in code
- TF-IDF vocabulary is per-Interpreter instance (no cross-process data leakage)
- EmbeddingManager is Send+Sync safe for use in async server context

## Consequences
- Breaking: `recall` default min_confidence changed from 0.0 to 0.3
- Breaking: `step_ident` grammar changed (recall removed from blacklist)
- `MemoryEntry` now has `embedding` field — serialization format changes
- All existing memory/recall contracts still pass (backward compatible via fallback)
- New module `src/embeddings.rs` added to project
