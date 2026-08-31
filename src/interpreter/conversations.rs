use super::*;
use crate::llm;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

impl Interpreter {
    // ── ADR-0053: Conversation builtins ──────────────────────────────────

    /// `conv_start(id)` — create or open a conversation. Returns the conversation id.
    pub(super) fn invoke_conv_start(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_start() requires 1 argument (id: String)".to_string()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut convs = self
            .conversations
            .lock()
            .map_err(|e| format!("conv_start() lock error: {}", e))?;
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
    pub(super) fn invoke_conv_add(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_add() requires 3 arguments (id, role, text)".to_string()),
        };
        let role = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_add() requires 3 arguments (id, role, text)".to_string()),
        };
        let text = match args.get(2) {
            Some(Value::String(s)) => s.clone(),
            Some(other) => format!("{}", other),
            None => return Err("conv_add() requires 3 arguments (id, role, text)".to_string()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut convs = self
            .conversations
            .lock()
            .map_err(|e| format!("conv_add() lock error: {}", e))?;
        let conv = convs
            .get_mut(&id)
            .ok_or_else(|| format!("conv_add() conversation '{}' not found", id))?;

        // Enforce max_messages: if at limit, remove oldest message
        if conv.messages.len() >= self.conversation_config.max_messages {
            conv.messages.remove(0);
        }

        conv.messages.push(ConvMessage {
            role,
            text: text.clone(),
            timestamp: now,
        });
        conv.last_active = now;

        // ADR-0053: auto-compress when message count exceeds compress_after
        if conv.messages.len() > self.conversation_config.compress_after {
            self.compress_conversation(conv);
        }

        Ok(Value::String(text))
    }

    /// `conv_history(id)` — return the full message history as a List of Structs.
    pub(super) fn invoke_conv_history(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_history() requires 1 argument (id: String)".to_string()),
        };
        let convs = self
            .conversations
            .lock()
            .map_err(|e| format!("conv_history() lock error: {}", e))?;
        let conv = convs
            .get(&id)
            .ok_or_else(|| format!("conv_history() conversation '{}' not found", id))?;

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

    /// `conv_context(id)` — return a formatted string of conversation history for LLM injection.
    pub(super) fn invoke_conv_context(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_context() requires 1 argument (id: String)".to_string()),
        };
        let convs = self
            .conversations
            .lock()
            .map_err(|e| format!("conv_context() lock error: {}", e))?;
        let conv = convs
            .get(&id)
            .ok_or_else(|| format!("conv_context() conversation '{}' not found", id))?;

        let mut parts = Vec::new();
        for msg in &conv.messages {
            parts.push(format!("{}: {}", msg.role, msg.text));
        }
        Ok(Value::String(parts.join("\n")))
    }

    /// `conv_end(id)` — terminate a conversation. Returns "ok".
    pub(super) fn invoke_conv_end(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_end() requires 1 argument (id: String)".to_string()),
        };
        let mut convs = self
            .conversations
            .lock()
            .map_err(|e| format!("conv_end() lock error: {}", e))?;
        convs.remove(&id);
        Ok(Value::String("ok".to_string()))
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
        let backend = llm::create_llm_backend();
        backend.call(prompt, text)
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
