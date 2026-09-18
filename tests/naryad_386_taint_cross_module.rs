//! Наряд №386 (issue #480) — cross-module persistence taint: memory-key
//! summaries in `PatternSummary` (№376 extension) + the fingerprint-keyed
//! module registry + the recall→sink flow check.
//!
//! Contract under test:
//! - a cross-module pair (module A: `memorize(<key>, call_llm(..))`;
//!   module B: `recall(<key>)` → `respond(..)`) is caught — the finding is
//!   the SAME `TAINT_PERSISTENCE` Category-A class, naming the matched key
//!   and the writer scope;
//! - the same pair with a sanitizer before the store (or with no LLM
//!   source at all) stays GREEN;
//! - the prefix heuristic: a recorded leading literal (`"pfx:" + suffix`)
//!   matches recall keys under that prefix;
//! - dynamic keys (no leading literal) are the documented, honest boundary;
//! - `METALOGOS_TAINT_STRICT=1` (default OFF) flags ANY recall→sink flow
//!   when another module writes LLM output to memory at all;
//! - the №376 summaries-cache contract is untouched (insert/hit counters),
//!   and the in-slice cross-pattern flow (merged run paths) is caught too.

use std::sync::Mutex;

/// The registry and the env flag are process-global: every test in this
/// file serializes on this lock and clears the registry first.
static TAINT_LOCK: Mutex<()> = Mutex::new(());

fn reset_taint_state() {
    metalogos::audit::memory_taint_registry_clear();
    metalogos::audit::summaries_cache_clear();
}

/// Error-severity check_ids of an `audit_program` run.
fn audit_error_classes(source: &str) -> Vec<String> {
    match metalogos::audit_program(source) {
        Ok(result) => result
            .findings
            .iter()
            .filter(|f| f.severity == metalogos::audit::Severity::Error)
            .map(|f| f.check_id.to_string())
            .collect(),
        Err(_) => vec!["PARSE".to_string()],
    }
}

/// Full findings (for message assertions).
fn audit_findings(source: &str) -> Vec<metalogos::audit::AuditFinding> {
    metalogos::audit_program(source)
        .expect("audit source must parse")
        .findings
}

const WRITER: &str = r#"
pattern StoreDraft(data: String) -> String {
  let _ = memorize("draft386:", call_llm(data))
  return "stored"
}
"#;

const READER: &str = r#"
pattern Handle(_tick: String) -> String {
  let y = recall("draft386:latest")
  let r = respond("200 OK", y)
  return r
}
"#;

// ── (а) the red/green cross-module pair ──────────────────────────────

#[test]
fn n386_cross_module_flow_is_caught_with_key_and_writer_named() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    // Module A: the write side is already a memory-sink refusal, and it
    // seeds the registry with the tainted key.
    let writer_classes = audit_error_classes(WRITER);
    assert!(
        writer_classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the unsanitized LLM→memory write must fail: {writer_classes:?}"
    );

    // Module B: never writes memory itself — the file-level heuristic has
    // nothing to see; the cross-module registry does.
    let reader_findings = audit_findings(READER);
    let cross = reader_findings
        .iter()
        .find(|f| {
            f.check_id == "TAINT_PERSISTENCE"
                && f.message.contains("cross-module taint through memory")
        })
        .expect("the cross-module recall→respond flow must be caught");
    assert!(
        cross.message.contains("draft386"),
        "the finding names the matched key: {}",
        cross.message
    );
    assert!(
        cross.message.contains("StoreDraft"),
        "the finding names the writer scope: {}",
        cross.message
    );
    assert_eq!(
        cross.severity,
        metalogos::audit::Severity::Error,
        "TAINT_PERSISTENCE stays Category A"
    );
}

#[test]
fn n386_green_unrelated_keys_pass() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    let _ = audit_error_classes(WRITER); // registers "draft386:"
    let unrelated = r#"
pattern Handle(_tick: String) -> String {
  let y = recall("totally-different-key")
  let r = respond("200 OK", y)
  return r
}
"#;
    let classes = audit_error_classes(unrelated);
    assert!(
        !classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "a non-matching recall key must stay green: {classes:?}"
    );
}

#[test]
fn n386_green_sanitize_before_store() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    // NOTE: the registry is NOT polluted here — the sanitized module must
    // stand on its own (its writer/reader pair lives in ONE module; the
    // store is sanitized so no tainted key is ever recorded).
    let sanitized = r#"
pattern StoreDraft(data: String) -> String {
  let _ = memorize("clean386", redact(call_llm(data), "hash_only"))
  return "stored"
}
pattern ReadClean(_tick: String) -> String {
  let y = recall("clean386")
  let r = respond("200 OK", y)
  return r
}
"#;
    // The store is sanitized → no tainted key recorded under that prefix;
    // the recall is clean even though the module shape looks similar.
    let findings = audit_findings(sanitized);
    assert!(
        !findings
            .iter()
            .any(|f| f.check_id == "TAINT_PERSISTENCE" && f.message.contains("cross-module")),
        "a sanitized store + clean recall must not produce a cross-module finding"
    );
    assert!(
        !findings.iter().any(|f| f.check_id == "TAINT_PERSISTENCE"),
        "no memory-sink refusal either — the store itself is sanitized"
    );
}

// ── prefix heuristic ─────────────────────────────────────────────────

#[test]
fn n386_prefix_keys_match_under_the_recorded_prefix() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    let writer = r#"
pattern Store(data: String) -> String {
  let _ = memorize("pfx386:" + data, call_llm(data))
  return "stored"
}
"#;
    let _ = audit_error_classes(writer);
    let reader = r#"
pattern Read(_tick: String) -> String {
  let y = recall("pfx386:tail")
  let r = respond("200 OK", y)
  return r
}
"#;
    let findings = audit_findings(reader);
    assert!(
        findings
            .iter()
            .any(|f| f.check_id == "TAINT_PERSISTENCE" && f.message.contains("pfx386")),
        "a recall under the recorded prefix must be caught: {findings:?}"
    );
}

// ── honest boundary: dynamic keys are not covered ────────────────────

#[test]
fn n386_dynamic_keys_are_the_documented_boundary() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    // The key has NO leading literal — the MVP cannot name it. The write
    // side still registers `any_writes` (strict-mode fuel), but the
    // non-strict default stays green for the reader. This gap is the
    // loud limitation documented in docs/limitations.md.
    let writer = r#"
pattern Store(data: String) -> String {
  let _ = memorize(data, call_llm(data))
  return "stored"
}
"#;
    let _ = audit_error_classes(writer);
    let reader = r#"
pattern Read(_tick: String) -> String {
  let y = recall("whatever-the-key-may-be")
  let r = respond("200 OK", y)
  return r
}
"#;
    let classes = audit_error_classes(reader);
    assert!(
        !classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "dynamic-key writes are out of the MVP scope (documented): {classes:?}"
    );
}

// ── strict mode (default OFF) ────────────────────────────────────────

#[test]
fn n386_strict_mode_flags_any_recall_when_other_modules_taint_memory() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    std::env::set_var("METALOGOS_TAINT_STRICT", "1");
    let result = std::panic::catch_unwind(|| {
        let _ = audit_error_classes(WRITER); // any_writes = true
        let reader = r#"
pattern Handle(_tick: String) -> String {
  let y = recall("no-key-match-at-all")
  let r = respond("200 OK", y)
  return r
}
"#;
        let findings = audit_findings(reader);
        let strict = findings
            .iter()
            .find(|f| f.check_id == "TAINT_PERSISTENCE")
            .expect("strict mode must flag the flow without a key match");
        assert!(
            strict.message.contains("strict mode"),
            "the finding says strict mode fired: {}",
            strict.message
        );
    });
    std::env::remove_var("METALOGOS_TAINT_STRICT");
    result.expect("strict-mode contract must hold");
}

#[test]
fn n386_strict_mode_default_is_off() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    std::env::remove_var("METALOGOS_TAINT_STRICT");
    let _ = audit_error_classes(WRITER);
    let reader = r#"
pattern Handle(_tick: String) -> String {
  let y = recall("no-key-match-at-all")
  let r = respond("200 OK", y)
  return r
}
"#;
    let classes = audit_error_classes(reader);
    assert!(
        !classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "without METALOGOS_TAINT_STRICT the strict flag must not fire: {classes:?}"
    );
}

// ── in-slice cross-pattern flows (merged run paths) ──────────────────

#[test]
fn n386_in_slice_cross_pattern_flow_is_caught() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    // Both patterns in ONE module — what a merged root+file audit sees on
    // the run/compile paths. The reader pattern excludes only its OWN
    // writes; the writer pattern's key is a different scope.
    let one_file = r#"
pattern StoreDraft(data: String) -> String {
  let _ = memorize("shared386", call_llm(data))
  return "stored"
}
pattern Handle(_tick: String) -> String {
  let y = recall("shared386")
  let r = respond("200 OK", y)
  return r
}
"#;
    let findings = audit_findings(one_file);
    assert!(
        findings.iter().any(|f| f.check_id == "TAINT_PERSISTENCE"),
        "a cross-pattern flow inside one module must also be caught: {findings:?}"
    );
}

// ── (б) the №376 cache contract is untouched ─────────────────────────

#[test]
fn n386_summaries_cache_contract_unchanged() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    let module = r#"
pattern Wrap(x: String) -> String {
  return upper(x)
}
"#;
    let _ = audit_error_classes(module);
    let (ins, _) = metalogos::audit::summaries_cache_stats();
    assert_eq!(ins, 1, "first audit of a module inserts exactly one entry");
    let _ = audit_error_classes(module);
    let (_, hits) = metalogos::audit::summaries_cache_stats();
    assert!(hits >= 1, "an unchanged module must hit the cache");
}

// ── the registry is bounded (loud safety valve) ──────────────────────

#[test]
fn n386_registry_clear_hook_works() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();
    let _ = audit_error_classes(WRITER);
    // Without a clear, the reader would be caught (the registry persists);
    // after the clear it is green — proving both the persistence and the
    // isolation hook.
    metalogos::audit::memory_taint_registry_clear();
    let reader = r#"
pattern Handle(_tick: String) -> String {
  let y = recall("draft386:latest")
  let r = respond("200 OK", y)
  return r
}
"#;
    let classes = audit_error_classes(reader);
    assert!(
        !classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "after a registry clear there is nothing to match: {classes:?}"
    );
}
