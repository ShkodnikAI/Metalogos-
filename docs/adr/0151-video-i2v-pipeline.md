# ADR-0151: Video I2V pipeline — first/last-frame anchors, RIFE-class interpolation, AV sidecar mux

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** №309 (issue #389, P2/feature/video — VIDEO-A3-I2V-PIPELINE)
**Precedent:** ADR-0147 (Video scope), ADR-0148 (Video value-registry), ADR-0149 (Video security gates — D5 UNTRUSTED_FRAME), ADR-0145 D6 (UNTRUSTED_AUDIO taint лекало), №310 (real tiny VAE/DiT/sampler — the no-stubs template), №240 (VISION_PROMPT_USER_INPUT advisory taint implementation)

## Context

The Video pillar has real tiny-tensor machinery since №310 (`VideoVae`, `VideoDit`, flow-matching Euler sampler, seed-deterministic E2E). №309 turns the silent contour into a practical pipeline: I2V first-frame anchoring, first–last two-anchor contract, RIFE-class interpolation, clip extension, cross-pillar A/V mux with `AudioId`, and provenance-carrying export. The environment constraint is unchanged (4 GB RAM, no GPU): everything is real and runs on tiny seeded tensors on CPU — the №310 template. Production-weights inference remains a documented No-Go (№294 class); `video_fetch_weights` stays a loud error behind the shared `MODEL_WEIGHTS_UNSAFE` static gate (not a hidden stub — a formally recorded boundary).

## Decision

### D1. I2V = T2V with pinned latent anchors (real algorithm)

`render_i2v` runs the existing flow-matching Euler loop and, after every step (and at init), overwrites the anchor latent frames:

- first anchor: `x[:, :, 0] = encode_frame(ref_first)`
- two-anchor (first–last contract): additionally `x[:, :, T-1] = encode_frame(ref_last)`

The anchor latent is produced by the existing `VideoVae::encode_frame` from the reference pixels ([3, 32, 32] f32, 3072 values). Overwriting after each step is the standard I2V anchor-pinning technique (renoise-based editing class); it is a real algorithm, deterministic by seed, not a blend hack.

Arity encodes the mode (no kwargs in the language):

- `video_render(decl, prompt)` — T2V (arity 2)
- `video_render(decl, prompt, ref_first)` — I2V first-anchor (arity 3)
- `video_render(decl, prompt, ref_first, ref_last)` — two-anchor first–last (arity 4)

`decl` is validated against `KNOWN_VIDEO_MODELS` (SSOT, ADR-0150). The seed is derived deterministically: `seed = sha256(model_id | prompt)` (first 8 bytes LE) — same model+prompt+refs ⇒ same video, different prompt ⇒ different latent init. `ref_hash` / `ref_last_hash` (SHA-256 of the reference f32 bytes) are recorded in `VideoManifest` — the manifest is the provenance SSOT (ADR-0148).

### D2. RIFE-class interpolation = linear latent blending (real MVP)

`frame_interp(handle, factor)` with `factor ∈ {2, 4}`: between each consecutive latent frame pair, `(factor − 1)` in-between latents are inserted as linear blends `lerp(l_t, l_{t+1}, k/factor)`. Output latent temporal size `T' = (T−1)·factor + 1`; the endpoints are preserved exactly (out[0] = in[0], out[T'] = in[T-1]) — the RIFE-class contract (first and last frame of the clip never move). The new latent is decoded through the same-seed `VideoVae` (reconstructed from `manifest.seed`) into a new artifact. This is frame-rate interpolation at the latent level per the issue; the flow-guided (optical-flow warp) RIFE upgrade is the V7 research item — documented boundary, not a hidden gap.

### D3. Extension = anchored continuation sampling

`video_extend(handle, extra)` (1..=8 extra latent frames): builds a continuation latent `[1, C, T+extra, H, W]` where the source latent is concatenated with newly sampled frames. The new frames are produced by the flow-matching sampler with the first anchor pinned to the source's last latent frame and seed `source_seed + 1`. Provenance: `kind = Extend`, `source_sha = manifest.video_sha` of the source artifact.

### D4. av_mux = deterministic `.mlgv.av` sidecar container (real artifact, no muxing crate)

No MP4 muxing crate fits the constraints (heavy, platform-dependent, or license-unclear for the identity of the project), so per the naryad's option (b): `av_mux(VideoId, AudioId)` emits a deterministic sidecar container:

```
magic "MLGVAV\x01" | u32_le json_len | JSON header | video_bytes | audio_bytes
```

JSON header: `{version, video_id, audio_id, fps, frames, video_sha256, audio_sha256, video_duration_s, audio_duration_s, drift_s, timestamps}` — frame-aligned timestamps `i/fps` computed from the render fps (8 for the tiny pipeline), audio duration parsed from the real PCM WAV header (RIFF/WAVE walk; MP3 is a documented non-goal of this phase). `drift_s = |video_duration − audio_duration|` is recorded, not hidden. The mux result is a new `VideoArtifact` with `kind = AvMux` and `audio_ref = Some(AudioId.0)` (ADR-0148 field), so provenance of the composed artifact is by construction. Re-muxing an `AvMux` artifact is a loud error (no nested containers).

### D5. video_export = signed-by-construction `.mlgv` container; unsigned export does not exist

`video_export(handle, path)` writes:

```
magic "MLGV\x01" | u32_le json_len | manifest JSON | watermark (16 bytes) | payload
```

- **Manifest by construction**: every artifact produced by `video_render`/`frame_interp`/`video_extend`/`av_mux` carries a full `VideoManifest`; `video_export` refuses (`Err`, loud) any artifact whose manifest is absent — the runtime gate `VIDEO_UNSIGNED_EXPORT`. ADR-0149 D3's static Category-A form lands in V6 with `video { }` declarations (the issue explicitly scopes the static gate out of №309: "в полной механике V6; здесь — контракт + контрактный тест"); the contract test in this naryad pins the runtime gate.
- **Watermark**: 16 bytes = first 16 bytes of `sha256(model_id | seed | video_sha256)` — deterministic, derived from the manifest, embedded between header and payload. EU AI Act Art. 50 provenance posture: the export never leaves the process without its manifest and watermark.

### D6. UNTRUSTED_FRAME taint (audit.rs) — лекало UNTRUSTED_AUDIO / №240

Implemented in `check_secret_leak`'s expression walk (the exact home of the vision prompt taint checks, №240/№243/№244):

- `video_render` with `args.len() ≥ 3`: ref args (positions 2 and 3) are inspected. A ref is untrusted when it carries `UserInput` taint (form_data/json_body/query_param/mcp_call) OR is a direct untrusted-source call (`http_get`, `read_file`) — this covers the http/form/file classes of ADR-0149 D5 without changing global taint-source semantics for other pillars.
- Finding: `Severity::Warning`, `check_id = "UNTRUSTED_FRAME"` — advisory in `mlog audit` (`audit_program`), and a loud compile error on the `mlog check`/`run` path (semantic.rs №98 promotion of `audit_category_a` findings), exactly the behavior of the `VISION_PROMPT_USER_INPUT` лекало. The full screen+consent ritual (`frame_screen`, `LikenessToken`) arrives with V6 per ADR-0149 D1/D6; until then the detector is the loud path (per issue A.4), and its honest boundary is the same as all MVP taint detectors: let-bound variables holding previously-fetched untrusted frames are not tracked (no new global taint sources were introduced).

### D7. Honest boundaries (recorded, not hidden)

- Tiny seeded pipeline: real Conv2d/attention/Euler computation on CPU (№310 template); production Wan 2.2 / CogVideoX weights remain No-Go (№294) — `video_fetch_weights` stays a loud error covered by `MODEL_WEIGHTS_UNSAFE`.
- Prompt conditioning operates through seed derivation (latent init) and the manifest `prompt_hash`; the DiT text path is the №310 API surface (its use is a V4+ item).
- `video_bytes` serialization: raw little-endian f32 frames (`MLGV-RAW-F32` payload class) — deterministic and hashable; a compressed container codec is a V7 research item.
- Interpolation is latent-space linear blending (D2), not flow-warped synthesis.

## Consequences

- Six builtins are real (`video_render`, `frame_interp`, `video_extend`, `av_mux`, `video_export`) plus the loud-error `video_fetch_weights` (formal boundary) — no `unimplemented!()`, no contract-only files.
- `VideoManifest` gains additive fields (`fps`, `ref_last_hash`, `source_sha`); `VideoArtifact` gains `latent: Option<LatentData>` (needed by interp/extend; opaque — never enters `Value`).
- Process-global registries: `VIDEO_REGISTRY` (video/mod.rs) and `VOICE_REGISTRY` (voice/mod.rs) — `once_cell::Lazy<Mutex<…>>`, the BPE/LLM-stream pattern; library functions take explicit `&mut` registries so tests are byte-deterministic.
- REFERENCE.md rows + counts updated (100% coverage gate), README counts synced, cross-pillar summary (three pillars, composition across modalities) added to README.
- Tests: unit + E2E «озвученная сцена» (frame → i2v → interp → mux with WAV audio → export) are seed-deterministic and byte-comparable; UNTRUSTED_FRAME taint tests (form_data, inline http_get/read_file, clean paths) green in default features (static check is feature-independent).
