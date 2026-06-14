# ADR-0053: Conversation State — Managed Dialog Context

**Status:** Implemented  
**Date:** 2026-06-11  
**Priority:** HIGH — required for Fosved Office multi-turn dialog support

## Context

Metalogos learnable patterns execute single-turn LLM calls: prompt + input → response.
Real chatbots and assistants need multi-turn conversations with message history,
auto-compression of old messages, and configurable limits.

Without conversation state, Fosved Office cannot lead coherent multi-turn dialogs
with users. Each learnable pattern invocation is stateless.

Prior art:
- **OpenHands ConversationState**: per-session message buffer with TTL and summarization
- **LangChain ConversationBufferMemory**: windowed memory with compression trigger
- **ChatGPT**: conversation model with system/user/assistant messages and token budget

## Decision

### 1. Top-level `conversation` declaration

```mlog
conversation {
    ttl: 1800            // 30 min auto-cleanup
    max_messages: 50      // max messages per conversation
    compress_after: 20    // compress older messages via LLM summary
}
```

Sets global config. Without declaration, defaults apply (ttl=1800, max=50, compress=20).

### 2. Five builtins

| Builtin | Signature | Description |
|---------|-----------|-------------|
| `conv_start(id)` | → String | Create/open conversation, returns id |
| `conv_add(id, role, text)` | → String | Append message (role: "user"/"assistant"/"system") |
| `conv_history(id)` | → List<Struct> | Full history as list of {role, text, timestamp} structs |
| `conv_context(id)` | → String | Formatted "role: text" lines for LLM injection |
| `conv_end(id)` | → String | Remove conversation, returns "ok" |

### 3. Storage

- **In-memory**: `HashMap<String, Conversation>` behind Mutex
- `Conversation { id, messages: Vec<ConvMessage>, created_at, last_active, metadata }`
- `ConvMessage { role, text, timestamp }`
- SQLite persist planned for future (similar to memory store pattern)

### 4. Learnable pattern integration

```mlog
learnable pattern Reply(text: String) -> String {
    prompt: "You are a helpful assistant"
    conversation: "current"    // bind to active conversation
}
```

The `conversation` field stores the conversation id. When set, the interpreter
injects conversation history into the LLM prompt via `get_conversation_for_llm()`.

### 5. Auto-compression

When `messages.len() > compress_after`:
1. Older messages (beyond threshold) are extracted
2. Summarized via `call_llm("Summarize concisely", old_messages_text)`
3. Replaced by a single `system` message containing the summary
4. On LLM failure, a fallback message notes N messages were omitted

### 6. Max messages enforcement

When `messages.len() >= max_messages`:
- Oldest message is evicted before adding the new one
- Ensures conversations don't grow unbounded

## Implementation

### Grammar (grammar.pest)
- `conversation_decl` rule with `conversation_body`
- `conversation_ttl`, `conversation_max_messages`, `conversation_compress_after`
- `CONVERSATION_KW` keyword token
- `conversation_line` in `learnable_body` for learnable pattern binding
- "conversation" added to `step_ident` negative lookahead

### AST (ast.rs)
- `ConversationDecl { ttl, max_messages, compress_after }`
- `Declaration::Conversation(ConversationDecl)` variant
- `conversation: Option<String>` added to `LearnablePatternDecl`

### Parser (parser.rs)
- `parse_conversation_decl()` function
- `conversation_line` extraction in `parse_learnable_pattern_decl()`

### Interpreter (interpreter.rs)
- `ConvMessage`, `Conversation`, `ConversationConfig` structs
- `conversations: Mutex<HashMap<String, Conversation>>` field
- `conversation_config: ConversationConfig` field
- `invoke_conv_start/add/history/context/end()` methods
- `compress_conversation()` and `summarize_conversation()` methods
- `get_conversation_for_llm()` public helper for LLM integration
- `Declaration::Conversation` dispatch in `run()`
- FnCall dispatch for `conv_*` builtins

### Tests (tests/conversation_state_contract.rs)
- C1: conv_start creates empty conversation
- C2: conv_add adds 3 messages, verified via lock
- C3: Message struct has role, text, timestamp > 0
- C4: conv_context returns formatted "role: text" string
- C5: conv_end removes conversation
- C6: Config applied from declaration (ttl=900, max=10, compress=5)
- C7: max_messages enforced — oldest evicted
- C8: Default config (ttl=1800, max=50, compress=20)
- C9: `conversation: "current"` parsed in learnable pattern
- C10: Multiple conversations are independent

**10 tests, all passing.**

## Consequences

- Multi-turn dialogs are now possible within Metalogos programs
- Conversation history is automatically managed with configurable limits
- LLM summarization compresses old messages transparently
- Learnable patterns can bind to conversations for context-aware responses
- All state is in-memory per interpreter instance; SQLite persist is a future enhancement
- Thread-safe via Mutex for concurrent access in server mode
