// ── Naryad №523 (P0, compiler/security): semantic findings block run/serve ──
//
// Contract (issue #832, audit 30.09 N-1 release-block):
//
// 1. THE LIAR STRING IS GONE: an unknown function used to evaluate to the
//    STRING "[ERROR: unknown function 'x']" as the call's VALUE. Non-empty
//    strings are truthy in is_truthy, so `if is_admn(user)` with no
//    `is_admn` defined took the then-branch — a security check became its
//    own bypass. The interpreter now returns an err-origin carrying the
//    stable №479 code UNDEFINED_FUNCTION (pinned in
//    n523_tw_interpreter_err_origin_not_liar_string).
//
// 2. THE GATE BLOCKS EVERYTHING: `mlog run` blocks on EVERY
//    semantic::check_program error, not just two substring classes
//    ("distill_to", "[DENY_"). Exemptions classify ONLY by the structured
//    kind in semantic::is_exempt_from_blocking (the ONE explicit place,
//    empty today). The №181/№392 classes still block — they are a subset
//    of "every error" now (pinned by the _still_blocks tests).
//
// 3. SERVE CHECKS SEMANTICS TOO: `mlog serve` previously ran NO semantic
//    pass at all — run_server now refuses at STARTUP on any semantic
//    finding (pinned through the real entry point, the same posture as
//    the №455 startup-refusal tests: a refused program returns Err before
//    binding; an accepted one would run forever, which these tests never
//    reach).
//
// 4. NO FALSE POSITIVES: the historical reason NOT to block was
//    "undefined function" on symbols merged from imported modules at
//    runtime. The gate resolves the import tree statically first (the
//    same file-lookup rule as the runtime loader), so an import-using
//    program still runs (pinned with examples/p5_modules.mlog —
//    std/string + std/math through aliased imports).
//
// 5. THE GOLDEN CORPUS: examples/p50_unknown_fn flipped from a .expected
//    contract (stdout = the liar string — the bug, enshrined) to a .error
//    contract (the gate refusal — the fix). The harness
//    (tests/golden.rs all_error_tests_pass) enforces it on every run.

use metalogos::interpreter::Interpreter;
use metalogos::semantic::{format_blocking_line, is_exempt_from_blocking, SemanticErrorKind};

/// The audit's exploit shape: a security gate over an undefined function.
/// If the interpreter returns the truthy liar string, AdminCheck returns
/// "ACCESS GRANTED" and the flow prints it — the bypass.
const ADMN_PROGRAM: &str = r#"
pattern AdminCheck(u: String) -> String {
  if is_admn(u) {
    return "ACCESS GRANTED"
  }
  return "denied"
}
entity who: String = "bob"
flow Main { input: String = who -> AdminCheck -> output }
"#;

/// №523-1: `mlog run` refuses the program at the gate — the then-branch
/// never executes (an Err carries no output), and the finding names the
/// undefined function.
#[test]
fn n523_run_blocks_unknown_function_in_condition() {
    let err = metalogos::run_program(ADMN_PROGRAM)
        .expect_err("run must refuse: is_admn is undefined — the liar string must not come back");
    assert!(
        err.contains("Naryad #523"),
        "the gate refusal must carry the наряд stamp: {}",
        err
    );
    assert!(
        err.contains("is_admn"),
        "the refusal must name the undefined function: {}",
        err
    );
    assert!(
        !err.contains("ACCESS GRANTED"),
        "no execution output may leak through the refusal: {}",
        err
    );
}

/// №523-2: the VM backend refuses at COMPILE time — `mlog compile` runs
/// the bytecode compiler, whose undefined-function error is the №479
/// coded diagnostic. Both backends reject the program before anything
/// executes.
#[test]
fn n523_vm_compile_blocks_unknown_function() {
    let err = metalogos::compile_program(ADMN_PROGRAM)
        .expect_err("compile must refuse: is_admn is undefined on the VM path too");
    assert!(
        err.contains("undefined function: is_admn"),
        "the compiler refusal must name the undefined function: {}",
        err
    );
}

/// №523-3: the TW interpreter's err-origin. Even when a caller bypasses
/// the lib-level gate (REPL, feed_line, library consumers running the
/// interpreter directly), the interpreter itself returns
/// [UNDEFINED_FUNCTION] — never the truthy liar string.
#[test]
fn n523_tw_interpreter_err_origin_not_liar_string() {
    let declarations = metalogos::parser::parse(ADMN_PROGRAM)
        .expect("the exploit program parses (the failure is semantic, not syntactic)");
    let mut interp = Interpreter::new();
    let out = interp.run(declarations);
    match out {
        Ok(value) => panic!(
            "the interpreter must not succeed on an unknown function; got {:?}",
            value
        ),
        Err(err) => {
            assert!(
                err.contains("[UNDEFINED_FUNCTION]"),
                "the err-origin must carry the №479 stable code: {}",
                err
            );
            assert!(
                err.contains("is_admn"),
                "the err-origin must name the undefined function: {}",
                err
            );
            assert!(
                !err.starts_with("[ERROR:"),
                "the liar-string form is banned: {}",
                err
            );
        }
    }
}

/// №523-3b: the same err-origin at the `run`-path call site — the second
/// liar-string origin (the call resolution inside `run`'s own statement
/// loop) carries the same contract. An entity initializer that calls a
/// pattern which names an unknown function must surface the coded error
/// from the declaration phase — no liar value, no success.
#[test]
fn n523_tw_run_path_call_site_err_origin() {
    let src = r#"
pattern Gate() -> String {
  return phantom_gate("x")
}
entity verdict: String = Gate()
"#;
    let declarations = metalogos::parser::parse(src).expect("parses: the failure is semantic");
    let mut interp = Interpreter::new();
    let out = interp.run(declarations);
    let err = out.expect_err("phantom_gate is undefined — the declaration phase must refuse");
    assert!(
        err.contains("[UNDEFINED_FUNCTION]"),
        "the run-path err-origin must carry the stable code: {}",
        err
    );
}

/// №523-4: the serve entry point refuses at STARTUP — no semantic pass
/// existed on this path before №523. The route body names the undefined
/// function OUTSIDE the respond sink (a trusted literal feeds the sink,
/// so the №391 category-A clearance passes and the refusal comes from
/// the №523 gate — the layering is the point).
#[cfg(feature = "server")]
#[tokio::test]
async fn n523_serve_startup_refuses_unknown_function() {
    let src = r#"
mlogserver {
  port: 0
  route "/check" method=GET {
    let verdict = is_admn("bob")
    respond("checked")
  }
}
"#;
    let err = metalogos_server::server::run_server(src)
        .await
        .expect_err("serve must refuse at startup: is_admn is undefined");
    let text = err.to_string();
    assert!(
        text.contains("Naryad #523"),
        "the serve refusal must carry the наряд stamp: {}",
        text
    );
    assert!(
        text.contains("is_admn"),
        "the serve refusal must name the undefined function: {}",
        text
    );
}

/// №523-4b: the serve gate also covers the PATTERN bodies a route could
/// call — the undefined function sits inside AdminCheck, not in the route
/// body. The route feeds the respond sink a trusted literal, so the
/// №391 category-A clearance passes and the №523 gate is the refuser.
#[cfg(feature = "server")]
#[tokio::test]
async fn n523_serve_startup_refuses_undefined_in_pattern_body() {
    let src = r#"
mlogserver {
  port: 0
  route "/check" method=GET {
    respond("ok")
  }
}
pattern AdminCheck(u: String) -> String {
  if is_admn(u) {
    return "ACCESS GRANTED"
  }
  return "denied"
}
"#;
    let err = metalogos_server::server::run_server(src)
        .await
        .expect_err("serve must refuse: the program's pattern body names is_admn");
    let text = err.to_string();
    assert!(
        text.contains("is_admn"),
        "the refusal must name the undefined function: {}",
        text
    );
}

/// №523-4: NO false positives — the historical non-blocking rationale was
/// imported symbols. An import-using program (aliased std/string +
/// std/math) still runs to completion through the gate.
#[test]
fn n523_imported_symbols_pass_the_gate() {
    // №567: the corpus lives at the repo root (the include path re-parented);
    // the std/ module corpus too — the base dir is pinned to it explicitly
    // (the CWD is the package dir now, not the repo root).
    let source = include_str!("../../examples/p5_modules.mlog");
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let out = metalogos::run_program_with_dir(source, repo_root)
        .expect("an import-using program must run: the gate resolves imports statically");
    let out = out.expect("p5_modules produces a flow output");
    assert!(
        out.contains("hello"),
        "the flow output must be the real result, not a refusal: {:?}",
        out
    );
}

/// №523-2: the №181 class still blocks — now as a subset of "every
/// semantic error", not as a substring special case.
#[test]
fn n523_distill_to_still_blocks() {
    let src = r#"
reflex FreeFormHead {
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["placeholder_to_avoid_grammar_error"]
  seed: 42
}
learnable pattern FreeForm(input: String) -> String {
  prompt: "generate"
  distill_to: NonExistentHead
  distill_after: 5
}
flow Main { input: String = "x" -> FreeForm -> output }
"#;
    let err = metalogos::run_program(src)
        .expect_err("distill_to referencing an undeclared reflex must still block");
    assert!(
        err.contains("distill_to references"),
        "the refusal must name the distill_to finding: {}",
        err
    );
}

/// №523-2: the №392 class still blocks — same subset posture.
#[test]
fn n523_deny_still_blocks() {
    let src = r#"
on_deny(db) {
  match deny_reason() {
    "SECRET_TO_EXEC" then { print("deny:SECRET_TO_EXEC") }
  }
}
flow Main { input: String = "x" -> output }
"#;
    let err = metalogos::run_program(src)
        .expect_err("a non-exhaustive deny match must still block the run path");
    assert!(
        err.contains("[DENY_MATCH_EXHAUSTIVE]"),
        "the refusal must carry the deny finding: {}",
        err
    );
}

/// №523-2: the exemption list is the ONE explicit place — and it is empty
/// today. Every kind blocks. If a future наряд adds an exemption, this
/// test is the visible ratchet it must argue against.
#[test]
fn n523_exemption_list_is_empty_and_explicit() {
    assert!(
        !is_exempt_from_blocking(SemanticErrorKind::UndefinedFunction),
        "UNDEFINED_FUNCTION blocks — the audit's exploit class"
    );
    assert!(
        !is_exempt_from_blocking(SemanticErrorKind::Other),
        "Other blocks — every semantic finding blocks by default"
    );
}

/// №523-2: one formatting rule for the gate's per-finding lines — the
/// line suffix only when the span is known (line 0 is the unknown-span
/// fallback; "(line 0)" would be a lie of its own).
#[test]
fn n523_blocking_line_formatting() {
    let known = metalogos::semantic::SpannedError::at(
        "finding with a place",
        metalogos::ast::Span::new(7, 0, 7, 5),
    );
    assert_eq!(
        format_blocking_line(&known),
        "finding with a place (line 7)"
    );
    let unknown = metalogos::semantic::SpannedError::at(
        "finding without a place",
        metalogos::ast::Span::unknown(),
    );
    assert_eq!(format_blocking_line(&unknown), "finding without a place");
}
