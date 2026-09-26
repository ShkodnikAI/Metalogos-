//! №466 (gh#687) — the sessions transfer group: the shared live module.
//!
//! The third group of the TW/VM dedup (gate gh#680, decision 4-A, step 3;
//! the CI threshold gate is №462/gh#683, the diff-fuzzer is №465/gh#686).
//! Seven builtin names — `conv_start`, `conv_add`, `conv_history`,
//! `conv_context`, `conv_end`, `consent_grant`, `consent_revoke` — moved
//! OUT of both backends: `src/vm.rs` and `src/interpreter/` keep their
//! exact per-site marshaling hooks (const-name checks in the SAME dispatch
//! order as before) and the bodies live here. After the move the name
//! literals appear only in this module, so the №462 counter drops 49 → 42.
//!
//! The module is the shared HOME, not a unification (the group 2 rule):
//! the TW and the VM `conv_add` lanes stay deliberately separate where
//! their behavior differs — the TW runs the ADR-0053 auto-compression
//! tail (LLM summarization through the interpreter's SmartRouter, the
//! condition `messages.len() > compress_after` checked AFTER the push and
//! the `last_active` update) and the VM does not. The shared body takes
//! that tail as a `FnOnce(&mut Conversation)`; the TW site passes its
//! compression callback (the condition lives there — it reads the
//! interpreter's config), the VM site passes a no-op. Every per-backend
//! form below is a verbatim transplant; the genuinely shared pieces
//! factored here once are the full conv bodies themselves (they were
//! line-for-line identical between the backends) and the timestamp
//! helper. The consent pair never had duplicated bodies — both backends
//! already delegate to `crate::builtins::consent` — so the transfer here
//! is the name literals (the constants below) plus the `handles()` hook.
//!
//! The live-contract requirement of the naryad (the owner's
//! "revive-or-delete" strengthening for the dead `RuntimeContext`)
//! continues the group 1/2 posture: no hidden global and no revived dead
//! struct — each backend passes its own conversations store and config
//! explicitly, the bodies are unit-tested here over plain stores without
//! either backend, and the divergent tail is an injected closure (the
//! mock-tested seam).

use crate::interpreter::{ConvMessage, Conversation, ConversationConfig, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The sessions-group names this module owns. The backends compare their
/// dispatch names against the constants below — the name strings are
/// spelled here and nowhere else outside the registry.
pub const NAME_CONV_START: &str = "conv_start";
pub const NAME_CONV_ADD: &str = "conv_add";
pub const NAME_CONV_HISTORY: &str = "conv_history";
pub const NAME_CONV_CONTEXT: &str = "conv_context";
pub const NAME_CONV_END: &str = "conv_end";
pub const NAME_CONSENT_GRANT: &str = "consent_grant";
pub const NAME_CONSENT_REVOKE: &str = "consent_revoke";

/// The seven sessions-group names this module owns, as a single hook.
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        NAME_CONV_START
            | NAME_CONV_ADD
            | NAME_CONV_HISTORY
            | NAME_CONV_CONTEXT
            | NAME_CONV_END
            | NAME_CONSENT_GRANT
            | NAME_CONSENT_REVOKE
    )
}

/// The wall-clock seconds both backends used inline (verbatim transplant:
/// `as_secs() as i64`, epoch-underscore fallback 0).
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `conv_start(id)` — create or open a conversation. Returns the
/// conversation id. Existing conversations are left untouched
/// (`or_insert_with`), exactly as both backends did inline.
pub fn conv_start(
    args: &[Value],
    convs: &Mutex<HashMap<String, Conversation>>,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{}() requires 1 argument (id: String)",
                NAME_CONV_START
            ))
        }
    };
    let now = now_secs();
    let mut convs = convs
        .lock()
        .map_err(|e| format!("{}() lock error: {}", NAME_CONV_START, e))?;
    convs.entry(id.clone()).or_insert_with(|| Conversation {
        id: id.clone(),
        messages: Vec::new(),
        created_at: now,
        last_active: now,
        metadata: HashMap::new(),
    });
    Ok(Value::String(id))
}

/// `conv_add(id, role, text)` — add a message to a conversation.
///
/// The shared core is the evict-at-`max_messages` + push + `last_active`
/// sequence, identical in both backends. The backend's own tail arrives
/// as `compress_tail` and runs AFTER the push and the `last_active`
/// update (the original TW order): the TW passes the ADR-0053
/// auto-compression callback (its condition lives at the call site, where
/// the interpreter's config is at hand), the VM passes a no-op. A
/// non-string third argument is coerced with `format!` (both backends
/// did); the error texts are byte-identical to the inline originals.
pub fn conv_add(
    args: &[Value],
    convs: &Mutex<HashMap<String, Conversation>>,
    config: &ConversationConfig,
    compress_tail: impl FnOnce(&mut Conversation),
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{}() requires 3 arguments (id, role, text)",
                NAME_CONV_ADD
            ))
        }
    };
    let role = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{}() requires 3 arguments (id, role, text)",
                NAME_CONV_ADD
            ))
        }
    };
    let text = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => {
            return Err(format!(
                "{}() requires 3 arguments (id, role, text)",
                NAME_CONV_ADD
            ))
        }
    };
    let now = now_secs();

    let mut convs = convs
        .lock()
        .map_err(|e| format!("{}() lock error: {}", NAME_CONV_ADD, e))?;
    let conv = convs
        .get_mut(&id)
        .ok_or_else(|| format!("{}() conversation '{}' not found", NAME_CONV_ADD, id))?;

    // Enforce max_messages: if at limit, remove oldest message
    if conv.messages.len() >= config.max_messages {
        conv.messages.remove(0);
    }

    conv.messages.push(ConvMessage {
        role,
        text: text.clone(),
        timestamp: now,
    });
    conv.last_active = now;

    // The backend's own tail: TW auto-compresses (ADR-0053), VM does not.
    compress_tail(conv);

    Ok(Value::String(text))
}

/// `conv_history(id)` — return the full message history as a List of
/// `Message` structs (role/text/timestamp), the shape both backends
/// built inline.
pub fn conv_history(
    args: &[Value],
    convs: &Mutex<HashMap<String, Conversation>>,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{}() requires 1 argument (id: String)",
                NAME_CONV_HISTORY
            ))
        }
    };
    let convs = convs
        .lock()
        .map_err(|e| format!("{}() lock error: {}", NAME_CONV_HISTORY, e))?;
    let conv = convs
        .get(&id)
        .ok_or_else(|| format!("{}() conversation '{}' not found", NAME_CONV_HISTORY, id))?;

    let mut list = Vec::new();
    for msg in &conv.messages {
        let mut fields = HashMap::new();
        fields.insert("role".to_string(), Value::String(msg.role.clone()));
        fields.insert("text".to_string(), Value::String(msg.text.clone()));
        fields.insert("timestamp".to_string(), Value::Float(msg.timestamp as f64));
        list.push(Value::Struct {
            type_name: "Message".to_string(),
            fields,
        });
    }
    Ok(Value::List(list))
}

/// `conv_context(id)` — return the conversation history formatted as
/// `"{role}: {text}"` lines joined by newlines (the LLM-injection shape
/// both backends built inline).
pub fn conv_context(
    args: &[Value],
    convs: &Mutex<HashMap<String, Conversation>>,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{}() requires 1 argument (id: String)",
                NAME_CONV_CONTEXT
            ))
        }
    };
    let convs = convs
        .lock()
        .map_err(|e| format!("{}() lock error: {}", NAME_CONV_CONTEXT, e))?;
    let conv = convs
        .get(&id)
        .ok_or_else(|| format!("{}() conversation '{}' not found", NAME_CONV_CONTEXT, id))?;

    let mut parts = Vec::new();
    for msg in &conv.messages {
        parts.push(format!("{}: {}", msg.role, msg.text));
    }
    Ok(Value::String(parts.join("\n")))
}

/// `conv_end(id)` — terminate a conversation. Returns "ok"; removing a
/// missing conversation is still "ok" (both backends removed unconditionally).
pub fn conv_end(
    args: &[Value],
    convs: &Mutex<HashMap<String, Conversation>>,
) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(format!(
                "{}() requires 1 argument (id: String)",
                NAME_CONV_END
            ))
        }
    };
    let mut convs = convs
        .lock()
        .map_err(|e| format!("{}() lock error: {}", NAME_CONV_END, e))?;
    convs.remove(&id);
    Ok(Value::String("ok".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn store() -> Mutex<HashMap<String, Conversation>> {
        Mutex::new(HashMap::new())
    }

    fn config(max_messages: usize, compress_after: usize) -> ConversationConfig {
        ConversationConfig {
            ttl: 1800,
            max_messages,
            compress_after,
        }
    }

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }

    // ── conv_start ──────────────────────────────────────────────────────

    #[test]
    fn conv_start_creates_and_returns_id() {
        let convs = store();
        let out = conv_start(&[s("chat-1")], &convs).unwrap();
        assert!(matches!(&out, Value::String(v) if v == "chat-1"));
        let map = convs.lock().unwrap();
        let conv = map.get("chat-1").expect("conversation created");
        assert_eq!(conv.id, "chat-1");
        assert!(conv.messages.is_empty());
    }

    #[test]
    fn conv_start_is_idempotent_for_existing_conversation() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        conv_add(
            &[s("chat-1"), s("user"), s("hello")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap();
        conv_start(&[s("chat-1")], &convs).unwrap();
        let map = convs.lock().unwrap();
        assert_eq!(map.get("chat-1").unwrap().messages.len(), 1);
    }

    #[test]
    fn conv_start_rejects_missing_and_non_string_id() {
        let convs = store();
        let err = conv_start(&[], &convs).unwrap_err();
        assert_eq!(err, "conv_start() requires 1 argument (id: String)");
        let err = conv_start(&[Value::Float(1.0)], &convs).unwrap_err();
        assert_eq!(err, "conv_start() requires 1 argument (id: String)");
    }

    // ── conv_add ────────────────────────────────────────────────────────

    #[test]
    fn conv_add_appends_and_coerces_non_string_text() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        let out = conv_add(
            &[s("chat-1"), s("user"), Value::Float(1.5)],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap();
        assert!(matches!(&out, Value::String(v) if v == "1.5"));
        let map = convs.lock().unwrap();
        let msg = &map.get("chat-1").unwrap().messages[0];
        assert_eq!(msg.role, "user");
        assert_eq!(msg.text, "1.5");
    }

    #[test]
    fn conv_add_rejects_bad_arguments_with_exact_texts() {
        let convs = store();
        let err = conv_add(&[s("chat-1")], &convs, &config(50, 20), |_| {}).unwrap_err();
        assert_eq!(err, "conv_add() requires 3 arguments (id, role, text)");
        let err = conv_add(
            &[s("chat-1"), Value::Float(2.0), s("hi")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err, "conv_add() requires 3 arguments (id, role, text)");
        let err = conv_add(&[s("chat-1"), s("user")], &convs, &config(50, 20), |_| {}).unwrap_err();
        assert_eq!(err, "conv_add() requires 3 arguments (id, role, text)");
    }

    #[test]
    fn conv_add_unknown_conversation_error_text_is_exact() {
        let convs = store();
        let err = conv_add(
            &[s("ghost"), s("user"), s("hi")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err, "conv_add() conversation 'ghost' not found");
    }

    #[test]
    fn conv_add_evicts_oldest_message_at_max_messages() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        let cfg = config(2, 20);
        conv_add(&[s("chat-1"), s("user"), s("m1")], &convs, &cfg, |_| {}).unwrap();
        conv_add(&[s("chat-1"), s("user"), s("m2")], &convs, &cfg, |_| {}).unwrap();
        conv_add(&[s("chat-1"), s("user"), s("m3")], &convs, &cfg, |_| {}).unwrap();
        let map = convs.lock().unwrap();
        let messages = &map.get("chat-1").unwrap().messages;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "m2");
        assert_eq!(messages[1].text, "m3");
    }

    #[test]
    fn conv_add_runs_the_injected_backend_tail() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        let tail_calls = Arc::new(AtomicUsize::new(0));
        let calls = Arc::clone(&tail_calls);
        conv_add(
            &[s("chat-1"), s("user"), s("m1")],
            &convs,
            &config(50, 20),
            |conv| {
                calls.fetch_add(1, Ordering::SeqCst);
                // The tail observes the conversation AFTER the push and the
                // last_active update (the original TW order).
                assert_eq!(conv.messages.len(), 1);
            },
        )
        .unwrap();
        assert_eq!(tail_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn conv_add_updates_last_active() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        let created_at = convs.lock().unwrap().get("chat-1").unwrap().created_at;
        conv_add(
            &[s("chat-1"), s("user"), s("m1")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap();
        let map = convs.lock().unwrap();
        let conv = map.get("chat-1").unwrap();
        assert_eq!(conv.created_at, created_at);
        assert!(conv.last_active >= conv.created_at);
    }

    // ── conv_history ────────────────────────────────────────────────────

    #[test]
    fn conv_history_returns_message_structs() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        conv_add(
            &[s("chat-1"), s("user"), s("hello")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap();
        let out = conv_history(&[s("chat-1")], &convs).unwrap();
        match out {
            Value::List(list) => {
                assert_eq!(list.len(), 1);
                match &list[0] {
                    Value::Struct { type_name, fields } => {
                        assert_eq!(type_name, "Message");
                        assert!(
                            matches!(fields.get("role"), Some(Value::String(v)) if v == "user")
                        );
                        assert!(
                            matches!(fields.get("text"), Some(Value::String(v)) if v == "hello")
                        );
                        assert!(matches!(fields.get("timestamp"), Some(Value::Float(_))));
                    }
                    other => panic!("expected struct, got {:?}", other),
                }
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn conv_history_unknown_conversation_error_text_is_exact() {
        let convs = store();
        let err = conv_history(&[s("ghost")], &convs).unwrap_err();
        assert_eq!(err, "conv_history() conversation 'ghost' not found");
    }

    #[test]
    fn conv_history_rejects_missing_id() {
        let convs = store();
        let err = conv_history(&[], &convs).unwrap_err();
        assert_eq!(err, "conv_history() requires 1 argument (id: String)");
    }

    // ── conv_context ────────────────────────────────────────────────────

    #[test]
    fn conv_context_joins_role_and_text_lines() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        conv_add(
            &[s("chat-1"), s("user"), s("hi")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap();
        conv_add(
            &[s("chat-1"), s("assistant"), s("hello")],
            &convs,
            &config(50, 20),
            |_| {},
        )
        .unwrap();
        let out = conv_context(&[s("chat-1")], &convs).unwrap();
        assert!(matches!(&out, Value::String(v) if v == "user: hi\nassistant: hello"));
    }

    #[test]
    fn conv_context_unknown_conversation_error_text_is_exact() {
        let convs = store();
        let err = conv_context(&[s("ghost")], &convs).unwrap_err();
        assert_eq!(err, "conv_context() conversation 'ghost' not found");
    }

    // ── conv_end ────────────────────────────────────────────────────────

    #[test]
    fn conv_end_removes_and_returns_ok() {
        let convs = store();
        conv_start(&[s("chat-1")], &convs).unwrap();
        let out = conv_end(&[s("chat-1")], &convs).unwrap();
        assert!(matches!(&out, Value::String(v) if v == "ok"));
        assert!(convs.lock().unwrap().get("chat-1").is_none());
    }

    #[test]
    fn conv_end_of_missing_conversation_is_still_ok() {
        let convs = store();
        let out = conv_end(&[s("ghost")], &convs).unwrap();
        assert!(matches!(&out, Value::String(v) if v == "ok"));
    }

    #[test]
    fn conv_end_rejects_missing_id() {
        let convs = store();
        let err = conv_end(&[], &convs).unwrap_err();
        assert_eq!(err, "conv_end() requires 1 argument (id: String)");
    }

    // ── the name contract ───────────────────────────────────────────────

    #[test]
    fn handles_covers_exactly_the_seven_names() {
        for name in [
            NAME_CONV_START,
            NAME_CONV_ADD,
            NAME_CONV_HISTORY,
            NAME_CONV_CONTEXT,
            NAME_CONV_END,
            NAME_CONSENT_GRANT,
            NAME_CONSENT_REVOKE,
        ] {
            assert!(handles(name), "{} must be covered", name);
        }
        assert!(!handles("conv"));
        assert!(!handles("session_start"));
        assert!(!handles("consent_check"));
    }

    #[test]
    fn name_constants_match_the_registry_spelling() {
        // The constants are the single spelling of the group outside the
        // registry; the values must stay byte-identical to the historical
        // inline literals (a rename is an owner-gated change, not a
        // transfer).
        assert_eq!(NAME_CONV_START, "conv_start");
        assert_eq!(NAME_CONV_ADD, "conv_add");
        assert_eq!(NAME_CONV_HISTORY, "conv_history");
        assert_eq!(NAME_CONV_CONTEXT, "conv_context");
        assert_eq!(NAME_CONV_END, "conv_end");
        assert_eq!(NAME_CONSENT_GRANT, "consent_grant");
        assert_eq!(NAME_CONSENT_REVOKE, "consent_revoke");
    }
}
