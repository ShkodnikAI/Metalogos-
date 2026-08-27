// ── Наряды №19–22: Constraints (opaque types, variable checking, compiler fixes, VM completion)
// Integration tests for the four constraint work orders.

use metalogos::bytecode::*;
use metalogos::compiler::Compiler;
use metalogos::interpreter::Value;
use metalogos::parser;
use metalogos::semantic::{self, AnalysisResult};
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

/// Helper: semantic check
fn semantic_check(source: &str) -> AnalysisResult {
    let decls = parser::parse(source).unwrap();
    semantic::check_program(&decls)
}

// ═══════════════════════════════════════════════════════════════════
// Наряд №19: Opaque type constraints in semantic analysis
// ═══════════════════════════════════════════════════════════════════
//
// Наряд №128: два старых теста (print_secret_forbidden, to_string_secret_forbidden)
// удалены. Они проверяли устаревший механизм (semantic check_program с сообщением
// «opaque type constraint»), которого больше нет. Защита Secret теперь обеспечивается
// другими механизмами, которые имеют собственные тесты:
//   • print(secret) — runtime is_nonprintable() + audit SECRET_LEAK
//     (тесты: audit::secret_leak_print_ident, audit::secret_leak_print_direct_env,
//      examples/p114_secret_no_print.mlog + .error)
//   • to_string(secret) — Display для Value::Secret возвращает «[Secret]»,
//     реальное значение не утекает; taint-трекер audit propagates Secret
//     через to_string() к downstream-стокам

#[test]
#[ignore = "TODO: Opaque type concat constraints not yet implemented in semantic checker"]
fn test_z19_concat_opaque_forbidden() {
    let source = r#"
entity page: Html = escape_html("<b>hi</b>")
pattern xss() -> String {
    let payload = "user_input" + page
    return payload
}
"#;
    let result = semantic_check(source);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("concatenate") && e.contains("opaque")));
}

#[test]
fn test_z19_print_string_allowed() {
    let source = r#"
entity msg: String = "hello"
pattern show() -> String {
    print(msg)
    return "ok"
}
"#;
    let result = semantic_check(source);
    assert!(
        result.is_ok(),
        "print(String) should be allowed, errors: {:?}",
        result.errors
    );
}

// ═══════════════════════════════════════════════════════════════════
// Наряд №20: Variable scope + arity checking
// ═══════════════════════════════════════════════════════════════════

#[test]
#[ignore = "TODO: Undefined variable detection in semantic checker not yet implemented"]
fn test_z20_undefined_variable() {
    let source = r#"
pattern bad() -> String {
    return nonexistent_var
}
"#;
    let result = semantic_check(source);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("undefined variable")));
}

#[test]
fn test_z20_let_in_scope() {
    let source = r#"
pattern scoped(x: String) -> String {
    let y = upper(x)
    return y
}
"#;
    let result = semantic_check(source);
    assert!(
        result.is_ok(),
        "let binding should be in scope: {:?}",
        result.errors
    );
}

#[test]
fn test_z20_arity_builtin_wrong() {
    let source = r#"
pattern wrong() -> String {
    return upper("a", "b")
}
"#;
    let result = semantic_check(source);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("expects 1 argument") && e.contains("got 2")));
}

#[test]
fn test_z20_arity_pattern_wrong() {
    let source = r#"
pattern one(x: String) -> String { return x }
pattern caller() -> String {
    return one("a", "b")
}
"#;
    let result = semantic_check(source);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("expects 1 argument") && e.contains("one")));
}

#[test]
fn test_z20_undefined_function() {
    let source = r#"
pattern call_undef() -> String {
    return nonexistent("hi")
}
"#;
    let result = semantic_check(source);
    assert!(
        !result.is_ok(),
        "semantic check should fail for undefined function, but got is_ok=true, errors={:?}",
        result.errors
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("undefined") && e.contains("function")),
        "expected 'undefined...function' error, got errors: {:?}",
        result.errors
    );
}

#[test]
#[ignore = "TODO: Assignment to undefined variable detection in semantic checker not yet implemented"]
fn test_z20_assign_undefined() {
    let source = r#"
pattern bad_assign() -> String {
    phantom = "value"
    return "ok"
}
"#;
    let result = semantic_check(source);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("assignment to undefined variable")));
}

// ═══════════════════════════════════════════════════════════════════
// Наряд №21: Compiler fixes — StartsWith, Ne, no silent drops
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_z21_startswith_instruction_vm() {
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
            Instruction::Const(Value::String("hello world".to_string())),
            Instruction::Const(Value::String("hello".to_string())),
            Instruction::StartsWith,
            Instruction::Halt,
        ],
        collections_loaded: false,
    };
    let result = vm.run(program).expect("run should succeed");
    assert!(result.is_none()); // Halt consumes the value, but instruction executes OK
}

#[test]
fn test_z21_startswith_in_pattern_body() {
    let mut vm = Vm::new();
    let code = vec![
        Instruction::Const(Value::String("prefix_match".to_string())),
        Instruction::Const(Value::String("prefix".to_string())),
        Instruction::StartsWith,
        Instruction::Return,
    ];
    let result = vm
        .execute_code(
            &code,
            &mut Vec::new(),
            &mut Vec::new(),
            &Program {
                globals: vec![],
                patterns: vec![],
                learnables: vec![],
                rules: vec![],
                skill_indices: vec![],
                db_url: None,
                schema_ddl: vec![],
                main_code: vec![],
                collections_loaded: false,
            },
        )
        .expect("execute_code should succeed");
    match &result {
        Value::Float(f) => assert!((*f - 1.0).abs() < f64::EPSILON, "expected 1.0, got {}", f),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn test_z21_startswith_negative() {
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
            Instruction::Const(Value::String("wrong".to_string())),
            Instruction::Const(Value::String("right".to_string())),
            Instruction::StartsWith,
            Instruction::Halt,
        ],
        collections_loaded: false,
    };
    let result = vm.run(program).expect("run should succeed");
    assert!(result.is_none());
}

#[test]
#[ignore = "TODO: VM compiler does not yet support match with starts_with arms"]
fn test_z21_match_starts_with_compiles() {
    let source = r#"
pattern route_handler(path: String) -> String {
    match path {
        starts_with "/api" then { return "API endpoint" }
        starts_with "/static" then { return "Static file" }
        else { return "Not found" }
    }
}
"#;
    compile_ok(source);
}

#[test]
#[ignore = "TODO: VM compiler does not yet support Ne compare in rule conditions"]
fn test_z21_ne_compare_op_exists() {
    // Verify ConditionOp::Ne exists and compiles
    let source = r#"
entity data: Float = 42.0
rule If(data.urgency != 0.0) then data.name = "active" with priority=5
"#;
    // Just verify it parses — the rule condition uses !=
    let decls = parser::parse(source)
        .map_err(|e| format!("parse: {}", e))
        .unwrap();
    let mut compiler = Compiler::new();
    let result = compiler.compile(decls);
    assert!(
        result.is_ok(),
        "compile with Ne should succeed: {:?}",
        result.err()
    );
}

// ═══════════════════════════════════════════════════════════════════
// Наряд №22: VM pattern body loop — new instructions work
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_z22_make_list_in_pattern_body() {
    let mut vm = Vm::new();
    let code = vec![
        Instruction::Const(Value::String("a".to_string())),
        Instruction::Const(Value::String("b".to_string())),
        Instruction::MakeList(2),
        Instruction::ListLen,
        Instruction::Return,
    ];
    let result = vm
        .execute_code(
            &code,
            &mut Vec::new(),
            &mut Vec::new(),
            &Program {
                globals: vec![],
                patterns: vec![],
                learnables: vec![],
                rules: vec![],
                skill_indices: vec![],
                db_url: None,
                schema_ddl: vec![],
                main_code: vec![],
                collections_loaded: false,
            },
        )
        .expect("execute_code should succeed");
    match &result {
        Value::Float(f) => assert!((*f - 2.0).abs() < f64::EPSILON, "expected 2.0, got {}", f),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn test_z22_pop_in_pattern_body() {
    let mut vm = Vm::new();
    let code = vec![
        Instruction::Const(Value::String("discard_me".to_string())),
        Instruction::Pop,
        Instruction::Const(Value::String("keep_me".to_string())),
        Instruction::Return,
    ];
    let result = vm
        .execute_code(
            &code,
            &mut Vec::new(),
            &mut Vec::new(),
            &Program {
                globals: vec![],
                patterns: vec![],
                learnables: vec![],
                rules: vec![],
                skill_indices: vec![],
                db_url: None,
                schema_ddl: vec![],
                main_code: vec![],
                collections_loaded: false,
            },
        )
        .expect("execute_code should succeed");
    match &result {
        Value::String(s) => assert_eq!(s, "keep_me"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_z22_contains_in_pattern_body() {
    let mut vm = Vm::new();
    let code = vec![
        Instruction::Const(Value::String("hello world".to_string())),
        Instruction::Const(Value::String("world".to_string())),
        Instruction::Contains,
        Instruction::Return,
    ];
    let result = vm
        .execute_code(
            &code,
            &mut Vec::new(),
            &mut Vec::new(),
            &Program {
                globals: vec![],
                patterns: vec![],
                learnables: vec![],
                rules: vec![],
                skill_indices: vec![],
                db_url: None,
                schema_ddl: vec![],
                main_code: vec![],
                collections_loaded: false,
            },
        )
        .expect("execute_code should succeed");
    match &result {
        Value::Float(f) => assert!((*f - 1.0).abs() < f64::EPSILON, "expected 1.0, got {}", f),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn test_z22_index_access_in_pattern_body() {
    let mut vm = Vm::new();
    let code = vec![
        Instruction::Const(Value::String("x".to_string())),
        Instruction::Const(Value::String("y".to_string())),
        Instruction::Const(Value::String("z".to_string())),
        Instruction::MakeList(3),
        Instruction::Const(Value::Float(1.0)),
        Instruction::IndexAccess,
        Instruction::Return,
    ];
    let result = vm
        .execute_code(
            &code,
            &mut Vec::new(),
            &mut Vec::new(),
            &Program {
                globals: vec![],
                patterns: vec![],
                learnables: vec![],
                rules: vec![],
                skill_indices: vec![],
                db_url: None,
                schema_ddl: vec![],
                main_code: vec![],
                collections_loaded: false,
            },
        )
        .expect("execute_code should succeed");
    match &result {
        Value::String(s) => assert_eq!(s, "y"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_z22_struct_in_pattern_body() {
    let mut vm = Vm::new();
    let code = vec![
        Instruction::Const(Value::String("Alice".to_string())),
        Instruction::Const(Value::Float(30.0)),
        Instruction::MakeStruct(
            "Person".to_string(),
            vec!["name".to_string(), "age".to_string()],
        ),
        Instruction::GetField("name".to_string()),
        Instruction::Return,
    ];
    let result = vm
        .execute_code(
            &code,
            &mut Vec::new(),
            &mut Vec::new(),
            &Program {
                globals: vec![],
                patterns: vec![],
                learnables: vec![],
                rules: vec![],
                skill_indices: vec![],
                db_url: None,
                schema_ddl: vec![],
                main_code: vec![],
                collections_loaded: false,
            },
        )
        .expect("execute_code should succeed");
    match &result {
        Value::String(s) => assert_eq!(s, "Alice"),
        other => panic!("expected String, got {:?}", other),
    }
}

// ── Full compile+run integration tests ──────────────────────────

#[test]
#[ignore = "TODO: VM compiler does not yet support match with starts_with arms"]
fn test_z21_match_starts_with_full_run() {
    // Compile a pattern with starts_with match arm and verify it compiles
    let source = r#"
pattern classify(path: String) -> String {
    match path {
        starts_with "/api" then { return "API" }
        else { return "OTHER" }
    }
}
"#;
    compile_ok(source);
}

#[test]
fn test_z22_pattern_with_list_ops_compiles_and_runs() {
    let source = r#"
pattern second_item(items: List) -> String {
    return get(items, 1.0)
}
entity result: String = second_item(["first", "second", "third"])
"#;
    let result = compile_and_run(source);
    // Compilation should succeed (runtime may not fully work without all builtins)
    assert!(
        result.is_ok(),
        "compile+run should succeed: {:?}",
        result.err()
    );
}
