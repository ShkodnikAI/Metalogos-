// ── Embedding backend for METALOGOS Phase 2 ─────────────────────────────────
// Provides semantic similarity for memory recall.
// Phase 2.2: SimpleEmbeddingBackend — concept-group word vectors, cosine similarity,
//   no external dependencies. Based on hand-crafted semantic concept maps.
// Phase 2.3: PyO3EmbeddingBackend — real embeddings via sentence-transformers / PyO3.
//
// The EmbeddingBackend trait abstracts away the embedding source so that recall
// works with any similarity metric without changing interpreter code.

/// Trait for computing semantic similarity between two texts.
/// Implementations can use TF-IDF, word vectors, sentence-transformers, etc.
pub trait EmbeddingBackend: Send + Sync {
    /// Compute cosine similarity between two text strings.
    /// Returns a value in [0.0, 1.0] where 1.0 = identical meaning.
    fn similarity(&self, text_a: &str, text_b: &str) -> f64;

    /// Embed a text into a fixed-size vector for storage/indexing.
    /// Returns a Vec<f64> of the embedding dimension.
    fn embed(&self, text: &str) -> Vec<f64>;
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── Simple Embedding Backend (no external deps) ────────────────────────────
//
// Uses a hand-crafted concept map: groups of semantically related words map to
// shared "concept" dimensions. Each text is converted to a bag-of-concepts vector
// (frequency-weighted), then compared via cosine similarity.
//
// This is NOT a real embedding model — it's a minimal semantic baseline that
// catches obvious related-word similarity (e.g., "food" ↔ "spicy" via the
// "food_cuisine" concept). Phase 2.3 will replace this with real embeddings.
//
// Prior art: Latent Semantic Analysis (Deerwester et al., 1990), bag-of-concepts.

/// A concept group: a label and a set of words that belong to this concept.
struct ConceptGroup {
    #[allow(dead_code)]
    label: &'static str,
    words: Vec<&'static str>,
}

/// Build the hardcoded concept groups for basic semantic similarity.
/// Words in the same group are considered semantically related.
/// The number of concepts defines the embedding dimension.
fn concept_groups() -> Vec<ConceptGroup> {
    vec![
        ConceptGroup { label: "food_cuisine", words: vec![
            "food", "meal", "dish", "cuisine", "eat", "eating", "ate", "cook", "cooking",
            "spicy", "sweet", "sour", "bitter", "salty", "umami", "taste", "tasty",
            "hungry", "hunger", "dinner", "lunch", "breakfast", "supper", "snack",
            "restaurant", "kitchen", "recipe", "ingredient", "flavor", "delicious",
            "soup", "bread", "meat", "fish", "vegetable", "fruit", "dessert",
            "pizza", "pasta", "rice", "salad", "cake", "pie", "sandwich",
            "beverage", "drink", "water", "tea", "coffee", "juice", "wine", "beer",
            "diet", "nutrition", "healthy", "unhealthy", "organic", "vegan", "vegetarian",
        ]},
        ConceptGroup { label: "preference_opinion", words: vec![
            "like", "likes", "liked", "dislike", "hates", "hated", "love", "loves",
            "loved", "prefer", "prefers", "preference", "preferences", "favorite",
            "enjoy", "enjoys", "enjoyed", "want", "wants", "wanted", "wish", "wishes",
            "desire", "desires", "craving", "appeal", "appealing", "fond", "adore",
            "appreciate", "recommend", "suggest", "opinion", "feel", "feeling",
            "think", "believe", "consider", "rate", "rating", "review",
        ]},
        ConceptGroup { label: "technology", words: vec![
            "computer", "server", "network", "internet", "software", "hardware",
            "code", "programming", "algorithm", "data", "database", "system",
            "machine", "learning", "ai", "artificial", "intelligence", "model",
            "api", "cloud", "storage", "memory", "cpu", "gpu", "process",
            "uptime", "downtime", "deploy", "deployed", "build", "compile",
            "runtime", "framework", "library", "module", "function", "interface",
        ]},
        ConceptGroup { label: "person_identity", words: vec![
            "user", "person", "people", "human", "man", "woman", "child",
            "customer", "client", "admin", "manager", "employee", "worker",
            "individual", "member", "owner", "author", "creator", "developer",
            "someone", "anyone", "everyone", "myself", "yourself", "themselves",
        ]},
        ConceptGroup { label: "emotion", words: vec![
            "happy", "sad", "angry", "excited", "bored", "afraid", "scared",
            "joy", "sorrow", "fear", "surprise", "disgust", "trust", "anxiety",
            "stress", "relax", "calm", "peace", "comfort", "pain", "pleasure",
            "satisfaction", "frustration", "confusion", "curiosity", "wonder",
            "good", "bad", "great", "terrible", "awful", "nice", "wonderful",
        ]},
        ConceptGroup { label: "time", words: vec![
            "time", "today", "yesterday", "tomorrow", "now", "then", "before",
            "after", "during", "while", "morning", "evening", "night", "day",
            "week", "month", "year", "hour", "minute", "second", "recent",
            "recently", "soon", "later", "early", "late", "always", "never",
            "often", "sometimes", "usually", "occasionally", "frequently",
        ]},
        ConceptGroup { label: "quantity_measure", words: vec![
            "count", "number", "amount", "quantity", "size", "weight", "length",
            "height", "width", "depth", "volume", "area", "distance", "speed",
            "rate", "percent", "percentage", "ratio", "proportion", "fraction",
            "total", "sum", "average", "maximum", "minimum", "range", "scale",
            "score", "level", "degree", "price", "cost", "value", "budget",
        ]},
        ConceptGroup { label: "location", words: vec![
            "place", "location", "position", "area", "region", "zone", "district",
            "city", "town", "village", "country", "state", "province", "street",
            "address", "building", "room", "office", "home", "house", "apartment",
            "here", "there", "where", "near", "far", "close", "remote", "local",
        ]},
    ]
}

/// Simple embedding backend based on concept-group word vectors.
/// Converts text to a bag-of-concepts vector (one dimension per concept group),
/// weighted by word frequency. Similarity via cosine distance.
///
/// Limitations:
/// - Only English words are mapped (no stemming — "running" ≠ "run")
/// - Concept coverage is limited (~300 words in 8 groups)
/// - No context sensitivity — "bank" (river) = "bank" (finance) if in same group
/// - This is a placeholder for Phase 2.3 (real embeddings via PyO3)
pub struct SimpleEmbeddingBackend {
    /// Precomputed word → concept index lookup for fast embedding.
    word_to_concepts: Vec<(String, usize)>,
    /// Dimension of the embedding vector (number of concept groups).
    dimension: usize,
}

impl SimpleEmbeddingBackend {
    pub fn new() -> Self {
        let groups = concept_groups();
        let mut word_to_concepts: Vec<(String, usize)> = Vec::new();
        for (concept_idx, group) in groups.iter().enumerate() {
            for word in &group.words {
                word_to_concepts.push((word.to_lowercase(), concept_idx));
            }
        }
        let dimension = groups.len();
        SimpleEmbeddingBackend { word_to_concepts, dimension }
    }

    /// Convert text to a bag-of-concepts vector.
    /// Each dimension = count of words from that concept group in the text.
    fn text_to_vector(&self, text: &str) -> Vec<f64> {
        let mut vec = vec![0.0_f64; self.dimension];
        let words = tokenize(text);
        for word in words {
            for (w, concept_idx) in &self.word_to_concepts {
                if w == &word {
                    vec[*concept_idx] += 1.0;
                }
            }
        }
        // Normalize to unit length for cosine similarity
        let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        }
        vec
    }
}

impl EmbeddingBackend for SimpleEmbeddingBackend {
    fn similarity(&self, text_a: &str, text_b: &str) -> f64 {
        let vec_a = self.embed(text_a);
        let vec_b = self.embed(text_b);
        cosine_similarity(&vec_a, &vec_b)
    }

    fn embed(&self, text: &str) -> Vec<f64> {
        self.text_to_vector(text)
    }
}

/// Default factory: creates a SimpleEmbeddingBackend.
/// Phase 2.3 will switch this to PyO3EmbeddingBackend when available.
pub fn create_embedding_backend() -> Box<dyn EmbeddingBackend> {
    Box::new(SimpleEmbeddingBackend::new())
}

/// Tokenize text: lowercase, split on whitespace and punctuation, filter empty.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-9);
    }

    #[test]
    fn test_concept_similarity_food_related() {
        let backend = SimpleEmbeddingBackend::new();
        let sim = backend.similarity("food preferences", "user likes spicy food");
        assert!(sim > 0.0, "food-related texts should have non-zero similarity");
    }

    #[test]
    fn test_concept_similarity_unrelated() {
        let backend = SimpleEmbeddingBackend::new();
        let sim = backend.similarity("food preferences", "server uptime is 99.9");
        assert!(
            sim < 0.5,
            "unrelated texts should have low similarity, got {}",
            sim
        );
    }

    #[test]
    fn test_food_finds_spicy() {
        let backend = SimpleEmbeddingBackend::new();
        let query_sim = backend.similarity("food preferences", "user likes spicy food");
        let noise_sim = backend.similarity("food preferences", "server uptime is 99.9 percent");
        assert!(
            query_sim > noise_sim,
            "food preferences should match spicy food better than server uptime: {} vs {}",
            query_sim,
            noise_sim
        );
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! user's test.");
        assert_eq!(tokens, vec!["hello", "world", "user", "s", "test"]);
    }
}
