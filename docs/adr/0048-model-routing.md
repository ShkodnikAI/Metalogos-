# ADR-0048: Cost-Aware Model Routing for Learnable Patterns

**Status:** Implemented
**Date:** 2026-06-09

## Context

All learnable patterns in Metalogos use the same global LLM model (configured via `METALOGOS_LLM_MODEL` env var). In practice, different tasks have different complexity and cost profiles: a simple keyword classification does not need `claude-sonnet-4-20250514` — `claude-3-5-haiku` at 1/10 the cost suffices. Conversely, complex analytical tasks benefit from `claude-opus-4`. Forcing every pattern to use the same model wastes money on over-provisioned calls and degrades quality on under-provisioned ones.

The alternative considered was an `llm_route` block with rule-based model selection, but this adds significant complexity (rule evaluation, estimated_complexity variable) for marginal benefit. A per-pattern `model` override is simpler, more explicit, and sufficient for the primary use case: developers know which patterns need which model at declaration time.

## Decision

Add an optional `model` field to `learnable pattern` declarations:

```mlog
learnable pattern QuickClassify(text: String) -> String {
  prompt: "Classify as positive | negative | neutral."
  model: "haiku"
}

learnable pattern DeepAnalysis(text: String) -> String {
  prompt: "Analyze sentiment, tone, and intent."
  model: "opus"
}

learnable pattern DefaultTask(text: String) -> String {
  prompt: "Translate to French."
  // no model → uses global METALOGOS_LLM_MODEL
}
```

### Semantics

1. **`model: "name"`**: Optional string field. When set, this model name is passed to the LLM backend instead of the global `METALOGOS_LLM_MODEL`. When absent, the global model is used (backward compatible).

2. **Model resolution**: The per-pattern model override is passed to the `LlmBackend::call_with_model()` method. The backend uses the override model in the API JSON body's `"model"` field. If the override equals the global model, no special handling is needed.

3. **Cache key integration**: The model override is NOT part of the cache key (`hash(prompt + input)`). This is intentional — the same prompt+input with different models would produce different responses, but the current cache key doesn't include model. If model-specific caching is needed, users should use different prompt text to differentiate.

   **Correction**: Upon implementation review, the cache key is `hash(effective_prompt + input)`. Since the model override doesn't change the effective_prompt, patterns with the same prompt+input but different models share a cache key. This is acceptable because: (a) cache is per-interpreter, (b) typically each pattern has unique prompt text, (c) if needed, a future enhancement can include model in the cache key.

4. **Provider compatibility**: The model string is passed verbatim to the LLM API. It works with all three providers (Anthropic, OpenAI, Ollama) since they all accept a `"model"` field in their request JSON. The user is responsible for specifying a valid model name for their configured provider.

### Implementation

- **Grammar**: `model_line = { "model" ~ COLON ~ expression }` rule in `learnable_body`.
- **AST**: `LearnablePatternDecl` gains `model: Option<String>`.
- **Parser**: Extracts model string from `model_line` expression.
- **LlmBackend trait**: New `call_with_model(prompt, input, model: Option<&str>)` method with default impl that delegates to `call()`. This is backward compatible — existing backends and tests are unaffected.
- **RealLlm**: Implements `call_with_model()` by cloning self with the overridden model field, then calling `call()` through the clone.
- **MockLlm**: Implements `call_with_model()` to record the model name in a static `MOCK_LLM_LAST_MODEL: Mutex<String>` for contract tests.
- **Interpreter**: `CompiledLearnable` gains `model: Option<String>`. `invoke_learnable_with_env()` calls `backend.call_with_model()` instead of `backend.call()`.

### Backward Compatibility

- `model` defaults to `None` — no existing patterns are affected.
- The `LlmBackend::call()` trait method is unchanged; `call_with_model()` has a default impl.
- All existing golden tests continue to pass (MockLlm.call() returns the prompt unchanged).
- `RealLlm` gains `#[derive(Clone)]` (fields already implement Clone).

## Consequences

- **Positive**: Per-pattern model selection enables cost optimization. Simple tasks (classification, keyword extraction) can use cheap models like Haiku. Complex tasks (analysis, generation) can use premium models like Opus. No changes needed to environment configuration.
- **Negative**: The model string is not validated at parse time. An invalid model name will only fail at runtime when the LLM API returns an error. A future enhancement could validate model names against a known list per provider.
- **Neutral**: The `llm_route` block design (rule-based routing with `estimated_complexity`) is not implemented but could be added as a higher-level abstraction in the future. The per-pattern `model` field provides the building block for any routing strategy.
