// ── Наряд №194 Contract 1: round-trip correctness ───────────────────
//
// `detokenize(tokenize(s)) == s` for any Unicode string, including
// non-ASCII (Cyrillic, emoji, multi-byte characters).

use metalogos::builtins::{builtin_reflex_detokenize, builtin_reflex_tokenize};
use metalogos::interpreter::Value;

#[test]
fn roundtrip_ascii() {
    let s = "hello world";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    let back = builtin_reflex_detokenize(&[tokens]).expect("detokenize");
    match back {
        Value::String(result) => assert_eq!(result, s),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ ASCII roundtrip: '{}'", s);
}

#[test]
fn roundtrip_cyrillic() {
    let s = "Привет, мир!";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    let back = builtin_reflex_detokenize(&[tokens]).expect("detokenize");
    match back {
        Value::String(result) => assert_eq!(result, s),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ Cyrillic roundtrip: '{}'", s);
}

#[test]
fn roundtrip_emoji() {
    // Multi-code-point emoji (ZWJ sequences, multi-byte)
    let s = "Hello 🌍🚀❤️";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    let back = builtin_reflex_detokenize(&[tokens]).expect("detokenize");
    match back {
        Value::String(result) => assert_eq!(result, s),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ Emoji roundtrip: '{}'", s);
}

#[test]
fn roundtrip_mixed() {
    let s = "Hello мир 🌍 — Привет!";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    let back = builtin_reflex_detokenize(&[tokens]).expect("detokenize");
    match back {
        Value::String(result) => assert_eq!(result, s),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ Mixed roundtrip: '{}'", s);
}

#[test]
fn roundtrip_empty_string() {
    let s = "";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    let back = builtin_reflex_detokenize(&[tokens]).expect("detokenize");
    match back {
        Value::String(result) => assert_eq!(result, s),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ Empty string roundtrip");
}

#[test]
fn roundtrip_single_char() {
    let s = "A";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    let back = builtin_reflex_detokenize(&[tokens]).expect("detokenize");
    match back {
        Value::String(result) => assert_eq!(result, s),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ Single char roundtrip: '{}'", s);
}

#[test]
fn tokenize_produces_correct_code_points() {
    // Verify the actual code points match expectations
    let s = "AB";
    let tokens = builtin_reflex_tokenize(&[Value::String(s.to_string())]).expect("tokenize");
    match tokens {
        Value::List(items) => {
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::Float(n) => assert_eq!(*n as u32, 65), // 'A' = 65
                other => panic!("expected Float, got {}", other.type_name()),
            }
            match &items[1] {
                Value::Float(n) => assert_eq!(*n as u32, 66), // 'B' = 66
                other => panic!("expected Float, got {}", other.type_name()),
            }
        }
        other => panic!("expected List, got {}", other.type_name()),
    }
    println!("✓ Code points correct: 'A'=65, 'B'=66");
}

#[test]
fn detokenize_invalid_code_point_errors() {
    // Surrogate code points (0xD800-0xDFFF) are invalid Unicode chars
    let invalid_tokens = Value::List(vec![Value::Float(0xD800 as f64)]);
    let result = builtin_reflex_detokenize(&[invalid_tokens]);
    assert!(result.is_err(), "should error on invalid code point");
    let err = result.unwrap_err();
    assert!(
        err.contains("invalid Unicode code point"),
        "error should mention 'invalid Unicode code point', got: {}",
        err
    );
    println!("✓ Invalid code point (0xD800) → clean error");
}

#[test]
fn tokenize_wrong_arity_errors() {
    assert!(builtin_reflex_tokenize(&[]).is_err());
    assert!(
        builtin_reflex_tokenize(&[Value::String("x".into()), Value::String("y".into())]).is_err()
    );
    println!("✓ Wrong arity → clean error");
}

#[test]
fn detokenize_wrong_type_errors() {
    let result = builtin_reflex_detokenize(&[Value::String("not a list".into())]);
    assert!(result.is_err(), "should error on non-List argument");
    println!("✓ Wrong type → clean error");
}
