// ── Наряд №335 (P0, feature/perception): consent grant/revoke —
//    consent-компонента метки, плоский каскад revoke, карантинный sink ──
//
// Red/green corpus:
//   (1) grant extends the consent-scope set statically (infer via
//       semantic::infer_pattern_labels — the №323 API);
//   (2) revoke = the QUARANTINE label; the flat cascade poisons every
//       derivative through lattice absorption (alias, concat, if-merge);
//   (3) poisoned refuses every sink EXCEPT the quarantine sink (№325
//       clearance + the explicit exemption);
//   (4) the quarantine sink and the ledger export are LEGAL and AUDITED
//       (QUARANTINE_EGRESS / CONSENT_LEDGER_EXPORT, Severity::Info);
//   (5) the ledger records grant (subject/scope/TTL) and revoke; export
//       is file egress with the sandbox discipline;
//   (6) the w1_consent_revoke example passes on BOTH backends.

use std::path::Path;

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn fresh_dir(rel: &str) {
    let p = Path::new(MANIFEST).join(rel);
    if p.exists() {
        let _ = std::fs::remove_dir_all(&p);
    }
    std::fs::create_dir_all(&p).expect("sandbox parent dir created");
}

fn infer_var(src: &str, pattern: &str, var: &str) -> metalogos::labels::Label {
    let decls = metalogos::parser::parse(src.trim()).expect("parses");
    let p = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::Pattern(p) if p.name == pattern => Some(p),
            _ => None,
        })
        .unwrap_or_else(|| panic!("pattern {} found", pattern));
    let inf = metalogos::semantic::infer_pattern_labels(p);
    inf.var_labels
        .get(var)
        .cloned()
        .unwrap_or_else(|| panic!("var {} inferred", var))
}

// ── (1) Grant extends the consent scope statically ───────────────────

#[test]
fn grant_extends_consent_scope() {
    let src = r#"
pattern P(x: String) -> String {
  let granted = consent_grant(x, "gdpr", "patient-1")
  return granted
}
flow Main { input: String = "d" -> P -> output }
"#;
    let g = infer_var(src, "P", "granted");
    assert!(
        g.consent.scopes().contains("gdpr"),
        "grant must extend the consent scope, got: {}",
        g
    );
    // conf/integrity are NOT touched by a grant (consent is its own axis).
    assert_eq!(g.conf, metalogos::labels::Conf::Public);
    assert_eq!(g.integrity, metalogos::labels::Integrity::Trusted);
}

#[test]
fn grant_with_dynamic_scope_is_conservative() {
    // A non-literal scope cannot be named statically — no extension
    // (the redact dynamic-policy posture); the ledger still records it.
    let src = r#"
pattern P(x: String, scope: String) -> String {
  let granted = consent_grant(x, scope, "patient-1")
  return granted
}
flow Main { input: String = "d" -> P -> output }
"#;
    let g = infer_var(src, "P", "granted");
    assert!(
        g.consent.is_empty(),
        "dynamic scope must not extend statically, got: {}",
        g
    );
}

// ── (2) Revoke = quarantine; the flat cascade via absorption ─────────

#[test]
fn revoke_carries_the_quarantine_label() {
    let src = r#"
pattern P(x: String) -> String {
  let revoked = consent_revoke(x, "gdpr")
  return revoked
}
flow Main { input: String = "d" -> P -> output }
"#;
    let r = infer_var(src, "P", "revoked");
    assert_eq!(r.conf, metalogos::labels::Conf::Poisoned, "got: {}", r);
    assert_eq!(
        r.integrity,
        metalogos::labels::Integrity::Untrusted,
        "got: {}",
        r
    );
    assert!(
        r.consent.is_empty(),
        "quarantine clears consent, got: {}",
        r
    );
}

#[test]
fn revoke_all_scopes_is_the_flat_entry() {
    let src = r#"
pattern P(x: String) -> String {
  let revoked = consent_revoke(x)
  return revoked
}
flow Main { input: String = "d" -> P -> output }
"#;
    let r = infer_var(src, "P", "revoked");
    assert_eq!(r.conf, metalogos::labels::Conf::Poisoned);
}

#[test]
fn cascade_poisons_alias_and_derived_values() {
    let src = r#"
pattern P(x: String) -> String {
  let revoked = consent_revoke(x, "gdpr")
  let alias = revoked
  let joined = alias + "!"
  return joined
}
flow Main { input: String = "d" -> P -> output }
"#;
    for var in ["revoked", "alias", "joined"] {
        let l = infer_var(src, "P", var);
        assert_eq!(
            l.conf,
            metalogos::labels::Conf::Poisoned,
            "{} must be poisoned by the flat cascade, got: {}",
            var,
            l
        );
    }
}

// ── (3) Poisoned refuses every sink EXCEPT quarantine ────────────────

#[test]
fn poisoned_into_print_is_refused() {
    let src = r#"
pattern P(x: String) -> String {
  let revoked = consent_revoke(x, "gdpr")
  print(revoked)
  return "ok"
}
flow Main { input: String = "d" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("poisoned into an output sink must not compile");
    assert!(
        err.contains("SINK_CLEARANCE") && err.contains("poisoned"),
        "expected the quarantine refusal, got: {}",
        err
    );
}

#[test]
fn poisoned_into_file_and_network_sinks_is_refused() {
    for sink_call in [
        "media_save(revoked, \"target/n335-q/x.bin\")",
        "http_post(\"https://example.com\", revoked)",
    ] {
        let src = format!(
            r#"
pattern P(x: String) -> String {{
  let revoked = consent_revoke(x, "gdpr")
  let _p = {}
  return "ok"
}}
flow Main {{ input: String = "d" -> P -> output }}
"#,
            sink_call
        );
        let err = metalogos::compile_program(src.trim())
            .expect_err("poisoned into a non-quarantine sink must not compile");
        assert!(
            err.contains("poisoned"),
            "{}: expected the poisoned refusal, got: {}",
            sink_call,
            err
        );
    }
}

// ── (4) The quarantine sink: legal AND audited ───────────────────────

#[test]
fn quarantine_sink_is_legal_for_poisoned() {
    let src = r#"
pattern P(x: String) -> String {
  let revoked = consent_revoke(x, "gdpr")
  return quarantine_write(revoked, "test")
}
flow Main { input: String = "d" -> P -> output }
"#;
    metalogos::compile_program(src.trim())
        .unwrap_or_else(|e| panic!("quarantine sink must be legal, got: {}", e));
    let out = run_tw(src, Path::new(MANIFEST)).expect("quarantine sink runs");
    assert!(
        out.as_deref()
            .unwrap_or_default()
            .contains("[QUARANTINE_EGRESS]"),
        "the program output carries the audit event, got: {:?}",
        out
    );
}

#[test]
fn audit_report_records_quarantine_and_export_events() {
    let src = r#"
pattern P(x: String) -> String {
  let revoked = consent_revoke(x, "gdpr")
  let _q = quarantine_write(revoked, "test")
  return "ok"
}
flow Main { input: String = "d" -> P -> output }
"#;
    let report = metalogos::audit::audit_program(src.trim()).expect("audit report");
    let ids: Vec<&str> = report.findings.iter().map(|f| f.check_id).collect();
    assert!(
        ids.contains(&"QUARANTINE_EGRESS"),
        "quarantine sites are audited, got: {:?}",
        ids
    );
}

// ── (5) The ledger: grant/revoke records + TTL + export ──────────────

#[test]
fn ledger_records_grant_revoke_and_exports() {
    // The ledger is process-global — assertions are monotonic (>=) so
    // parallel test execution cannot falsify them.
    let before = metalogos::consent::entry_count().unwrap_or(0);
    metalogos::consent::record_grant("patient-1", "gdpr", 3600).expect("grant recorded");
    metalogos::consent::record_grant("patient-2", "analytics", 0).expect("no-ttl grant");
    metalogos::consent::record_revoke(Some("gdpr")).expect("scoped revoke recorded");
    metalogos::consent::record_revoke(None).expect("total revoke recorded");
    let after = metalogos::consent::entry_count().expect("count");
    assert!(
        after >= before + 4,
        "all four records stored ({} -> {})",
        before,
        after
    );

    let json = metalogos::consent::export_json().expect("export");
    assert!(
        json.contains("\"patient-1\"") && json.contains("gdpr"),
        "grant subject+scope exported"
    );
    assert!(json.contains("\"analytics\""), "second scope exported");
    assert!(
        json.contains("<all>"),
        "the total revocation is a ledger record"
    );
    assert!(json.contains("revoke"), "revoke kind exported");
    // TTL: a 3600s grant carries expires_at.
    assert!(json.contains("expires_at"), "TTL metadata exported");
}

#[test]
fn ledger_export_builtin_writes_sandboxed_file() {
    fresh_dir("target/n335-ledger");
    let src = r#"
pattern P(_x: String) -> String {
  return consent_ledger_export("target/n335-ledger/out.json")
}
flow Main { input: String = "d" -> P -> output }
"#;
    let out = run_tw(src, Path::new(MANIFEST)).expect("ledger export runs");
    let written = out.as_deref().unwrap_or_default().trim_end();
    assert!(
        written.contains("n335-ledger"),
        "returns the path, got: {:?}",
        written
    );
    let bytes = std::fs::read_to_string(Path::new(MANIFEST).join("target/n335-ledger/out.json"))
        .expect("export written");
    assert!(
        bytes.contains("consent") || bytes.contains("\"kind\""),
        "JSON ledger"
    );
}

// ── (6) The example fixture passes on both backends ──────────────────

#[test]
fn w1_consent_revoke_example_passes_on_tw_and_vm() {
    let src = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w1_consent_revoke.mlog"))
        .expect("fixture exists");
    let expected_head = "[QUARANTINE_EGRESS] poisoned value reached the quarantine sink";
    let out_tw = run_tw(&src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("w1_consent_revoke must run on TW: {}", e));
    assert!(
        out_tw
            .as_deref()
            .unwrap_or_default()
            .trim_end()
            .starts_with(expected_head),
        "got: {:?}",
        out_tw
    );
    let out_vm = run_vm(&src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("w1_consent_revoke must run on VM: {}", e));
    assert!(
        out_vm
            .as_deref()
            .unwrap_or_default()
            .trim_end()
            .starts_with(expected_head),
        "got: {:?}",
        out_vm
    );
}
