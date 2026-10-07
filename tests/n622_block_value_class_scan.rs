// Naryad №622 (gh#1085): the class-scan of the implicit block value —
// the consumers OUTSIDE the loops (the №612 etalon applied to the rest
// of the grammar). The scan verdict table (the «consumer × semantics ×
// file fact» shape the naryad mandates):
//
// | Consumer | The implicit block value's fate | The file fact |
// |---|---|---|
// | `Statement::Match` arms + else (execution.rs) | DISCARDED — the arm blocks run through `eval_block!`, which (№612) replaces the running implicit value UNCONDITIONALLY and propagates ONLY the explicit Return | `eval_block!` macro, execution.rs |
// | `Statement::IfThen` (if WITHOUT else) body | DISCARDED — the same `eval_block!` path | execution.rs (the IfThen arm) |
// | `Statement::IfBlock` (if/else) branches | DISCARDED — the same `eval_block!` path | execution.rs |
// | `Expr::MatchExpr` arm blocks | DESIGNATED — the block's tail value IS the match expression's result (the expression channel cannot carry a control signal; a `return` inside the arm is captured as the block value — the documented divergence) | execution.rs, the MatchExpr eval |
// | `Declaration::Sandbox` "body" | N/A — the grammar has NO sandbox body: `SandboxDecl { span, name, allowed, forbidden, timeout }` (ast.rs) is a POLICY declaration; execution.rs registers it, nothing executes a statement block | ast.rs:909, execution.rs:228 |
// | `Declaration::Flow` "body" | N/A — the flow body is a PIPELINE of pattern invocations over values (`run_flow` in flow.rs); no statement block exists in `FlowDecl` | flow.rs:267 |
//
// THE ETALON (№612): the implicit block value NEVER becomes an early
// return of the enclosing function; the termination happens ONLY
// through the explicit return/respond. Every consumer above obeys it.
//
// The pins below run the SAME program through the production order of
// BOTH backends (the №629 file's idiom) and demand the identical
// output: a bare non-Unit call as the LAST statement of the construct
// must neither terminate the construct early nor the function; the
// code AFTER the construct must run.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let module_decls = metalogos::semantic::resolve_imports_statically(&declarations, base_dir)
        .map_err(|e| format!("Compilation error (Naryad #523): {}", e))?;
    let mut merged_decls = module_decls;
    merged_decls.extend(declarations.clone());
    let sem_result = metalogos::semantic::check_program(&merged_decls);
    let blocking: Vec<&metalogos::semantic::SpannedError> = sem_result
        .errors
        .iter()
        .filter(|err| !metalogos::semantic::is_exempt_from_blocking(err.kind))
        .collect();
    if !blocking.is_empty() {
        let code = blocking.iter().find_map(|err| err.kind.stable_code());
        let stamp = code.map(|c| format!("[{}] ", c)).unwrap_or_default();
        let lines: Vec<String> = blocking
            .iter()
            .map(|err| metalogos::semantic::format_blocking_line(err))
            .collect();
        return Err(format!(
            "Compilation error (Naryad #523): semantic findings block execution:\n{}{}",
            stamp,
            lines.join("\n")
        ));
    }
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(merged_decls)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_parity(name: &str, src: &str, expected: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tw = run_tw(src, &repo).unwrap_or_else(|e| panic!("{}: TW must run: {}", name, e));
    let vm = run_vm(src, &repo).unwrap_or_else(|e| panic!("{}: VM must run: {}", name, e));
    assert_eq!(
        tw.as_deref(),
        Some(expected),
        "{}: TW output mismatch (the etalon read)",
        name
    );
    assert_eq!(
        vm.as_deref(),
        Some(expected),
        "{}: VM output mismatch (the parity target)",
        name
    );
}

#[test]
fn n622_match_arm_bare_call_never_returns_early() {
    // The matched arm's body ENDS with the bare non-Unit call; the code
    // after the match must run; the function returns through the
    // explicit return ONLY.
    let src = r#"db { url: "sqlite::memory:" }
pattern p(x: String) -> String {
  match x {
    "a" then {
      len(x)
      let marker = "arm-done"
    }
    starts_with "z" then {
      to_string(len(x))
    }
    else {
      len("zz")
    }
  }
  return "tail:" + to_string(len(x))
}
flow Main {
  input: String = "a" -> p -> output
}
"#;
    assert_parity("match_arm_bare_call", src, "tail:1");
}

#[test]
fn n622_match_else_bare_call_never_returns_early() {
    let src = r#"db { url: "sqlite::memory:" }
pattern p(x: String) -> String {
  match x {
    "q" then {
      let m = "q"
    }
    else {
      len(x)
    }
  }
  return "tail:" + to_string(len(x))
}
flow Main {
  input: String = "a" -> p -> output
}
"#;
    assert_parity("match_else_bare_call", src, "tail:1");
}

#[test]
fn n622_if_without_else_bare_call_never_returns_early() {
    // The №612-class shape in the if-WITHOUT-else consumer: the body's
    // last statement is the bare call; the code after the if must run.
    let src = r#"db { url: "sqlite::memory:" }
pattern p(x: String) -> String {
  if x == "a" {
    len(x)
    let m = "hit"
  }
  return "tail:" + to_string(len(x))
}
flow Main {
  input: String = "a" -> p -> output
}
"#;
    assert_parity("if_without_else_bare_call", src, "tail:1");
}

#[test]
fn n622_if_block_branch_bare_call_never_returns_early() {
    let src = r#"db { url: "sqlite::memory:" }
pattern p(x: String) -> String {
  if x == "a" {
    len(x)
    let m = "eq"
  } else {
    len("zz")
    let m = "ne"
  }
  return "tail:" + to_string(len(x))
}
flow Main {
  input: String = "a" -> p -> output
}
"#;
    assert_parity("if_block_branch_bare_call", src, "tail:1");
}

#[test]
fn n622_match_expr_block_value_is_designated() {
    // The MatchExpr arm block's tail value IS the match's value (the
    // designated semantics) — the bare call's value flows into `v`,
    // NOT into a function return; the code after runs.
    let src = r#"db { url: "sqlite::memory:" }
pattern p(x: String) -> String {
  let v = match x {
    "a" then { len(x) }
    else { 0 }
  }
  return "v=" + to_string(v) + ":tail"
}
flow Main {
  input: String = "a" -> p -> output
}
"#;
    assert_parity("match_expr_designated", src, "v=1:tail");
}

#[test]
fn n622_match_expr_return_in_arm_is_captured_not_propagated() {
    // The documented MatchExpr divergence: a `return` inside the arm
    // body is captured as the block value — it does NOT terminate the
    // function (the expression channel carries values, not signals).
    let src = r#"db { url: "sqlite::memory:" }
pattern p(x: String) -> String {
  let v = match x {
    "a" then { return 5.0 }
    else { 0 }
  }
  return "v=" + to_string(v) + ":tail"
}
flow Main {
  input: String = "a" -> p -> output
}
"#;
    assert_parity("match_expr_return_captured", src, "v=5:tail");
}
