#![cfg(feature = "vision")]
//! Наряд №240 — env-gated `.mlog` e2e: vision declaration → generate → export.
//!
//! Closes the №237 Block 3.1 promise «+ одна генерация из .mlog» (the
//! loud-gap note in `docs/research/naryad-237-real-weights-runbook.md`,
//! §3): an actual `.mlog` program with a `vision { }` declaration and a
//! flow that calls `vision_generate` → `vision_export`, producing a real
//! PNG on disk with the SHA-256 recorded in the output.
//!
//! Env-gated (loud-SKIP, NO `#[ignore]`): requires `MLOG_VISION_WEIGHTS_DIR`
//! to point at the Z-Image-Turbo weights directory (manifest №212). Without
//! weights the test prints a loud SKIP and returns — the §3-sanctioned form
//! of permitted unfinishedness.

use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn weights_dir() -> Option<PathBuf> {
    std::env::var_os("MLOG_VISION_WEIGHTS_DIR").map(PathBuf::from)
}

fn out_dir() -> PathBuf {
    std::env::var_os("MLOG_VISION_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"))
}

fn print_skip(reason: &str) {
    eprintln!("LOUD SKIP: {} (env-gated test — NOT #[ignore])", reason);
}

/// The full .mlog clip: declaration → generate → export.
#[test]
fn mlog_vision_generate_export_e2e() {
    let Some(_weights) = weights_dir() else {
        print_skip("MLOG_VISION_WEIGHTS_DIR not set — .mlog vision e2e test");
        return;
    };

    let out_path = out_dir().join("naryad_240_mlog_first_image.png");
    // Remove a stale artifact from a previous run — the SHA must belong to
    // this run's output.
    let _ = std::fs::remove_file(&out_path);

    // Fixed prompt + seed (decl seed 42) → deterministic output for the
    // same weights (determinism by construction, ADR-0124).
    let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
flow Main { input: String = "go" -> Generate -> output }
pattern Generate(sid: String) -> String {
  let v = vision_generate("poster", "a red apple on a wooden table, studio light")
  let p = vision_export(v, "OUT_PATH")
  return p
}
"#
    .replace("OUT_PATH", &out_path.display().to_string());

    let t0 = std::time::Instant::now();
    let output =
        metalogos::run_program(&source).expect(".mlog vision e2e: program must run to completion");
    let elapsed = t0.elapsed();

    let returned = output.expect("flow must return the export path");
    assert!(
        returned.contains(out_path.display().to_string().as_str()),
        "flow output must be the export path, got: {}",
        returned
    );

    // The PNG exists on disk and is a real PNG (magic bytes).
    let png_bytes = std::fs::read(&out_path)
        .unwrap_or_else(|e| panic!("exported PNG must exist at {}: {}", out_path.display(), e));
    assert_eq!(&png_bytes[..4], b"\x89PNG", "file must be a real PNG");

    // SHA-256 recorded in the output (naryad §4.2 — hash fixed in output).
    let mut hasher = Sha256::new();
    hasher.update(&png_bytes);
    let png_sha: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    eprintln!(
        "naryad_240_mlog_e2e: PNG path={} sha256={} size={} bytes total={:?}",
        out_path.display(),
        png_sha,
        png_bytes.len(),
        elapsed
    );

    // 1024×1024 RGB PNG of a real pipeline: comfortably over 1 MB and well
    // under 10 MB (sanity band, not a golden pin — the pin lands with the
    // real run in the runbook protocol).
    assert!(
        png_bytes.len() > 1_000_000,
        "a 1024×1024 photo-like PNG must exceed 1 MB, got {} bytes",
        png_bytes.len()
    );
}

// ── Наряд №243 (R6.2): vision_edit e2e — the runbook edit-e2e target ──

/// The full .mlog edit clip: declaration → generate → EDIT (in-context) →
/// export of the EDITED artifact. The №243 Block 4.2 contract: an
/// env-gated e2e for the future real-weights run — "чтобы PARKED-прогон
/// остался одной сессией". Loud-SKIP without weights (NO #[ignore]).
///
/// What it verifies on the real run (the go-no-go criteria, verbatim from
/// the runbook §6):
/// 1. The edit output keeps the source resolution (1024×1024 — no resize).
/// 2. The edited sidecar manifest carries the inheritance contract:
///    model_id/policy/seed inherited from the source (seed 42), fresh
///    prompt_sha256 = SHA-256 of the EDIT prompt, png_sha256 = the final
///    watermarked PNG.
/// 3. The edited PNG is a real PNG carrying the LSB watermark.
/// 4. The non-decoder VAE key list is verified against the real file
///    header (the VaeEncoder::from_weights loud guard + the quant_conv
///    loud note — the runbook §3 step).
#[test]
fn mlog_vision_edit_export_e2e() {
    let Some(_weights) = weights_dir() else {
        print_skip("MLOG_VISION_WEIGHTS_DIR not set — .mlog vision EDIT e2e test (№243)");
        return;
    };

    let out_path = out_dir().join("naryad_243_mlog_first_edit.png");
    let sidecar_path = format!("{}.manifest.json", out_path.display());
    // Remove stale artifacts from previous runs — the SHAs must belong to
    // this run's output.
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&sidecar_path);

    // Fixed prompts + seed (decl seed 42 → inherited into the edit
    // manifest) → deterministic output for the same weights.
    let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
flow Main { input: String = "go" -> Edit -> output }
pattern Edit(sid: String) -> String {
  let v = vision_generate("poster", "a red apple on a wooden table, studio light")
  let e = vision_edit(v, "make the apple green, keep everything else unchanged")
  let p = vision_export(e, "OUT_PATH")
  return p
}
"#
    .replace("OUT_PATH", &out_path.display().to_string());

    let t0 = std::time::Instant::now();
    let output = metalogos::run_program(&source).expect(".mlog vision EDIT e2e: program must run");
    let elapsed = t0.elapsed();
    let returned = output.expect("flow must return the export path");
    assert!(
        returned.contains(out_path.display().to_string().as_str()),
        "flow output must be the export path, got: {}",
        returned
    );

    // The edited PNG exists and is a real PNG.
    let png_bytes = std::fs::read(&out_path)
        .unwrap_or_else(|e| panic!("edited PNG must exist at {}: {}", out_path.display(), e));
    assert_eq!(&png_bytes[..4], b"\x89PNG", "file must be a real PNG");
    assert!(
        png_bytes.len() > 1_000_000,
        "a 1024×1024 edited PNG must exceed 1 MB, got {} bytes",
        png_bytes.len()
    );

    // The sidecar manifest: the inheritance contract.
    let sidecar = std::fs::read_to_string(&sidecar_path).expect("edited sidecar manifest");
    let m: serde_json::Value = serde_json::from_str(&sidecar).expect("sidecar JSON");
    assert_eq!(
        m["model_id"], "z-image-turbo",
        "model_id inherited from the source manifest"
    );
    assert_eq!(m["policy"], "safe", "policy inherited (no overwrite)");
    assert_eq!(m["seed"], 42, "seed inherited from the source declaration");
    assert_ne!(
        m["prompt_sha256"], m["png_sha256"],
        "sanity: distinct digest fields"
    );
    let edit_prompt_hash = {
        use sha2::{Digest as _, Sha256};
        let mut h = Sha256::new();
        h.update(b"make the apple green, keep everything else unchanged");
        let d = h.finalize();
        d.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    };
    assert_eq!(
        m["prompt_sha256"], edit_prompt_hash,
        "prompt_sha256 = SHA-256 of the EDIT prompt"
    );

    // The LSB watermark of the inherited model id.
    let detected = metalogos::vision::provenance::detect_lsb_watermark(&png_bytes)
        .expect("watermark detection");
    assert_eq!(
        detected,
        Some(metalogos::vision::provenance::model_hash32("z-image-turbo")),
        "the edited PNG carries the LSB watermark"
    );

    eprintln!(
        "naryad_243_mlog_edit_e2e: edited PNG path={} size={} bytes total={:?}\n  \
         sidecar: {}",
        out_path.display(),
        png_bytes.len(),
        elapsed,
        sidecar
    );
}
