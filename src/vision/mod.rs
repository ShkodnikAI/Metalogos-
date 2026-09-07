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

/// Registry of vision artifacts — the runtime store for opaque handles.
///
/// Mirrors `ReflexRegistry` (`src/nn/mod.rs`): owns artifacts behind a
/// `Mutex`, provides insert/get/remove/len/is_empty API. The artifact
/// type is currently `()` (empty) — R2/R3 will replace it with actual
/// image tensors or generation state.
///
/// In the interpreter, the registry will be stored as
/// `Mutex<VisionRegistry>` on the `Interpreter` struct (same pattern as
/// `reflex_registry` at `src/interpreter/mod.rs:218`). This wiring lands
/// in R3 (naryad 212) when real vision state exists — not in R1.
#[derive(Debug, Default)]
pub struct VisionRegistry {
    /// Map from VisionId → artifact. Currently `()` — R2/R3 will add real types.
    artifacts: HashMap<u64, ()>,
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
    pub fn insert(&mut self) -> VisionId {
        let id = VisionId(self.next_id);
        self.next_id += 1;
        self.artifacts.insert(id.0, ());
        id
    }

    /// Get an artifact by handle. Returns `Some(())` if it exists.
    pub fn get(&self, id: VisionId) -> Option<&()> {
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

// Наряд №211 (R2): текст-энкодер (Qwen3-архитектура на Reflex-примитивах).
// Feature-gated behind `vision` (которая влечёт `candle`).
#[cfg(feature = "vision")]
pub mod text_encoder;

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

    #[test]
    fn registry_insert_returns_monotonic_ids() {
        let mut reg = VisionRegistry::new();
        let id0 = reg.insert();
        let id1 = reg.insert();
        let id2 = reg.insert();
        assert_eq!(id0.0, 0);
        assert_eq!(id1.0, 1);
        assert_eq!(id2.0, 2);
    }

    #[test]
    fn registry_get_after_insert() {
        let mut reg = VisionRegistry::new();
        let id = reg.insert();
        assert!(reg.get(id).is_some());
        assert!(reg.get(VisionId(999)).is_none());
    }

    #[test]
    fn registry_remove() {
        let mut reg = VisionRegistry::new();
        let id = reg.insert();
        assert_eq!(reg.len(), 1);
        reg.remove(id);
        assert_eq!(reg.len(), 0);
        assert!(reg.get(id).is_none());
    }

    #[test]
    fn registry_len_and_is_empty() {
        let mut reg = VisionRegistry::new();
        assert!(reg.is_empty());
        reg.insert();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        reg.insert();
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn registry_list_ids_sorted() {
        let mut reg = VisionRegistry::new();
        reg.insert();
        reg.insert();
        reg.insert();
        let ids = reg.list_ids();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0].0, 0);
        assert_eq!(ids[1].0, 1);
        assert_eq!(ids[2].0, 2);
    }
}
