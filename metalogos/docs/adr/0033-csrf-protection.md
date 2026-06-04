# ADR-0033: CSRF Protection via Double-Submit Cookie Pattern

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.5 — CSRF Protection

## Context

Cross-Site Request Forgery (CSRF) tricks an authenticated user's browser into submitting unwanted requests to a vulnerable application. OWASP ranks CSRF as a top vulnerability (A08:2021).

Metalogos web applications need CSRF protection that is enabled by default with no opt-in ceremony from the developer.

## Decision

Implement the **double-submit cookie pattern**: a CSRF token is set as a cookie on GET requests and must be submitted as a header on state-changing requests.

### Protocol

1. **GET Response:** Server generates a random 256-bit token, sets cookie `_csrf_token=<token>` with `SameSite=Strict`, `HttpOnly=false` (JavaScript must read it).
2. **POST/PUT/DELETE Request:** Require header `X-CSRF-Token: <token>`.
3. **Validation:** Compare `X-CSRF-Token` header value against `_csrf_token` cookie. If mismatch or missing → **403 Forbidden**.
4. **Exemptions:** Safe methods (GET, HEAD, OPTIONS) are never checked.

```mlog
mlogserver {
    route POST "/transfer" => {
        // CSRF validated automatically before handler runs
        // If invalid, 403 is returned; handler never executes
        respond status:200 body:"Transfer complete"
    }
}
```

### Implementation

- CSRF middleware sits in the Tower stack after session validation.
- Token generation uses `ring::rand::SystemRandom` for cryptographic quality.
- Constant-time comparison (`ring::hmac`) to prevent timing attacks on token validation.

## Prior Art

- **Django CSRF Middleware:** Same double-submit pattern with `CsrfViewMiddleware`.
- **Express.js `csurf`:** Token in cookie, validated on POST via hidden form field or header.
- **AngularJS:** `$http` automatically reads `XSRF-TOKEN` cookie and sends `X-XSRF-TOKEN` header.

## Consequences

- **Positive:** CSRF protection is automatic — no developer action required to enable it.
- **Positive:** Double-submit pattern works without server-side session storage (stateless validation).
- **Neutral:** JavaScript on the client must read `_csrf_token` cookie and attach it to AJAX requests; the Metalogos standard library template includes this automatically.
- **Negative:** Subdomains sharing the cookie domain could exploit the token if CORS is misconfigured. Mitigated by `SameSite=Strict`.
