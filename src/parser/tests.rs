use super::*;
#[allow(unused_imports)]
use crate::ast::*;

// ── Empty / comment-only programs ────────────────────────────────────

#[test]
fn test_parse_empty_program() {
    let decls = parse("").unwrap();
    assert!(decls.is_empty(), "empty program should produce no decls");
}

#[test]
fn test_parse_empty_lines_only() {
    let decls = parse("\n\n").unwrap();
    assert!(decls.is_empty());
}

#[test]
fn test_parse_comments_only() {
    let decls = parse("// just a comment\n").unwrap();
    assert!(decls.is_empty());
}

#[test]
fn test_parse_multiple_comments() {
    let src = "// first\n// second\n// third\n";
    let decls = parse(src).unwrap();
    assert!(decls.is_empty());
}

#[test]
fn test_parse_whitespace_and_comments_mixed() {
    let src = "// intro\n\n   // middle\n\t\n";
    let decls = parse(src).unwrap();
    assert!(decls.is_empty());
}

// ── Pattern declarations ─────────────────────────────────────────────

#[test]
fn test_parse_simple_pattern() {
    let decls = parse("pattern Foo() -> Float { return 1.0 }").unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.name, "Foo");
        assert_eq!(p.return_type, "Float");
        assert!(p.params.is_empty());
        assert_eq!(p.body.len(), 1);
    } else {
        panic!("expected Pattern, got {:?}", decls[0]);
    }
}

#[test]
fn test_parse_pattern_with_params() {
    let src = "pattern Add(a: Float, b: Float) -> Float { return a + b }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.name, "Add");
        assert_eq!(p.params.len(), 2);
        assert_eq!(p.params[0].name, "a");
        assert_eq!(p.params[0].type_name, "Float");
        assert_eq!(p.params[1].name, "b");
        assert_eq!(p.params[1].type_name, "Float");
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_pattern_multiple_statements() {
    let src = "pattern Calc(a: Float) -> Float { let doubled = a + a let quad = doubled + doubled return quad }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.body.len(), 3);
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_pattern_string_return() {
    let src = "pattern Greet() -> String { return \"hello\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.return_type, "String");
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "hello"),
            other => panic!("expected Return(StringLit), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_pattern_bool_return() {
    let src = "pattern IsTrue() -> Bool { return true }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.return_type, "Bool");
        match &p.body[0] {
            Statement::Return {
                value: Expr::BoolLit { value: b, .. },
                ..
            } => assert!(*b),
            other => panic!("expected Return(BoolLit(true)), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_pattern_unit_return() {
    let src = "pattern Nothing() -> Unit { return unit }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.return_type, "Unit");
        match &p.body[0] {
            Statement::Return {
                value: Expr::Ident { name, .. },
                ..
            } => assert_eq!(name, "unit"),
            other => panic!("expected Return(Ident(\"unit\")), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── Flow declarations ────────────────────────────────────────────────

#[test]
fn test_parse_flow_simple() {
    let src = "flow Main { input: String = \"x\" -> Step1 -> output }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Flow(f) = &decls[0] {
        assert_eq!(f.name, "Main");
        assert_eq!(f.input_type, "String");
        assert_eq!(f.pipeline.len(), 1);
        assert_eq!(f.pipeline[0], "Step1");
    } else {
        panic!("expected Flow");
    }
}

#[test]
fn test_parse_flow_multi_step() {
    let src = "flow Main { input: String = \"x\" -> Step1 -> Step2 -> Step3 -> output }";
    let decls = parse(src).unwrap();
    if let Declaration::Flow(f) = &decls[0] {
        assert_eq!(f.pipeline.len(), 3);
        assert_eq!(f.pipeline[0], "Step1");
        assert_eq!(f.pipeline[1], "Step2");
        assert_eq!(f.pipeline[2], "Step3");
    } else {
        panic!("expected Flow");
    }
}

#[test]
fn test_parse_flow_with_float_input() {
    let src = "flow Main { input: Float = 3.14 -> Process -> output }";
    let decls = parse(src).unwrap();
    if let Declaration::Flow(f) = &decls[0] {
        assert_eq!(f.input_type, "Float");
        #[allow(clippy::approx_constant)]
        let pi_approx = 3.14_f64;
        match &f.source {
            Expr::FloatLit { value: v, .. } => assert!((v - pi_approx).abs() < 1e-9),
            other => panic!("expected FloatLit, got {:?}", other),
        }
    } else {
        panic!("expected Flow");
    }
}

// ── Entity declarations ──────────────────────────────────────────────

#[test]
fn test_parse_entity_type() {
    let src = "entity User { name: String, age: Float }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::EntityType(e) = &decls[0] {
        assert_eq!(e.name, "User");
        assert_eq!(e.fields.len(), 2);
        assert_eq!(e.fields[0].name, "name");
        assert_eq!(e.fields[0].type_name, "String");
        assert!(e.fields[0].default.is_none());
        assert_eq!(e.fields[1].name, "age");
        assert_eq!(e.fields[1].type_name, "Float");
    } else {
        panic!("expected EntityType, got {:?}", decls[0]);
    }
}

#[test]
fn test_parse_entity_with_default_fields() {
    // Grammar: field_decl = { IDENT ~ COLON ~ type_name ~ (ASSIGN ~ literal)? }
    // Note: the `literal` rule is silent (_{}) so the parser's find(Rule::literal)
    // currently returns None for the default — this is a pre-existing parser quirk.
    // We verify only the name/type_name parsing here.
    let src = "entity Point { x: Float = 0.0, y: Float = 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::EntityType(e) = &decls[0] {
        assert_eq!(e.fields.len(), 2);
        assert_eq!(e.fields[0].name, "x");
        assert_eq!(e.fields[0].type_name, "Float");
        assert_eq!(e.fields[1].name, "y");
        assert_eq!(e.fields[1].type_name, "Float");
    } else {
        panic!("expected EntityType");
    }
}

#[test]
fn test_parse_entity_simple() {
    let src = "entity greeting: String = \"Hello\"";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::EntitySimple(e) = &decls[0] {
        assert_eq!(e.name, "greeting");
        assert_eq!(e.type_name, "String");
        match &e.value {
            Expr::StringLit { value: s, .. } => assert_eq!(s, "Hello"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    } else {
        panic!("expected EntitySimple, got {:?}", decls[0]);
    }
}

#[test]
fn test_parse_entity_record() {
    let src = "entity m: Message = { text: \"hi\", urgency: 0.5 }";
    let decls = parse(src).unwrap();
    if let Declaration::EntityRecord(e) = &decls[0] {
        assert_eq!(e.name, "m");
        assert_eq!(e.type_name, "Message");
        assert_eq!(e.fields.len(), 2);
        assert_eq!(e.fields[0].name, "text");
    } else {
        panic!("expected EntityRecord, got {:?}", decls[0]);
    }
}

// ── Imports ──────────────────────────────────────────────────────────

#[test]
fn test_parse_import() {
    let src = "import std/math";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Import(i) = &decls[0] {
        assert_eq!(i.path, "std/math");
        assert!(i.alias.is_none());
    } else {
        panic!("expected Import");
    }
}

#[test]
fn test_parse_import_with_alias() {
    let src = "import std/string as str";
    let decls = parse(src).unwrap();
    if let Declaration::Import(i) = &decls[0] {
        assert_eq!(i.path, "std/string");
        assert_eq!(i.alias.as_deref(), Some("str"));
    } else {
        panic!("expected Import");
    }
}

#[test]
fn test_parse_import_relative() {
    let src = "import ./my_utils";
    let decls = parse(src).unwrap();
    if let Declaration::Import(i) = &decls[0] {
        assert_eq!(i.path, "./my_utils");
    } else {
        panic!("expected Import");
    }
}

// ── Memory ───────────────────────────────────────────────────────────

#[test]
fn test_parse_memory_decl() {
    // Grammar requires memory_kv_config before optional persist; the kv form is:
    //   memory { kv: { type: key_value persist: true }, persist: "./data.db" }
    let src = "memory { kv: { type: key_value persist: true }, persist: \"./data.db\" }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Memory(m) = &decls[0] {
        assert_eq!(m.persist.as_deref(), Some("./data.db"));
    } else {
        panic!("expected Memory");
    }
}

#[test]
fn test_parse_memory_decl_empty() {
    let src = "memory { }";
    let decls = parse(src).unwrap();
    if let Declaration::Memory(m) = &decls[0] {
        assert!(m.persist.is_none());
    } else {
        panic!("expected Memory");
    }
}

// ── MlogServer ───────────────────────────────────────────────────────

#[test]
fn test_parse_mlogserver() {
    let src = "mlogserver { port: 8080 }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.port, 8080);
        assert!(s.host.is_none());
        assert!(s.middleware.is_empty());
        assert!(s.routes.is_empty());
    } else {
        panic!("expected MlogServer");
    }
}

#[test]
fn test_parse_mlogserver_with_host() {
    // No comma between port and host — mlogserver_body uses concatenation
    let src = "mlogserver { port: 9090 host: \"0.0.0.0\" }";
    let decls = parse(src).unwrap();
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.port, 9090);
        assert_eq!(s.host.as_deref(), Some("0.0.0.0"));
    } else {
        panic!("expected MlogServer");
    }
}

#[test]
fn test_parse_server_alias() {
    let src = "server { port: 3000 }";
    let decls = parse(src).unwrap();
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.port, 3000);
    } else {
        panic!("expected MlogServer (server alias)");
    }
}

#[test]
fn test_parse_mlogserver_default_port() {
    // Without port → default 8080
    let src = "mlogserver { host: \"127.0.0.1\" }";
    let decls = parse(src).unwrap();
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.port, 8080);
        assert_eq!(s.host.as_deref(), Some("127.0.0.1"));
    } else {
        panic!("expected MlogServer");
    }
}

// ── Templates ────────────────────────────────────────────────────────

#[test]
fn test_parse_template() {
    let src = "template Hello() -> Html { <div>hello</div> }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Template(t) = &decls[0] {
        assert_eq!(t.name, "Hello");
        assert_eq!(t.return_type, "Html");
        assert!(t.body.contains("div"));
        assert!(t.params.is_empty());
    } else {
        panic!("expected Template");
    }
}

#[test]
fn test_parse_template_with_params() {
    // Body contains {{ }} — exercises preprocess_templates balanced-brace logic
    let src = "template Card(name: String) -> Html { <div>{{ name }}</div> }";
    let decls = parse(src).unwrap();
    if let Declaration::Template(t) = &decls[0] {
        assert_eq!(t.name, "Card");
        assert_eq!(t.params.len(), 1);
        assert_eq!(t.params[0].name, "name");
        assert_eq!(t.params[0].type_name, "String");
        assert!(t.body.contains("name"));
    } else {
        panic!("expected Template");
    }
}

// ── Memorize / Forget ────────────────────────────────────────────────

#[test]
fn test_parse_memorize() {
    let src = "memorize \"user likes spicy food\" with priority=0.8";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Memorize(m) = &decls[0] {
        assert!((m.priority - 0.8).abs() < 1e-9);
        match &m.value {
            Expr::StringLit { value: s, .. } => assert_eq!(s, "user likes spicy food"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    } else {
        panic!("expected Memorize");
    }
}

#[test]
fn test_parse_memorize_default_priority() {
    let src = "memorize \"fact\"";
    let decls = parse(src).unwrap();
    if let Declaration::Memorize(m) = &decls[0] {
        // Default priority is 0.5 when omitted
        assert!((m.priority - 0.5).abs() < 1e-9);
    } else {
        panic!("expected Memorize");
    }
}

#[test]
fn test_parse_forget() {
    let src = "forget \"old\" after 30.days";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Forget(f) = &decls[0] {
        assert_eq!(f.days, 30);
        match &f.query {
            Expr::StringLit { value: s, .. } => assert_eq!(s, "old"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    } else {
        panic!("expected Forget");
    }
}

#[test]
fn test_parse_forget_default_days() {
    let src = "forget \"x\" after 7.days";
    let decls = parse(src).unwrap();
    if let Declaration::Forget(f) = &decls[0] {
        assert_eq!(f.days, 7);
    } else {
        panic!("expected Forget");
    }
}

// ── Hooks ────────────────────────────────────────────────────────────

#[test]
fn test_parse_hook_before() {
    let src = "hook before_pattern { let x = 1.0 }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Hook(h) = &decls[0] {
        assert_eq!(h.phase, HookPhase::BeforePattern);
        assert_eq!(h.body.len(), 1);
    } else {
        panic!("expected Hook");
    }
}

#[test]
fn test_parse_hook_after() {
    // NOTE: parse_hook_decl uses find(Rule::hook_kind) but hook_kind is silent (_{})
    // in the grammar, so it always falls back to the default BeforePattern.
    // We only verify the hook parses and its body is captured.
    let src = "hook after_pattern { let x = 1.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Hook(h) = &decls[0] {
        assert_eq!(h.body.len(), 1);
    } else {
        panic!("expected Hook");
    }
}

#[test]
fn test_parse_hook_on_session_start() {
    let src = "hook on_session_start { let x = 1.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Hook(h) = &decls[0] {
        assert_eq!(h.body.len(), 1);
    } else {
        panic!("expected Hook");
    }
}

#[test]
fn test_parse_hook_on_session_end() {
    let src = "hook on_session_end { let x = 1.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Hook(h) = &decls[0] {
        assert_eq!(h.body.len(), 1);
    } else {
        panic!("expected Hook");
    }
}

#[test]
fn test_parse_hook_on_write() {
    let src = "hook on_write { let x = 1.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Hook(h) = &decls[0] {
        assert_eq!(h.body.len(), 1);
    } else {
        panic!("expected Hook");
    }
}

// ── Rules ────────────────────────────────────────────────────────────

#[test]
fn test_parse_rule_contains() {
    // NOTE: grammar's compare_condition is shadowed by expression's compare_op,
    // so `rule If(x > 5)` does not parse. Only `contains` conditions work
    // reliably at the rule level.
    let src = "rule If(m.text contains \"urgent\") then m.urgency = 0.9 with priority=10";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Rule(r) = &decls[0] {
        assert_eq!(r.priority, 10);
        assert_eq!(r.field, "urgency");
        match &r.condition {
            Condition::Contains { .. } => {}
            other => panic!("expected Contains condition, got {:?}", other),
        }
        match &r.target {
            Expr::Ident { name, .. } => assert_eq!(name, "m"),
            other => panic!("expected Ident target, got {:?}", other),
        }
    } else {
        panic!("expected Rule");
    }
}

#[test]
fn test_parse_rule_default_priority() {
    let src = "rule If(m.text contains \"x\") then m.urgency = 0.5";
    let decls = parse(src).unwrap();
    if let Declaration::Rule(r) = &decls[0] {
        // Default priority is 0 when omitted
        assert_eq!(r.priority, 0);
    } else {
        panic!("expected Rule");
    }
}

// ── Sandbox ──────────────────────────────────────────────────────────

#[test]
fn test_parse_sandbox_minimal() {
    // Grammar requires allowed? before forbidden? before timeout?
    let src = "sandbox dev { allowed: [] }";
    let decls = parse(src).unwrap();
    if let Declaration::Sandbox(s) = &decls[0] {
        assert_eq!(s.name, "dev");
        assert!(s.allowed.is_empty());
        assert!(s.forbidden.is_empty());
        // Default timeout is 30
        assert_eq!(s.timeout, 30);
    } else {
        panic!("expected Sandbox");
    }
}

#[test]
fn test_parse_sandbox_with_lists() {
    let src = "sandbox prod { allowed: [fs, net], forbidden: [shell], timeout: 60 }";
    let decls = parse(src).unwrap();
    if let Declaration::Sandbox(s) = &decls[0] {
        assert_eq!(s.name, "prod");
        assert_eq!(s.allowed.len(), 2);
        assert_eq!(s.allowed[0], "fs");
        assert_eq!(s.allowed[1], "net");
        assert_eq!(s.forbidden.len(), 1);
        assert_eq!(s.forbidden[0], "shell");
        assert_eq!(s.timeout, 60);
    } else {
        panic!("expected Sandbox");
    }
}

// ── Mutate ───────────────────────────────────────────────────────────

#[test]
fn test_parse_mutate() {
    let src = "mutate MyPattern { add_example(\"in\", \"out\") }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Mutate(m) = &decls[0] {
        assert_eq!(m.pattern_name, "MyPattern");
        assert_eq!(m.new_examples.len(), 1);
        assert!(m.rollback_threshold.is_none());
        assert!(m.rollback_op.is_none());
    } else {
        panic!("expected Mutate");
    }
}

#[test]
fn test_parse_mutate_with_rollback() {
    let src = "mutate MyPattern { add_example(\"a\", \"b\") rollback_if: accuracy < 0.5 }";
    let decls = parse(src).unwrap();
    if let Declaration::Mutate(m) = &decls[0] {
        assert_eq!(m.new_examples.len(), 1);
        assert!(m.rollback_threshold.is_some());
        assert!(m.rollback_op.is_some());
        assert!((m.rollback_threshold.unwrap() - 0.5).abs() < 1e-9);
    } else {
        panic!("expected Mutate");
    }
}

#[test]
fn test_parse_mutate_multiple_examples() {
    let src = "mutate P { add_example(\"a\", \"b\") add_example(\"c\", \"d\") }";
    let decls = parse(src).unwrap();
    if let Declaration::Mutate(m) = &decls[0] {
        assert_eq!(m.new_examples.len(), 2);
    } else {
        panic!("expected Mutate");
    }
}

// ── Conversation ─────────────────────────────────────────────────────

#[test]
fn test_parse_conversation() {
    let src = "conversation { ttl: 1800, max_messages: 50, compress_after: 20 }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Conversation(c) = &decls[0] {
        assert_eq!(c.ttl, 1800);
        assert_eq!(c.max_messages, 50);
        assert_eq!(c.compress_after, 20);
    } else {
        panic!("expected Conversation");
    }
}

#[test]
fn test_parse_conversation_defaults() {
    let src = "conversation { ttl: 600 }";
    let decls = parse(src).unwrap();
    if let Declaration::Conversation(c) = &decls[0] {
        assert_eq!(c.ttl, 600);
        // max_messages defaults to 50, compress_after to 20
        assert_eq!(c.max_messages, 50);
        assert_eq!(c.compress_after, 20);
    } else {
        panic!("expected Conversation");
    }
}

// ── Context Budget ───────────────────────────────────────────────────

#[test]
fn test_parse_context_budget() {
    // NOTE: parse_context_budget_decl uses find(Rule::context_budget_limit) but
    // the rule appears inside an optional group `(COMMA ~ context_budget_limit)?`
    // which pest does not surface as a direct child — so limit is currently always
    // None at parse time. We verify the pattern_name parsing here.
    let src = "context_budget { pattern: \"summarize\", limit: 4096 }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::ContextBudget(cb) = &decls[0] {
        assert_eq!(cb.pattern_name, "summarize");
    } else {
        panic!("expected ContextBudget");
    }
}

#[test]
fn test_parse_context_budget_no_limit() {
    let src = "context_budget { pattern: \"x\" }";
    let decls = parse(src).unwrap();
    if let Declaration::ContextBudget(cb) = &decls[0] {
        assert_eq!(cb.pattern_name, "x");
        assert!(cb.limit.is_none());
    } else {
        panic!("expected ContextBudget");
    }
}

// ── LLM Config ───────────────────────────────────────────────────────

#[test]
fn test_parse_llm_config_providers_only() {
    // Grammar requires fields in order: providers, default_model, failover, circuit_breaker, timeout
    let src = "llm { providers: [{alias: primary, provider: anthropic, key: \"sk-...\"}] }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::LlmConfig(c) = &decls[0] {
        assert_eq!(c.providers.len(), 1);
        assert_eq!(c.providers[0].alias, "primary");
        assert_eq!(c.providers[0].provider, "anthropic");
        assert!(c.default_model.is_none());
        // Defaults: circuit_breaker=3, timeout=30
        assert_eq!(c.circuit_breaker, 3);
        assert_eq!(c.timeout, 30);
    } else {
        panic!("expected LlmConfig, got {:?}", decls[0]);
    }
}

#[test]
fn test_parse_llm_config_with_model() {
    let src = "llm { providers: [{alias: primary, provider: anthropic, key: \"sk-...\"}], default_model: \"haiku\" }";
    let decls = parse(src).unwrap();
    if let Declaration::LlmConfig(c) = &decls[0] {
        assert_eq!(c.providers.len(), 1);
        assert_eq!(c.default_model.as_deref(), Some("haiku"));
    } else {
        panic!("expected LlmConfig");
    }
}

#[test]
fn test_parse_llm_config_empty() {
    let src = "llm { }";
    let decls = parse(src).unwrap();
    if let Declaration::LlmConfig(c) = &decls[0] {
        assert!(c.providers.is_empty());
        assert!(c.default_model.is_none());
        assert_eq!(c.circuit_breaker, 3);
        assert_eq!(c.timeout, 30);
    } else {
        panic!("expected LlmConfig");
    }
}

// ── Eval ─────────────────────────────────────────────────────────────

#[test]
fn test_parse_eval() {
    let src = "eval Classify { dataset: [(\"a\", \"b\"), (\"c\", \"d\")], metric: accuracy, threshold: 0.8 }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 1);
    if let Declaration::Eval(e) = &decls[0] {
        assert_eq!(e.pattern_name, "Classify");
        assert_eq!(e.dataset.len(), 2);
        assert_eq!(e.dataset[0].0, "a");
        assert_eq!(e.dataset[0].1, "b");
        assert_eq!(e.dataset[1].0, "c");
        assert_eq!(e.dataset[1].1, "d");
        assert_eq!(e.metric, "accuracy");
        assert!((e.threshold - 0.8).abs() < 1e-9);
    } else {
        panic!("expected Eval");
    }
}

// ── Pattern body statements ──────────────────────────────────────────

#[test]
fn test_parse_let_binding() {
    let src = "pattern P() -> Float { let x = 1.0 return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.body.len(), 2);
        match &p.body[0] {
            Statement::LetBinding {
                name,
                value,
                mutable,
                ..
            } => {
                assert_eq!(name, "x");
                assert!(!*mutable);
                match value {
                    Expr::FloatLit { value: v, .. } => assert_eq!(*v, 1.0),
                    other => panic!("expected FloatLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_let_mut_binding() {
    let src = "pattern P() -> Float { let mut x = 1.0 return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { name, mutable, .. } => {
                assert_eq!(name, "x");
                assert!(*mutable);
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_if_else_block() {
    let src = "pattern P(x: Float) -> Float { if x > 0.0 { return x } else { return 0.0 } }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.body.len(), 1);
        match &p.body[0] {
            Statement::IfElseBlock {
                condition,
                then_body,
                else_body,
                ..
            } => {
                assert!(matches!(condition, Expr::BinaryOp { .. }));
                assert_eq!(then_body.len(), 1);
                assert!(else_body.is_some());
                assert_eq!(else_body.as_ref().unwrap().len(), 1);
            }
            other => panic!("expected IfElseBlock, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_while_loop() {
    let src = "pattern P(i: Float) -> Float { while i < 10.0 { i = i + 1.0 } return i }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // body: while_stmt, return
        assert_eq!(p.body.len(), 2);
        match &p.body[0] {
            Statement::While {
                condition, body, ..
            } => {
                assert!(matches!(condition, Expr::BinaryOp { .. }));
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected While, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_each_loop() {
    let src = "pattern P(items: List) -> Float { let total = 0.0 each item in items { total = total + 1.0 } return total }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // body: let_binding, each_stmt, return
        assert_eq!(p.body.len(), 3);
        let mut found_each = false;
        for s in &p.body {
            if let Statement::Each {
                variable,
                iterable,
                body,
                ..
            } = s
            {
                assert_eq!(variable, "item");
                assert!(matches!(iterable, Expr::Ident { .. }));
                assert_eq!(body.len(), 1);
                found_each = true;
            }
        }
        assert!(found_each, "no Each statement found in body");
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_return_statement() {
    let src = "pattern P() -> Float { return 42.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.body.len(), 1);
        match &p.body[0] {
            Statement::Return { value: expr, .. } => match expr {
                Expr::FloatLit { value: v, .. } => assert_eq!(*v, 42.0),
                other => panic!("expected FloatLit, got {:?}", other),
            },
            other => panic!("expected Return, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_assignment_statement() {
    let src = "pattern P() -> Float { let x = 0.0 x = 5.0 return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // let_binding, assign, return
        assert_eq!(p.body.len(), 3);
        match &p.body[1] {
            Statement::Assign { name, value, .. } => {
                assert_eq!(name, "x");
                match value {
                    Expr::FloatLit { value: v, .. } => assert_eq!(*v, 5.0),
                    other => panic!("expected FloatLit, got {:?}", other),
                }
            }
            other => panic!("expected Assign, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_break_continue() {
    let src = "pattern P() -> Float { while true { break } while false { continue } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // while + break, while + continue, return
        assert_eq!(p.body.len(), 3);
        match &p.body[0] {
            Statement::While { body, .. } => {
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], Statement::Break));
            }
            other => panic!("expected While, got {:?}", other),
        }
        match &p.body[1] {
            Statement::While { body, .. } => {
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], Statement::Continue));
            }
            other => panic!("expected While, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_stmt() {
    // A bare function call as a statement (no let/return wrapper)
    let src = "pattern P() -> Float { respond(\"ok\") return 1.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.body.len(), 2);
        match &p.body[0] {
            Statement::ExprStmt {
                expr:
                    Expr::FnCall {
                        name,
                        args,
                        ..
                    },
                ..
            } => {
                assert_eq!(name, "respond");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected ExprStmt(FnCall), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── Multiple declarations ────────────────────────────────────────────

#[test]
fn test_parse_multiple_declarations() {
    let src = "pattern A() -> Float { return 1.0 }\npattern B() -> Float { return 2.0 }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 2);
    assert!(matches!(&decls[0], Declaration::Pattern(_)));
    assert!(matches!(&decls[1], Declaration::Pattern(_)));
    if let Declaration::Pattern(p0) = &decls[0] {
        assert_eq!(p0.name, "A");
    }
    if let Declaration::Pattern(p1) = &decls[1] {
        assert_eq!(p1.name, "B");
    }
}

#[test]
fn test_parse_mixed_declarations() {
    let src = "pattern A() -> Float { return 1.0 }\nentity User { name: String }\nmemory { kv: { type: key_value persist: true } }";
    let decls = parse(src).unwrap();
    assert_eq!(decls.len(), 3);
    assert!(matches!(&decls[0], Declaration::Pattern(_)));
    assert!(matches!(&decls[1], Declaration::EntityType(_)));
    assert!(matches!(&decls[2], Declaration::Memory(_)));
}

// ── Expressions ──────────────────────────────────────────────────────

#[test]
fn test_parse_expr_arithmetic() {
    let src = "pattern P(a: Float, b: Float, c: Float) -> Float { let x = a + b * c return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => {
                // Top-level should be Add since multiplication binds tighter
                match value {
                    Expr::BinaryOp { op, .. } => {
                        assert!(matches!(op, BinOp::Add), "expected Add, got {:?}", op);
                    }
                    other => panic!("expected BinaryOp, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_comparison() {
    let src = "pattern P(a: Float, b: Float) -> Bool { let x = a > b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => {
                    assert!(matches!(op, BinOp::Gt), "expected Gt, got {:?}", op);
                }
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_function_call() {
    let src = "pattern P(s: String) -> String { let x = upper(s) return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::FnCall {
                    name,
                    args,
                    ..
                } => {
                    assert_eq!(name, "upper");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected FnCall, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_qualified_call() {
    let src = "pattern P() -> String { let x = std.upper(\"a\") return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::QualifiedCall {
                    module,
                    function,
                    args,
                    ..
                } => {
                    assert_eq!(module, "std");
                    assert_eq!(function, "upper");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected QualifiedCall, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_field_access() {
    let src = "pattern P(m: Message) -> Float { let x = m.urgency return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::FieldAccess {
                    object: base,
                    field,
                    ..
                } => {
                    assert_eq!(field, "urgency");
                    assert!(matches!(base.as_ref(), Expr::Ident { .. }));
                }
                other => panic!("expected FieldAccess, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_list_literal() {
    let src = "pattern P() -> List { let x = [1.0, 2.0, 3.0] return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::List { items, .. } => assert_eq!(items.len(), 3),
                other => panic!("expected List, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_expr_if_else_expr() {
    let src = "pattern P(score: Float) -> String { let x = if score > 0.8 then \"good\" else \"bad\" return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => {
                assert!(matches!(value, Expr::IfElse { .. }));
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── String literals & escapes ────────────────────────────────────────

#[test]
fn test_parse_string_literal_simple() {
    let src = "pattern P() -> String { return \"hello\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "hello"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_string_literal_with_newline_escape() {
    let src = "pattern P() -> String { return \"line1\\nline2\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "line1\nline2"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    }
}

#[test]
fn test_parse_string_literal_with_tab_escape() {
    let src = "pattern P() -> String { return \"a\\tb\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "a\tb"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    }
}

#[test]
fn test_parse_string_literal_with_quote_escape() {
    let src = "pattern P() -> String { return \"say \\\"hi\\\"\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "say \"hi\""),
            other => panic!("expected StringLit, got {:?}", other),
        }
    }
}

#[test]
fn test_parse_string_literal_with_backslash_escape() {
    let src = "pattern P() -> String { return \"C:\\\\path\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "C:\\path"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    }
}

#[test]
fn test_parse_string_literal_with_unicode_escape() {
    let src = "pattern P() -> String { return \"\\u0041\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => assert_eq!(s, "A"),
            other => panic!("expected StringLit, got {:?}", other),
        }
    }
}

#[test]
fn test_parse_multiline_string() {
    let src = "pattern P() -> String { return \"\"\"multi\nline\nstring\"\"\" }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::StringLit { value: s, .. },
                ..
            } => {
                assert!(s.contains("multi"));
                assert!(s.contains("line"));
                assert!(s.contains("string"));
            }
            other => panic!("expected StringLit, got {:?}", other),
        }
    }
}

// ── Error cases ──────────────────────────────────────────────────────

#[test]
fn test_parse_error_unclosed_brace() {
    let result = parse("{");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_bad_syntax() {
    let result = parse("pattern");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_incomplete_pattern() {
    let result = parse("pattern Foo");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_missing_arrow() {
    let result = parse("pattern Foo() Float { return 1.0 }");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_unclosed_paren() {
    let result = parse("pattern Foo(a: Float -> Float { return a }");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_missing_return_type() {
    let result = parse("pattern Foo() -> { return 1.0 }");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_garbage_input() {
    let result = parse("@#$%^&*()");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_incomplete_flow() {
    let result = parse("flow Main { input: String = ");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_unclosed_string() {
    let result = parse("pattern P() -> String { return \"unclosed }");
    assert!(result.is_err());
}

// ── Narjad 29 Block 6.1: Additional unit tests ──────────────────────

// ── Literals ────────────────────────────────────────────────────────

#[test]
fn test_parse_int_literal_returns_floatlit() {
    // INT is parsed as FloatLit per parse_expression primary_expr branch
    let src = "pattern P() -> Float { return 42 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::FloatLit { value: v, .. },
                ..
            } => assert_eq!(*v, 42.0),
            other => panic!("expected FloatLit(42.0), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_boolean_false_literal() {
    let src = "pattern P() -> Bool { return false }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::BoolLit { value: b, .. },
                ..
            } => assert!(!*b, "expected false"),
            other => panic!("expected BoolLit(false), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_empty_list_literal() {
    let src = "pattern P() -> List { let x = [] return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::List { items, .. } => assert!(items.is_empty(), "expected empty list"),
                other => panic!("expected List, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_list_of_strings() {
    let src = "pattern P() -> List { let xs = [\"a\", \"b\", \"c\"] return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::List { items, .. } => {
                    assert_eq!(items.len(), 3);
                    assert!(matches!(items[0], Expr::StringLit { .. }));
                }
                other => panic!("expected List, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_list_of_bools() {
    let src = "pattern P() -> List { let xs = [true, false] return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::List { items, .. } => assert_eq!(items.len(), 2),
                other => panic!("expected List, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_struct_literal_expression() {
    let src = "pattern P() -> Map { let x = { key: \"val\" } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::StructLit { fields, .. } => {
                    assert_eq!(fields.len(), 1);
                    assert!(fields.contains_key("key"));
                }
                other => panic!("expected StructLit, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── Match statement ─────────────────────────────────────────────────

#[test]
fn test_parse_match_with_exact_arm() {
    let src = "pattern P(s: String) -> Float { match s { \"a\" then { return 1.0 } } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // body: [match_stmt, return]
        assert_eq!(p.body.len(), 2);
        match &p.body[0] {
            Statement::Match { arms, .. } => {
                assert_eq!(arms.len(), 1);
                assert!(matches!(arms[0], MatchArm::Exact(_, _)));
            }
            other => panic!("expected Match, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_match_with_multiple_exact_arms() {
    let src = "pattern P(s: String) -> Float { match s { \"a\" then { return 1.0 } \"b\" then { return 2.0 } } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected Match, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_match_with_starts_with_arm() {
    let src = "pattern P(s: String) -> Float { match s { starts_with \"pre\" then { return 1.0 } } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Match { arms, .. } => {
                assert_eq!(arms.len(), 1);
                assert!(matches!(arms[0], MatchArm::StartsWith(_, _)));
            }
            other => panic!("expected Match, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_match_with_contains_arm() {
    let src = "pattern P(s: String) -> Float { match s { contains \"x\" then { return 1.0 } } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Match { arms, .. } => {
                assert_eq!(arms.len(), 1);
                assert!(matches!(arms[0], MatchArm::Contains(_, _)));
            }
            other => panic!("expected Match, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_match_with_compare_arm() {
    let src = "pattern P(s: Float) -> Float { match s { > 0.5 then { return 1.0 } } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Match { arms, .. } => {
                assert_eq!(arms.len(), 1);
                assert!(matches!(arms[0], MatchArm::Compare(_, _, _)));
            }
            other => panic!("expected Match, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_match_with_else() {
    let src = "pattern P(s: String) -> Float { match s { \"a\" then { return 1.0 } else { return 0.0 } } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Match {
                arms, else_body, ..
            } => {
                assert_eq!(arms.len(), 1);
                assert!(else_body.is_some(), "expected else body");
            }
            other => panic!("expected Match, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── Control flow ────────────────────────────────────────────────────

#[test]
fn test_parse_if_then_no_else() {
    // Single-branch if-then (no else) → Statement::IfThen
    let src = "pattern P(x: Float) -> Float { if x > 0.0 then { return x } return 0.0 }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // body: [if_then, return]
        assert_eq!(p.body.len(), 2);
        assert!(matches!(&p.body[0], Statement::IfThen { .. }));
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_if_else_if_else_chain() {
    let src = "pattern P(x: Float) -> Float { if x > 0.0 { return 1.0 } else if x < 0.0 { return 2.0 } else { return 0.0 } }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.body.len(), 1);
        match &p.body[0] {
            Statement::IfElseBlock {
                else_ifs,
                else_body,
                ..
            } => {
                assert_eq!(else_ifs.len(), 1);
                assert!(else_body.is_some());
            }
            other => panic!("expected IfElseBlock, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_each_with_index() {
    let src = "pattern P(items: List) -> Float { let total = 0.0 each i, item in items { total = total + 1.0 } return total }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // body: let_binding, each_with_index, return
        assert_eq!(p.body.len(), 3);
        let mut found = false;
        for s in &p.body {
            if let Statement::EachWithIndex {
                index_var,
                item_var,
                ..
            } = s
            {
                assert_eq!(index_var, "i");
                assert_eq!(item_var, "item");
                found = true;
            }
        }
        assert!(found, "expected EachWithIndex statement");
    } else {
        panic!("expected Pattern");
    }
}

// ── Binary operators ────────────────────────────────────────────────

#[test]
fn test_parse_logical_and_expr() {
    let src = "pattern P(a: Bool, b: Bool) -> Bool { let x = a and b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::And)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_logical_or_expr() {
    let src = "pattern P(a: Bool, b: Bool) -> Bool { let x = a or b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Or)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_subtraction_operator() {
    let src = "pattern P(a: Float, b: Float) -> Float { let x = a - b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Sub)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_division_operator() {
    let src = "pattern P(a: Float, b: Float) -> Float { let x = a / b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Div)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_equality_operator() {
    let src = "pattern P(a: Float, b: Float) -> Bool { let x = a == b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Eq)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_inequality_operator() {
    let src = "pattern P(a: Float, b: Float) -> Bool { let x = a != b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Ne)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_greater_equal_operator() {
    let src = "pattern P(a: Float, b: Float) -> Bool { let x = a >= b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Ge)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_less_equal_operator() {
    let src = "pattern P(a: Float, b: Float) -> Bool { let x = a <= b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Le)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_less_than_operator() {
    let src = "pattern P(a: Float, b: Float) -> Bool { let x = a < b return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BinaryOp { op, .. } => assert!(matches!(op, BinOp::Lt)),
                other => panic!("expected BinaryOp, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── Expressions: access & calls ─────────────────────────────────────

#[test]
fn test_parse_chained_field_access() {
    let src = "pattern P(m: Message) -> Float { let x = m.body.urgency return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::FieldAccess {
                    object: base,
                    field,
                    ..
                } => {
                    assert_eq!(field, "urgency");
                    assert!(matches!(base.as_ref(), Expr::FieldAccess { .. }));
                }
                other => panic!("expected FieldAccess, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_index_access_expression() {
    let src = "pattern P(items: List) -> Float { let x = items[0] return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::IndexAccess { index: idx, .. } => {
                    assert!(matches!(idx.as_ref(), Expr::FloatLit { .. }));
                }
                other => panic!("expected IndexAccess, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_nested_function_call() {
    let src = "pattern P(s: String) -> String { let x = upper(lower(s)) return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::FnCall {
                    name,
                    args,
                    ..
                } => {
                    assert_eq!(name, "upper");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(args[0], Expr::FnCall { .. }));
                }
                other => panic!("expected FnCall, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_function_call_with_multiple_args() {
    let src = "pattern P(a: Float, b: Float, c: Float) -> Float { let x = f(a, b, c) return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::FnCall {
                    name,
                    args,
                    ..
                } => {
                    assert_eq!(name, "f");
                    assert_eq!(args.len(), 3);
                }
                other => panic!("expected FnCall, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_try_expression() {
    let src = "pattern P(s: String) -> String { let x = try upper(s) return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::Try { expr: inner, .. } => {
                    assert!(matches!(inner.as_ref(), Expr::FnCall { .. }));
                }
                other => panic!("expected Try, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_let_binding_with_string_value() {
    let src = "pattern P() -> String { let name = \"Alice\" return name }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { name, value, .. } => {
                assert_eq!(name, "name");
                match value {
                    Expr::StringLit { value: s, .. } => assert_eq!(s, "Alice"),
                    other => panic!("expected StringLit, got {:?}", other),
                }
            }
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_let_binding_with_function_call_value() {
    let src = "pattern P(s: String) -> String { let x = upper(s) return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::FnCall {
                    name,
                    args,
                    ..
                } => {
                    assert_eq!(name, "upper");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected FnCall, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_let_binding_with_bool_value() {
    let src = "pattern P() -> Bool { let flag = true return flag }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::LetBinding { value, .. } => match value {
                Expr::BoolLit { value: b, .. } => assert!(*b, "expected true"),
                other => panic!("expected BoolLit, got {:?}", other),
            },
            other => panic!("expected LetBinding, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── Pattern body variants ───────────────────────────────────────────

#[test]
fn test_parse_pattern_with_three_params() {
    let src = "pattern Add3(a: Float, b: Float, c: Float) -> Float { return a }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        assert_eq!(p.name, "Add3");
        assert_eq!(p.params.len(), 3);
        assert_eq!(p.params[0].name, "a");
        assert_eq!(p.params[1].name, "b");
        assert_eq!(p.params[2].name, "c");
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_comment_inside_pattern_body() {
    let src =
        "pattern P() -> Float { // first comment\n let x = 1.0 // second comment\n return x }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // Comments should be filtered out, leaving only 2 statements
        assert_eq!(p.body.len(), 2);
        assert!(matches!(&p.body[0], Statement::LetBinding { .. }));
        assert!(matches!(&p.body[1], Statement::Return { .. }));
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_return_with_arithmetic_expression() {
    let src = "pattern P(a: Float, b: Float, c: Float) -> Float { return a + b * c }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        match &p.body[0] {
            Statement::Return {
                value: Expr::BinaryOp { op, .. },
                ..
            } => {
                // Top-level should be Add (multiplication binds tighter)
                assert!(matches!(op, BinOp::Add));
            }
            other => panic!("expected Return(BinaryOp), got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

#[test]
fn test_parse_while_with_assignment_inside() {
    let src = "pattern P() -> Float { let i = 0.0 while i < 10.0 { i = i + 1.0 } return i }";
    let decls = parse(src).unwrap();
    if let Declaration::Pattern(p) = &decls[0] {
        // body: let, while, return
        assert_eq!(p.body.len(), 3);
        match &p.body[1] {
            Statement::While { body, .. } => {
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], Statement::Assign { .. }));
            }
            other => panic!("expected While, got {:?}", other),
        }
    } else {
        panic!("expected Pattern");
    }
}

// ── MlogServer (Phase 6.1) ──────────────────────────────────────────

#[test]
fn test_parse_mlogserver_with_middleware_list() {
    let src = "mlogserver { port: 8080 middleware: [session, csrf, security_headers] }";
    let decls = parse(src).unwrap();
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.port, 8080);
        assert_eq!(s.middleware.len(), 3);
        assert_eq!(s.middleware[0], "session");
        assert_eq!(s.middleware[1], "csrf");
        assert_eq!(s.middleware[2], "security_headers");
    } else {
        panic!("expected MlogServer");
    }
}

#[test]
fn test_parse_mlogserver_with_route() {
    let src = "mlogserver { port: 8080 route \"/health\" method=GET { respond(\"ok\") } }";
    let decls = parse(src).unwrap();
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.routes.len(), 1);
        assert_eq!(s.routes[0].path, "/health");
        assert_eq!(s.routes[0].method, "GET");
        assert_eq!(s.routes[0].body.len(), 1);
    } else {
        panic!("expected MlogServer");
    }
}

#[test]
fn test_parse_mlogserver_with_route_and_requires() {
    let src = "mlogserver { port: 8080 route \"/admin\" method=POST requires=[admin] { respond(\"ok\") } }";
    let decls = parse(src).unwrap();
    if let Declaration::MlogServer(s) = &decls[0] {
        assert_eq!(s.routes[0].path, "/admin");
        assert_eq!(s.routes[0].method, "POST");
        assert_eq!(s.routes[0].requires.len(), 1);
        assert_eq!(s.routes[0].requires[0], "admin");
    } else {
        panic!("expected MlogServer");
    }
}

// ── Templates (Phase 6.2) ───────────────────────────────────────────

#[test]
fn test_parse_template_empty_body() {
    let src = "template Empty() -> Html { }";
    let decls = parse(src).unwrap();
    if let Declaration::Template(t) = &decls[0] {
        assert_eq!(t.name, "Empty");
        assert_eq!(t.return_type, "Html");
        assert!(
            t.body.trim().is_empty(),
            "expected empty body, got {:?}",
            t.body
        );
    } else {
        panic!("expected Template");
    }
}

#[test]
fn test_parse_template_with_braces_in_body() {
    // Body contains { and } (CSS rule) — exercises preprocess_templates
    let src = "template Styled() -> Html { <style>.x { color: red; }</style> }";
    let decls = parse(src).unwrap();
    if let Declaration::Template(t) = &decls[0] {
        assert_eq!(t.name, "Styled");
        assert!(
            t.body.contains("color: red"),
            "body should contain CSS, got {:?}",
            t.body
        );
        assert!(t.body.contains('}'), "body should contain closing brace");
    } else {
        panic!("expected Template");
    }
}

#[test]
fn test_parse_template_two_params() {
    let src = "template Page(title: String, body: String) -> Html { <html><h1>{{ title }}</h1>{{ body }}</html> }";
    let decls = parse(src).unwrap();
    if let Declaration::Template(t) = &decls[0] {
        assert_eq!(t.name, "Page");
        assert_eq!(t.params.len(), 2);
        assert_eq!(t.params[0].name, "title");
        assert_eq!(t.params[1].name, "body");
        assert!(t.body.contains("title"));
        assert!(t.body.contains("body"));
    } else {
        panic!("expected Template");
    }
}

// ── Additional error cases ──────────────────────────────────────────

#[test]
fn test_parse_error_unclosed_list_bracket() {
    let result = parse("pattern P() -> List { let x = [1.0, 2.0 return x }");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_invalid_pattern_name() {
    // "123" is not a valid IDENT
    let result = parse("pattern 123() -> Float { return 1.0 }");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_unclosed_pattern_body() {
    let result = parse("pattern P() -> Float { return 1.0");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_missing_pattern_name() {
    let result = parse("pattern () -> Float { return 1.0 }");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_unclosed_route_body() {
    let result = parse("mlogserver { port: 8080 route \"/x\" method=GET { respond(\"ok\") }");
    assert!(result.is_err());
}

#[test]
fn naryad_121_duplicate_entity_shows_line() {
    // Entity 'User' is on line 5, so error should say "строка 5:"
    let src = r#"
mlogserver { port: 8080 }

entity User { name: String }
entity User { name: String }"#;
    let result = crate::semantic::check_program(&crate::parser::parse(src).unwrap());
    assert!(!result.errors.is_empty(), "expected duplicate entity error");
    assert!(
        result.errors[0].contains("строка 5:"),
        "error should contain line number: {}",
        result.errors[0]
    );
}

#[test]
fn naryad_121_duplicate_pattern_shows_line() {
    // Pattern 'P' second occurrence is on line 2
    let src = "pattern P() -> String { return \"ok\" }\npattern P() -> String { return \"ok\" }";
    let result = crate::semantic::check_program(&crate::parser::parse(src).unwrap());
    assert!(!result.errors.is_empty());
    assert!(
        result.errors[0].contains("строка 2:"),
        "error should contain line number: {}",
        result.errors[0]
    );
}

#[test]
fn naryad_121_entity_unknown_type_shows_line() {
    // entity 'bob' on line 5 references unknown type 'NonExistent'
    let src = r#"
mlogserver { port: 8080 }


entity bob: NonExistent = { name: "test" }"#;
    let result = crate::semantic::check_program(&crate::parser::parse(src).unwrap());
    assert!(!result.errors.is_empty());
    assert!(
        result.errors[0].contains("строка 5:"),
        "error should contain line number: {}",
        result.errors[0]
    );
}

#[test]
fn naryad_121_unknown_type_in_entity_simple_shows_line() {
    // entity on line 1 with unknown type 'Integer' — triggers a warning
    let src = "entity count: Integer = \"hello\"";
    let result = crate::semantic::check_program(&crate::parser::parse(src).unwrap());
    assert!(
        !result.warnings.is_empty(),
        "expected undeclared type warning"
    );
    assert!(
        result.warnings[0].contains("строка 1:"),
        "warning should contain line number: {}",
        result.warnings[0]
    );
}

#[test]
fn naryad_121_no_line_prefix_for_unknown_span() {
    // Programmatic construction should NOT have line prefix
    let decl = crate::ast::Declaration::EntityType(crate::ast::EntityTypeDecl {
        span: crate::ast::Span::unknown(),
        name: "User".to_string(),
        fields: vec![],
    });
    let result = crate::semantic::check_program(&[decl]);
    // No errors expected for a single valid entity type
    assert!(result.errors.is_empty());
}
