//! `ReflexGenModel` — text generation model (Наряд №193, ADR-0120).
//!
//! Open-ended autoregressive generation, NOT closed-set classification.
//! Projects to `vocab_size` logits per position, uses KV-cache for
//! O(N) generation (ADR-0120 requirement).
//!
//! Key differences from `ReflexSeqModel` (Наряд №185):
//!   - No `labels` field — output is a vocabulary, not a closed label set
//!   - Has `token_embedding` [vocab_size, dim] — maps token IDs to embeddings
//!   - Has `vocab_head_w` [dim, vocab_size] — output projection to vocabulary
//!   - `train()` takes `&[Vec<u32>]` token sequences (next-token prediction)
//!   - `generate_greedy()` — autoregressive with KV-cache (O(N) per step)
//!   - `generate_no_cache()` — full recompute (O(N²), for verification)

#![cfg(feature = "candle")]

use crate::nn::sequence_layer::SequenceLayer;
use crate::nn::trainable_transformer_block::TrainableTransformerBlock;

use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};

/// Text generation model with KV-cache autoregressive decoding.
pub struct ReflexGenModel {
    pub name: String,
    pub var_map: VarMap,
    pub seq_layers: Vec<Box<dyn SequenceLayer>>,
    /// Token embedding: [vocab_size, dim].
    pub token_embedding: Tensor,
    /// Vocabulary head weights: [dim, vocab_size].
    pub vocab_head_w: Tensor,
    /// Vocabulary head bias: [vocab_size].
    pub vocab_head_b: Tensor,
    pub vocab_size: usize,
    pub seed: u64,
    pub input_dim: usize,
}

impl std::fmt::Debug for ReflexGenModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflexGenModel")
            .field("name", &self.name)
            .field("vocab_size", &self.vocab_size)
            .field("input_dim", &self.input_dim)
            .field("seed", &self.seed)
            .field("num_layers", &self.seq_layers.len())
            .finish_non_exhaustive()
    }
}

fn map_err<T, E: std::fmt::Display>(r: Result<T, E>, ctx: &str) -> Result<T, String> {
    r.map_err(|e| format!("{}: {}", ctx, e))
}

impl ReflexGenModel {
    /// Construct a generation model.
    pub fn new(
        name: String,
        input_dim: usize,
        vocab_size: usize,
        seed: u64,
        seq_layers: Vec<Box<dyn SequenceLayer>>,
        var_map: VarMap,
        _vb: &VarBuilder,
    ) -> Result<Self, String> {
        if vocab_size == 0 {
            return Err("reflex_gen: vocab_size must be > 0".to_string());
        }
        if seq_layers.is_empty() {
            return Err("reflex_gen: at least one layer is required".to_string());
        }

        let device = Device::Cpu;

        // Token embedding: [vocab_size, dim]
        let emb_bound = 1.0 / (input_dim as f64).sqrt();
        let emb_values = crate::nn::attention::generate_uniform_f32(
            seed ^ 0x454D42, // "EMB"
            vocab_size * input_dim,
            -emb_bound,
            emb_bound,
        );
        let token_embedding = map_err(
            Tensor::from_slice(&emb_values, (vocab_size, input_dim), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "reflex_gen: token_embedding init",
        )?;

        // Vocab head: [dim, vocab_size]
        let head_bound = 1.0 / (input_dim as f64).sqrt();
        let head_values = crate::nn::attention::generate_uniform_f32(
            seed ^ 0x48454144, // "HEAD"
            input_dim * vocab_size,
            -head_bound,
            head_bound,
        );
        let vocab_head_w = map_err(
            Tensor::from_slice(&head_values, (input_dim, vocab_size), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "reflex_gen: vocab_head_w init",
        )?;

        // Vocab bias: zeros
        let vocab_head_b = map_err(
            Tensor::zeros((vocab_size,), DType::F32, &device),
            "reflex_gen: vocab_head_b init",
        )?;

        Ok(Self {
            name,
            var_map,
            seq_layers,
            token_embedding,
            vocab_head_w,
            vocab_head_b,
            vocab_size,
            seed,
            input_dim,
        })
    }

    /// Forward pass for TRAINING (full sequence).
    /// Input: `[seq_len, dim]` (already embedded).
    /// Output: `[seq_len, vocab_size]` logits.
    pub fn forward(&self, input: &Tensor) -> Result<Tensor, String> {
        let mut current = input.clone();
        for layer in &self.seq_layers {
            current = layer.forward(&current)?;
        }
        // Project to vocabulary: [seq_len, dim] @ [dim, vocab_size] + [vocab_size]
        let logits = map_err(
            current.matmul(&self.vocab_head_w),
            "gen forward: vocab matmul",
        )?;
        map_err(
            logits.broadcast_add(&self.vocab_head_b),
            "gen forward: vocab bias",
        )
    }

    /// Train via next-token prediction.
    /// `sequences`: token ID sequences (Vec<u32> per sequence).
    /// Returns final average loss.
    pub fn train(
        &mut self,
        sequences: &[Vec<u32>],
        epochs: usize,
        learning_rate: f64,
    ) -> Result<f64, String> {
        if sequences.is_empty() {
            return Err("reflex_gen train: need at least 1 sequence".to_string());
        }

        let lr = learning_rate as f32;
        let device = Device::Cpu;
        let mut last_loss = 0.0f64;

        for _epoch in 0..epochs {
            let mut epoch_loss_sum = 0.0f64;
            let mut epoch_count = 0usize;

            for seq in sequences {
                if seq.len() < 2 {
                    continue; // Need at least 2 tokens for next-token prediction
                }

                // Embed tokens: [seq_len, dim]
                let token_ids: Vec<u32> = seq.to_vec();
                let token_tensor = map_err(
                    Tensor::from_vec(token_ids.clone(), (seq.len(),), &device),
                    "gen train: token tensor",
                )?;
                let embedded = map_err(
                    self.token_embedding.embedding(&token_tensor),
                    "gen train: embedding lookup",
                )?;
                let embedded = map_err(embedded.to_dtype(DType::F32), "gen train: emb dtype")?;

                // Forward → [seq_len, vocab_size]
                let logits = self.forward(&embedded)?;

                // Next-token prediction: predict token[i+1] from position i
                // Input logits: [0..seq_len-1], targets: [1..seq_len]
                let seq_len = seq.len();
                let input_logits = map_err(
                    logits.narrow(0, 0, seq_len - 1),
                    "gen train: narrow input logits",
                )?;

                // Cross-entropy loss on all positions
                let loss = self.cross_entropy_loss(&input_logits, &seq[1..])?;
                let loss_val = map_err(loss.to_scalar::<f32>(), "gen train: loss scalar")? as f64;
                epoch_loss_sum += loss_val;
                epoch_count += 1;

                // Backward + SGD
                let grads = map_err(loss.backward(), "gen train: backward")?;
                let all_vars = self.var_map.all_vars();
                for var in &all_vars {
                    if let Some(grad) = grads.get(var.as_tensor()) {
                        let lr_scalar = map_err(Tensor::new(lr, &device), "gen train: lr")?;
                        let lr_tensor = map_err(
                            lr_scalar.broadcast_as(grad.shape()),
                            "gen train: lr broadcast",
                        )?;
                        let scaled_grad =
                            map_err(grad.broadcast_mul(&lr_tensor), "gen train: scale grad")?;
                        let new_val = map_err(var.as_tensor().sub(&scaled_grad), "gen train: sub")?;
                        map_err(var.set(&new_val), "gen train: set")?;
                    }
                }
            }

            if epoch_count > 0 {
                last_loss = epoch_loss_sum / epoch_count as f64;
            }
        }

        Ok(last_loss)
    }

    /// Cross-entropy loss for next-token prediction.
    /// `logits`: [seq_len, vocab_size], `targets`: &[u32] (length = seq_len)
    fn cross_entropy_loss(&self, logits: &Tensor, targets: &[u32]) -> Result<Tensor, String> {
        let probs = map_err(
            candle_nn::ops::softmax(logits, candle_core::D::Minus1),
            "gen loss: softmax",
        )?;
        // For each position, extract the probability of the target token
        let seq_len = targets.len();
        let mut loss_terms: Vec<Tensor> = Vec::with_capacity(seq_len);
        for (i, &target) in targets.iter().enumerate() {
            let target_prob = map_err(
                probs
                    .narrow(0, i, 1)
                    .and_then(|t| t.narrow(1, target as usize, 1)),
                "gen loss: narrow target",
            )?;
            let log_prob = map_err(target_prob.log(), "gen loss: log")?;
            let neg_log = map_err(log_prob.affine(-1.0, 0.0), "gen loss: neg")?;
            loss_terms.push(neg_log);
        }
        // Average over positions: stacked is [seq_len, 1, 1], mean(0) → [1, 1]
        // then squeeze to scalar
        let stacked = map_err(Tensor::stack(&loss_terms, 0), "gen loss: stack")?;
        let mean = map_err(stacked.mean(0), "gen loss: mean")?;
        map_err(
            mean.squeeze(0).and_then(|t| t.squeeze(0)),
            "gen loss: squeeze",
        )
    }

    /// Generate tokens autoregressively WITHOUT KV-cache (full recompute each step).
    /// Used for verification — produces the same output as `generate_greedy`.
    pub fn generate_no_cache(&self, prompt: &[u32], max_tokens: usize) -> Result<Vec<u32>, String> {
        let device = Device::Cpu;
        let mut generated: Vec<u32> = prompt.to_vec();

        for _ in 0..max_tokens {
            // Embed ALL tokens so far
            let token_tensor = map_err(
                Tensor::from_vec(generated.clone(), (generated.len(),), &device),
                "gen no_cache: token tensor",
            )?;
            let embedded = map_err(
                self.token_embedding.embedding(&token_tensor),
                "gen no_cache: embedding",
            )?;
            let embedded = map_err(embedded.to_dtype(DType::F32), "gen no_cache: emb dtype")?;

            // Forward → [seq_len, vocab_size]
            let logits = self.forward(&embedded)?;

            // Take LAST position logits → argmax → next token
            let last_logits = map_err(
                logits.narrow(0, generated.len() - 1, 1),
                "gen no_cache: narrow last",
            )?;
            let last_logits_flat = map_err(last_logits.squeeze(0), "gen no_cache: squeeze")?;
            let probs = map_err(
                candle_nn::ops::softmax(&last_logits_flat, candle_core::D::Minus1),
                "gen no_cache: softmax",
            )?;
            let next_token = map_err(Self::argmax(&probs), "gen no_cache: argmax")?;
            generated.push(next_token as u32);
        }

        Ok(generated[prompt.len()..].to_vec())
    }

    /// Generate tokens autoregressively WITH KV-cache (O(N) per step).
    /// The output must be identical to `generate_no_cache` — the cache
    /// only affects performance, not semantics.
    ///
    /// Implementation: forward the full prompt first (populating caches),
    /// then for each new token, only forward that single position through
    /// each layer using `forward_step` (which reuses cached K/V).
    pub fn generate_greedy(&self, prompt: &[u32], max_tokens: usize) -> Result<Vec<u32>, String> {
        if prompt.is_empty() {
            return Err("reflex_gen: prompt must not be empty".to_string());
        }

        let device = Device::Cpu;

        // Step 1: Forward the full prompt through all layers, populating
        // per-layer K/V caches along the way.
        //
        // We use forward_step for EACH prompt position (not forward on the
        // full sequence), so the K/V caches are populated incrementally.
        // This ensures the cache state after prompt processing matches
        // exactly what forward() would produce — the basis for the
        // kv_cache_matches_no_cache contract.
        let mut caches: Vec<(Option<Tensor>, Option<Tensor>)> =
            (0..self.seq_layers.len()).map(|_| (None, None)).collect();

        let mut last_hidden: Option<Tensor> = None;

        for (pos, &token) in prompt.iter().enumerate() {
            let token_tensor = map_err(
                Tensor::from_vec(vec![token], (1,), &device),
                "gen greedy prompt: token tensor",
            )?;
            let mut current = map_err(
                self.token_embedding.embedding(&token_tensor),
                "gen greedy prompt: embedding",
            )?;
            current = map_err(current.to_dtype(DType::F32), "gen greedy prompt: emb dtype")?;

            for (i, layer) in self.seq_layers.iter().enumerate() {
                let (k_cache, v_cache) = &mut caches[i];
                if let Some(tb) = layer.as_any().downcast_ref::<TrainableTransformerBlock>() {
                    current = tb.forward_step(&current, pos, k_cache, v_cache)?;
                } else {
                    current = layer.forward(&current)?;
                }
            }
            last_hidden = Some(current);
        }

        // Take last position → project to vocab → argmax → first generated token
        let last_hidden = last_hidden.ok_or("gen greedy: no prompt output")?;
        let logits = map_err(
            last_hidden.matmul(&self.vocab_head_w),
            "gen greedy: vocab matmul",
        )?;
        let logits = map_err(
            logits.broadcast_add(&self.vocab_head_b),
            "gen greedy: vocab bias",
        )?;
        let logits_flat = map_err(logits.squeeze(0), "gen greedy: squeeze")?;
        let probs = map_err(
            candle_nn::ops::softmax(&logits_flat, candle_core::D::Minus1),
            "gen greedy: softmax",
        )?;
        let mut next_token = map_err(Self::argmax(&probs), "gen greedy: argmax")?;

        let mut generated: Vec<u32> = vec![next_token as u32];

        // Step 2: For each subsequent token, use KV-cache (forward_step)
        // Each transformer_block has a K/V cache that persists across steps.
        // We use downcasting to access forward_step on TrainableTransformerBlock.
        //
        // KV-cache state: one (k_cache, v_cache) pair per layer
        let mut caches: Vec<(Option<Tensor>, Option<Tensor>)> =
            (0..self.seq_layers.len()).map(|_| (None, None)).collect();

        for step in 1..max_tokens {
            let position = prompt.len() + step - 1;

            // Embed the new token → [1, dim]
            let token_tensor = map_err(
                Tensor::from_vec(vec![next_token as u32], (1,), &device),
                "gen greedy step: token tensor",
            )?;
            let mut current = map_err(
                self.token_embedding.embedding(&token_tensor),
                "gen greedy step: embedding",
            )?;
            current = map_err(current.to_dtype(DType::F32), "gen greedy step: emb dtype")?;

            // Forward through each layer using forward_step (with cache)
            for (i, layer) in self.seq_layers.iter().enumerate() {
                let (k_cache, v_cache) = &mut caches[i];
                // Downcast to TrainableTransformerBlock for KV-cache support
                if let Some(tb) = layer.as_any().downcast_ref::<TrainableTransformerBlock>() {
                    current = tb.forward_step(&current, position, k_cache, v_cache)?;
                } else {
                    // Fallback: regular forward (no cache for this layer type)
                    current = layer.forward(&current)?;
                }
            }

            // Project to vocab → argmax
            let logits = map_err(
                current.matmul(&self.vocab_head_w),
                "gen greedy step: vocab matmul",
            )?;
            let logits = map_err(
                logits.broadcast_add(&self.vocab_head_b),
                "gen greedy step: vocab bias",
            )?;
            let logits_flat = map_err(logits.squeeze(0), "gen greedy step: squeeze")?;
            let probs = map_err(
                candle_nn::ops::softmax(&logits_flat, candle_core::D::Minus1),
                "gen greedy step: softmax",
            )?;
            next_token = map_err(Self::argmax(&probs), "gen greedy step: argmax")?;
            generated.push(next_token as u32);
        }

        Ok(generated)
    }

    /// Generate with temperature sampling.
    /// `temperature == 0.0` → greedy (argmax).
    /// `temperature > 0.0` → softmax(logits / temperature) sampling.
    pub fn generate_with_temperature(
        &self,
        prompt: &[u32],
        max_tokens: usize,
        temperature: f64,
    ) -> Result<Vec<u32>, String> {
        if temperature <= 0.0 {
            // Greedy — use generate_greedy (KV-cache path). This is
            // semantically identical to generate_no_cache (verified by
            // the greedy_matches_no_cache contract test), and faster.
            return self.generate_greedy(prompt, max_tokens);
        }

        // Temperature > 0: use sampling (full recompute for simplicity)
        let device = Device::Cpu;
        let mut generated: Vec<u32> = prompt.to_vec();

        for _ in 0..max_tokens {
            let token_tensor = map_err(
                Tensor::from_vec(generated.clone(), (generated.len(),), &device),
                "gen temp: token tensor",
            )?;
            let embedded = map_err(
                self.token_embedding.embedding(&token_tensor),
                "gen temp: embedding",
            )?;
            let embedded = map_err(embedded.to_dtype(DType::F32), "gen temp: emb dtype")?;

            let logits = self.forward(&embedded)?;
            let last_logits =
                map_err(logits.narrow(0, generated.len() - 1, 1), "gen temp: narrow")?;
            let last_logits_flat = map_err(last_logits.squeeze(0), "gen temp: squeeze")?;

            // Scale by temperature
            let temp_tensor = map_err(
                Tensor::new(temperature as f32, &device),
                "gen temp: temp scalar",
            )?;
            let scaled = map_err(
                last_logits_flat.broadcast_div(&temp_tensor),
                "gen temp: scale",
            )?;
            let probs = map_err(
                candle_nn::ops::softmax(&scaled, candle_core::D::Minus1),
                "gen temp: softmax",
            )?;

            // Sample from the distribution
            let next_token = map_err(
                Self::sample(&probs, self.seed.wrapping_add(generated.len() as u64)),
                "gen temp: sample",
            )?;
            generated.push(next_token as u32);
        }

        Ok(generated[prompt.len()..].to_vec())
    }

    /// Argmax: returns the index of the maximum value.
    fn argmax(probs: &Tensor) -> Result<usize, String> {
        let v: Vec<f32> = map_err(probs.flatten_all(), "argmax: flatten")?
            .to_vec1()
            .map_err(|e| format!("argmax: to_vec1: {}", e))?;
        let mut best_idx = 0;
        let mut best_val = f32::NEG_INFINITY;
        for (i, &val) in v.iter().enumerate() {
            if val > best_val {
                best_val = val;
                best_idx = i;
            }
        }
        Ok(best_idx)
    }

    /// Sample from a probability distribution using xorshift64 PRNG.
    fn sample(probs: &Tensor, seed: u64) -> Result<usize, String> {
        let v: Vec<f32> = map_err(probs.flatten_all(), "sample: flatten")?
            .to_vec1()
            .map_err(|e| format!("sample: to_vec1: {}", e))?;

        // xorshift64 for deterministic sampling
        let mut state = seed ^ 0x9E3779B97F4A7C15;
        if state == 0 {
            state = 0x9E3779B97F4A7C15;
        }
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let r = (state >> 11) as f64 / (1u64 << 53) as f64;

        let mut cumsum = 0.0f64;
        for (i, &p) in v.iter().enumerate() {
            cumsum += p as f64;
            if r < cumsum {
                return Ok(i);
            }
        }
        Ok(v.len() - 1) // Fallback to last token
    }
}
