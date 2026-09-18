# ADR-0136: redact(text, mode) — PII/secrets as a taint sanitizer

**Status:** Accepted (stop-gate SG-2 approved by the owner 2026-09-12)
**Date:** 2026-09-12
**Naryad:** #274 (issue #310)
**Precedent:** #201/ADR-0117 (taint classes), ADR-0038/0079 (secrets), #54 (linear regex engine — the patterns create no ReDoS risk), #256 (fuzz-target convention)

## Context

The Metalogos taint system today can DENY (sink rejections `SECRET_LEAK`, `HTML_INJECTION`, `UNTRUSTED_TRAINING_DATA`) but cannot TRANSFORM: there is no language mechanism to mask a secret/PII before use. `TaintKind::Sanitized` is assigned only by `render`/`escape_html` (`src/audit.rs`, `binding_taint()`) and only for the HTML class. GDPR argument for NLnet: "PII masking at the compiler level, not the prompt level". Stop-gate SG-2 (dispatch #316): taint-removal semantics is an owner decision, not an implementer one.

## Decision

### D1. Builtin and pattern sets

`redact(text, mode) -> String`, arity 2, category `string`, mode ∈ `{"pii", "secrets", "all"}` — an unknown mode is a loud error. The builtin accepts both `Value::String` and `Value::Secret` (masking a secret in place is exactly its purpose; the result is a `Value::String` — a `Value::Secret` is unprintable and would not survive the runtime path to a sink).

- **secrets**: API keys (`sk-…` ≥16 chars, `AKIA…` 16 uppercase, `ghp_/gho_/ghu_/ghs_/ghr_…` ≥20), JWT (`eyJ…` three segments), PEM blocks (`-----BEGIN … -----END …`), `Bearer <token>` (token ≥8).
- **pii**: email (mask `***@***.<tld>` — the TLD is preserved for diagnosability), phones (international `+` format and RU `8` format, a 7..15-digit filter), cards (13–19 digits + Luhn validation + vendor by prefix: visa/mastercard/amex/discover/unionpay), IBAN (uppercase, 15–34 chars, the tail group may be incomplete).
- **Entropy net** (only in `secrets`/`all`): base64/hex runs ≥24 chars; only runs containing BOTH a digit AND a hex letter (`a-fA-F`) are masked. Long words without digits, purely numeric identifiers, and kebab/snake identifiers (separators break the run) pass through. This is insurance against formats outside the pattern sets, not exhaustive detection.

Masks are deterministic and typed: `[REDACTED:<type>…<last 4>]` (e.g. `[REDACTED:sk-…abc4]`), PEM `[REDACTED:pem-block]`, phone `[REDACTED:phone]` — logs stay diagnosable. Order of application: secrets → PII → entropy net (last); the masks themselves re-trigger none of the patterns (idempotence `redact(redact(x)) == redact(x)` — a test invariant and a fuzz-target invariant).

### D2. Taint semantics (SG-2 brief)

- `redact(x, "secrets" | "all")` removes ONLY the `Secret` taint → the result is `Sanitized` — the legal "mask before sink" path: `secret → redact("secrets") → http_post` passes the audit; without redact it is rejected.
- `redact(x, "pii")` does NOT remove `Secret` — the test invariant `secret → redact("pii") → http_post` is rejected.
- `LlmOutput` is not removed by redact at all: the sanitizer of model output is one — `render` (masking ≠ HTML-escape; `HTML_INJECTION` stays in force).
- `UserInput` is not removed: masking does not change the data's provenance.
- mode is read STATICALLY from a string literal (`src/audit.rs`, `redact_result_taint`, applied both in `binding_taint` and in `get_expr_taint` — chains through let and inline calls are equivalent). A dynamic/non-literal mode — **fail-closed**: taint is inherited without removal.

### D3. Residual risk (honest)

Trust in the pattern sets: a secret of a format outside the sets and without an entropy profile (e.g. a short base64 without separator characters) is not masked — the entropy net requires both a digit and a hex letter in a run of ≥24, so as not to strangle identifiers. False positives are possible (the direction is safe: extra masking, not a leak). A fuzz target is mandatory (`fuzz_target_redact`: panic-freedom, determinism, idempotence). Redact neither encrypts nor deletes: the original remains in the caller's source string — only the result is sanitized. Redact does not bypass the env-gate read (#259): `env()` in route bodies remains denied regardless of a subsequent redact.

## Consequences

- Positive: "mask before sink" is a legal path for publishing diagnostic logs containing secrets; the GDPR argument "PII masking at the compiler level"; the `SECRET_LEAK` line of the threat model gets a mitigation layer; the contract is pinned by a pair of DoD tests and the fuzz target.
- Negative: keeping the last 4 characters in the mask is a diagnosability trade-off (theoretically distinguishable tails of long secrets); false positives of the entropy net in `all` mode on long alphanumeric runs.
- Neutral: registry 398→399; grammar and bytecode surface unchanged (a builtin, not a declaration); TW/VM parity comes free (shared registry handler).
