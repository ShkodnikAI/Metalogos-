# ADR-0162: Unified media handles and the media store (lazy materialization, refcount, at-rest sealing)

**Status:** Accepted
**Date:** 2026-09-16
**Naryad:** №331 (issue #458, Волна 2 · Фаза 2; absorbs №311 v1)
**Depends on:** ADR-0114 (opaque handle pattern), ADR-0154 (label lattice), №316 (classification SSOT), №325 (sink clearance), №172 (secret()/Zeroizing contour)

## 1. Context

Wave 2 (Фаза 2 «Медиа-хэндлы и реестр бэкендов») fills the Voice/Video pillars with a typed media layer. Today media bytes reach the runtime as raw buffers: the interpreter holds PNG bytes in `Value::Vision`-indexed artifacts, voice/audio handles are pillar-scoped (`crate::voice::AudioId`, u32, `VoiceRegistry`), and there is no unified way for the language to hold an image, an audio clip, a video frame, or a video segment as a VALUE with a label. №311 v1 (MediaStore) is folded into this naryad.

The opaque-handle pattern (ADR-0114) is proven three times in this tree: `Value::Reflex(ReflexId)`, `Value::Vision(VisionId)`, `Value::Voice/VoiceId/AudioId`. The label lattice (ADR-0154) is proven by №322–№325. What №331 adds is the UNIFIED media layer both pillars (and №332–№337) build on.

## 2. Decision

### 2.1 Four opaque handle types, one `Value` variant

`src/media/mod.rs` defines `ImageId`, `AudioId`, `VideoFrameId`, `VideoSegmentId` (opaque `u64` indices, ADR-0114 pattern, `Display` as `[Image#N]` etc.), wrapped by one enum `MediaHandle`. The language sees them through a single new `Value::Media(MediaHandle)` variant whose `type_name()` is `"Image"` / `"Audio"` / `"VideoFrame"` / `"VideoSegment"`. One variant instead of four keeps every exhaustive `Value` match small; the four TYPES stay distinct because the dispatch builtins are separate (`media_store_image` … `media_store_video_segment`), so a static arity/type contract exists per type.

**Naming boundary (loud):** `crate::voice::AudioId` (u32, TTS artifacts in `VoiceRegistry`, skeleton phase) and `crate::media::AudioId` (u64, unified media store) are DIFFERENT types in DIFFERENT registries. The voice pillar is not migrated in №331 — its handle stays pillar-scoped; convergence is a later wave's decision if ever needed.

### 2.2 The store: bytes never live in `Value`

`MediaStore` (per-Interpreter, `Mutex`; per-VM plain field — the established Vision split) holds `MediaEntry { kind, label, refs, payload }`. `Value` carries only the handle. Lazy materialization: nothing materializes until a sanctioned sink asks for bytes.

**Labels on handles without new lattice rules:** exactly as the naryad requires, the STATIC label of a handle flows through the existing №323 inference (a handle bound to `media_store_image(data, sensitivity)` carries the join of the arguments' labels — private data in, private handle out) and hits the №325 sink gate at every materialization. Additionally the STORE entry carries a runtime conf (from the declared `sensitivity` argument) used for the at-rest decision and as the runtime backstop — mirroring №320's static-gate + runtime-backstop split.

### 2.3 At-rest sealing through the secret() contour

`sensitivity` ∈ {`public`, `consented`, `private`} (loud validation; `poisoned` is not constructible — quarantine comes only from the taint machinery). Entries with `conf != Public` are sealed with AES-256-GCM (the №172/Phase 7.3 contour: same primitive, nonce‖ciphertext format); the 256-bit key is generated per store, lives in `Zeroizing`, is never serialized, and `Debug` never renders key or plaintext. Plaintext buffers are `Zeroizing` and dropped on eviction. `public` entries stay plaintext — sealing world-visible data would be ceremony, not security.

### 2.4 Refcount

Handles are `Copy` — silent clones cannot be counted (honest boundary). Refcounting is the explicit API: `media_retain(h) -> h` (+1), `media_release(h) -> Float` (−1; at 0 the entry is EVICTED — sealed bytes zeroized; a later materialization is a loud "unknown handle"). Double-release is a loud error. `media_meta(h) -> Struct{kind, conf, refs, sealed}` observes the contract without materializing.

### 2.5 Bytes are reachable only through sanctioned sinks

The language has no byte-extraction syntax on handles; the one syntactic surface (field access) is a COMPILE error: any `.field` on a media-typed expression fails semantic analysis with the opacity message (ADR-0114). Byte egress exists only via `media_save(handle, path)` — a classified **Sink** (`builtins_classification`, №316 SSOT) wired into the №325 sink-kind table as file-egress: a private-labelled handle fails `private-egress` at compile time; at runtime the backstop refuses materializing a non-public entry (declassification is №326 redact territory — media policies are a later boundary). File writes reuse the io sandbox (`SandboxMode::ForWrite`, №252/№254 loud violations).

## 3. Alternatives considered

- **Reuse `crate::voice::AudioId` for the store** — rejected: mixing the TTS registry with captured media would couple the voice skeleton to the unified layer before №334 defines backends; the pillar boundary stays loud.
- **Four `Value` variants instead of one** — rejected: four exhaustive-match sites × every match, for zero additional safety (the types are distinguished by `MediaHandle`).
- **Global static store (voice `Lazy` style)** — rejected: media belongs to a program run, not to the process; per-Interpreter state (Vision pattern) keeps tests and flows isolated.
- **Runtime label propagation into `Value`** — rejected: labels in this language are static (ADR-0154); duplicating them at runtime would create two truths. The store entry conf is a declared, audited sensitivity, not a second lattice.

## 4. Consequences

- №332 (AST perception, origin chain) can attach provenance to `MediaStore` entries; №335 (consent) can operate on the entry label's consent component; №337 (C2PA) can sign/verify at the entry boundary.
- Materializing private media is IMPOSSIBLE by design until a media declassification policy exists — loud boundary, recorded here.
- The interpreter/VM each hold a store; state does not cross backends (same as Vision).
- New builtins (8, category `media`): `media_store_image`, `media_store_audio`, `media_store_video_frame`, `media_store_video_segment`, `media_save`, `media_retain`, `media_release`, `media_meta`. All classified in the №316 SSOT; `media_save` is a Sink.

## 5. Verification

- `tests/naryad_331_media_handles.rs`: opaque compile-error contract (examples/w1_handle_opaque pair), private-handle sink-gate refusal, materialization round-trip, refcount/eviction contract, at-rest sealing (ciphertext ≠ plaintext, decrypt exact), voice/Vision paths untouched.
- №316 coverage tests: 8/8 new builtins classified; REFERENCE block regenerated from the map.
- `grep todo!/unimplemented!/SKELETON` in new files: 0.
