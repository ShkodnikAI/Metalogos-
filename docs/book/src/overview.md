# Overview

## What is Metalogos

Metalogos (mlog) is an open source programming language where AI operations — LLM calls, memory, learning, adaptation — are first-class language constructs, not library integrations. An LLM invocation is as natural as calling a function. Security constraints (XSS prevention, SQL injection prevention, secret opacity) are enforced at the language level, not through middleware.

```mlog
// doc-test: skip
// Learnable pattern: LLM call as a language construct
learnable pattern Classify(msg: String) -> String {
  prompt: "Classify as: question | complaint | greeting | urgent"
}

// Memory Tree: hierarchical knowledge with L0 → L1 → L2 compression
mtree_store("User prefers email over chat")
mtree_store("Project deadline is July 15")
mtree_summarize()  // L0 → L1 chunking

// Cron scheduler: run patterns on schedule
cron_run("0 9 * * 1-5", "MorningReport")  // weekdays at 09:00

// Goals and Todos
goal_set("launch v1.0", "2026-12-01")
todo_add("Fix cron edge case", "high")

// Call it like any other pattern
let result = Classify("where is my order?")
```

No frameworks. No boilerplate. No `import ai_sdk`. The language *is* the AI infrastructure.

---

## Why Metalogos

**In 30 seconds** — call an LLM like a function and get validated JSON back, then watch the compiler refuse to leak a secret:

```mlog
pattern Extract(x: String) -> String {
  let person = call_llm_schema("Extract the user as JSON", x,
    "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"]}")
  return json_get(person, "name")          // Struct field, not text scraping
}

pattern LeakApiKey() -> String {
  let key = env("FAKE_API_KEY")
  respond("200", "key is " + key)          // ← rejected: SECRET_LEAK
  return key
}
```

```text
$ mlog check api_leak.mlog
1 error:
  1: строка 1: [SECRET_LEAK] secret may be leaked — env() value passed to respond()
```

(Both outputs are real — this exact probe is what `mlog check` prints, exit code 1. See [REFERENCE.md §4.5](REFERENCE.md) for `call_llm_schema`, and [ADR-0133](docs/adr/0133-llm-schema-validator.md) for the validation contract.)

**Security by design** — not a library, a property of the language: static taint tracking turns secret leaks, SQL injection and raw LLM output into compile errors (Category A); file I/O is sandboxed; `exec()`/`env()` in serve routes are denied by default with stable diagnostic codes and an opt-in allowlist model; subprocesses land in the audit log. The full contract — including the honest list of what static analysis does NOT catch — is in [Security by Design](#2-security-by-design--zero-configuration) below.

**AI-native** — eight semantic primitives (Entity, Pattern, Flow, Memory, Rule, Learn, Adapt, Reflex) make AI operations first-class: learnable patterns teach from examples, Memory combines BM25 + vector search with rank fusion, `adapt` modifies the program's own patterns under a sandbox, Reflex distills LLM teachers into local models.

**MCP-native** — Metalogos speaks the integration standard of 2026-era AI agents in both directions, with the same security gates: the MCP client design is pinned in [ADR-0132](docs/adr/0132-mcp-client.md) and implemented as two stateless builtins, `mcp_call` / `mcp_list_tools` (Naryad №268) — stdio transport, hand-rolled JSON-RPC, exec-gated server spawn, untrusted `UserInput` taint on tool output; a live security-gate walkthrough is in [Security by Design](#2-security-by-design--zero-configuration). The reverse bridge is live: `mlog mcp-serve` exposes Metalogos `tool` constructs as MCP tools (Naryad №297; transports `stdio|http|sse` per [ADR-0168](docs/adr/0168-mcp-server-transports.md)/№394 — fail-closed allowlist on every transport, the per-tool policy compiled from the №316 classification, Bearer auth or a token-less localhost-only bind; externally verified by a raw JSON-RPC client over HTTP and SSE in naryad №401 — protocol in gh#488).

---
