# ADR 0003: M3 — Learnable Patterns and LLM Integration

**Status:** Accepted
**Date:** 2026-05-31
**Milestone:** M3 — Learnable pattern = LLM call

## Context

M3 introduces the first "wow" moment of METALOGOS: a `learnable pattern` is executed
by sending its prompt to an LLM and returning the model's response as a typed value.
This transforms METALOGOS from a pure-functional pipeline language into an AI-native
programming language where patterns can invoke external intelligence.

The contract program is `examples/m3_classify.mlog`:
```mlog
entity text: String = "ваш сервис ужасен"

learnable pattern Classify(msg: String) -> String {
  prompt: "complaint"
}

pattern Respond(category: String) -> String { return "Response: " + category }

flow Main { input: String = text -> Classify -> Respond -> output }
```

**Done when:** `mlog run m3_classify.mlog` prints `Response: complaint`.

## Decision

### Learnable pattern syntax

```mlog
learnable pattern Name(params) -> ReturnType {
  prompt: "prompt string"
}
```

The `learnable` keyword precedes `pattern`. The body contains one or more `prompt:` lines
(only the first is used in M3). The prompt is a string literal that serves as the
instruction to the LLM. In production, the pattern's arguments are concatenated and
appended as input to the prompt.

**Grammar addition:** `learnable_pattern_decl` in grammar.pest, with `LEARNABLE_KW = @{ "learnable" }`
and `PROMPT_KW = @{ "prompt" }`. Both added to the `step_ident` negative lookahead to
prevent them from being consumed as pipeline step names.

### LLM backend abstraction (trait-based)

The `src/llm.rs` module defines an `LlmBackend` trait with two implementations:

1. **`MockLlm`** (test mode): Returns the prompt string as-is. This makes golden tests
   deterministic — the `prompt:` field doubles as the expected response. No network
   calls, no randomness, no flaky tests.

2. **`RealLlm`** (production mode): HTTP POST via `curl` subprocess to a configurable
   API endpoint (`METALOGOS_LLM_ENDPOINT`, default: Ollama `http://localhost:11434/api/generate`).
   Supports OpenAI-compatible (`choices[0].message.content`) and Ollama-compatible
   (`response`) JSON formats, with raw-text fallback.

**Mode selection:** `METALOGOS_MOCK_LLM=1` (default: `true` for safety). When set to `0`
or `false`, the real LLM backend is used.

This trait-based design, grounded in prior art (DSPy, LMQL), separates the language
semantics from the LLM transport. Future milestones can add:
- HTTP client libraries (`reqwest`) instead of `curl` subprocess
- Streaming responses
- Structured output parsing (JSON schema validation)
- Multiple model backends (OpenAI, Anthropic, local)

### Interpreter integration

Learnable patterns are stored in a separate `learnable_patterns: HashMap<String, CompiledLearnable>`
in the interpreter. The `invoke()` method checks learnable patterns before builtins and
pure patterns, giving them the highest dispatch priority. This matches the semantic intent:
learnable patterns are the "smart" variants that should override pure fallbacks.

The `invoke_learnable()` method:
1. Validates argument count against parameter count
2. Concatenates arguments into an input string
3. Calls `llm::create_llm_backend()` to get the appropriate backend
4. Passes prompt + input to the backend
5. Returns the response as `Value::String`

### Confidence semantics (honest, documented)

Per the `metalogos-language-semantics` skill, M3 uses the simplest defensible approach to
confidence:

- **M3 does NOT implement Fluid Types or probabilistic confidence propagation.**
- The learnable pattern returns a `Value::String`. There is no confidence score attached.
- Confidence propagation (min/product of input confidences) is deferred to later milestones
  when the type system supports it.
- In M3, confidence is implicit: if the LLM returns a result, the flow continues. If it
  fails (network error, parse error), the flow errors out. This is not soft-failure —
  that comes later.

**Documented limitation:** M3 learnable patterns are essentially `prompt -> string` functions.
There is no structured output parsing, no confidence scoring, and no error recovery. These
are intentional simplifications for the MVP milestone.

### Test strategy

Golden tests use `MockLlm` by default (`METALOGOS_MOCK_LLM` defaults to `true`). The
prompt string serves as both the instruction (in production) and the deterministic
expected response (in tests). This eliminates the need for network mocking libraries
and keeps the test suite fast and hermetic.

For integration testing with a real LLM, set `METALOGOS_MOCK_LLM=0` and provide a running
LLM endpoint. These tests are not part of the automated CI suite.

## Consequences

- M3 proves that METALOGOS can invoke external AI models: the language is now AI-native.
- The `MockLlm`/`RealLlm` split keeps tests deterministic while enabling real LLM calls
  in production. This is a permanent architectural decision.
- The `prompt:` syntax is the simplest possible form. Future milestones will add:
  - Multiple prompt lines with variable interpolation
  - Few-shot examples via `adapt` (M5)
  - Structured output with type coercion
- The `curl`-based HTTP client is a deliberate MVP choice. It avoids adding `reqwest`
  as a dependency and works in any environment with `curl` installed. The trade-off is
  process overhead per call, which is acceptable for M3 but will need upgrading in
  production use.
- M1 and M2 tests remain green — no regressions from M3 changes.
