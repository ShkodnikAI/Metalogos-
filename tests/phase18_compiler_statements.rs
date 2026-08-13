// ── Наряд №18: Bytecode compiler — full statement compilation ─────────
// Tests for IfElseBlock, Each, EachWithIndex, While, Assign, Match,
// ExprStmt, IfThen, Break, Continue in bytecode path.

use metalogos::bytecode::*;
use metalogos::compiler::Compiler;
use metalogos::interpreter::Value;
use metalogos::parser;
use metalogos::vm::Vm;

/// Helper: parse + compile source to a Program, then run on VM.
fn compile_and_run(source: &str) -> Result<Option<String>, String> {
    let decls = parser::parse(source).map_err(|e| format!("parse: {}", e))?;
    let mut compiler = Compiler::new();
    let program = compiler.compile(decls)?;
    let mut vm = Vm::new();
    vm.run(program)
}

/// Helper: compile and verify it succeeds (no error).
fn compile_ok(source: &str) {
    let decls = parser::parse(source)
        .map_err(|e| format!("parse: {}", e))
        .unwrap();
    let mut compiler = Compiler::new();
    compiler.compile(decls).expect("compile should succeed");
}

// ── З-18.0: New instructions (MakeList, ListLen, Pop) ─────────────

#[test]
fn test_make_list_instruction() {
    let mut vm = Vm::new();
    let program = Program {
        globals: vec![],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        db_url: None,
        schema_ddl: vec![],
        main_code: vec![
            Instruction::Const(Value::String("a".to_string())),
            Instruction::Const(Value::String("b".to_string())),
            Instruction::Const(Value::String("c".to_string())),
            Instruction::MakeList(3),
            Instruction::Halt,
        ],
        collections_loaded: false,
    };
    let result = vm.run(program).expect("vm run should succeed");
    // No flow output, but list was created on stack (discarded by Halt)
    assert!(result.is_none());
}

#[test]
fn test_list_len_instruction() {
    let mut vm = Vm::new();
    let program = Program {
        globals: vec![],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        db_url: None,
        schema_ddl: vec![],
        main_code: vec![
            Instruction::Const(Value::String("a".to_string())),
            Instruction::Const(Value::String("b".to_string())),
            Instruction::MakeList(2),
            Instruction::ListLen,
            Instruction::Halt,
        ],
        collections_loaded: false,
    };
    let result = vm.run(program).expect("vm run should succeed");
    assert!(result.is_none());
}

#[test]
fn test_pop_instruction() {
    let mut vm = Vm::new();
    let program = Program {
        globals: vec![],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        db_url: None,
        schema_ddl: vec![],
        main_code: vec![
            Instruction::Const(Value::String("x".to_string())),
            Instruction::Pop,
            Instruction::Const(Value::String("y".to_string())),
            Instruction::Halt,
        ],
        collections_loaded: false,
    };
    let result = vm.run(program).expect("vm run should succeed");
    assert!(result.is_none());
}

// ── З-18.1: IfElseBlock compilation ────────────────────────────────

#[test]
fn test_compile_ifelse_block() {
    let source = r#"
pattern check_flag(x: Float) -> String {
    if x > 0.0 {
        return "positive"
    } else {
        return "non-positive"
    }
}
entity val: String = check_flag(5.0)
"#;
    compile_ok(source);
}

#[test]
fn test_compile_ifelse_elseif() {
    let source = r#"
pattern classify(x: Float) -> String {
    if x > 10.0 {
        return "big"
    } else if x > 5.0 {
        return "medium"
    } else {
        return "small"
    }
}
"#;
    compile_ok(source);
}

// ── З-18.2: Each compilation ───────────────────────────────────────

#[test]
fn test_compile_each_loop() {
    let source = r#"
pattern sum_list(items: List) -> Float {
    let mut total = 0.0
    each item in items {
        total = total + item
    }
    return total
}
"#;
    compile_ok(source);
}

#[test]
#[ignore = "TODO: VM compiler does not yet support process-style declarations (legacy syntax)"]
fn test_compile_each_empty_list() {
    let source = r#"
process_empty() {
    let mut count = 0.0
    each item in ["nothing"] {
        count = 1.0
    }
    print(count)
}
"#;
    compile_ok(source);
}

// ── З-18.3: While compilation ──────────────────────────────────────

#[test]
fn test_compile_while_loop() {
    let source = r#"
pattern countdown(n: Float) -> String {
    let mut result = ""
    while n > 0.0 {
        result = str(n)
        n = n - 1.0
    }
    return result
}
"#;
    compile_ok(source);
}

// ── З-18.4: Assign compilation ─────────────────────────────────────

#[test]
fn test_compile_assign_local() {
    let source = r#"
pattern assign_test(x: Float) -> Float {
    let mut y = x
    y = y + 1.0
    return y
}
"#;
    compile_ok(source);
}

// ── З-18.5: Match compilation ──────────────────────────────────────

#[test]
#[ignore = "TODO: VM compiler does not yet support match with string literal arms"]
fn test_compile_match_exact() {
    let source = r#"
pattern greet(name: String) -> String {
    match name {
        "world" then { return "Hello, world!" }
        "Metalogos" then { return "Welcome to Metalogos!" }
        else { return "Hello, " + name }
    }
}
"#;
    compile_ok(source);
}

#[test]
#[ignore = "TODO: VM compiler does not yet support match contains"]
fn test_compile_match_contains() {
    let source = r#"
pattern classify_msg(msg: String) -> String {
    match msg {
        contains "error" then { return "ERROR" }
        contains "warn" then { return "WARN" }
        else { return "INFO" }
    }
}
"#;
    compile_ok(source);
}

#[test]
#[ignore = "TODO: VM compiler does not yet support match with compare operators"]
fn test_compile_match_compare() {
    let source = r#"
pattern rating_level(score: Float) -> String {
    match score {
        > 90.0 then { return "A" }
        > 80.0 then { return "B" }
        > 70.0 then { return "C" }
        else { return "F" }
    }
}
"#;
    compile_ok(source);
}

// ── З-18.6: QualifiedCall compilation ───────────────────────────────

#[test]
fn test_compile_qualified_call() {
    let source = r#"
pattern helper(x: String) -> String {
    return upper(x)
}

pattern main_fn(x: String) -> String {
    return std.string.upper(x)
}
"#;
    // QualifiedCall with module prefix — should resolve to builtin upper
    compile_ok(source);
}

// ── З-18.7: ExprStmt compilation ───────────────────────────────────

#[test]
fn test_compile_expr_stmt() {
    let source = r#"
pattern with_side_effect(x: String) -> String {
    print(x)
    return "done"
}
"#;
    compile_ok(source);
}

// ── З-18.8: IfThen compilation ─────────────────────────────────────

#[test]
fn test_compile_if_then() {
    let source = r#"
pattern maybe_double(x: Float) -> Float {
    let mut result = x
    if x > 0.0 then {
        result = x * 2.0
    }
    return result
}
"#;
    compile_ok(source);
}

// ── З-18.9: EachWithIndex compilation ──────────────────────────────

#[test]
fn test_compile_each_with_index() {
    let source = r#"
pattern indexed_items(items: List) -> String {
    let mut result = ""
    each i, item in items {
        if i == 1.0 {
            result = item
        }
    }
    return result
}
"#;
    compile_ok(source);
}

// ── З-18.10: Break/Continue in bytecode ───────────────────────────

#[test]
fn test_compile_break_in_each() {
    let source = r#"
pattern find_second(items: List) -> String {
    let mut result = "none"
    let mut count = 0.0
    each item in items {
        count = count + 1.0
        if count == 2.0 {
            result = item
            break
        }
    }
    return result
}
"#;
    compile_ok(source);
}

#[test]
fn test_compile_continue_in_each() {
    let source = r#"
pattern skip_first(items: List) -> String {
    let mut result = ""
    let mut i = 0.0
    each item in items {
        if i == 0.0 {
            i = i + 1.0
            continue
        }
        result = item
        i = i + 1.0
    }
    return result
}
"#;
    compile_ok(source);
}

#[test]
fn test_compile_break_in_while() {
    let source = r#"
pattern loop_until(n: Float) -> Float {
    let mut i = 0.0
    while 1.0 > 0.0 {
        i = i + 1.0
        if i >= n {
            break
        }
    }
    return i
}
"#;
    compile_ok(source);
}

// ── З-18.11: MakeList in expression compilation ────────────────────

#[test]
fn test_compile_list_expr_make_list() {
    let source = r#"
pattern make_list_test() -> String {
    let items = ["a", "b", "c"]
    return get(items, 1.0)
}
"#;
    compile_ok(source);
}

// ── З-18.12: Builtin index consistency ─────────────────────────────

#[test]
fn test_compiler_vm_builtin_index_match() {
    // Verify that compiler and VM have the same builtin count
    let source = r#"
pattern use_upper(x: String) -> String {
    return upper(x)
}
"#;
    let decls = parser::parse(source).unwrap();
    let mut compiler = Compiler::new();
    let program = compiler.compile(decls).unwrap();
    // If indices don't match, VM will call wrong builtin or panic
    let mut vm = Vm::new();
    let result = vm.run(program);
    assert!(
        result.is_ok(),
        "VM should run successfully with matching builtin indices"
    );
}

// ── З-18.13: Full compile+run for simple pattern ───────────────────

#[test]
fn test_compile_run_simple_pattern() {
    let source = r#"
pattern add(a: Float, b: Float) -> Float {
    return a + b
}
entity result: String = str(add(3.0, 4.0))
"#;
    let result = compile_and_run(source);
    assert!(
        result.is_ok(),
        "compile+run should succeed: {:?}",
        result.err()
    );
}
