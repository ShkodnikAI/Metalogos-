//! Unified media layer (Наряд №331, ADR-0162) — opaque media handles and
//! the media store: lazy materialization, refcount, at-rest sealing.
//!
//! ## ADR-0162 — handles are values, bytes are not
//!
//! `ImageId` / `AudioId` / `VideoFrameId` / `VideoSegmentId` are opaque
//! handles (ADR-0114 pattern, mirrors `VisionId`): the runtime `Value`
//! carries only the index, the bytes live in `MediaStore` behind the
//! interpreter's `Mutex` (and the VM's own store — the established
//! Vision/Interpreter split). Byte egress is reachable ONLY through
//! sanctioned sinks (`media_save` — classified Sink, gated by the №325
//! sink-clearance machinery at compile time plus the runtime backstop).
//!
//! ## Labels on handles without new lattice rules
//!
//! The static label of a handle flows through the existing №323
//! inference (the handle carries the join of the producing call's
//! arguments) and hits the №325 gate at materialization. The store
//! entry additionally carries the DECLARED runtime sensitivity (conf
//! axis of ADR-0154) used for the at-rest decision and as the runtime
//! backstop — mirroring №320's static-gate + runtime-backstop split.
//!
//! ## At-rest sealing (the secret() contour, precedent №172)
//!
//! Entries declared with a non-public sensitivity are sealed with
//! AES-256-GCM (same primitive and nonce‖ciphertext format as the
//! Phase 7.3 encrypt() contour). The per-store key lives in
//! `Zeroizing`, is never serialized, and `Debug` never renders key,
//! ciphertext, or plaintext — the same discipline as `SecretString`
//! (`src/interpreter/values.rs`).

use std::collections::HashMap;
use zeroize::Zeroizing;

// ── Opaque handles (ADR-0114 pattern) ────────────────────────────────

/// Opaque handle to an image in `MediaStore`. Bytes never enter `Value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ImageId(pub u64);

impl std::fmt::Display for ImageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Image#{}]", self.0)
    }
}

/// Opaque handle to an audio clip in `MediaStore`.
///
/// NOTE the loud boundary (ADR-0162 §2.1): this is the UNIFIED media
/// audio handle (u64, media store) — distinct from
/// `crate::voice::AudioId` (u32, VoiceRegistry, TTS artifacts of the
/// voice skeleton). Different registries, different ID spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AudioId(pub u64);

impl std::fmt::Display for AudioId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Audio#{}]", self.0)
    }
}

/// Opaque handle to a video frame in `MediaStore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VideoFrameId(pub u64);

impl std::fmt::Display for VideoFrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[VideoFrame#{}]", self.0)
    }
}

/// Opaque handle to a video segment in `MediaStore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VideoSegmentId(pub u64);

impl std::fmt::Display for VideoSegmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[VideoSegment#{}]", self.0)
    }
}

/// Media kind — which of the four handle types a store entry backs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    Image,
    Audio,
    VideoFrame,
    VideoSegment,
}

impl MediaKind {
    /// Language-level type name (`Value::type_name` for `Value::Media`).
    pub fn type_name(self) -> &'static str {
        match self {
            MediaKind::Image => "Image",
            MediaKind::Audio => "Audio",
            MediaKind::VideoFrame => "VideoFrame",
            MediaKind::VideoSegment => "VideoSegment",
        }
    }

    /// Lowercase store/report name (`media_meta` kind field).
    pub fn slug(self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Audio => "audio",
            MediaKind::VideoFrame => "video_frame",
            MediaKind::VideoSegment => "video_segment",
        }
    }

    /// Parse the origin-declaration `media` field (№332: loud validation —
    /// unknown words are errors, never silent defaults).
    pub fn from_slug(slug: &str) -> Result<Self, String> {
        match slug {
            "image" => Ok(MediaKind::Image),
            "audio" => Ok(MediaKind::Audio),
            "video_frame" => Ok(MediaKind::VideoFrame),
            "video_segment" => Ok(MediaKind::VideoSegment),
            other => Err(format!(
                "origin: unknown media kind '{}' (expected image | audio | video_frame | video_segment)",
                other
            )),
        }
    }
}

/// One handle value covering the four media types (one `Value::Media`
/// variant keeps every exhaustive `Value` match small; the TYPES stay
/// distinct through the per-type dispatch builtins).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MediaHandle {
    Image(ImageId),
    Audio(AudioId),
    VideoFrame(VideoFrameId),
    VideoSegment(VideoSegmentId),
}

impl MediaHandle {
    /// Store entry index (the id is shared across kinds; the kind tag
    /// prevents cross-kind confusion at the dispatch layer).
    pub fn id(&self) -> u64 {
        match self {
            MediaHandle::Image(i) => i.0,
            MediaHandle::Audio(a) => a.0,
            MediaHandle::VideoFrame(f) => f.0,
            MediaHandle::VideoSegment(s) => s.0,
        }
    }

    pub fn kind(&self) -> MediaKind {
        match self {
            MediaHandle::Image(_) => MediaKind::Image,
            MediaHandle::Audio(_) => MediaKind::Audio,
            MediaHandle::VideoFrame(_) => MediaKind::VideoFrame,
            MediaHandle::VideoSegment(_) => MediaKind::VideoSegment,
        }
    }
}

impl std::fmt::Display for MediaHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaHandle::Image(i) => write!(f, "{}", i),
            MediaHandle::Audio(a) => write!(f, "{}", a),
            MediaHandle::VideoFrame(fr) => write!(f, "{}", fr),
            MediaHandle::VideoSegment(s) => write!(f, "{}", s),
        }
    }
}

// ── Store entries ────────────────────────────────────────────────────

/// At-rest payload of a media entry (ADR-0162 §2.3): plaintext for
/// public content, AES-256-GCM sealed for private+ content (the secret()
/// contour — precedent №172/Phase 7.3). Format: `nonce(12) || ciphertext+tag`.
#[derive(Clone)]
pub enum MediaPayload {
    /// World-visible content — stored as-is (sealing it would be
    /// ceremony, not security).
    Plain(Vec<u8>),
    /// Non-public content — sealed at rest. Key material never lives
    /// here (the store holds it); plaintext never outlives a
    /// materialization call.
    Sealed(Zeroizing<Vec<u8>>),
}

impl std::fmt::Debug for MediaPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaPayload::Plain(b) => write!(f, "Plain({} bytes)", b.len()),
            MediaPayload::Sealed(_) => write!(f, "Sealed([REDACTED])"),
        }
    }
}

/// One store entry: bytes + declared label + refcount.
#[derive(Debug, Clone)]
pub struct MediaEntry {
    pub kind: MediaKind,
    /// Declared runtime sensitivity (ADR-0154 conf axis; the static
    /// lattice tracks the full label through №323 flow — no new rules).
    pub label: crate::labels::Label,
    /// Refcount (ADR-0162 §2.4): starts at 1 on insert; `media_retain`
    /// +1, `media_release` −1; eviction at 0 (sealed bytes zeroized).
    pub refs: u64,
    pub payload: MediaPayload,
    /// №332 (ADR-0164): the origin this entry came from (declared
    /// `origin` name). Set by `media_source_capture` / `media_bind_origin`;
    /// `None` never survives a sanctioned construction (the origin-chain
    /// rule refuses unbound construction at compile time) — the Option is
    /// the honest state for direct store API use (Rust tests, №337 flows).
    pub origin: Option<String>,
}

/// AES-256-GCM seal/unseal — reuses the №172 contour primitives exactly
/// (`src/builtins/crypto.rs`): 32-byte key, 96-bit random nonce,
/// `nonce || ciphertext+tag` self-contained format.
mod sealing {
    use zeroize::Zeroizing;

    /// Seal `plaintext` under `key`; returns `nonce || ciphertext+tag`.
    pub fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Key, Nonce};

        let key = Key::<Aes256Gcm>::try_from(key.as_slice())
            .map_err(|_| "media store: key conversion failed".to_string())?;
        let cipher = Aes256Gcm::new(&key);
        let mut nonce_bytes = [0u8; 12];
        use rand::Rng;
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::try_from(nonce_bytes.as_slice())
            .map_err(|_| "media store: nonce conversion failed".to_string())?;
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| format!("media store: AES-256-GCM seal failed: {}", e))?;
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(Zeroizing::new(out))
    }

    /// Unseal a `nonce || ciphertext+tag` payload; plaintext is
    /// `Zeroizing` (dropped → zeroized, the №172 contour discipline).
    pub fn unseal(key: &[u8; 32], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Key, Nonce};

        if sealed.len() < 13 {
            return Err("media store: sealed payload too short".to_string());
        }
        let (nonce_bytes, ciphertext) = sealed.split_at(12);
        let nonce = Nonce::try_from(nonce_bytes)
            .map_err(|_| "media store: nonce conversion failed".to_string())?;
        let key = Key::<Aes256Gcm>::try_from(key.as_slice())
            .map_err(|_| "media store: key conversion failed".to_string())?;
        let cipher = Aes256Gcm::new(&key);
        let plaintext = cipher.decrypt(&nonce, ciphertext).map_err(|_| {
            "media store: unseal failed (key mismatch or corrupted data)".to_string()
        })?;
        Ok(Zeroizing::new(plaintext))
    }
}

/// Parse the `sensitivity` argument of the `media_store_*` builtins.
/// Loud validation: unknown words are errors, never silent defaults.
/// `poisoned` is deliberately NOT constructible here — quarantine comes
/// only from the taint machinery (ADR-0154 §2.1).
pub fn parse_sensitivity(word: &str) -> Result<crate::labels::Conf, String> {
    match word {
        "public" => Ok(crate::labels::Conf::Public),
        "consented" => Ok(crate::labels::Conf::Consented),
        "private" => Ok(crate::labels::Conf::Private),
        other => Err(format!(
            "media_store: unknown sensitivity '{}' (expected public | consented | private)",
            other
        )),
    }
}

/// The unified media store (ADR-0162 §2.2). Per-Interpreter (behind the
/// interpreter's `Mutex`) and per-VM (plain field) — media belongs to a
/// program run, not to the process.
pub struct MediaStore {
    entries: HashMap<u64, MediaEntry>,
    next_id: u64,
    /// Per-store sealing key (Zeroizing — zeroed on drop; never
    /// serialized; `Debug` redacts it).
    key: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for MediaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaStore")
            .field("entries", &self.entries.len())
            .field("next_id", &self.next_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

impl Default for MediaStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaStore {
    /// New empty store with a fresh random 256-bit sealing key.
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        use rand::Rng;
        rand::rng().fill_bytes(&mut key);
        MediaStore {
            entries: HashMap::new(),
            next_id: 0,
            key: Zeroizing::new(key),
        }
    }

    /// Insert bytes, returning the opaque handle. The declared label's
    /// conf decides plaintext vs sealed at rest. `refs` starts at 1.
    pub fn insert(
        &mut self,
        kind: MediaKind,
        bytes: Vec<u8>,
        label: crate::labels::Label,
    ) -> Result<MediaHandle, String> {
        let payload = if label.conf == crate::labels::Conf::Public {
            MediaPayload::Plain(bytes)
        } else {
            MediaPayload::Sealed(sealing::seal(&self.key, &bytes)?)
        };
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(
            id,
            MediaEntry {
                kind,
                label,
                refs: 1,
                payload,
                origin: None,
            },
        );
        Ok(match kind {
            MediaKind::Image => MediaHandle::Image(ImageId(id)),
            MediaKind::Audio => MediaHandle::Audio(AudioId(id)),
            MediaKind::VideoFrame => MediaHandle::VideoFrame(VideoFrameId(id)),
            MediaKind::VideoSegment => MediaHandle::VideoSegment(VideoSegmentId(id)),
        })
    }

    /// Entry metadata (no bytes leave the store).
    pub fn entry(&self, handle: MediaHandle) -> Result<&MediaEntry, String> {
        self.entries.get(&handle.id()).ok_or_else(|| {
            format!(
                "media_store: unknown handle {} (evicted or never stored)",
                handle
            )
        })
    }

    /// Materialize the bytes behind a handle (the ONLY byte path; every
    /// language-level caller is a sanctioned sink gated by №325 + the
    /// runtime backstop). Sealed payloads are unsealed on the fly;
    /// plaintext is `Zeroizing` and dropped at the end of the call.
    pub fn materialize(&self, handle: MediaHandle) -> Result<Zeroizing<Vec<u8>>, String> {
        let entry = self.entry(handle)?;
        match &entry.payload {
            MediaPayload::Plain(b) => Ok(Zeroizing::new(b.clone())),
            MediaPayload::Sealed(sealed) => sealing::unseal(&self.key, sealed.as_slice()),
        }
    }

    /// №332 (ADR-0164): bind the origin of an entry (and join the
    /// declared origin conf into the entry label — re-sealing when a
    /// public entry becomes non-public, so the at-rest contract tracks
    /// the STRONGEST declared label). Loud on unknown handles.
    pub fn bind_origin(
        &mut self,
        handle: MediaHandle,
        origin_name: String,
        conf: crate::labels::Conf,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .get_mut(&handle.id())
            .ok_or_else(|| format!("media_bind_origin: unknown handle {}", handle))?;
        // Re-seal when the joined conf demands it (public → consented/
        // private): materialize the plaintext, seal it, drop the plaintext.
        let needs_reseal =
            entry.label.conf == crate::labels::Conf::Public && conf != crate::labels::Conf::Public;
        entry.label.conf = entry.label.conf.join(conf);
        entry.origin = Some(origin_name);
        if needs_reseal {
            let plaintext = match &entry.payload {
                MediaPayload::Plain(b) => b.clone(),
                MediaPayload::Sealed(_) => Vec::new(), // already sealed
            };
            if !plaintext.is_empty() {
                entry.payload = MediaPayload::Sealed(sealing::seal(&self.key, &plaintext)?);
            }
        }
        Ok(())
    }

    /// Refcount +1 (ADR-0162 §2.4). Loud on unknown handles.
    pub fn retain(&mut self, handle: MediaHandle) -> Result<u64, String> {
        let entry = self
            .entries
            .get_mut(&handle.id())
            .ok_or_else(|| format!("media_retain: unknown handle {}", handle))?;
        entry.refs += 1;
        Ok(entry.refs)
    }

    /// Refcount −1; at 0 the entry is EVICTED (sealed bytes zeroized
    /// with the buffer). Loud on unknown handles (double-release is a
    /// programmer error, not a soft failure).
    pub fn release(&mut self, handle: MediaHandle) -> Result<u64, String> {
        let refs = {
            let entry = self
                .entries
                .get_mut(&handle.id())
                .ok_or_else(|| format!("media_release: unknown handle {}", handle))?;
            entry.refs -= 1;
            entry.refs
        };
        if refs == 0 {
            // Removing the entry drops the payload; for sealed entries the
            // Zeroizing buffer zeroizes its bytes on drop (the №172
            // contour discipline — key/sealed material never outlives its
            // purpose).
            self.entries.remove(&handle.id());
        }
        Ok(refs)
    }

    /// Number of live entries (tests/observability).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labels::{Conf, Integrity, Label};

    fn label(conf: Conf) -> Label {
        Label {
            conf,
            integrity: Integrity::Trusted,
            consent: Default::default(),
        }
    }

    #[test]
    fn insert_public_is_plain_and_materializes_exact_bytes() {
        let mut store = MediaStore::new();
        let h = store
            .insert(
                MediaKind::Image,
                b"png-bytes-here".to_vec(),
                label(Conf::Public),
            )
            .unwrap();
        assert!(matches!(
            store.entry(h).unwrap().payload,
            MediaPayload::Plain(_)
        ));
        assert_eq!(store.materialize(h).unwrap().as_slice(), b"png-bytes-here");
        assert_eq!(h.to_string(), "[Image#0]");
    }

    #[test]
    fn private_is_sealed_at_rest_and_unseals_exactly() {
        let mut store = MediaStore::new();
        let plaintext = b"secret-frame-payload".to_vec();
        let h = store
            .insert(
                MediaKind::VideoFrame,
                plaintext.clone(),
                label(Conf::Private),
            )
            .unwrap();
        // At rest: ciphertext differs from plaintext (real AES-GCM), and
        // the Debug form never renders the bytes.
        match &store.entry(h).unwrap().payload {
            MediaPayload::Sealed(sealed) => {
                assert_ne!(sealed.as_slice(), plaintext.as_slice());
                let dbg = format!("{:?}", store.entry(h).unwrap());
                assert!(!dbg.contains("secret-frame-payload"));
            }
            other => panic!("private entry must be sealed, got {:?}", other),
        }
        // Materialization unseals exactly.
        assert_eq!(
            store.materialize(h).unwrap().as_slice(),
            plaintext.as_slice()
        );
        assert_eq!(h.to_string(), "[VideoFrame#0]");
    }

    #[test]
    fn consented_is_sealed_too() {
        let mut store = MediaStore::new();
        let h = store
            .insert(
                MediaKind::Audio,
                b"gdpr-audio".to_vec(),
                label(Conf::Consented),
            )
            .unwrap();
        assert!(matches!(
            store.entry(h).unwrap().payload,
            MediaPayload::Sealed(_)
        ));
        assert_eq!(h.to_string(), "[Audio#0]");
    }

    #[test]
    fn refcount_retain_release_and_eviction() {
        let mut store = MediaStore::new();
        let h = store
            .insert(MediaKind::Image, b"x".to_vec(), label(Conf::Public))
            .unwrap();
        assert_eq!(store.entry(h).unwrap().refs, 1);
        assert_eq!(store.retain(h).unwrap(), 2);
        assert_eq!(store.release(h).unwrap(), 1);
        assert_eq!(store.release(h).unwrap(), 0);
        // Evicted: metadata and materialization are loud.
        assert!(store.entry(h).is_err());
        assert!(store.materialize(h).is_err());
        // Double release is a loud programmer error.
        assert!(store.release(h).is_err());
        assert!(store.is_empty());
    }

    #[test]
    fn kinds_are_distinct_handle_types() {
        let mut store = MediaStore::new();
        let img = store
            .insert(MediaKind::Image, b"i".to_vec(), label(Conf::Public))
            .unwrap();
        let seg = store
            .insert(MediaKind::VideoSegment, b"s".to_vec(), label(Conf::Public))
            .unwrap();
        assert_eq!(img.kind(), MediaKind::Image);
        assert_eq!(seg.kind(), MediaKind::VideoSegment);
        assert_eq!(seg.to_string(), "[VideoSegment#1]");
        assert_ne!(img.id(), seg.id());
    }

    #[test]
    fn sensitivity_validation_is_loud() {
        assert_eq!(parse_sensitivity("public").unwrap(), Conf::Public);
        assert_eq!(parse_sensitivity("consented").unwrap(), Conf::Consented);
        assert_eq!(parse_sensitivity("private").unwrap(), Conf::Private);
        assert!(parse_sensitivity("poisoned").is_err());
        assert!(parse_sensitivity("topsecret").is_err());
    }
}
