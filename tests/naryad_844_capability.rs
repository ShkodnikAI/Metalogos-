#![allow(clippy::disallowed_methods)]

// ── Naryad №844 (wave 72 — W-B, P1, [core][security]): the capability
// token in code (ADR-0203 §2.1, the dispatch gh#1666) ───────────────────
//
// The typed Capability carrier lands:
//   - grant_issue / grant_subgrant return the read-only Capability view
//     {scope, class, ttl, quota, revoked, grant} (the ledger stays the
//     SSOT; the token is an untrusted cache);
//   - the migration 1:1: every grant-argument extractor accepts BOTH the
//     opaque Value::Grant and the typed Capability view — existing
//     programs (the w2 grant examples) keep working byte-for-byte;
//   - the single consume points stay grant_use and the carrier gates'
//     own post-success consumption; capability_state is read-only;
//   - fail-closed: a revoked or expired capability refuses BEFORE the
//     action, on BOTH lanes (TW/VM parity — the №829 lesson), with the
//     DenyEvent IRREVERSIBLE_NO_GRANT class (the deny vocabulary does
//     not grow — ADR-0203 §2.4);
//   - consume writes the Ledger (the ACTION-path binding, ADR-0167 §3.4);
//   - negative controls: (а) no consume outside the single point (the
//     view is read-only, copying the token does not duplicate power —
//     the ledger refuses the second use), (б) profile legacy does not
//     disable the fail-closed semantics.

use std::path::Path;

/// The lane-runner type — the TW/VM parity harness form (№748/№829);
/// the named alias keeps the clippy type_complexity lint honest (the
/// inline copies collapsed into one definition).
type LaneRunner = fn(&str) -> Result<Option<String>, String>;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), Path::new(MANIFEST).to_path_buf())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(Path::new(MANIFEST).to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const DB_PRELUDE: &str = r#"
db { url: "sqlite::memory:" }
"#;

// ── The typed mint ─────────────────────────────────────────────────────

#[test]
fn grant_issue_returns_the_typed_capability_view() {
    let out = run_tw(
        r#"
pattern T(_t: String) -> String {
  let c = grant_issue("db:delete:sessions", 3600, "n", 2)
  return str(c.scope) + "|" + c.class + "|" + to_string(c.quota) + "|" + to_string(c.revoked)
}
flow Main { input: String = "t" -> T -> output }
"#,
    )
    .unwrap()
    .unwrap();
    assert_eq!(out.trim(), "db:delete:sessions|n:2|2|false", "got: {out}");
}

#[test]
fn capability_fields_are_the_adr_set_on_both_lanes() {
    for runner in [run_tw, run_vm] {
        let out = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  let c = grant_issue("db:delete:sessions", 3600, "once")
  return str(c.scope) + "|" + c.class + "|" + to_string(c.ttl > now_unix()) + "|" + to_string(c.revoked)
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "db:delete:sessions|once|true|false",
            "the ADR field set rides both lanes"
        );
    }
}

// ── The migration 1:1 ──────────────────────────────────────────────────

#[test]
fn the_typed_token_drives_the_existing_allowing_path() {
    // The w2_grant_linear shape, re-spelled through the typed carrier:
    // db_execute_with_grant accepts the Capability view (and the extractors
    // keep accepting the opaque Grant — the Rust-level unit tests pin it).
    for runner in [run_tw, run_vm] {
        let out = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  db_execute("CREATE TABLE IF NOT EXISTS s (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO s (user) VALUES ('alice')")
  let c = grant_issue("db:delete:s", 3600, "once")
  let n = db_execute_with_grant(c, "DELETE FROM s WHERE user = 'alice'")
  return "deleted:" + n
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(out.trim(), "deleted:1", "got: {out}");
    }
}

// ── Fail-closed: revoked / expired refuse BEFORE the action ────────────

#[test]
fn revoked_capability_refuses_before_the_action_on_both_lanes() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let err = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  db_execute("CREATE TABLE IF NOT EXISTS s (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO s (user) VALUES ('alice')")
  let c = grant_issue("db:delete:s", 3600, "once")
  let _r = grant_revoke(c)
  let n = db_execute_with_grant(c, "DELETE FROM s")
  return "deleted:" + n
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap_err();
        assert!(
            err.starts_with("GRANT_REVOKED"),
            "{name}: a revoked capability must refuse with the typed error, got: {err}"
        );
    }
}

#[test]
fn revoked_capability_deny_event_fires_and_the_table_survives() {
    // The on_deny(db) handler receives the IRREVERSIBLE_NO_GRANT reason
    // (the deny vocabulary does not grow — ADR-0203 §2.4), the refused
    // call degrades to Unit and execution CONTINUES (the w2_deny_exhaustive
    // contract), and the DESTRUCTIVE SQL never executes: the row inserted
    // before the refusal is still there.
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let out = runner(&format!(
            r#"
{DB_PRELUDE}
on_deny(db) {{
  match deny_reason() {{
    "IRREVERSIBLE_NO_GRANT" then {{ print("deny:IRREVERSIBLE_NO_GRANT") }}
    else {{ print("deny:OTHER") }}
  }}
}}
pattern T(_t: String) -> String {{
  db_execute("CREATE TABLE IF NOT EXISTS s (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO s (user) VALUES ('alice')")
  let c = grant_issue("db:delete:s", 3600, "once")
  let _r = grant_revoke(c)
  let n = db_execute_with_grant(c, "DELETE FROM s")
  let found = query("SELECT COUNT(*) AS n FROM s")
  return "refused:" + type_of(n) + "|rows:" + to_string(found[0].n)
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "refused:Unit|rows:1",
            "{name}: the refusal degrades to Unit and the DELETE never touches the table, got: {out}"
        );
    }
}

#[test]
fn expired_capability_refuses_before_the_action_on_both_lanes() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let err = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  db_execute("CREATE TABLE IF NOT EXISTS s (id INTEGER PRIMARY KEY, user TEXT)")
  let c = grant_issue("db:delete:s", 0, "once")
  let n = db_execute_with_grant(c, "DELETE FROM s")
  return "deleted:" + n
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap_err();
        assert!(
            err.starts_with("GRANT_EXPIRED"),
            "{name}: a born-expired capability must refuse before the action, got: {err}"
        );
    }
}

// ── The Ledger binding (ACTION paths, ADR-0167 §3.4) ───────────────────

#[test]
fn capability_consume_writes_the_ledger_trace() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let out = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  db_execute("CREATE TABLE IF NOT EXISTS s (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO s (user) VALUES ('alice')")
  let before = ledger_count()
  let c = grant_issue("db:delete:s", 3600, "once")
  let _ = db_execute_with_grant(c, "DELETE FROM s WHERE user = 'alice'")
  let after = ledger_count()
  return "grew:" + to_string(after > before)
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "grew:true",
            "{name}: the ACTION path must journal itself, got: {out}"
        );
        // The Rust-side read of the in-process chain: the typed lifecycle
        // records rode the ACTION paths (issue + use), ADR-0167 §3.4.
        let records = metalogos::ledger::all_records().unwrap();
        let actions: Vec<&str> = records.iter().map(|r| r.action.as_str()).collect();
        assert!(
            actions.contains(&"grant.issued") && actions.contains(&"grant.used"),
            "{name}: the ledger must carry grant.issued AND grant.used, got: {actions:?}"
        );
    }
}

// ── The LIVE view ──────────────────────────────────────────────────────

#[test]
fn capability_state_is_the_live_ledger_read_on_both_lanes() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let out = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  let c = grant_issue("db:delete:s", 3600, "n", 3)
  let s1 = capability_state(c)
  let _ = grant_use(c)
  let s2 = capability_state(c)
  let _ = grant_revoke(c)
  let s3 = capability_state(c)
  return to_string(s1.quota) + "->" + to_string(s2.quota) + "|state:" + s1.state + "/" + s3.state + "|revoked:" + to_string(s3.revoked)
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "3->2|state:active/revoked|revoked:true",
            "{name}: the live view must read the ledger at call time, got: {out}"
        );
    }
}

#[test]
fn capability_state_refuses_an_unknown_token_fail_closed() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let s = capability_state("not-a-capability")
  return "x"
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.contains("must be a Grant"),
            "{name}: a non-capability argument refuses typed, got: {err}"
        );
    }
}

// ── Negative control (а): no consume outside the single point ──────────

#[test]
fn the_view_is_read_only_never_moves_the_quota() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        // capability_state × many (and copies of the token) never move
        // the quota — the view is a read, the consume points are not it.
        let out = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  let c = grant_issue("db:delete:s", 3600, "once")
  let alias = c
  let s1 = capability_state(c)
  let s2 = capability_state(alias)
  let s3 = capability_state(c)
  return to_string(s1.quota) + "/" + to_string(s2.quota) + "/" + to_string(s3.quota)
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "1/1/1",
            "{name}: the read-only view must never move the quota, got: {out}"
        );
    }
}

#[test]
fn copying_the_token_does_not_duplicate_power() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        // A COPY of the token is the SAME grant: the once linearity lives
        // in the LEDGER — the second consume refuses GRANT_REUSED (the
        // static linearity lint may or may not see the alias; the ledger
        // is the authority — either way the refusal fires).
        let outcome = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  let c = grant_issue("db:delete:s", 3600, "once")
  let alias = c
  let first = grant_use(c)
  let second = grant_use(alias)
  return to_string(first) + "|" + to_string(second)
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ));
        match outcome {
            Err(e) => assert!(
                e.contains("GRANT_REUSED"),
                "{name}: the alias consume must refuse GRANT_REUSED, got: {e}"
            ),
            Ok(v) => {
                let out = v.unwrap_or_default();
                assert!(
                    out.contains("GRANT_REUSED"),
                    "{name}: the alias consume must refuse GRANT_REUSED at runtime, got: {out}"
                );
            }
        }
    }
}

// ── Negative control (б): profile legacy does not disable fail-closed ──

#[test]
fn profile_legacy_does_not_weaken_the_capability_refusal() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let err = runner(&format!(
            r#"
profile legacy {{ egress: permissive_with_audit }}
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  db_execute("CREATE TABLE IF NOT EXISTS s (id INTEGER PRIMARY KEY, user TEXT)")
  let c = grant_issue("db:delete:s", 3600, "once")
  let _r = grant_revoke(c)
  let n = db_execute_with_grant(c, "DELETE FROM s")
  return "deleted:" + n
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap_err();
        assert!(
            err.starts_with("GRANT_REVOKED"),
            "{name}: the legacy compatibility profile must not weaken the capability refusal, got: {err}"
        );
    }
}

// ── Subgrant returns the typed carrier too ─────────────────────────────

#[test]
fn subgrant_mints_the_typed_carrier_and_stays_attenuation_only() {
    let runners: [(&str, LaneRunner); 2] = [("tw", run_tw), ("vm", run_vm)];
    for (name, runner) in runners {
        let err = runner(&format!(
            r#"
{DB_PRELUDE}
pattern T(_t: String) -> String {{
  let parent = grant_issue("db:delete:s", 3600, "once")
  let child = grant_subgrant(parent, "db:delete:s", 60)
  let first = db_execute_with_grant(child, "CREATE TABLE IF NOT EXISTS t844 (id INTEGER)")
  let _second = db_execute_with_grant(parent, "CREATE TABLE IF NOT EXISTS t844b (id INTEGER)")
  return first
}}
flow Main {{ input: String = "t" -> T -> output }}
"#,
        ))
        .unwrap_err();
        // The Once parent was consumed by the split (the linear transfer):
        // the parent's later use refuses — statically (the GRANT_REUSED
        // Category A error) or at the ledger (the typed GRANT_REUSED).
        assert!(
            err.contains("GRANT_REUSED"),
            "{name}: the linear-transfer parent must refuse, got: {err}"
        );
    }
}
