---
Task ID: 1
Agent: main
Task: Phase 7.1 — Real LLM Backend (Anthropic, OpenAI, Ollama)

Work Log:
- Read existing src/llm.rs (MockLlm + curl-based RealLlm stub)
- Added reqwest dependency with rustls-tls (no OpenSSL) to Cargo.toml
- Rewrote src/llm.rs (~500 lines): Provider enum, RealLlm with 3 providers
- Implemented Anthropic Claude API client (x-api-key, anthropic-version, content[].text parsing)
- Implemented OpenAI GPT API client (Authorization: Bearer, choices[].message.content parsing)
- Implemented Ollama local model client (no auth, response field parsing)
- Added retry logic: 3 retries with exponential backoff (1s, 2s, 4s), 30s timeout
- Added error classification: fatal 4xx (no retry), 429 rate limit (retry), 5xx (retry)
- Updated interpreter.rs: JSON response auto-parsing to Value::Struct + json_value_to_value helper
- Updated vm.rs: same JSON parsing + json_to_value helper
- Fixed eval_expr visibility (pub(crate) → pub) for phase6_contract tests
- Created ADR-0037-real-llm-backend.md
- 31 LLM tests green (28 passed + 3 ignored integration tests)
- Committed as b97dd38, pushed to GitHub

Stage Summary:
- Phase 7.1 COMPLETE: learnable pattern now works with real AI models
- 3 providers supported: Anthropic, OpenAI, Ollama
- All existing mock tests pass unchanged
- Pre-existing Phase 6 test failures (template parsing, confidence) noted but not caused by this change

---
Task ID: 2
Agent: main
Task: Phase 7.3 — Real Encryption (Argon2id, AES-256-GCM, Zeroize)

Work Log:
- Read current src/builtins.rs: stub hash_password (DefaultHasher), mock encrypt/decrypt (XOR), mock generate_key
- Read src/interpreter.rs: Value::Secret(String) — bare string, no memory protection
- Added argon2 = "0.5" and zeroize = "1" to Cargo.toml
- Created SecretString wrapper (interpreter.rs): wraps Zeroizing<String>, implements serde (serializes as "[SECRET]"), Deref for ergonomic access
- Changed Value::Secret(String) → Value::Secret(SecretString) in interpreter.rs
- Implemented real hash_password: Argon2id with random salt via OsRng, PHC format output
- Implemented real verify_password: PasswordHash::new() + Argon2::verify_password with constant-time comparison
- Implemented real encrypt: AES-256-GCM with random 96-bit nonce, stores nonce||ciphertext format
- Implemented real decrypt: splits nonce, decrypts AES-256-GCM, returns Err on wrong key
- Implemented real generate_key: 32 random bytes via rand::thread_rng(), hex-encoded
- Updated all Value::Secret constructors in builtins.rs, server.rs, tests/phase6_contract.rs
- Wrote 8 contract tests: verify roundtrip, wrong password, encrypt/decrypt roundtrip, wrong key, key size, print block, hash format, random salt
- All 8 Phase 7.3 tests pass, all 5 Phase 6.4 encryption tests pass, MockLlm untouched
- Created ADR-0038: docs/adr/0038-real-encryption.md
- Committed as 7a0f766, pushed to origin/main

Stage Summary:
- hash_password: Argon2id with random salt, PHC format ($argon2id$v=19$m=...)
- verify_password: constant-time comparison, returns false on wrong password (not panic)
- encrypt/decrypt: AES-256-GCM, random 96-bit nonce, wrong key returns Err (not panic)
- generate_key: 256-bit cryptographically random, hex-encoded
- Zeroize: Value::Secret wraps SecretString(Zeroizing<String>), memory zeroed on drop
- Serde safety: Secret serializes as "[SECRET]", never persists actual value
- Files changed: Cargo.toml, src/interpreter.rs, src/builtins.rs, src/server.rs, tests/phase6_contract.rs, docs/adr/0038-real-encryption.md

---
Task ID: 3
Agent: main
Task: Phase 7.4 — Real Sessions, CSRF, Rate Limiting

Work Log:
- Read current server.rs: in-memory HashMap sessions, HMAC cookie signing, CSRF stub
- Added rusqlite = "0.31" (bundled) to Cargo.toml
- Discovered: std::sync::Mutex<rusqlite::Connection> breaks axum Handler trait
- Solution: use tokio::sync::Mutex<Connection> for async-safe access
- Added SQLite session store: sessions table (id TEXT PK, user_id TEXT, data TEXT, created_at INT, expires_at INT)
- Implemented create_session_db, validate_session_in_db, delete_session_db, clean_expired_sessions_db (all async)
- CSRF: generate_csrf_token() (16 random bytes → 32 hex), cookie _mlog_csrf, X-CSRF-Token header
- GET requests with csrf middleware auto-generate token and Set-Cookie header
- Rate limiting: sliding window per IP, HashMap<IpAddr, Vec<Instant>>, 100 req/min default
- Client IP extraction from X-Forwarded-For / X-Real-IP headers
- Session cookie: _mlog_session with HttpOnly, Secure, SameSite=Strict, Max-Age=86400
- Wrote 16 contract tests: CSRF (4), sessions (5), rate limiting (3), utilities (4)
- All 16 Phase 7.4 tests pass, 8 Phase 7.3 tests pass, all LLM tests pass
- Created ADR-0039: docs/adr/0039-real-auth.md
- Committed as 2e3522e, pushed to origin/main

Stage Summary:
- SQLite in-memory session store with 24h expiry and index on expires_at
- CSRF double-submit: random token on GET, cookie+header match on POST/PUT/DELETE
- Rate limiting: per-IP sliding window (100 req/min), 429 response
- Key discovery: rusqlite::Connection must use tokio::sync::Mutex (not std::sync::Mutex) for axum Handler
- Files changed: Cargo.toml, src/server.rs, docs/adr/0039-real-auth.md
- Total: 631 insertions, 125 deletions
