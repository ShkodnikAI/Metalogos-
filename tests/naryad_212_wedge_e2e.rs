#![cfg(feature = "vision")]
//! Наряд №212 — Vision R3 wedge e2e tests.
//!
//! Two-tier test architecture:
//! - **CI-visible:** tiny-fixture goldens (VaeDecoder, ZImageTransformer,
//!   FlowMatchEuler scheduler) — bit-exact, no weights, no network.
//! - **env-gated:** real-weights tests run only when `MLOG_VISION_WEIGHTS_DIR`
//!   is set. Otherwise tests SKIP loudly (not `#[ignore]`).
//!
//! ## Env vars
//!
//! - `MLOG_VISION_WEIGHTS_DIR` — path to the weights directory (see
//!   `docs/research/naryad-212-weights-manifest.md` for the expected layout).
//! - `MLOG_VISION_OUT` (optional, default `target/`) — output directory for
//!   generated PNG files.

// Style nits suppressed — see src/vision/{vae,dit}.rs for rationale.
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(clippy::needless_borrow)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::identity_op)]
#![allow(clippy::erasing_op)]

use std::path::PathBuf;
use std::time::Instant;

use candle_core::{DType, Device, Tensor};
use sha2::{Digest, Sha256};

use metalogos::vision::text_encoder::{TextEncoder, QWEN3_4B_CONFIG};
use metalogos::vision::vae::{fixed_latent, tiny_vae_config, VaeDecoder};
use metalogos::vision::weights::{load_safetensors_sharded, load_safetensors_single};

// ── Env helpers ────────────────────────────────────────────────────

fn weights_dir() -> Option<PathBuf> {
    std::env::var_os("MLOG_VISION_WEIGHTS_DIR").map(PathBuf::from)
}

fn out_dir() -> PathBuf {
    std::env::var_os("MLOG_VISION_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"))
}

fn print_skip(reason: &str) {
    eprintln!(
        "SKIP (loud): {}\n  Set MLOG_VISION_WEIGHTS_DIR to a directory laid out per \
         docs/research/naryad-212-weights-manifest.md to enable this test.\n  \
         (Loud skip per §3 — permitted unfinishedness, not a bare ignore attribute.)",
        reason
    );
}

// ── Helpers (pinning procedure mirrors naryad №230) ─────────────────

fn tensor_sha256(t: &Tensor) -> String {
    let t = t.to_dtype(DType::F32).unwrap();
    let t = t.contiguous().unwrap();
    let t = t.flatten_all().unwrap();
    let bytes = t.to_vec1::<f32>().unwrap();
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            bytes.as_ptr() as *const u8,
            bytes.len() * std::mem::size_of::<f32>(),
        )
    };
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

// ═══════════════════════════════════════════════════════════════════════
// CI-VISIBLE TINY GOLDENS (no env-gate, no weights required)
// ═══════════════════════════════════════════════════════════════════════

// ── Block 3.2: VAE decoder tiny CI golden (3 bit-exact runs) ──────

const VAE_TINY_SEED: u64 = 21230;

// Pinned after 3 bit-identical local runs on 2026-09-08 (procedure mirrors naryad №230).
// n232: re-pinned after VAE ritual direction fix (latent/scaling+shift, not (latent-shift)/scaling).
// Old hash (n212): 3e8c058d7107a19e18ae8287112a8990964e8b30614a498dd61ad18e5f7493ce
const VAE_GOLDEN_HASH: &str = "7f1ac2181af30178945647ceeb748f652c50068b1227ab22647e9c2d3c0992c8";
const VAE_GOLDEN_ANCHOR_BITS: [u32; 4] = [1056952556, 1056974176, 1056900913, 1056960006];

#[test]
fn vae_tiny_decode_golden() {
    let cfg = tiny_vae_config();
    let decoder = VaeDecoder::new_tiny(&cfg, VAE_TINY_SEED).expect("VaeDecoder::new_tiny");
    // Latent: [1, latent_channels, h, w] — tiny: 8×8 input, decoder → 64×64 output.
    let latent = fixed_latent(VAE_TINY_SEED, cfg.latent_channels, 8, 8);

    let img = decoder.decode(&latent).expect("VAE decode");
    let dims = img.dims();
    assert_eq!(dims, &[3, 64, 64], "VAE tiny decode shape: {:?}", dims);

    let hash = tensor_sha256(&img);
    eprintln!("vae_tiny_decode_golden: hash={}", hash);

    // Anchor bits — corner values [0,0], [0,w-1], [h-1,0], [h-1,w-1] of the [3, H, W] image.
    let img_f = img.to_dtype(DType::F32).unwrap().contiguous().unwrap();
    let vals = img_f.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    let h = 64usize;
    let w = 64usize;
    let anchors = [
        vals[0 * h * w + 0 * w + 0],             // [0,0]
        vals[0 * h * w + 0 * w + (w - 1)],       // [0,w-1]
        vals[0 * h * w + (h - 1) * w + 0],       // [h-1,0]
        vals[0 * h * w + (h - 1) * w + (w - 1)], // [h-1,w-1]
    ];
    eprintln!(
        "vae_tiny_decode_golden: anchors=({:.6}, {:.6}, {:.6}, {:.6}) bits=[{:?}, {:?}, {:?}, {:?}]",
        anchors[0],
        anchors[1],
        anchors[2],
        anchors[3],
        anchors[0].to_bits(),
        anchors[1].to_bits(),
        anchors[2].to_bits(),
        anchors[3].to_bits(),
    );

    // Pinning check — bit-exact against pinned records (n232 re-pinned).
    assert_eq!(
        hash, VAE_GOLDEN_HASH,
        "VAE tiny golden hash drifted. Expected {}, got {}.\n\
         If intentional (forward/init change), re-pin after 3 bit-identical runs.",
        VAE_GOLDEN_HASH, hash
    );
    let actual_bits = [
        anchors[0].to_bits(),
        anchors[1].to_bits(),
        anchors[2].to_bits(),
        anchors[3].to_bits(),
    ];
    assert_eq!(
        actual_bits, VAE_GOLDEN_ANCHOR_BITS,
        "VAE tiny golden anchor bits drifted. Expected {:?}, got {:?}",
        VAE_GOLDEN_ANCHOR_BITS, actual_bits
    );
}

#[test]
fn vae_tiny_decode_determinism() {
    let cfg = tiny_vae_config();
    let decoder = VaeDecoder::new_tiny(&cfg, VAE_TINY_SEED).expect("decoder 1");
    let decoder2 = VaeDecoder::new_tiny(&cfg, VAE_TINY_SEED).expect("decoder 2");
    let latent = fixed_latent(VAE_TINY_SEED, cfg.latent_channels, 8, 8);

    let h1 = tensor_sha256(&decoder.decode(&latent).expect("decode 1"));
    let h2 = tensor_sha256(&decoder2.decode(&latent).expect("decode 2"));
    assert_eq!(h1, h2, "same seed → bit-exact identical output");
}

// ── Block 4.3: DiT tiny golden (pinned, naryad №231) ──────────────

use metalogos::nn::attention::generate_uniform_f32;
use metalogos::vision::dit::{tiny_dit_config, ZImageTransformer};
use metalogos::vision::text_encoder::param_seed;

/// Test seed for DiT tiny golden. 99 = test "slot" for latent/cap derivation
/// (does not collide with PARAM_DIT_* 200..240).
const SEED_DIT: u64 = 21201;

// Pinned after 3 bit-identical local runs on 2026-09-08.
// n233: re-pinned after RoPE wire-in (AxialRoPE::apply now called in attention path).
// n232: 211cf4f5... (pre-RoPE architecture)
// n231: e686167b... (pre-rebuild architecture)
const GOLDEN_DIT_TINY_HASH: &str =
    "860c85b311905f6c23b90a4e9e3192928027a24bf3e4a00a08096336abad4b3c";
const GOLDEN_DIT_TINY_ANCHOR_BITS: [u32; 4] = [3164026950, 3162841220, 1013795100, 3189363858];

#[test]
fn dit_tiny_forward_golden() {
    let cfg = tiny_dit_config();
    let dit = ZImageTransformer::new_tiny(&cfg, SEED_DIT).expect("new_tiny");

    // Deterministic inputs via SSOT (no new generator — §3.4).
    // latent [1, 4, 8, 8] — 99 is a test slot, not a PARAM_DIT_* constant.
    let latent_vals = generate_uniform_f32(param_seed(SEED_DIT, 99, 0), 4 * 8 * 8, -1.0, 1.0);
    let latent = Tensor::from_vec(latent_vals, (1, 4, 8, 8), &Device::Cpu).expect("latent");

    // cap [4, 32] — [cap_seq, cap_feat_dim] per DiT forward contract.
    let cap_vals = generate_uniform_f32(param_seed(SEED_DIT, 99, 1), 4 * 32, -1.0, 1.0);
    let cap = Tensor::from_vec(cap_vals, (4, 32), &Device::Cpu).expect("cap");

    let t = 0.375;
    let out = dit.forward(&latent, &cap, t).expect("forward");

    // Shape: [1, in_channels, H, W] = [1, 4, 8, 8] (velocity prediction = latent shape).
    let dims = out.dims();
    assert_eq!(dims, [1, 4, 8, 8], "DiT tiny forward shape: {:?}", dims);

    // Finiteness: 0 NaN/Inf.
    let out_f32 = out.to_dtype(DType::F32).unwrap().contiguous().unwrap();
    let vals = out_f32.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    let non_finite = vals.iter().filter(|v| !v.is_finite()).count();
    assert_eq!(non_finite, 0, "non-finite values: {}", non_finite);

    // Hash + anchor bits (pinning procedure mirrors naryad №230).
    let hash = tensor_sha256(&out);
    let anchors = [vals[0], vals[1], vals[vals.len() - 2], vals[vals.len() - 1]];
    eprintln!(
        "dit_tiny_forward_golden: hash={} anchors=({:.6}, {:.6}, {:.6}, {:.6}) bits=[{}, {}, {}, {}]",
        hash, anchors[0], anchors[1], anchors[2], anchors[3],
        anchors[0].to_bits(), anchors[1].to_bits(), anchors[2].to_bits(), anchors[3].to_bits()
    );

    // Pinning check — bit-exact against pinned records (n233 re-pinned after RoPE wire-in).
    assert_eq!(
        hash, GOLDEN_DIT_TINY_HASH,
        "DiT tiny golden hash drifted. Expected {}, got {}.\n\
         If intentional, re-pin after 3 bit-identical runs.",
        GOLDEN_DIT_TINY_HASH, hash
    );
    let actual_bits = [
        anchors[0].to_bits(),
        anchors[1].to_bits(),
        anchors[2].to_bits(),
        anchors[3].to_bits(),
    ];
    assert_eq!(
        actual_bits, GOLDEN_DIT_TINY_ANCHOR_BITS,
        "DiT tiny golden anchor bits drifted. Expected {:?}, got {:?}",
        GOLDEN_DIT_TINY_ANCHOR_BITS, actual_bits
    );
}

#[test]
fn dit_tiny_seed_determinism() {
    let cfg = tiny_dit_config();

    // Same seed → bit-exact identical.
    let dit1 = ZImageTransformer::new_tiny(&cfg, SEED_DIT).expect("dit1");
    let dit2 = ZImageTransformer::new_tiny(&cfg, SEED_DIT).expect("dit2");

    let latent_vals = generate_uniform_f32(param_seed(SEED_DIT, 99, 0), 4 * 8 * 8, -1.0, 1.0);
    let latent = Tensor::from_vec(latent_vals, (1, 4, 8, 8), &Device::Cpu).expect("latent");
    let cap_vals = generate_uniform_f32(param_seed(SEED_DIT, 99, 1), 4 * 32, -1.0, 1.0);
    let cap = Tensor::from_vec(cap_vals, (4, 32), &Device::Cpu).expect("cap");

    let h1 = tensor_sha256(&dit1.forward(&latent, &cap, 0.375).expect("fwd1"));
    let h2 = tensor_sha256(&dit2.forward(&latent, &cap, 0.375).expect("fwd2"));
    assert_eq!(h1, h2, "same seed → bit-exact identical output");

    // Different model seed → different output.
    let dit3 = ZImageTransformer::new_tiny(&cfg, SEED_DIT + 1).expect("dit3");
    let h3 = tensor_sha256(&dit3.forward(&latent, &cap, 0.375).expect("fwd3"));
    assert_ne!(h1, h3, "different model seed → different output");

    // Different INPUT seed (latent/cap derivation) → different output.
    let latent2_vals = generate_uniform_f32(param_seed(SEED_DIT + 2, 99, 0), 4 * 8 * 8, -1.0, 1.0);
    let latent2 = Tensor::from_vec(latent2_vals, (1, 4, 8, 8), &Device::Cpu).expect("latent2");
    let cap2_vals = generate_uniform_f32(param_seed(SEED_DIT + 2, 99, 1), 4 * 32, -1.0, 1.0);
    let cap2 = Tensor::from_vec(cap2_vals, (4, 32), &Device::Cpu).expect("cap2");
    let h4 = tensor_sha256(&dit1.forward(&latent2, &cap2, 0.375).expect("fwd4"));
    assert_ne!(
        h1, h4,
        "different input seed → different output (inputs affect output)"
    );
}

#[test]
fn dit_tiny_config_contract() {
    let cfg = tiny_dit_config();
    assert_eq!(cfg.dim, 64, "tiny_dit_config dim");
    assert_eq!(cfg.n_layers, 2, "tiny_dit_config n_layers");
    assert_eq!(cfg.in_channels, 4, "tiny_dit_config in_channels");
    assert_eq!(cfg.patch_size, 2, "tiny_dit_config patch_size");
    assert_eq!(cfg.cap_feat_dim, 32, "tiny_dit_config cap_feat_dim");
    assert_eq!(cfg.n_refiner_layers, 1, "tiny_dit_config n_refiner_layers");
    // n232: intermediate = int(dim/3*8) = int(64/3*8) = 170 (was 256=4*dim in №212).
    assert_eq!(
        cfg.intermediate(),
        170,
        "tiny_dit_config intermediate (int(dim/3*8))"
    );
}

#[test]
fn sampler_sigmas_pinned() {
    use metalogos::vision::sampler::flow_match_euler_sigmas;
    // 9 sigmas (8 forward steps), shift=3.0, num_train=1000.
    let sigmas = flow_match_euler_sigmas(9, 3.0, 1000);
    assert_eq!(sigmas.len(), 9, "expected 9 sigmas");

    // First sigma should be the largest (close to 1.0 after shift transform).
    let first = sigmas[0];
    let last = sigmas[8];
    assert!(first > last, "sigmas must be monotonically decreasing");
    assert!(last >= 0.0, "last sigma must be non-negative");

    // Monotonic decrease.
    for i in 0..sigmas.len() - 1 {
        assert!(sigmas[i] > sigmas[i + 1], "non-monotonic at index {}", i);
    }

    // Print for pinning (procedural — mirrors №230 Block 2).
    eprintln!("sampler_sigmas_pinned: sigmas={:?}", sigmas);

    // n232: pinned sigma vector — exact match required.
    const PINNED_SIGMAS: [f64; 9] = [
        1.0,
        0.9549418604651164,
        0.9004796163069546,
        0.8339253996447603,
        0.7507492507492509,
        0.6438356164383562,
        0.5013315579227696,
        0.3019169329073482,
        0.002994011976047907,
    ];
    for (i, (&actual, &pinned)) in sigmas.iter().zip(PINNED_SIGMAS.iter()).enumerate() {
        assert!(
            (actual - pinned).abs() < 1e-15,
            "sigma[{}] mismatch: got {}, expected {}",
            i,
            actual,
            pinned
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ENV-GATED REAL-WEIGHTS TESTS (loud SKIP if MLOG_VISION_WEIGHTS_DIR unset)
// ═══════════════════════════════════════════════════════════════════════

// ── Block 2.2: TextEncoder::from_weights env-gated ─────────────────

#[test]
fn text_encoder_real_weights_forward() {
    let Some(weights_dir) = weights_dir() else {
        print_skip("MLOG_VISION_WEIGHTS_DIR not set — TextEncoder real-weights test");
        return;
    };
    let te_dir = weights_dir.join("text_encoder");
    let t0 = Instant::now();
    let tensors = match load_safetensors_sharded(&te_dir, "model", &Device::Cpu) {
        Ok(t) => t,
        Err(e) => {
            panic!(
                "load_safetensors_sharded(text_encoder) failed: {}\n\
                 Verify MLOG_VISION_WEIGHTS_DIR layout per \
                 docs/research/naryad-212-weights-manifest.md",
                e
            );
        }
    };
    let load_elapsed = t0.elapsed();

    let t0 = Instant::now();
    let encoder = TextEncoder::from_weights(&QWEN3_4B_CONFIG, &tensors)
        .expect("TextEncoder::from_weights failed");
    let build_elapsed = t0.elapsed();
    drop(tensors);

    let tokens: Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let t0 = Instant::now();
    let out = encoder.forward(&tokens).expect("forward pass");
    let fwd_elapsed = t0.elapsed();

    let dims = out.dims();
    assert_eq!(
        dims,
        [tokens.len(), QWEN3_4B_CONFIG.hidden],
        "shape mismatch: {:?}",
        dims
    );

    let out_f32 = out.to_dtype(DType::F32).unwrap().contiguous().unwrap();
    let vals = out_f32.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    let non_finite = vals.iter().filter(|v| !v.is_finite()).count();
    assert_eq!(non_finite, 0, "non-finite values in output");

    eprintln!(
        "text_encoder_real_weights_forward: load={:?} build={:?} forward={:?} \
         ({} tokens, hidden {})",
        load_elapsed,
        build_elapsed,
        fwd_elapsed,
        tokens.len(),
        QWEN3_4B_CONFIG.hidden
    );
}

// ── Block 3.3: VAE real-weights env-gated ──────────────────────────

#[test]
fn vae_real_weights_decode_fixed_latent() {
    let Some(weights_dir) = weights_dir() else {
        print_skip("MLOG_VISION_WEIGHTS_DIR not set — VAE real-weights test");
        return;
    };
    let vae_dir = weights_dir.join("vae");
    let t0 = Instant::now();
    let tensors = match load_safetensors_single(&vae_dir, "diffusion_pytorch_model", &Device::Cpu) {
        Ok(t) => t,
        Err(e) => {
            panic!(
                "load_safetensors_single(vae) failed: {}\n\
                 Verify MLOG_VISION_WEIGHTS_DIR layout per \
                 docs/research/naryad-212-weights-manifest.md",
                e
            );
        }
    };
    let load_elapsed = t0.elapsed();

    let t0 = Instant::now();
    let decoder = VaeDecoder::from_weights(&tensors).expect("VaeDecoder::from_weights failed");
    let build_elapsed = t0.elapsed();
    drop(tensors);

    let latent = fixed_latent(21200, 16, 128, 128);

    let t0 = Instant::now();
    let img = decoder.decode(&latent).expect("VAE decode");
    let decode_elapsed = t0.elapsed();

    let dims = img.dims();
    assert_eq!(dims, &[3, 1024, 1024], "VAE output shape: {:?}", dims);

    let img_f32 = img.to_dtype(DType::F32).unwrap().contiguous().unwrap();
    let vals = img_f32.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    let non_finite = vals.iter().filter(|v| !v.is_finite()).count();
    assert_eq!(non_finite, 0, "non-finite pixels: {}", non_finite);

    let out_path = out_dir().join("n212_vae_fixed_latent.png");
    metalogos::vision::vae::save_png(&img_f32, &out_path).expect("save_png");
    eprintln!(
        "vae_real_weights_decode_fixed_latent: load={:?} build={:?} decode={:?}\n  PNG: {}",
        load_elapsed,
        build_elapsed,
        decode_elapsed,
        out_path.display()
    );
}

// ── Block 5.1: end-to-end clinical test (env-gated) ───────────────

#[test]
fn clinical_e2e_first_image() {
    let Some(weights_dir) = weights_dir() else {
        print_skip("MLOG_VISION_WEIGHTS_DIR not set — clinical e2e test");
        return;
    };

    let seed: u64 = 21200;
    let prompt = "a red apple on a wooden table, studio light";

    // Stage A: tokenize
    let t0 = Instant::now();
    let tokenizer =
        metalogos::vision::tokenizer::Tokenizer::from_dir(&weights_dir.join("tokenizer"))
            .expect("Tokenizer load failed");
    let tokens = tokenizer.encode(prompt).expect("encode");
    let tok_elapsed = t0.elapsed();
    assert!(!tokens.is_empty(), "tokens empty");
    eprintln!(
        "clinical_e2e: tokenize: {:?} ({} tokens)",
        tok_elapsed,
        tokens.len()
    );

    // Stage B: text encoder
    let t0 = Instant::now();
    let te_tensors =
        load_safetensors_sharded(&weights_dir.join("text_encoder"), "model", &Device::Cpu)
            .expect("text_encoder load");
    let encoder = TextEncoder::from_weights(&QWEN3_4B_CONFIG, &te_tensors)
        .expect("TextEncoder::from_weights");
    drop(te_tensors);
    let t1 = Instant::now();
    let cap = encoder.forward(&tokens).expect("text encoder forward");
    let encode_elapsed = t0.elapsed();
    let encode_fwd = t1.elapsed();
    eprintln!(
        "clinical_e2e: encode total: {:?} (forward: {:?}) — cap shape {:?}",
        encode_elapsed,
        encode_fwd,
        cap.dims()
    );

    // Stage C: DiT + sampler
    let t0 = Instant::now();
    let dit_tensors = load_safetensors_sharded(
        &weights_dir.join("transformer"),
        "diffusion_pytorch_model",
        &Device::Cpu,
    )
    .expect("transformer load");
    let dit = metalogos::vision::dit::ZImageTransformer::from_weights(&dit_tensors)
        .expect("ZImageTransformer::from_weights");
    drop(dit_tensors);
    let t1 = Instant::now();
    let latent = metalogos::vision::sampler::flow_match_euler_sample(&dit, &cap, seed, 9, 0.0)
        .expect("flow_match_euler_sample");
    let sampler_elapsed = t0.elapsed();
    let sampler_fwd = t1.elapsed();
    eprintln!(
        "clinical_e2e: sampler total: {:?} (8 forward: {:?}) — latent shape {:?}",
        sampler_elapsed,
        sampler_fwd,
        latent.dims()
    );

    // Stage D: VAE decode
    let t0 = Instant::now();
    let vae_tensors = load_safetensors_single(
        &weights_dir.join("vae"),
        "diffusion_pytorch_model",
        &Device::Cpu,
    )
    .expect("vae load");
    let decoder = VaeDecoder::from_weights(&vae_tensors).expect("VaeDecoder::from_weights");
    drop(vae_tensors);
    let t1 = Instant::now();
    let img = decoder.decode(&latent).expect("VAE decode");
    let decode_elapsed = t0.elapsed();
    let decode_only = t1.elapsed();
    eprintln!(
        "clinical_e2e: decode total: {:?} (decode only: {:?})",
        decode_elapsed, decode_only
    );

    let img_f32 = img.to_dtype(DType::F32).unwrap().contiguous().unwrap();
    let vals = img_f32.flatten_all().unwrap().to_vec1::<f32>().unwrap();
    let non_finite = vals.iter().filter(|v| !v.is_finite()).count();
    assert_eq!(non_finite, 0, "non-finite pixels: {}", non_finite);

    let out_path = out_dir().join("first_image.png");
    metalogos::vision::vae::save_png(&img_f32, &out_path).expect("save_png");

    let png_bytes = std::fs::read(&out_path).expect("read PNG");
    let mut hasher = Sha256::new();
    hasher.update(&png_bytes);
    let png_sha: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    let dims = img.dims();
    assert_eq!(dims, &[3, 1024, 1024], "image shape: {:?}", dims);

    eprintln!(
        "clinical_e2e_first_image: PNG path={} sha256={} size={} bytes\n  \
         total stages: tok {:?} + encode {:?} + sampler {:?} + decode {:?}",
        out_path.display(),
        png_sha,
        png_bytes.len(),
        tok_elapsed,
        encode_elapsed,
        sampler_elapsed,
        decode_elapsed
    );
}
