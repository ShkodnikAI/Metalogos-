//! №483 (gh#731) — the media/vision group of the TW/VM duplicate-name
//! transfer (the №466 лекало, the FINAL group): the shared LIVE home of
//! the twenty media/vision builtin NAMES (gate gh#680, decision 4-A;
//! the threshold 20 → 0 — the transfer's fact).
//!
//! The №480 boundary holds: this is a NAME transfer, not a logic
//! unification — the per-backend dispatch bodies stay where they are
//! (the TW keeps its interpreter-state dispatch in
//! `interpreter/execution.rs`, the VM its store dispatch in `vm.rs`;
//! both call the shared `builtins/*_dispatch` functions). What moved
//! here is the single spelling of the twenty names outside
//! `BUILTIN_REGISTRY`: the two backends now compare against these
//! constants, so the name literals live nowhere else in the backends'
//! code (the №462 counter reads the literals — the group drops out of
//! the TW ∩ VM duplicate set entirely).
//!
//! The module is intentionally feature-independent (pure name
//! constants): the media/vision dispatch code may sit behind feature
//! splits (№472), the NAMES must resolve on every build the backends
//! compile for. Transferring into dead code is forbidden (№483) —
//! every constant below has live readers on BOTH backends.

/// `media_bind_origin` — the origin binding surface.
pub const MEDIA_BIND_ORIGIN: &str = "media_bind_origin";
/// `media_manifest` — the C2PA-style manifest surface.
pub const MEDIA_MANIFEST: &str = "media_manifest";
/// `media_meta` — the media metadata surface.
pub const MEDIA_META: &str = "media_meta";
/// `media_release` — the media release surface.
pub const MEDIA_RELEASE: &str = "media_release";
/// `media_retain` — the media retention surface.
pub const MEDIA_RETAIN: &str = "media_retain";
/// `media_save` — the media save surface (the №325-sealed egress gate).
pub const MEDIA_SAVE: &str = "media_save";
/// `media_source_capture` — the capture surface.
pub const MEDIA_SOURCE_CAPTURE: &str = "media_source_capture";
/// `media_store_audio` — the audio store surface.
pub const MEDIA_STORE_AUDIO: &str = "media_store_audio";
/// `media_store_image` — the image store surface.
pub const MEDIA_STORE_IMAGE: &str = "media_store_image";
/// `media_store_video_frame` — the video-frame store surface.
pub const MEDIA_STORE_VIDEO_FRAME: &str = "media_store_video_frame";
/// `media_store_video_segment` — the video-segment store surface.
pub const MEDIA_STORE_VIDEO_SEGMENT: &str = "media_store_video_segment";
/// `vision_edit` — the vision edit surface.
pub const VISION_EDIT: &str = "vision_edit";
/// `vision_export` — the vision export surface.
pub const VISION_EXPORT: &str = "vision_export";
/// `vision_export_raw` — the raw (unredacted) vision export surface.
pub const VISION_EXPORT_RAW: &str = "vision_export_raw";
/// `vision_generate` — the vision generation surface.
pub const VISION_GENERATE: &str = "vision_generate";
/// `vision_list` — the vision listing surface.
pub const VISION_LIST: &str = "vision_list";
/// `vision_load` — the vision load surface.
pub const VISION_LOAD: &str = "vision_load";
/// `vision_lora_generate` — the LoRA vision generation surface.
pub const VISION_LORA_GENERATE: &str = "vision_lora_generate";
/// `vision_lora_load` — the LoRA vision load surface.
pub const VISION_LORA_LOAD: &str = "vision_lora_load";
/// `vision_save` — the vision save surface.
pub const VISION_SAVE: &str = "vision_save";

/// The media/vision names this module owns — the live membership hook
/// (the audit/semantic tooling and both backends' dispatch prologues ask
/// this instead of re-spelling the twenty literals).
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        MEDIA_BIND_ORIGIN
            | MEDIA_MANIFEST
            | MEDIA_META
            | MEDIA_RELEASE
            | MEDIA_RETAIN
            | MEDIA_SAVE
            | MEDIA_SOURCE_CAPTURE
            | MEDIA_STORE_AUDIO
            | MEDIA_STORE_IMAGE
            | MEDIA_STORE_VIDEO_FRAME
            | MEDIA_STORE_VIDEO_SEGMENT
            | VISION_EDIT
            | VISION_EXPORT
            | VISION_EXPORT_RAW
            | VISION_GENERATE
            | VISION_LIST
            | VISION_LOAD
            | VISION_LORA_GENERATE
            | VISION_LORA_LOAD
            | VISION_SAVE
    )
}
