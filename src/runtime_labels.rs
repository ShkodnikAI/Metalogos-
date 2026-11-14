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
        // №455 sensitive-path rule with the REAL argument value.
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
        _ => Label::bottom(),
    }
}

// ── The KV label sidecar (the №952 kv round-trip contract) ───────────
//
// `kv_set(key, value)` records the value's runtime label under the key;
// `kv_get(key)` returns the stored label joined over the class floor.
// The store is process-wide (the same static KV_STORE the handlers use),
// so the label survives the round-trip on BOTH backends and across
// pattern frames — the «kv/mem/сессии в пределах процесса сохраняют»
// clause of ADR-0156 §5.

fn kv_label_table() -> &'static Mutex<HashMap<String, Label>> {
    static TABLE: OnceLock<Mutex<HashMap<String, Label>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record the label a value carried INTO the kv store (kv_set, the
/// dispatch wrapper calls this with the computed argument label).
pub fn kv_label_record(key: &str, label: &Label) {
    if let Ok(mut t) = kv_label_table().lock() {
        t.insert(key.to_string(), label.clone());
    }
}

fn kv_label_lookup_from_args(args: &[crate::interpreter::values::Value]) -> Label {
    let key = args.first().and_then(|v| match v {
        crate::interpreter::values::Value::String(s) => Some(s.clone()),
        _ => None,
    });
    match key {
        Some(k) => kv_label_table()
            .lock()
            .ok()
            .and_then(|t| t.get(&k).cloned())
            .unwrap_or_else(Label::bottom),
        None => Label::bottom(),
    }
}

/// The full label of a `kv_get` result: the stored label joined over the
/// classification floor (bottom today — the №951 owner decision).
pub fn kv_get_return_label(args: &[crate::interpreter::values::Value]) -> Label {
    kv_label_lookup_from_args(args)
}

// ── The lift policy (the runtime mirror of the static sanitizer rule) ──
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
pub fn gate_labels_only(fn_name: &str, arg_labels: &[Label]) -> Result<(), String> {
    if !is_gated_dispatch(fn_name) {
        return Ok(());
    }
    for (i, label) in arg_labels.iter().enumerate() {
        runtime_sink_check(fn_name, i, label, 0)?;
    }
    Ok(())
}

pub fn runtime_sink_check(
    fn_name: &str,
    arg_index: usize,
    label: &Label,
    line: u32,
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
        eprintln!("[SINK_CLEARANCE][audit-event] {}", message);
        return Err(message);
    }
    Ok(())
}
