use std::collections::HashMap;
use zeroize::Zeroizing;

/// A single variant inside a Fluid value (runtime). Contains a concrete
/// value, its declared type name, and a confidence score (0.0..1.0).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FluidValueVariant {
    pub type_name: String,
    pub value: Value,
    pub confidence: f64,
}

/// Opaque secret string with automatic memory zeroing on drop (Phase 7.3).
/// Implements serde by serializing as "[SECRET]" marker — actual value is NEVER persisted.
#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);

impl serde::Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Never serialize the actual secret — emit a safe marker
        s.serialize_str("[SECRET]")
    }
}

impl<'de> serde::Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let inner = String::deserialize(d)?;
        Ok(SecretString(Zeroizing::new(inner)))
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString([REDACTED])")
    }
}

impl SecretString {
    pub fn new(s: String) -> Self {
        SecretString(Zeroizing::new(s))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Runtime value.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
    String(String),
    Float(f64),
    Bool(bool),
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    /// List value: ordered collection of items.
    List(Vec<Value>),
    /// Fluid value: superposition of typed variants with confidence scores.
    /// Collapses lazily at point of use (see `maybe_collapse`).
    Fluid(Vec<FluidValueVariant>),
    Unit,
    /// Opaque HTML content (Phase 6.2) — cannot be concatenated, printed, or converted to String
    Html(String),
    /// Opaque SQL query (Phase 6.3) — only created via query() builtin
    Query(String),
    /// Opaque secret value (Phase 6.4) — cannot be printed or converted to String.
    /// Phase 7.3: Internally uses SecretString (Zeroizing<String>) — memory is zeroed on drop.
    Secret(SecretString),
    /// Opaque encrypted data (Phase 6.4)
    Encrypted(Vec<u8>),
    /// Opaque password hash (Phase 6.4)
    Hash(String),
    /// Opaque session data (Phase 6.5)
    Session(std::collections::HashMap<String, String>),
    /// HTTP response value (Phase 6.1)
    HttpResponse {
        status: u16,
        body: String,
    },
    /// Graph subgraph — opaque first-class graph value (V3).
    /// Contains a serializable GraphSnapshot that can be passed between functions.
    Subgraph(crate::memory_graph::GraphSnapshot),
    /// Opaque Reflex model handle (Наряд №178, ADR-0114).
    /// Contains an index into ReflexRegistry — weights never enter Value.
    /// Debug prints only name and last_metric, not weights.
    Reflex(crate::nn::ReflexId),
    /// Opaque BPE vocabulary handle (Наряд №195).
    /// Contains an index into BPE_REGISTRY — vocab data never enters Value.
    BpeVocab(crate::nn::bpe::BpeVocabId),
    /// Opaque Vision artifact handle (Наряд №210, ADR-0124).
    /// Contains an index into VisionRegistry — vision artifacts never enter Value.
    Vision(crate::vision::VisionId),
    /// Opaque Voice artifact handle (Наряд №302, ADR-0144).
    /// Contains an index into VoiceRegistry — voiceprints/audio never enter Value.
    Voice(crate::voice::VoiceId),
    /// Opaque Audio artifact handle (Наряд №302, ADR-0144).
    /// Contains an index into VoiceRegistry — audio bytes never enter Value.
    Audio(crate::voice::AudioId),
    /// Opaque Video artifact handle (Наряд №307, ADR-0148).
    /// Contains an index into VideoRegistry — video bytes never enter Value.
    Video(crate::video::VideoId),
    /// Opaque LLM stream handle (Наряд №275, ADR-0137).
    /// Contains an index into `crate::llm::LLM_STREAM_REGISTRY` —
    /// the active `reqwest::blocking::Response` + SSE line buffer +
    /// provenance/usage aggregation never enter `Value`.
    LlmStream(crate::llm::LlmStreamId),
    /// Unified media handles (Наряд №331, ADR-0162): Image/Audio/
    /// VideoFrame/VideoSegment. Bytes never enter `Value` — only the
    /// opaque index into the per-interpreter (or per-VM) `MediaStore`.
    /// Byte egress is reachable only through sanctioned sinks
    /// (`media_save`, gated by №325 + the runtime backstop).
    Media(crate::media::MediaHandle),
    /// Opaque Grant capability handle (Naryad #390, ADR-0155 §3.1).
    /// The handle is an untrusted cache of the grant-ledger record
    /// (`src/grants.rs` — the SSOT for class/quota/revocation state).
    /// Non-printable, non-serializable (serde emits a dead "[GRANT]"
    /// marker — rule 2 of the ADR linearity rules); a grant consumed by
    /// use, or reconstructed through deserialization, refuses every
    /// later use with a typed error (GRANT_REUSED / GRANT_EXPIRED).
    Grant(crate::grants::GrantHandle),
    /// Opaque one-time likeness challenge (Naryad #387, ADR-0149 D1).
    /// Consumed by `likeness_verify`; serde emits a dead marker.
    LikenessChallenge(crate::likeness::ChallengeHandle),
    /// Opaque likeness consent token (Naryad #387, ADR-0149 D1/D6) —
    /// the cross-pillar credential for private/camera-origin media
    /// egress. Constructed ONLY by `likeness_verify`; a String can
    /// never occupy a token position (serde dead marker, non-printable,
    /// typed challenge parameter — the P1-7 unforgeability contract).
    Likeness(crate::likeness::TokenHandle),
    /// Opaque typed-memory container handle (Naryad #350 — the
    /// Memory<K> layer; K is the Phase-1 confidentiality label carried
    /// in the projection map). The registry in `src/memory_typed.rs` is
    /// the state; the map (`id`/`subject`/`label`) is the printable
    /// projection only (the ADR-0114 opaque pattern, the Session
    /// precedent). Appended LAST — bincode variant indices of the
    /// existing variants stay stable (.mbc compat, the №250/№264 rule).
    Memory(std::collections::HashMap<String, String>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Struct { type_name, fields } => {
                write!(f, "{} {{", type_name)?;
                let pairs: Vec<_> = fields.iter().collect();
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Fluid(variants) => {
                // Display as the highest-confidence variant
                let best = variants.iter().max_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                match best {
                    Some(v) => write!(f, "{}", v.value),
                    None => write!(f, "()"),
                }
            }
            Value::Unit => write!(f, "()"),
            // Наряд №173b: Html renders its content through Display — the
            // content was already sanitized by render()/escape_html() before
            // being wrapped in Value::Html. The [Html] placeholder was
            // causing 3 golden test failures (p115_render_basic,
            // p115_xss_escape, p6_xss_safe) where the expected output is
            // the actual HTML string, not "[Html]".
            //
            // This is safe because Value::Html is only created by:
            //   1. render() — which escapes all interpolated values
            //   2. escape_html() — which escapes the entire string
            //   3. template rendering — which uses {{ }} (auto-escaped)
            // There is no way to create Value::Html with unsanitized content.
            Value::Html(s) => write!(f, "{}", s),
            Value::Query(_) => write!(f, "[Query]"),
            Value::Secret(_) => write!(f, "[Secret]"),
            Value::Encrypted(_) => write!(f, "[Encrypted]"),
            Value::Hash(_) => write!(f, "[Hash]"),
            Value::Session(_) => write!(f, "[Session]"),
            Value::Memory(_) => write!(f, "[Memory]"),
            Value::HttpResponse { status, .. } => write!(f, "[HttpResponse {}]", status),
            Value::Subgraph(snap) => write!(
                f,
                "[Subgraph {} nodes, {} edges]",
                snap.nodes.len(),
                snap.edges.len()
            ),
            Value::Reflex(id) => write!(f, "[Reflex#{}]", id.0),
            Value::BpeVocab(id) => write!(f, "[BpeVocab#{}]", id.0),
            // Наряд №210: Vision handle display — лекала Reflex.
            Value::Vision(id) => write!(f, "[Vision#{}]", id.0),
            // Наряд №302: Voice/Audio handle display — лекала Vision.
            Value::Voice(id) => write!(f, "[Voice#{}]", id.0),
            Value::Audio(id) => write!(f, "[Audio#{}]", id.0),
            // Наряд №307: Video handle display — лекала Vision/Voice.
            Value::Video(id) => write!(f, "[Video#{}]", id.0),
            // Наряд №275 (ADR-0137): LLM stream handle display —
            // лекала Reflex/Vision.
            Value::LlmStream(id) => write!(f, "[LlmStream#{}]", id.0),
            // Наряд №331 (ADR-0162): media handle display — per-kind
            // format from the handle itself ([Image#N] etc.).
            Value::Media(h) => write!(f, "{}", h),
            // Naryad #390 (ADR-0155): grant handles display as an opaque
            // marker — no scope/class detail leaks through Display (the
            // print()/to_string() surface refuses Grants outright).
            Value::Grant(_) => write!(f, "[Grant]"),
            // Naryad #387 (ADR-0149 D1/D6): likeness handles display as
            // opaque markers — no subject/scope detail leaks.
            Value::LikenessChallenge(_) => write!(f, "[LikenessChallenge]"),
            Value::Likeness(_) => write!(f, "[LikenessToken]"),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "String",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::List(_) => "List",
            Value::Struct { .. } => "Struct",
            Value::Fluid(_) => "Fluid",
            Value::Unit => "Unit",
            Value::Html(_) => "Html",
            Value::Query(_) => "Query",
            Value::Secret(_) => "Secret",
            Value::Encrypted(_) => "Encrypted",
            Value::Hash(_) => "Hash",
            Value::Session(_) => "Session",
            Value::Memory(_) => "Memory",
            Value::HttpResponse { .. } => "HttpResponse",
            Value::Subgraph(_) => "Subgraph",
            Value::Reflex(_) => "Reflex",
            Value::BpeVocab(_) => "BpeVocab",
            // Наряд №210: Vision handle type name.
            Value::Vision(_) => "vision",
            // Наряд №302: Voice/Audio handle type name.
            Value::Voice(_) => "Voice",
            Value::Audio(_) => "Audio",
            // Наряд №307: Video handle type name.
            Value::Video(_) => "Video",
            // Наряд №275 (ADR-0137): LLM stream handle type name.
            Value::LlmStream(_) => "LlmStream",
            // Наряд №331 (ADR-0162): media handle type names —
            // "Image" / "Audio" / "VideoFrame" / "VideoSegment".
            Value::Media(h) => h.kind().type_name(),
            Value::Grant(_) => "Grant",
            // Naryad #387 (ADR-0149): likeness handle type names —
            // "LikenessChallenge" / "LikenessToken".
            Value::LikenessChallenge(_) => "LikenessChallenge",
            Value::Likeness(_) => "LikenessToken",
        }
    }

    /// Get a field value from a struct. Returns Err if not a struct or field missing.
    pub fn get_field(&self, field: &str) -> Result<&Value, String> {
        match self {
            Value::Struct { fields, .. } => fields
                .get(field)
                .ok_or_else(|| format!("field '{}' not found on struct", field)),
            _ => Err(format!(
                "cannot access field '{}' on non-struct value ({})",
                field,
                self.type_name()
            )),
        }
    }

    /// Set a field value on a mutable struct.
    pub fn set_field(&mut self, field: &str, value: Value) -> Result<(), String> {
        match self {
            Value::Struct { fields, .. } => {
                if fields.contains_key(field) {
                    fields.insert(field.to_string(), value);
                    Ok(())
                } else {
                    Err(format!("field '{}' not found on struct", field))
                }
            }
            _ => Err(format!("cannot set field '{}' on non-struct value", field)),
        }
    }

    /// Convert to f64 for numeric comparisons.
    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::String(s) => s
                .parse::<f64>()
                .map_err(|_| format!("cannot convert '{}' to Float", s)),
            _ => Err(format!("cannot convert {} to Float", self.type_name())),
        }
    }

    /// Convert to bool for condition checking.
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Float(f) => Ok(*f != 0.0),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Unit => Ok(false),
            _ => Err(format!("cannot convert {} to Bool", self.type_name())),
        }
    }

    /// Check if this value is a Fluid superposition.
    pub fn is_fluid(&self) -> bool {
        matches!(self, Value::Fluid(_))
    }
}

/// ADR-0142 (№374): the structured result of `try expr` —
/// `Struct { ok: Bool, value: Value, error: Unit | Struct { code, message } }`.
///
/// - success: `ok = true`, `value` = the inner expression's value, `error` = Unit;
/// - error:   `ok = false`, `value` = Unit, `error` = `Struct { code, message }`
///   where `code` is a STABLE diagnostic code (naryad №385, ADR-0169 — the
///   names below are frozen contracts under the ADR-0131 convention: the code
///   is a contract, the message text may change) and `message` carries the
///   full runtime error text.
///
/// The code set (ADR-0169 §3.1): `RUNTIME_ERROR` (honest fallback — an error
/// whose origin carries no source stamp), `LLM_TIMEOUT`,
/// `LLM_PROVIDER_UNAVAILABLE`, `SQL_ERROR`, `SANDBOX_VIOLATION`,
/// `SINK_CLEARANCE_RUNTIME`, `MEDIA_SEALED_EGRESS`, `BACKEND_DEGRADED`.
/// Classification happens ONCE per caught error, by the shared
/// [`stable_try_error_code`] reading the origin stamp the failing subsystem
/// put on the error — never by re-deriving the subsystem from message text.
///
/// Shared by BOTH backends (TW `Expr::Try` and VM `Instruction::TryEval`) so
/// the shape — and now the `code` — cannot diverge.
pub fn try_result_struct(ok: bool, value: Value, error: Option<(String, String)>) -> Value {
    let mut fields = std::collections::HashMap::new();
    fields.insert("ok".to_string(), Value::Bool(ok));
    fields.insert("value".to_string(), value);
    fields.insert(
        "error".to_string(),
        match error {
            None => Value::Unit,
            Some((code, message)) => Value::Struct {
                type_name: "TryError".to_string(),
                fields: {
                    let mut f = std::collections::HashMap::new();
                    f.insert("code".to_string(), Value::String(code));
                    f.insert("message".to_string(), Value::String(message));
                    f
                },
            },
        },
    );
    Value::Struct {
        type_name: "TryResult".to_string(),
        fields,
    }
}

// ── Stable `try.error.code` contracts (naryad №385, ADR-0169) ──────────
//
// ADR-0131 convention: a code is a FROZEN contract, the message text may
// change. These names are consumed by agent scenarios that branch on the
// failure kind (retry on LLM_TIMEOUT, hard-fail on SANDBOX_VIOLATION) —
// renaming any of them is a breaking language change and needs an ADR.

/// Honest fallback: the error's origin carries no source stamp.
pub const CODE_RUNTIME_ERROR: &str = "RUNTIME_ERROR";
/// Deadline / provider timeout in the `call_llm` contour.
pub const CODE_LLM_TIMEOUT: &str = "LLM_TIMEOUT";
/// LLM provider unreachable: connect failure or SmartRouter circuit open.
pub const CODE_LLM_PROVIDER_UNAVAILABLE: &str = "LLM_PROVIDER_UNAVAILABLE";
/// A `rusqlite::Error` raised by a `db_*` builtin (the SQL layer itself).
pub const CODE_SQL_ERROR: &str = "SQL_ERROR";
/// IO/exec sandbox refusal (`[SANDBOX_VIOLATION]` loud format, №254).
pub const CODE_SANDBOX_VIOLATION: &str = "SANDBOX_VIOLATION";
/// Runtime twin of the №325 static sink gate (VM `SinkCheck` backstop).
pub const CODE_SINK_CLEARANCE_RUNTIME: &str = "SINK_CLEARANCE_RUNTIME";
/// Sealed-at-rest media refused materialization (№325/ADR-0162 §2.5 backstop).
pub const CODE_MEDIA_SEALED_EGRESS: &str = "MEDIA_SEALED_EGRESS";
/// Backend ladder exhausted (№336/ADR-0165). The typed `Degraded(t)` result
/// reuses the SAME frozen name for its `error.code` field (single constant,
/// see `src/builtins/backends.rs`) — the typed path stays typed; if such a
/// failure ever travels the String error channel, it carries this stamp.
pub const CODE_BACKEND_DEGRADED: &str = "BACKEND_DEGRADED";
/// A cron/reminder mechanics failure (naryad №413): arg/type refusals,
/// the 5-field cron-expression contract, persistence lock errors — the
/// subsystem = the scheduler support surface (`src/builtins/cron.rs`).
pub const CODE_CRON_JOB_FAILED: &str = "CRON_JOB_FAILED";
/// The MCP server process failed to spawn (existing origin marker
/// `src/builtins/mcp.rs`, now whitelisted for `try`; naryad №413).
pub const CODE_MCP_SPAWN_FAILED: &str = "MCP_SPAWN_FAILED";
/// An MCP contour timeout (existing origin marker, now whitelisted).
pub const CODE_MCP_TIMEOUT: &str = "MCP_TIMEOUT";
/// An MCP stdio IO failure (existing origin marker, now whitelisted).
pub const CODE_MCP_IO_ERROR: &str = "MCP_IO_ERROR";
/// The MCP server reported `isError=true` for the tool call (existing
/// origin marker `src/builtins/mcp.rs`, now whitelisted for `try`).
pub const CODE_MCP_TOOL_ERROR: &str = "MCP_TOOL_ERROR";
/// The MCP server does not know the tool (JSON-RPC -32602 on tools/call;
/// existing origin marker, now whitelisted for `try`).
pub const CODE_MCP_TOOL_NOT_FOUND: &str = "MCP_TOOL_NOT_FOUND";
/// An MCP JSON-RPC protocol violation or unsupported shape (existing
/// origin marker, now whitelisted for `try`).
pub const CODE_MCP_PROTOCOL_ERROR: &str = "MCP_PROTOCOL_ERROR";
/// The MCP allowlist refused the server (№268/ADR-0132 D3 policy refusal;
/// existing origin marker, now whitelisted for `try`).
pub const CODE_MCP_NOT_ALLOWLISTED: &str = "MCP_NOT_ALLOWLISTED";

/// The whitelist of codes a subsystem may stamp onto the String error
/// channel. `RUNTIME_ERROR` is deliberately NOT in this list: it is the
/// fallback for unstamped errors, never an explicit stamp.
const ORIGIN_STAMPED_CODES: &[&str] = &[
    CODE_LLM_TIMEOUT,
    CODE_LLM_PROVIDER_UNAVAILABLE,
    CODE_SQL_ERROR,
    CODE_SANDBOX_VIOLATION,
    CODE_SINK_CLEARANCE_RUNTIME,
    CODE_MEDIA_SEALED_EGRESS,
    CODE_BACKEND_DEGRADED,
    // №413: the cron mechanics + the MCP contour. The MCP taxonomy ALREADY
    // existed as position-0 markers at the origin (spawn/timeout/io/
    // protocol/tool-error/tool-not-found/allowlist) — the naryad whitelists
    // them for `try` instead of adding a coarser duplicate; see the
    // constant docs and the report in issue #558.
    CODE_CRON_JOB_FAILED,
    CODE_MCP_SPAWN_FAILED,
    CODE_MCP_TIMEOUT,
    CODE_MCP_IO_ERROR,
    CODE_MCP_TOOL_ERROR,
    CODE_MCP_TOOL_NOT_FOUND,
    CODE_MCP_PROTOCOL_ERROR,
    CODE_MCP_NOT_ALLOWLISTED,
];

/// Stamp an error at its ORIGIN with a stable code (naryad №385, ADR-0169).
///
/// The stamp is the existing loud `[CODE] ` prefix convention (№254's
/// `[SANDBOX_VIOLATION] …` generalized): it is set at the place where the
/// subsystem KNOWS what failed, and read back by [`stable_try_error_code`]
/// at the `try` sewing points. The visible message text is the stamp plus
/// the plain text — nothing is hidden, and an unstamped error is untouched.
pub fn coded_error(code: &str, msg: impl std::fmt::Display) -> String {
    format!("[{}] {}", code, msg)
}

/// Split an origin stamp off the front of an error string.
///
/// Returns `(code, rest_without_stamp)` when `err` starts with a whitelisted
/// `[<CODE>] ` marker, `None` otherwise. Only a marker at position 0 counts —
/// a `[CODE]`-looking substring mid-message is content, not a stamp, so a
/// program cannot forge a classification by echoing a marker into its text.
pub fn split_origin_stamp(err: &str) -> Option<(&'static str, &str)> {
    for code in ORIGIN_STAMPED_CODES {
        let marker = format!("[{}] ", code);
        if let Some(rest) = err.strip_prefix(marker.as_str()) {
            return Some((code, rest));
        }
    }
    None
}

/// The ONE classification point for `try.error.code` (naryad №385).
///
/// Called by ALL THREE sewing points (TW `Expr::Try` in
/// `src/interpreter/execution.rs`, VM `Instruction::TryEval` in `src/vm.rs`)
/// so TW and VM necessarily agree: the same error string classifies to the
/// same code on both backends — parity by construction, divergence is a bug
/// caught by the crosscheck parity gate plus `tests/naryad_385_try_codes.rs`.
///
/// Classification is by ORIGIN STAMP (the failing subsystem's own marker),
/// never by parsing message prose: an unstamped error — whatever its text —
/// is honestly `RUNTIME_ERROR`.
pub fn stable_try_error_code(err: &str) -> &'static str {
    split_origin_stamp(err)
        .map(|(code, _)| code)
        .unwrap_or(CODE_RUNTIME_ERROR)
}

/// Prepend `head` to an error while keeping its origin stamp at the FRONT.
///
/// Wrapper layers ("call_llm() failed: …", "All LLM providers failed. …")
/// must not bury the origin stamp mid-message — the classifier reads only
/// position 0, so a naive `format!("{}: {}", head, err)` would demote a
/// stamped provider failure to an unspecific `RUNTIME_ERROR`. Unstamped
/// errors wrap exactly as before.
pub fn wrap_error_preserving_code(head: &str, err: &str) -> String {
    match split_origin_stamp(err) {
        Some((code, rest)) => coded_error(code, format!("{}: {}", head, rest)),
        None => format!("{}: {}", head, err),
    }
}

/// Opaque / sensitive values that must not be rendered by print.
/// Наряд №114.
pub fn is_nonprintable(v: &Value) -> bool {
    matches!(
        v,
        Value::Html(_)
            | Value::Query(_)
            | Value::Secret(_)
            | Value::Encrypted(_)
            | Value::Hash(_)
            | Value::Subgraph(_)
            | Value::Reflex(_)
            | Value::BpeVocab(_)
            // Naryad #390 (ADR-0155 rule 2): a grant is a capability —
            // printing/displaying it is refused like every other opaque
            // security value.
            | Value::Grant(_)
            // Naryad #387 (ADR-0149 D1/D6): likeness handles are opaque
            // security credentials — printing them is refused like every
            // other opaque handle (the ADR-0114 convention).
            | Value::LikenessChallenge(_)
            | Value::Likeness(_)
            // Наряд №210: Vision handle is opaque — must not be printed directly.
            | Value::Vision(_)
            // Наряд №302: Voice/Audio handles are opaque.
            | Value::Voice(_)
            | Value::Audio(_)
            // Наряд №307: Video handle is opaque.
            | Value::Video(_)
            // Наряд №275 (ADR-0137): LLM stream handle is opaque —
            // printing it would leak the active stream's identity
            // (provider, model, handle index) but no PII; still, the
            // convention for all opaque handles is non-printable.
            | Value::LlmStream(_)
            // Наряд №331 (ADR-0162): media handles are opaque — printing
            // them exposes only the index, but the convention for ALL
            // opaque handles is non-printable.
            | Value::Media(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_never_leaks() {
        let secret = SecretString::new("super-secret-value-12345".to_string());
        let debug_output = format!("{:?}", secret);
        assert!(
            !debug_output.contains("super-secret-value-12345"),
            "Debug output must not contain the actual secret value"
        );
        assert_eq!(debug_output, "SecretString([REDACTED])");
    }

    #[test]
    fn value_secret_debug_never_leaks() {
        let value = Value::Secret(SecretString::new("another-secret".to_string()));
        let debug_output = format!("{:?}", value);
        assert!(
            !debug_output.contains("another-secret"),
            "Value Debug output must not contain the actual secret value"
        );
    }
}
