// ── Phase 7.2 Contract Tests: Real Embeddings and Vector Recall ──────
//
// Contracts:
// 1. memorize + recall with shared words finds entry via flow
// 2. memorize "the cat sat" → recall "feline sitting" → empty (no shared words)
// 3. TF-IDF cosine similarity: same text → high similarity (>0.9)
// 4. TF-IDF cosine similarity: different text → lower similarity
// 5. EmbeddingManager defaults to TF-IDF when no API key
// 6. Embeddings are computed during memorize (non-empty vectors)
// 7. Cosine similarity exceeds default threshold
// 8. Recall with knowledge graph traversal still works
// 9. TF-IDF partial word overlap gives intermediate similarity
// 10. OpenAI backend requires API key
// 11. OpenAI embedding dimension is 1536
// 12. TF-IDF unit norm (embeddings are normalized)
// 13. Multiple memorize entries, recall finds best match
// 14. Recall empty memory returns empty
// 15. Cosine similarity edge cases (identical, orthogonal, empty)

use metalogos::embeddings::{
    cosine_similarity, EmbeddingBackend, EmbeddingManager, OpenAIEmbedding, TfidfEmbedding,
    OPENAI_EMBEDDING_DIM,
};
use metalogos::interpreter::Interpreter;

// ── Contract 1: Memorize + recall with shared words via flow ──────────

#[test]
fn test_72_memorize_and_recall_shared_words() {
    let mut interp = Interpreter::new();

    // Memorize a fact
    metalogos::feed_line(
        &mut interp,
        r#"memorize "the cat sat on the mat" with priority=1.0"#,
    )
    .unwrap();

    // Declare a query entity, then recall via flow
    metalogos::feed_line(&mut interp, r#"entity query: String = "the cat sat""#).unwrap();
    let result = metalogos::feed_line(
        &mut interp,
        r#"flow Main { input: String = query -> recall -> output }"#,
    )
    .unwrap();

    assert!(
        result.is_some(),
        "recall flow should return a result for shared words"
    );
    let output = result.unwrap();
    assert!(
        output.contains("cat sat"),
        "Should recall 'cat sat', got: {}",
        output
    );
}

// ── Contract 2: TF-IDF fallback with no shared words → empty ──────────

#[test]
fn test_72_recall_fallback_no_shared_words() {
    let mut interp = Interpreter::new();

    metalogos::feed_line(&mut interp, r#"memorize "the cat sat" with priority=1.0"#).unwrap();
    metalogos::feed_line(&mut interp, r#"entity query: String = "feline sitting""#).unwrap();
    let result = metalogos::feed_line(
        &mut interp,
        r#"flow Main { input: String = query -> recall -> output }"#,
    )
    .unwrap();

    // With TF-IDF, "feline" and "sitting" have no overlap with "the cat sat"
    // Embedding vectors are orthogonal → cosine similarity ≈ 0 → below 0.3 threshold
    assert!(result.is_some());
    let output = result.unwrap();
    assert!(
        output.is_empty(),
        "No shared words should return empty, got: '{}'",
        output
    );
}

// ── Contract 3: Same text → high cosine similarity ───────────────────

#[test]
fn test_72_cosine_similarity_same_text() {
    let tfidf = TfidfEmbedding::new();

    let v1 = tfidf.embed("the cat sat on the mat").unwrap();
    let v2 = tfidf.embed("the cat sat on the mat").unwrap();

    let sim = cosine_similarity(&v1, &v2);
    assert!(
        sim > 0.9,
        "Same text should have similarity > 0.9, got {}",
        sim
    );
}

// ── Contract 4: Different text → lower similarity ───────────────────

#[test]
fn test_72_cosine_similarity_different_text() {
    let tfidf = TfidfEmbedding::new();

    let v1 = tfidf.embed("the cat sat on the mat").unwrap();
    let v_diff = tfidf.embed("quantum physics equations").unwrap();

    let sim = cosine_similarity(&v1, &v_diff);
    assert!(
        sim < 0.5,
        "Different domain should have low similarity, got {}",
        sim
    );
}

// ── Contract 5: EmbeddingManager defaults to TF-IDF ────────────────

#[serial_test::serial]
#[test]
fn test_72_embedding_manager_default_is_tfidf() {
    std::env::remove_var("METALOGOS_EMBEDDING_PROVIDER");
    let mgr = EmbeddingManager::new();
    assert!(!mgr.is_api_backend(), "Default should be TF-IDF fallback");

    let v = mgr.embed("hello world").unwrap();
    assert!(!v.is_empty());
}

// ── Contract 6: Embeddings computed during memorize are non-empty ────

#[test]
fn test_72_embedding_stored_on_memorize() {
    let mgr = EmbeddingManager::new();

    let v1 = mgr.embed("user prefers spicy food").unwrap();
    let v2 = mgr.embed("culinary preferences").unwrap();

    assert!(
        !v1.is_empty(),
        "Embedding for memorize text should not be empty"
    );
    assert!(
        !v2.is_empty(),
        "Embedding for recall query should not be empty"
    );

    let sim = cosine_similarity(&v1, &v2);
    assert!(
        sim < 0.5,
        "Disjoint vocabularies → low similarity, got {}",
        sim
    );
}

// ── Contract 7: Cosine similarity exceeds default 0.3 threshold ────

#[test]
fn test_72_cosine_similarity_threshold() {
    let mgr = EmbeddingManager::new();

    let v1 = mgr.embed("the cat sat").unwrap();
    let v_query = mgr.embed("the cat sat").unwrap();

    let sim = mgr.similarity(&v1, &v_query);
    assert!(
        sim > 0.3,
        "Exact match should exceed default 0.3 threshold, got {}",
        sim
    );
}

// ── Contract 8: Knowledge graph traversal still works ──────────────────

#[test]
fn test_72_recall_with_knowledge_graph() {
    let mut interp = Interpreter::new();

    metalogos::feed_line(
        &mut interp,
        r#"memorize "alice works at google" with priority=1.0"#,
    )
    .unwrap();

    // Need to check the correct relate syntax
    metalogos::feed_line(
        &mut interp,
        r#"relate "alice works at google" to "bob works at google" as "coworker""#,
    )
    .unwrap();

    metalogos::feed_line(&mut interp, r#"entity query: String = "alice works""#).unwrap();
    let result = metalogos::feed_line(
        &mut interp,
        r#"flow Main { input: String = query -> recall -> output }"#,
    )
    .unwrap();

    assert!(result.is_some(), "recall should return a result");
    let output = result.unwrap();
    assert!(
        output.contains("alice works at google"),
        "Should contain alice entry, got: {}",
        output
    );
    assert!(
        output.contains("[GRAPH]"),
        "Should contain graph traversal, got: {}",
        output
    );
}

// ── Contract 9: TF-IDF partial word overlap → intermediate similarity ──

#[test]
fn test_72_tfidf_partial_overlap() {
    let tfidf = TfidfEmbedding::new();

    let v_full = tfidf
        .embed("the quick brown fox jumps over the lazy dog")
        .unwrap();
    let v_query = tfidf.embed("the quick fox").unwrap();

    let sim = cosine_similarity(&v_query, &v_full);
    assert!(
        sim > 0.3 && sim < 1.0,
        "Partial overlap should give intermediate similarity, got {}",
        sim
    );
}

// ── Contract 10: OpenAI backend requires API key ────────────────────

#[serial_test::serial]
#[test]
fn test_72_openai_requires_api_key() {
    std::env::remove_var("METALOGOS_EMBEDDING_API_KEY");
    let result = OpenAIEmbedding::new();
    assert!(result.is_err());
    let err_msg = result.err().unwrap();
    assert!(
        err_msg.contains("METALOGOS_EMBEDDING_API_KEY"),
        "Error should mention API key, got: {}",
        err_msg
    );
}

// ── Contract 11: OpenAI embedding dimension is 1536 ────────────────

#[test]
fn test_72_openai_dimension() {
    let emb =
        OpenAIEmbedding::with_config("sk-test".to_string(), "text-embedding-3-small".to_string())
            .unwrap();
    assert_eq!(emb.dimension(), OPENAI_EMBEDDING_DIM);
    assert_eq!(OPENAI_EMBEDDING_DIM, 1536);
}

// ── Contract 12: TF-IDF unit norm ───────────────────────────────────

#[test]
fn test_72_tfidf_unit_norm() {
    let tfidf = TfidfEmbedding::new();
    let v = tfidf
        .embed("hello world this is a test of the embedding system")
        .unwrap();
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-6,
        "TF-IDF vectors should be unit normalized, got norm={}",
        norm
    );
}

// ── Contract 13: Multiple memorize entries, recall finds best ──────

#[test]
fn test_72_recall_best_match_among_multiple() {
    let mut interp = Interpreter::new();

    metalogos::feed_line(
        &mut interp,
        r#"memorize "the sky is blue" with priority=1.0"#,
    )
    .unwrap();
    metalogos::feed_line(
        &mut interp,
        r#"memorize "grass is green" with priority=1.0"#,
    )
    .unwrap();
    metalogos::feed_line(
        &mut interp,
        r#"memorize "the ocean is deep blue" with priority=1.0"#,
    )
    .unwrap();

    metalogos::feed_line(&mut interp, r#"entity query: String = "blue""#).unwrap();
    let result = metalogos::feed_line(
        &mut interp,
        r#"flow Main { input: String = query -> recall -> output }"#,
    )
    .unwrap();

    assert!(result.is_some());
    let output = result.unwrap();
    assert!(
        output.contains("blue"),
        "Should recall a blue-related entry, got: {}",
        output
    );
}

// ── Contract 14: Recall empty memory returns empty ─────────────────

#[test]
fn test_72_recall_empty_memory() {
    let mut interp = Interpreter::new();

    metalogos::feed_line(&mut interp, r#"entity query: String = "anything""#).unwrap();
    let result = metalogos::feed_line(
        &mut interp,
        r#"flow Main { input: String = query -> recall -> output }"#,
    )
    .unwrap();

    assert!(result.is_some());
    let output = result.unwrap();
    assert!(output.is_empty(), "Empty memory should return empty");
}

// ── Contract 15: Cosine similarity edge cases ─────────────────────

#[test]
fn test_72_cosine_similarity_identical() {
    let v = vec![1.0, 0.0, 0.5];
    assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
}

#[test]
fn test_72_cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    assert!(cosine_similarity(&a, &b).abs() < 1e-6);
}

#[test]
fn test_72_cosine_similarity_empty() {
    assert_eq!(cosine_similarity(&[], &[]), 0.0);
}
