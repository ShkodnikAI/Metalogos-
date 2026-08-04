// ── Наряд №17: break, continue, each-with-index ──────────────────────
// Tests for loop control flow constructs and indexed iteration.

use metalogos::ast::{Declaration, Statement};
use metalogos::interpreter::{Interpreter, Value};
use metalogos::parser;

/// Helper: parse source, run through interpreter, then call a pattern by name.
fn call_pattern(source: &str, pattern_name: &str, args: Vec<Value>) -> Result<Value, String> {
    let decls = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(decls)?;
    interp.call_pattern(pattern_name, &args)
}

// ── З-17.1: break ──────────────────────────────────────────────────

#[test]
fn test_break_exits_each_loop() {
    let source = r#"
pattern find_first_even(items: List) -> String {
    let mut result = "none"
    each item in items {
        if item == "2" {
            result = item
            break
        }
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "find_first_even",
        vec![Value::List(vec![
            Value::String("1".to_string()),
            Value::String("2".to_string()),
            Value::String("3".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "2"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_break_in_nested_if() {
    // break inside a nested if block should propagate out of the loop
    let source = r#"
pattern break_in_if(items: List) -> String {
    let mut found = "no"
    each item in items {
        if 1 == 1 {
            if item == "target" {
                found = "yes"
                break
            }
        }
    }
    return found
}
"#;
    let val = call_pattern(
        source,
        "break_in_if",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("target".to_string()),
            Value::String("b".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "yes"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_break_in_while_loop() {
    let source = r#"
pattern break_while() -> String {
    let mut i = 0.0
    let mut result = "not found"
    while i < 100.0 {
        if i == 5.0 {
            result = "found at 5"
            break
        }
        i = i + 1.0
    }
    return result
}
"#;
    let val = call_pattern(source, "break_while", vec![]).unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "found at 5"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_break_outside_loop_is_error() {
    let source = r#"
pattern bad_break() -> String {
    break
    return "unreachable"
}
"#;
    let result = call_pattern(source, "bad_break", vec![]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("break/continue used outside of a loop"));
}

// ── З-17.2: continue ───────────────────────────────────────────────

#[test]
fn test_continue_skips_iteration() {
    let source = r#"
pattern skip_odds(items: List) -> String {
    let mut result = ""
    each item in items {
        if item == "1" {
            continue
        }
        if item == "3" {
            continue
        }
        result = result + item
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "skip_odds",
        vec![Value::List(vec![
            Value::String("1".to_string()),
            Value::String("2".to_string()),
            Value::String("3".to_string()),
            Value::String("4".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "24"),
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_continue_in_while_loop() {
    // continue skips iteration 3, so sum = 1+2+4+5 = 12
    let source = r#"
pattern sum_skip3() -> String {
    let mut sum = 0.0
    let mut i = 0.0
    while i < 5.0 {
        i = i + 1.0
        if i == 3.0 {
            continue
        }
        sum = sum + i
    }
    return str(sum)
}
"#;
    let val = call_pattern(source, "sum_skip3", vec![]).unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "12"), // 1+2+4+5
        other => panic!("expected String, got {:?}", other),
    }
}

#[test]
fn test_continue_in_nested_if() {
    let source = r#"
pattern continue_nested(items: List) -> String {
    let mut result = ""
    each item in items {
        if item == "skip" {
            continue
        }
        result = result + item
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "continue_nested",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("skip".to_string()),
            Value::String("b".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "ab"),
        other => panic!("expected String, got {:?}", other),
    }
}

// ── З-17.3: each with index ────────────────────────────────────────

#[test]
fn test_each_with_index() {
    let source = r#"
pattern indexed(items: List) -> String {
    let mut result = ""
    each i, item in items {
        result = result + str(i) + ":" + item + " "
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "indexed",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
            Value::String("c".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "0:a 1:b 2:c "),
        other => panic!("expected String '0:a 1:b 2:c ', got {:?}", other),
    }
}

#[test]
fn test_each_with_index_break() {
    let source = r#"
pattern indexed_break(items: List) -> String {
    let mut result = ""
    each i, item in items {
        if i == 2.0 {
            break
        }
        result = result + item
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "indexed_break",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
            Value::String("c".to_string()),
            Value::String("d".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "ab"), // stops at index 2
        other => panic!("expected String 'ab', got {:?}", other),
    }
}

#[test]
fn test_each_with_index_continue() {
    let source = r#"
pattern indexed_continue(items: List) -> String {
    let mut result = ""
    each i, item in items {
        if i == 1.0 {
            continue
        }
        result = result + item
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "indexed_continue",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("SKIP".to_string()),
            Value::String("c".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "ac"),
        other => panic!("expected String 'ac', got {:?}", other),
    }
}

// ── Parse tests ─────────────────────────────────────────────────────

#[test]
fn test_parse_break() {
    let source = r#"
pattern test() -> String {
    each item in ["a", "b"] {
        break
    }
    return "done"
}
"#;
    let decls = parser::parse(source).expect("parse failed");
    assert_eq!(decls.len(), 1);
    if let Declaration::Pattern(p) = &decls[0] {
        assert!(matches!(p.body[0], Statement::Each { .. }));
        if let Statement::Each { body, .. } = &p.body[0] {
            assert!(matches!(body[0], Statement::Break));
        }
    } else {
        panic!("expected Pattern declaration");
    }
}

#[test]
fn test_parse_continue() {
    let source = r#"
pattern test() -> String {
    each item in ["a", "b"] {
        continue
    }
    return "done"
}
"#;
    let decls = parser::parse(source).expect("parse failed");
    if let Declaration::Pattern(p) = &decls[0] {
        if let Statement::Each { body, .. } = &p.body[0] {
            assert!(matches!(body[0], Statement::Continue));
        }
    }
}

#[test]
fn test_parse_each_with_index() {
    let source = r#"
pattern test() -> String {
    each i, item in ["a", "b"] {
        let x = str(i)
    }
    return "done"
}
"#;
    let decls = parser::parse(source).expect("parse failed");
    if let Declaration::Pattern(p) = &decls[0] {
        assert!(
            matches!(p.body[0], Statement::EachWithIndex { ref index_var, ref item_var, .. } if index_var == "i" && item_var == "item")
        );
    }
}

#[test]
fn test_break_in_match_inside_loop() {
    // break propagates through match arms inside a loop
    let source = r#"
pattern break_via_match(items: List) -> String {
    let mut result = ""
    each item in items {
        match item {
            "stop" then {
                break
            }
            else {
                result = result + item
            }
        }
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "break_via_match",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("stop".to_string()),
            Value::String("b".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "a"),
        other => panic!("expected String 'a', got {:?}", other),
    }
}

#[test]
fn test_continue_in_match_inside_loop() {
    let source = r#"
pattern continue_via_match(items: List) -> String {
    let mut result = ""
    each item in items {
        match item {
            "skip" then {
                continue
            }
            else {
                result = result + item
            }
        }
    }
    return result
}
"#;
    let val = call_pattern(
        source,
        "continue_via_match",
        vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("skip".to_string()),
            Value::String("b".to_string()),
        ])],
    )
    .unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "ab"),
        other => panic!("expected String 'ab', got {:?}", other),
    }
}

#[test]
fn test_implicit_return_still_works_after_break() {
    // implicit return (last expr value) must still work
    let source = r#"
pattern implicit_ret() -> String {
    let mut x = 0.0
    while x < 3.0 {
        x = x + 1.0
        if x == 2.0 {
            break
        }
    }
    str(x)
}
"#;
    let val = call_pattern(source, "implicit_ret", vec![]).unwrap();
    match val {
        Value::String(s) => assert_eq!(s, "2"),
        other => panic!("expected String '2', got {:?}", other),
    }
}
