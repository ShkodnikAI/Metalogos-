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
    // №337 (ADR-0166 §2.2): the sidecar manifest is part of the SINK —
    // the №241 continuity (the same-named `<path>.manifest.json` the
    // vision export writes). Built from the ENTRY's manifest facts, so
    // the origin chain (№332) and the C2PA record cannot disagree. A
    // failed sidecar write is a LOUD error — a manifest-less media
    // egress cannot happen through media_save.
    let sidecar_path = safe_path.with_extension(format!(
        "{}.manifest.json",
        safe_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default()
    ));
    let manifest = crate::media::MediaManifest {
        kind: entry.kind.slug().to_string(),
        origin: entry.origin.clone().unwrap_or_default(),
        conf: entry.label.conf.as_str().to_string(),
        bytes_sha256: entry.bytes_sha256.clone(),
        synthetic: entry.synthetic,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let json = crate::media::manifest_sidecar_json(&manifest)?;
    let mut sidecar = crate::builtins::io::open_sandbox_write(&sidecar_path, false)
        .map_err(crate::builtins::io::sandbox_violation)?;
    std::io::Write::write_all(&mut sidecar, json.as_bytes())
        .map_err(|e| format!("{}: write sidecar {}: {}", fn_name, path, e))?;
    Ok(Value::String(path))
}

/// `media_manifest(handle)` — the in-program provenance read (№337,
/// ADR-0166 §2.4): `Struct { kind, origin, conf, synthetic, bytes_sha256,
/// refs, sealed }` WITHOUT materializing bytes (the hash is the
/// entry-level insert-time fact).
pub fn media_manifest_dispatch(store: &MediaStore, args: &[Value]) -> Result<Value, String> {
    let fn_name = "media_manifest";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (handle), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle = expect_media_handle(fn_name, args, 0)?;
    let entry = store.entry(handle)?;
    let mut fields = std::collections::HashMap::new();
    fields.insert(
        "kind".to_string(),
        Value::String(entry.kind.slug().to_string()),
    );
    fields.insert(
        "origin".to_string(),
        Value::String(entry.origin.clone().unwrap_or_default()),
    );
    fields.insert(
        "conf".to_string(),
        Value::String(entry.label.conf.as_str().to_string()),
    );
    fields.insert("synthetic".to_string(), Value::Bool(entry.synthetic));
    fields.insert(
        "bytes_sha256".to_string(),
        Value::String(entry.bytes_sha256.clone()),
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
        type_name: "MediaManifest".to_string(),
        fields,
    })
}

/// `media_manifest_read(path)` — the sidecar READ path (№337, ADR-0166
/// §2.4): parses a `<...>.manifest.json` from the sandbox and returns
/// the same struct shape as media_manifest. Missing/empty/corrupt
/// manifests are LOUD errors (№320 posture); a manifest without
/// `synthetic` reads TRUE (conservative, unknown ⇒ marked). Stateless —
/// a plain registry builtin.
pub(crate) fn builtin_media_manifest_read(args: &[Value]) -> Result<Value, String> {
    let fn_name = "media_manifest_read";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (path), got {}",
            fn_name,
            args.len()
        ));
    }
    let path = expect_string(fn_name, args, 0)?;
    // №254 outcome split with the №337 posture: a MISSING sidecar is a
    // loud provenance refusal (not a sandbox violation, not a soft
    // failure — "вход без манифеста" is exactly what the read path
    // surfaces); textual violations (absolute path, `..`) stay loud
    // SANDBOX_VIOLATION.
    let safe_path = match crate::builtins::io::sandbox_path(&path) {
        Ok(p) => p,
        Err(e) => {
            if crate::builtins::io::sandbox_path_missing(&path) {
                return Err(format!(
                    "{}: cannot read sidecar '{}': no such file — provenance you \
                     cannot read is refused, never defaulted (№320 posture)",
                    fn_name, path
                ));
            }
            return Err(crate::builtins::io::sandbox_violation(e));
        }
    };
    let json = std::fs::read_to_string(&safe_path).map_err(|e| {
        format!(
            "{}: cannot read sidecar '{}': {} — provenance you cannot read is \
             refused, never defaulted (№320 posture)",
            fn_name, path, e
        )
    })?;
    let manifest = crate::media::sidecar_parse_manifest(&json)?;
    let mut fields = std::collections::HashMap::new();
    fields.insert("kind".to_string(), Value::String(manifest.kind));
    fields.insert("origin".to_string(), Value::String(manifest.origin));
    fields.insert("conf".to_string(), Value::String(manifest.conf));
    fields.insert("synthetic".to_string(), Value::Bool(manifest.synthetic));
    fields.insert(
        "bytes_sha256".to_string(),
        Value::String(manifest.bytes_sha256),
    );
    fields.insert("timestamp".to_string(), Value::String(manifest.timestamp));
    Ok(Value::Struct {
        type_name: "MediaManifest".to_string(),
        fields,
    })
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
    // №332 (ADR-0164): the bound origin name — the provenance of the
    // handle, observable WITHOUT materializing bytes. Empty string when
    // the entry was constructed unbound (pre-№332 store API surface).
    fields.insert(
        "origin".to_string(),
        Value::String(entry.origin.clone().unwrap_or_default()),
    );
    Ok(Value::Struct {
        type_name: "MediaMeta".to_string(),
        fields,
    })
}

/// Runtime shape of a declared origin for the dispatch layer (extracted
/// from the compiled declaration; conf re-validated loudly).
fn runtime_origin(
    origins: &std::collections::HashMap<String, crate::bytecode::CompiledOriginDecl>,
    fn_name: &str,
    name: &str,
) -> Result<(String, MediaKind, crate::labels::Conf, Option<String>), String> {
    let decl = origins.get(name).ok_or_else(|| {
        format!(
            "{}: unknown origin '{}' (no `origin` declaration in this program)",
            fn_name, name
        )
    })?;
    let conf =
        parse_sensitivity(&decl.conf).map_err(|e| format!("origin '{}': {}", decl.name, e))?;
    let media =
        MediaKind::from_slug(&decl.media).map_err(|e| format!("origin '{}': {}", decl.name, e))?;
    Ok((decl.kind.clone(), media, conf, decl.path.clone()))
}

/// `media_source_capture(origin_name)` — the HandleSource runtime
/// (№332, ADR-0164): resolves the declared origin and captures through
/// the store. `kind: file` reads the sandboxed path (loud on missing
/// files); `kind: camera` is a loud PARKED boundary (real capture
/// hardware does not exist in this environment — №294 class); the
/// STATIC origin chain is unaffected (compile-time denies still hold).
pub fn media_source_capture_dispatch(
    store: &mut MediaStore,
    origins: &std::collections::HashMap<String, crate::bytecode::CompiledOriginDecl>,
    args: &[Value],
) -> Result<Value, String> {
    let fn_name = "media_source_capture";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (origin name), got {}",
            fn_name,
            args.len()
        ));
    }
    let name = expect_string(fn_name, args, 0)?;
    let (kind, media, conf, path) = runtime_origin(origins, fn_name, &name)?;
    match kind.as_str() {
        "file" => {
            let path = path.ok_or_else(|| {
                format!("{}: origin '{}' (file) has no path", fn_name, name)
            })?;
            // №131/№252/№254: sandboxed read, loud violations, missing
            // file classified loudly (the capture source MUST exist).
            let safe_path = crate::builtins::io::sandbox_path(&path)
                .map_err(crate::builtins::io::sandbox_violation)?;
            let bytes = std::fs::read(&safe_path).map_err(|e| {
                format!(
                    "{}: cannot capture from '{}' ({}): {}",
                    fn_name, name, path, e
                )
            })?;
            let handle = store.insert(
                media,
                bytes,
                Label {
                    conf,
                    integrity: crate::labels::Integrity::Trusted,
                    consent: Default::default(),
                },
            )?;
            store.bind_origin(handle, name.clone(), conf, &kind)?;
            Ok(Value::Media(handle))
        }
        "camera" => Err(format!(
            "{}: camera capture for origin '{}' is a PARKED boundary (real capture hardware does not exist in this environment; №294 class) — use a file-backed origin for end-to-end runs; the static origin chain is unaffected (compile-time denies still hold)",
            fn_name, name
        )),
        other => Err(format!(
            "{}: origin '{}' has kind '{}' — source capture requires camera | file",
            fn_name, name, other
        )),
    }
}

/// `media_bind_origin(origin_name, handle)` — the ProvBind runtime
/// (№332, ADR-0164): binds the store entry's origin and joins the
/// declared origin conf into the entry label (re-sealing when a public
/// entry becomes non-public). The handle value passes through unchanged.
pub fn media_bind_origin_dispatch(
    store: &mut MediaStore,
    origins: &std::collections::HashMap<String, crate::bytecode::CompiledOriginDecl>,
    args: &[Value],
) -> Result<Value, String> {
    let fn_name = "media_bind_origin";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (origin name, handle), got {}",
            fn_name,
            args.len()
        ));
    }
    let name = expect_string(fn_name, args, 0)?;
    let handle = expect_media_handle(fn_name, args, 1)?;
    let (kind, _, conf, _) = runtime_origin(origins, fn_name, &name)?;
    // №337 (ADR-0166 §2.3): the declared kind drives the Art. 50 marking
    // — a generation bind FORCES synthetic: true on the bound entry.
    store.bind_origin(handle, name, conf, &kind)?;
    Ok(Value::Media(handle))
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

/// `media_source_capture(origin_name)` — the HandleSource runtime (№332, ADR-0164): resolves the declared origin and captures a handle through the media store. `kind: file` reads the sandboxed path (loud on missing files); `kind: camera` is a loud PARKED boundary (real capture hardware does not exist in this environment). Source: the handle label is the origin's declared conf. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_source_capture_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_source_capture: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0164); the generic fallback must not be reached".to_string())
}

/// `media_bind_origin(origin_name, handle)` — the ProvBind runtime (№332, ADR-0164): binds the store entry's origin and joins the declared origin conf into the entry label (re-sealing when a public entry becomes non-public). The handle value passes through unchanged. Pure: store bookkeeping, no byte movement. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_bind_origin_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_bind_origin: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0164); the generic fallback must not be reached".to_string())
}

/// `media_manifest(handle)` — the in-program provenance read (№337, ADR-0166 §2.4): Struct { kind, origin, conf, synthetic, bytes_sha256, refs, sealed } WITHOUT materializing bytes. State-carrying: interpreter/VM intercept before the generic fallback.
pub(crate) fn builtin_media_manifest_stub(_args: &[Value]) -> Result<Value, String> {
    Err("media_manifest: state-carrying media builtin — dispatched via interpreter/vm interception (ADR-0166); the generic fallback must not be reached".to_string())
}
