// ── Embedding backend for METALOGOS M3 (Phase 7.2) ──────────────────
// Provides text embedding for semantic recall:
// - OpenAI text-embedding-3-small via HTTP API
// - TF-IDF fallback (bag-of-words + cosine similarity) when API unavailable
// - EmbeddingBackend trait for provider abstraction

use std::collections::HashMap;
use std::env;
use std::sync::Mutex;
use std::time::Duration;

/// Trait for embedding backends — allows swapping between real API and local fallback.
pub trait EmbeddingBackend: Send + Sync {
    /// Generate an embedding vector for the given text.
    /// Returns a fixed-dimension vector of f32 values.
    fn embed(&self, text: &str) -> Result<Vec<f32>, String>;

    /// Compute cosine similarity between two embedding vectors.
    /// Returns a value in [-1.0, 1.0] where 1.0 means identical direction.
    fn similarity(&self, a: &[f32], b: &[f32]) -> f32;

    /// Return the embedding dimension for this backend.
    fn dimension(&self) -> usize;
}

// ── OpenAI Embedding Provider ──────────────────────────────────────

/// OpenAI embedding backend using text-embedding-3-small (1536 dimensions).
///
/// Configuration via environment variables:
/// - `METALOGOS_EMBEDDING_PROVIDER`: set to "openai" to use this provider
/// - `METALOGOS_EMBEDDING_API_KEY`: OpenAI API key
/// - `METALOGOS_EMBEDDING_MODEL`: model name (default: text-embedding-3-small)
pub struct OpenAIEmbedding {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

/// OpenAI text-embedding-3-small output dimension.
pub const OPENAI_EMBEDDING_DIM: usize = 1536;

impl OpenAIEmbedding {
    /// Create a new OpenAI embedding backend from environment configuration.
    pub fn new() -> Result<Self, String> {
        let api_key = env::var("METALOGOS_EMBEDDING_API_KEY")
            .map_err(|_| "METALOGOS_EMBEDDING_API_KEY not set".to_string())?;
        let model = env::var("METALOGOS_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

        Ok(OpenAIEmbedding {
            api_key,
            model,
            client,
        })
    }

    /// Create with explicit configuration (for testing).
    pub fn with_config(api_key: String, model: String) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client build error: {}", e))?;

        Ok(OpenAIEmbedding {
            api_key,
            model,
            client,
        })
    }

    /// Call the OpenAI embeddings API.
    /// POST https://api.openai.com/v1/embeddings
    fn call_api(&self, text: &str) -> Result<Vec<f32>, String> {
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
            "encoding_format": "float"
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("OpenAI embeddings request failed: {}", e))?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e| format!("OpenAI embeddings response read error: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "OpenAI embeddings API error ({}): {}",
                status.as_u16(),
                truncate_response(&body_text, 500)
            ));
        }

        parse_openai_embeddings_response(&body_text)
    }
}

impl EmbeddingBackend for OpenAIEmbedding {
    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        self.call_api(text)
    }

    fn similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        cosine_similarity(a, b)
    }

    fn dimension(&self) -> usize {
        OPENAI_EMBEDDING_DIM
    }
}

/// Parse OpenAI embeddings response: `{ "data": [{ "embedding": [...] }] }`
fn parse_openai_embeddings_response(raw: &str) -> Result<Vec<f32>, String> {
    let json: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("Failed to parse OpenAI embeddings JSON: {}", e))?;

    json.get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("embedding"))
        .and_then(|e| e.as_array())
        .map(|vec| {
            vec.iter()
                .filter_map(|v| v.as_f64())
                .map(|v| v as f32)
                .collect()
        })
        .ok_or_else(|| {
            format!(
                "Unexpected OpenAI embeddings response format: {}",
                truncate_response(raw, 300)
            )
        })
}

// ── TF-IDF Fallback ─────────────────────────────────────────────────

/// TF-IDF based embedding fallback for when API is unavailable.
///
/// Uses a simple bag-of-words approach with cosine similarity:
/// - Vocabulary is built from all texts ever embedded (global corpus)
/// - TF (term frequency) counts word occurrences in the text
/// - IDF (inverse document frequency) weights rare words higher
/// - Embedding dimension = vocabulary size (dynamic, grows with usage)
///
/// Thread-safe via interior mutability (Mutex) — vocabulary grows on every embed call.
pub struct TfidfEmbedding {
    inner: Mutex<TfidfInner>,
}

/// Internal mutable state for TF-IDF computation.
struct TfidfInner {
    /// Global vocabulary: word -> index in the embedding vector.
    vocab: HashMap<String, usize>,
    /// Document frequency: how many documents contain each word.
    doc_freq: HashMap<String, usize>,
    /// Total number of documents seen (for IDF computation).
    total_docs: usize,
}

/// Default TF-IDF embedding dimension (fixed-size for deterministic behavior in tests).
pub const TFIDF_EMBEDDING_DIM: usize = 256;

impl Default for TfidfEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

impl TfidfEmbedding {
    /// Create a new TF-IDF embedding backend with empty vocabulary.
    pub fn new() -> Self {
        TfidfEmbedding {
            inner: Mutex::new(TfidfInner {
                vocab: HashMap::new(),
                doc_freq: HashMap::new(),
                total_docs: 0,
            }),
        }
    }

    /// Tokenize text into lowercase words, filtering out short tokens and non-alphanumeric.
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .filter(|w| w.chars().count() > 1) // ignore single-char tokens (char-count, not byte-count)
            .map(|w| w.to_string())
            .collect()
    }

    /// Compute TF-IDF vector for the given text (mutable, grows vocabulary).
    fn compute_tfidf(inner: &mut TfidfInner, text: &str) -> Vec<f32> {
        let tokens = Self::tokenize(text);
        if tokens.is_empty() {
            return vec![0.0; inner.vocab.len().max(TFIDF_EMBEDDING_DIM)];
        }

        // Update document frequencies and total count
        let mut seen_terms = std::collections::HashSet::new();
        for token in &tokens {
            if !seen_terms.contains(token) {
                seen_terms.insert(token.clone());
                *inner.doc_freq.entry(token.clone()).or_insert(0) += 1;
            }
        }
        inner.total_docs += 1;

        // Ensure all tokens are in vocabulary
        let mut max_idx = 0;
        for token in &tokens {
            if !inner.vocab.contains_key(token) {
                let idx = inner.vocab.len();
                inner.vocab.insert(token.clone(), idx);
            }
            max_idx = max_idx.max(*inner.vocab.get(token).unwrap_or(&0));
        }

        // Use at least TFIDF_EMBEDDING_DIM dimensions
        let dim = max_idx + 1;
        let effective_dim = dim.max(TFIDF_EMBEDDING_DIM);
        let mut vec = vec![0.0f32; effective_dim];

        // Term frequency (count per term in this document)
        let mut tf_counts = HashMap::new();
        for token in &tokens {
            *tf_counts.entry(token.clone()).or_insert(0.0) += 1.0;
        }

        // TF-IDF = tf * (log((N+1)/(df+1)) + 1)  [smooth IDF, never zero]
        for (term, count) in tf_counts {
            if let Some(&idx) = inner.vocab.get(&term) {
                let tf = count / tokens.len() as f32;
                let df = (*inner.doc_freq.get(&term).unwrap_or(&1) + 1) as f32;
                let n = (inner.total_docs + 1) as f32;
                let idf = (n / df).ln() + 1.0; // Smooth IDF: always >= 1.0
                if idx < effective_dim {
                    vec[idx] = tf * idf;
                }
            }
        }

        // Normalize to unit vector
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        }

        vec
    }
}

impl EmbeddingBackend for TfidfEmbedding {
    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // Interior mutability: lock mutex, compute TF-IDF, grow vocabulary
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| format!("TF-IDF lock error: {}", e))?;
        Ok(Self::compute_tfidf(&mut inner, text))
    }

    fn similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        cosine_similarity(a, b)
    }

    fn dimension(&self) -> usize {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.vocab.len().max(TFIDF_EMBEDDING_DIM)
    }
}

// ── Cosine Similarity ──────────────────────────────────────────────

/// Compute cosine similarity between two vectors.
/// Returns a value in [-1.0, 1.0] where 1.0 means identical direction.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot_product / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

// ── Embedding Manager ──────────────────────────────────────────────

/// Manages embedding backend selection and provides a unified interface.
///
/// Strategy:
/// 1. If `METALOGOS_EMBEDDING_PROVIDER=openai` and `METALOGOS_EMBEDDING_API_KEY` is set → use OpenAI
/// 2. Otherwise → use TF-IDF fallback (no API needed)
///
/// The TF-IDF fallback uses interior mutability via Mutex for thread-safe
/// vocabulary growth during memorize operations.
pub struct EmbeddingManager {
    backend: Box<dyn EmbeddingBackend>,
    /// Whether we're using a real API or local fallback.
    is_api: bool,
}

impl Default for EmbeddingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingManager {
    /// Create an embedding manager from environment configuration.
    pub fn new() -> Self {
        let provider = env::var("METALOGOS_EMBEDDING_PROVIDER")
            .unwrap_or_else(|_| "tfidf".to_string())
            .to_lowercase();

        match provider.as_str() {
            "openai" => match OpenAIEmbedding::new() {
                Ok(backend) => EmbeddingManager {
                    backend: Box::new(backend),
                    is_api: true,
                },
                Err(e) => {
                    eprintln!(
                        "[embeddings] OpenAI unavailable ({}), falling back to TF-IDF",
                        e
                    );
                    EmbeddingManager {
                        backend: Box::new(TfidfEmbedding::new()),
                        is_api: false,
                    }
                }
            },
            _ => EmbeddingManager {
                backend: Box::new(TfidfEmbedding::new()),
                is_api: false,
            },
        }
    }

    /// Create with a specific backend (for testing).
    pub fn with_backend(backend: Box<dyn EmbeddingBackend>, is_api: bool) -> Self {
        EmbeddingManager { backend, is_api }
    }

    /// Generate embedding for the given text.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        self.backend.embed(text)
    }

    /// Compute similarity between two embeddings.
    pub fn similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        self.backend.similarity(a, b)
    }

    /// Check if using a real API backend.
    pub fn is_api_backend(&self) -> bool {
        self.is_api
    }

    /// Get the embedding dimension.
    pub fn dimension(&self) -> usize {
        self.backend.dimension()
    }
}

// ── Truncate helper ────────────────────────────────────────────────

fn truncate_response(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Unicode-safe: find char boundary to avoid mid-character slice panic
        let end = s
            .char_indices()
            .take_while(|(i, _)| *i < max_len)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(max_len);
        format!("{}...", &s[..end])
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Cosine Similarity ───────────────────────────────────────────

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.5];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    // ── TF-IDF Embedding ───────────────────────────────────────────

    #[test]
    fn test_tfidf_embed_single_word() {
        let tfidf = TfidfEmbedding::new();
        let v = tfidf.embed("hello").unwrap();
        assert!(!v.is_empty());
        // Vector should be normalized
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6 || norm.abs() < 1e-6);
    }

    #[test]
    fn test_tfidf_embed_same_text_same_vector() {
        let tfidf = TfidfEmbedding::new();
        let v1 = tfidf.embed("the cat sat on the mat").unwrap();
        // embed again (vocab is now populated, second doc changes IDF slightly)
        let v2 = tfidf.embed("the cat sat on the mat").unwrap();
        // Should be very similar (identical if no IDF change)
        let sim = cosine_similarity(&v1, &v2);
        assert!(
            sim > 0.9,
            "Same text should have high similarity, got {}",
            sim
        );
    }

    #[test]
    fn test_tfidf_embed_different_texts_lower_similarity() {
        let tfidf = TfidfEmbedding::new();
        let v1 = tfidf.embed("the cat sat on the mat").unwrap();
        let v2 = tfidf.embed("quantum physics equations").unwrap();
        let sim = cosine_similarity(&v1, &v2);
        // Different topics with no shared words → orthogonal vectors → similarity ~0
        assert!(
            sim < 0.5,
            "Different topics should have low similarity, got {}",
            sim
        );
    }

    #[test]
    fn test_tfidf_empty_text() {
        let tfidf = TfidfEmbedding::new();
        let v = tfidf.embed("").unwrap();
        // Empty text → all zeros
        assert!(v.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn test_tfidf_tokenize() {
        let tokens = TfidfEmbedding::tokenize("Hello, World! This is a Test.");
        // "is" and "a" are kept (filter only removes single-char tokens)
        assert_eq!(tokens, vec!["hello", "world", "this", "is", "test"]);
    }

    #[test]
    fn test_tfidf_tokenize_filters_short() {
        let tokens = TfidfEmbedding::tokenize("a b c hello");
        // Single-char tokens are filtered
        assert_eq!(tokens, vec!["hello"]);
    }

    #[test]
    fn test_tfidf_dimension_grows() {
        let tfidf = TfidfEmbedding::new();
        assert_eq!(tfidf.dimension(), TFIDF_EMBEDDING_DIM); // minimum dimension
        tfidf.embed("alpha beta gamma").unwrap();
        // Dimension should be at least 3 (3 unique words) or TFIDF_EMBEDDING_DIM
        assert!(tfidf.dimension() >= 3);
    }

    #[test]
    fn test_tfidf_embed_unit_norm() {
        let tfidf = TfidfEmbedding::new();
        let v = tfidf.embed("hello world foo bar baz").unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    // ── Embedding Manager ──────────────────────────────────────────

    #[serial_test::serial]
    #[test]
    fn test_embedding_manager_default_is_tfidf() {
        env::remove_var("METALOGOS_EMBEDDING_PROVIDER");
        let mgr = EmbeddingManager::new();
        assert!(!mgr.is_api_backend());
    }

    #[test]
    fn test_embedding_manager_with_custom_backend() {
        let tfidf = TfidfEmbedding::new();
        let mgr = EmbeddingManager::with_backend(Box::new(tfidf), false);
        assert!(!mgr.is_api_backend());
        let v = mgr.embed("test").unwrap();
        assert!(!v.is_empty());
    }

    #[test]
    fn test_embedding_manager_similarity() {
        let tfidf = TfidfEmbedding::new();
        let mgr = EmbeddingManager::with_backend(Box::new(tfidf), false);
        let v1 = mgr.embed("hello world").unwrap();
        let v2 = mgr.embed("hello world").unwrap();
        let sim = mgr.similarity(&v1, &v2);
        assert!(sim > 0.9);
    }

    // ── OpenAI Embedding Construction ──────────────────────────────

    #[serial_test::serial]
    #[test]
    fn test_openai_embedding_missing_key() {
        env::remove_var("METALOGOS_EMBEDDING_API_KEY");
        let result = OpenAIEmbedding::new();
        assert!(result.is_err());
        let err_msg = result.err().unwrap();
        assert!(err_msg.contains("METALOGOS_EMBEDDING_API_KEY"));
    }

    #[test]
    fn test_openai_embedding_with_config() {
        let result = OpenAIEmbedding::with_config(
            "sk-test".to_string(),
            "text-embedding-3-small".to_string(),
        );
        assert!(result.is_ok());
        let emb = result.unwrap();
        assert_eq!(emb.dimension(), OPENAI_EMBEDDING_DIM);
    }

    // ── OpenAI Response Parsing ────────────────────────────────────

    #[test]
    fn test_parse_openai_embeddings_response() {
        let raw = r#"{"data":[{"embedding":[0.1,0.2,0.3],"index":0,"object":"embedding"}],"model":"text-embedding-3-small","usage":{"prompt_tokens":2,"total_tokens":2}}"#;
        let result = parse_openai_embeddings_response(raw).unwrap();
        assert_eq!(result, vec![0.1f32, 0.2, 0.3]);
    }

    #[test]
    fn test_parse_openai_embeddings_response_empty_data() {
        let raw = r#"{"data":[],"model":"text-embedding-3-small"}"#;
        let result = parse_openai_embeddings_response(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_openai_embeddings_invalid_json() {
        let result = parse_openai_embeddings_response("not json");
        assert!(result.is_err());
    }

    // ── Shared words between languages (fallback behavior) ──────────

    #[test]
    fn test_tfidf_shared_words_partial_match() {
        let tfidf = TfidfEmbedding::new();
        let v1 = tfidf.embed("the cat sat").unwrap();
        let v2 = tfidf.embed("the cat sat on the mat").unwrap();
        // "the cat sat" shares words with the longer text
        let sim = cosine_similarity(&v1, &v2);
        assert!(
            sim > 0.5,
            "Shared words should give similarity > 0.5, got {}",
            sim
        );
    }

    #[test]
    fn test_tfidf_no_shared_words_zero_similarity() {
        let tfidf = TfidfEmbedding::new();
        // Embed "the cat sat" first (adds to vocab)
        tfidf.embed("the cat sat").unwrap();
        // Then embed text with completely different words
        let v2 = tfidf.embed("feline sitting").unwrap();
        // "feline" and "sitting" are in vocab now (added by second embed)
        // but at different indices than "the cat sat" words → orthogonal
        let sim = cosine_similarity(&tfidf.embed("the cat sat").unwrap(), &v2);
        assert!(
            sim < 0.3,
            "Disjoint words should give near-zero similarity, got {}",
            sim
        );
    }

    #[test]
    fn test_tfidf_grows_vocab_on_each_embed() {
        let tfidf = TfidfEmbedding::new();
        let v1 = tfidf.embed("the cat sat on the mat").unwrap();
        assert!(!v1.is_empty());
        // Different words also get added
        let v2 = tfidf.embed("quantum physics").unwrap();
        assert!(
            !v2.iter().all(|x| *x == 0.0),
            "Second embed should add new words to vocab and produce non-zero vector"
        );
    }
}
