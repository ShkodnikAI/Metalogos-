use super::*;
use crate::llm;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

impl Interpreter {
    // ── ADR-0053: Conversation builtins ──────────────────────────────────
    // №466 (gh#687) group 3 (sessions): the five conv bodies live in the
    // shared live module (src/session_ops.rs); these sites marshal the
    // interpreter's own store/config only. The TW conv_add lane keeps its
    // ADR-0053 auto-compression tail (the condition `messages.len() >
    // compress_after` reads this interpreter's config) — the VM lane passes
    // a no-op tail there.

    /// `conv_start(id)` — create or open a conversation. Returns the conversation id.
    pub(super) fn invoke_conv_start(&self, args: &[Value]) -> Result<Value, String> {
        crate::session_ops::conv_start(args, &self.conversations)
    }

    /// `conv_add(id, role, text)` — add a message to a conversation.
    pub(super) fn invoke_conv_add(&self, args: &[Value]) -> Result<Value, String> {
        crate::session_ops::conv_add(
            args,
            &self.conversations,
            &self.conversation_config,
            |conv| {
                // ADR-0053: auto-compress when message count exceeds compress_after
                if conv.messages.len() > self.conversation_config.compress_after {
                    self.compress_conversation(conv);
                }
            },
        )
    }

    /// `conv_history(id)` — return the full message history as a List of Structs.
    pub(super) fn invoke_conv_history(&self, args: &[Value]) -> Result<Value, String> {
        crate::session_ops::conv_history(args, &self.conversations)
    }

    /// `conv_context(id)` — return a formatted string of conversation history for LLM injection.
    pub(super) fn invoke_conv_context(&self, args: &[Value]) -> Result<Value, String> {
        crate::session_ops::conv_context(args, &self.conversations)
    }

    /// `conv_end(id)` — terminate a conversation. Returns "ok".
    pub(super) fn invoke_conv_end(&self, args: &[Value]) -> Result<Value, String> {
        crate::session_ops::conv_end(args, &self.conversations)
    }

    /// Get a reference to the conversations store (for testing).
    pub fn get_conversations(&self) -> &std::sync::Mutex<HashMap<String, Conversation>> {
        &self.conversations
    }

    /// Get conversation config (for testing).
    pub fn get_conversation_config(&self) -> &ConversationConfig {
        &self.conversation_config
    }

    /// Compress older messages in a conversation by summarizing them via LLM.
    /// Replaces messages beyond compress_after with a single system summary message.
    fn compress_conversation(&self, conv: &mut Conversation) {
        if conv.messages.len() <= self.conversation_config.compress_after {
            return;
        }
        let old_count = conv.messages.len() - self.conversation_config.compress_after;
        let old_messages: Vec<ConvMessage> = conv.messages.drain(..old_count).collect();

        // Build text from old messages for summarization
        let old_text: Vec<String> = old_messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.text))
            .collect();
        let text_to_summarize = old_text.join("\n");

        // Attempt LLM summarization. On failure, keep a simple prefix summary.
        let summary = match self.summarize_conversation(&text_to_summarize) {
            Ok(s) => s,
            Err(_) => format!(
                "[Previous conversation summary: {} messages omitted]",
                old_count
            ),
        };

        // Prepend summary as a system message
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        conv.messages.insert(
            0,
            ConvMessage {
                role: "system".to_string(),
                text: summary,
                timestamp: now,
            },
        );
    }

    /// Summarize conversation text via LLM call.
    /// Наряд #156: routes through SmartRouter when available (same as learnable calls).
    fn summarize_conversation(&self, text: &str) -> Result<String, String> {
        let prompt = "Summarize this conversation concisely, preserving key facts and decisions.";
        // Route through SmartRouter if configured, otherwise legacy backend.
        if let Ok(guard) = self.smart_router.lock() {
            if let Some(ref router) = *guard {
                return router.call(prompt, text, None, None);
            }
        }
        // Наряд №276: legacy fallback traced here (the SmartRouter path
        // above traces inside SmartRouter::call — one line per actual call).
        let t0 = std::time::Instant::now();
        let result = {
            let backend = llm::create_llm_backend();
            backend.call(prompt, text)
        };
        let mock_mode = crate::llm::mock_llm_requested();
        crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
            provider_name: Some(if mock_mode {
                "mock"
            } else {
                llm::provider_env_name()
            }),
            model: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: t0.elapsed().as_millis() as u64,
            status: if result.is_ok() { "ok" } else { "error" },
            cache: "miss",
            provider_alias: None,
        });
        result
    }

    /// Get conversation history as a formatted string for LLM multi-turn injection.
    /// Returns None if conversation not found or empty.
    pub fn get_conversation_for_llm(&self, conv_id: &str) -> Option<String> {
        let convs = self.conversations.lock().ok()?;
        let conv = convs.get(conv_id)?;
        if conv.messages.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for msg in &conv.messages {
            parts.push(format!("{}: {}", msg.role, msg.text));
        }
        Some(parts.join("\n"))
    }
}
