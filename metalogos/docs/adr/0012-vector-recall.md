# ADR-0012: Vector Recall (Embedding-Based Semantic Similarity)

**Status:** Implemented (Phase 2.2 — SimpleEmbeddingBackend)
**Date:** 2026-05-31
**Milestone:** Phase 2

---

## Context

Before this ADR, `recall` in Metalogos used **substring matching** (`entry.value.contains(&query)`).
This meant recall could only find memories whose text literally contained the query string.
A query like `"food preferences"` would NOT find `"user likes spicy food"` because no words
overlap — despite clear semantic relatedness.

The user requirement: "recall 'food preferences' находит 'user likes spicy food', хотя
слов нет в общих."

Question: how should recall find semantically related memories without exact word overlap,
and how should this be extensible for future real embedding models?

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| Substring / keyword match | grep, Lucene term queries | Fast but misses semantic matches |
| TF-IDF + cosine similarity | Classic IR (Salton & Buckley, 1988) | Statistical, no semantics |
| Bag-of-concepts / ontology | LSA (Deerwester et al., 1990) | Captures synonymy via concept groups |
| Word2Vec / GloVe | Mikolov et al., 2013; Pennington et al., 2014 | Distributional semantics, needs pre-trained vectors |
| Sentence-transformers | Reimers & Gurevych, 2019 | State-of-art, requires PyTorch |
| Vector DB (qdrant/milvus) | Production vector search | Scalable, external dependency |

## Decision

### EmbeddingBackend Trait

A trait abstracts the embedding source, so recall works with any similarity metric:

```rust
pub trait EmbeddingBackend: Send + Sync {
    fn similarity(&self, text_a: &str, text_b: &str) -> f64;
    fn embed(&self, text: &str) -> Vec<f64>;
}
```

Phase 2.3 will implement `PyO3EmbeddingBackend` using this trait with real
sentence-transformers embeddings. The trait is in `src/embedding.rs`.

### Phase 2.2: SimpleEmbeddingBackend (No External Dependencies)

Uses a **bag-of-concepts** approach: 8 hand-crafted concept groups (~300 words total),
each defining a semantic domain:

| Concept | Sample Words |
|---|---|
| food_cuisine | food, spicy, soup, drink, taste, vegetarian... |
| preference_opinion | like, prefer, favorite, enjoy, want... |
| technology | server, code, ai, uptime, compile... |
| person_identity | user, person, admin, developer... |
| emotion | happy, sad, good, bad, stress... |
| time | today, yesterday, now, often, always... |
| quantity_measure | percent, count, price, score, rate... |
| location | city, office, home, remote, near... |

**Algorithm:**
1. Tokenize: lowercase, split on non-alphanumeric
2. Map each token to concept dimensions it belongs to (word → concept index lookup)
3. Build 8-dimensional vector (frequency count per concept group)
4. Normalize to unit length
5. Cosine similarity = dot product of normalized vectors

**How "food preferences" → "user likes spicy food" works:**
- "food" → concept 0 (food_cuisine) += 1
- "preferences" → concept 1 (preference_opinion) += 1
- "likes" → concept 1 (preference_opinion) += 1
- "spicy" → concept 0 (food_cuisine) += 1
- "food" → concept 0 (food_cuisine) += 1
- Both vectors have non-zero values in dimensions 0 and 1 → cosine > 0

**Scoring in recall:** `score = similarity × activation`, where activation = `priority × exp(-decay_rate × age_days)`. This combines semantic relevance with recency/priority decay.

### Limitations (Documented)

1. **~300 words only.** No coverage for domain-specific terms.
2. **No stemming.** "running" ≠ "run" — would need a stemmer or lemmatizer.
3. **No context sensitivity.** "bank" (river) = "bank" (finance) if in same group.
4. **English only.** No multilingual support.
5. **8 dimensions only.** Granularity is very coarse — many unrelated texts will
   share concept dimensions.
6. **This is a placeholder.** Phase 2.3 will replace with real embeddings.

## Rationale

- **Why not TF-IDF?** TF-IDF requires a document corpus to build the IDF matrix.
    Metalogos memories are added incrementally, making corpus-level statistics
    impractical at startup. Bag-of-concepts works with zero corpus.
- **Why not Word2Vec?** Requires pre-trained vectors (~1GB for standard models) or
    training data. Violates the "no external dependencies" constraint.
- **Why cosine similarity?** Standard metric for normalized vector comparison.
    Range [0, 1] maps directly to "no match" → "identical."
- **Why `similarity × activation`?** A highly relevant but very old/low-priority
    memory shouldn't beat a moderately relevant fresh one. The product balances
    semantic relevance with temporal decay.

## Impact

- **`src/embedding.rs`:** New module — `EmbeddingBackend` trait, `SimpleEmbeddingBackend`,
    `cosine_similarity()`, `tokenize()`, `concept_groups()`. 6 unit tests.
- **`src/interpreter.rs`:** `Interpreter` gains `embedding: Box<dyn EmbeddingBackend>` field.
    `invoke_recall()` rewritten: substring match replaced with `self.embedding.similarity()`.
    Scoring changed from `activation` to `similarity × activation`.
- **`src/lib.rs`:** Added `pub mod embedding`.
- **Backward compatible.** All 9 existing golden tests + 4 error tests pass.
    `m4_memory.mlog` still works because "spicy" (the query) and "user likes spicy food"
    (the memory) share the word "spicy" → concept match → cosine > 0.
- **New test:** `p2_vector_recall.mlog` — recall "food preferences" finds "user likes spicy food"
    even though no words overlap (matched via food_cuisine + preference_opinion concepts).

## Phase 2.3 Path

The `EmbeddingBackend` trait is the extension point. To add real embeddings:

```rust
// Phase 2.3: PyO3EmbeddingBackend
pub struct PyO3EmbeddingBackend {
    model: Py<PyModule>,  // sentence-transformers via PyO3
}

impl EmbeddingBackend for PyO3EmbeddingBackend {
    fn similarity(&self, a: &str, b: &str) -> f64 {
        let emb_a = self.embed(a);
        let emb_b = self.embed(b);
        cosine_similarity(&emb_a, &emb_b)
    }
    fn embed(&self, text: &str) -> Vec<f64> {
        // Call Python: model.encode(text)
    }
}
```

The `create_embedding_backend()` factory function switches between backends based on
availability.
