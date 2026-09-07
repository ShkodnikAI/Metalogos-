//! BPE (Byte-Pair Encoding) tokenizer — Наряд №195.
//!
//! Implements standard BPE: start with character-level tokens, iteratively
//! merge the most frequent adjacent pair until vocab_size is reached.
//!
//! ## Tie-breaking rule
//!
//! When multiple pairs have the same frequency, the pair whose string
//! representation (`first + second`) sorts first lexicographically wins.
//! This is deterministic and documented — no hidden ordering dependency.
//!
//! ## Determinism
//!
//! The algorithm is fully deterministic: same corpus + vocab_size → same
//! vocab (bitwise identical merges list). No seed needed.

use std::collections::HashMap;
use std::sync::Mutex;

/// Opaque handle to a BPE vocabulary in the global registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BpeVocabId(pub usize);

/// A trained BPE vocabulary.
pub struct BpeVocab {
    /// Merge rules in priority order (first = highest priority).
    pub merges: Vec<(String, String)>,
    /// Token string → token ID mapping.
    pub vocab: HashMap<String, u32>,
    /// Reverse: token ID → token string.
    pub id_to_token: HashMap<u32, String>,
    /// Total vocabulary size (including base character set).
    pub vocab_size: usize,
}

impl std::fmt::Debug for BpeVocab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BpeVocab")
            .field("vocab_size", &self.vocab_size)
            .field("num_merges", &self.merges.len())
            .finish_non_exhaustive()
    }
}

/// Global registry of BPE vocabularies (same pattern as ReflexRegistry).
pub struct BpeRegistry {
    vocabs: Vec<BpeVocab>,
}

impl Default for BpeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BpeRegistry {
    pub fn new() -> Self {
        Self { vocabs: Vec::new() }
    }

    pub fn register(&mut self, vocab: BpeVocab) -> BpeVocabId {
        let id = BpeVocabId(self.vocabs.len());
        self.vocabs.push(vocab);
        id
    }

    pub fn get(&self, id: BpeVocabId) -> Option<&BpeVocab> {
        self.vocabs.get(id.0)
    }

    pub fn len(&self) -> usize {
        self.vocabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vocabs.is_empty()
    }
}

impl std::fmt::Debug for BpeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BpeRegistry({} vocabs)", self.vocabs.len())
    }
}

/// Thread-safe global BPE registry (lazy_static).
pub static BPE_REGISTRY: Mutex<BpeRegistry> = Mutex::new(BpeRegistry { vocabs: Vec::new() });

/// Train a BPE vocabulary on a corpus.
///
/// Algorithm:
///   1. Initialize: each character in the corpus is a separate token.
///   2. Count all adjacent token pairs.
///   3. Merge the most frequent pair (tie-break: lexicographic order of
///      `first + second`).
///   4. Repeat until vocab_size reached or no more pairs.
///
/// Returns the trained BpeVocab.
pub fn train_bpe(corpus: &str, target_vocab_size: usize) -> Result<BpeVocab, String> {
    if corpus.is_empty() {
        return Err("reflex_bpe_train: corpus is empty".to_string());
    }
    if target_vocab_size < 2 {
        return Err(format!(
            "reflex_bpe_train: vocab_size must be >= 2, got {}",
            target_vocab_size
        ));
    }

    // Step 1: Convert corpus to sequence of token strings (one per char).
    // Also build the initial vocab (unique chars).
    let mut tokens: Vec<String> = corpus.chars().map(|c| c.to_string()).collect();
    let mut vocab: HashMap<String, u32> = HashMap::new();
    for token in &tokens {
        if !vocab.contains_key(token) {
            let id = vocab.len() as u32;
            vocab.insert(token.clone(), id);
        }
    }

    // Step 2: Iteratively merge most frequent pairs.
    let mut merges: Vec<(String, String)> = Vec::new();

    while vocab.len() < target_vocab_size {
        // Count adjacent pairs
        let mut pair_counts: HashMap<(String, String), usize> = HashMap::new();
        for window in tokens.windows(2) {
            let pair = (window[0].clone(), window[1].clone());
            *pair_counts.entry(pair).or_insert(0) += 1;
        }

        if pair_counts.is_empty() {
            break; // No more pairs to merge
        }

        // Find most frequent pair.
        // Tie-break: lexicographic order of (first + second) — smaller wins.
        let best_pair = pair_counts
            .iter()
            .max_by(|(pair_a, count_a), (pair_b, count_b)| {
                // First compare by count (descending)
                match count_b.cmp(count_a) {
                    std::cmp::Ordering::Equal => {
                        // Tie-break: lexicographic order of merged string (ascending — smaller wins)
                        let merged_a = format!("{}{}", pair_a.0, pair_a.1);
                        let merged_b = format!("{}{}", pair_b.0, pair_b.1);
                        merged_b.cmp(&merged_a) // Reverse: we want min, max_by picks max
                    }
                    other => other,
                }
            })
            .map(|(pair, _)| pair.clone())
            .ok_or("reflex_bpe_train: no pairs found")?;

        let merged_token = format!("{}{}", best_pair.0, best_pair.1);
        merges.push(best_pair.clone());

        // Add merged token to vocab
        if !vocab.contains_key(&merged_token) {
            let id = vocab.len() as u32;
            vocab.insert(merged_token.clone(), id);
        }

        // Apply merge to the token sequence
        tokens = apply_merge(&tokens, &best_pair, &merged_token);
    }

    // Build reverse mapping (before moving vocab into BpeVocab)
    let id_to_token: HashMap<u32, String> = vocab.iter().map(|(k, v)| (*v, k.clone())).collect();
    let vocab_size = vocab.len();

    Ok(BpeVocab {
        merges,
        vocab,
        id_to_token,
        vocab_size,
    })
}

/// Apply a single merge rule to a token sequence.
fn apply_merge(tokens: &[String], pair: &(String, String), merged: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() && tokens[i] == pair.0 && tokens[i + 1] == pair.1 {
            result.push(merged.to_string());
            i += 2;
        } else {
            result.push(tokens[i].clone());
            i += 1;
        }
    }
    result
}

/// Encode text into BPE token IDs using a trained vocabulary.
///
/// Applies merge rules in priority order to the character sequence.
pub fn encode_bpe(text: &str, vocab: &BpeVocab) -> Result<Vec<u32>, String> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    // Start with one token per character
    let mut tokens: Vec<String> = text.chars().map(|c| c.to_string()).collect();

    // Apply each merge rule in order
    for (first, second) in &vocab.merges {
        let merged = format!("{}{}", first, second);
        // Only merge if the merged token is in the vocab
        if !vocab.vocab.contains_key(&merged) {
            continue;
        }
        tokens = apply_merge(&tokens, &(first.clone(), second.clone()), &merged);
    }

    // Convert tokens to IDs
    let mut ids: Vec<u32> = Vec::with_capacity(tokens.len());
    for token in &tokens {
        match vocab.vocab.get(token) {
            Some(&id) => ids.push(id),
            None => {
                return Err(format!(
                    "reflex_bpe_encode: token '{}' not in vocabulary (vocab_size={})",
                    token, vocab.vocab_size
                ))
            }
        }
    }
    Ok(ids)
}

/// Decode BPE token IDs back to text.
pub fn decode_bpe(ids: &[u32], vocab: &BpeVocab) -> Result<String, String> {
    let mut result = String::new();
    for (i, &id) in ids.iter().enumerate() {
        match vocab.id_to_token.get(&id) {
            Some(token) => result.push_str(token),
            None => {
                return Err(format!(
                    "reflex_bpe_decode: token ID {} at position {} not in vocabulary",
                    id, i
                ))
            }
        }
    }
    Ok(result)
}

/// Serialize a BPE vocab to bytes (for SQLite persistence).
pub fn serialize_vocab(vocab: &BpeVocab) -> Vec<u8> {
    // Format: num_merges (4 bytes LE) + each merge (len_a, a, len_b, b) + vocab entries
    let mut data = Vec::new();

    // Number of merges
    data.extend_from_slice(&(vocab.merges.len() as u32).to_le_bytes());
    for (a, b) in &vocab.merges {
        let a_bytes = a.as_bytes();
        let b_bytes = b.as_bytes();
        data.extend_from_slice(&(a_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(a_bytes);
        data.extend_from_slice(&(b_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(b_bytes);
    }

    // Number of vocab entries
    data.extend_from_slice(&(vocab.vocab.len() as u32).to_le_bytes());
    for (token, &id) in &vocab.vocab {
        let token_bytes = token.as_bytes();
        data.extend_from_slice(&(token_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(token_bytes);
        data.extend_from_slice(&id.to_le_bytes());
    }

    data
}

/// Deserialize a BPE vocab from bytes.
pub fn deserialize_vocab(data: &[u8]) -> Result<BpeVocab, String> {
    if data.len() < 4 {
        return Err("reflex_bpe_load: data too short".to_string());
    }

    let mut offset = 0;

    // Read number of merges
    let num_merges = u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|e| format!("reflex_bpe_load: conversion error: {:?}", e))?,
    ) as usize;
    offset += 4;

    let mut merges: Vec<(String, String)> = Vec::with_capacity(num_merges);
    for _ in 0..num_merges {
        // Read first string
        if offset + 4 > data.len() {
            return Err("reflex_bpe_load: truncated merge first len".to_string());
        }
        let a_len = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|e| format!("reflex_bpe_load: conversion error: {:?}", e))?,
        ) as usize;
        offset += 4;
        if offset + a_len > data.len() {
            return Err("reflex_bpe_load: truncated merge first data".to_string());
        }
        let a = String::from_utf8(data[offset..offset + a_len].to_vec())
            .map_err(|e| format!("reflex_bpe_load: invalid UTF-8 in merge first: {}", e))?;
        offset += a_len;

        // Read second string
        if offset + 4 > data.len() {
            return Err("reflex_bpe_load: truncated merge second len".to_string());
        }
        let b_len = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|e| format!("reflex_bpe_load: conversion error: {:?}", e))?,
        ) as usize;
        offset += 4;
        if offset + b_len > data.len() {
            return Err("reflex_bpe_load: truncated merge second data".to_string());
        }
        let b = String::from_utf8(data[offset..offset + b_len].to_vec())
            .map_err(|e| format!("reflex_bpe_load: invalid UTF-8 in merge second: {}", e))?;
        offset += b_len;

        merges.push((a, b));
    }

    // Read vocab entries
    if offset + 4 > data.len() {
        return Err("reflex_bpe_load: truncated vocab count".to_string());
    }
    let num_vocab = u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|e| format!("reflex_bpe_load: conversion error: {:?}", e))?,
    ) as usize;
    offset += 4;

    let mut vocab: HashMap<String, u32> = HashMap::with_capacity(num_vocab);
    for _ in 0..num_vocab {
        if offset + 4 > data.len() {
            return Err("reflex_bpe_load: truncated vocab token len".to_string());
        }
        let token_len = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|e| format!("reflex_bpe_load: conversion error: {:?}", e))?,
        ) as usize;
        offset += 4;
        if offset + token_len > data.len() {
            return Err("reflex_bpe_load: truncated vocab token data".to_string());
        }
        let token = String::from_utf8(data[offset..offset + token_len].to_vec())
            .map_err(|e| format!("reflex_bpe_load: invalid UTF-8 in token: {}", e))?;
        offset += token_len;

        if offset + 4 > data.len() {
            return Err("reflex_bpe_load: truncated vocab id".to_string());
        }
        let id = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|e| format!("reflex_bpe_load: conversion error: {:?}", e))?,
        );
        offset += 4;

        vocab.insert(token, id);
    }

    let id_to_token: HashMap<u32, String> = vocab.iter().map(|(k, v)| (*v, k.clone())).collect();
    let vocab_size = vocab.len();

    Ok(BpeVocab {
        merges,
        vocab,
        id_to_token,
        vocab_size,
    })
}
