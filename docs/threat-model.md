# Threat Model — Metalogos Static Audit

Concrete threat checks implemented in `src/audit.rs` (`mlog audit`).
Each row maps to a real `check_id` emitted by the analyzer.

## Category A — Blocking (Severity::Error)

These checks prevent compilation/execution. The program is rejected
until the data-flow shape is fixed.

| check_id | Threat | Source → Sink | OWASP | Mitigation |
|---|---|---|---|---|
| `SQL_DYNAMIC` | SQL injection | Non-literal SQL string → `query()` | A03 Injection | Reject `query()` unless the SQL argument is a string literal. Only parameterized queries compile. |
| `SECRET_LEAK` | Plaintext secret leakage | `env()` / `secret()` → `respond()`, `write_file()`, `http_post()` body | A02 Cryptographic Failures | Static taint-tracking: both `env()` and `secret()` results are flagged as secret-derived (Category A). Audit rejects if the tainted value reaches a network or response sink. Runtime opacity (`Value::Secret`) is enforced in two ways: (1) `secret("KEY")` returns `Value::Secret` directly (hard-failure if env var missing); (2) `env("KEY")` returns `Value::String`, and `Value::Secret` appears only when the value is bound to a `Secret`-typed entity (`entity k: Secret = env("KEY")`). |
| `HTML_INJECTION` | XSS via LLM output | `call_llm()` / `call_claude()` / `call_gemini()` → `respond()` without sanitization | A03 Injection | Requires `render()` or `escape_html()` between LLM source and `respond()`. Single-level inline nesting (e.g. `respond(upper(call_llm(...)))`) is detected; deeper nesting is a known boundary. |

## Category B — Advisory (Severity::Warning)

These are heuristic checks. They may false-positive in safe code and
miss complex indirection. They produce warnings, not errors.

| check_id | Threat | Detection Method | OWASP | Why Warning, Not Error |
|---|---|---|---|---|
| `SECRETS` | Hardcoded secrets in source | Regex scan for secret-like patterns in string literals | A02 | Cannot distinguish real secrets from test fixtures or error messages. |
| `OPEN_REDIRECT` | Open redirect via user-controlled URL | `query_param()` / `json_body()` → `respond_html()` without validation | A01 | Custom URL validation (allowlist) is not recognized by the static check. |
| `TAINT_PERSISTENCE` | XSS via memorize/recall persistence | File-level: `memorize(call_llm(...))` + same-scope `recall()` + `respond()` | A03 | Data flow through persistence is not tracked. Same-scope heuristic only. |
| `TAINT_PASSTHROUGH` | XSS via trivial passthrough pattern | `respond(Passthrough(call_llm(...)))` where `Passthrough` is a 1-param `return param` pattern | A03 | Interprocedural taint is not tracked. Only exact trivial passthroughs (1 param, single return) are flagged. |
| `SANDBOX_COVERAGE` | Unsandboxed self-modification | `adapt` / `mutate` without enclosing `sandbox` block | A05 | Sandbox is opt-in. External review or infra-level isolation may handle the risk. |
| `RATE_LIMIT` | Missing rate limiting | No `rate_limit` middleware in `mlogserver` block | A05 | External infra (reverse proxy, CDN) may enforce rate limits. |
| `CSRF` | Missing CSRF protection | No `csrf` middleware in `mlogserver` block | A01 | Token-authenticated APIs do not need CSRF (cookies not used for auth). Cookie-based sessions do. |

## Known Boundaries

Patterns the audit does **not** detect (see README for full table):

- **Interprocedural taint**: LLM output passed through a non-trivial pattern call chain.
- **Persistence taint**: LLM output stored via `memorize()` then read back via `recall()` in a different scope.
- **`format()` in SQL**: `query(format("...", x))` — `format()` output is not a compile-time constant.
- **Inline nesting in open redirect**: `respond_html(query_param("url"))` — the check only tracks via variable, not inline call.
- **Raw template output**: `{{{ var }}}` in `template_render` with `raw=true` skips escaping by design.

## Runtime Protections (Outside Audit)

These are not audit checks but contribute to the overall threat posture:

- **Opaque `Secret` type**: opacity is enforced via two paths: (1) `secret("KEY")` returns `Value::Secret` directly — runtime opacity enforced immediately, hard-failure if env var missing (unlike `env()` which returns empty string); (2) `entity k: Secret = env("KEY")` → `coerce_to_declared_type` produces `Value::Secret` at binding time. In both cases, `Value::Secret` is zeroized on drop, and `print`/concat are blocked at runtime. A bare `env()` call (not bound to a `Secret` entity) returns a plain `String` — protection is then static-only (Category A taint).
- **SSRF prevention**: `http_get` / `http_post` reject private/internal IP ranges unless `METALOGOS_HTTP_ALLOW_PRIVATE=1` is set.
- **Sandbox timeout**: `sandbox { timeout: N }` cancels the *wait* at the deadline (preemptive via `mpsc::recv_timeout`). The in-flight HTTP request to the LLM provider may still complete.
- **File access sandbox**: `sandbox_path()` resolves symlinks via `canonicalize()` and verifies the path stays within the allowed base directory.
