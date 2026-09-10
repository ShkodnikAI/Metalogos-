#![cfg(feature = "vision")]
//! Наряд №244 — Vision R6.3: LoRA-адаптеры (SQLite BLOB + применение к DiT).
//!
//! Tiny-контракты + negatives. Лекало wedge/№243: synthetic fixtures, NO
//! network, NO real weights, NO `#[ignore]`. The adapter blob is written
//! with the candle safetensors writer (Block 4.1 — «минимальный
//! safetensors-блоб строится в тесте candle-райтером»); the DiT is the
//! same `tiny_dit_config` the wedge goldens pin.
//!
//! ## Coverage (naryad §4.1)
//!
//! - (а) parse: rank/alpha/scale/targets from a blob (BOTH canonical name
//!   forms — diffusers-PEFT and ComfyUI);
//! - (б) parse negatives: B without A, non-attention target, unknown
//!   prefix, dims mismatch, orphaned key — loud Err with the FULL list;
//! - (в) the merge changes the tiny-DiT output (the math itself is pinned
//!   by the direct-matmul unit contract in `src/vision/dit.rs`);
//! - (г) a zero up OR down merge → the output is BYTE-EXACT the base
//!   (Block 1.3 identity);
//! - (д) determinism: two full tiny runs (merge + sample + PNG) →
//!   identical bytes;
//! - (е) the №212 wedge goldens stay green WITHOUT a single edit — enforced
//!   by the №212 CI job on this same branch (not duplicated here, №243
//!   precedent);
//! - (ж) store: bytes+meta roundtrip, collision = Err, corrupted meta =
//!   Err (direct UPDATE, №242 лекало);
//! - (з) integrity: sha mismatch between the DB bytes and the pinned meta
//!   = loud Err BEFORE any compute;
//! - (и) lora_load: no-db, no-env, `..`, non-.safetensors, missing file —
//!   loud refusals;
//! - (к) lora_generate: unknown decl / unknown lora (with the list) /
//!   empty prompt / non-1024 — loud refusals;
//! - (л) signing: the 7-field manifest, the composite
//!   `sha256("{base}\nlora:{name}:{sha}")` structure, the watermark, the
//!   declared policy.
//!
//! (м) taint coverage lives in `tests/naryad_240_vision_dispatch.rs`
//! (лекало №243) and (н) the stub-group truth-up in
//! `tests/naryad_210_vision_skeleton.rs`.
//!
//! Environment discipline: the tests that READ `MLOG_VISION_WEIGHTS_DIR`
//! and the ONE test that sets it share a file-local mutex — no
//! cross-test env races within this binary (other test files run as
//! separate processes, sequenced by cargo).

use std::collections::HashMap;
use std::sync::Mutex;

use candle_core::{DType, Device, Tensor};

use metalogos::builtins::vision::vision_generate_sign_and_insert;
use metalogos::builtins::vision::{
    vision_lora_check_adapter_path, vision_lora_composite_model_sha256,
    vision_lora_generate_dispatch, vision_lora_load_dispatch,
};
use metalogos::bytecode::{CompiledVisionDecl, CompiledVisionPolicy, CompiledVisionProfile};
use metalogos::interpreter::Value;
use metalogos::nn::attention::generate_uniform_f32;
use metalogos::vision::dit::{tiny_dit_config, ZImageTransformer};
use metalogos::vision::lora::LoraAdapter;
use metalogos::vision::provenance::{detect_lsb_watermark, model_hash32, prompt_hash, sha256_hex};
use metalogos::vision::sampler::flow_match_euler_sample;
use metalogos::vision::store::{lora_get, lora_list, lora_save};
use metalogos::vision::vae::{encode_png, tiny_vae_config, VaeDecoder};
use metalogos::vision::{VisionId, VisionRegistry};

// ── Env mutex (see the header) ───────────────────────────────────────

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Adapter blob writer (candle-райтер, Block 4.1) ──────────────────

/// Write a minimal safetensors adapter blob with the candle writer and
/// read it back as BYTES (the form `LoraAdapter::parse` consumes and the
/// form `vision_lora_load` persists as a BLOB). `zero_up`/`zero_down`
/// allow the identity contract to zero ONE half of each pair.
fn adapter_blob(
    pairs: Vec<(String, usize, usize, usize)>,
    alpha: Option<f32>,
    zero_up: bool,
    zero_down: bool,
) -> Vec<u8> {
    // Unique-per-call temp dir (parallel tests share the process).
    static BLOB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = BLOB_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("n244_blob_{}_{}", std::process::id(), seq));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("adapter.safetensors");
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    for (i, (key, r, out, inn)) in pairs.into_iter().enumerate() {
        let up: Vec<f32> = if zero_up {
            vec![0.0; out * r]
        } else {
            generate_uniform_f32(100 + i as u64, out * r, -0.5, 0.5)
        };
        let down: Vec<f32> = if zero_down {
            vec![0.0; r * inn]
        } else {
            generate_uniform_f32(200 + i as u64, r * inn, -0.5, 0.5)
        };
        tensors.insert(
            format!("{}.lora_A.weight", key),
            Tensor::from_vec(down, (r, inn), &Device::Cpu).expect("down"),
        );
        tensors.insert(
            format!("{}.lora_B.weight", key),
            Tensor::from_vec(up, (out, r), &Device::Cpu).expect("up"),
        );
    }
    if let Some(a) = alpha {
        tensors.insert(
            "layers.0.attention.to_q.weight.alpha".to_string(),
            Tensor::new(a, &Device::Cpu).expect("alpha"),
        );
    }
    candle_core::safetensors::save(&tensors, &path).expect("candle safetensors save");
    let bytes = std::fs::read(&path).expect("read blob back");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
    bytes
}

fn blob_bytes_only(tensors: HashMap<String, Tensor>) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("n244_blob2_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("raw.safetensors");
    candle_core::safetensors::save(&tensors, &path).expect("save");
    let bytes = std::fs::read(&path).expect("read");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
    bytes
}

// ── (а) Parse: both canonical forms ──────────────────────────────────

/// diffusers-PEFT form (`<target>.lora_A.weight` / `.lora_B.weight` +
/// `<target>.alpha`): rank = the pair dimension, scale = alpha/rank,
/// exactly the validated targets survive.
#[test]
fn parse_diffusers_peft_form_rank_alpha_scale_targets() {
    let key = "layers.1.attention.to_q.weight";
    let bytes = adapter_blob(vec![(key.to_string(), 4, 64, 64)], Some(2.0), false, false);
    let adapter = LoraAdapter::parse(&bytes).expect("diffusers-PEFT adapter");
    assert_eq!(adapter.targets.len(), 1, "exactly one target");
    assert!(adapter.targets.contains_key(key), "target = the base key");
    assert_eq!(adapter.rank, 4, "rank = the pair dimension");
    assert_eq!(adapter.alpha, Some(2.0), "alpha from the tensor");
    assert!(
        (adapter.scale - 0.5).abs() < 1e-9,
        "scale = alpha/rank = 2/4, got {}",
        adapter.scale
    );
}

/// ComfyUI form (`<target>.lora_down.weight` / `.lora_up.weight`): the
/// same pair semantics, no alpha → scale = 1.0 (the loud default — the
/// eprintln note is the loud layer, the value is pinned here).
#[test]
fn parse_comfyui_form_without_alpha_scale_is_one() {
    let key = "noise_refiner.0.attention.to_out.0.weight";
    static COMFY_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "n244_comfy_{}_{}",
        std::process::id(),
        COMFY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("comfy.safetensors");
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    let r = 8usize;
    let (out_dim, in_dim) = (64usize, 64usize);
    tensors.insert(
        format!("{}.lora_down.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(11, r * in_dim, -0.5, 0.5),
            (r, in_dim),
            &Device::Cpu,
        )
        .expect("down"),
    );
    tensors.insert(
        format!("{}.lora_up.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(12, out_dim * r, -0.5, 0.5),
            (out_dim, r),
            &Device::Cpu,
        )
        .expect("up"),
    );
    candle_core::safetensors::save(&tensors, &path).expect("save");
    let bytes = std::fs::read(&path).expect("read");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);

    let adapter = LoraAdapter::parse(&bytes).expect("ComfyUI adapter");
    assert_eq!(adapter.targets.len(), 1);
    assert!(adapter.targets.contains_key(key));
    assert_eq!(adapter.rank, 8);
    assert_eq!(adapter.alpha, None, "no alpha tensor in the ComfyUI blob");
    assert_eq!(adapter.scale, 1.0, "alpha absent → scale = 1.0 (loud note)");
}

// ── (б) Parse negatives — loud, with the FULL problem list ───────────

/// B without A (a half pair) is refused; the error names the target.
#[test]
fn parse_half_pair_loud_error() {
    let dir = std::env::temp_dir().join(format!("n244_half_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("half.safetensors");
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    tensors.insert(
        "layers.0.attention.to_v.weight.lora_B.weight".to_string(),
        Tensor::from_vec(
            generate_uniform_f32(3, 64 * 2, -0.5, 0.5),
            (64, 2),
            &Device::Cpu,
        )
        .expect("b"),
    );
    candle_core::safetensors::save(&tensors, &path).expect("save");
    let bytes = std::fs::read(&path).expect("read");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);

    let err = LoraAdapter::parse(&bytes).expect_err("half pair must fail");
    assert!(err.contains("without"), "names the missing half: {}", err);
    assert!(
        err.contains("layers.0.attention.to_v.weight"),
        "names the target: {}",
        err
    );
    assert!(err.contains("1 problem"), "the full list header: {}", err);
}

/// A non-attention target (an FFN weight) and an unknown prefix are
/// refused — quiet dropping of keys is forbidden.
#[test]
fn parse_non_attention_and_unknown_prefix_loud_errors() {
    // FFN target: layers.0.feed_forward.w1.weight.lora_A.weight — not an
    // attention projection.
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    let key = "layers.0.feed_forward.w1.weight";
    tensors.insert(
        format!("{}.lora_A.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(4, 2 * 256, -0.5, 0.5),
            (2, 256),
            &Device::Cpu,
        )
        .expect("a"),
    );
    tensors.insert(
        format!("{}.lora_B.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(5, 256 * 2, -0.5, 0.5),
            (256, 2),
            &Device::Cpu,
        )
        .expect("b"),
    );
    let err = LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("FFN target must fail");
    assert!(
        err.contains("NOT an attention projection"),
        "names the rule: {}",
        err
    );

    // Unknown prefix: blocks.0.… (the real keys are layers./noise_refiner./
    // context_refiner. per zimage_expected_keys).
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    let key = "blocks.0.attention.to_q.weight";
    tensors.insert(
        format!("{}.lora_A.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(6, 2 * 64, -0.5, 0.5),
            (2, 64),
            &Device::Cpu,
        )
        .expect("a"),
    );
    tensors.insert(
        format!("{}.lora_B.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(7, 64 * 2, -0.5, 0.5),
            (64, 2),
            &Device::Cpu,
        )
        .expect("b"),
    );
    let err = LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("unknown prefix must fail");
    assert!(
        err.contains("blocks.0"),
        "names the offending target: {}",
        err
    );

    // Orphaned key: matches no canonical suffix.
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    tensors.insert(
        "layers.0.attention.to_q.weight.lora_C.weight".to_string(),
        Tensor::from_vec(vec![0.0f32; 4], (2, 2), &Device::Cpu).expect("orphan"),
    );
    let err = LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("orphan must fail");
    assert!(err.contains("orphaned key"), "names the orphan: {}", err);
}

/// Mismatched inner dimensions (up.dims()[1] != down.dims()[0]) are
/// refused with the shapes named.
#[test]
fn parse_dims_mismatch_loud_error() {
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    let key = "layers.0.attention.to_k.weight";
    tensors.insert(
        format!("{}.lora_A.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(8, 4 * 64, -0.5, 0.5),
            (4, 64),
            &Device::Cpu,
        )
        .expect("a"),
    );
    tensors.insert(
        format!("{}.lora_B.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(9, 64 * 8, -0.5, 0.5),
            (64, 8),
            &Device::Cpu,
        )
        .expect("b"),
    );
    let err = LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("dims mismatch must fail");
    assert!(
        err.contains("inner rank disagrees"),
        "names the shape contract: {}",
        err
    );
}

// ── (№245) Mixed-form targets — loud ambiguous-form refusals ─────────

/// (Наряд №245 Block 1.2a) A target present in BOTH canonical forms —
/// even with CONSISTENT values — is a loud `ambiguous form` error that
/// names the target, BOTH forms and the exact keys; the adapter is
/// refused entirely. (The pre-№245 union merge silently let the ComfyUI
/// form overwrite the PEFT form — the exact quiet key drop the contract
/// and §3.1 forbid.)
#[test]
fn parse_mixed_form_complete_loud_ambiguous_error() {
    let key = "layers.0.attention.to_q.weight";
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    // diffusers-PEFT form: lora_A [4, 64] + lora_B [64, 4].
    tensors.insert(
        format!("{}.lora_A.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(21, 4 * 64, -0.5, 0.5),
            (4, 64),
            &Device::Cpu,
        )
        .expect("a"),
    );
    tensors.insert(
        format!("{}.lora_B.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(22, 64 * 4, -0.5, 0.5),
            (64, 4),
            &Device::Cpu,
        )
        .expect("b"),
    );
    // ComfyUI form for the SAME target, consistent shapes (rank 4).
    tensors.insert(
        format!("{}.lora_down.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(23, 4 * 64, -0.5, 0.5),
            (4, 64),
            &Device::Cpu,
        )
        .expect("down"),
    );
    tensors.insert(
        format!("{}.lora_up.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(24, 64 * 4, -0.5, 0.5),
            (64, 4),
            &Device::Cpu,
        )
        .expect("up"),
    );
    let err =
        LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("mixed form must fail, not parse");
    assert!(err.contains("ambiguous form"), "names the defect: {}", err);
    assert!(err.contains(key), "names the target: {}", err);
    assert!(
        err.contains("diffusers-PEFT") && err.contains("ComfyUI"),
        "names BOTH forms: {}",
        err
    );
    assert!(
        err.contains(&format!("{}.lora_A.weight", key))
            && err.contains(&format!("{}.lora_down.weight", key)),
        "names the exact keys: {}",
        err
    );
    assert!(err.contains("1 problem"), "the full list header: {}", err);
}

/// (Наряд №245 Block 1.2b) A PARTIAL mixed form — PEFT lora_A + ComfyUI
/// lora_down for the low half, lora_B only from PEFT — is ambiguous too:
/// refused loudly, NOT resolved by quietly preferring one form's low half
/// (the pre-№245 code silently assembled a (down, B) pair across forms).
#[test]
fn parse_mixed_form_partial_loud_ambiguous_error() {
    let key = "layers.1.attention.to_k.weight";
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    tensors.insert(
        format!("{}.lora_A.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(25, 4 * 64, -0.5, 0.5),
            (4, 64),
            &Device::Cpu,
        )
        .expect("a"),
    );
    tensors.insert(
        format!("{}.lora_B.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(26, 64 * 4, -0.5, 0.5),
            (64, 4),
            &Device::Cpu,
        )
        .expect("b"),
    );
    tensors.insert(
        format!("{}.lora_down.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(27, 4 * 64, -0.5, 0.5),
            (4, 64),
            &Device::Cpu,
        )
        .expect("down"),
    );
    let err =
        LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("partial mixed form must fail");
    assert!(err.contains("ambiguous form"), "names the defect: {}", err);
    assert!(err.contains(key), "names the target: {}", err);
    assert!(
        err.contains(&format!("{}.lora_B.weight", key))
            && err.contains(&format!("{}.lora_down.weight", key)),
        "names the exact keys: {}",
        err
    );
    assert!(err.contains("1 problem"), "the full list header: {}", err);
}

/// (Наряд №245 Block 1.2) The CROSS-form pair — low from ONE form (PEFT
/// lora_A) + high from the OTHER (ComfyUI lora_up), nothing else — is
/// ambiguous as well: a pair assembled across forms is a silent form
/// preference, not a canonical form. (The pre-№245 code parsed it OK.)
#[test]
fn parse_mixed_form_cross_pair_loud_ambiguous_error() {
    let key = "layers.0.attention.to_out.0.weight";
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    tensors.insert(
        format!("{}.lora_A.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(28, 4 * 64, -0.5, 0.5),
            (4, 64),
            &Device::Cpu,
        )
        .expect("a"),
    );
    tensors.insert(
        format!("{}.lora_up.weight", key),
        Tensor::from_vec(
            generate_uniform_f32(29, 64 * 4, -0.5, 0.5),
            (64, 4),
            &Device::Cpu,
        )
        .expect("up"),
    );
    let err = LoraAdapter::parse(&blob_bytes_only(tensors)).expect_err("cross-form pair must fail");
    assert!(err.contains("ambiguous form"), "names the defect: {}", err);
    assert!(err.contains(key), "names the target: {}", err);
    assert!(
        err.contains(&format!("{}.lora_A.weight", key))
            && err.contains(&format!("{}.lora_up.weight", key)),
        "names the exact keys: {}",
        err
    );
    assert!(err.contains("1 problem"), "the full list header: {}", err);
}

// ── Tiny-DiT harness (wedge лекало) ──────────────────────────────────

const TINY_SEED: u64 = 24401;

/// Deterministic cap `[4, 32]` (tiny cap_feat_dim = 32) — the stand-in for
/// the text encoder (the wedge-лекало stand-in, documented in the header).
fn tiny_cap(seed: u64) -> Tensor {
    Tensor::from_vec(
        generate_uniform_f32(seed, 4 * 32, -1.0, 1.0),
        (4usize, 32),
        &Device::Cpu,
    )
    .expect("cap")
}

fn tiny_noise(seed: u64) -> Tensor {
    Tensor::from_vec(
        generate_uniform_f32(seed, 4 * 8 * 8, -1.0, 1.0),
        (1usize, 4, 8, 8),
        &Device::Cpu,
    )
    .expect("noise")
}

fn tensor_sha256(t: &Tensor) -> String {
    let t = t
        .to_dtype(DType::F32)
        .unwrap()
        .contiguous()
        .unwrap()
        .flatten_all()
        .unwrap();
    let bytes = t.to_vec1::<f32>().unwrap();
    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const u8, bytes.len() * 4) };
    sha256_hex(bytes)
}

/// The merged tiny adapter for the output-level contracts: two attention
/// targets of the tiny DiT (layers.0 to_q, layers.1 to_out), rank 4,
/// alpha 2.0 → scale 0.5.
fn tiny_adapter_blob() -> Vec<u8> {
    adapter_blob(
        vec![
            ("layers.0.attention.to_q.weight".to_string(), 4, 64, 64),
            ("layers.1.attention.to_out.0.weight".to_string(), 4, 64, 64),
        ],
        Some(2.0),
        false,
        false,
    )
}

fn base_tensors_for(keys: &[&str], shape: (usize, usize)) -> HashMap<String, Tensor> {
    let mut m = HashMap::new();
    for (i, k) in keys.iter().enumerate() {
        m.insert(
            k.to_string(),
            Tensor::from_vec(
                generate_uniform_f32(300 + i as u64, shape.0 * shape.1, -0.25, 0.25),
                shape,
                &Device::Cpu,
            )
            .expect("base tensor"),
        );
    }
    m
}

// ── (в) The merge changes the tiny-DiT output ────────────────────────

/// The merged DiT produces a DIFFERENT velocity than the base one (the
/// adapter is not a silent no-op). The direct-matmul pin of the merge
/// math itself lives in `src/vision/dit.rs` (unit contract).
#[test]
fn lora_merge_changes_tiny_dit_output() {
    let blob = tiny_adapter_blob();
    let adapter = LoraAdapter::parse(&blob).expect("adapter");
    let base_tensors = base_tensors_for(
        &[
            "layers.0.attention.to_q.weight",
            "layers.1.attention.to_out.0.weight",
        ],
        (64, 64),
    );

    let base = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED).expect("base dit");
    let v_base = base
        .forward(&tiny_noise(9), &tiny_cap(10), 500.0)
        .expect("base forward");

    let mut merged = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED).expect("dit");
    merged
        .merge_lora_in_place(&adapter, &base_tensors)
        .expect("merge");
    let v_merged = merged
        .forward(&tiny_noise(9), &tiny_cap(10), 500.0)
        .expect("merged forward");

    assert_ne!(
        tensor_sha256(&v_base),
        tensor_sha256(&v_merged),
        "the adapter merge must change the DiT output (no silent no-op)"
    );
}

// ── (г) Identity: zero up OR down → byte-exact base ──────────────────

/// A zero up (and a zero down) merge leaves the tiny-DiT output
/// BYTE-EXACT (Block 1.3 control invariant).
#[test]
fn lora_zero_delta_identity_output_bit_exact() {
    let cap = tiny_cap(10);
    let noise = tiny_noise(9);
    let base_tensors = base_tensors_for(
        &[
            "layers.0.attention.to_q.weight",
            "layers.1.attention.to_out.0.weight",
        ],
        (64, 64),
    );

    let base = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED).expect("base dit");
    let v_base = base.forward(&noise, &cap, 500.0).expect("base forward");
    let base_sha = tensor_sha256(&v_base);

    for zero in ["up", "down"] {
        let zero_up = zero == "up";
        let blob = adapter_blob(
            vec![
                ("layers.0.attention.to_q.weight".to_string(), 4, 64, 64),
                ("layers.1.attention.to_out.0.weight".to_string(), 4, 64, 64),
            ],
            Some(2.0),
            zero_up,
            !zero_up,
        );
        let adapter = LoraAdapter::parse(&blob).expect("adapter");
        let mut dit = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED).expect("dit");
        dit.merge_lora_in_place(&adapter, &base_tensors)
            .expect("merge");
        let v = dit.forward(&noise, &cap, 500.0).expect("forward");
        assert_eq!(
            tensor_sha256(&v),
            base_sha,
            "zero {} → the output must stay byte-exact (Block 1.3)",
            zero
        );
    }
}

// ── (д) Determinism: two full tiny runs → identical bytes ────────────

/// Two runs of the whole tiny lora_generate stand-in (build base weights →
/// merge → sample → PNG) produce IDENTICAL PNG bytes. The REAL
/// generate_real_with path is env-gated on the real weights (the PARKED
/// runbook); the determinism that matters here is the merge + sampler +
/// PNG chain, which is the compute core of that path.
#[test]
fn lora_generate_determinism_two_runs_bit_exact() {
    let blob = tiny_adapter_blob();
    let adapter = LoraAdapter::parse(&blob).expect("adapter");
    let base_tensors = base_tensors_for(
        &[
            "layers.0.attention.to_q.weight",
            "layers.1.attention.to_out.0.weight",
        ],
        (64, 64),
    );

    let run = || -> Result<Vec<u8>, String> {
        let mut dit = ZImageTransformer::new_tiny(&tiny_dit_config(), TINY_SEED)?;
        dit.merge_lora_in_place(&adapter, &base_tensors)?;
        let latent = flow_match_euler_sample(&dit, &tiny_cap(10), 4242, 4, 0.0)?;
        // The wedge-лекало tiny VAE decode → [3, H, W] → PNG bytes (the
        // full generate chain minus the env-gated weights stages).
        let decoder = VaeDecoder::new_tiny(&tiny_vae_config(), TINY_SEED)?;
        let img = decoder.decode(&latent)?;
        let img = img.to_dtype(DType::F32).map_err(|e| e.to_string())?;
        encode_png(&img)
    };
    let png1 = run().expect("run 1");
    let png2 = run().expect("run 2");
    assert_eq!(png1, png2, "two lora_generate runs must be bit-exact");
    assert!(!png1.is_empty(), "PNG bytes are real");
}

// ── (ж) Store: roundtrip / collision / corrupted meta ────────────────

/// Bytes + meta roundtrip verbatim; collision = loud Err (no upsert);
/// corrupted meta JSON = loud Err (direct UPDATE, №242 лекало).
#[test]
fn lora_store_roundtrip_collision_and_corrupted_meta() {
    let conn = rusqlite::Connection::open_in_memory().expect("mem db");
    let bytes = tiny_adapter_blob();
    let sha = sha256_hex(&bytes);
    let meta = format!(
        r#"{{"sha256":"{}","rank":4,"alpha":2.0,"scale":0.5,"targets":2}}"#,
        sha
    );
    lora_save(&conn, "alpha-lora", &bytes, &meta).expect("save");

    // Roundtrip: the bytes and the meta come back EXACTLY as saved.
    let (got_bytes, got_meta) = lora_get(&conn, "alpha-lora").expect("get").expect("row");
    assert_eq!(got_bytes, bytes, "BLOB verbatim");
    assert_eq!(got_meta, meta, "meta verbatim");

    // Collision: plain INSERT refuses loudly.
    let err = lora_save(&conn, "alpha-lora", &bytes, &meta).expect_err("collision must be loud");
    assert!(err.contains("already exists"), "err: {}", err);
    let (b2, _) = lora_get(&conn, "alpha-lora").expect("get").expect("row");
    assert_eq!(b2, bytes, "the first adapter survives untouched");

    // Corrupted meta: a direct UPDATE breaks the JSON — the read refuses.
    conn.execute(
        "UPDATE vision_lora_adapters SET meta_json = '{ not json' WHERE name = 'alpha-lora'",
        [],
    )
    .expect("corrupt meta");
    let err = lora_get(&conn, "alpha-lora").expect_err("corrupted meta must be loud");
    assert!(err.contains("corrupted"), "err: {}", err);

    // list: sorted names for loud diagnostics.
    let conn2 = rusqlite::Connection::open_in_memory().expect("mem db 2");
    lora_save(&conn2, "zeta", &bytes, &meta).expect("save zeta");
    lora_save(&conn2, "mid", &bytes, &meta).expect("save mid");
    assert_eq!(
        lora_list(&conn2).expect("list"),
        vec!["mid".to_string(), "zeta".to_string()]
    );
}

// ── (з) Integrity: sha mismatch = loud Err BEFORE any compute ────────

fn one_decl(width: u32, height: u32) -> HashMap<String, CompiledVisionDecl> {
    let decl = CompiledVisionDecl {
        name: "poster".to_string(),
        model: "z-image-turbo".to_string(),
        steps: 8,
        width,
        height,
        seed: 42,
        policy: Some(CompiledVisionPolicy::Safe),
        profile: CompiledVisionProfile::Fp16,
    };
    HashMap::from([(decl.name.clone(), decl)])
}

/// The DB bytes no longer match the pinned meta sha → the dispatch refuses
/// loudly BEFORE env gates (the refusal does not depend on the environment).
#[test]
fn lora_generate_integrity_sha_mismatch_loud_error() {
    let conn = rusqlite::Connection::open_in_memory().expect("mem db");
    let bytes = tiny_adapter_blob();
    let mut corrupted = bytes.clone();
    corrupted[0] ^= 0xFF;
    // Save with the sha of the ORIGINAL bytes, then overwrite the BLOB —
    // the pin no longer matches the stored bytes.
    let meta = format!(
        r#"{{"sha256":"{}","rank":4,"alpha":2.0,"scale":0.5,"targets":2}}"#,
        sha256_hex(&bytes)
    );
    lora_save(&conn, "good", &bytes, &meta).expect("save");
    conn.execute(
        "UPDATE vision_lora_adapters SET bytes = ?1 WHERE name = 'good'",
        rusqlite::params![corrupted],
    )
    .expect("corrupt bytes");

    let mut reg = VisionRegistry::new();
    let decls = one_decl(1024, 1024);
    let args = vec![
        Value::String("poster".to_string()),
        Value::String("a red apple".to_string()),
        Value::String("good".to_string()),
    ];
    let err = vision_lora_generate_dispatch(&decls, &mut reg, Some(&conn), &args)
        .expect_err("integrity failure must be loud");
    assert!(
        err.contains("integrity failure"),
        "err must name the integrity check: {}",
        err
    );
    assert!(reg.is_empty(), "a failed dispatch must not insert");
}

// ── (и) lora_load: no-db / no-env / path negatives ───────────────────

/// no-db: the dispatch refuses loudly naming the db declaration — BEFORE
/// the env check (the contract refusal does not depend on the environment).
#[test]
fn lora_load_no_db_loud_error() {
    let args = vec![
        Value::String("adapter".to_string()),
        Value::String("lora/adapter.safetensors".to_string()),
    ];
    let err = vision_lora_load_dispatch(None, &args).expect_err("no-db must fail");
    assert!(err.contains("vision_lora_load"), "err: {}", err);
    assert!(err.contains("no database connection"), "err: {}", err);
    assert!(err.contains("db { url:"), "must say HOW to enable: {}", err);
}

/// no-env: with a db but without `MLOG_VISION_WEIGHTS_DIR` the dispatch
/// refuses loudly (loud-SKIP when the host has the env set).
#[test]
fn lora_load_no_env_loud_error() {
    let _g = env_lock();
    if std::env::var_os("MLOG_VISION_WEIGHTS_DIR").is_some() {
        eprintln!("LOUD SKIP: MLOG_VISION_WEIGHTS_DIR is set — no-env negative not exercisable");
        return;
    }
    let conn = rusqlite::Connection::open_in_memory().expect("mem db");
    let args = vec![
        Value::String("adapter".to_string()),
        Value::String("lora/adapter.safetensors".to_string()),
    ];
    let err = vision_lora_load_dispatch(Some(&conn), &args).expect_err("no-env must fail");
    assert!(err.contains("MLOG_VISION_WEIGHTS_DIR"), "err: {}", err);
}

/// Path safety (the contract function — env-free, лекало check_dims):
/// `..` traversal, absolute paths, non-.safetensors, missing file — all
/// loud; reading is allowed ONLY inside the weights dir.
#[test]
fn lora_load_path_safety_contract() {
    let dir = std::env::temp_dir().join(format!("n244_wd_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("lora")).expect("weights dir");
    // A real adapter file inside the weights dir (the OK case).
    std::fs::write(dir.join("lora/ok.safetensors"), tiny_adapter_blob()).expect("file");

    // `..` traversal.
    let err = vision_lora_check_adapter_path(&dir, "lora/../ok.safetensors")
        .expect_err("traversal must fail");
    assert!(err.contains("traversal"), "err: {}", err);
    // Absolute path.
    let abs = format!("{}/lora/ok.safetensors", dir.display());
    let err = vision_lora_check_adapter_path(&dir, &abs).expect_err("absolute must fail");
    assert!(err.contains("absolute"), "err: {}", err);
    // Non-.safetensors extension.
    let err = vision_lora_check_adapter_path(&dir, "lora/ok.pt")
        .expect_err("foreign extension must fail");
    assert!(err.contains(".safetensors"), "err: {}", err);
    // Missing file.
    let err = vision_lora_check_adapter_path(&dir, "lora/missing.safetensors")
        .expect_err("missing file must fail");
    assert!(err.contains("not found"), "err: {}", err);
    // OK case resolves to the file inside the weights dir.
    let full = vision_lora_check_adapter_path(&dir, "lora/ok.safetensors").expect("ok");
    assert!(full.is_file(), "the resolved path is the real file");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── (к) lora_generate dispatch negatives ─────────────────────────────

/// Unknown declaration name → loud Err with the list of declared names.
#[test]
fn lora_generate_unknown_decl_loud_error() {
    let mut reg = VisionRegistry::new();
    let decls = one_decl(1024, 1024);
    let args = vec![
        Value::String("nope".to_string()),
        Value::String("a cat".to_string()),
        Value::String("adapter".to_string()),
    ];
    let err = vision_lora_generate_dispatch(&decls, &mut reg, None, &args)
        .expect_err("unknown decl must fail");
    assert!(err.contains("not declared"), "err: {}", err);
    assert!(
        err.contains("\"poster\""),
        "lists the declared names: {}",
        err
    );
    assert!(reg.is_empty(), "failed dispatch must not insert");
}

/// Unknown lora_name → loud Err WITH the list of loaded adapters.
#[test]
fn lora_generate_unknown_lora_loud_error_with_list() {
    let conn = rusqlite::Connection::open_in_memory().expect("mem db");
    let bytes = tiny_adapter_blob();
    let meta = format!(
        r#"{{"sha256":"{}","rank":4,"alpha":2.0,"scale":0.5,"targets":2}}"#,
        sha256_hex(&bytes)
    );
    lora_save(&conn, "alpha-lora", &bytes, &meta).expect("save");
    lora_save(&conn, "beta-lora", &bytes, &meta).expect("save");

    let mut reg = VisionRegistry::new();
    let decls = one_decl(1024, 1024);
    let args = vec![
        Value::String("poster".to_string()),
        Value::String("a cat".to_string()),
        Value::String("gamma-lora".to_string()),
    ];
    let err = vision_lora_generate_dispatch(&decls, &mut reg, Some(&conn), &args)
        .expect_err("unknown lora must fail");
    assert!(
        err.contains("no adapter named 'gamma-lora'"),
        "err: {}",
        err
    );
    assert!(
        err.contains("alpha-lora") && err.contains("beta-lora"),
        "the refusal lists the loaded adapters: {}",
        err
    );
}

/// Empty prompt → loud Err (before the declaration resolution).
#[test]
fn lora_generate_empty_prompt_loud_error() {
    let mut reg = VisionRegistry::new();
    let decls = one_decl(1024, 1024);
    let args = vec![
        Value::String("poster".to_string()),
        Value::String(String::new()),
        Value::String("adapter".to_string()),
    ];
    let err = vision_lora_generate_dispatch(&decls, &mut reg, None, &args)
        .expect_err("empty prompt must fail");
    assert!(err.contains("prompt must not be empty"), "err: {}", err);
}

/// A declaration asking for a non-1024 size → loud Err AFTER the env
/// gates (the prescribed order — the test provides a minimal weights
/// TREE: the four component dirs; the env is set under the file-local
/// mutex and removed after, no cross-test race inside this binary).
#[test]
fn lora_generate_non_1024_loud_error() {
    let _g = env_lock();
    let dir = std::env::temp_dir().join(format!("n244_weights_tree_{}", std::process::id()));
    for sub in ["tokenizer", "text_encoder", "transformer", "vae"] {
        std::fs::create_dir_all(dir.join(sub)).expect("component dir");
    }
    std::env::set_var("MLOG_VISION_WEIGHTS_DIR", &dir);
    let result = std::panic::catch_unwind(|| {
        let conn = rusqlite::Connection::open_in_memory().expect("mem db");
        let bytes = tiny_adapter_blob();
        let meta = format!(
            r#"{{"sha256":"{}","rank":4,"alpha":2.0,"scale":0.5,"targets":2}}"#,
            sha256_hex(&bytes)
        );
        lora_save(&conn, "good", &bytes, &meta).expect("save");
        let mut reg = VisionRegistry::new();
        let decls = one_decl(512, 512);
        let args = vec![
            Value::String("poster".to_string()),
            Value::String("a red apple".to_string()),
            Value::String("good".to_string()),
        ];
        let err = vision_lora_generate_dispatch(&decls, &mut reg, Some(&conn), &args)
            .expect_err("non-1024 must fail");
        assert!(err.contains("fixed 1024x1024"), "err: {}", err);
        assert!(reg.is_empty(), "failed dispatch must not insert");
    });
    std::env::remove_var("MLOG_VISION_WEIGHTS_DIR");
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

// ── (л) Signing: composite provenance + 7 fields + watermark ────────

/// The composite formula is pinned byte-for-byte:
/// `sha256("{base}\nlora:{name}:{lora_sha256}")` — INCLUDING over the
/// honest "unpinned" base marker (Block 2.4).
#[test]
fn composite_model_sha256_formula_pinned() {
    let base = "unpinned";
    let lora_sha = sha256_hex(b"adapter bytes");
    let got = vision_lora_composite_model_sha256(base, "alpha-lora", &lora_sha);
    let expect = sha256_hex(format!("unpinned\nlora:alpha-lora:{}", lora_sha).as_bytes());
    assert_eq!(got, expect, "composite = sha256(base \\n lora:name:sha)");
    // A real base string works the same way.
    let got2 = vision_lora_composite_model_sha256("deadbeef", "n", "s");
    assert_eq!(got2, sha256_hex(b"deadbeef\nlora:n:s"));
    // And the composite differs from the plain base fingerprint.
    assert_ne!(got, base);
    assert_ne!(got, sha256_hex(b"unpinned"));
}

/// The signing path (the shared function behind `generate_real_with`)
/// produces a SIGNED artifact: the 7-field manifest with the composite
/// `model_sha256`, the declared policy, the fresh timestamp and the
/// watermark of the BASE model (an adapter is a delta, not a model).
#[test]
fn lora_generation_signs_always_with_composite_manifest() {
    for (policy, want) in [
        (Some(CompiledVisionPolicy::Safe), "safe"),
        (None, "unspecified"),
    ] {
        let reg = &mut VisionRegistry::new();
        let decl = CompiledVisionDecl {
            name: "poster".to_string(),
            model: "z-image-turbo".to_string(),
            steps: 8,
            width: 1024,
            height: 1024,
            seed: 42,
            policy,
            profile: CompiledVisionProfile::Fp16,
        };
        let prompt = "a red apple on a wooden table";
        // A real (tiny) PNG so the LSB watermark can travel.
        let img = Tensor::from_vec(
            generate_uniform_f32(77, 3 * 8 * 8, 0.0, 1.0),
            (3usize, 8, 8),
            &Device::Cpu,
        )
        .expect("img");
        let png = encode_png(&img).expect("png");

        let base_sha = "unpinned";
        let lora_sha = sha256_hex(b"adapter bytes");
        let composite = vision_lora_composite_model_sha256(base_sha, "alpha-lora", &lora_sha);
        let id: VisionId =
            vision_generate_sign_and_insert(reg, &decl, prompt, &png, composite.clone())
                .expect("sign + insert");
        let artifact = reg.get(id).expect("artifact");
        let m = artifact
            .manifest
            .as_ref()
            .expect("sign ALWAYS — a manifest is present");

        // The 7 fields.
        assert_eq!(
            m.model_id, "z-image-turbo",
            "model_id = the BASE model from the decl"
        );
        assert_eq!(
            m.model_sha256, composite,
            "model_sha256 = the composite fingerprint"
        );
        assert_eq!(m.seed, 42, "seed from the decl");
        assert_eq!(m.prompt_sha256, prompt_hash(prompt), "prompt hash");
        assert_eq!(
            m.policy, want,
            "policy from the decl (or the honest marker)"
        );
        assert!(
            chrono::DateTime::parse_from_rfc3339(&m.timestamp).is_ok(),
            "timestamp is RFC 3339: {}",
            m.timestamp
        );
        assert_eq!(
            m.png_sha256,
            sha256_hex(&artifact.png_bytes),
            "png_sha256 describes exactly the shipped (watermarked) bytes"
        );

        // The watermark carries the BASE model hash.
        assert_eq!(
            detect_lsb_watermark(&artifact.png_bytes).expect("detect"),
            Some(model_hash32("z-image-turbo")),
            "the watermark is the base model's (an adapter is a delta)"
        );
    }
}

/// The generate dispatch refuses a wrong-typed argument loudly (the typed
/// contract of the 3-arity family, лекало №240).
#[test]
fn lora_generate_wrong_types_loud_errors() {
    let mut reg = VisionRegistry::new();
    let decls = one_decl(1024, 1024);
    // Non-String prompt.
    let args = vec![
        Value::String("poster".to_string()),
        Value::Float(1.0),
        Value::String("adapter".to_string()),
    ];
    let err = vision_lora_generate_dispatch(&decls, &mut reg, None, &args)
        .expect_err("Float prompt must fail");
    assert!(err.contains("must be a prompt (String)"), "err: {}", err);
    // Arity 2.
    let args = vec![
        Value::String("poster".to_string()),
        Value::String("p".to_string()),
    ];
    let err = vision_lora_generate_dispatch(&decls, &mut reg, None, &args)
        .expect_err("arity 2 must fail");
    assert!(err.contains("expects 3 arguments"), "err: {}", err);
    assert!(reg.is_empty(), "failed dispatch must not insert");
}

/// no-db for the generate dispatch: the adapter's only home is SQLite
/// (ADR-0124 §6) — the refusal names the load builtin.
#[test]
fn lora_generate_no_db_loud_error() {
    let mut reg = VisionRegistry::new();
    let decls = one_decl(1024, 1024);
    let args = vec![
        Value::String("poster".to_string()),
        Value::String("a cat".to_string()),
        Value::String("adapter".to_string()),
    ];
    let err =
        vision_lora_generate_dispatch(&decls, &mut reg, None, &args).expect_err("no-db must fail");
    assert!(err.contains("no database connection"), "err: {}", err);
    assert!(
        err.contains("vision_lora_load"),
        "must say HOW to get an adapter: {}",
        err
    );
}
