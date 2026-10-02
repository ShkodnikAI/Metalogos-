//! The VAE FS-glue façade (№545 (в), issue #883): the heavy VAE
//! machinery (encoder/decoder/configs) lives in the metalogos-reflex
//! crate; this module re-exports it and hosts the ONE function whose
//! home is the language crate — `save_png`, whose write half goes
//! through the №475 fs gate (the language's security perimeter; the
//! reflex crate is gate-free by design).
//!
//! Every historical `crate::vision::vae::*` path resolves here: the
//! glob re-export preserves the machinery surface, `save_png` keeps
//! its original signature (the naryad's no-API-change contract).

#[cfg(feature = "vision")]
pub use metalogos_reflex::vision::vae::*;

/// Save a `[3, H, W]` F32 image tensor (in [0,1]) as a PNG file —
/// the encoding comes from the reflex crate, the write goes through
/// the №475 fs gate. Bit-identical to `encode_png`'s bytes (№240).
#[cfg(feature = "vision")]
pub fn save_png(img: &candle_core::Tensor, path: &std::path::Path) -> Result<(), String> {
    let bytes = metalogos_reflex::vision::vae::encode_png(img)?;
    crate::fs_gate::write_bytes(&path.to_string_lossy(), "vision save_png", &bytes)
        .map_err(|e| format!("save_png: write to {}: {}", path.display(), e))
}
