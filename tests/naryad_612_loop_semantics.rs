// ── Naryad №612 (issue #1061): loop semantics — P0 bugfix ───────────
//
// The bug (the Камертон matrix, reproduced on main c694b6d, TW):
//   - a loop whose body is (or starts with) a BARE CALL ran exactly ONE
//     iteration out of N: `eval_statements_cf` keeps the block's №13
//     implicit-return value in `last_expr_value`, the loop arms read a
//     non-Unit `ControlFlow::ContinueNormal(v)` from the body as an
//     early `Return(v)` from the ENCLOSING function — the first
//     iteration became the last one and the tail after the loop was
//     silently cut;
//   - the poison persisted through trailing assignments/bindings (the
//     Assign/LetBinding arms never reset the block's implicit value);
//   - in TEST bodies the harness flattened the fabricated `Return(v)`
//     into `Ok(v)` — the test came back GREEN VACUOUSLY: the tail
//     after the loop never executed and the asserts never ran (a
//     deliberately false assert stayed green).
//
// The fix:
//   1. the loop arms (Each / EachWithIndex / While) DISCARD the body's
//      implicit value — a bare call in the body never terminates the
//      loop, never cuts the tail, never becomes a Return;
//   2. a loop statement, a binding and an assignment produce NO value —
//      the block's implicit value resets (the poison class);
//   3. `eval_block!` replaces the running implicit value
//      UNCONDITIONALLY (a sub-block ending in a no-value statement
//      produces Unit);
//   4. the test harness evaluates bodies ControlFlow-PRESERVINGLY — a
//      `Return` in a test body is a LOUD failure (a test that returns
//      early proved nothing).
//
// Done-when (§3.6 of the naryad): R3 red-honest, R4 green AND executed
// (the mutation probe must go red), R5 without regression, R6 the full
// pass with i == 3.0, the matrix pinned by regression tests, the
// TW↔VM parity e2e (the VM runs loops with its own jump machinery —
// its extent is pinned here).

use metalogos::interpreter::TestResult;

/// Run all test blocks of a program through the TW interpreter harness.
fn run_tests(source: &str) -> Vec<TestResult> {
    metalogos::test_program_with_dir(source, std::path::PathBuf::from(".")).unwrap()
}

/// The shared matrix program: schema + markers table (№612 counter).
const MATRIX_SCHEMA: &str = r#"
db { url: "sqlite::memory:" }

schema app {
  table markers { id: Int primary_key auto_increment, tag: String }
}
"#;

/// R3: `each` with a BARE-CALL-ONLY body in a TEST body — full pass,
/// the assert after the loop actually runs (pre-№612: 1 insert, the
/// tail cut, the test green vacuously).
#[test]
fn n612_r3_each_bare_call_body_full_pass() {
    let src = format!(
        "{}\ntest \"R3\" {{\n  each x in [1.0, 2.0, 3.0] {{\n    db_insert(\"markers\", {{ tag: \"m\" }})\n  }}\n  let rows = query(\"SELECT COUNT(*) AS n FROM markers\", [])\n  let first = get(rows, 0)\n  assert_eq(first.n, 3.0)\n}}\n",
        MATRIX_SCHEMA
    );
    let results = run_tests(&src);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].passed,
        "R3 must pass honestly (3 iterations), got: {:?}",
        results[0].error
    );
}

/// R4 (the mutation probe): the SAME program with a deliberately false
/// assert must go RED — proving the assert actually executed. The
/// pre-№612 vacuous green kept this red-honest probe green too.
#[test]
fn n612_r4_mutation_probe_false_assert_goes_red() {
    let src = format!(
        "{}\ntest \"R4\" {{\n  each x in [1.0, 2.0, 3.0] {{\n    db_insert(\"markers\", {{ tag: \"m\" }})\n  }}\n  let rows = query(\"SELECT COUNT(*) AS n FROM markers\", [])\n  let first = get(rows, 0)\n  assert_eq(first.n, 999.0)\n}}\n",
        MATRIX_SCHEMA
    );
    let results = run_tests(&src);
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].passed,
        "R4: the deliberately false assert MUST go red (the assert executed)"
    );
    assert!(
        results[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("assert_eq failed"),
        "the failure must be the honest assertion error, got: {:?}",
        results[0].error
    );
}

/// R5: the assignment body (`acc = acc + x`) — the pre-existing full
/// pass MUST NOT regress.
#[test]
fn n612_r5_assignment_body_no_regression() {
    let src = r#"
test "R5" {
  let mut acc = 0.0
  each x in [10.0, 20.0, 30.0] {
    acc = acc + x
  }
  assert_eq(acc, 60.0)
}
"#;
    let results = run_tests(src);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].passed,
        "R5 must pass without regression, got: {:?}",
        results[0].error
    );
}

/// R6: `while` with a bare call EARLIER in the body and a trailing
/// assignment — the terminating assignment resets the body's implicit
/// value (the matrix «отравляющее значение даёт голый вызов ранее в
/// теле»), the loop runs the full pass, `i` reaches 3.0.
#[test]
fn n612_r6_while_poison_body_full_pass() {
    let src = format!(
        "{}\ntest \"R6\" {{\n  let mut i = 0.0\n  while i < 3.0 {{\n    db_insert(\"markers\", {{ tag: \"m\" }})\n    i = i + 1.0\n  }}\n  assert_eq(i, 3.0)\n}}\n",
        MATRIX_SCHEMA
    );
    let results = run_tests(&src);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].passed,
        "R6 must pass (full pass, i == 3.0), got: {:?}",
        results[0].error
    );
}

/// The tail after the loop EXECUTES: the table created after the loop
/// exists (pre-№612 the tail was silently cut and the table never
/// appeared).
#[test]
fn n612_tail_after_loop_executes() {
    let src = format!(
        "{}\ntest \"tail\" {{\n  each x in [1.0, 2.0] {{\n    db_insert(\"markers\", {{ tag: \"m\" }})\n  }}\n  db_execute(\"CREATE TABLE tail_check (id INTEGER PRIMARY KEY, note TEXT)\", [])\n  db_insert(\"tail_check\", {{ note: \"t\" }})\n  let rows = query(\"SELECT COUNT(*) AS n FROM tail_check\", [])\n  let first = get(rows, 0)\n  assert_eq(first.n, 1.0)\n}}\n",
        MATRIX_SCHEMA
    );
    let results = run_tests(&src);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].passed,
        "the tail after the loop must execute, got: {:?}",
        results[0].error
    );
}

/// №612 harness rule: an explicit `return` in a test body is a LOUD
/// failure — a body that terminated early proved nothing. (The corpus
/// scan on main 4fca8df found ZERO test bodies using explicit return —
/// the rule breaks nothing, it closes the vacuum class.)
#[test]
fn n612_explicit_return_in_test_body_is_loud() {
    let src = r#"
test "early return" {
  let x = 1.0
  return x
  assert_eq(x, 999.0)
}
"#;
    let results = run_tests(src);
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].passed,
        "a test body with an explicit return MUST fail loudly"
    );
    assert!(
        results[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("terminated early"),
        "the failure must name the early termination, got: {:?}",
        results[0].error
    );
}

/// №612 pattern-level: a pattern whose loop is followed by a `return`
/// statement returns the HONEST computed value (pre-№612 the loop
/// fabricated an early Return carrying the FIRST bare-call value —
/// the count query never ran and the pattern returned `1.0`, the first
/// inserted id).
#[test]
fn n612_pattern_returns_the_honest_value_after_a_loop() {
    let src = format!(
        "{}\npattern Count(x: Float) -> Float {{\n  each y in [1.0, 2.0, 3.0] {{\n    db_insert(\"markers\", {{ tag: \"m\" }})\n  }}\n  let rows = query(\"SELECT COUNT(*) AS n FROM markers\", [])\n  let first = get(rows, 0)\n  return first.n\n}}\nflow Main {{ input: Float = 0.0 -> Count -> output }}\n",
        MATRIX_SCHEMA
    );
    // TW
    let tw = metalogos::run_program_with_dir(&src, std::path::PathBuf::from(".")).unwrap();
    let tw = tw.unwrap_or_default().trim_end().to_string();
    assert_eq!(tw, "3", "TW must return the honest count 3, got: {}", tw);
}

/// №612 TW↔VM parity e2e (the №607 precedent): the SAME program on the
/// bytecode VM. The VM compiles loops with its own jump machinery — its
/// extent is PINNED here: the honest count 3 on BOTH backends.
#[test]
fn n612_tw_vm_parity_loop_matrix() {
    let src = format!(
        "{}\npattern Count(x: Float) -> Float {{\n  each y in [1.0, 2.0, 3.0] {{\n    db_insert(\"markers\", {{ tag: \"m\" }})\n  }}\n  let rows = query(\"SELECT COUNT(*) AS n FROM markers\", [])\n  let first = get(rows, 0)\n  return first.n\n}}\nflow Main {{ input: Float = 0.0 -> Count -> output }}\n",
        MATRIX_SCHEMA
    );
    // VM: parse → compile → run
    let declarations = metalogos::parser::parse(&src).unwrap();
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations).unwrap();
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(program).unwrap();
    let vm_out = vm_out.unwrap_or_default().trim_end().to_string();
    assert_eq!(
        vm_out, "3",
        "VM parity: the VM must return the honest count 3, got: {}",
        vm_out
    );

    // TW (same program, same expectation)
    let tw = metalogos::run_program_with_dir(&src, std::path::PathBuf::from(".")).unwrap();
    let tw = tw.unwrap_or_default().trim_end().to_string();
    assert_eq!(tw, "3", "TW parity: got: {}", tw);
}
