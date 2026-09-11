# ADR-0133: `call_llm_schema` — structured LLM output through a hand-rolled JSON-Schema subset validator

**Status:** Accepted
**Date:** 2026-09-11
**Naryad:** #269 (issue #305)
**Precedent:** ADR-0131 diagnostic-code convention (`LLM_SCHEMA_MISMATCH`,
`LLM_SCHEMA_UNSUPPORTED_FEATURE`); FEATURE_INTAKE §4-C dependency discipline;
ADR-0048 SmartRouter slots.

## Context

`call_llm(prompt, input) -> String` returns raw text; the language's JSON
tooling is `json_encode`/`json_get`/`json_body` — no schemas, no validation
(grep over `spec!("json_` — exactly these three). LLM providers already
support structural output (response_format / json_schema), and the primary
consumer (FOSVED) classifies LLM "format errors" as a distinct failure class
they must hand-scrape today. A program that wants structured data from an LLM
currently concatenates format instructions into the prompt and hopes; every
format deviation silently flows downstream as a garbage `String`.

`call_claude` additionally hardcodes `max_tokens: 4096` — large schemas are a
truncation risk; truncation must surface as a loud diagnostic, not garbage.

## Decision

### D1. Validator subset — closed allowlist, explicit rejection

The builtin validates answers against a supported subset of JSON Schema:
`type` (object/array/string/number/integer/boolean/null), `properties`,
`required`, `items`, `enum`. Pure-annotation keywords (`$schema`, `$id`,
`$comment`, `title`, `description`, `default`, `examples`) are inert metadata
that cannot weaken validation — they are ignored, so real-world schemas
(which carry annotations) do not bounce. **Any other keyword is a loud
`LLM_SCHEMA_UNSUPPORTED_FEATURE` error naming the keyword** — the "explicit
error" option of the naryad, chosen over ignore-with-warning: a silently
ignored `pattern`/`format`/`minimum` means the author believes validation is
stronger than it is, which is exactly the dishonest-silence class this
language rejects. The root schema must be `{"type":"object",...}` — the
builtin returns a Struct.

### D2. Strict-by-default extras (documented deviation from JSON Schema)

Fields present in the answer but not declared in `properties` are
**violations**. JSON Schema's default (`additionalProperties: true`) is the
opposite; we do not support the `additionalProperties` keyword because
strict-by-default makes it redundant. Rationale: unvalidated keys would
smuggle untrusted LLM-derived data into the `Value::Struct` under the
program's assumption that "everything in the Struct passed validation".

### D3. Two diagnostic codes + retry policy

- `LLM_SCHEMA_MISMATCH` — answer-side failure: unparseable JSON (including
  max_tokens truncation, which is named in the error text as the likely
  cause), schema violations, enum misses. **Retryable**: up to
  `METALOGOS_LLM_SCHEMA_RETRIES` extra attempts (default 2, hard cap 10 —
  a runaway budget must not spin the LLM loop), each retry carrying the
  full validator report (all violations, not fail-fast) plus the directive
  to answer with JSON only.
- `LLM_SCHEMA_UNSUPPORTED_FEATURE` — schema-side failure: keyword outside
  the subset, malformed `required`/`enum`, non-object root, invalid schema
  JSON. **Not retried** — retrying cannot fix the program's own schema, and
  burning LLM calls on it is waste.
- Transport errors from the backend propagate immediately without
  schema-retries — SmartRouter (ADR-0048) owns failover; a second retry
  layer here would double-call dead providers.

### D4. Mock contract

With no SmartRouter installed and mock mode active (the default tier), the
builtin does NOT return `[MOCK: ...]` text — that would be a guaranteed loud
validation failure and useless for tests. Instead the mock returns a
deterministic minimal instance derived from the schema itself (every declared
property present; enums answer with their first literal; strings answer with
their own key name). The instance flows through the real parse → validate →
convert pipeline, so tests exercise the whole contract without network.
`METALOGOS_LLM_MOCK=json` is accepted as an explicit alias with identical
behavior. The mock shape is a test convenience, not a language contract.

### D5. Reuse, not reinvention

The answer provider is injected as a function mirroring the SmartRouter slot
signature (`call_via_smart_router(prompt, input, None, None)` — same slots as
`call_llm`, ADR-0048; model/timeout overrides stay `None`). The core loop
`call_llm_schema_core` is a pure function over that provider — the retry
semantics are unit-tested with scripted providers, no network. The result is
a `Value::Struct` with `type_name: "Dict"` (the JSON-toolchain convention),
so `json_get`/`has_field`/`dict_*` interoperate without new plumbing. The
result binding carries `LlmOutput` taint (audit.rs) — structured or not, the
content is LLM-derived and untrusted.

### D6. Dependency and size discipline

No new dependencies — `schemars`/`valico` are rejected per FEATURE_INTAKE
§4-C; the validator is ~300 lines of pure Rust. This exceeds the §1
"<200 lines" guideline for Tier-1 builtins — the exceedance is explicit and
recorded here (precedent: the per-module LOC limit is 5 000; the builtin
lands in its own module `src/builtins/llm_schema.rs`, well under every
guard). A full JSON-Schema implementation was never on the table; the subset
is the contract.

## Consequences

- Programs get a loud, machine-actionable failure mode for format errors
  instead of silent garbage; FOSVED's llm_verifier can consume the
  diagnostic codes directly.
- The subset does not cover `pattern`, `format`, numeric/length bounds,
  composition (`allOf`/`anyOf`/`oneOf`), or references. Schemas using them
  fail loudly at schema-check time, before any LLM call is paid for.
  Extending the subset is additive by design (new supported keywords join
  D1's list; annotation list is closed).
- `call_claude`'s hardcoded `max_tokens: 4096` is now a *visible* risk:
  truncation produces `LLM_SCHEMA_MISMATCH` with a truncation hint, and the
  retry loop burns calls on an answer that will never fit. Mitigation stays
  with the program author (smaller schemas); revisit `max_tokens` plumbing
  if a real use case hits it.
- Discovered during testing (pre-existing, out of scope here, reported to
  the owner): the parser's `MULTILINE_STRING` handler returns the literal
  text including the `"""` delimiters (`src/parser/expr.rs`, MULTILINE arm
  does not strip them). Triple-quoted strings are currently unusable for
  JSON embedding; tests here use escaped double-quoted strings instead.

## Verification

`tests/naryad_269_schema.rs` (24 tests): the mlog contract
`call_llm_schema → Struct → json_get` green on TW and VM (flat + nested +
array paths); every supported/ignored/rejected schema construct; retry loop
(fail-once-then-valid, no-budget loud mismatch, truncation hint,
schema-side no-call, transport no-retry, feedback-in-prompt); mock
determinism self-validation; retries-env defaults/cap. The
`registry_arity_check` exhaustive list gains the `(2, 3)` case.
