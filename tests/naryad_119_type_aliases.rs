// ── НАРЯД №119: Type Aliases — contract tests ───────────────────────

#[cfg(test)]
mod tests {
    use metalogos::ast::*;
    use std::collections::HashMap;

    // ── Unit: resolve_type_alias ────────────────────────────────────

    #[test]
    fn resolve_simple_alias() {
        let mut m = HashMap::new();
        m.insert("Token".to_string(), "Secret".to_string());
        assert_eq!(resolve_type_alias(&m, "Token").unwrap(), "Secret");
    }

    #[test]
    fn resolve_non_alias_passthrough() {
        let m = HashMap::new();
        assert_eq!(resolve_type_alias(&m, "String").unwrap(), "String");
    }

    #[test]
    fn resolve_chain_alias() {
        let mut m = HashMap::new();
        m.insert("A".to_string(), "B".to_string());
        m.insert("B".to_string(), "String".to_string());
        assert_eq!(resolve_type_alias(&m, "A").unwrap(), "String");
    }

    #[test]
    fn resolve_cycle_detected() {
        let mut m = HashMap::new();
        m.insert("A".to_string(), "B".to_string());
        m.insert("B".to_string(), "A".to_string());
        assert!(resolve_type_alias(&m, "A").is_err());
    }

    #[test]
    fn resolve_self_cycle_detected() {
        let mut m = HashMap::new();
        m.insert("A".to_string(), "A".to_string());
        assert!(resolve_type_alias(&m, "A").is_err());
    }

    // ── Unit: build_type_alias_map ──────────────────────────────────

    #[test]
    fn build_map_detects_duplicates() {
        let decls = vec![
            Declaration::TypeAlias(TypeAliasDecl {
                alias: "X".to_string(),
                target: "String".to_string(),
            }),
            Declaration::TypeAlias(TypeAliasDecl {
                alias: "X".to_string(),
                target: "Float".to_string(),
            }),
        ];
        let (_, errors) = build_type_alias_map(&decls);
        assert!(errors.iter().any(|e| e.contains("duplicate type alias: X")));
    }

    #[test]
    fn build_map_chain_ok() {
        let decls = vec![
            Declaration::TypeAlias(TypeAliasDecl {
                alias: "A".to_string(),
                target: "B".to_string(),
            }),
            Declaration::TypeAlias(TypeAliasDecl {
                alias: "B".to_string(),
                target: "String".to_string(),
            }),
        ];
        let (map, errors) = build_type_alias_map(&decls);
        assert!(errors.is_empty());
        assert_eq!(map.get("A").unwrap(), "B");
        assert_eq!(map.get("B").unwrap(), "String");
    }

    #[test]
    fn build_map_cycle_error() {
        let decls = vec![
            Declaration::TypeAlias(TypeAliasDecl {
                alias: "A".to_string(),
                target: "B".to_string(),
            }),
            Declaration::TypeAlias(TypeAliasDecl {
                alias: "B".to_string(),
                target: "A".to_string(),
            }),
        ];
        let (_, errors) = build_type_alias_map(&decls);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("cyclic type alias")));
    }

    // ── Integration: parse + semantic + execution ───────────────────

    #[test]
    fn basic_alias_parse_and_run() {
        let src = r#"
type MyStr = String
entity greeting: MyStr = "hello"
"#;
        let decls = metalogos::parser::parse(src).expect("parse");
        assert_eq!(decls.len(), 2);
        assert!(matches!(&decls[0], Declaration::TypeAlias(_)));

        let result = metalogos::semantic::check_program(&decls);
        assert!(result.is_ok(), "semantic: {}", result.format());

        let mut interp = metalogos::interpreter::Interpreter::new();
        let _ = interp.run(decls).expect("run");
        let val = interp.get_variable("greeting").expect("entity exists");
        match val {
            metalogos::interpreter::Value::String(s) => assert_eq!(s, "hello"),
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn no_conflict_with_memory_type_keyword() {
        // `type` inside memory { kv: { type: key_value } } must still parse
        let src = r#"
memory { kv: { type: key_value } }
type Token = String
entity t: Token = "ok"
"#;
        let decls = metalogos::parser::parse(src).expect("parse");
        assert!(decls.len() >= 2);
        let mut interp = metalogos::interpreter::Interpreter::new();
        let _ = interp.run(decls).expect("run");
    }

    #[test]
    fn cycle_detected_at_runtime() {
        let src = r#"
type A = B
type B = A
entity x: String = "hi"
"#;
        let decls = metalogos::parser::parse(src).expect("parse");
        let mut interp = metalogos::interpreter::Interpreter::new();
        let result = interp.run(decls);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cyclic type alias"));
    }

    // ── Contract: opaque alias inherits Secret protection ────────────

    #[test]
    fn opaque_alias_inherits_secret_protection() {
        // Direct Secret entity gets Value::Secret (Наряд №114)
        let src_direct = r#"entity sec: Secret = "s3cret"
"#;
        let decls_direct = metalogos::parser::parse(src_direct).expect("parse");
        let mut interp = metalogos::interpreter::Interpreter::new();
        let _ = interp.run(decls_direct).expect("run direct");
        let val = interp.get_variable("sec").unwrap();
        match &val {
            metalogos::interpreter::Value::Secret(_) => {}
            other => panic!("expected Secret, got {:?}", other),
        }
    }

    #[test]
    fn opaque_alias_same_behavior_as_direct_secret() {
        // Aliased Secret should behave identically to direct Secret
        let src_aliased = r#"
type Token = Secret
entity tok: Token = "my-token"
"#;
        let decls_aliased = metalogos::parser::parse(src_aliased).expect("parse");
        let mut interp = metalogos::interpreter::Interpreter::new();
        let _ = interp.run(decls_aliased).expect("run aliased");
        let val = interp.get_variable("tok").unwrap();
        match &val {
            metalogos::interpreter::Value::Secret(_) => {}
            other => panic!("expected Secret (via alias), got {:?}", other),
        }
    }
}
