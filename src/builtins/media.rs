//! Unified media builtins (Наряд №331, ADR-0162).
//!
//! Лекало: the vision family (`src/builtins/vision.rs`) — the interpreter
//! and the VM each hold their own `MediaStore` and route through these
//! shared dispatch functions (state-carrying interception before the
//! generic registry fallback; the `spec!` stubs below are last-resort
//! handlers + the arity/type contract for LSP and checks).
//!
//! Byte-egress discipline (ADR-0162 §2.5): the ONLY byte path out of the
//! store is `media_save` — a classified Sink. Compile-time, the №325
//! sink-clearance gate refuses private-labelled handles (`private-egress`
//! when the data's static label is non-public); at runtime the backstop
//! refuses materializing a non-public entry (the №320 static+runtime
//! split). File writes go through the io sandbox (№252/№254 — loud
//! `SANDBOX_VIOLATION`).

use crate::interpreter::Value;
use crate::labels::{Integrity, Label};
use crate::media::{parse_sensitivity, MediaHandle, MediaKind, MediaStore};

/// Extract a media handle argument (loud type errors).
fn expect_media_handle(fn_name: &str, args: &[Value], idx: usize) -> Result<MediaHandle, String> {
    match args.get(idx) {
        Some(Value::Media(h)) => Ok(*h),
        Some(other) => Err(format!(
            "{}: argument {} must be a media handle (Image/Audio/VideoFrame/VideoSegment), got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing argument {}", fn_name, idx + 1)),
    }
}

/// Extract a String argument (loud type errors).
fn expect_string(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!(
            "{}: argument {} must be a String, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing argument {}", fn_name, idx + 1)),
    }
}

/// Shared body of the four `media_store_*` builtins (ADR-0162 §2.2/§2.3):
/// validate the declared sensitivity loudly, build the entry label
/// (declared conf, trusted integrity, empty consent scope — the static
/// lattice tracks the FULL label through №323 flow; this declared conf
/// drives at-rest sealing + the runtime backstop), insert into the store.
pub fn media_store_dispatch(
    store: &mut MediaStore,
    kind: MediaKind,
    args: &[Value],
) -> Result<Value, String> {
    let fn_name = match kind {
        MediaKind::Image => "media_store_image",
        MediaKind::Audio => "media_store_audio",
        MediaKind::VideoFrame => "media_store_video_frame",
        MediaKind::VideoSegment => "media_store_video_segment",
    };
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (data, sensitivity), got {}",
            fn_name,
            args.len()
        ));
    }
    let data = expect_string(fn_name, args, 0)?;
    let sensitivity = expect_string(fn_name, args, 1)?;
    let conf = parse_sensitivity(&sensitivity).map_err(|e| format!("{}: {}", fn_name, e))?;
    let label = Label {
        conf,
        integrity: Integrity::Trusted,
        consent: Default::default(),
    };
    let handle = store.insert(kind, data.into_bytes(), label)?;
    Ok(Value::Media(handle))
}

/// `media_save(handle, path)` — the sanctioned materialization sink
/// (ADR-0162 §2.5). Runtime backstop: non-public entries are refused
/// (they are sealed at rest; declassification is №326 territory).
/// Writes through the io sandbox (ForWrite, loud violations).
pub fn media_save_dispatch(store: &MediaStore, args: &[Value]) -> Result<Value, String> {
    let fn_name = "media_save";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, path), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle = expect_media_handle(fn_name, args, 0)?;
    let path = expect_string(fn_name, args, 1)?;
    let entry = store.entry(handle)?;
    if entry.label.conf != crate::labels::Conf::Public {
        return Err(format!(
            "MEDIA_SEALED_EGRESS: {} carries declared sensitivity '{}' — private/consented \
             media is sealed at rest and cannot be materialized (№325 sink clearance; \
             media declassification policies are a later boundary, ADR-0162 §2.5)",
            handle,
            entry.label.conf.as_str()
        ));
    }
    let bytes = store.materialize(handle)?;
    // №131 (ForWrite) + №252/№254: sandbox violations are LOUD here —
    // they are programmer errors, not environmental failures.
    let safe_path =
        crate::builtins::io::sandbox_path_ex(&path, crate::builtins::io::SandboxMode::ForWrite)
            .map_err(crate::builtins::io::sandbox_violation)?;
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort, as write_file
    }
    let mut file = crate::builtins::io::open_sandbox_write(&safe_path, false)
        .map_err(crate::builtins::io::sandbox_violation)?;
    std::io::Write::write_all(&mut file, bytes.as_slice())
        .map_err(|e| format!("{}: write to {}: {}", fn_name, path, e))?;
    Ok(Value::String(path))
}

/// `media_retain(handle)` — refcount +1 (ADR-0162 §2.4); returns the
/// same handle (chainable).
pub fn media_retain_dispatch(store: &mut MediaStore, args: &[Value]) -> Result<Value, String> {
    let handle = expect_media_handle("media_retain", args, 0)?;
    store.retain(handle)?;
    Ok(Value::Media(handle))
}

/// `media_release(handle)` — refcount −1; 0 evicts the entry (sealed
/// bytes zeroized). Returns the remaining refcount as a Float.
pub fn media_release_dispatch(store: &mut MediaStore, args: &[Value]) -> Result<Value, String> {
    let handle = expect_media_handle("media_release", args, 0)?;
    let refs = store.release(handle)?;
    Ok(Value::Float(refs as f64))
}

/// `media_meta(handle)` — store metadata WITHOUT materializing bytes:
/// `Struct { kind: String, conf: String, refs: Float, sealed: Bool }`.
pub fn media_meta_dispatch(store: &MediaStore, args: &[Value]) -> Result<Value, String> {
    let handle = expect_media_handle("media_meta", args, 0)?;
    let entry = store.entry(handle)?;
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "kind".to_string(),
        Value::String(entry.kind.slug().to_string()),
    );
    fields.insert(
        "conf".to_string(),
        Value::String(entry.label.conf.as_str().to_string()),
    );
    fields.insert("refs".to_string(), Value::Float(entry.refs as f64));
    fields.insert(
        "sealed".to_string(),
        Value::Bool(matches!(
            entry.payload,
            crate::media::MediaPayload::Sealed(_)
        )),
    );
    Ok(Value::Struct {
        type_name: "MediaMeta".to_string(),
        fields,
    })
}

// ── Registry last-resort handlers (лекало vision stubs) ──────────────
//
// The real paths are state-carrying: interpreter and VM intercept these
// names BEFORE the generic registry fallback and route through the
// dispatch functions above. The stubs exist for the spec! arity/type
// contract (LSP, checks) and must never be reached on live paths — a
// loud error keeps any missed interception honest instead of silent.

/// `media_store_image(data, sensitivity)` — wraps provided bytes into an opaque Image handle (ADR-0162). Sensitivity: public | consented | private; non-public content is AES-256-GCM sealed at rest. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_store_image_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_store_image: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_store_audio(data, sensitivity)` — wraps provided bytes into an opaque Audio handle (ADR-0162). Sensitivity: public | consented | private; non-public content is AES-256-GCM sealed at rest. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_store_audio_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_store_audio: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_store_video_frame(data, sensitivity)` — wraps provided bytes into an opaque VideoFrame handle (ADR-0162). Sensitivity: public | consented | private; non-public content is AES-256-GCM sealed at rest. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_store_video_frame_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_store_video_frame: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_store_video_segment(data, sensitivity)` — wraps provided bytes into an opaque VideoSegment handle (ADR-0162). Sensitivity: public | consented | private; non-public content is AES-256-GCM sealed at rest. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_store_video_segment_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_store_video_segment: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_save(handle, path)` — the ONLY sanctioned media materialization: writes the exact bytes to a sandboxed file (№131/№252). Sink: №325 clearance at compile time (SECRET_LEAK for private labels) + runtime backstop MEDIA_SEALED_EGRESS for sealed entries. Returns the path.
pub(crate) fn builtin_media_save_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_save: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_retain(handle)` — refcount +1 on a media handle (ADR-0162 §2.4); returns the same handle (chainable).
pub(crate) fn builtin_media_retain_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_retain: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_release(handle)` — refcount −1; at 0 the entry is evicted (sealed bytes zeroized). Returns the remaining refcount. Loud on unknown handles.
pub(crate) fn builtin_media_release_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_release: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}

/// `media_meta(handle)` — store metadata WITHOUT materializing bytes: Struct { kind, conf, refs, sealed } (ADR-0162 §2.4).
pub(crate) fn builtin_media_meta_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_meta: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0162); the generic fallback must not be reached".to_string())
}
