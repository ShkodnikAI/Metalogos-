//! Naryad №405 (issue #528) — persistence taint layer 2: locally-bound
//! key prefixes (ADR-0170).
//!
//! Contract under test:
//! (а) the office key shape — `let key = "rate_limit:" + provider;
//!     memorize(key, call_llm(...))` — resolves through the let-binding:
//!     a cross-module recall→respond flow on the same prefix is caught
//!     with the SAME TAINT_PERSISTENCE class (no new codes, №385);
//! (б) chained bindings compose (`let k1 = "p:" + id; let k2 = k1 +
//!     ":summary"` names the prefix `"p:"`);
//! (в) fail-closed forks: an unresolvable re-assignment KEEPS the seen
//!     prefix; branch bodies share the enclosing map (monotone);
//! (г) the green paths stay green: sanitized stores and LLM-free writes
//!     register nothing;
//! (д) the №386 literal/prefix MVP pins replay unchanged (inline keys).
//!
//! Strategy: ADR-0170 (points-to explicitly deferred to Phase 7).
//! Manual: `cargo test --test naryad_405_taint_prefix_bindings`.

use std::sync::Mutex;

/// The registry and the env flag are process-global: every test in this
/// file serializes on this lock and clears the registry first.
static TAINT_LOCK: Mutex<()> = Mutex::new(());

fn reset_taint_state() {
    metalogos::audit::memory_taint_registry_clear();
    metalogos::audit::summaries_cache_clear();
    // The strict-mode flag must never leak in from a sibling test process
    // env: this layer's default is OFF (ADR-0170 §3.4).
    std::env::remove_var("METALOGOS_TAINT_STRICT");
}

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

fn audit_findings(source: &str) -> Vec<metalogos::audit::AuditFinding> {
    metalogos::audit_program(source)
        .expect("audit source must parse")
        .findings
}

/// The motivating shape: the office builds keys in locals first
/// (app.mlog:579/707, dept/legal.mlog:325 — the №395 dogfood corpus).
const OFFICE_WRITER: &str = r#"
pattern StoreSummary(data: String) -> String {
  let key = "user405:" + data + ":summary"
  let _ = memorize(key, call_llm(data))
  return "stored"
}
"#;

const OFFICE_READER: &str = r#"
pattern Handle(uid: String) -> String {
  let k = "user405:" + uid
  let y = recall(k)
  let r = respond("200 OK", y)
  return r
}
"#;

// ── (а) the office shape end-to-end ───────────────────────────────────

#[test]
fn n405_cross_module_flow_with_let_bound_keys_is_caught() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    let writer_classes = audit_error_classes(OFFICE_WRITER);
    assert!(
        writer_classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the unsanitized LLM→memory write fails as before: {writer_classes:?}"
    );

    let reader_findings = audit_findings(OFFICE_READER);
    let cross = reader_findings
        .iter()
        .find(|f| {
            f.check_id == "TAINT_PERSISTENCE"
                && f.message.contains("cross-module taint through memory")
        })
        .expect("the let-bound recall→respond flow must be caught");
    assert!(
        cross.message.contains("user405:"),
        "the matched prefix is named: {}",
        cross.message
    );
}

// ── (б) chained bindings compose ──────────────────────────────────────

#[test]
fn n405_chained_bindings_compose_to_the_leading_literal() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    let writer = r#"
pattern StoreChained(data: String) -> String {
  let k1 = "ch405:" + data
  let k2 = k1 + ":summary"
  let _ = memorize(k2, call_llm(data))
  return "stored"
}
"#;
    let writer_classes = audit_error_classes(writer);
    assert!(
        writer_classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the chained-key write fails: {writer_classes:?}"
    );

    let reader = r#"
pattern ReadChained(uid: String) -> String {
  let k = "ch405:" + uid
  let y = recall(k)
  let _ = respond("200 OK", y)
  return y
}
"#;
    let findings = audit_findings(reader);
    let cross = findings
        .iter()
        .find(|f| f.message.contains("cross-module taint through memory"))
        .expect("the chained prefix must be matched");
    assert!(
        cross.message.contains("ch405:"),
        "the chained prefix is named: {}",
        cross.message
    );
}

// ── (в) the fail-closed forks ─────────────────────────────────────────

#[test]
fn n405_unresolvable_reassignment_keeps_the_seen_prefix() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    // `key` is re-assigned from an UNRESOLVABLE shape (a pattern-call
    // result — no leading literal anywhere). The detector must NOT drop
    // the prefix it has already seen (fail-closed).
    let writer = r#"
pattern BuildKey(data: String) -> String {
  return "fc405:" + data
}
pattern StoreRebound(data: String) -> String {
  let mut key = "fc405:" + data
  key = BuildKey(data)
  let _ = memorize(key, call_llm(data))
  return "stored"
}
"#;
    let writer_classes = audit_error_classes(writer);
    assert!(
        writer_classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the re-bound write still fails: {writer_classes:?}"
    );

    let reader = r#"
pattern ReadRebound(uid: String) -> String {
  let k = "fc405:" + uid
  let y = recall(k)
  let _ = respond("200 OK", y)
  return y
}
"#;
    let findings = audit_findings(reader);
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("cross-module taint through memory")),
        "the seen prefix survives the unresolvable re-assignment: {findings:?}"
    );
}

#[test]
fn n405_branch_bound_keys_are_seen_monotonically() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    // The binding is made INSIDE the if-branch and the write happens
    // there too — the shared monotone map sees it (the same convention
    // the recall walker's taint vars use).
    let writer = r#"
pattern StoreBranchy(data: String, flag: Bool) -> String {
  if flag then {
    let key = "br405:" + data
    let _ = memorize(key, call_llm(data))
  }
  return "stored"
}
"#;
    let writer_classes = audit_error_classes(writer);
    assert!(
        writer_classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the branch-bound write fails: {writer_classes:?}"
    );

    let reader = r#"
pattern ReadBranchy(uid: String) -> String {
  let k = "br405:" + uid
  let y = recall(k)
  let _ = respond("200 OK", y)
  return y
}
"#;
    let findings = audit_findings(reader);
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("cross-module taint through memory")
                && f.message.contains("br405:")),
        "the branch-bound prefix is registered and matched: {findings:?}"
    );
}

// ── (г) the green paths stay green ────────────────────────────────────

#[test]
fn n405_sanitized_or_llm_free_bindings_register_nothing() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    // Sanitized at the top of the stored value — nothing taints memory.
    let sanitized_writer = r#"
pattern StoreClean(data: String) -> String {
  let key = "san405:" + data
  let _ = memorize(key, redact(call_llm(data), "secrets"))
  return "stored"
}
"#;
    let classes = audit_error_classes(sanitized_writer);
    assert!(
        !classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the sanitized write stays green: {classes:?}"
    );

    // No LLM source anywhere — the recall→respond flow is legal.
    let llm_free = r#"
pattern SaveFact(_tick: String) -> String {
  let key = "fact405:" + "plain"
  let _ = memorize(key, "plain text, no LLM anywhere")
  return "saved"
}
pattern UseFact(_tick: String) -> String {
  let k = "fact405:" + "plain"
  let v = recall(k)
  let _ = respond("200 OK", v)
  return "ok"
}
flow Main { input: String = "x" -> UseFact -> output }
"#;
    let classes = audit_error_classes(llm_free);
    assert!(
        !classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the LLM-free let-bound pair stays green: {classes:?}"
    );
}

// ── (д) the №386 MVP pins replay unchanged ────────────────────────────

#[test]
fn n405_inline_literal_pins_replay_unchanged() {
    let _g = TAINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_taint_state();

    let writer = r#"
pattern StoreDraft(data: String) -> String {
  let _ = memorize("draft405:", call_llm(data))
  return "stored"
}
"#;
    let reader = r#"
pattern Handle(_tick: String) -> String {
  let y = recall("draft405:latest")
  let r = respond("200 OK", y)
  return r
}
"#;
    let writer_classes = audit_error_classes(writer);
    assert!(
        writer_classes.iter().any(|c| c == "TAINT_PERSISTENCE"),
        "the inline-literal write behaves exactly as №386: {writer_classes:?}"
    );
    let findings = audit_findings(reader);
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("cross-module taint through memory")
                && f.message.contains("draft405:")),
        "the inline-literal cross-module flow is caught as before: {findings:?}"
    );
}
