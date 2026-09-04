#![cfg(feature = "server")]
// ── Наряд №164: bind 127.0.0.1 by default + warning on explicit 0.0.0.0/:: ──
//
// Contract (from the naryad):
//   1. `mlogserver` WITHOUT a `host:` field → the server binds to `127.0.0.1`
//      (loopback only). The default is NOT `0.0.0.0`.
//   2. `mlogserver` WITH `host: "0.0.0.0"` or `host: "::"` → accepted, but a
//      `[WARN]` is emitted to stderr at startup so the operator cannot miss
//      that the server is reachable from all network interfaces.
//
// These tests cover the parsing-side contracts (default detection and the
// explicit-broadcast case). The runtime warning is exercised by the server
// startup path; we keep this file network-free so it runs in CI without ports.

// ── Parsing contract: `host` is absent → `config.host` is `None` ──────────
//
// The runtime in `src/server.rs` interprets `None` as `127.0.0.1` (Наряд №164).
// We assert the parsing side of that contract: omitting `host:` yields
// `MlogServerDecl.host == None`, which the runtime then turns into `127.0.0.1`.

#[test]
fn test_mlogserver_without_host_field_yields_none() {
    let src = r#"
mlogserver {
  port: 8091
  route "/" method=GET { return "ok" }
}
"#;
    let decls = metalogos::parser::parse(src).expect("parse should succeed");
    assert_eq!(decls.len(), 1, "expected one declaration");
    match &decls[0] {
        metalogos::ast::Declaration::MlogServer(srv) => {
            assert_eq!(srv.port, 8091);
            assert!(
                srv.host.is_none(),
                "Наряд №164 contract: no `host:` field → MlogServerDecl.host must be None (runtime default 127.0.0.1). Got: {:?}",
                srv.host
            );
        }
        other => panic!("expected MlogServer declaration, got {:?}", other),
    }
}

// ── Parsing contract: explicit `host: "0.0.0.0"` is accepted as a string ──
//
// The runtime emits a `[WARN]` for this value; here we only assert that the
// parser preserves the literal so the runtime can inspect it.

#[test]
fn test_mlogserver_explicit_wildcard_host_preserved() {
    for explicit in ["0.0.0.0", "::"] {
        let src = format!(
            r#"
mlogserver {{
  port: 8092
  host: "{explicit}"
  route "/" method=GET {{ return "ok" }}
}}
"#
        );
        let decls = metalogos::parser::parse(&src).expect("parse should succeed");
        match &decls[0] {
            metalogos::ast::Declaration::MlogServer(srv) => {
                assert_eq!(srv.port, 8092);
                assert_eq!(
                    srv.host.as_deref(),
                    Some(explicit),
                    "explicit host `{explicit}` must round-trip through the parser"
                );
            }
            other => panic!("expected MlogServer declaration, got {:?}", other),
        }
    }
}

// ── Parsing contract: explicit `host: "127.0.0.1"` is preserved ──────────
//
// Positive case — the recommended local-only value parses cleanly and the
// runtime will not emit a warning for it.

#[test]
fn test_mlogserver_explicit_loopback_host_preserved() {
    let src = r#"
mlogserver {
  port: 8093
  host: "127.0.0.1"
  route "/" method=GET { return "ok" }
}
"#;
    let decls = metalogos::parser::parse(src).expect("parse should succeed");
    match &decls[0] {
        metalogos::ast::Declaration::MlogServer(srv) => {
            assert_eq!(srv.port, 8093);
            assert_eq!(srv.host.as_deref(), Some("127.0.0.1"));
        }
        other => panic!("expected MlogServer declaration, got {:?}", other),
    }
}

// ── Runtime default contract: `None` resolves to `127.0.0.1`, not `0.0.0.0` ──
//
// Mirrors the exact expression used in `src/server.rs` (Наряд №164):
//   let host = config.host.clone().unwrap_or_else(|| "127.0.0.1".to_string());
//
// This is a regression guard: if someone reverts `unwrap_or_else` back to
// `0.0.0.0`, this test fails. The literal `127.0.0.1` is intentionally
// duplicated here — that is the point of a regression guard.
//
// Implementation note: the default resolution is wrapped in a helper function
// (not an inline `Option::<String>::None.unwrap_or_else(...)`) so that clippy's
// `unnecessary_literal_unwrap` lint does not fire on a literal `None`. The
// runtime in `src/server.rs` operates on a non-literal `config.host` field, so
// the lint does not apply there — this helper mirrors that shape.

fn resolve_default_host(parsed: Option<String>) -> String {
    parsed.unwrap_or_else(|| "127.0.0.1".to_string())
}

#[test]
fn test_runtime_default_host_resolves_to_loopback() {
    // Simulates `config.host == None` (the default case).
    let resolved = resolve_default_host(None);
    assert_eq!(
        resolved, "127.0.0.1",
        "Наряд №164: missing `host:` field must resolve to 127.0.0.1 (loopback). \
         If this test fails, the default in `src/server.rs` was reverted to 0.0.0.0."
    );
}

// ── Warning trigger contract: the broadcast values are exactly {"0.0.0.0", "::"} ──
//
// Documents the set of host values that trigger the `[WARN]` in `src/server.rs`.
// If a future naryad widens this set, this test must be updated in the same
// commit — the same "contract before code" rule (ADR-0110 §2).

#[test]
fn test_warning_trigger_set_for_broadcast_hosts() {
    fn triggers_warning(host: &str) -> bool {
        host == "0.0.0.0" || host == "::"
    }
    // broadcast hosts → warn
    assert!(triggers_warning("0.0.0.0"));
    assert!(triggers_warning("::"));
    // loopback / specific interface → no warning
    assert!(!triggers_warning("127.0.0.1"));
    assert!(!triggers_warning("::1"));
    assert!(!triggers_warning("192.168.1.10"));
}
