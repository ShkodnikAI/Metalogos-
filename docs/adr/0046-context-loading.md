# ADR-0046: Context Auto-Loading in Learnable Patterns

**Status:** Implemented (extended with `auto`/`none`/literal variants)
**Date:** 2026-06-08 (initial), 2026-06-11 (extended)

## Context

Learnable patterns (`learnable pattern`) call an LLM with a static system prompt and user input. When building AI applications, the LLM often needs access to domain-specific facts stored in Metalogos memory. Previously, developers had to manually call `recall()` in the pattern body and concatenate results into the prompt. This was verbose, error-prone, and didn't compose well with the prompt field.

## Decision

Add two optional fields to `learnable pattern` declarations, with multiple context modes:

```mlog
learnable pattern Ask(text: String) -> String {
  prompt: "Answer the question about Metalogos."
  context: auto                          // recall(first_param, limit=5)
  context: none                          // no context (default behavior)
  context: recall(text, limit=5)         // explicit recall query + limit
  context: "Always respond in Russian"    // static literal prepended as-is
  max_tokens: 4000                       // optional — stored for future LLM backend use
}
```

### Context Modes

1. **`context: auto`**: Syntactic sugar for `recall(first_param, limit=5)`. When the learnable pattern is invoked, the first parameter's runtime value is used as the recall query. Up to 5 facts are collected by relevance score and prepended to the system prompt. This is the recommended mode for most use cases — it requires no explicit query expression and automatically uses the user's input.

2. **`context: none`**: Explicitly disables context loading. Functionally identical to omitting the `context` field entirely, but useful for documentation clarity and for overriding inherited defaults. This is the default behavior for backward compatibility.

3. **`context: recall(query_expr, limit=N)`**: Full control over the recall query. The `query_expr` is evaluated in an environment where pattern parameters are bound to the actual arguments. The resulting string is used as a query to recall relevant memories from the semantic store. Up to `limit` facts (default 5) are collected, sorted by relevance score (cosine similarity * confidence * priority), and formatted as a "Relevant context:" block prepended to the base system prompt. This mode is useful when the recall query should differ from the raw input (e.g., extracting a keyword, combining parameters).

4. **`context: "literal string"`**: Prepends a static string literal directly to the system prompt, without any memory recall. Useful for injecting fixed instructions, persona definitions, or formatting constraints that don't depend on runtime data.

5. **`max_tokens: N`**: Stored on the compiled learnable pattern for future use when the `LlmBackend` trait is extended with a `max_tokens` parameter. Currently, the trait uses `call(prompt, input)` with hardcoded 1024 in provider implementations.

### Effective Prompt Format

When context is loaded and facts are found (auto/recall modes):
```
Relevant context:
- fact1
- fact2
- fact3
<base prompt>
```

When context is a literal string:
```
<literal text>
<base prompt>
```

### Fallback Behavior

- If no facts match the query (below 0.1 confidence threshold), the base prompt is used unchanged.
- If the memory store is locked, the base prompt is used unchanged.
- If the query expression evaluation fails (recall mode), the base prompt is used unchanged.
- For `context: none` or omitted `context`, the base prompt is always used unchanged.

### Scoring

Uses cosine similarity between query embedding and stored fact embeddings, multiplied by the fact's confidence and priority. Falls back to substring matching (case-insensitive) when embeddings are empty or unavailable.

## Implementation

- **Grammar** (`grammar.pest`): Extended `learnable_body` with `context_line` as a silent choice (`_{}`) of four variants: `context_recall_line`, `context_auto_line`, `context_none_line`, `context_literal_line`. PEG ordered choice ensures recall is tried first (most specific), then auto/none (keyword literals), then literal (most general expression match).

- **AST** (`ast.rs`): Introduced `ContextMode` enum with four variants (`None`, `Auto`, `Recall(Expr, Option<usize>)`, `Literal(String)`). `LearnablePatternDecl` replaced `context_query`/`context_limit` with a single `context: Option<ContextMode>` field.

- **Parser** (`parser.rs`): `parse_learnable_pattern_decl()` detects which context variant matched by inspecting the inner rule (since `context_line` is a silent rule, the inner rule appears directly in `learnable_body` children).

- **Interpreter** (`interpreter.rs`):
  - `CompiledLearnable.context: Option<ContextMode>` replaces the old `context_query`/`context_limit` fields.
  - `build_effective_prompt()` handles all four `ContextMode` variants:
    - `None`: returns base prompt unchanged.
    - `Auto`: uses `args[0]` as recall query with limit=5.
    - `Recall(expr, limit)`: evaluates expr in param-bound environment, uses result as recall query.
    - `Literal(text)`: prepends static text directly (no recall).
  - `invoke_learnable_with_env()` calls `build_effective_prompt()` before the LLM call.
  - Two registration sites (`run()` and `load_module_inner()`) updated to propagate the new field.

### Test Contracts

- `examples/p12_context_auto.mlog` / `.expected`: Tests `context: auto` — memorize 2 facts, learnable pattern with `context: auto`, verify MockLlm returns effective prompt with "Relevant context:" block containing both facts.
- `examples/p12_context_literal.mlog` / `.expected`: Tests `context: "You are helpful."` — verify literal text is prepended to base prompt.
- `examples/p12_context_none.mlog` / `.expected`: Tests `context: none` — verify no context is added (same as omitting field).
- `examples/p11_context_loading_fixed.mlog` / `.expected`: Tests backward compatibility of `context: recall(text, limit=3)` — verify facts are recalled and prepended.

### Backward Compatibility

- `context` and `max_tokens` are fully optional. Existing learnable patterns without these fields behave identically.
- The `context: recall(text, limit=N)` syntax continues to work as before.
- The `LlmBackend::call(prompt, input)` trait signature is unchanged.
- MockLlm continues to return the effective prompt as-is, making context loading visible in tests.
- No existing test contracts needed modification (p11_context_loading.mlog had a pre-existing `let` at top-level issue, unrelated to this ADR).

## Consequences

- **Positive**: Four context modes cover all common use cases. `context: auto` eliminates even the query expression boilerplate. `context: "literal"` enables persona/style injection without memory. Clean separation between domain knowledge (memorize) and prompt engineering.
- **Negative**: `all_entries()` loads the entire memory store for scoring in auto/recall modes — not efficient for large stores. A future optimization would be a `recall_top_k(query, k)` method on the MemoryStore trait.
- **Neutral**: `max_tokens` is stored but not yet wired to the LLM backend. This is intentional — changing the `LlmBackend` trait is a separate concern.
