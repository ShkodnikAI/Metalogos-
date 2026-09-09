//! Vision provenance — signing MVP (Наряд №241, R5; ADR-0125).
//!
//! The core promise of the Vision pillar (ADR-0125): "the agent cannot
//! accidentally ship unsigned media" — security is a type, not a
//! procedure. This module is the mechanism:
//!
//! - [`VisionManifest`] — the provenance record (model-id + weights
//!   SHA, seed, prompt-hash, policy, timestamp, SHA-256 of the final
//!   PNG). Stored on every [`crate::vision::VisionArtifact`] produced by
//!   `vision_generate` and written next to every default export as the
//!   sidecar `<path>.manifest.json`.
//! - LSB watermark — a steganographic mark in the least-significant bits
//!   of the RGB channels of the artifact PNG. Payload = 32-bit magic
//!   `"MLGV"` + 32-bit model-hash (first 4 bytes of SHA-256(model_id)) —
//!   compact and deterministic. **Honest boundary (ADR-0125):** this is a
//!   detectable-by-us MVP, NOT adversarially robust watermarking
//!   (resize/JPEG survives nothing — that is research backlog, phase 2).
//!
//! ## Honest-boundary notes (loud, per ADR-0125)
//!
//! - `model_sha256` is a *weights-tree fingerprint*: SHA-256 over the
//!   sorted `filename=sha256` lines of the pinned `manifest.json` from
//!   the weights directory (cheap and deterministic; reuses
//!   `WeightsManifest`, does not re-hash gigabytes). When the weights
//!   directory has no `manifest.json`, the honest marker `"unpinned"` is
//!   recorded — the absence of pinning is loud in the artifact itself.
//! - `timestamp` is wall-clock (RFC 3339, UTC) and is deliberately NOT
//!   pinned in tests — only presence/format is asserted.
//!
//! ## Feature boundaries
//!
//! The manifest + hashing layer compiles in ALL builds (no `image`
//! dependency) — the non-gated `VisionArtifact` carries it. The LSB
//! watermark needs PNG decode/encode (`image` crate) and is therefore
//! feature-gated behind `vision` — exactly like the generation pipeline
//! that produces the bytes it marks.

use sha2::{Digest, Sha256};

// ── Non-gated: manifest + hashing ────────────────────────────────────

/// Provenance manifest of a generated vision artifact (Наряд №241
/// Block 1.2; ADR-0125 "Manifest JSON" MVP).
///
/// Written on every `vision_generate` (stored on the artifact) and on
/// every default `vision_export` (sidecar `<path>.manifest.json`).
/// `timestamp` is wall-clock and not pinned in tests — presence/format
/// only (Наряд №241 Block 1.2).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VisionManifest {
    /// Model id as declared (`z-image-turbo` — R4.1 SSOT list).
    pub model_id: String,
    /// Weights-tree fingerprint: SHA-256 over the sorted
    /// `filename=sha256` lines of the pinned `manifest.json`; the honest
    /// marker `"unpinned"` when no manifest.json is present (see module
    /// docs — the absence of pinning is loud, never faked).
    pub model_sha256: String,
    /// Fixed generation seed (reproducibility by construction, ADR-0124).
    pub seed: u64,
    /// SHA-256 of the prompt string (not the prompt itself — the prompt
    /// is user content, only its fingerprint is recorded).
    pub prompt_sha256: String,
    /// Usage policy from the `vision { }` declaration: `"safe"`, or the
    /// honest marker `"unspecified"` when the declaration omitted the
    /// field (R5 policy-relax per ADR-0125 — Block 3.1).
    pub policy: String,
    /// Wall-clock generation time, RFC 3339 UTC (not pinned in tests).
    pub timestamp: String,
    /// SHA-256 of the FINAL exported PNG bytes — i.e. after the LSB
    /// watermark was embedded (the manifest describes exactly the bytes
    /// shipped next to it).
    pub png_sha256: String,
}

/// SHA-256 of arbitrary bytes, lowercase hex. Reused by the
/// `vision_fetch_weights` SHA-pinning verification (Block 3.2) — one
/// hashing path for the whole pillar.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// SHA-256 of the prompt string (Наряд №241 Block 1.2: "prompt-hash
/// (SHA-256 строки промпта)").
pub fn prompt_hash(prompt: &str) -> String {
    sha256_hex(prompt.as_bytes())
}

/// Verify a downloaded file against its manifest SHA-256 pin
/// (Наряд №241 Block 3.2г). Public so the pin contract is mechanically
/// testable WITHOUT network or weights: the contract test feeds synthetic
/// bytes and asserts the loud mismatch refusal (expected vs computed).
pub fn verify_sha_pin(name: &str, expected_sha: &str, bytes: &[u8]) -> Result<(), String> {
    let computed = sha256_hex(bytes);
    let expected = expected_sha.to_lowercase();
    if computed != expected {
        return Err(format!(
            "MODEL_WEIGHTS_UNSAFE: SHA-256 pin mismatch for '{}' — expected {}, computed {} \
             (file NOT written; poisoned or corrupted weights, ADR-0125)",
            name, expected, computed
        ));
    }
    Ok(())
}

/// Serialize a manifest to the sidecar JSON form (pretty-printed, so the
/// sidecar stays human-auditable — provenance you cannot read is
/// provenance you cannot verify).
pub fn manifest_sidecar_json(manifest: &VisionManifest) -> Result<String, String> {
    serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("provenance: manifest serialization failed: {}", e))
}

/// Weights-tree fingerprint (see [`VisionManifest::model_sha256`]).
///
/// Feature-gated: reads `manifest.json` via `WeightsManifest::load_from_dir`
/// (reuse, not duplication — `src/vision/weights.rs` is NOT modified).
#[cfg(feature = "vision")]
pub fn weights_tree_sha256(weights_dir: &std::path::Path) -> Result<String, String> {
    let loaded = crate::vision::weights::WeightsManifest::load_from_dir(weights_dir)?;
    let Some(manifest) = loaded else {
        // Honest marker — the weights tree carries no pinned SHA list.
        // Recorded as-is into the manifest (loud absence, never faked).
        return Ok("unpinned".to_string());
    };
    let mut lines: Vec<String> = manifest
        .entries
        .iter()
        .map(|e| format!("{}={}", e.filename, e.sha256.to_lowercase()))
        .collect();
    lines.sort();
    Ok(sha256_hex(lines.join("\n").as_bytes()))
}

// ── Gated: LSB watermark (needs `image` for PNG decode/encode) ──────

/// Magic prefix of the watermark payload: `"MLGV"` (METALOGOS Vision).
#[cfg(feature = "vision")]
pub const WATERMARK_MAGIC: u32 = 0x4D4C_4756;

/// Payload length in bits: 32 magic bits + 32 model-hash bits.
#[cfg(feature = "vision")]
pub const WATERMARK_BITS: usize = 64;

/// First 4 bytes of SHA-256(model_id) — the compact deterministic model
/// fingerprint carried by the watermark payload (Наряд №241 Block 1.1:
/// "магия + id модели-хэш").
#[cfg(feature = "vision")]
pub fn model_hash32(model_id: &str) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(model_id.as_bytes());
    let result = hasher.finalize();
    [result[0], result[1], result[2], result[3]]
}

/// The 64 watermark bits (MSB-first per 32-bit word): magic then
/// model-hash.
#[cfg(feature = "vision")]
fn watermark_bits(model_id: &str) -> [u8; WATERMARK_BITS] {
    let magic = WATERMARK_MAGIC.to_be_bytes();
    let hash = model_hash32(model_id);
    let mut bits = [0u8; WATERMARK_BITS];
    for (i, byte) in magic.iter().chain(hash.iter()).enumerate() {
        for b in 0..8 {
            bits[i * 8 + b] = (byte >> (7 - b)) & 1;
        }
    }
    bits
}

/// Embed the LSB watermark into a PNG (Наряд №241 Block 1.1).
///
/// Decodes the PNG, flips the least-significant bit of each RGB channel
/// byte to the payload bit (row-major pixels, R→G→B per pixel, MSB-first
/// payload), re-encodes. Decoding pixel data then reading LSBs is the
/// mechanical detection path — the same one the unit test and
/// `detect_lsb_watermark` use.
///
/// Loud errors: decode failure, capacity (< 64 channel bytes), encode
/// failure. No silent pass-through — an unsigned artifact must never be
/// produced by a "signed" call (Block 1.3).
#[cfg(feature = "vision")]
pub fn embed_lsb_watermark(png_bytes: &[u8], model_id: &str) -> Result<Vec<u8>, String> {
    use image::ImageBuffer;

    let bits = watermark_bits(model_id);
    let img = image::load_from_memory(png_bytes)
        .map_err(|e| format!("provenance: watermark embed — PNG decode failed: {}", e))?;
    let rgb: ImageBuffer<image::Rgb<u8>, Vec<u8>> = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let capacity = (w as usize) * (h as usize) * 3;
    if capacity < WATERMARK_BITS {
        return Err(format!(
            "provenance: watermark embed — capacity too small: image {}x{} has {} \
             channel bytes, need >= {} for the payload",
            w, h, capacity, WATERMARK_BITS
        ));
    }
    let mut idx = 0usize;
    let mut marked = rgb;
    for pixel in marked.pixels_mut() {
        for channel in pixel.0.iter_mut() {
            if idx >= WATERMARK_BITS {
                break;
            }
            // Force the LSB to the payload bit (clear, then set) — never
            // XOR-flip, so embedding is idempotent and deterministic.
            *channel = (*channel & 0xFE) | bits[idx];
            idx += 1;
        }
        if idx >= WATERMARK_BITS {
            break;
        }
    }
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(marked)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("provenance: watermark embed — PNG re-encode failed: {}", e))?;
    Ok(out)
}

/// Detect the LSB watermark in a PNG: decode pixel data, read the 64
/// LSBs (same order as [`embed_lsb_watermark`]), check the magic.
/// Returns `Some(model_hash32)` when the magic matches, `None` for an
/// unmarked (or differently-marked) image. Decode errors are loud.
#[cfg(feature = "vision")]
pub fn detect_lsb_watermark(png_bytes: &[u8]) -> Result<Option<[u8; 4]>, String> {
    use image::ImageBuffer;

    let img = image::load_from_memory(png_bytes)
        .map_err(|e| format!("provenance: watermark detect — PNG decode failed: {}", e))?;
    let rgb: ImageBuffer<image::Rgb<u8>, Vec<u8>> = img.to_rgb8();
    let capacity = (rgb.width() as usize) * (rgb.height() as usize) * 3;
    if capacity < WATERMARK_BITS {
        return Ok(None);
    }
    let mut bits = [0u8; WATERMARK_BITS];
    let mut idx = 0usize;
    'outer: for pixel in rgb.pixels() {
        for channel in pixel.0.iter() {
            if idx >= WATERMARK_BITS {
                break 'outer;
            }
            bits[idx] = channel & 1;
            idx += 1;
        }
    }
    let mut bytes = [0u8; 8];
    for (i, byte) in bytes.iter_mut().enumerate() {
        let mut acc = 0u8;
        for b in 0..8 {
            acc = (acc << 1) | bits[i * 8 + b];
        }
        *byte = acc;
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != WATERMARK_MAGIC {
        return Ok(None);
    }
    Ok(Some([bytes[4], bytes[5], bytes[6], bytes[7]]))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Non-gated: manifest serde roundtrip + hashing determinism.
    #[test]
    fn manifest_serde_roundtrip_preserves_fields() {
        let m = VisionManifest {
            model_id: "z-image-turbo".to_string(),
            model_sha256: "abc123".to_string(),
            seed: 42,
            prompt_sha256: "def456".to_string(),
            policy: "safe".to_string(),
            timestamp: "2026-09-09T00:00:00+00:00".to_string(),
            png_sha256: "789abc".to_string(),
        };
        let json = manifest_sidecar_json(&m).expect("sidecar json");
        let parsed: VisionManifest = serde_json::from_str(&json).expect("sidecar parse-back");
        assert_eq!(m, parsed);
    }

    #[test]
    fn manifest_sidecar_contains_all_seven_fields() {
        // Наряд №241 Block 4.2: manifest-поля presence.
        let m = VisionManifest {
            model_id: "m".into(),
            model_sha256: "s".into(),
            seed: 1,
            prompt_sha256: "p".into(),
            policy: "unspecified".into(),
            timestamp: "t".into(),
            png_sha256: "x".into(),
        };
        let json = manifest_sidecar_json(&m).expect("sidecar json");
        for field in [
            "model_id",
            "model_sha256",
            "seed",
            "prompt_sha256",
            "policy",
            "timestamp",
            "png_sha256",
        ] {
            assert!(
                json.contains(&format!("\"{}\"", field)),
                "missing {}",
                field
            );
        }
    }

    #[test]
    fn sha256_hex_is_deterministic_and_hex() {
        let a = sha256_hex(b"metalogos");
        let b = sha256_hex(b"metalogos");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn prompt_hash_matches_sha256_of_string() {
        assert_eq!(
            prompt_hash("a red apple"),
            sha256_hex("a red apple".as_bytes())
        );
    }

    // Gated: LSB watermark roundtrip on a synthetic tensor — weights not
    // needed (Наряд №241 Block 1.1: "unit-тест на синтетическом тензоре
    // encode→decode→биты на месте").
    #[cfg(feature = "vision")]
    mod watermark {
        use super::*;

        fn synthetic_png(w: u32, h: u32) -> Vec<u8> {
            use candle_core::{DType, Device, Tensor};
            // Deterministic non-trivial pixel data in [0, 1].
            let n = (3 * w * h) as usize;
            let data: Vec<f32> = (0..n).map(|i| ((i * 37 % 251) as f32) / 251.0).collect();
            let img = Tensor::from_vec(data, (3usize, h as usize, w as usize), &Device::Cpu)
                .expect("synthetic tensor");
            crate::vision::vae::encode_png(&img.to_dtype(DType::F32).expect("dtype"))
                .expect("encode_png")
        }

        #[test]
        fn watermark_roundtrip_bits_on_place() {
            let png = synthetic_png(64, 64);
            let model_id = "z-image-turbo";

            // Unmarked image: no watermark.
            let none = detect_lsb_watermark(&png).expect("detect");
            assert!(
                none.is_none(),
                "synthetic PNG must be unmarked before embed"
            );

            // Embed → detect: magic + exact model hash.
            let marked = embed_lsb_watermark(&png, model_id).expect("embed");
            let detected = detect_lsb_watermark(&marked).expect("detect");
            assert_eq!(
                detected,
                Some(model_hash32(model_id)),
                "watermark roundtrip must return the model hash"
            );

            // Bits on place, literally: first 8 LSBs of the marked image
            // decode to the magic's first byte ('M' = 0x4D).
            let img = image::load_from_memory(&marked).expect("decode");
            let rgb = img.to_rgb8();
            let mut first_byte = 0u8;
            let mut idx = 0;
            'outer: for pixel in rgb.pixels() {
                for channel in pixel.0.iter() {
                    first_byte = (first_byte << 1) | (channel & 1);
                    idx += 1;
                    if idx == 8 {
                        break 'outer;
                    }
                }
            }
            assert_eq!(first_byte, 0x4D, "first payload byte must be 'M'");

            // Idempotence: embedding twice yields the same detectable hash.
            let twice = embed_lsb_watermark(&marked, model_id).expect("re-embed");
            assert_eq!(
                detect_lsb_watermark(&twice).expect("detect"),
                Some(model_hash32(model_id)),
                "embedding must be idempotent (force-set, not XOR-flip)"
            );
        }

        #[test]
        fn watermark_model_hash_distinguishes_ids() {
            let a = model_hash32("z-image-turbo");
            let b = model_hash32("other-model");
            assert_ne!(a, b, "different model ids must hash differently");
            // Determinism: same id → same hash.
            assert_eq!(model_hash32("z-image-turbo"), a);
        }

        #[test]
        fn watermark_tiny_image_is_loud_error() {
            // 4x4 = 48 channel bytes < 64 — capacity refusal, not a silent
            // unmarked passthrough (Block 1.3).
            let png = synthetic_png(4, 4);
            let err = embed_lsb_watermark(&png, "z-image-turbo")
                .expect_err("capacity refusal must be loud");
            assert!(err.contains("capacity"), "got: {}", err);
        }
    }
}
