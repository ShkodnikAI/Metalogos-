//! Наряд №320 (issue #407) — C2PA mini-slice: EU AI Act Art. 50 synthetic
//! marking on egress (ADR-0152).
//!
//! Tests:
//! - (а) raw egress of a synthetic (signed) artifact → loud runtime refusal
//!   `MEDIA_SYNTHETIC_UNMARKED` (static gate is covered in src/audit.rs tests);
//! - (б) marked egress (`vision_export`) → OK, sidecar ships `synthetic: true`;
//! - (в) roundtrip: generate (sign_and_insert) → save → load → `synthetic`
//!   preserved;
//! - (г) sidecar external-validator: the JSON structure carries the required
//!   C2PA-shaped fields (no COSE/JUMBF validation in this slice — loud honest
//!   boundary, ADR-0152 D5);
//! - generation-constructors enumeration: every `VisionManifest {` literal in
//!   the generation/store sources sets `synthetic: true` (ADR-0152 D1);
//! - sidecar read path (`sidecar_read_report`): missing/corrupt manifest is a
//!   loud report, never a silent default.

use metalogos::builtins::vision::{vision_export_dispatch, vision_export_raw_dispatch};
use metalogos::interpreter::Value;
use metalogos::vision::provenance::{sidecar_read_report, VisionManifest};
use metalogos::vision::{VisionArtifact, VisionRegistry};

fn signed_synthetic(png: Vec<u8>) -> VisionArtifact {
    VisionArtifact {
        png_bytes: png,
        manifest: Some(VisionManifest {
            model_id: "z-image-turbo".to_string(),
            model_sha256: "unpinned".to_string(),
            seed: 42,
            prompt_sha256: "cafebabe".to_string(),
            policy: "safe".to_string(),
            timestamp: "2026-09-14T00:00:00+00:00".to_string(),
            png_sha256: metalogos::vision::provenance::sha256_hex(&[9, 9, 9]),
            synthetic: true,
        }),
    }
}

// ── (а) raw egress of synthetic content is refused ─────────────────────

#[test]
fn n320_raw_export_refuses_synthetic_artifact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("synthetic.png");
    let mut reg = VisionRegistry::new();
    let id = reg.insert(signed_synthetic(vec![1, 2, 3]));
    let args = vec![Value::Vision(id), Value::String(path.display().to_string())];
    let err = vision_export_raw_dispatch(&reg, &args)
        .expect_err("raw egress of synthetic content must be refused (ADR-0152 D3)");
    assert!(
        err.contains("MEDIA_SYNTHETIC_UNMARKED"),
        "gate check-id must be named: {}",
        err
    );
    assert!(
        err.contains("Art. 50"),
        "the reason must cite Art. 50: {}",
        err
    );
    assert!(
        !path.exists(),
        "the refused raw export must not write any bytes"
    );
}

#[test]
fn n320_raw_export_refuses_manifestless_artifact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("unmarked.png");
    let mut reg = VisionRegistry::new();
    let id = reg.insert(VisionArtifact {
        png_bytes: vec![4, 5, 6],
        manifest: None,
    });
    let args = vec![Value::Vision(id), Value::String(path.display().to_string())];
    let err = vision_export_raw_dispatch(&reg, &args)
        .expect_err("unmarked (manifest-less) egress must be refused");
    assert!(err.contains("MEDIA_SYNTHETIC_UNMARKED"), "{}", err);
}

#[test]
fn n320_non_synthetic_artifact_raw_exports() {
    // The `synthetic: false` flag exists for the future ingest contour
    // (foreign non-synthetic media); raw egress stays legal for it
    // (ADR-0152 D3) — the gate is about SYNTHETIC marking, not about raw
    // per se.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("foreign.png");
    let mut artifact = signed_synthetic(vec![7, 8]);
    if let Some(m) = artifact.manifest.as_mut() {
        m.synthetic = false;
    }
    let mut reg = VisionRegistry::new();
    let id = reg.insert(artifact);
    let args = vec![Value::Vision(id), Value::String(path.display().to_string())];
    vision_export_raw_dispatch(&reg, &args).expect("non-synthetic raw egress is legal");
    assert_eq!(std::fs::read(&path).expect("bytes"), vec![7, 8]);
}

// ── (б) marked egress ships the manifest with synthetic: true ──────────

#[test]
fn n320_marked_export_ships_synthetic_sidecar() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("marked.png");
    let mut reg = VisionRegistry::new();
    let id = reg.insert(signed_synthetic(vec![1, 1, 1]));
    let args = vec![Value::Vision(id), Value::String(path.display().to_string())];
    vision_export_dispatch(&reg, &args).expect("marked egress must succeed");
    let sidecar_path = dir.path().join("marked.png.manifest.json");
    let json = std::fs::read_to_string(&sidecar_path).expect("sidecar written");
    // (г) external-validator: the sidecar is structurally valid JSON with
    // the required C2PA-shaped fields (COSE/JUMBF validation is №337 —
    // loud honest boundary, ADR-0152 D5).
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON sidecar");
    for field in [
        "model_id",
        "model_sha256",
        "seed",
        "prompt_sha256",
        "policy",
        "timestamp",
        "png_sha256",
        "synthetic",
    ] {
        assert!(
            parsed.get(field).is_some(),
            "sidecar missing required field {}",
            field
        );
    }
    assert_eq!(parsed["synthetic"], true, "Art. 50 marking present");
}

// ── (в) generation → persistence roundtrip preserves synthetic ─────────

/// Generation-path tests run under `--features vision` (the sign/insert
/// contract is feature-gated, №240); they execute in the vision-tests CI job.
#[cfg(feature = "vision")]
mod generation_roundtrip {
    use super::*;
    use metalogos::builtins::vision::{
        vision_generate_sign_and_insert, vision_load_dispatch, vision_save_dispatch,
    };
    use metalogos::bytecode::{CompiledVisionDecl, CompiledVisionPolicy, CompiledVisionProfile};

    /// A real decodable tiny PNG — the sign/insert path embeds the LSB
    /// watermark, which decodes the image (image codec is vision-gated).
    fn tiny_png() -> Vec<u8> {
        let img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::from_fn(16, 16, |x, y| {
                image::Rgb([(x * 7) as u8, (y * 5) as u8, 128])
            });
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("tiny PNG encode");
        out
    }

    fn tiny_decl() -> CompiledVisionDecl {
        CompiledVisionDecl {
            name: "n320_decl".to_string(),
            model: "z-image-turbo".to_string(),
            steps: 2,
            width: 8,
            height: 8,
            seed: 7,
            policy: Some(CompiledVisionPolicy::Safe),
            profile: CompiledVisionProfile::Fp16,
        }
    }

    #[test]
    fn n320_generation_marks_synthetic() {
        let mut reg = VisionRegistry::new();
        let id = vision_generate_sign_and_insert(
            &mut reg,
            &tiny_decl(),
            "a test scene",
            &tiny_png(),
            "unpinned".to_string(),
        )
        .expect("generation contract inserts");
        let artifact = reg.get(id).expect("present");
        let manifest = artifact.manifest.as_ref().expect("signed by construction");
        assert!(
            manifest.synthetic,
            "every generation path must mark synthetic: true (ADR-0152 D1)"
        );
    }

    #[test]
    fn n320_generate_save_load_roundtrip_preserves_synthetic() {
        let mut reg_a = VisionRegistry::new();
        let id = vision_generate_sign_and_insert(
            &mut reg_a,
            &tiny_decl(),
            "roundtrip scene",
            &tiny_png(),
            "unpinned".to_string(),
        )
        .expect("generate");

        let conn = rusqlite::Connection::open_in_memory().expect("in-memory sqlite");
        vision_save_dispatch(
            &reg_a,
            Some(&conn),
            &[Value::Vision(id), Value::String("n320_art".to_string())],
        )
        .expect("save");

        let mut reg_b = VisionRegistry::new();
        let loaded = vision_load_dispatch(
            &mut reg_b,
            Some(&conn),
            &[Value::String("n320_art".to_string())],
        )
        .expect("load");
        let Value::Vision(id_b) = loaded else {
            panic!("expected a Vision handle");
        };
        let artifact = reg_b.get(id_b).expect("present after load");
        let manifest = artifact.manifest.as_ref().expect("provenance survives DB");
        assert!(
            manifest.synthetic,
            "synthetic must survive generate → save → load (ADR-0152 D1)"
        );
    }
}

// ── generation-constructor enumeration (ADR-0152 D1) ────────────────────

/// Every `VisionManifest {` literal construction in the generation/store
/// sources sets `synthetic` — no generation → egress path may remain
/// unmarked. Source-level enumeration (the constructors are the writers).
#[test]
fn n320_all_manifest_constructors_set_synthetic() {
    for src_path in [
        "src/builtins/vision.rs",
        "src/vision/store.rs",
        "src/vision/provenance.rs",
    ] {
        let src = std::fs::read_to_string(src_path).expect(src_path);
        let mut rest: &str = &src;
        let mut sites = 0;
        while let Some(i) = rest.find("VisionManifest {") {
            // Skip the struct DEFINITION (`pub struct VisionManifest {`).
            let prefix = &rest[..i];
            if !prefix.ends_with("pub struct ") && !prefix.ends_with("struct ") {
                sites += 1;
                // The literal body is ~10 short fields; a 1000-char window
                // from the constructor opening always covers it (the first
                // `}` is inside the policy `match`, not the literal end).
                let window = &rest[i..(i + 1000).min(rest.len())];
                assert!(
                    window.contains("synthetic:"),
                    "{}: a VisionManifest constructor does not set `synthetic` \
                     (ADR-0152 D1 — no unmarked generation path):\n{}",
                    src_path,
                    &window[..window.len().min(400)]
                );
            }
            rest = &rest[i + 16..];
        }
        assert!(sites > 0, "{}: expected constructions", src_path);
    }
}

// ── sidecar read path (ADR-0152 D4) ─────────────────────────────────────

#[test]
fn n320_sidecar_read_extracts_synthetic() {
    let json = r#"{
        "model_id": "z-image-turbo",
        "model_sha256": "unpinned",
        "seed": 42,
        "prompt_sha256": "cafebabe",
        "policy": "safe",
        "timestamp": "2026-09-14T00:00:00+00:00",
        "png_sha256": "abc",
        "synthetic": true
    }"#;
    let manifest = sidecar_read_report(json).expect("valid sidecar parses");
    assert!(manifest.synthetic, "synthetic extracted from the sidecar");
}

#[test]
fn n320_sidecar_read_default_synthetic_true_for_pre_n320_sidecars() {
    // A pre-№320 sidecar has NO synthetic field — the conservative read is
    // `true` (the only historical writer was the generation path).
    let json = r#"{
        "model_id": "z-image-turbo",
        "model_sha256": "unpinned",
        "seed": 42,
        "prompt_sha256": "cafebabe",
        "policy": "safe",
        "timestamp": "2026-09-14T00:00:00+00:00",
        "png_sha256": "abc"
    }"#;
    let manifest = sidecar_read_report(json).expect("parses with the default");
    assert!(
        manifest.synthetic,
        "unknown ⇒ marked (ADR-0152 D1 conservative default)"
    );
}

#[test]
fn n320_sidecar_read_loud_on_missing_and_corrupt() {
    let err = sidecar_read_report("").expect_err("empty sidecar is a loud report");
    assert!(err.contains("MEDIA_SYNTHETIC_UNMARKED"), "{}", err);
    let err = sidecar_read_report("not json at all").expect_err("corrupt sidecar is a loud report");
    assert!(err.contains("MEDIA_SYNTHETIC_UNMARKED"), "{}", err);
}
