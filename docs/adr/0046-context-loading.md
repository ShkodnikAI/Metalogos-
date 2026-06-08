# ADR-0046: Context Auto-Loading in Learnable Patterns

**Status:** Implemented
**Date:** 2026-06-08

## Context

Learnable patterns (`learnable pattern`) call an LLM with a static system prompt and user input. When building AI applications, the LLM often needs access to domain-specific facts stored in Metalogos memory. Previously, developers had to manually call `recall()` in the pattern body and concatenate results into the prompt. This was verbose, error-prone, and didn't compose well with the prompt field.

## Decision

Add two optional fields to `learnable pattern` declarations:

```mlog
learnable pattern Ask(text: String) -> String {
  prompt: "Answer the question about Metalogos."
  context: recall(text, limit=5)   // optional — auto-loads relevant memories
  max_tokens: 4000                 // optional — stored for future LLM backend use
}
```

### Semantics

1. **`context: recall(query_expr, limit=N)`**: When the learnable pattern is invoked, the `query_expr` is evaluated in an environment where pattern parameters are bound to the actual arguments. The resulting string is used as a query to recall relevant memories from the semantic store. Up to `limit` facts (default 5) are collected, sorted by relevance score (cosine similarity × confidence × priority), and formatted as a "Relevant context:" block prepended to the base system prompt.

2. **`max_tokens: N`**: Stored on the compiled learnable pattern for future use when the `LlmBackend` trait is extended with a `max_tokens` parameter. Currently, the trait uses `call(prompt, input)` with hardcoded 1024 in provider implementations.

3. **Effective prompt format**: When context is loaded and facts are found:
   ```
   Relevant context:
   - fact1
   - fact2
   - fact3
   <base prompt>
   ```

4. **Fallback**: If no facts match the query (below 0.1 confidence threshold), or if the memory store is locked, or if the query expression evaluation fails, the base prompt is used unchanged.

5. **Scoring**: Uses cosine similarity between query embedding and stored fact embeddings, multiplied by the fact's confidence and priority. Falls back to substring matching when embeddings are empty.

### Implementation

- **Grammar**: Extended `learnable_body` to accept optional `context_line` and `max_tokens_line` rules.
- **AST**: `LearnablePatternDecl` gained `context_query: Option<Expr>`, `context_limit: Option<usize>`, `max_tokens: Option<u32>`.
- **Parser**: `parse_learnable_pattern_decl()` extracts context query expression, limit, and max_tokens from the learnable body.
- **Interpreter**:
  - `CompiledLearnable` stores `context_query`, `context_limit`, `max_tokens`.
  - `build_effective_prompt()` evaluates the context query, performs multi-result recall via `all_entries()` + manual scoring, and prepends the context block to the base prompt.
  - `invoke_learnable_with_env()` calls `build_effective_prompt()` before the LLM call.
  - Two registration sites (`run()` and `load_module_inner()`) updated to propagate new fields.

### Backward Compatibility

- `context` and `max_tokens` are fully optional. Existing learnable patterns without these fields behave identically.
- The `LlmBackend::call(prompt, input)` trait signature is unchanged.
- MockLlm continues to return the effective prompt as-is, making context loading visible in tests.

## Consequences

- **Positive**: Declarative context loading eliminates boilerplate recall+concat code. Facts are automatically injected into prompts. Clean separation between domain knowledge (memorize) and prompt engineering.
- **Negative**: `all_entries()` loads the entire memory store for scoring — not efficient for large stores. A future optimization would be a `recall_top_k(query, k)` method on the MemoryStore trait.
- **Neutral**: `max_tokens` is stored but not yet wired to the LLM backend. This is intentional — changing the `LlmBackend` trait is a separate concern.
