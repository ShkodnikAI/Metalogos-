// ── ADR-0053: Conversation State — Contract Tests ─────────────────────
// All tests use valid .mlog syntax.
// Tests must run with --test-threads=1 (shared static state in MockLlm).

use metalogos::ast::{ConversationDecl, Declaration, LearnablePatternDecl, Param};
use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run declarations, return interpreter.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    interp.run(declarations)?;
    Ok(interp)
}

// ── C1: conv_start creates a conversation ──────────────────────────────

#[test]
fn test_conv_start_creates_conversation() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    // Set conversation config
    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        })])
        .unwrap();

    // conv_start via entity assignment triggers the builtin
    let source = r#"
entity _start: String = conv_start("chat_1")
"#;
    interp.run(parser::parse(source).unwrap()).unwrap();

    let convs = interp.get_conversations().lock().unwrap();
    assert!(
        convs.contains_key("chat_1"),
        "conversation should exist after conv_start"
    );
    assert_eq!(
        convs["chat_1"].messages.len(),
        0,
        "new conversation should be empty"
    );
}

// ── C2: conv_add adds messages, verified via conversations lock ────────

#[test]
fn test_conv_add_and_history() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        })])
        .unwrap();

    let source = r#"
entity _s: String = conv_start("chat_1")
entity _a1: String = conv_add("chat_1", "user", "Привет!")
entity _a2: String = conv_add("chat_1", "assistant", "Здравствуйте!")
entity _a3: String = conv_add("chat_1", "user", "Как дела?")
"#;
    interp.run(parser::parse(source).unwrap()).unwrap();

    let convs = interp.get_conversations().lock().unwrap();
    assert_eq!(convs["chat_1"].messages.len(), 3);
    assert_eq!(convs["chat_1"].messages[0].role, "user");
    assert_eq!(convs["chat_1"].messages[0].text, "Привет!");
    assert_eq!(convs["chat_1"].messages[1].role, "assistant");
    assert_eq!(convs["chat_1"].messages[1].text, "Здравствуйте!");
    assert_eq!(convs["chat_1"].messages[2].role, "user");
    assert_eq!(convs["chat_1"].messages[2].text, "Как дела?");
}

// ── C3: conv_history returns List<Struct> with role, text, timestamp ──

#[test]
fn test_conv_history_struct_fields() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        })])
        .unwrap();

    let source = r#"
entity _s: String = conv_start("hist_test")
entity _a: String = conv_add("hist_test", "user", "hello")
entity _h: String = conv_context("hist_test")
"#;
    interp.run(parser::parse(source).unwrap()).unwrap();

    let convs = interp.get_conversations().lock().unwrap();
    let conv = &convs["hist_test"];
    assert_eq!(conv.messages.len(), 1);
    assert_eq!(conv.messages[0].role, "user");
    assert_eq!(conv.messages[0].text, "hello");
    assert!(
        conv.messages[0].timestamp > 0,
        "timestamp should be positive"
    );
}

// ── C4: conv_context returns formatted string with all messages ────────

#[test]
fn test_conv_context_formatting() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        })])
        .unwrap();

    let source = r#"
entity _s: String = conv_start("ctx_test")
entity _a1: String = conv_add("ctx_test", "user", "Q1")
entity _a2: String = conv_add("ctx_test", "assistant", "A1")
entity ctx: String = conv_context("ctx_test")
"#;
    let interp = run_source(source).unwrap();

    // conv_context returns a formatted string of all messages
    let ctx_val = interp.get_variable("ctx").unwrap();
    let ctx_str = format!("{}", ctx_val);
    assert!(
        ctx_str.contains("user: Q1"),
        "context should contain user message"
    );
    assert!(
        ctx_str.contains("assistant: A1"),
        "context should contain assistant message"
    );
}

// ── C5: conv_end removes conversation ─────────────────────────────────

#[test]
fn test_conv_end_removes_conversation() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        })])
        .unwrap();

    let source = r#"
entity _s: String = conv_start("end_test")
entity _a: String = conv_add("end_test", "user", "bye")
entity _e: String = conv_end("end_test")
"#;
    interp.run(parser::parse(source).unwrap()).unwrap();

    let convs = interp.get_conversations().lock().unwrap();
    assert!(
        !convs.contains_key("end_test"),
        "conversation should be removed after conv_end"
    );
}

// ── C6: conversation config is applied from declaration ──────────────

#[test]
fn test_conversation_config_from_declaration() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 900,
            max_messages: 10,
            compress_after: 5,
        })])
        .unwrap();

    let config = interp.get_conversation_config();
    assert_eq!(config.ttl, 900);
    assert_eq!(config.max_messages, 10);
    assert_eq!(config.compress_after, 5);
}

// ── C7: max_messages enforced — oldest message removed ────────────────

#[test]
fn test_max_messages_enforced() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    // max_messages = 3, compress_after = 100 (disable compression)
    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 3,
            compress_after: 100,
        })])
        .unwrap();

    let source = r#"
entity _s: String = conv_start("max_test")
entity _a1: String = conv_add("max_test", "user", "msg1")
entity _a2: String = conv_add("max_test", "user", "msg2")
entity _a3: String = conv_add("max_test", "user", "msg3")
entity _a4: String = conv_add("max_test", "user", "msg4")
"#;
    interp.run(parser::parse(source).unwrap()).unwrap();

    let convs = interp.get_conversations().lock().unwrap();
    let conv = &convs["max_test"];
    // Should have at most 3 messages (msg2, msg3, msg4; msg1 evicted)
    assert_eq!(conv.messages.len(), 3, "should enforce max_messages");
    assert_eq!(
        conv.messages[0].text, "msg2",
        "oldest message should have been evicted"
    );
    assert_eq!(conv.messages[2].text, "msg4");
}

// ── C8: conversation config defaults when no declaration ────────────

#[test]
fn test_conversation_config_defaults() {
    let interp = Interpreter::new();
    let config = interp.get_conversation_config();
    assert_eq!(config.ttl, 1800, "default ttl should be 1800");
    assert_eq!(config.max_messages, 50, "default max_messages should be 50");
    assert_eq!(
        config.compress_after, 20,
        "default compress_after should be 20"
    );
}

// ── C9: conversation: "current" parsed in learnable pattern ────────────

#[test]
fn test_conversation_field_in_learnable_pattern() {
    let source = r#"
learnable pattern Reply(text: String) -> String {
    prompt: "You are a helpful assistant"
    conversation: "current"
}
"#;
    let declarations = parser::parse(source).unwrap();
    assert_eq!(declarations.len(), 1);

    match &declarations[0] {
        Declaration::LearnablePattern(lp) => {
            assert_eq!(lp.name, "Reply");
            assert_eq!(lp.conversation.as_deref(), Some("current"));
        }
        _ => panic!("expected LearnablePattern declaration"),
    }
}

// ── C10: multiple conversations are independent ──────────────────────

#[test]
fn test_multiple_conversations_independent() {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    interp
        .run(vec![Declaration::Conversation(ConversationDecl {
            ttl: 1800,
            max_messages: 50,
            compress_after: 100,
        })])
        .unwrap();

    let source = r#"
entity _s1: String = conv_start("conv_a")
entity _s2: String = conv_start("conv_b")
entity _a1: String = conv_add("conv_a", "user", "hello A")
entity _a2: String = conv_add("conv_b", "user", "hello B")
"#;
    interp.run(parser::parse(source).unwrap()).unwrap();

    let convs = interp.get_conversations().lock().unwrap();
    assert_eq!(convs.len(), 2);
    assert_eq!(convs["conv_a"].messages.len(), 1);
    assert_eq!(convs["conv_a"].messages[0].text, "hello A");
    assert_eq!(convs["conv_b"].messages.len(), 1);
    assert_eq!(convs["conv_b"].messages[0].text, "hello B");
}
