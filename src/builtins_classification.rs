// ── Builtin classification: role × label × reversibility (Наряд №316) ──
//
// SSOT static map for ALL registered builtins (устав §11 Шаг 3, план v2
// §13.2 шаг 0.3, §16.2). Authored by scripts/gen_classification.py from
// its curated table; the REFERENCE.md classification block is generated
// FROM this map. Tests in this module enforce: coverage 100% of
// BUILTIN_REGISTRY, uniqueness, no extras, rationale on every non-Pure.
//
// Role semantics:
// - Pure   — deterministic compute on in-program values (no ingress/egress,
//            no persistent-state effect). Parsing already-present bytes is
//            Pure.
// - Source — brings data from beyond the expression boundary INTO the
//            program: external services, files, env, wall clock, entropy,
//            AND the runtime's own persistent stores (state reads carry
//            provenance).
// - Lift   — raises the clearance of its input for further flow: taint
//            sanitizers (redact/render/escape_*, the audit.rs vocabulary)
//            and one-way de-identifiers (hash_password, encrypt, digests).
// - Sink   — sends data beyond the expression boundary OUT: public
//            channels, network delivery, persistent state writes, host
//            effects.
//
// Label semantics (default_label — the clearance of data at the builtin's
// boundary by default; №316 proposal, refined by the Фаза 1 label-checker):
// - Public   — safe for public channels (sanitized outputs, stdout)
// - Internal — program/user data inside the perimeter (local files, DB,
//              persistent stores, untrusted user input)
// - Secret   — secrets and biometrics (env/secret, keys, plaintext
//              credentials, voiceprints — GDPR Art. 9)
// - Network  — data crossing the network boundary (external services)
//
// Reversibility semantics (the EFFECT, not the function):
// - Pure         — no external effect to undo (local compute)
// - Reversible   — external/persistent effect that CAN be undone
// - Irreversible — external/persistent effect that CANNOT be undone
//                  (delivery, host exec, destructive deletes)

/// Data-clearance label at the builtin boundary (№316 proposal;
/// refined by the Фаза 1 label-checker per план v2 §16.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    Public,
    Internal,
    Secret,
    Network,
}

impl Label {
    pub fn as_str(&self) -> &'static str {
        match self {
            Label::Public => "public",
            Label::Internal => "internal",
            Label::Secret => "secret",
            Label::Network => "network",
        }
    }
}

/// Data-flow role of a builtin (№316, устав §11 Шаг 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Pure,
    Source,
    Lift,
    Sink,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Pure => "pure",
            Role::Source => "source",
            Role::Lift => "lift",
            Role::Sink => "sink",
        }
    }
}

/// Undoability of the builtin's external effect (№316).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reversibility {
    Pure,
    Reversible,
    Irreversible,
}

impl Reversibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Reversibility::Pure => "pure",
            Reversibility::Reversible => "reversible",
            Reversibility::Irreversible => "irreversible",
        }
    }
}

/// Classification of one builtin (№316): role × default label ×
/// reversibility + one-sentence rationale for every non-Pure role.
#[derive(Debug, Clone, Copy)]
pub struct BuiltClass {
    pub role: Role,
    pub default_label: Label,
    pub reversibility: Reversibility,
    pub rationale: &'static str,
}

/// One classified registry entry.
#[derive(Debug, Clone, Copy)]
pub struct BuiltClassEntry {
    pub name: &'static str,
    pub class: BuiltClass,
}

/// Classification lookup — linear scan over a small static array; the map
/// is compile-time data, uniqueness is test-enforced.
pub fn classify(name: &str) -> Option<&'static BuiltClass> {
    BUILTIN_CLASSES
        .iter()
        .find(|e| e.name == name)
        .map(|e| &e.class)
}

/// SSOT map: имя → BuiltClass for EVERY registered builtin (№316).
pub static BUILTIN_CLASSES: &[BuiltClassEntry] = &[
    BuiltClassEntry { name: "upper", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "lower", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "len", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "str", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "contains", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "index_of", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "substring", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "char_at", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "starts_with", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "ends_with", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "trim", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "replace", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "split", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "join", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "length", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reverse", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "escape_html", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "HTML-escapes its input — taint-sanitizer per audit.rs (Sanitized)" } },
    BuiltClassEntry { name: "escape_json", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "JSON-escapes its input — sanitizer family of escape_html" } },
    BuiltClassEntry { name: "redact", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "taint-sanitizer — removes Secret taint (mask before sink, ADR-0136; audit.rs redact_result_taint)" } },
    BuiltClassEntry { name: "escape_js", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "JS-escapes its input — sanitizer family of escape_html" } },
    BuiltClassEntry { name: "fuzzy_match", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "strip", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chomp", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "repeat", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pad_left", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pad_right", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "lines", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "words", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "token_count", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "type_of", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "format", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "trim_start", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "trim_end", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "truncate", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "slugify", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "word_wrap", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "capitalize", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "title_case", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__trim", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__replace", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__split", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__join", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__abs", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__min", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__max", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__clamp", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__round", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__first", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "__last", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    // ── Наряд №333 (ADR-0163): backend registry listing ──
    BuiltClassEntry { name: "backend_list", class: BuiltClass { role: Role::Source, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "reads the static backend registry metadata (name/class/weights_id/pin/license — ADR-0163) — no weights bytes exist behind the entries" } },
    // №336 (ADR-0165): the backend try-chain — reads registry metadata,
    // returns selection/degradation data. No egress, no execution behind
    // it (the class callables execute); the ladder attempts are audit
    // events (№326 posture).
    BuiltClassEntry { name: "backend_select", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "backend try-chain over the №333 registry SSOT (№336, ADR-0165): picks the first available rung or returns Degraded(t) — a typed result, never a panic, never a silent mock; every attempt is an audit event" } },
    // №334: the local backend call surface — mock-first, no upload, no
    // egress (the audio/image/prompt stay in-process; real mode requires
    // SHA-pinned local weights and refuses loudly without them).
    BuiltClassEntry { name: "stt_transcribe", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "local STT backend call (№334, whisper-turbo canon): ingests the transcript into the flow; the audio stays local (no upload — unlike whisper_transcribe); real mode requires SHA-pinned weights (PARKED №294)" } },
    BuiltClassEntry { name: "omni_ask", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "local omni backend call (№334, nemotron canon): ingests the model answer into the flow; no network egress; real mode requires SHA-pinned weights (PARKED №294)" } },
    BuiltClassEntry { name: "vision_understand", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "local vision-understanding backend call (№334, molmoact2 canon): ingests the answer about an image into the flow; no upload, no egress; real mode requires SHA-pinned weights (PARKED №294)" } },
    // №335: the consent component (spec §7.2 v2). grant/revoke are label
    // transforms with process-local ledger bookkeeping (not egress); the
    // quarantine sink and the ledger export ARE egress — audited.
    BuiltClassEntry { name: "consent_grant", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "records (subject, scope, TTL) in the consent ledger and passes the value through with the consent scope EXTENDED (semantic.rs label_source) — process-local bookkeeping, no egress" } },
    BuiltClassEntry { name: "consent_revoke", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "records the revocation and returns the value under the QUARANTINE label — the flat cascade is lattice absorption (poison is absorbing, ADR-0154 §2.1); process-local bookkeeping" } },
    BuiltClassEntry { name: "quarantine_write", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "THE quarantine sink — the only legal egress for poisoned values (№325 clearance exempts it); unconditional QUARANTINE_EGRESS audit event (№326 posture)" } },
    BuiltClassEntry { name: "consent_ledger_export", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "dumps the consent ledger as JSON to a sandboxed path — FILE EGRESS with an audit event (grant/TTL/revoke records never leave the process silently)" } },
    // Naryad #390 (ADR-0155): the Grant algebra surface. Issue/subgrant
    // mint capability values from process-local ledger bookkeeping (no
    // egress, revocable → reversible); revoke/use are irreversible state
    // transitions (quota consumption and cascading revocation cannot be
    // undone); db_execute_with_grant is the same DB sink as db_execute,
    // now capability-gated (still irreversible).
    BuiltClassEntry { name: "grant_issue", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "mints an opaque Grant capability (ADR-0155 §3.1) recorded in the grant ledger — process-local bookkeeping, revocable via grant_revoke" } },
    BuiltClassEntry { name: "grant_subgrant", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "attenuation-only derivation of a Grant (ADR-0155 §3.3 rule 4) — narrower scope, shorter TTL, lower class power; ledger-recorded and revocable" } },
    BuiltClassEntry { name: "grant_revoke", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "cascading revocation (ADR-0155 §3.3 rule 5) — the target and every descendant transition to revoked; the ledger records are append-only" } },
    BuiltClassEntry { name: "grant_use", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "consumes one use of a grant (Once → consumed, N(n) → decrement) — quota consumption cannot be undone" } },
    BuiltClassEntry { name: "db_execute_with_grant", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "arbitrary SQL write under a capability grant (ADR-0155 §3.2) — same egress class as db_execute, gated by ledger state/TTL/scope/quota" } },
    BuiltClassEntry { name: "abs", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "min", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "max", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "clamp", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "round", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "exp", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "ln", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "sqrt", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pow", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "tanh", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "sigmoid", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "softmax", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "random_seed", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "random", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "newline", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "stdin", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "intended external stdin ingress (registry-only stub)" } },
    BuiltClassEntry { name: "split_tokens", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "if_eq", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "is_string_token", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "db_insert", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "intended DB row insert (registry-only stub) — persistent write" } },
    BuiltClassEntry { name: "float", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "to_string", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "to_float", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "print", class: BuiltClass { role: Role::Sink, default_label: Label::Public, reversibility: Reversibility::Irreversible, rationale: "prints to the public stdout channel — SECRET_LEAK semantics (№157), cannot be unsaid" } },
    BuiltClassEntry { name: "read_file", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests external file content into the program (input by provenance)" } },
    BuiltClassEntry { name: "write_file", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "writes persistent local state (undoable by file deletion)" } },
    BuiltClassEntry { name: "append_file", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "appends to persistent local state (undoable by truncation)" } },
    BuiltClassEntry { name: "delete_file", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destroys a local file with no undo path (issue minimum list)" } },
    BuiltClassEntry { name: "file_exists", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads filesystem metadata (state probe)" } },
    BuiltClassEntry { name: "list_dir", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads filesystem directory state" } },
    BuiltClassEntry { name: "exec", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "arbitrary host command execution — external effect on the host that cannot be undone (issue minimum list)" } },
    BuiltClassEntry { name: "exec_argv", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "argv-form of exec — same irreversible host effect" } },
    BuiltClassEntry { name: "git_push", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "pushes to a remote repository — external, non-undoable effect (issue minimum list)" } },
    BuiltClassEntry { name: "mcp_call", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests untrusted MCP tool output — UserInput taint by ADR-0132 D3" } },
    BuiltClassEntry { name: "mcp_list_tools", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external tool metadata over MCP (not tainted per ADR-0132, still external ingress)" } },
    // ── Наряд №331 (ADR-0162): unified media layer ──
    BuiltClassEntry { name: "media_store_image", class: BuiltClass { role: Role::Lift, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "wraps provided bytes into an opaque Image handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop" } },
    BuiltClassEntry { name: "media_store_audio", class: BuiltClass { role: Role::Lift, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "wraps provided bytes into an opaque Audio handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop" } },
    BuiltClassEntry { name: "media_store_video_frame", class: BuiltClass { role: Role::Lift, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "wraps provided bytes into an opaque VideoFrame handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop" } },
    BuiltClassEntry { name: "media_store_video_segment", class: BuiltClass { role: Role::Lift, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "wraps provided bytes into an opaque VideoSegment handle in the media store (ADR-0162) — no egress; declared sensitivity drives at-rest AES-GCM sealing and the runtime backstop" } },
    BuiltClassEntry { name: "media_save", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "the ONLY sanctioned materialization of media bytes — file egress through the io sandbox; №325 sink clearance (private-egress) + runtime backstop MEDIA_SEALED_EGRESS (ADR-0162 §2.5)" } },
    BuiltClassEntry { name: "media_retain", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "refcount +1 on a media handle (ADR-0162 §2.4) — pure store bookkeeping, no byte movement" } },
    BuiltClassEntry { name: "media_release", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "refcount −1 on a media handle; 0 evicts the entry (sealed bytes zeroized) — store bookkeeping, no external effect" } },
    BuiltClassEntry { name: "media_meta", class: BuiltClass { role: Role::Source, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "reads media store METADATA only (kind/conf/refs/sealed/origin) — no bytes leave the store" } },
    BuiltClassEntry { name: "media_source_capture", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "HandleSource runtime (№332/ADR-0164): captures a handle from a DECLARED origin (file-backed through the io sandbox; camera is a loud PARKED boundary) — the handle label is the origin's declared conf" } },
    BuiltClassEntry { name: "media_bind_origin", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "ProvBind runtime (№332/ADR-0164): binds an entry's origin and joins the declared conf into the entry label (re-seals when public becomes non-public) — store bookkeeping, no byte movement" } },
    // №337 (ADR-0166): the C2PA contour of handles — provenance reads.
    BuiltClassEntry { name: "media_manifest", class: BuiltClass { role: Role::Source, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "reads the entry-level manifest facts (kind/origin/conf/synthetic/bytes_sha256 — ADR-0166 §2.4) WITHOUT materializing bytes — store metadata, no egress" } },
    BuiltClassEntry { name: "media_manifest_read", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests a provenance sidecar (<path>.manifest.json) from the sandbox (ADR-0166 §2.4): manifest content enters the flow; missing/empty/corrupt sidecars are loud refusals (№320 posture), synthetic reads conservatively true" } },
    BuiltClassEntry { name: "get", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "push", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "slice", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "zip", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "sort_by", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "filter", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reduce", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "dedup", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "condense", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "unique", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chunk", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "sort", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "first", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "last", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "make_list", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "matches_any", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "parse_json", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "json_encode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "json_get", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "has_field", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "dict_get", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "dict_set", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "dict_has", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "dict_keys", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "dict_values", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "respond", class: BuiltClass { role: Role::Sink, default_label: Label::Public, reversibility: Reversibility::Irreversible, rationale: "writes the HTTP response — public channel, cannot be unsent" } },
    BuiltClassEntry { name: "respond_html", class: BuiltClass { role: Role::Sink, default_label: Label::Public, reversibility: Reversibility::Irreversible, rationale: "writes the HTTP response as HTML — public channel (escaping contract)" } },
    BuiltClassEntry { name: "form_data", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests untrusted user form input — UserInput taint (№201 vocabulary)" } },
    BuiltClassEntry { name: "json_body", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests untrusted request body — UserInput taint" } },
    BuiltClassEntry { name: "query_param", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests untrusted request query parameter — UserInput taint" } },
    BuiltClassEntry { name: "render", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "taint-sanitizing template render — output is public-safe (audit.rs sanitizer)" } },
    BuiltClassEntry { name: "http_get", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external network data (SSRF-guarded, №130)" } },
    BuiltClassEntry { name: "http_post", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "transmits program data to an external endpoint — cannot be unsent (issue minimum list)" } },
    BuiltClassEntry { name: "http_post_multipart", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "multipart upload to an external endpoint — same egress as http_post" } },
    BuiltClassEntry { name: "http_download", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "ingests remote bytes to a local file (network ingress with a disk side-effect)" } },
    BuiltClassEntry { name: "require", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads and enforces request context (auth/rate precondition state)" } },
    BuiltClassEntry { name: "request_body", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "alias of json_body — untrusted request body ingress" } },
    BuiltClassEntry { name: "web_search", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external search results" } },
    BuiltClassEntry { name: "geo_ip", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external geolocation data" } },
    BuiltClassEntry { name: "weather", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external weather data" } },
    BuiltClassEntry { name: "geo_distance", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "weather_forecast", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external forecast data" } },
    BuiltClassEntry { name: "hash_password", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "one-way de-identification of a password — output is safe for storage (argon2)" } },
    BuiltClassEntry { name: "verify_password", class: BuiltClass { role: Role::Pure, default_label: Label::Secret, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "encrypt", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "ciphertext is safe for untrusted channels — sensitivity lifted (AES-GCM)" } },
    BuiltClassEntry { name: "decrypt", class: BuiltClass { role: Role::Pure, default_label: Label::Secret, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "generate_key", class: BuiltClass { role: Role::Source, default_label: Label::Secret, reversibility: Reversibility::Pure, rationale: "materializes a fresh Secret from CSPRNG entropy" } },
    BuiltClassEntry { name: "base64_encode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "base64_decode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "authenticate", class: BuiltClass { role: Role::Pure, default_label: Label::Secret, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "session_login", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "creates a session (registry-only stub intent)" } },
    BuiltClassEntry { name: "session_logout", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "destroys the current session (stub intent)" } },
    BuiltClassEntry { name: "session_clear", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "wipes session state — no undo" } },
    BuiltClassEntry { name: "send_message", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "delivers a message to an external chat — cannot be unsent (issue minimum list)" } },
    BuiltClassEntry { name: "answer_callback_query", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "answers an external callback query" } },
    BuiltClassEntry { name: "edit_message_text", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "edits an already-delivered external message (reversible by further edits)" } },
    BuiltClassEntry { name: "whisper_transcribe", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external transcription of user audio; DUAL: uploads the audio to an external STT provider (№317 corpus)" } },
    BuiltClassEntry { name: "tts_send", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "synthesizes AND delivers audio externally — cannot be unsent (issue minimum list)" } },
    BuiltClassEntry { name: "tts_generate", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests an audio artifact from an external TTS provider; DUAL: transmits the text to the provider (№317 corpus)" } },
    BuiltClassEntry { name: "env", class: BuiltClass { role: Role::Source, default_label: Label::Secret, reversibility: Reversibility::Pure, rationale: "ingests environment secrets — Secret taint (audit.rs)" } },
    BuiltClassEntry { name: "query", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the program's persistent DB (state input with provenance)" } },
    BuiltClassEntry { name: "db_execute", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "arbitrary SQL write against the persistent DB — destructive statements are non-undoable (issue minimum list)" } },
    BuiltClassEntry { name: "call_llm", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests untrusted model output (LlmOutput taint, ADR-0117); DUAL: the prompt is transmitted to an external provider — №317 corpus must cover prompt-egress" } },
    BuiltClassEntry { name: "call_claude", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests untrusted model output (LlmOutput taint); DUAL: prompt egress to provider" } },
    BuiltClassEntry { name: "llm_usage", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads LLM usage accounting state" } },
    BuiltClassEntry { name: "call_llm_schema", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests schema-validated (still untrusted) model output; DUAL: prompt egress" } },
    BuiltClassEntry { name: "kv_set", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists to the program KV store (redact-before-persist per ADR-0136 applies)" } },
    BuiltClassEntry { name: "kv_get", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the program KV store — state input" } },
    BuiltClassEntry { name: "kv_delete", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destroys a persisted KV entry — no undo" } },
    BuiltClassEntry { name: "kv_exists", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads KV store state" } },
    BuiltClassEntry { name: "kv_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads KV store state" } },
    BuiltClassEntry { name: "mem_set", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists to long-term memory store" } },
    BuiltClassEntry { name: "mem_get", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads long-term memory store" } },
    BuiltClassEntry { name: "mem_delete", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destroys a memory entry — no undo" } },
    BuiltClassEntry { name: "memorize", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "alias of kv_set — persists to the memory store" } },
    BuiltClassEntry { name: "embed", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "vec_store", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists embeddings into the vector store (ADR-0134)" } },
    BuiltClassEntry { name: "vec_search", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the vector store (KNN state input)" } },
    BuiltClassEntry { name: "recall", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "intended memory recall — state read" } },
    BuiltClassEntry { name: "forget", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "intended destructive memory removal" } },
    BuiltClassEntry { name: "find", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "intended memory search — state read" } },
    BuiltClassEntry { name: "inspect", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "intended runtime introspection — state read" } },
    BuiltClassEntry { name: "deny_event", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "№392 DenyEvent read — handler-scoped runtime state, no egress" } },
    BuiltClassEntry { name: "deny_reason", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "№392 deny reason word — handler-scoped runtime state, no egress" } },
    BuiltClassEntry { name: "conv_start", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "intended conversation state creation" } },
    BuiltClassEntry { name: "conv_add", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "intended conversation state append" } },
    BuiltClassEntry { name: "conv_history", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "intended conversation state read" } },
    BuiltClassEntry { name: "conv_context", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "intended conversation state read" } },
    BuiltClassEntry { name: "conv_end", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "intended conversation state close" } },
    BuiltClassEntry { name: "session_set", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists web session state" } },
    BuiltClassEntry { name: "session_get", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads web session state" } },
    BuiltClassEntry { name: "ref", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "creates a reference into the content store — state read" } },
    BuiltClassEntry { name: "deref", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads content store state" } },
    BuiltClassEntry { name: "now", class: BuiltClass { role: Role::Source, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "wall-clock read — external (nondeterministic) input" } },
    BuiltClassEntry { name: "sleep", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "temporal effect — suspends execution (no data flow)" } },
    BuiltClassEntry { name: "time", class: BuiltClass { role: Role::Source, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "wall-clock read — external (nondeterministic) input" } },
    BuiltClassEntry { name: "add_days", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "add_hours", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "date_parts", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "format_date", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "days_between", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "days_in_month", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "is_leap_year", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "weekday_name", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "graph_query", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the global memory graph — state input" } },
    BuiltClassEntry { name: "graph_path", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the global memory graph" } },
    BuiltClassEntry { name: "graph_neighbors", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the global memory graph" } },
    BuiltClassEntry { name: "memory_decay", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "adjusts memory weights — undoable state change" } },
    BuiltClassEntry { name: "memory_boost", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "adjusts memory weights — undoable state change" } },
    BuiltClassEntry { name: "memory_prune", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destructively removes memory entries — no undo" } },
    BuiltClassEntry { name: "memory_revise", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "revises memory entries — undoable state change" } },
    BuiltClassEntry { name: "subgraph_extract", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "extracts a subgraph value from global graph state" } },
    BuiltClassEntry { name: "subgraph_nodes", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "subgraph_json", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "trace_start", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "mutates trace state" } },
    BuiltClassEntry { name: "trace_end", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "mutates trace state" } },
    BuiltClassEntry { name: "memory_score", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "mtree_summarize", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads memory-tree state" } },
    BuiltClassEntry { name: "mtree_retrieve", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads memory-tree state" } },
    BuiltClassEntry { name: "mtree_store", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists to the memory tree" } },
    BuiltClassEntry { name: "mtree_stats", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads memory-tree state" } },
    BuiltClassEntry { name: "mtree_forget", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destructively forgets memory-tree entries" } },
    BuiltClassEntry { name: "cron_mark_fired", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "mutates schedule firing state" } },
    BuiltClassEntry { name: "cron_add", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a schedule entry (undoable by cron_remove)" } },
    BuiltClassEntry { name: "cron_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads schedule state" } },
    BuiltClassEntry { name: "cron_remove", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "removes a schedule entry — destructive" } },
    BuiltClassEntry { name: "cron_run", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "fires scheduled flows — downstream external effects" } },
    BuiltClassEntry { name: "event_count", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "VM-native event-log read — state input" } },
    BuiltClassEntry { name: "events_since", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "VM-native event-log read — state input" } },
    BuiltClassEntry { name: "event_sum", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "VM-native event-log aggregation — state input" } },
    BuiltClassEntry { name: "query_scalar", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "VM-native DB read — state input" } },
    BuiltClassEntry { name: "query_row", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "VM-native DB read — state input" } },
    BuiltClassEntry { name: "assert_eq", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "assert_contains", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "confidence", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "toon_encode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "toon_decode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "recipe_save", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a recipe" } },
    BuiltClassEntry { name: "recipe_search", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads recipe state" } },
    BuiltClassEntry { name: "recipe_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads recipe state" } },
    BuiltClassEntry { name: "dag_phases", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "topo_sort", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "resolve_skill_index", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "VM-native skill resolver — state read" } },
    BuiltClassEntry { name: "fit_to_budget", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "map", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "fuzzy_find_best", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "hashline_read", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "hashline_edit", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "compact_list", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "budget_check", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "replay_snapshot", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads runtime snapshot state" } },
    BuiltClassEntry { name: "policy_check", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads runtime policy state" } },
    BuiltClassEntry { name: "semantic_search", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the semantic vault (KNN state input)" } },
    BuiltClassEntry { name: "config_load", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests a config file from disk" } },
    BuiltClassEntry { name: "vault_validate", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "todo_add", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a todo entry" } },
    BuiltClassEntry { name: "todo_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads todo state" } },
    BuiltClassEntry { name: "todo_update", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "updates todo state — undoable" } },
    BuiltClassEntry { name: "goal_get", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads goal state" } },
    BuiltClassEntry { name: "goal_set", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists goal state" } },
    BuiltClassEntry { name: "goals_add", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists goal state" } },
    BuiltClassEntry { name: "goals_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads goal state" } },
    BuiltClassEntry { name: "remind", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "schedules a future external delivery (the reminder itself is undoable)" } },
    BuiltClassEntry { name: "get_profile", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads persisted profile (PII state input)" } },
    BuiltClassEntry { name: "human_mood", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the persisted human-state model" } },
    BuiltClassEntry { name: "ask_approval", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "sends an approval request to the human — external interaction" } },
    BuiltClassEntry { name: "goal_complete", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "updates goal state" } },
    BuiltClassEntry { name: "goals_reflect", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "updates goal state" } },
    BuiltClassEntry { name: "cancel_remind", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "cancels a scheduled reminder" } },
    BuiltClassEntry { name: "check_reminders", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads reminder state" } },
    BuiltClassEntry { name: "list_reminders", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads reminder state" } },
    BuiltClassEntry { name: "remind_recurring", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "schedules recurring future deliveries" } },
    BuiltClassEntry { name: "human_create", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a human profile (PII)" } },
    BuiltClassEntry { name: "human_delete", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destroys a human profile — no undo" } },
    BuiltClassEntry { name: "human_forget", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destructively forgets human data (GDPR erasure semantics) — no undo" } },
    BuiltClassEntry { name: "human_personas", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads persisted personas" } },
    BuiltClassEntry { name: "human_recall", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads persisted human data" } },
    BuiltClassEntry { name: "human_remember", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists human data (PII)" } },
    BuiltClassEntry { name: "human_respond", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "delivers a response to the human — cannot be unsent" } },
    BuiltClassEntry { name: "compress_html", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "estimate_tokens", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "extract_entities", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "extract_param", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "learn_preference", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a learned preference (PII)" } },
    BuiltClassEntry { name: "read_file_tokens", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests file content (token-budgeted)" } },
    BuiltClassEntry { name: "squeeze", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "to_int", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_classify", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_to_markdown", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_extract_regions", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_ocr", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_create", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_add_page", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_write_text", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_draw_line", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_draw_rect", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_save", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_merge", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_split", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_metadata", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_set_metadata", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "html_to_pdf", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "send_document", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "delivers a document externally — cannot be unsent" } },
    BuiltClassEntry { name: "sha256", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "one-way digest — de-identifies its input (used to hash secrets)" } },
    BuiltClassEntry { name: "hmac_sha256", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "keyed digest — de-identifies its input" } },
    BuiltClassEntry { name: "hex_encode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "hex_decode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "secret", class: BuiltClass { role: Role::Source, default_label: Label::Secret, reversibility: Reversibility::Pure, rationale: "materializes a Secret value — Secret taint (№172)" } },
    BuiltClassEntry { name: "regex_match", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "regex_captures", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "regex_replace", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_draw_table", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_add_image", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_set_page_header", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_set_page_footer", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_page_numbers", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_watermark", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_fill_form", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_rotate_page", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_delete_pages", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "pdf_extract_images", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "smtp_send", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "sends an email externally — cannot be unsent" } },
    BuiltClassEntry { name: "smtp_send_html", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "sends an HTML email externally — cannot be unsent" } },
    BuiltClassEntry { name: "imap_list", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external mailbox listing" } },
    BuiltClassEntry { name: "imap_read", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external email content" } },
    BuiltClassEntry { name: "imap_search", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external mailbox search results" } },
    BuiltClassEntry { name: "imap_mark_read", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "mutates external mailbox flags (undoable by flag change)" } },
    BuiltClassEntry { name: "imap_move", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "moves an external email between folders (undoable by moving back)" } },
    BuiltClassEntry { name: "cal_connect", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external calendar connection state" } },
    BuiltClassEntry { name: "cal_list", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external calendar listings" } },
    BuiltClassEntry { name: "cal_events", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external calendar events" } },
    BuiltClassEntry { name: "cal_read", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests an external calendar event" } },
    BuiltClassEntry { name: "cal_create", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "creates an external calendar event — external state change" } },
    BuiltClassEntry { name: "cal_update", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "updates an external calendar event (undoable by update)" } },
    BuiltClassEntry { name: "cal_delete", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "deletes an external calendar event — external state change" } },
    BuiltClassEntry { name: "cal_freebusy", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external free/busy data" } },
    BuiltClassEntry { name: "ical_parse", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "ical_generate", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "card_connect", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external CardDAV connection state" } },
    BuiltClassEntry { name: "card_list", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external address-book listings" } },
    BuiltClassEntry { name: "card_contacts", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external contacts (PII ingress)" } },
    BuiltClassEntry { name: "card_read", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests an external contact (PII ingress)" } },
    BuiltClassEntry { name: "card_create", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "creates an external contact — external state change" } },
    BuiltClassEntry { name: "card_update", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "updates an external contact (undoable by update)" } },
    BuiltClassEntry { name: "card_delete", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Irreversible, rationale: "deletes an external contact — external state change" } },
    BuiltClassEntry { name: "card_search", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests external contact search results" } },
    BuiltClassEntry { name: "vcard_parse", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "vcard_generate", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_rect", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_circle", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_line", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_text", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_path", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_group", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_canvas", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_style", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_sketchy_filter", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_icon", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_callout", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_bar", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_donut", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_line", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_scatter", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_area", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_radar", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_heatmap", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "chart_boxplot", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "color_palette", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_generate", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "svg_canvas_preset", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_tree", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_org_chart", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_flowchart", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_layers", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_sequence", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_timeline", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_gantt", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_process", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_loop", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_venn", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_quadrant", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_pyramid", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_nested", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_medallion", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_er", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_state", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_swimlane", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_data_flow", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_high_level", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "diagram_architecture", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "template_render", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "auto-escaped template rendering — output is public-safe" } },
    BuiltClassEntry { name: "html_render", class: BuiltClass { role: Role::Lift, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "sanitizing HTML render — output is public-safe" } },
    BuiltClassEntry { name: "infographic_qa", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_train", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists trained weights in the Reflex registry (ADR-0114)" } },
    BuiltClassEntry { name: "reflex_predict", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_save", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists weights to SQLite (ADR-0116)" } },
    BuiltClassEntry { name: "reflex_load", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests persisted weights from SQLite" } },
    BuiltClassEntry { name: "reflex_metrics", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the Reflex registry state" } },
    BuiltClassEntry { name: "reflex_generate", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests untrusted model output (LlmOutput-equivalent per №201)" } },
    BuiltClassEntry { name: "reflex_tokenize", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_detokenize", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_bpe_train", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_bpe_encode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_bpe_decode", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "reflex_bpe_save", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists the BPE vocab (№195)" } },
    BuiltClassEntry { name: "reflex_bpe_load", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests a persisted BPE vocab" } },
    BuiltClassEntry { name: "vision_generate", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a generated artifact in the VisionRegistry (№210) — local compute, egress only at vision_export" } },
    BuiltClassEntry { name: "vision_edit", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists an edited artifact in the VisionRegistry" } },
    BuiltClassEntry { name: "vision_export", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "writes the signed image artifact to disk — egress point (gate VISION_UNSIGNED_EXPORT, ADR-0125)" } },
    BuiltClassEntry { name: "vision_export_raw", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "explicit unsigned opt-out (ADR-0125); №320/ADR-0152: raw egress of synthetic or manifest-less artifacts is REFUSED — EU AI Act Art. 50 marking (static gate MEDIA_SYNTHETIC_UNMARKED + runtime backstop); legal only for synthetic: false" } },
    BuiltClassEntry { name: "vision_fetch_weights", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "ingests external weights (allowlist+SSRF+SHA-pinned, №300); writes the local weight cache" } },
    BuiltClassEntry { name: "vision_list", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads the VisionRegistry state" } },
    BuiltClassEntry { name: "vision_save", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a Vision artifact (№242)" } },
    BuiltClassEntry { name: "vision_load", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests a persisted Vision artifact" } },
    BuiltClassEntry { name: "vision_lora_load", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests a persisted LoRA adapter" } },
    BuiltClassEntry { name: "vision_lora_generate", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a LoRA-generated artifact in the VisionRegistry" } },
    BuiltClassEntry { name: "canary_insert", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "plants canary markers into channels — security-instrumentation state write (№284)" } },
    BuiltClassEntry { name: "canary_check", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads canary leak-detection state (№284)" } },
    BuiltClassEntry { name: "json_validate", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "memory_forget", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Irreversible, rationale: "destructively forgets memory (№280) — no undo" } },
    BuiltClassEntry { name: "user_profile", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "reads persisted user profile (PII state input)" } },
    BuiltClassEntry { name: "text_chunk", class: BuiltClass { role: Role::Pure, default_label: Label::Public, reversibility: Reversibility::Pure, rationale: "" } },
    BuiltClassEntry { name: "llm_stream_open", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "opens an external SSE stream — ingests untrusted model output; DUAL: prompt egress" } },
    BuiltClassEntry { name: "llm_stream_next", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Pure, rationale: "ingests the next untrusted model chunk from the external stream" } },
    BuiltClassEntry { name: "llm_stream_close", class: BuiltClass { role: Role::Sink, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "closes the external stream (cleanup effect, no data egress)" } },
    BuiltClassEntry { name: "server_path_param", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests untrusted request path parameter — UserInput taint" } },
    BuiltClassEntry { name: "voice_enroll", class: BuiltClass { role: Role::Sink, default_label: Label::Secret, reversibility: Reversibility::Reversible, rationale: "persists a BIOMETRIC voiceprint (GDPR Art. 9 — Secret label, encrypted at rest per ADR-0145 D4)" } },
    BuiltClassEntry { name: "tts_speak", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a locally synthesized audio artifact (ADR-0143)" } },
    BuiltClassEntry { name: "audio_export", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "writes the signed audio artifact to disk (gate AUDIO_UNSIGNED_EXPORT, ADR-0145)" } },
    BuiltClassEntry { name: "voice_design", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a designed voice artifact" } },
    BuiltClassEntry { name: "voice_save", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a Voice artifact" } },
    BuiltClassEntry { name: "voice_load", class: BuiltClass { role: Role::Source, default_label: Label::Internal, reversibility: Reversibility::Pure, rationale: "ingests a persisted Voice artifact" } },
    BuiltClassEntry { name: "video_render", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists a generated video artifact in VIDEO_REGISTRY — local tiny pipeline; egress only at video_export" } },
    BuiltClassEntry { name: "video_export", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "writes the signed .mlgv container to disk — egress point (gate VIDEO_UNSIGNED_EXPORT, ADR-0151 D5)" } },
    BuiltClassEntry { name: "av_mux", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists the A/V sidecar container in VIDEO_REGISTRY (ADR-0151 D4)" } },
    BuiltClassEntry { name: "frame_interp", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists an interpolated artifact in VIDEO_REGISTRY (ADR-0151 D2)" } },
    BuiltClassEntry { name: "video_extend", class: BuiltClass { role: Role::Sink, default_label: Label::Internal, reversibility: Reversibility::Reversible, rationale: "persists an extended artifact in VIDEO_REGISTRY (ADR-0151 D3)" } },
    BuiltClassEntry { name: "video_fetch_weights", class: BuiltClass { role: Role::Source, default_label: Label::Network, reversibility: Reversibility::Reversible, rationale: "intended external weights fetch (formal No-Go №294 class, ADR-0151 D7); covered by MODEL_WEIGHTS_UNSAFE" } },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Source-level registry names — NOT the cfg-filtered compiled registry:
    /// the classification map is complete at the source level, while
    /// BUILTIN_REGISTRY at runtime is feature-gated (video/voice/vec... are
    /// cfg'd out under default features). include_str! keeps the tests
    /// feature-independent (№316 coverage is a source-level contract).
    fn registry_source_names() -> HashSet<String> {
        let src = include_str!("builtins/registry.rs");
        let mut out = HashSet::new();
        let mut rest = src;
        while let Some(i) = rest.find("spec!(\"") {
            let after = &rest[i + 7..];
            if let Some(j) = after.find('"') {
                let line_start = rest[..i].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_end = rest[i..].find('\n').map(|p| i + p).unwrap_or(rest.len());
                let line = &rest[line_start..line_end];
                let code = line.split("//").next().unwrap_or(line);
                if code.contains("spec!(") {
                    out.insert(after[..j].to_string());
                }
            }
            rest = &rest[i + 7..];
        }
        out
    }

    fn registry_names() -> HashSet<String> {
        registry_source_names()
    }

    /// №316 «Сделано, когда» (а): every registered builtin is classified.
    #[test]
    fn coverage_every_registered_builtin_is_classified() {
        let map: HashSet<&str> = BUILTIN_CLASSES.iter().map(|e| e.name).collect();
        let missing: Vec<String> = registry_names()
            .into_iter()
            .filter(|n| !map.contains(n.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "builtins without classification ({}): {:?}",
            missing.len(),
            missing
        );
    }

    /// №316 «Сделано, когда» (а): the map has no extra names.
    #[test]
    fn no_extra_names() {
        let registry = registry_names();
        let extras: Vec<&str> = BUILTIN_CLASSES
            .iter()
            .map(|e| e.name)
            .filter(|n| !registry.contains(*n))
            .collect();
        assert!(
            extras.is_empty(),
            "classified names not in registry: {:?}",
            extras
        );
    }

    /// №316 «Сделано, когда» (а): rationale on every non-Pure entry.
    #[test]
    fn rationale_on_every_non_pure() {
        let bad: Vec<&str> = BUILTIN_CLASSES
            .iter()
            .filter(|e| e.class.role != Role::Pure && e.class.rationale.trim().is_empty())
            .map(|e| e.name)
            .collect();
        assert!(bad.is_empty(), "non-Pure without rationale: {:?}", bad);
    }

    /// №316: uniqueness of the map (one class per name).
    #[test]
    fn map_is_unique() {
        let mut seen = HashSet::new();
        let dupes: Vec<&str> = BUILTIN_CLASSES
            .iter()
            .map(|e| e.name)
            .filter(|n| !seen.insert(*n))
            .collect();
        assert!(dupes.is_empty(), "duplicate classifications: {:?}", dupes);
    }

    /// №316: the named minimum classes from the issue are present.
    #[test]
    fn issue_minimum_classes() {
        let expect_sink = [
            "http_post",
            "write_file",
            "send_message",
            "print",
            "db_execute",
            "tts_send",
        ];
        for n in expect_sink {
            let c = classify(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!(c.role, Role::Sink, "{} must be a Sink", n);
        }
        let expect_source = ["http_get", "env", "json_body", "form_data", "query_param"];
        for n in expect_source {
            let c = classify(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!(c.role, Role::Source, "{} must be a Source", n);
        }
        for n in ["exec", "git_push", "delete_file"] {
            let c = classify(n).unwrap_or_else(|| panic!("{}", n));
            assert_eq!(c.reversibility, Reversibility::Irreversible, "{}", n);
        }
        let redact = classify("redact").unwrap();
        assert_eq!(
            redact.role,
            Role::Lift,
            "redact — taint-sanitizer lift (ADR-0136)"
        );
    }

    /// №316: Sink/Source/Lift/Pure distribution is sane (sanity counts,
    /// guards against a silent default Pure swallowing the risky surface).
    #[test]
    fn distribution_sanity() {
        let n = |r: Role| BUILTIN_CLASSES.iter().filter(|e| e.class.role == r).count();
        assert!(n(Role::Sink) >= 60, "sinks: {}", n(Role::Sink));
        assert!(n(Role::Source) >= 40, "sources: {}", n(Role::Source));
        assert!(n(Role::Lift) >= 5, "lifts: {}", n(Role::Lift));
        assert!(n(Role::Pure) >= 200, "pure: {}", n(Role::Pure));
    }

    /// №316 «Сделано, когда» (б): the REFERENCE.md classification block is
    /// regenerated from THIS map and matches it exactly (doc cannot drift).
    #[test]
    fn reference_classification_block_matches_map() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let path = std::path::Path::new(&manifest).join("REFERENCE.md");
        let reference = std::fs::read_to_string(&path).unwrap_or_default();
        let begin = "BEGIN GENERATED BUILTIN CLASSIFICATION";
        let end = "END GENERATED BUILTIN CLASSIFICATION";
        let (block, found) = match (reference.find(begin), reference.find(end)) {
            (Some(b), Some(e)) if b < e => (&reference[b..e], true),
            _ => ("", false),
        };
        assert!(
            found,
            "REFERENCE.md must contain the classification block markers"
        );

        let mut expected =
            String::from("| Builtin | Role | Default label | Reversibility |\n|---|---|---|---|\n");
        for e in BUILTIN_CLASSES {
            let role = e.class.role.as_str();
            let label = e.class.default_label.as_str();
            let rev = e.class.reversibility.as_str();
            if e.class.rationale.is_empty() {
                expected.push_str(&format!(
                    "| `{}` | {} | {} | {} | — |\n",
                    e.name, role, label, rev
                ));
            } else {
                expected.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} |\n",
                    e.name, role, label, rev, e.class.rationale
                ));
            }
        }

        let block_rows: Vec<String> = block
            .lines()
            .filter(|l| l.starts_with("| `"))
            .map(|l| l.trim_end().to_string())
            .collect();
        let expected_rows: Vec<String> = expected
            .lines()
            .skip(2)
            .map(|l| l.trim_end().to_string())
            .collect();
        assert_eq!(
            block_rows.len(),
            expected_rows.len(),
            "REFERENCE block row count {} != map count {}",
            block_rows.len(),
            expected_rows.len()
        );
        for (got, want) in block_rows.iter().zip(expected_rows.iter()) {
            assert_eq!(got, want, "REFERENCE block row drift");
        }
    }
}
