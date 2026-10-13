// ── Contract tests for Tool Abstraction (ADR-0054) ─────────────────────
//
// Tests:
// 1. tool with two methods — basic math_api double/square (contract from spec)
// 2. tool method with string operations
// 3. tool namespace isolation — same method name in different tools
// 4. tool with no methods — empty tool is valid
// 5. tool method calling a regular pattern
// 6. tool with multiple params
// 7. tool method used in flow pipeline
// 8. undefined tool — error on QualifiedCall
// 9. tool with 3 methods (telegram-like)

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run declarations, return interpreter for inspection.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

/// Helper: parse source, run, and get flow output (returned from run()).
fn get_flow_output(source: &str) -> Result<String, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    let output = interp.run(declarations)?;
    output.ok_or_else(|| "no flow output".to_string())
}

// ── Test 1: tool with two methods — double and square (contract) ───────

#[test]
fn test_tool_basic_double_square() {
    let source = r#"
        tool math_api {
            double(n: Float) -> Float { return n + n }
            square(n: Float) -> Float { return n * n }
        }
        entity x: Float = 5.0
        pattern Test(n: Float) -> String {
            let d = math_api.double(n)
            let s = math_api.square(n)
            return to_string(d) + " " + to_string(s)
        }
        flow Main { input: Float = x -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "10 25");
}

// ── Test 2: tool method with string operations ────────────────────────

#[test]
fn test_tool_string_methods() {
    let source = r#"
        tool str_api {
            greet(name: String) -> String {
                return "Hello, " + name + "!"
            }
            dup(text: String) -> String {
                return text + " " + text
            }
        }
        pattern Test(name: String) -> String {
            let greeting = str_api.greet(name)
            let doubled = str_api.dup(greeting)
            return doubled
        }
        flow Main { input: String = "world" -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "Hello, world! Hello, world!");
}

// ── Test 3: tool namespace isolation ────────────────────────────────

#[test]
fn test_tool_namespace_isolation() {
    // NOTE: tool names must not start with "tool" since "tool" is a keyword.
    // Use calc_a / calc_b instead.
    let source = r#"
        tool calc_a {
            compute(x: Float) -> Float { return x + 1.0 }
        }
        tool calc_b {
            compute(x: Float) -> Float { return x * 2.0 }
        }
        pattern Test(x: Float) -> String {
            let a = calc_a.compute(x)
            let b = calc_b.compute(x)
            return to_string(a) + " " + to_string(b)
        }
        flow Main { input: Float = 3.0 -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "4 6");
}

// ── Test 4: tool with no methods — valid but unusable ────────────────

#[test]
fn test_tool_empty() {
    // An empty tool should parse and run without error
    let source = r#"
        tool empty_tool {
        }
    "#;

    let result = run_source(source);
    assert!(result.is_ok());
}

// ── Test 5: tool method calling a regular pattern ───────────────────

#[test]
fn test_tool_method_calls_pattern() {
    let source = r#"
        pattern helper(n: Float) -> Float {
            return n * 10.0
        }
        tool calc {
            scaled(n: Float) -> Float {
                let h = helper(n)
                return h + 1.0
            }
        }
        pattern Test(n: Float) -> String {
            let r = calc.scaled(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "51");
}

// ── Test 6: tool with multiple params ────────────────────────────────

#[test]
fn test_tool_multi_param_flow() {
    let source = r#"
        tool math_api {
            add(a: Float, b: Float) -> Float { return a + b }
            mul(a: Float, b: Float) -> Float { return a * b }
        }
        pattern Sum(a: Float) -> String {
            let s = math_api.add(a, 7.0)
            return to_string(s)
        }
        flow Main { input: Float = 3.0 -> Sum -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "10");
}

// ── Test 7: tool method used in flow pipeline step ────────────────────

#[test]
fn test_tool_in_flow_pipeline() {
    let source = r#"
        tool calc {
            double(n: Float) -> Float { return n * 2.0 }
        }
        pattern DoubleIt(n: Float) -> Float {
            return calc.double(n)
        }
        pattern Format(n: Float) -> String {
            return to_string(n)
        }
        flow Main { input: Float = 4.0 -> DoubleIt -> Format -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "8");
}

// ── Test 8: undefined tool — error on QualifiedCall ──────────────────

#[test]
fn test_tool_undefined_error() {
    let source = r#"
        pattern Test(n: Float) -> String {
            let r = nonexistent.do_something(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;

    let result = get_flow_output(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // QualifiedCall checks module_namespaces first
    assert!(
        err.contains("undefined module"),
        "expected 'undefined module' in error, got: {}",
        err
    );
}

// ── Test 9: tool with 3 methods (telegram-like) ────────────────────

#[test]
fn test_tool_three_methods() {
    let source = r#"
        tool api {
            base_url() -> String { return "https://example.com" }
            full_url(path: String) -> String {
                let base = api.base_url()
                return base + path
            }
            status() -> String { return "ok" }
        }
        pattern Test(path: String) -> String {
            let url = api.full_url(path)
            let st = api.status()
            return url + " " + st
        }
        flow Main { input: String = "/api" -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "https://example.com/api ok");
}

// ─────────────────────────────────────────────────────────────────────────
// №684 (В43): VM leg — the SAME contracts on the bytecode backend (§2.4).
//
// Before №684 the compiler had no tool branch at all: `Declaration::Tool`
// was skipped in both passes (compiler.rs «Phase 6+: handled elsewhere»)
// and the QualifiedCall resolve had no tool-namespace branch, so the VM
// could not even COMPILE a tool call — the В42 no-progress flag (gh#1233,
// comment 6146003277) pinned it with the minimal repro below:
//   mlog check → OK · TW run → 42 · VM compile →
//   error: compile: qualified call 'calc.double()' — not found
//
// ADR-0054 Implementation Strategy is now mirrored in the compiler: tool
// methods compile as regular patterns under qualified keys ("tool.method")
// and the QualifiedCall resolve carries a tool branch between the
// qualified-builtin and the flat-pattern priorities. The VM leg runs every
// contract source through the compile → Vm::run pipeline and asserts the
// transcript is IDENTICAL to the interpreter's (§2.4 parity rule).
// ─────────────────────────────────────────────────────────────────────────

/// Execute via bytecode VM (the tests/crosscheck_backends.rs shape).
/// Normalized to the TW helper's contract: no flow output → empty string.
fn run_vm_output(source: &str) -> Result<String, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::new();
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map(|out| out.unwrap_or_default())
}

/// Contract helper: the same source on BOTH backends must produce the
/// same flow output (§2.4 — the transcript is identical).
fn assert_both_backends(source: &str, expected: &str) {
    let tw = get_flow_output(source).unwrap_or_else(|e| panic!("TW failed: {}", e));
    let vm = run_vm_output(source).unwrap_or_else(|e| panic!("VM failed: {}", e));
    assert_eq!(tw, expected, "TW transcript diverged from the contract");
    assert_eq!(vm, expected, "VM transcript diverged from the contract");
    assert_eq!(tw, vm, "backend divergence on a tool contract (§2.4)");
}

// ── VM contract 1: double/square math contract ──────────────────────────

#[test]
fn test_tool_vm_basic_double_square() {
    assert_both_backends(
        r#"
        tool math_api {
            double(n: Float) -> Float { return n + n }
            square(n: Float) -> Float { return n * n }
        }
        entity x: Float = 5.0
        pattern Test(n: Float) -> String {
            let d = math_api.double(n)
            let s = math_api.square(n)
            return to_string(d) + " " + to_string(s)
        }
        flow Main { input: Float = x -> Test -> output }
    "#,
        "10 25",
    );
}

// ── VM contract 2: string operations in tool methods ────────────────────

#[test]
fn test_tool_vm_string_methods() {
    assert_both_backends(
        r#"
        tool str_api {
            greet(name: String) -> String {
                return "Hello, " + name + "!"
            }
            dup(text: String) -> String {
                return text + " " + text
            }
        }
        pattern Test(name: String) -> String {
            let greeting = str_api.greet(name)
            let doubled = str_api.dup(greeting)
            return doubled
        }
        flow Main { input: String = "world" -> Test -> output }
    "#,
        "Hello, world! Hello, world!",
    );
}

// ── VM contract 3: namespace isolation (same method, different tools) ───

#[test]
fn test_tool_vm_namespace_isolation() {
    assert_both_backends(
        r#"
        tool calc_a {
            compute(x: Float) -> Float { return x + 1.0 }
        }
        tool calc_b {
            compute(x: Float) -> Float { return x * 2.0 }
        }
        pattern Test(x: Float) -> String {
            let a = calc_a.compute(x)
            let b = calc_b.compute(x)
            return to_string(a) + " " + to_string(b)
        }
        flow Main { input: Float = 3.0 -> Test -> output }
    "#,
        "4 6",
    );
}

// ── VM contract 4: empty tool compiles and runs on the VM ───────────────

#[test]
fn test_tool_vm_empty() {
    let source = r#"
        tool empty_tool {
        }
    "#;
    let vm = run_vm_output(source);
    assert!(vm.is_ok(), "VM failed on an empty tool: {:?}", vm.err());
    let tw = run_source(source);
    assert!(tw.is_ok(), "TW failed on an empty tool: {:?}", tw.err());
}

// ── VM contract 5: tool method calling a regular pattern ────────────────

#[test]
fn test_tool_vm_method_calls_pattern() {
    assert_both_backends(
        r#"
        pattern helper(n: Float) -> Float {
            return n * 10.0
        }
        tool calc {
            scaled(n: Float) -> Float {
                let h = helper(n)
                return h + 1.0
            }
        }
        pattern Test(n: Float) -> String {
            let r = calc.scaled(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#,
        "51",
    );
}

// ── VM contract 6: multi-param tool methods ──────────────────────────────

#[test]
fn test_tool_vm_multi_param_flow() {
    assert_both_backends(
        r#"
        tool math_api {
            add(a: Float, b: Float) -> Float { return a + b }
            mul(a: Float, b: Float) -> Float { return a * b }
        }
        pattern Sum(a: Float) -> String {
            let s = math_api.add(a, 7.0)
            return to_string(s)
        }
        flow Main { input: Float = 3.0 -> Sum -> output }
    "#,
        "10",
    );
}

// ── VM contract 7: tool method as an intermediate flow step ─────────────

#[test]
fn test_tool_vm_in_flow_pipeline() {
    assert_both_backends(
        r#"
        tool calc {
            double(n: Float) -> Float { return n * 2.0 }
        }
        pattern DoubleIt(n: Float) -> Float {
            return calc.double(n)
        }
        pattern Format(n: Float) -> String {
            return to_string(n)
        }
        flow Main { input: Float = 4.0 -> DoubleIt -> Format -> output }
    "#,
        "8",
    );
}

// ── VM contract 8: undefined module — loud on BOTH backends ─────────────
// Pre-existing negative-path contract: the error TEXTS diverge by design
// (TW: runtime "undefined module"; VM: compile-time "not found") — the
// assertion is that neither backend is silent.

#[test]
fn test_tool_vm_undefined_module_loud() {
    let source = r#"
        pattern Test(n: Float) -> String {
            let r = nonexistent.do_something(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;
    let tw = get_flow_output(source);
    assert!(tw.is_err(), "TW must fail loudly on an undefined module");
    let vm = run_vm_output(source);
    assert!(vm.is_err(), "VM must fail loudly on an undefined module");
}

// ── VM contract 9: three methods with inter-method calls (ADR-0054
//    «Internal Method Calls» — the qualified form inside tool bodies) ────

#[test]
fn test_tool_vm_three_methods() {
    assert_both_backends(
        r#"
        tool api {
            base_url() -> String { return "https://example.com" }
            full_url(path: String) -> String {
                let base = api.base_url()
                return base + path
            }
            status() -> String { return "ok" }
        }
        pattern Test(path: String) -> String {
            let url = api.full_url(path)
            let st = api.status()
            return url + " " + st
        }
        flow Main { input: String = "/api" -> Test -> output }
    "#,
        "https://example.com/api ok",
    );
}

// ── VM contract 10 (№684): unknown method on a KNOWN tool — the loud
//    mirror error on both backends (interpreter: execution.rs
//    «undefined method '<m>' in tool '<t>'»; compiler: the same wording
//    at compile time — never a fallthrough to flat patterns / builtins).

#[test]
fn test_tool_vm_unknown_method_loud() {
    let source = r#"
        tool calc {
            double(n: Float) -> Float { return n + n }
        }
        pattern Test(n: Float) -> String {
            let r = calc.nope(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;
    let tw = get_flow_output(source);
    assert!(tw.is_err(), "TW must fail loudly on an unknown tool method");
    assert!(
        tw.unwrap_err()
            .contains("undefined method 'nope' in tool 'calc'"),
        "TW error must mirror the ADR-0054 wording"
    );
    let vm = run_vm_output(source);
    assert!(vm.is_err(), "VM must fail loudly on an unknown tool method");
    assert!(
        vm.unwrap_err()
            .contains("undefined method 'nope' in tool 'calc'"),
        "VM compile error must mirror the ADR-0054 wording"
    );
}

// ── VM contract 11 (№684): the В42 no-progress flag's minimal repro
//    (gh#1233, comment 6146003277) — the golden both-backend pair.
//    Before №684: check OK · TW 42 · VM `qualified call 'calc.double()'
//    — not found`. After №684: 42 == 42, divergence eliminated in zero.

#[test]
fn test_tool_vm_repro_flag_gh1233() {
    assert_both_backends(
        r#"
        tool calc {
            double(n: Float) -> Float { return n + n }
        }
        pattern Agent(task: String) -> String {
            let d = calc.double(21.0)
            return to_string(d)
        }
        flow Main { input: String = "x" -> Agent -> output }
    "#,
        "42",
    );
}

// ── VM contract 12 (№684): the interpreter's dispatch-order mirror —
//    plain builtins SHADOW tool methods (execution.rs resolves the
//    builtin by function name BEFORE the is_tool pattern lookup), and
//    the runtime-op intercepts (query/db_execute/...) fire before the
//    tool namespace too. The compiler mirrors both: a tool method
//    sharing a builtin name is unreachable on BOTH backends — the same
//    runtime call, the same result (§2.4: probe-pinned on the В43 lane).

#[test]
fn test_tool_vm_builtin_shadowing_mirror() {
    let source = r#"
        tool calc {
            len(n: Float) -> Float { return n * 1000.0 }
        }
        pattern Test(n: Float) -> String {
            let r = calc.len(n)
            return to_string(r)
        }
        flow Main { input: Float = 2.0 -> Test -> output }
    "#;
    let tw = get_flow_output(source);
    let vm = run_vm_output(source);
    assert!(
        tw.is_err(),
        "TW must shadow the tool method with the plain builtin (len)"
    );
    assert!(
        vm.is_err(),
        "VM must mirror the shadowing — the same builtin call, not the tool body"
    );
    assert_eq!(
        tw.unwrap_err(),
        vm.unwrap_err(),
        "backend divergence on the builtin-shadowing mirror (§2.4)"
    );
}

#[test]
fn test_tool_vm_runtime_op_intercept_mirror() {
    let source = r#"
        tool calc {
            query(n: Float) -> Float { return n + n }
        }
        pattern Test(n: Float) -> String {
            let r = calc.query(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;
    let tw = get_flow_output(source);
    let vm = run_vm_output(source);
    assert!(
        tw.is_err(),
        "TW must intercept query() by function name before the tool lookup"
    );
    assert!(
        vm.is_err(),
        "VM must mirror the runtime-op intercept — the same db-op dispatch"
    );
    // Behavioral parity pinned: the SAME op fails on BOTH backends with
    // the same diagnostic head. The exact suffix diverges — a PRE-EXISTING
    // message-shape gap in the db-op dispatch itself (db_ops.rs TW path
    // ", got {}" vs the VM-path arm without the suffix; reachable through
    // a plain wrong-typed query() long before №684) — surfaced by the
    // mirror, NOT introduced by it; flagged to the conveyor for the
    // follow-up micro-naryad, out of this naryad's verbatim scope.
    assert!(
        tw.unwrap_err().contains("query() expected String SQL"),
        "TW diagnostic head must name the intercepted op"
    );
    assert!(
        vm.unwrap_err().contains("query() expected String SQL"),
        "VM diagnostic head must name the intercepted op"
    );
}
