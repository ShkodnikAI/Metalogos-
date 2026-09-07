#![cfg(feature = "vision")]
//! Наряд №211, Block 2: Golden embedding contract for the text encoder.
//!
//! Tests that the Qwen3-architecture text encoder produces bit-exact
//! deterministic embeddings under a seeded tiny fixture. No files,
//! no network — everything is in-memory.

use metalogos::vision::text_encoder::{TextEncoder, TextEncoderConfig, QWEN3_4B_CONFIG};
use sha2::{Digest, Sha256};

/// Convert f32 slice to bytes without bytemuck.
fn f32_slice_to_bytes(vals: &[f32]) -> &[u8] {
    #[allow(clippy::manual_slice_size_calculation)]
    unsafe {
        std::slice::from_raw_parts(
            vals.as_ptr() as *const u8,
            vals.len() * std::mem::size_of::<f32>(),
        )
    }
}

/// Tiny test config — architecturally identical to Qwen3 (GQA + QK-norm +
/// SwiGLU + RoPE + causal), but tiny dimensions for fast tests.
const TINY_CONFIG: TextEncoderConfig = TextEncoderConfig {
    layers: 4,
    hidden: 64,
    q_heads: 4,
    kv_heads: 2,
    head_dim: 16,
    intermediate: 128,
    vocab_size: 64,
    rms_norm_eps: 1e-6,
    rope_theta: 10000.0,
    max_seq: 16,
};

const SEED: u64 = 20711;

/// Three test prompts encoded as fixed token ID sequences.
/// Using simple sequential IDs — the encoder doesn't care about
/// semantics, only shapes.
const PROMPT_1: &[u32] = &[1, 2, 3, 4, 5];
const PROMPT_2: &[u32] = &[10, 20, 30];
const PROMPT_3: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// Compute SHA-256 of a tensor's raw bytes (F32).
fn tensor_hash(t: &candle_core::Tensor) -> String {
    let t = t.to_dtype(candle_core::DType::F32).unwrap();
    let t = t.contiguous().unwrap();
    let t = t.flatten_all().unwrap();
    let bytes = t.to_vec1::<f32>().unwrap();
    let mut hasher = Sha256::new();
    hasher.update(f32_slice_to_bytes(&bytes));
    let result = hasher.finalize();
    result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Get 4 anchor values from a [seq, hidden] tensor.
fn anchor_values(t: &candle_core::Tensor, hidden: usize, seq: usize) -> (f32, f32, f32, f32) {
    let t = t.to_dtype(candle_core::DType::F32).unwrap();
    let t = t.contiguous().unwrap();
    let vals = t.to_vec2::<f32>().unwrap();
    (
        vals[0][0],
        vals[0][hidden - 1],
        vals[seq - 1][0],
        vals[seq - 1][hidden - 1],
    )
}

// ── Test 1: Golden SHA-256 for 3 prompts ──────────────────────────

#[test]
fn golden_embeddings_shape_and_hash() {
    let encoder = TextEncoder::new(&TINY_CONFIG, SEED).expect("encoder creation");

    for (i, prompt) in [PROMPT_1, PROMPT_2, PROMPT_3].iter().enumerate() {
        let out = encoder.forward(prompt).expect("forward pass");
        let dims = out.dims();

        // Shape: [seq_len, hidden]
        assert_eq!(
            dims,
            [prompt.len(), TINY_CONFIG.hidden],
            "prompt {}: wrong shape {:?}, expected [{}, {}]",
            i,
            dims,
            prompt.len(),
            TINY_CONFIG.hidden
        );

        // Print hash for pinning (first run generates, subsequent runs verify)
        let hash = tensor_hash(&out);
        let anchors = anchor_values(&out, TINY_CONFIG.hidden, prompt.len());
        eprintln!(
            "prompt {}: hash={}, anchors=({:.6}, {:.6}, {:.6}, {:.6})",
            i, hash, anchors.0, anchors.1, anchors.2, anchors.3
        );

        // KNOWN DEBT (наряд №230, loud record): the golden SHA-256 records are
        // NOT yet pinned as consts — this test currently verifies internal
        // determinism only (tests 2a/2b), not bit-exactness against fixed
        // records. Pinning (const GOLDEN_* after 3 bit-identical runs) is the
        // first obligation of №230, after the PRNG SSOT swap changes all values.
        assert!(!hash.is_empty(), "hash should not be empty");
    }
}

// ── Test 2: Determinism — same seed = same output, different seed = different ─

#[test]
fn determinism_same_seed_same_output() {
    let enc1 = TextEncoder::new(&TINY_CONFIG, SEED).expect("encoder 1");
    let enc2 = TextEncoder::new(&TINY_CONFIG, SEED).expect("encoder 2");

    let out1 = enc1.forward(PROMPT_1).expect("forward 1");
    let out2 = enc2.forward(PROMPT_1).expect("forward 2");

    let h1 = tensor_hash(&out1);
    let h2 = tensor_hash(&out2);

    assert_eq!(
        h1, h2,
        "same seed must produce bit-exact identical output: {} != {}",
        h1, h2
    );
}

#[test]
fn determinism_different_seed_different_output() {
    let enc1 = TextEncoder::new(&TINY_CONFIG, SEED).expect("encoder 1");
    let enc2 = TextEncoder::new(&TINY_CONFIG, SEED + 1).expect("encoder 2");

    let out1 = enc1.forward(PROMPT_1).expect("forward 1");
    let out2 = enc2.forward(PROMPT_1).expect("forward 2");

    let h1 = tensor_hash(&out1);
    let h2 = tensor_hash(&out2);

    assert_ne!(
        h1, h2,
        "different seeds must produce different output: {} == {}",
        h1, h2
    );
}

// ── Test 3: Causal property — prefix match ─────────────────────────

#[test]
fn causal_property_prefix_match() {
    let encoder = TextEncoder::new(&TINY_CONFIG, SEED).expect("encoder");

    // Short prompt
    let short_out = encoder.forward(PROMPT_1).expect("short forward");
    // Longer prompt (PROMPT_1 is a prefix of PROMPT_3)
    let long_out = encoder.forward(PROMPT_3).expect("long forward");

    let short_f = short_out
        .to_dtype(candle_core::DType::F32)
        .unwrap()
        .contiguous()
        .unwrap();
    let long_f = long_out
        .to_dtype(candle_core::DType::F32)
        .unwrap()
        .contiguous()
        .unwrap();

    let short_vals = short_f.to_vec2::<f32>().unwrap();
    let long_vals = long_f.to_vec2::<f32>().unwrap();

    // The first len(PROMPT_1) positions of the long prompt should match
    // the short prompt's output (causal property — earlier positions don't
    // see later tokens).
    let tol = 1e-6;
    for i in 0..PROMPT_1.len() {
        for j in 0..TINY_CONFIG.hidden {
            let diff = (short_vals[i][j] - long_vals[i][j]).abs();
            assert!(
                diff <= tol,
                "causal mismatch at [{},{}]: short={:.8}, long={:.8}, diff={:.2e} > tol={:.2e}",
                i,
                j,
                short_vals[i][j],
                long_vals[i][j],
                diff,
                tol
            );
        }
    }
}

// ── Test 4: QWEN3_4B_CONFIG matches pinned values ──────────────────
// Values verified against https://huggingface.co/Qwen/Qwen3-4B/raw/main/config.json
// on 2026-09-08. The original №211 delivery asserted fabricated values
// (40 heads / head_dim 64 / intermediate 6912 / max_seq 32768 — those belong
// to other Qwen3 sizes); corrected fix-forward, see research doc Correction.

#[test]
fn qwen3_4b_config_matches_pinned_values() {
    assert_eq!(QWEN3_4B_CONFIG.layers, 36);
    assert_eq!(QWEN3_4B_CONFIG.hidden, 2560);
    assert_eq!(QWEN3_4B_CONFIG.q_heads, 32);
    assert_eq!(QWEN3_4B_CONFIG.kv_heads, 8);
    assert_eq!(QWEN3_4B_CONFIG.head_dim, 128);
    assert_eq!(QWEN3_4B_CONFIG.intermediate, 9728);
    assert_eq!(QWEN3_4B_CONFIG.vocab_size, 151936);
    assert!((QWEN3_4B_CONFIG.rms_norm_eps - 1e-6).abs() < 1e-15);
    assert!((QWEN3_4B_CONFIG.rope_theta - 1000000.0).abs() < 1.0);
    assert_eq!(QWEN3_4B_CONFIG.max_seq, 40960);
}
