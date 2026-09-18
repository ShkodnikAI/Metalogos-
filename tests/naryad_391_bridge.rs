// ── Naryad #391 (P0, security/action): the data ↔ action bridge ────────
//
// Verifies the bridge rule over the six ACTION sinks (exec, exec_argv,
// git_push, http_post, send_message, db_execute): the decision argument
// must satisfy label ⊑ maxTaint (conf) AND integrity(args) ≥ trusted —
// with explainable denies (arg, label, failed threshold) and NO
// weakening of the existing specialized classes.

use metalogos::audit::{audit_category_a, Severity};

fn findings_for(src: &str) -> Vec<(String, String)> {
    let decls = metalogos::parser::parse(src.trim()).expect("parses");
    audit_category_a(&decls, "")
        .into_iter()
        .map(|f| (f.check_id.to_string(), f.message))
        .collect()
}

fn has_class(src: &str, class: &str) -> bool {
    findings_for(src).iter().any(|(id, _)| id == class)
}

// ── DoD (а): the bridge table covers all six action sinks ──────────────

#[test]
fn bridge_table_covers_all_six_action_sinks() {
    const SINKS: &[&str] = &[
        "exec",
        "exec_argv",
        "git_push",
        "http_post",
        "send_message",
        "db_execute",
    ];
    assert_eq!(metalogos::semantic::ACTION_BRIDGE.len(), SINKS.len());
    for sink in SINKS {
        let row = metalogos::semantic::ACTION_BRIDGE
            .iter()
            .find(|r| r.sink == *sink)
            .unwrap_or_else(|| panic!("bridge table misses {}", sink));
        assert_eq!(row.max_conf, "public", "{}: conf threshold", sink);
        assert_eq!(
            row.min_integrity, "trusted",
            "{}: integrity threshold",
            sink
        );
        assert!(!row.decision_arg.is_empty());
        assert!(!row.integrity_enforcement.is_empty());
    }
}

// ── DoD (в): existing classes are NOT weakened — old reds stay red ─────

#[test]
fn existing_specialized_classes_still_fire() {
    // untrusted command → exec
    let src = r#"
pattern P(_t: String) -> String {
  let cmd = http_get("https://example.com/cmd")
  let _ = exec(cmd)
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    assert!(has_class(src, "UNTRUSTED_EXEC_DECISION"));

    // private label → exec
    let src = r#"
pattern P(_t: String) -> String {
  let cmd = env("HOME")
  let _ = exec(cmd)
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    assert!(has_class(src, "SECRET_TO_EXEC"));

    // private URL → http_post address (the address-position rule)
    let src = r#"
pattern P(_t: String) -> String {
  let _ = http_post("http://intranet.local/endpoint", "x")
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    assert!(has_class(src, "SECRET_EGRESS_NETWORK"));

    // private repo → git_push
    let src = r#"
pattern P(_t: String) -> String {
  let _ = git_push(env("MLOG_TOKEN"))
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    assert!(has_class(src, "SECRET_EGRESS_VCS"));

    // destructive SQL literal without a grant — fail-closed unchanged
    let src = r#"
pattern P(_t: String) -> String {
  let _ = db_execute("DROP TABLE users")
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    assert!(has_class(src, "IRREVERSIBLE_NO_GRANT"));
}

// ── The NEW bridge coverage: vcs integrity (the one true gap) ──────────

#[test]
fn untrusted_git_push_target_is_refused_with_explanation() {
    let src = r#"
pattern P(_t: String) -> String {
  let remote = http_get("https://example.com/next-push-target")
  let _ = git_push(remote)
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    let fs = findings_for(src);
    let hit = fs
        .iter()
        .find(|(id, _)| id == "UNTRUSTED_EGRESS_NETWORK")
        .expect("the bridge must refuse an untrusted git_push target");
    // Explainable refusal (DoD + №392 consumption): argument index, sink,
    // label, BOTH thresholds, and the failed threshold name.
    let (_, msg) = hit;
    assert!(msg.contains("argument 0 of git_push"), "msg: {}", msg);
    assert!(msg.contains("public, untrusted"), "msg: {}", msg);
    assert!(msg.contains("conf ⊑ public"), "msg: {}", msg);
    assert!(msg.contains("integrity ≥ trusted"), "msg: {}", msg);
    assert!(msg.contains("untrusted-egress"), "msg: {}", msg);
}

#[test]
fn trusted_constants_pass_the_bridge_on_all_action_sinks() {
    // The decision args are trusted literals — no findings (and the
    // runnable subset executes: the golden ok_391 example runs it).
    let src = r#"
pattern P(_t: String) -> String {
  let _ = git_push("https://git.example.com/proj")
  let _ = http_post("https://example.com/hook", "hello")
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    let fs = findings_for(src);
    assert!(
        fs.is_empty(),
        "trusted constants must clear the bridge: {:?}",
        fs.iter().map(|(id, _)| id).collect::<Vec<_>>()
    );
}

// ── Grant orthogonality (№390): the bridge does not duplicate grants ───

#[test]
fn grant_bridge_is_orthogonal() {
    // db_execute_with_grant is NOT a №325 sink: the bridge adds no static
    // finding for it (its gates are runtime-only, №390).
    let src = r#"
pattern P(_t: String) -> String {
  let g = grant_issue("db:delete:users", 3600, "n", 2)
  let _ = db_execute_with_grant(g, "DELETE FROM users WHERE id = 1")
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    let fs = findings_for(src);
    assert!(
        fs.is_empty(),
        "granted actions carry no bridge findings: {:?}",
        fs.iter().map(|(id, _)| id).collect::<Vec<_>>()
    );
    // …and the runtime grant gates are still the ones that refuse.
    let refuse = r#"
db { url: "sqlite::memory:" }
pattern P(_t: String) -> String {
  let _ = db_execute("CREATE TABLE users (id INTEGER PRIMARY KEY)")
  let g = grant_issue("db:delete:users", 3600, "n", 1)
  let _ = db_execute_with_grant(g, "DELETE FROM users")
  let _ = db_execute_with_grant(g, "DELETE FROM users")
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    let out = metalogos::run_program(refuse.trim());
    let err = match out {
        Err(e) => e,
        Ok(_) => panic!("the second granted delete must fail at runtime (GRANT_REUSED)"),
    };
    // N(1): the second delete exhausts the quota — either typed refusal
    // (GRANT_EXHAUSTED here) proves the runtime grant gate is intact.
    assert!(err.contains("GRANT_EXHAUSTED"), "got: {}", err);
}

// ── Leak-suite corpus: the new red/green pair ──────────────────────────

#[test]
fn leak_corpus_n391_is_red_for_the_right_reason() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/leak/n391_git_push_untrusted_url.mlog"),
    )
    .expect("corpus file exists");
    assert!(has_class(&src, "UNTRUSTED_EGRESS_NETWORK"));
}

#[test]
fn leak_corpus_ok391_is_green_and_runs() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/leak/ok_391_trusted_actions.mlog"),
    )
    .expect("corpus file exists");
    let errors: Vec<_> = findings_for(&src)
        .into_iter()
        .filter(|(_, _)| true)
        .collect();
    // (findings_for returns (id, message); re-check severity separately —
    // SQL_DYNAMIC emits an all-literal INFO confirmation that is not a gate.)
    let decls = metalogos::parser::parse(src.trim()).expect("parses");
    let blocking = metalogos::audit::audit_category_a(&decls, "")
        .into_iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    assert_eq!(blocking, 0, "trusted actions must pass: {:?}", errors);
    assert!(blocking == 0 && !errors.is_empty() || errors.is_empty());
    let out = metalogos::run_program(src.trim())
        .expect("runs")
        .expect("output");
    assert_eq!(out.trim(), "actions-ok");
}

// ── Explainable refusal is Error-severity on the blocking path ─────────

#[test]
fn bridge_denies_are_error_severity() {
    let src = r#"
pattern P(_t: String) -> String {
  let remote = http_get("https://example.com/next-push-target")
  let _ = git_push(remote)
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    let decls = metalogos::parser::parse(src.trim()).expect("parses");
    let findings = audit_category_a(&decls, "");
    assert!(
        findings
            .iter()
            .any(|f| f.check_id == "UNTRUSTED_EGRESS_NETWORK" && f.severity == Severity::Error),
        "bridge denies are blocking"
    );
}
