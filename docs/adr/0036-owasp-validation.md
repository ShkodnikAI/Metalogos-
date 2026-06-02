# ADR-0036: OWASP Top 10 Compliance via Language-Level Security

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.7 — OWASP Top 10 Validation

## Context

OWASP Top 10 is the industry-standard awareness document for web application security. Most frameworks address these vulnerabilities through libraries, middleware, and developer education — all of which can be bypassed or misconfigured.

Metalogos has the unique opportunity to encode OWASP protections directly into the language semantics, making violations structurally impossible rather than merely discouraged.

## Decision

Metalogos closes all OWASP Top 10 (2021) categories through language-level security constructs implemented in Phase 6.

### Mapping

| OWASP ID | Vulnerability | Metalogos Mechanism | ADR |
|----------|--------------|---------------------|-----|
| A01 | Broken Access Control | `requires=[role]` on routes, `require` in patterns | 0034 |
| A02 | Cryptographic Failures | `Secret`, `Encrypted`, `Hash` opaque types | 0031 |
| A03 | Injection | `Query` opaque type, string-literal SQL only | 0030 |
| A04 | Insecure Design | Unsafe operations (`eval`, `exec`, raw SQL) do not exist | — |
| A05 | Security Misconfiguration | `HttpOnly`, `Secure`, `SameSite=Strict` headers by default | 0032, 0033 |
| A06 | Vulnerable Components | `mlogpkg.lock` pins all dependency hashes | 0019 |
| A07 | Auth Failures | `hash_password()`/`verify_password()` builtins, Argon2id | 0031 |
| A08 | Data Integrity | CSRF double-submit cookie, HMAC-SHA256 by default | 0033 |
| A09 | Logging/Monitoring | Structured audit log on every 403/auth failure | 0034 |
| A10 | SSRF | Sandbox forbids outbound network connections from handlers | 0023 |

### Principle

Where other frameworks add security layers, Metalogos removes unsafe surface. There is no `raw_query()`, no `String → Html` conversion, no `print(secret)`. The language grammar, type system, and runtime conspire to make violations unexpressible.

## Prior Art

- **Chez Scheme:** Security through language design — no pointer arithmetic, memory-safe by default.
- **Vault (HashiCorp):** Security as a first-class architectural principle, not a feature bolted on.
- **Cloudflare Workers:** Isolate-based sandboxing with no raw filesystem or network access.

## Consequences

- **Positive:** OWASP compliance is a property of the language, not a checklist developers must follow.
- **Positive:** New Metalogos web applications are secure by default — no security configuration required to deploy safely.
- **Positive:** Code review can verify security by checking that no unsafe constructs exist (grep for banned APIs — but there are none).
- **Neutral:** Some advanced use cases (e.g., raw SQL for DB migrations, outgoing SSRF-like calls to trusted APIs) require explicit escape hatches (`unsafe_exec`) with audit logging.
- **Negative:** The restrictive model may frustrate developers porting legacy code that assumes unrestricted access.
