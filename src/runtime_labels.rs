// ── №952 (the audit 12.11, P0): runtime VALUE labels — the second line ──
//
//! The №328 runtime label machinery tracked labels per NAME in a
//! compiler-guided env (`LabelJoin`/`SinkCheck` instructions): where the
//! static lattice was blind, the runtime was blind too — every audit probe
//! of №950/№951 ran to completion on the published v0.39.0. This module is
//! the №952 second line: the label rides the VALUE at runtime on BOTH
//! backends (the tree-walking interpreter and the bytecode VM), seeded by
//! the SSOT classification, propagated by joins at the binding/dispatch
//! sites, and checked by the sink dispatcher with THE SAME clearance rule
//! the static gate applies (`semantic::clearance_failure` — one rule, two
//! lanes, so "the static gate and the runtime agree on the verdict" is a
//! fact, not a slogan).
//!
//! Contract (ADR-0156 §5, the №949 amendment; naryad №952 / gh#1919):
//! - sources (`env`, `secret`, `call_llm`, `http_get`, `query_param`,
//!   `read_file`, … — the SSOT classification) mark the value;
//! - operations and builtins propagate the join (per the classification);
//! - sanitizers strip per the SAME policy the static lane reads
//!   (`escape_html`/`ct_eq` restore the INTEGRITY only — the
//!   confidentiality is the join of the arguments, the A2 rule);
//! - pattern calls, returns, collections, struct fields, `kv`/`mem`/
//!   sessions within the process preserve the label;
//! - a sink checks the value's label and refuses
//!   `[SINK_CLEARANCE_RUNTIME]` with an audit event.
//!
//! The static twin can be silenced with `METALOGOS_STATIC_LABEL_GATE_OFF=1`
//! (a TEST-ONLY flag, loud in the docs): the runtime line alone must then
//! catch every №950/№951 probe on both backends.

use crate::labels::{Conf, Integrity, Label};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// The TEST-ONLY static-gate silencer (the №952 "static off" test).
/// `METALOGOS_STATIC_LABEL_GATE_OFF=1` makes `mlog check` skip the
/// sink-clearance findings so a test can prove the RUNTIME line catches
/// the flow alone. Documented as test-only; production runs never set it.
pub fn static_labels_disabled() -> bool {
    std::env::var("METALOGOS_STATIC_LABEL_GATE_OFF").as_deref() == Ok("1")
}

// ── The source→label mapping (the runtime mirror of `semantic::label_source`) ──
//
// The static lane names its sources by hand in `label_source`; this
// mirror maps the SAME families to the SAME labels so the two lanes
// agree wherever static knows the flow. The runtime adds the one thing
// static cannot have: the read of the ACTUAL argument values (the №455
// sensitive-path rule reads the real path, not the literal guess).

fn kind_label(kind: &str) -> Label {
    crate::labels::legacy_taint_label(kind).unwrap_or_else(Label::bottom)
}

fn private_trusted() -> Label {
    Label {
        conf: Conf::Private,
        integrity: Integrity::Trusted,
        consent: Default::default(),
    }
}

/// The runtime seed label of a source builtin's RETURN value.
/// `args` are the ALREADY-EVALUATED argument values (the runtime can
/// read the actual data where static had to guess from literals).
pub fn source_return_label(fn_name: &str, args: &[crate::interpreter::values::Value]) -> Label {
    // The runtime mirror of `semantic::label_source` — ONLY the hand-named
    // families carry a floor; every other Source-classified builtin
    // (handle constructors, device factories, …) returns BOTTOM, exactly
    // like the static lane (an unnamed source is not a taint source —
    // the classification role alone never raises the label).
    match fn_name {
        // Secret sources (ADR-0154 §5): private/trusted.
        "env" | "secret" => private_trusted(),
        // LLM-output sources (model output is untrusted — ADR-0117).
        "call_llm" | "call_claude" | "call_llm_schema" | "reflex_generate" => {
            kind_label("UserInput")
        }
        // Untrusted-input sources (№268/№N2/№N5/№N6 — the label_source
        // table verbatim).
        "form_data" | "json_body" | "query_param" | "request_header" | "mcp_call"
        | "html_select" | "html_text" | "html_links" | "feed_parse" | "sitemap_parse"
        | "xml_parse" => kind_label("UserInput"),
        // №325: network/file ingress — untrusted; read_file keeps the
        // №455 sensitive-path rule with the REAL argument value, and
        // since №982 consults the file stream of the store-label
        // sidecar FIRST: a recorded write of THIS path rides the
        // read-back (the stored label — a trusted own-config write
        // reads back trusted); no record = the №325 ingress default
        // (the file outlives the process — someone outside wrote it).
        "http_get" => kind_label("UserInput"),
        "read_file" | "read_file_tokens" => {
            let sensitive = args.first().and_then(|v| match v {
                crate::interpreter::values::Value::String(path) => {
                    Some(crate::builtins::io::sensitive_path_match(path))
                }
                _ => None,
            });
            if sensitive.unwrap_or(false) {
                private_trusted()
            } else if let Some(path) = store_key_from_args(args) {
                store_get_return_label("file", &path)
            } else {
                kind_label("UserInput")
            }
        }
        "kv_get" => {
            // №952: the KV round-trip preserves the label the value
            // carried INTO the store (the sidecar) — the classification
            // floor stays bottom (the №951 owner decision).
            kv_label_lookup_from_args(args)
        }
        // №982: the store axis of the sidecar — the recorded label
        // rides the read-back; no record → `bottom` (the №951 owner
        // decision: an unwritten key is not untrusted data). The
        // stream tags keep mem, session and kv from sharing labels
        // across their store boundaries. THE SIGNATURES DIFFER:
        // mem_get(key) takes the key at #0, session_get(sid, key) at
        // #1 — the per-name index is part of the arm.
        "mem_get" | "mem_get_or" => match store_key_from_args(args) {
            Some(k) => store_get_return_label("mem", &k),
            None => Label::bottom(),
        },
        "session_get" | "session_get_or" => match args.get(1).and_then(|v| match v {
            crate::interpreter::values::Value::String(s) => Some(s.clone()),
            _ => None,
        }) {
            Some(k) => store_get_return_label("session", &k),
            None => Label::bottom(),
        },
        _ => Label::bottom(),
    }
}

// ── The STORE label sidecar (№952's kv contract, generalized by №982) ─
//
// The owner's 19.11 decision (issue #1996): the sidecar is the ONE
// mechanism for label persistence through storage — no write gate. One
// table, stream-tagged keys, the same record point in every runtime
// lane, the same lookup point in `source_return_label`. The miss
// default is PER STREAM: the kv/mem/session stores answer `bottom` (the
// №951 owner decision — an unwritten key is not untrusted data), the
// file stream answers `public, untrusted` (the sidecar is in-process;
// a file nobody in THIS process wrote was written by someone outside —
// that is the definition of untrusted, and the same default the №325
// ingress rule already applies). The kv stream keeps its historical
// bare keys (its records predate the streams).
//
// THE MEASURED BOUNDARY (named in the PR, not hidden): the sidecar lives
// in the process — a write in one run and a read in another share
// nothing. The inter-process form stays open, recorded as the
// threat-model residual (№982).

fn kv_label_table() -> &'static Mutex<HashMap<String, Label>> {
    static KV_LABELS: OnceLock<Mutex<HashMap<String, Label>>> = OnceLock::new();
    KV_LABELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record the label a value carried INTO the store (the dispatch
/// wrappers call this with the computed argument label; the empty
/// stream tag is the kv stream's historical bare-key form).
pub fn store_label_record(stream: &str, key: &str, label: &Label) {
    let tagged = if stream.is_empty() {
        key.to_string()
    } else {
        format!("{stream}:{key}")
    };
    if let Ok(mut t) = kv_label_table().lock() {
        t.insert(tagged, label.clone());
    }
}

/// The full label of a store read-back: the recorded label if this
/// process wrote the value, else the stream's default — `bottom` for
/// the stores, `public, untrusted` for the file stream.
pub fn store_get_return_label(stream: &str, key: &str) -> Label {
    let tagged = if stream.is_empty() {
        key.to_string()
    } else {
        format!("{stream}:{key}")
    };
    match kv_label_table()
        .lock()
        .ok()
        .and_then(|t| t.get(&tagged).cloned())
    {
        Some(l) => l,
        None if stream == "file" => kind_label("UserInput"),
        None => Label::bottom(),
    }
}

fn store_key_from_args(args: &[crate::interpreter::values::Value]) -> Option<String> {
    args.first().and_then(|v| match v {
        crate::interpreter::values::Value::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// №982: record the label a value carried INTO a store (kv/mem/session)
/// or a file (write_file/append_file). The dispatch gates call this with
/// the evaluated values (the keys) and the walk's labels (the stored
/// labels) — the one point where BOTH halves exist. THE SIGNATURES ARE
/// PART OF THE TABLE: kv_set(key, value) — the key at #0;
/// mem_set(key, value) — #0; session_set(sid, key, value) — the KEY at
/// #1, the VALUE at #2; write_file/append_file(path, content) — the
/// path at #0. Recording only — NO WRITE GATE (the owner's 19.11
/// decision, issue #1996): the refusal behavior lives in the clearance
/// check, unchanged.
pub fn record_store_write(
    name: &str,
    values: &[crate::interpreter::values::Value],
    labels: &[Label],
) {
    let (stream, key_idx, val_idx): (&str, usize, usize) = if name == crate::recipe_ops::NAME_KV_SET
    {
        ("", 0, 1)
    } else {
        match name {
            "mem_set" => ("mem", 0, 1),
            "session_set" => ("session", 1, 2),
            "write_file" | "append_file" => ("file", 0, 1),
            _ => return,
        }
    };
    if let (Some(crate::interpreter::values::Value::String(key)), Some(label)) =
        (values.get(key_idx), labels.get(val_idx))
    {
        store_label_record(stream, key, label);
    }
}

/// №982: the recorded-label lookup for a store read — `Some` ONLY when
/// THIS process recorded a label for the key (the lift: a trusted
/// own-config write reads back trusted); `None` on a miss — the caller
/// keeps its conservative default (the static source table on the TW
/// lane, the per-stream defaults in `source_return_label` on the VM
/// lane). The №455 sensitive-path verdict is NEVER lifted: a recorded
/// label cannot lower the confidentiality a sensitive path carries.
pub fn store_lookup_recorded(name: &str, args: &[crate::ast::Expr]) -> Option<Label> {
    // The walk's arguments are EXPRESSIONS — the runtime key exists only
    // for a LITERAL argument; a computed key keeps the conservative
    // static default (which matches the VM's miss verdict exactly, so
    // the two lanes never disagree on the class).
    let literal = |i: usize| -> Option<String> {
        args.get(i).and_then(|a| match a {
            crate::ast::Expr::StringLit { value, .. } => Some(value.clone()),
            _ => None,
        })
    };
    let (stream, key): (&str, String) = match name {
        "kv_get" => ("", literal(0)?),
        "mem_get" | "mem_get_or" => ("mem", literal(0)?),
        "session_get" | "session_get_or" => ("session", literal(1)?),
        "read_file" | "read_file_tokens" => {
            let path = literal(0)?;
            if crate::builtins::io::sensitive_path_match(&path) {
                return None;
            }
            ("file", path)
        }
        _ => return None,
    };
    let tagged = if stream.is_empty() {
        key
    } else {
        format!("{stream}:{key}")
    };
    kv_label_table()
        .lock()
        .ok()
        .and_then(|t| t.get(&tagged).cloned())
}

/// Record the label a value carried INTO the kv store (kv_set, the
/// dispatch wrapper calls this with the computed argument label).
pub fn kv_label_record(key: &str, label: &Label) {
    store_label_record("", key, label);
}

fn kv_label_lookup_from_args(args: &[crate::interpreter::values::Value]) -> Label {
    match store_key_from_args(args) {
        Some(k) => store_get_return_label("", &k),
        None => Label::bottom(),
    }
}

/// The full label of a `kv_get` result: the stored label joined over the
/// classification floor (bottom today — the №951 owner decision).
pub fn kv_get_return_label(args: &[crate::interpreter::values::Value]) -> Label {
    kv_label_lookup_from_args(args)
}

// ── The lift policy (the runtime twin of the static sanitizer rule) ──
//
// A2 (naryad 781): escape_html restores the INTEGRITY only — the
// CONFIDENTIALITY is the join of the arguments (an escaped secret is
// still a secret). ct_eq: the same shape (the verdict is validation
// data, but derived from the compared material). render: NOT a sanitizer
// since №765 (raw slots conduct verbatim) — it propagates the join like
// a pure op. redact: the REDACT_POLICIES table reads target_conf
// («public» restores the public clearance, «private» keeps the value
// private-by-design) — the static lane reads the same table.

pub fn lift_return_label(fn_name: &str, arg_labels: &[Label]) -> Label {
    let joined = arg_labels
        .iter()
        .fold(Label::bottom(), |acc, l| acc.join(l));
    match fn_name {
        "escape_html" | "escape_json" | "escape_js" | "ct_eq" => Label {
            conf: joined.conf,
            integrity: Integrity::Trusted,
            consent: joined.consent,
        },
        "redact" => {
            // The redact ENGINE decides the target conf; the runtime
            // mirror reads the same policy word the handler applied
            // (arg 1). Unknown/absent → the conservative join (no strip).
            // The policy word is supplied by the caller via
            // `redact_return_label` — here the fallback keeps the join.
            joined
        }
        _ => joined,
    }
}

/// `redact(value, policy)` — the result's conf is the policy's
/// target_conf (the REDACT_POLICIES row the handler actually applied).
pub fn redact_return_label(policy_arg: Option<&str>, arg_labels: &[Label]) -> Label {
    let joined = arg_labels
        .iter()
        .fold(Label::bottom(), |acc, l| acc.join(l));
    let target = policy_arg
        .and_then(crate::builtins::string::redact_policy)
        .map(|p| p.target_conf)
        .unwrap_or("private");
    match target {
        "public" => Label {
            conf: Conf::Public,
            integrity: Integrity::Trusted,
            consent: Default::default(),
        },
        _ => Label {
            conf: joined.conf.max(Conf::Private),
            integrity: Integrity::Trusted,
            consent: joined.consent,
        },
    }
}

// ── The value-variant floor ──────────────────────────────────────────

/// A typed opaque value carries its own secrecy regardless of the walk:
/// `Secret` (Phase 6.4) is private/trusted; everything else starts at
/// `bottom`. WorldState (№355) is NOT floored here — the handle has
/// SANCTIONED consumers (proof_seal/bounds_attach, the sealing contour)
/// and its private-by-default materialization contract lives in the
/// per-builtin typed checks (WORLD_STATE_PRIVATE, №355/№691), not in a
/// blanket label.
pub(crate) fn value_variant_floor(v: &crate::interpreter::values::Value) -> Label {
    use crate::interpreter::values::Value;
    match v {
        Value::Secret(_) => Label {
            conf: Conf::Private,
            integrity: Integrity::Trusted,
            consent: Default::default(),
        },
        _ => Label::bottom(),
    }
}

// ── The runtime sink check (the ONE clearance rule, two lanes) ───────

/// The verdict rule is the STATIC one — `semantic::clearance_failure` —
/// called verbatim, so a runtime refusal and a static refusal of the same
/// flow agree on the reason class and the verdict. Returns Err with the
/// typed `[SINK_CLEARANCE_RUNTIME]` message when the sink refuses.
/// The №951 B6 family: the REQUEST side of a bidirectional builtin is
/// gated too (confidentiality only) — `http_get("?k=" + env(...))` is a
/// leak even though the builtin classifies as Source.
pub fn is_gated_dispatch(fn_name: &str) -> bool {
    if crate::semantic::REQUEST_EGRESS_NAMES.contains(&fn_name) {
        return true;
    }
    use crate::builtins_classification::{classify, Role};
    classify(fn_name)
        .map(|c| c.role == Role::Sink)
        .unwrap_or(false)
}

/// The pre-evaluation form of the sink gate: the walk's labels only (no
/// values yet). The intercepted dispatch paths (print/http_get/kv_*) call
/// this at the eval arm head — BEFORE the builtin executes.
pub fn gate_labels_only(fn_name: &str, arg_labels: &[Label], advisory: bool) -> Result<(), String> {
    if !is_gated_dispatch(fn_name) {
        return Ok(());
    }
    for (i, label) in arg_labels.iter().enumerate() {
        runtime_sink_check(fn_name, i, label, 0, advisory)?;
    }
    Ok(())
}

pub fn runtime_sink_check(
    fn_name: &str,
    arg_index: usize,
    label: &Label,
    line: u32,
    advisory: bool,
) -> Result<(), String> {
    if let Some(_reason) = crate::semantic::clearance_failure(fn_name, label) {
        let reason = crate::audit::sink_check_id(fn_name, arg_index, label);
        let message = crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_SINK_CLEARANCE_RUNTIME,
            format!(
                "sink clearance violated at runtime: {} argument #{} carries label '{}' (line {}) — the value-label second line (№952); deny reason class: {}",
                fn_name,
                arg_index,
                label,
                line,
                reason
            ),
        );
        // ADR-0161 §compatibility: `profile legacy { egress:
        // permissive_with_audit }` switches the gate to ADVISORY — the
        // audit event fires and the dispatch proceeds, exactly like the
        // static lane (audit.rs: the Severity::Info downgrade). The
        // posture is the same `profile::resolve(...).permissive()` the
        // static gate reads — the two lanes agree on the posture as well
        // as the verdict (the naryad's own «one rule, two lanes»).
        if advisory {
            eprintln!(
                "[SINK_CLEARANCE][audit-event] {} (audit event: profile legacy / egress permissive_with_audit)",
                message
            );
            return Ok(());
        }
        eprintln!("[SINK_CLEARANCE][audit-event] {}", message);
        return Err(message);
    }
    Ok(())
}
