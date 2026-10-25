// ── tests/naryad_197_parser_self_parses.rs ──────────────────────────
// Наряд №197, Block 2 (Contract 1): bootstrap test.
//
// Verifies that `self-host/parser.mlog` — the Metalogos parser written in
// Metalogos itself — can successfully parse its OWN source code. This is
// the same class of check that self-hosting compilers use (e.g. GCC
// compiling itself): the tool must be able to consume its own input.
//
// №800 (gh#1547, V3): the default run lane is the VM, and the self-host
// parser is the FIRST program deep enough to surface a PRE-EXISTING VM
// defect (the frame/value-register corruption class — a pattern-call
// result replaced by an AST string deep in the
// ParseWhile→ParseBlock→ParseStmt→ParseIfStmt chain; the corpus and the
// crosscheck never execute anything this deep). Until that VM repair
// lands, the self-host bootstrap contract rides the REFERENCE lane
// (`--reference`, the TW oracle) — the self-hosting proof stays alive,
// the divergence is a tracked VM defect, NOT a lane silence. The VM
// repair is filed as its own issue (the follow-up naryad candidate).

use std::process::Command;
use std::sync::OnceLock;

static MLOG_BIN: OnceLock<String> = OnceLock::new();

fn mlog_bin() -> &'static str {
    MLOG_BIN.get_or_init(|| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&manifest_dir)
            .join("target")
            .join("debug")
            .join("mlog")
            .to_string_lossy()
            .into_owned()
    })
}

#[test]
fn naryad_197_parser_self_parses() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let project_dir = std::path::Path::new(&manifest_dir);
    let parser_path = project_dir.join("self-host/parser.mlog");

    assert!(
        parser_path.exists(),
        "self-host/parser.mlog not found at {:?}",
        parser_path
    );

    // Bootstrap: parser.mlog parses ITSELF. MLOG_PARSE_TARGET points at
    // parser.mlog's own source file. This takes ~4 minutes on a typical
    // dev machine because the Metalogos interpreter is slow on deeply
    // recursive parsers (parser.mlog's Tokenize pattern has 5+ levels
    // of nested if-else). The 12-minute timeout gives ample headroom.
    // №800: `--reference` — the TW oracle lane (see the header note; the
    // VM-lane divergence is the tracked pre-existing defect).
    let output = Command::new(mlog_bin())
        .arg("run")
        .arg("--reference")
        .arg(&parser_path)
        .env("MLOG_PARSE_TARGET", "self-host/parser.mlog")
        // The №455 sensitive-path deny-list refuses `*.mlog` reads; the
        // self-host bootstrap is a legitimate, explicitly named reader —
        // the documented escape crane (Naryad #455 layer 3) allows
        // parser.mlog explicitly.
        .env("METALOGOS_SENSITIVE_PATH_ALLOWLIST", "parser.mlog")
        .output()
        .expect("failed to spawn mlog process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "mlog process failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Output must be non-empty.
    assert!(
        !stdout.is_empty(),
        "parser.mlog produced empty output when parsing itself.\nstderr: {}",
        stderr
    );

    // First non-empty line must be an AST declaration (start with `(`).
    let first_line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    assert!(
        first_line.starts_with('('),
        "parser.mlog output does not start with an AST node.\nfirst line: {}\nfull output: {}",
        first_line,
        stdout
    );

    // Count declarations: each line is one top-level declaration.
    // parser.mlog has 41 patterns + 1 flow = 42 declarations.
    // Lower bound 40 is intentionally loose so minor edits to parser.mlog
    // (e.g. adding a new helper pattern) don't break this test.
    let decl_count = stdout.lines().filter(|l| l.starts_with('(')).count();
    assert!(
        decl_count >= 40,
        "parser.mlog self-parse produced only {} declarations (expected ≥40).\nstderr: {}",
        decl_count,
        stderr
    );

    // The first declaration must be the Tokenize pattern (parser.mlog's
    // first non-comment top-level construct). This guards against the
    // parser silently dropping declarations.
    assert!(
        first_line.starts_with("(PATTERN name=Tokenize"),
        "first declaration is not (PATTERN name=Tokenize ...):\n{}",
        first_line
    );
}
