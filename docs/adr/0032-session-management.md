# ADR-0032: Session Management with HMAC-SHA256 Signed Cookies

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.5 — Session Management

## Context

Web applications need stateful session management to track authenticated users across requests. Poorly implemented sessions lead to session fixation, hijacking, and unauthorized access.

Metalogos needs a built-in session mechanism that is secure by default, requiring no configuration to achieve OWASP-recommended protections.

## Decision

Sessions use **HMAC-SHA256 signed session cookies** with an in-memory session store.

### Protocol

1. **Session Creation:** On successful authentication, generate a cryptographically random session ID (256-bit).
2. **Signing:** Compute `HMAC-SHA256(session_id, server_secret)`. Set cookie `session_id.signature`.
3. **Cookie Attributes:** `HttpOnly`, `Secure` (HTTPS only), `SameSite=Strict`.
4. **Validation:** On each request, recompute HMAC. If signature doesn't match → reject session.
5. **In-Memory Store:** `HashMap<String, SessionData>` with optional TTL expiry.
6. **Rotation:** Session ID regenerated on privilege change (login, role elevation).

```mlog
session.set("user_id", user.id)
session.set("role", user.role)
let user_id = session.get("user_id")
session.destroy()
```

### Server Secret

Loaded via `env("SESSION_SECRET")` → `Secret` type (ADR-0031). Never logged, never exposed in diagnostics.

## Prior Art

- **Ruby on Rails:** Signed cookies with `Rails.application.credentials.secret_key_base`.
- **Express.js `cookie-session`:** HMAC-signed cookies, no server-side store option.
- **Django:** Session middleware with `SECURE_SSL_REDIRECT`, `SESSION_COOKIE_SECURE`, `SESSION_COOKIE_HTTPONLY`.

## Consequences

- **Positive:** Session fixation is mitigated by ID rotation on privilege changes.
- **Positive:** `HttpOnly` prevents JavaScript access; `SameSite=Strict` blocks CSRF-style cross-origin attacks.
- **Positive:** HMAC signing detects tampered session cookies without needing a database lookup.
- **Neutral:** In-memory store limits horizontal scaling; Redis-backed store can replace it later.
- **Negative:** Session data is lost on process restart; persistent store needed for production deployments.
