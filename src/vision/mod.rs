//! Vision pillar — skeleton module (Наряд №210, ADR-0124).
//!
//! Implements the opaque handle pattern for vision artifacts, mirroring
//! the Reflex pillar's `ReflexId` / `ReflexRegistry` design (ADR-0114).
//!
//! `VisionId` and `VisionRegistry` are NOT feature-gated — they work in
//! the base build without `--features vision`. The actual inference stack
//! (model loading, generation) lands in R2/R3 (naryads 211/212) and will
//! be feature-gated behind `vision`.
//!
//! ## ADR-0124 — Value::Vision + registry
//!
//! `Value::Vision(VisionId)` is an opaque handle — vision artifacts (image
//! tensors, generation state) never enter `Value`, only an index. This is
//! the same pattern as `Value::Reflex(ReflexId)`.
//!
//! The registry lives in the `Interpreter` behind a `Mutex` (mirroring
//! `reflex_registry`). VM-owned state will be added in a future naryad when
//! real state exists (not empty stubs — per ADR-0121 spirit).

use std::collections::HashMap;
use std::sync::Mutex;

// Наряд №241 (R5): provenance — manifest + LSB watermark (ADR-0125).
// NOT feature-gated: the manifest layer compiles in all builds (the
// non-gated `VisionArtifact` carries it); only the watermark's PNG
// decode/encode paths inside are `image`-gated.
pub mod provenance;

// Наряд №242 (R6.1): SQLite persistence of vision artifacts
// (`vision_save`/`vision_load`). NOT feature-gated: the store operates
// on the non-gated `VisionArtifact` + `rusqlite` (a base dependency) —
// persistence needs no inference stack, so its contract is testable in
// the default build.
pub mod store;

/// Opaque handle to a vision artifact in `VisionRegistry`.
///
/// Contains only an index — the actual artifact data lives in the registry.
/// `Display` gives `[Vision#N]` (same format as `[Reflex#N]`).
///
/// Serializes as a plain u64 — same as `ReflexId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VisionId(pub u64);

impl std::fmt::Display for VisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Vision#{}]", self.0)
    }
}

/// Registry of vision artifacts — the runtime store for opaque handles
/// (Наряд №240, R4.2).
///
/// Stores the encoded PNG bytes of the generated image (the same bytes
/// `vision::vae::save_png` writes, produced via `encode_png`). Weights and
/// raw tensors never enter `Value` or the registry — only the encoded
/// artifact buffer.
///
/// Наряд №241 (R5, ADR-0125): every artifact produced by the real
/// `vision_generate` path is SIGNED — the PNG carries the LSB watermark
/// and the artifact carries its provenance manifest. `manifest: None`
/// exists only for hand-built/deserialized artifacts (tests, third-party
/// construction) — exporting such an artifact through the signed
/// `vision_export` is refused loudly (the runtime backstop of the
/// `VISION_UNSIGNED_EXPORT` gate, Block 2.2).
#[derive(Debug, Clone, PartialEq)]
pub struct VisionArtifact {
    /// Encoded PNG bytes (complete file image, writable as-is).
    /// Signed by `vision_generate` (№241 Block 1): LSB watermark inside.
    pub png_bytes: Vec<u8>,
    /// Provenance manifest (№241 Block 1.2): model-id + weights SHA,
    /// seed, prompt-hash, policy, timestamp, SHA-256 of the final PNG.
    /// `None` only for hand-built/deserialized artifacts — such artifacts
    /// cannot pass the signed `vision_export` (loud runtime backstop,
    /// Block 2.2).
    pub manifest: Option<crate::vision::provenance::VisionManifest>,
}

/// Mirrors `ReflexRegistry` (`src/nn/mod.rs`): owns artifacts behind a
/// `Mutex`, provides insert/get/remove/len/is_empty API.
///
/// Наряд №240 (R4.2): the artifact type is now the real `VisionArtifact`
/// (PNG buffer) — R1's `()` placeholder is gone. Insertion happens only
/// from the real `vision_generate` path (full clip: tokenizer → text
/// encoder → DiT+sampler → VAE → PNG encode).
#[derive(Debug, Default)]
pub struct VisionRegistry {
    /// Map from VisionId → artifact.
    artifacts: HashMap<u64, VisionArtifact>,
    next_id: u64,
}

impl VisionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            artifacts: HashMap::new(),
            next_id: 0,
        }
    }

    /// Insert a new artifact, return its handle. ID is monotonically increasing.
    pub fn insert(&mut self, artifact: VisionArtifact) -> VisionId {
        let id = VisionId(self.next_id);
        self.next_id += 1;
        self.artifacts.insert(id.0, artifact);
        id
    }

    /// Get an artifact by handle. Returns `Some(&VisionArtifact)` if it exists.
    pub fn get(&self, id: VisionId) -> Option<&VisionArtifact> {
        self.artifacts.get(&id.0)
    }

    /// Remove an artifact by handle.
    pub fn remove(&mut self, id: VisionId) {
        self.artifacts.remove(&id.0);
    }

    /// Number of registered artifacts.
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// List all registered vision IDs (sorted for determinism).
    pub fn list_ids(&self) -> Vec<VisionId> {
        let mut ids: Vec<VisionId> = self.artifacts.keys().map(|&k| VisionId(k)).collect();
        ids.sort_by_key(|id| id.0);
        ids
    }
}

/// Convenience wrapper for `Mutex<VisionRegistry>` — the form used by
/// the `Interpreter` struct (mirrors `reflex_registry: Mutex<ReflexRegistry>`).
pub type SharedVisionRegistry = Mutex<VisionRegistry>;

/// SSOT list of vision models known to the language (Наряд №238, R4.1).
///
/// A `vision { }` declaration's `model` field must name a model from this
/// list — enforced by semantic validation (`src/semantic.rs`). NOT
/// feature-gated, mirroring `VisionId`/`VisionRegistry` above: the
/// declaration grammar parses in all builds.
///
/// R4.1: exactly the R3 wedge model. Extend only when a new wedge lands
/// (ADR-0123 discipline — the wedge choice is an ADR decision, not a
/// code-level edit).
pub const KNOWN_VISION_MODELS: &[&str] = &["z-image-turbo"];

// Наряд №211 (R2): текст-энкодер (Qwen3-архитектура на Reflex-примитивах).
// Feature-gated behind `vision` (которая влечёт `candle`).
#[cfg(feature = "vision")]
pub mod text_encoder;

// Наряд №212 (R3): weights infrastructure + tokenizer wrapper.
// Both feature-gated behind `vision`. tokenizers crate is the canonical HF
// BPE implementation — see ADR-0124 update for rationale.
#[cfg(feature = "vision")]
pub mod dit;
#[cfg(feature = "vision")]
pub mod sampler;
#[cfg(feature = "vision")]
pub mod tokenizer;
#[cfg(feature = "vision")]
pub mod vae;
#[cfg(feature = "vision")]
pub mod weights;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vision_id_display() {
        let id = VisionId(1);
        assert_eq!(format!("{}", id), "[Vision#1]");
        let id2 = VisionId(42);
        assert_eq!(format!("{}", id2), "[Vision#42]");
    }

    fn test_artifact() -> VisionArtifact {
        VisionArtifact {
            png_bytes: vec![1, 2, 3],
            manifest: None, // hand-built — signed export refuses (Block 2.2)
        }
    }

    #[test]
    fn registry_insert_returns_monotonic_ids() {
        let mut reg = VisionRegistry::new();
        let id0 = reg.insert(test_artifact());
        let id1 = reg.insert(test_artifact());
        let id2 = reg.insert(test_artifact());
        assert_eq!(id0.0, 0);
        assert_eq!(id1.0, 1);
        assert_eq!(id2.0, 2);
    }

    #[test]
    fn registry_get_after_insert() {
        let mut reg = VisionRegistry::new();
        let id = reg.insert(test_artifact());
        assert!(reg.get(id).is_some());
        assert!(reg.get(VisionId(999)).is_none());
    }

    #[test]
    fn registry_remove() {
        let mut reg = VisionRegistry::new();
        let id = reg.insert(test_artifact());
        assert_eq!(reg.len(), 1);
        reg.remove(id);
        assert_eq!(reg.len(), 0);
        assert!(reg.get(id).is_none());
    }

    #[test]
    fn registry_len_and_is_empty() {
        let mut reg = VisionRegistry::new();
        assert!(reg.is_empty());
        reg.insert(test_artifact());
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        reg.insert(test_artifact());
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn registry_list_ids_sorted() {
        let mut reg = VisionRegistry::new();
        reg.insert(test_artifact());
        reg.insert(test_artifact());
        reg.insert(test_artifact());
        let ids = reg.list_ids();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0].0, 0);
        assert_eq!(ids[1].0, 1);
        assert_eq!(ids[2].0, 2);
    }
}
