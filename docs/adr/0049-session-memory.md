# ADR-0049: Session Memory (temporary conversation memory)

**Status**: Accepted
**Date**: 2025-06-09
**Naryad**: #5 (repeat of Naryad #1)

## Context

METALOGOS has global KV memory (`mem_set`/`mem_get`/`mem_delete`), which is
persistent via SQLite when `memory { persist: "..." }` is set. This is
suitable for global configuration and long-lived data.

But building chatbots and web applications needs **temporary** memory,
scoped to a specific session (chat_id, user_id). This memory must:

- Be isolated between sessions (chat_id A does not see chat_id B's data)
- Reset on server restart (by design — session data)
- Not persist (unlike global `mem_*`)

## Decision

Three new builtin functions with session-scoped storage:

```
session_set(session_id: String, key: String, value: String) -> String
session_get(session_id: String, key: String) -> String
session_clear(session_id: String) -> Unit
```

### Storage

```rust
static SESSION_STORE: OnceLock<Mutex<HashMap<String, HashMap<String, String>>>>
```

- Outer key = `session_id` (e.g. `"chat-42"`, `"user-alice"`)
- Inner HashMap = key-value pairs within the session
- **In-memory only** — no SQLite, no file, no persistence
- Reset on restart = simply disappears (the HashMap is cleared)

### Difference from mem_set/mem_get

| Aspect | `mem_set`/`mem_get` | `session_set`/`session_get` |
|--------|----------------------|----------------------------|
| Scope | Global | Per-session (session_id) |
| Persistence | SQLite write-through | In-memory only |
| Restart | Data survives | Data is lost |
| Isolation | None — shared across all | Yes — partitioned by session_id |

### Usage in .mlog

```
// Save conversation context
session_set(chat_id, "last_topic", "billing")
session_set(chat_id, "message_count", "5")

// Read later, in a different request
let topic = session_get(chat_id, "last_topic")

// Clear when the session ends
session_clear(chat_id)
```

## Contract tests

10 tests in `tests/session_memory_contract.rs`:

1. **set→get roundtrip** — write, read, matches
2. **set returns value** — session_set returns the saved value
3. **missing key** — session_get of a non-existent key → empty string
4. **missing session** — session_get of a non-existent session → empty string
5. **session isolation** — session A's data is not visible from session B
6. **session_clear** — after clear, all of the session's keys are empty
7. **restart empties** — reset_session_store() → all data gone
8. **multiple keys** — several keys coexist within one session
9. **overwrite** — rewriting a key replaces the old value
10. **no persistence** — no SQLite, purely in-memory

## Files

| File | Change |
|------|--------|
| `src/builtins.rs` | +3 builtin functions, SESSION_STORE static, helpers |
| `tests/session_memory_contract.rs` | NEW — 10 contract tests |
| `examples/p8_session_memory.mlog` | NEW — contract per Naryad #1 (TestSession pattern + isolation) |
| `docs/adr/0049-session-memory.md` | NEW — this document |

## Consequences

- **No changes to grammar/AST/parser** — these are ordinary function calls
- **No changes to the interpreter** — builtin dispatch already handles every FnCall
- **Thread safety**: `std::sync::Mutex` (same model as KV_STORE)
- **Backward compatible**: existing `mem_set`/`mem_get` unchanged
- **Global state**: `OnceLock<Mutex<...>>` — shared across all interpreters (by design for server mode)
