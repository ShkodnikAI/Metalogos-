# ADR-0064: Positional taint check for http_post body

## Status
Accepted

## Context

The audit check SECRET_LEAK flagged secrets passed to `respond`, `respond_html`, and
`write_file` (the SINK_FUNCTIONS list). `http_post` was explicitly excluded because
passing auth tokens in headers (Bearer authorization) is the primary usage pattern
of the language — every LLM call and Telegram message requires it.

However, `http_post(url, body, content_type, headers)` has four positional arguments.
A secret in argument 1 (body) is a genuine leak — the token leaves the system in the
request payload. A secret in argument 3 (headers) is normal authorization. The test
`test_secret_leak_to_http_post` passed the token as body and expected a SECRET_LEAK
finding, but the audit produced none — a false negative on a P0 security check.

## Decision

Add a positional taint check for `http_post` inside `check_expr_for_leak`, placed
*after* the generic `is_sink()` block. The check inspects only `args.get(1)` and
flags it if tainted. Argument 3 (headers) is not inspected.

`http_post` is NOT added to SINK_FUNCTIONS — that would flag header usage too,
producing false positives on every legitimate API call.

## Alternatives considered

1. **Add http_post to SINK_FUNCTIONS** — rejected: would flag every
   `http_post(url, body, type, { "Authorization": token })` call as a leak.
   False positive rate would make the check useless.

2. **Remove the test** — rejected: would leave a genuine P0 vulnerability
   (secret in request body) undetected.

3. **Positional check (chosen)** — precise: only body position is flagged,
   headers are allowed. No false positives on the normal auth pattern.

## Prior art

- Taint analysis: tracking data flow from sources (env()) to sinks, with
  positional sensitivity (OWASP A02:2021 — Cryptographic Failures,
  OWASP A09:2021 — Security Logging and Monitoring Failures).
- Semgrep/CodeQL rules for "secret in HTTP request body" follow the same
  positional distinction.