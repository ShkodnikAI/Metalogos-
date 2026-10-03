// ── tests/naryad_557_mcp_semantic_gate.rs ────────────────────────────
// №557 (Wave 25 P1; the audit 02.10 M-7; dispatch gh#925): the SEMANTIC
// startup gate for every mcp-serve entrypoint — the same level of
// protection `mlog serve` got in №523. mcp-serve ran NO semantic pass at
// all (grep 0 check_program sites on the audit revision), so a tool file
// never `mlog check`-ed could serve blocking semantic findings — the
// N-1 class on the MCP surface.
//
// The contract pinned here (and recorded machine-readably in
// scripts/ci/blocking_checks.tsv — the mcp-serve absent row is REPLACED
// by the real read/parse/audit/semantic cells):
//   - a program with a blocking semantic finding refuses mcp-serve
//     LOUDLY and EARLY (before the allowlist check, before any bind,
//     before the stdin loop) — the refusal carries the FIRST blocking
//     finding's stable code at position 0 (№479) and the serve refusal's
//     format, stamped Naryad #557;
//   - imports resolve statically with the same "." rule as serve; a
//     resolution failure is the coded refusal too;
//   - a clean tool file passes the gate (the gate never punishes the
//     honest surface);
//   - every mcp-serve entrypoint carries BOTH gates (stdio, the network
//     transports, the test harness — the call sites in mcp_server.rs).
#![allow(clippy::disallowed_methods)]

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mlog")
}

// ── mcp-serve|read|file-readable — blocks ────────────────────────────

#[test]
fn n557_mcp_missing_file_blocks() {
    let out = Command::new(bin())
        .args(["mcp-serve", "/nonexistent/n557.mlog", "--allowlist", "x"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "an unreadable file must refuse the mcp-serve startup (exit 1)"
    );
}

// ── mcp-serve|parse|parse-error — blocks ─────────────────────────────

#[test]
fn n557_mcp_parse_error_blocks() {
    let dir = std::env::temp_dir().join(format!("n557_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mcp_parse_err.mlog");
    std::fs::write(&path, "tool broken( { emit 1 }\n").unwrap();
    let out = Command::new(bin())
        .args(["mcp-serve", path.to_str().unwrap(), "--allowlist", "x"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a parse error must refuse the mcp-serve startup"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── mcp-serve|audit|category-a — blocks (the №536 gate, re-pinned) ───

#[test]
fn n557_mcp_cat_a_blocks() {
    // A tool carrying a literal destructive SQL statement — the Cat A
    // violation №536's enforce_category_a_startup refuses BEFORE the
    // allowlist check (gh#536 Finding 2).
    let src = r#"
tool wipe_db {
  wipe(table: String) -> String {
    return db_execute("DROP TABLE " + table)
  }
}
"#;
    let dir = std::env::temp_dir().join(format!("n557_cat_a_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mcp_cat_a.mlog");
    std::fs::write(&path, src).unwrap();
    let out = Command::new(bin())
        .args([
            "mcp-serve",
            path.to_str().unwrap(),
            "--allowlist",
            "wipe_db.wipe",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "the Cat A violation must refuse the mcp-serve startup"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── mcp-serve|semantic|check-program-errors — blocks (№557 proper) ───

#[test]
fn n557_mcp_startup_refuses_unknown_function() {
    // The №523 exploit shape on the MCP surface: is_admn is undefined — on
    // an ungated path the call evaluated to the truthy liar string at call
    // time. The blocking carrier is a PATTERN body: check_program walks
    // pattern and route bodies (the №523 walks) — the honest HONOR BOUNDARY
    // of №557 is that the semantic pass's walk surface is UNCHANGED (the
    // naryad forbids semantic changes), so the tool-method-body blindness
    // the walk has on serve persists here too (recorded in the PR as the
    // check_program extension candidate). The gate refuses the STARTUP.
    let src = r#"
tool access {
  check(user: String) -> String {
    return "checked"
  }
}

pattern verify_admin(user: String) -> String {
  let verdict = is_admn(user)
  return verdict
}
"#;
    let declarations = metalogos::parser::parse(src)
        .expect("the fixture must parse cleanly (the gate is the semantic pass)");
    // The gate runs BEFORE the allowlist check and BEFORE the stdin loop:
    // the refusal returns without ever reading stdin — safe to call here.
    let err = metalogos::mcp_server::run_mcp_server(&declarations, &["access.check".to_string()])
        .expect_err("mcp-serve must refuse at startup: is_admn is undefined");
    let text = err.to_string();
    assert!(
        text.contains("Naryad #557"),
        "the refusal must carry the наряд stamp: {}",
        text
    );
    assert!(
        text.contains("semantic findings block mcp-serve startup"),
        "the refusal must name the gate (the serve format mirrored): {}",
        text
    );
    assert!(
        text.contains("[UNDEFINED_FUNCTION]"),
        "the refusal must carry the stable code at position 0 (№479): {}",
        text
    );
    // The same refusal through the gate function directly (the
    // entrypoint-independent contract).
    assert!(metalogos::mcp_server::enforce_semantic_startup(&declarations).is_err());
}

// ── mcp-serve|semantic|import-resolution — blocks (№557 proper) ──────

#[test]
fn n557_mcp_startup_refuses_missing_import() {
    let src = r#"
import nonexistent/n557_module as ghost

tool greets {
  hello(name: String) -> String {
    return "hi"
  }
}
"#;
    let declarations = metalogos::parser::parse(src).expect("the fixture must parse");
    let err = metalogos::mcp_server::enforce_semantic_startup(&declarations)
        .expect_err("a statically-unresolvable import must refuse the mcp-serve startup");
    let text = err.to_string();
    assert!(
        text.contains("Naryad #557"),
        "the import-resolution refusal carries the наряд stamp: {}",
        text
    );
    // The entrypoint refuses too — BEFORE the stdin loop (never blocks).
    let err = metalogos::mcp_server::run_mcp_server(&declarations, &["greets.hello".to_string()])
        .expect_err("the stdio entrypoint must refuse the missing import");
    assert!(
        err.to_string().contains("Naryad #557"),
        "the entrypoint refusal is the coded gate refusal: {}",
        err
    );
}

// ── the clean path: the gate never punishes the honest surface ───────

#[test]
fn n557_mcp_clean_tool_file_passes_the_semantic_gate() {
    let src = r#"
tool greets {
  hello(name: String) -> String {
    return "hello"
  }
}
"#;
    let declarations = metalogos::parser::parse(src).expect("the clean fixture must parse");
    metalogos::mcp_server::enforce_semantic_startup(&declarations)
        .expect("a clean tool file passes the semantic gate");
    // And the Cat A gate agrees (the both-gates posture).
    metalogos::mcp_server::enforce_category_a_startup(&declarations)
        .expect("a clean tool file passes the Category A gate");
}

#[test]
fn n557_every_entrypoint_carries_both_gates() {
    // The structural pin: all three entrypoint sites call BOTH gates —
    // the transports cannot drift (the source is the contract, the same
    // posture as the №551 required-set sync).
    let src = include_str!("../src/mcp_server.rs");
    let cat_a_calls = src
        .matches("enforce_category_a_startup(declarations)?;")
        .count();
    let semantic_calls = src
        .matches("enforce_semantic_startup(declarations)?;")
        .count();
    assert!(
        cat_a_calls >= 3,
        "the Cat A gate must keep its three entrypoint sites, got {}",
        cat_a_calls
    );
    assert_eq!(
        cat_a_calls, semantic_calls,
        "every entrypoint that carries the Cat A gate must carry the semantic gate (№557)"
    );
}
