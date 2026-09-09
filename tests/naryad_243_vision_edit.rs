#![cfg(feature = "vision")]
//! Наряд №243 — Vision R6.2 `vision_edit`: tiny-контракты + negatives.
//!
//! Лекало wedge (tests/naryad_212_wedge_e2e.rs): synthetic fixtures, NO
//! network, NO real weights, NO `#[ignore]`. The tiny components are the
//! same ones the wedge goldens pin (`tiny_vae_config` + `tiny_dit_config`)
//! — the wedge goldens themselves stay in №212 untouched and bit-exact
//! (Block 1.3; they pass on this branch without a single edit — the CI job
//! is the authority).
//!
//! ## Coverage (naryad §4.1)
//!
//! - (а) the output depends on the SOURCE (different source bytes →
//!   different output PNG);
//! - (б) the output depends on the EDIT PROMPT (different prompt →
//!   different output PNG; the real path derives cap = TextEncoder(prompt)
//!   — weights-gated, so the tiny harness derives cap from the prompt hash
//!   via the SSOT PRNG, the same honest stand-in the wedge лекало uses for
//!   the text encoder);
//! - (в) the output is SIGNED (LSB watermark + 7-field manifest:
//!   model_id/policy/seed inherited from the source manifest, fresh
//!   timestamp/prompt_sha256/png_sha256, model_sha256 = the honest
//!   "unpinned" marker for a harness without a weights tree);
//! - (г) an unsigned source = loud Err (dispatch-level, fires BEFORE the
//!   env check — no environment needed);
//! - (д) incompatible source dimensions = loud Err (R4.1 bounds + ×16 via
//!   the dispatch contract; VAE-factor divisibility through the tiny
//!   compute path);
//! - (е) no-env / no-feature refusals are loud (dispatch with a signed
//!   source and no `MLOG_VISION_WEIGHTS_DIR` — loud-SKIP pattern when the
//!   host has the env set);
//! - (ж) wedge-goldens bit-exact — №212 untouched and green (see the
//!   header note above; enforced by the №212 CI job, not duplicated here).
//!
//! EDIT_STEPS is pinned to 8 in-situ (the distilled NFE of Turbo — the
//! loud constant's contract, Block 1.2).

use candle_core::{DType, Device, Tensor};
use sha2::Digest;

use metalogos::builtins::vision::{
    vision_edit_check_dims_r41, vision_edit_check_dims_vae_factor, vision_edit_dispatch,
    vision_edit_sign_and_insert,
};
use metalogos::interpreter::Value;
use metalogos::vision::dit::{tiny_dit_config, ZImageTransformer};
use metalogos::vision::provenance::{
    detect_lsb_watermark, embed_lsb_watermark, model_hash32, prompt_hash, sha256_hex,
    VisionManifest,
};
use metalogos::vision::sampler::{flow_match_euler_edit, EDIT_STEPS};
use metalogos::vision::vae::{
    encode_png, tiny_vae_config, vae_downsample_factor, VaeDecoder, VaeEncoder,
};
use metalogos::vision::{VisionArtifact, VisionRegistry};

// ── Tiny harness (wedge лекало) ──────────────────────────────────────

const TINY_SEED: u64 = 24301;

/// Deterministic source image `[3, 64, 64]` in [0,1] — 64 is divisible by
/// the tiny VAE factor (8) → latent 8×8 (the tiny DiT's shape). The seed
/// selects a DISTINCT SSOT-PRNG stream (xorshift — different seeds never
/// share values), so different seeds → genuinely different sources.
fn synthetic_source(seed: u64) -> Tensor {
    use metalogos::nn::attention::generate_uniform_f32;
    let n = 3 * 64 * 64;
    let data: Vec<f32> = generate_uniform_f32(seed, n, 0.0, 1.0);
    Tensor::from_vec(data, (3usize, 64, 64), &Device::Cpu).expect("synthetic source")
}

/// The tiny edit clip: source PNG bytes + prompt → signed edit artifact in
/// the registry. Mirrors `edit_real` stage-by-stage with tiny components
/// and NO weights (the harness derives cap from the prompt hash via the
/// SSOT PRNG — documented in the file header; the REAL path uses the
/// Qwen3-4B text encoder behind the env gate).
fn tiny_edit_clip(
    registry: &mut VisionRegistry,
    source_manifest: &VisionManifest,
    source_png: &[u8],
    prompt: &str,
) -> Result<metalogos::vision::VisionId, String> {
    // Stage A: source PNG → image tensor + dims contract (same order as
    // edit_real: R4.1 bounds are the real-model contract and are checked by
    // the DISPATCH; the factor check guards any config and is checked here
    // — the tiny 64×64 source passes it, a non-divisible source must not).
    let img01 = metalogos::vision::vae::decode_png(source_png)?;
    let dims = img01.dims();
    let (h, w) = (dims[1], dims[2]);
    let cfg = tiny_vae_config();
    let factor = vae_downsample_factor(&cfg);
    vision_edit_check_dims_vae_factor(h, w, factor)?;

    // Stage B: tiny VAE encoder + decoder.
    let vae_encoder = VaeEncoder::new_tiny(&cfg, TINY_SEED)?;
    let decoder = VaeDecoder::new_tiny(&cfg, TINY_SEED)?;

    // Stage C: cap from the EDIT prompt (tiny stand-in — see header).
    let cap = synthetic_cap(prompt);

    // Stage D: tiny DiT.
    let dit = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED)?;

    // Stage E: [0,1] → [-1,1] → reference latent.
    let img_enc = img01
        .affine(2.0, -1.0)
        .map_err(|e| format!("tiny clip: [-1,1] map: {}", e))?;
    let ref_latent = vae_encoder.encode(&img_enc)?;

    // Stage F: the in-context edit loop (EDIT_STEPS, seed inherited).
    let latent = flow_match_euler_edit(&dit, &cap, &ref_latent, source_manifest.seed, EDIT_STEPS)?;

    // Stage G: decode → PNG bytes.
    let img_out = decoder.decode(&latent)?;
    let img_out = img_out.to_dtype(DType::F32).map_err(|e| e.to_string())?;
    let png_bytes = encode_png(&img_out)?;

    // Stage H: sign ALWAYS + insert (the SHARED signing path — the same
    // function the real dispatch calls; weights_dir = None → "unpinned").
    vision_edit_sign_and_insert(registry, source_manifest, &png_bytes, prompt, None)
}

/// Deterministic cap `[4, 32]` derived from the prompt (tiny stand-in for
/// the text encoder): the prompt hash seeds the SSOT PRNG → different
/// prompts → different caps (this is what makes (б) a real contract).
fn synthetic_cap(prompt: &str) -> Tensor {
    use metalogos::nn::attention::generate_uniform_f32;
    let hash = prompt_hash(prompt);
    let mut seed: u64 = 0;
    for b in hash.as_bytes().iter().take(8) {
        seed = (seed << 8) | *b as u64;
    }
    let vals = generate_uniform_f32(seed, 4 * 32, -1.0, 1.0);
    Tensor::from_vec(vals, (4usize, 32), &Device::Cpu).expect("synthetic cap")
}

/// A hand-built SIGNED source manifest (the shape the dispatch extracts
/// from a real `vision_generate` artifact).
fn source_manifest(seed: u64) -> VisionManifest {
    VisionManifest {
        model_id: "z-image-turbo".to_string(),
        model_sha256: "unpinned".to_string(),
        seed,
        prompt_sha256: prompt_hash("a red apple on a wooden table"),
        policy: "safe".to_string(),
        timestamp: "2026-09-10T00:00:00+00:00".to_string(),
        png_sha256: "source".to_string(),
    }
}

fn source_png_bytes(seed: u64) -> Vec<u8> {
    encode_png(&synthetic_source(seed)).expect("source png")
}

fn insert_signed_source(registry: &mut VisionRegistry, seed: u64) -> metalogos::vision::VisionId {
    let manifest = source_manifest(seed);
    let png = source_png_bytes(seed);
    // The source is born signed (watermark embedded — the way
    // vision_generate produces artifacts).
    let png = embed_lsb_watermark(&png, &manifest.model_id).expect("watermark");
    registry.insert(VisionArtifact {
        png_bytes: png,
        manifest: Some(manifest),
    })
}

// ── Tiny VaeEncoder contracts (new compute-path component) ───────────

/// Encode shape contract: `[3, 64, 64]` in [-1,1] → `[1, 4, 8, 8]` (the
/// tiny factor is 8; latent_channels = 4). Determinism: same input →
/// bit-exact latent; different input → different latent.
#[test]
fn vae_encoder_tiny_shape_and_determinism() {
    let cfg = tiny_vae_config();
    let enc = VaeEncoder::new_tiny(&cfg, TINY_SEED).expect("VaeEncoder::new_tiny");

    let img01 = synthetic_source(TINY_SEED);
    let img_enc = img01.affine(2.0, -1.0).expect("affine");
    let latent = enc.encode(&img_enc).expect("encode");
    assert_eq!(
        latent.dims(),
        &[1, cfg.latent_channels, 8, 8],
        "tiny encode shape"
    );

    let h1 = tensor_sha256(&latent);
    let latent2 = enc.encode(&img_enc).expect("encode 2");
    assert_eq!(h1, tensor_sha256(&latent2), "same input → bit-exact latent");

    let img02 = synthetic_source(TINY_SEED + 1);
    let latent3 = enc
        .encode(&img02.affine(2.0, -1.0).expect("affine 2"))
        .expect("encode 3");
    assert_ne!(
        h1,
        tensor_sha256(&latent3),
        "different source → different latent (the encoder is not a no-op)"
    );
}

/// The downsample factor is derived from the config (4 blocks → 8), for
/// both the tiny and the real VAE config.
#[test]
fn vae_downsample_factor_from_config() {
    assert_eq!(
        vae_downsample_factor(&tiny_vae_config()),
        8,
        "tiny config: 4 block_out_channels → factor 8"
    );
    assert_eq!(
        vae_downsample_factor(&metalogos::vision::vae::zimage_turbo_vae_config()),
        8,
        "real config: 4 block_out_channels → factor 8"
    );
}

// ── The in-context edit loop contracts ────────────────────────────────

/// EDIT_STEPS is the loud distilled-NFE constant (8) — pinned in-situ.
#[test]
fn edit_steps_is_the_distilled_turbo_nfe() {
    // Block 1.2: `const EDIT_STEPS: usize = 8` (the turbo clip's 8 forward
    // steps). Any change here is a loud deviation that must be argued
    // BEFORE the merge — this assert makes the deviation impossible to
    // sneak in silently.
    assert_eq!(EDIT_STEPS, 8, "EDIT_STEPS = distilled turbo NFE");
}

/// forward_edit shape contract: velocity == noise shape; mismatched
/// noise/reference shapes are a loud Err (no silent reshape = no silent
/// resize, Block 1.4).
#[test]
fn forward_edit_shape_contract() {
    let dit = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED).expect("dit");
    let cap = synthetic_cap("a red apple");
    let noise = Tensor::from_vec(
        generate_uniform_f32_latent(101, 4 * 8 * 8),
        (1usize, 4, 8, 8),
        &Device::Cpu,
    )
    .expect("noise");
    let ref_latent = Tensor::from_vec(
        generate_uniform_f32_latent(102, 4 * 8 * 8),
        (1usize, 4, 8, 8),
        &Device::Cpu,
    )
    .expect("ref");

    let v = dit
        .forward_edit(&noise, &ref_latent, &cap, 375.0)
        .expect("forward_edit");
    assert_eq!(v.dims(), &[1, 4, 8, 8], "velocity shape == noise shape");

    let bad_ref = Tensor::from_vec(
        generate_uniform_f32_latent(103, 4 * 8 * 4),
        (1usize, 4, 8, 4),
        &Device::Cpu,
    )
    .expect("bad ref");
    let err = dit
        .forward_edit(&noise, &bad_ref, &cap, 375.0)
        .expect_err("shape mismatch must be loud");
    assert!(
        err.contains("same shape"),
        "error must name the shape contract: {}",
        err
    );
}

/// The edit loop depends on the reference latent: two runs identical
/// except for the reference produce different outputs (the source is not
/// ignored — the first half of Block §3.1's no-op prohibition).
#[test]
fn edit_loop_depends_on_reference_latent() {
    let dit = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED).expect("dit");
    let cap = synthetic_cap("a red apple");
    let ref_a = Tensor::from_vec(
        generate_uniform_f32_latent(201, 4 * 8 * 8),
        (1usize, 4, 8, 8),
        &Device::Cpu,
    )
    .expect("ref a");
    let ref_b = Tensor::from_vec(
        generate_uniform_f32_latent(202, 4 * 8 * 8),
        (1usize, 4, 8, 8),
        &Device::Cpu,
    )
    .expect("ref b");
    assert_ne!(
        tensor_sha256(&ref_a),
        tensor_sha256(&ref_b),
        "harness sanity: the two references must differ"
    );

    let out_a = flow_match_euler_edit(&dit, &cap, &ref_a, 4242, EDIT_STEPS).expect("edit a");
    let out_b = flow_match_euler_edit(&dit, &cap, &ref_b, 4242, EDIT_STEPS).expect("edit b");
    assert_ne!(
        tensor_sha256(&out_a),
        tensor_sha256(&out_b),
        "different reference latent → different edit output"
    );

    // Determinism: same inputs (incl. the inherited seed) → bit-exact.
    let out_a2 = flow_match_euler_edit(&dit, &cap, &ref_a, 4242, EDIT_STEPS).expect("edit a2");
    assert_eq!(
        tensor_sha256(&out_a),
        tensor_sha256(&out_a2),
        "same source + prompt + seed → bit-exact edit output (Block 2.3)"
    );
}

// ── (а)/(б)/(в): the signed edit artifact contracts ──────────────────

/// (а) The output depends on the source: two different sources, same
/// prompt → different output PNGs. (Also: the edit output KEEPS the source
/// dimensions — no resize anywhere in the loop, Block 1.4.)
#[test]
fn edit_output_depends_on_source() {
    let mut reg = VisionRegistry::new();
    let manifest = source_manifest(4242);
    let png_a = source_png_bytes(1);
    let png_b = source_png_bytes(2);
    assert_ne!(png_a, png_b, "different sources → different PNG bytes");

    let id_a = tiny_edit_clip(&mut reg, &manifest, &png_a, "a red apple").expect("edit a");
    let id_b = tiny_edit_clip(&mut reg, &manifest, &png_b, "a red apple").expect("edit b");
    let out_a = reg.get(id_a).expect("artifact a");
    let out_b = reg.get(id_b).expect("artifact b");
    assert_ne!(
        out_a.png_bytes, out_b.png_bytes,
        "different source → different edited PNG (no silent no-op)"
    );

    // The output resolution equals the source resolution (64×64 → 64×64).
    let img = metalogos::vision::vae::decode_png(&out_a.png_bytes).expect("decode out");
    assert_eq!(img.dims(), &[3, 64, 64], "output keeps the source size");
}

/// (б) The output depends on the edit prompt: same source, two prompts →
/// different output PNGs (the prompt reaches the loop through cap — see
/// the header note; the manifest part of this contract is asserted in (в)).
#[test]
fn edit_output_depends_on_prompt() {
    let mut reg = VisionRegistry::new();
    let manifest = source_manifest(4242);
    let png = source_png_bytes(1);

    let id_a = tiny_edit_clip(&mut reg, &manifest, &png, "a red apple").expect("edit a");
    let id_b = tiny_edit_clip(&mut reg, &manifest, &png, "a green apple").expect("edit b");
    let out_a = reg.get(id_a).expect("artifact a");
    let out_b = reg.get(id_b).expect("artifact b");
    assert_ne!(
        out_a.png_bytes, out_b.png_bytes,
        "different prompt → different edited PNG (no silent prompt-ignore)"
    );
}

/// (в) The output is SIGNED: LSB watermark of the inherited model_id +
/// 7-field manifest with the inheritance contract (model_id/policy/seed
/// from the source; fresh timestamp/prompt_sha256/png_sha256; the honest
/// "unpinned" weights marker for a harness without a weights tree).
#[test]
fn edit_output_is_signed_with_inherited_provenance() {
    let mut reg = VisionRegistry::new();
    let manifest = source_manifest(4242);
    let png = source_png_bytes(1);

    let id = tiny_edit_clip(&mut reg, &manifest, &png, "a green apple, studio light")
        .expect("tiny edit clip");
    let artifact = reg.get(id).expect("edited artifact in the registry");
    let out_manifest = artifact
        .manifest
        .as_ref()
        .expect("the edit output MUST carry a manifest (sign ALWAYS)");

    // 7 fields — inheritance (Block 2.3).
    assert_eq!(
        out_manifest.model_id, manifest.model_id,
        "model_id inherited"
    );
    assert_eq!(
        out_manifest.policy, manifest.policy,
        "policy inherited (no overwrite)"
    );
    assert_eq!(
        out_manifest.seed, manifest.seed,
        "seed inherited (determinism)"
    );
    assert_eq!(
        out_manifest.model_sha256, "unpinned",
        "no weights tree in the harness → the honest marker"
    );

    // Fresh fields.
    assert_eq!(
        out_manifest.prompt_sha256,
        prompt_hash("a green apple, studio light"),
        "prompt_sha256 = hash of the EDIT prompt"
    );
    assert_ne!(
        out_manifest.prompt_sha256, manifest.prompt_sha256,
        "the edit prompt differs from the source prompt"
    );
    assert_eq!(
        out_manifest.png_sha256,
        sha256_hex(&artifact.png_bytes),
        "png_sha256 describes exactly the shipped (watermarked) bytes"
    );
    assert_ne!(
        out_manifest.timestamp, manifest.timestamp,
        "timestamp is fresh (wall-clock), not inherited"
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(&out_manifest.timestamp).is_ok(),
        "timestamp is RFC 3339: {}",
        out_manifest.timestamp
    );

    // The watermark: detectable, and it carries the INHERITED model's hash.
    assert_eq!(
        detect_lsb_watermark(&artifact.png_bytes).expect("detect"),
        Some(model_hash32("z-image-turbo")),
        "the edited PNG carries the LSB watermark"
    );
}

// ── (г)/(е): dispatch negatives (loud, environment-free) ─────────────

/// (г) An unsigned source is refused loudly — BEFORE the env check (the
/// contract refusal does not depend on the environment). The raw export
/// path stays untouched (Block 2.2).
#[test]
fn edit_unsigned_source_loud_error() {
    let mut reg = VisionRegistry::new();
    let id = reg.insert(VisionArtifact {
        png_bytes: source_png_bytes(1),
        manifest: None, // hand-built/deserialized — unsigned
    });
    let args = vec![
        Value::Vision(id),
        Value::String("a green apple".to_string()),
    ];
    let err = vision_edit_dispatch(&mut reg, &args).expect_err("unsigned source must fail");
    assert!(
        err.contains("vision_edit"),
        "error must contain the builtin name: {}",
        err
    );
    assert!(
        err.contains("carries no provenance manifest"),
        "error must name the unsigned-source contract: {}",
        err
    );
    assert!(
        err.contains("vision_generate") || err.contains("vision_load"),
        "error must say HOW to get a signed source: {}",
        err
    );
    // A failed dispatch must not insert anything.
    assert_eq!(reg.len(), 1, "failed dispatch must not insert");
}

/// Dispatch negative: unknown handle → loud `[Vision#N]` refusal (лекало
/// export/save).
#[test]
fn edit_unknown_handle_loud_error() {
    let mut reg = VisionRegistry::new();
    let args = vec![
        Value::Vision(metalogos::vision::VisionId(77)),
        Value::String("a green apple".to_string()),
    ];
    let err = vision_edit_dispatch(&mut reg, &args).expect_err("unknown handle must fail");
    assert!(
        err.contains("[Vision#77]"),
        "error must repeat the handle: {}",
        err
    );
    assert!(
        err.contains("not found in the registry"),
        "error must name the lookup failure: {}",
        err
    );
}

/// Dispatch negative: wrong argument types are loud typed refusals
/// (лекало №240/№242).
#[test]
fn edit_wrong_types_loud_errors() {
    let mut reg = VisionRegistry::new();
    // String where the Vision handle is expected.
    let args = vec![
        Value::String("handle".to_string()),
        Value::String("prompt".to_string()),
    ];
    let err = vision_edit_dispatch(&mut reg, &args).expect_err("String handle must fail");
    assert!(
        err.contains("must be a Vision handle"),
        "typed handle contract: {}",
        err
    );
    // Arity.
    let err = vision_edit_dispatch(&mut reg, &[Value::Unit]).expect_err("arity 1 must fail");
    assert!(
        err.contains("expects 2 arguments"),
        "arity contract: {}",
        err
    );
}

/// (е) No-env refusal: a SIGNED source + no `MLOG_VISION_WEIGHTS_DIR` →
/// loud environment refusal naming the env var (loud-SKIP when the host
/// has the env set — the naryad_240 pattern).
#[test]
fn edit_missing_weights_env_loud_error() {
    if std::env::var_os("MLOG_VISION_WEIGHTS_DIR").is_some() {
        eprintln!(
            "LOUD SKIP: MLOG_VISION_WEIGHTS_DIR is set — missing-env negative \
             not exercisable in this environment"
        );
        return;
    }
    let mut reg = VisionRegistry::new();
    let id = insert_signed_source(&mut reg, 4242);
    let args = vec![
        Value::Vision(id),
        Value::String("a green apple".to_string()),
    ];
    let err = vision_edit_dispatch(&mut reg, &args).expect_err("no-env must fail loudly");
    assert!(
        err.contains("MLOG_VISION_WEIGHTS_DIR"),
        "error must name the env var: {}",
        err
    );
    assert!(
        err.contains("is not set"),
        "error must state what is wrong: {}",
        err
    );
    assert_eq!(reg.len(), 1, "failed dispatch must not insert");
}

// ── (д): the dimension contract ───────────────────────────────────────

/// R4.1 bounds + ×16 — loud refusals (Block 1.4).
#[test]
fn edit_dims_r41_contract() {
    // In-bounds, ×16 → Ok.
    assert!(vision_edit_check_dims_r41(1024, 1024).is_ok());
    assert!(vision_edit_check_dims_r41(256, 4096).is_ok());
    // Out of bounds → loud.
    let err = vision_edit_check_dims_r41(128, 1024).expect_err("below 256 must fail");
    assert!(err.contains("256..=4096"), "bounds contract: {}", err);
    assert!(
        err.contains("silent resize"),
        "no-resize rationale: {}",
        err
    );
    let err = vision_edit_check_dims_r41(4128, 1024).expect_err("above 4096 must fail");
    assert!(err.contains("256..=4096"), "bounds contract: {}", err);
    // Not ×16 → loud.
    let err = vision_edit_check_dims_r41(1020, 1024).expect_err("not ×16 must fail");
    assert!(err.contains("multiples of 16"), "×16 contract: {}", err);
}

/// VAE factor divisibility — loud refusal; the tiny compute path enforces
/// it BEFORE the encoder (a non-divisible source must never reach the
/// stride-2 convs).
#[test]
fn edit_dims_vae_factor_contract() {
    assert!(vision_edit_check_dims_vae_factor(64, 64, 8).is_ok());
    assert!(vision_edit_check_dims_vae_factor(1024, 1024, 8).is_ok());
    let err = vision_edit_check_dims_vae_factor(60, 64, 8).expect_err("60 % 8 != 0");
    assert!(
        err.contains("not divisible by the VAE"),
        "factor contract: {}",
        err
    );
}

// ── Shared helpers ────────────────────────────────────────────────────

fn generate_uniform_f32_latent(seed: u64, n: usize) -> Vec<f32> {
    use metalogos::nn::attention::generate_uniform_f32;
    generate_uniform_f32(seed, n, -1.0, 1.0)
}

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
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}
