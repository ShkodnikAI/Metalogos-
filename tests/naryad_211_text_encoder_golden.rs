#![cfg(feature = "vision")]
//! Наряд №211, Block 2: Golden embedding contract for the text encoder.
//!
//! Tests that the Qwen3-architecture text encoder produces bit-exact
//! deterministic embeddings under a seeded tiny fixture. No files,
//! no network — everything is in-memory.
//!
//! Наряд №230: PRNG SSOT via `crate::nn::attention::generate_uniform_f32` +
//! splitmix64 per-parameter derivation. Golden records pinned after 3
//! bit-identical local runs (2026-09-08).

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

// ── Pinned golden records (naryad №230) ──────────────────────────────
//
// SHA-256 of the raw F32 bytes of the encoder output `[seq, hidden]`
// tensor for each prompt. Anchor values are the 4 corner f32 elements
// (stored as `to_bits()` so the assertion is integer-exact, immune to
// float-printing drift).
//
// Pinned after 3 bit-identical local runs on 2026-09-08 against the
// PRNG SSOT + splitmix64 per-parameter derivation (Block 1). Any change
// to `forward()` math, dtype policy, or the param_seed/SSOT-PRNG
// contract MUST re-pin these via the Block 2 procedure.
//
// P1 (seq=5, hidden=64) anchors:
//   [0][0]=-0.58273077, [0][63]=-1.3008742, [4][0]=-0.17396767, [4][63]=-0.22194684
// P2 (seq=3, hidden=64) anchors:
//   [0][0]= 0.825586,    [0][63]= 0.364239,  [2][0]=-1.2400694, [2][63]= 1.365883
// P3 (seq=10, hidden=64) anchors:
//   [0][0]=-0.58273077, [0][63]=-1.3008742, [9][0]= 1.951048,   [9][63]=-0.231560
// (decimal forms above are informational; bits below are the contract.)

const GOLDEN_HASH_P1: &str = "163b87c0dd6caa9270a4c4c57536060b0220c766956ac49ca72106aef8465e4b";
const GOLDEN_HASH_P2: &str = "4979c2f95e6208ecde097c84f8aaeb4e6eae36d9c2d761ab4429513727262557";
const GOLDEN_HASH_P3: &str = "7f76c86dc4035db6635c6e870a653eab01201778aec29ca15f01e845cfd278c1";

const GOLDEN_ANCHOR_BITS_P1: [u32; 4] = [3205836248, 3215360780, 3190957205, 3194177032];
const GOLDEN_ANCHOR_BITS_P2: [u32; 4] = [1062427040, 1052409228, 3214850705, 1068422463];
const GOLDEN_ANCHOR_BITS_P3: [u32; 4] = [3205836242, 3215360781, 1073331188, 3194822133];

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

// ── Test 1: Golden SHA-256 + anchor bits for 3 prompts (pinned, naryad №230) ──

#[test]
fn golden_embeddings_shape_and_hash() {
    let encoder = TextEncoder::new(&TINY_CONFIG, SEED).expect("encoder creation");

    let pinned_hashes = [GOLDEN_HASH_P1, GOLDEN_HASH_P2, GOLDEN_HASH_P3];
    let pinned_anchor_bits = [
        GOLDEN_ANCHOR_BITS_P1,
        GOLDEN_ANCHOR_BITS_P2,
        GOLDEN_ANCHOR_BITS_P3,
    ];

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

        let hash = tensor_hash(&out);
        let anchors = anchor_values(&out, TINY_CONFIG.hidden, prompt.len());

        // Pinning observability — keep this eprintln so that re-pinning
        // (after a deliberate contract change) is one `--nocapture` away.
        // Bits are printed so re-pinning doesn't depend on float-print
        // rounding (the {:.6} decimal form is NOT round-trip exact for f32).
        eprintln!(
            "prompt {}: hash={}, anchors=({:.6}, {:.6}, {:.6}, {:.6}) bits=[{:?}, {:?}, {:?}, {:?}]",
            i,
            hash,
            anchors.0,
            anchors.1,
            anchors.2,
            anchors.3,
            anchors.0.to_bits(),
            anchors.1.to_bits(),
            anchors.2.to_bits(),
            anchors.3.to_bits(),
        );

        // Hash contract — bit-exact pinned record (naryad №230).
        assert_eq!(
            hash, pinned_hashes[i],
            "prompt {}: hash mismatch — golden record drifted. Expected {}, got {}.\n\
             If this is a deliberate contract change (forward math / dtype / PRNG), \
             re-pin via the Block 2 procedure after 3 bit-identical runs.",
            i, pinned_hashes[i], hash
        );

        // Anchor bits — integer-exact (f32::to_bits), immune to print drift.
        let actual_bits = [
            anchors.0.to_bits(),
            anchors.1.to_bits(),
            anchors.2.to_bits(),
            anchors.3.to_bits(),
        ];
        assert_eq!(
            actual_bits,
            pinned_anchor_bits[i],
            "prompt {}: anchor bits mismatch — golden record drifted.\n\
             Expected {:?}, got {:?}.\n\
             Decimals: expected {:?}, got {:?}",
            i,
            pinned_anchor_bits[i],
            actual_bits,
            pinned_anchor_bits[i].map(f32::from_bits),
            actual_bits.map(f32::from_bits),
        );
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

// ── Test 5: param_seed derivation (naryad №230 Block 1.6) ──────────
// Verifies that the per-parameter seed derivation produces pairwise-distinct
// seeds across the full (layer, param) grid used by the encoder, and that
// the derivation is NOT the identity function.

#[test]
fn param_seed_derivation_is_pairwise_distinct() {
    // Use the same master seed as the golden test — this is the seed that
    // actually flows through TextEncoder::new → param_seed → generate_uniform_f32.
    const MASTER: u64 = 20711;

    // Per naryad №230 Block 1.6: 4 layers × 8 params (0..8 to leave room for
    // the embedding slot at layer 0, plus the 7 weight tensors per layer).
    let mut seeds: Vec<u64> = Vec::with_capacity(4 * 8);
    for layer in 0..4u64 {
        for param in 0..8u64 {
            // Use the public-facing derivation formula (Block 1.2 spec).
            // TextEncoder's own param_seed is private — duplicating it here
            // tests the exact formula, not the symbol.
            let mut z = MASTER
                ^ 0x9E3779B97F4A7C15
                ^ layer.wrapping_mul(0xBF58476D1CE4E5B9)
                ^ param.wrapping_mul(0x94D049BB133111EB);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            let s = z ^ (z >> 31);
            seeds.push(s);
        }
    }
    assert_eq!(seeds.len(), 32);

    // Pairwise distinct — sort and compare neighbours.
    let mut sorted = seeds.clone();
    sorted.sort_unstable();
    for i in 0..sorted.len() - 1 {
        assert_ne!(
            sorted[i],
            sorted[i + 1],
            "param_seed collision: layer/param {:?} vs {:?}",
            // Recover (layer, param) by re-finding indices — but easier:
            // just report the two colliding values.
            sorted[i],
            sorted[i + 1]
        );
    }

    // Non-identity: param_seed(20711, 0, 1) != 20711.
    let mut z = MASTER
        ^ 0x9E3779B97F4A7C15
        ^ 0u64.wrapping_mul(0xBF58476D1CE4E5B9)
        ^ 1u64.wrapping_mul(0x94D049BB133111EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    let s01 = z ^ (z >> 31);
    assert_ne!(
        s01, MASTER,
        "param_seed(master, 0, 1) must not equal master (would imply no derivation)"
    );
}
