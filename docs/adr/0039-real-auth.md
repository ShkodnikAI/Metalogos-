# ADR-0039: Real Sessions, CSRF, and Rate Limiting (Phase 7.4)

## Status: Accepted

## Context

Phase 6.5 introduced session management with in-memory HashMap storage, HMAC-signed cookies,
and CSRF double-submit cookie pattern. However:
- Sessions were stored only in memory (lost on restart, no persistence)
- CSRF tokens were checked but not generated automatically
- No rate limiting existed — servers vulnerable to brute-force and DoS

For production use, sessions need persistent storage, CSRF tokens need automatic generation
on GET requests, and rate limiting prevents abuse.

## Decision

### 1. SQLite Session Store
- Added `rusqlite = { version = "0.31", features = ["bundled"] }` (no external SQLite needed)
- Table schema: `sessions (id TEXT PK, user_id TEXT, data TEXT, created_at INT, expires_at INT)`
- Index on `expires_at` for fast cleanup queries
- `ServerState.db: Arc<tokio::sync::Mutex<Connection>>` — async-safe SQLite access
- Note: `std::sync::Mutex<Connection>` fails axum's `Handler` trait (likely a `Send` edge case
  with `rusqlite::Connection`). `tokio::sync::Mutex` resolves this.
- Session TTL: 24 hours (expires_at = created_at + 86400)
- `clean_expired_sessions_db()` removes stale entries on demand

### 2. CSRF Double-Submit Cookie
- Token generation: `generate_csrf_token()` — 16 random bytes → 32 hex chars via `rand`
- On GET requests with `csrf` middleware: server generates token, stores in `csrf_tokens` HashMap,
  sets `Set-Cookie: _mlog_csrf=<token>; HttpOnly; SameSite=Strict; Path=/`
- On POST/PUT/DELETE with `csrf` middleware: server compares `_mlog_csrf` cookie value
  with `X-CSRF-Token` header. Mismatch → 403 Forbidden.
- Cookie name standardized to `_mlog_csrf` (was `_csrf_token` in stub)

### 3. Rate Limiting
- Sliding window: `rate_limit(100)` → max 100 requests per minute per IP
- `ServerState.rate_limits: Arc<RwLock<HashMap<String, Vec<Instant>>>>`
- On each request: remove entries older than 60 seconds, check count, reject if ≥ limit
- IP extraction from `X-Forwarded-For` or `X-Real-IP` headers (reverse proxy compatible)
- Response: `429 Too Many Requests` with audit log entry

### 4. Session Cookie Format
- Cookie name: `_mlog_session`
- Value: HMAC-SHA256 signed session ID (`{id}.{signature}`)
- Flags: `HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=86400`
- `make_session_cookie_value()` helper builds the Set-Cookie header

## Contract Tests (16 tests, all passing)

| Contract | Test |
|----------|------|
| CSRF token is random 32 hex chars | `test_74_csrf_token_generation_is_random` |
| POST without CSRF → 403 | `test_74_post_without_csrf_returns_403` |
| POST with matching CSRF → 200 | `test_74_post_with_matching_csrf_returns_ok` |
| POST with mismatched CSRF → 403 | `test_74_post_with_mismatched_csrf_returns_403` |
| Expired session → 401 | `test_74_expired_session_returns_401` |
| Valid session → 200 | `test_74_valid_session_returns_ok` |
| Nonexistent session → 401 | `test_74_nonexistent_session_returns_401` |
| Rate limit under threshold → OK | `test_74_rate_limit_under_threshold_passes` |
| Rate limit exceeded → 429 | `test_74_rate_limit_exceeded_returns_429` |
| Rate limit per-IP isolation | `test_74_rate_limit_per_ip_isolated` |
| Session create + delete roundtrip | `test_74_session_create_and_delete` |
| Expired session cleanup | `test_74_clean_expired_sessions` |
| Client IP extraction | `test_74_extract_client_ip_from_headers` |
| Session cookie format | `test_74_make_session_cookie_value` |

## Security Properties
- Sessions persist in SQLite (survive in-memory resets within server lifetime)
- CSRF tokens are cryptographically random (16 bytes of entropy)
- Rate limiting prevents brute-force attacks on login endpoints
- Session cookies are HttpOnly + Secure + SameSite=Strict
- HMAC-signed cookies prevent tampering
- Expired sessions are cleaned on validation and on demand

## Consequences
- `ServerState` gains `db` and `rate_limits` fields
- All DB functions are now `async` (use `tokio::sync::Mutex`)
- Builtins (`session_login`, `session_logout`, `authenticate`) remain stubs for interpreter mode
- Real session management happens at server middleware level
- `rate_limit` is a new middleware name recognized in mlogserver config
