// ── Naryad #349 (P1, security/session): the duty-profile compile rule ─
//
// The registry contract: in the duty (background) profile of the
// session model (№348/ADR-0172) —
//   D1  `materialization: denied` — lifting a private-labeled value
//       into TEXT (str/json_encode) is a COMPILE ERROR with the data
//       path named (the semantic violation carries container/arg);
//   D2  `surfaces: local_only` — every Network-class builtin (the №316
//       SSOT class list) is a COMPILE ERROR;
//   D3  the rule engages ONLY through the `profile duty` declaration —
//       NOT a compiler flag; programs without it are untouched (the
//       214+ example corpus and `profile legacy` stay green);
//   D4  `profile legacy` does NOT downgrade the duty rule (a program
//       declaring duty and violating it is contradictory — the
//       deepfake-gate posture);
//   D5  the leak-suite corpus: ≥5 duty negatives + ≥2 positives (the
//       runner asserts them — this file asserts the mechanics).
//
// Crosscheck note: the rule is a COMPILE-PHASE check (semantic walk) —
// backend-independent by construction; the positives still run through
// TW and VM below (the compile result is identical).

use metalogos::audit::{audit_category_a, AuditFinding, Severity};
use metalogos::parser;
use metalogos::semantic;

fn violations_for(src: &str) -> Vec<semantic::SinkViolation> {
    let decls = parser::parse(src).expect("parses");
    semantic::sink_clearance_violations(&decls)
}

fn duty_reasons(src: &str) -> Vec<String> {
    violations_for(src)
        .into_iter()
        .filter(|v| v.reason.starts_with("duty-"))
        .map(|v| v.reason.to_string())
        .collect()
}

// ── D1: private materialization is refused ────────────────────────────

#[test]
fn duty_materialization_denied_refuses_str_and_json_encode() {
    let src = r#"
profile duty { materialization: denied }
pattern P(_t: String) -> String {
  let doc = "confidential report"
  let t = str(doc)
  return t
}
flow Main { input: String = "x" -> P -> output }
"#;
    let reasons = duty_reasons(src);
    assert_eq!(
        reasons,
        vec!["duty-materialization"],
        "str lift must be refused"
    );

    let src = r#"
profile duty { materialization: denied }
pattern P(_t: String) -> String {
  let doc = "personal dossier: паспорт 4509 123456"
  let j = json_encode(doc)
  return j
}
flow Main { input: String = "x" -> P -> output }
"#;
    let reasons = duty_reasons(src);
    assert_eq!(
        reasons,
        vec!["duty-materialization"],
        "json_encode lift must be refused"
    );
}

// ── D2: network sinks are refused ─────────────────────────────────────

#[test]
fn duty_surfaces_local_only_refuses_network_class() {
    for (builtin, call) in [
        ("http_post", r#"http_post("https://example.com", "hi")"#),
        ("http_get", r#"http_get("https://example.com/s")"#),
        ("send_message", r#"send_message("chat-1", "hello")"#),
    ] {
        let src = format!(
            r#"
profile duty {{ surfaces: local_only }}
pattern P(_t: String) -> String {{
  let r = {}
  return type_of(r)
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
            call
        );
        let reasons = duty_reasons(&src);
        assert_eq!(
            reasons,
            vec!["duty-network-sink"],
            "{} must be refused in duty mode",
            builtin
        );
    }
}

// ── D3: the rule engages ONLY through the declaration ────────────────

#[test]
fn without_the_duty_declaration_nothing_changes() {
    // The SAME programs without `profile duty` — no duty violations
    // (the 214+ corpus and profile legacy stay untouched).
    let src = r#"
pattern P(_t: String) -> String {
  let doc = "confidential report"
  let t = str(doc)
  return t
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert!(
        duty_reasons(src).is_empty(),
        "no declaration — no duty rule"
    );

    let src = r#"
profile legacy { egress: permissive_with_audit }
pattern P(_t: String) -> String {
  let doc = "confidential report"
  let t = str(doc)
  return t
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert!(
        duty_reasons(src).is_empty(),
        "legacy is NOT duty — the compatibility profile must not engage the rule"
    );
}

// ── D4: legacy does not downgrade the duty rule ──────────────────────

#[test]
fn legacy_profile_does_not_downgrade_duty() {
    let src = r#"
profile legacy { egress: permissive_with_audit }
profile duty { materialization: denied surfaces: local_only }
pattern P(_t: String) -> String {
  let doc = "confidential report"
  let t = str(doc)
  let r = http_post("https://example.com", "hi")
  return t
}
flow Main { input: String = "x" -> P -> output }
"#;
    let mut reasons = duty_reasons(src);
    reasons.sort();
    assert_eq!(
        reasons,
        vec!["duty-materialization", "duty-network-sink"],
        "both duty rules fire; legacy's advisory posture must not downgrade them"
    );
    // The audit side: the findings are Severity::Error with the DUTY_*
    // check_ids (the compile path refuses them even under legacy).
    let decls = parser::parse(src).expect("parses");
    let findings = audit_category_a(&decls, "");
    let duty_findings: Vec<&AuditFinding> = findings
        .iter()
        .filter(|f| f.check_id.starts_with("DUTY_"))
        .collect();
    assert_eq!(duty_findings.len(), 2, "both duty findings present");
    for f in duty_findings {
        assert!(
            matches!(f.severity, Severity::Error),
            "duty findings are never advisory: {}",
            f.check_id
        );
    }
}

// ── D5: the audit promotion — compile-path codes ─────────────────────

#[test]
fn audit_carries_duty_check_ids() {
    let src = r#"
profile duty { materialization: denied surfaces: local_only }
pattern P(_t: String) -> String {
  let doc = "confidential report"
  let t = str(doc)
  return t
}
flow Main { input: String = "x" -> P -> output }
"#;
    let decls = parser::parse(src).expect("parses");
    let findings = audit_category_a(&decls, "");
    let ids: Vec<&str> = findings.iter().map(|f| f.check_id).collect();
    assert!(
        ids.contains(&"DUTY_MATERIALIZATION"),
        "the compile path must carry DUTY_MATERIALIZATION, got {:?}",
        ids
    );
}

// ── Positives compile + run on BOTH backends ─────────────────────────

#[test]
fn duty_positives_compile_and_run_tw_and_vm() {
    let src = r#"
profile duty { materialization: denied surfaces: local_only }
pattern P(_t: String) -> String {
  let doc = "confidential report"
  let masked = redact(doc, "all")
  let n = len(masked)
  return "ok:" + str(n)
}
flow Main { input: String = "x" -> P -> output }
"#;
    let decls = parser::parse(src).expect("parses");
    assert!(
        semantic::check_program(&decls).is_ok(),
        "the redact path is legal in duty mode"
    );
    assert!(duty_reasons(src).is_empty());
    // Backend-independent compile phase — but run both anyway (the
    // crosscheck note): TW
    let out_tw =
        metalogos::run_program_with_dir(src, std::path::PathBuf::from(".")).expect("TW run");
    // VM: compile to bytecode, then run.
    let program = metalogos::compile_program(src).expect("VM compile");
    let out_vm = metalogos::run_bytecode(program).expect("VM run");
    assert_eq!(out_tw, out_vm, "TW/VM parity of the duty positive");
}
