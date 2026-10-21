// ── gh#1336 (fosved-acc АСС-1.3/1.6): VM drops `each` nested in an `if` body ──
//
// The reported shape (v0.32.0, mlog serve, the DEFAULT vm backend since
// ADR-0171): an `each` inside an `if` body silently runs 0 times — no
// error, no warning. The same program on `interpreter` runs it. An `if`
// inside an `each` works on both. In the consumer the shape guards
// `db_execute`/`http_get` (incident confirmation) — per the №476 rule a
// silent control-flow divergence there is a blocking correctness class.
//
// Root cause (the fix PR carries the full map): `Statement::Each` /
// `EachWithIndex` compile ONLY in the pattern/route-body TOP-LEVEL
// compiler (`compile_pattern_body_with_locals`). The SHARED statement
// compiler (`compile_stmt_with_locals`) — the path every nested context
// takes (if/else branches, while bodies, each bodies, match arms) — had
// an explicit NO-OP for the forms ("№510: explicit no-op set"), so the
// nested loop compiled to NOTHING. The tree-walking interpreter executes
// the loop in every context → the silent TW/VM divergence.
//
// The matrix below pins the WHOLE class on both backends: each shape
// must (a) answer IDENTICALLY on TW and VM and (b) actually EXECUTE the
// loop (the accumulator is read after — a future "both sides drop it"
// regression cannot pass as parity).

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn norm(v: &Result<Option<String>, String>) -> String {
    match v {
        Ok(Some(s)) => s.trim().to_string(),
        Ok(None) => "<none>".to_string(),
        Err(e) => format!("ERR: {}", e),
    }
}

/// TW == VM AND the VM actually executed the loop (the expected value is
/// the loop's side effect; a dropped loop would answer the initial 0).
fn assert_parity_and_liveness(prog: &str, expected: &str, label: &str) {
    let tw = norm(&run_tw(prog));
    let vm = norm(&run_vm(prog));
    assert_eq!(
        tw, vm,
        "{label}: TW/VM divergence (gh#1336 class) — tw={tw:?} vm={vm:?}"
    );
    assert_eq!(
        vm, expected,
        "{label}: the loop did not execute on the VM (the gh#1336 silent-drop shape) — vm={vm:?}"
    );
}

// 1. THE issue shape: each inside an if body, accumulator after.
const IF_EACH: &str = r#"
pattern IfEach(s: String) -> Float {
  let mut n = 0.0
  if s != "" {
    each x in split(s, "|") {
      n = n + 1.0
    }
  }
  return n
}
flow Main { input: String = "a|b|c" -> IfEach -> output }
"#;

#[test]
fn n1336_each_inside_if_body() {
    assert_parity_and_liveness(IF_EACH, "3", "each-in-if");
}

// 2. each inside the ELSE branch and inside an ELSE IF branch.
const EACH_ELSE: &str = r#"
pattern ElseEach(s: String) -> Float {
  let mut n = 0.0
  if s == "" {
    n = -1.0
  } else if s == "z" {
    n = -2.0
  } else {
    each x in split(s, "|") {
      n = n + 1.0
    }
  }
  return n
}
flow Main { input: String = "a|b|c" -> ElseEach -> output }
"#;

#[test]
fn n1336_each_inside_else_branch() {
    assert_parity_and_liveness(EACH_ELSE, "3", "each-in-else");
}

// 3. each inside a WHILE body (the nested-loop shape the fuzzer
//    generates unobservable — the class escaped the differential run).
const WHILE_EACH: &str = r#"
pattern WhileEach(_s: String) -> Float {
  let mut n = 0.0
  let g = 0
  while g < 2 {
    each x in ["a", "b", "c"] {
      n = n + 1.0
    }
    let g = g + 1
  }
  return n
}
flow Main { input: String = "x" -> WhileEach -> output }
"#;

#[test]
fn n1336_each_inside_while_body() {
    assert_parity_and_liveness(WHILE_EACH, "6", "each-in-while");
}

// 4. each directly nested in an each body.
const EACH_EACH: &str = r#"
pattern EachEach(_s: String) -> Float {
  let mut n = 0.0
  each row in ["r1", "r2"] {
    each cell in ["c1", "c2", "c3"] {
      n = n + 1.0
    }
  }
  return n
}
flow Main { input: String = "x" -> EachEach -> output }
"#;

#[test]
fn n1336_each_nested_in_each() {
    assert_parity_and_liveness(EACH_EACH, "6", "each-in-each");
}

// 5. each_with_index inside an if body (the twin form).
const EACH_IDX: &str = r#"
pattern IdxEach(s: String) -> Float {
  let mut n = 0.0
  if s != "" {
    each i, x in split(s, "|") {
      n = n + to_float(i)
    }
  }
  return n
}
flow Main { input: String = "a|b|c" -> IdxEach -> output }
"#;

#[test]
fn n1336_each_with_index_inside_if_body() {
    assert_parity_and_liveness(EACH_IDX, "3", "each_with_index-in-if");
}

// 6. break/continue INSIDE the nested each (the №658 loop-stack
//    fixups must resolve to the NEAREST loop — the nested one).
const EACH_BREAK: &str = r#"
pattern BreakEach(s: String) -> Float {
  let mut n = 0.0
  if s != "" {
    each x in split(s, "|") {
      if x == "b" { break }
      n = n + 1.0
    }
  }
  return n
}
flow Main { input: String = "a|b|c" -> BreakEach -> output }
"#;

#[test]
fn n1336_break_inside_each_in_if() {
    assert_parity_and_liveness(EACH_BREAK, "1", "break-in-each-in-if");
}

const EACH_CONTINUE: &str = r#"
pattern ContEach(s: String) -> Float {
  let mut n = 0.0
  if s != "" {
    each x in split(s, "|") {
      if x == "b" { continue }
      n = n + 1.0
    }
  }
  return n
}
flow Main { input: String = "a|b|c" -> ContEach -> output }
"#;

#[test]
fn n1336_continue_inside_each_in_if() {
    assert_parity_and_liveness(EACH_CONTINUE, "2", "continue-in-each-in-if");
}

// ── №766 (gh#1419, Wave 59): the THIRD form of the class — each inside a
// match ARM ────────────────────────────────────────────────────────────
//
// The external audit (METALOGOS_AUDIT_9fa9821.md, §3 H-2, §6 п.3): the
// №369 comment (the nested match) + gh#1336 (the nested each) name ONE
// class — «form A inside form B» — and the matrix had no each-in-match
// test (the "match arms" mention lived only in the header comment). The
// shared compile_stmt_with_locals now compiles every nested context, so
// this pin closes the last row of the audit's list: the match arm is
// the same nested context the if/while/each bodies are.

#[test]
fn n766_each_inside_match_arm_executes_on_both_backends() {
    let prog = r#"
pattern P(x: String) -> String {
  let mut acc = 0.0
  match x {
    "go" then {
      each item in ["a", "b"] {
        acc = acc + len(item)
      }
    }
    else {
      acc = 0.0
    }
  }
  return to_string(acc)
}
flow Main { input: String = "go" -> P -> output }
"#;
    // The accumulator is read AFTER the match: a dropped each (either
    // backend) answers 0 instead of 2 — the gh#1336 silent-drop shape
    // cannot pass as parity.
    assert_parity_and_liveness(prog, "2", "each-in-match-arm (№766)");
}

#[test]
fn n766_each_with_index_inside_match_arm_executes() {
    // The each-with-index form — the second loop form of the №17.3 pair —
    // through the same match-arm context, the index riding the sum.
    let prog = r#"
pattern P(x: String) -> String {
  let mut acc = 0.0
  match x {
    "go" then {
      each i, item in ["a", "bc", "def"] {
        acc = acc + len(item) + i
      }
    }
    else {
      acc = 0.0
    }
  }
  return to_string(acc)
}
flow Main { input: String = "go" -> P -> output }
"#;
    // 0 + 1 + 2 + (1 + 2 + 3) = 9
    assert_parity_and_liveness(prog, "9", "each-with-index-in-match-arm (№766)");
}
