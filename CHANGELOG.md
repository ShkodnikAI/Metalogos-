# Changelog

All notable changes to the Metalogos project.

## [0.4.0] — 2025-06-03

**Phase 6: Full-stack web platform with security by design.**

### Security — 6 levels, OWASP Top 10 closed

- **Type-safe HTML templates** — `template` construct returns opaque `Html` type, auto-escaping prevents XSS
- **Parameterized database queries** — `query(sql_literal, params)`, opaque `Query` type, SQL injection syntactically impossible
- **Encryption primitives** — `Secret`, `Encrypted`, `Hash` opaque types; `env()` maps to `Secret`; `encrypt`/`decrypt` via AES-256-GCM; `hash_password`/`verify_password`
- **Authentication & authorization** — session management (HMAC-SHA256 signed cookies), role-based access (`requires=[role]`), `require` assertions, `authenticate`/`session_login`/`session_logout`
- **CSRF & security headers** — double-submit token pattern, CSP/HSTS/X-Frame-Options/X-Content-Type-Options middleware
- **LLM sandbox** — sandboxed execution for learnable patterns, no direct HTML injection from AI responses

### Web platform

- **HTTP server** — `mlogserver` block with `port`, `middleware`, `route` declarations (Axum 0.8 + Tokio)
- **Routing** — `route "/path" method=GET/POST requires=[roles] { handler }`
- **Request parsing** — `form_data()`, `json_body()` built-in functions
- **Response** — `respond(status)`, `render(template, args)` for HTML output
- **Bot integration** — Telegram/Discord webhook routes, `send_message(chat_id, text)` outbound HTTP
- **CLI** — `mlog serve <file>` starts the HTTP server

### Language additions

- **`db` block** — database configuration with `pool_size` and `migrate`
- **`template` construct** — type-safe HTML templates with `{{ var }}` auto-escaping
- **`require` statement** — runtime assertion for authorization checks
- **40+ built-in functions** across string, math, web, crypto, auth, and bot domains

### Examples

- `p6_full_app.mlog` — 170-line full-stack application demonstrating all 6 security levels: auth, CRUD, AI classification, and bot webhooks in a single `.mlog` file

### Infrastructure

- 26 Architecture Decision Records (`docs/adr/`)
- Phase 6 contract tests (`tests/phase6_contract.rs`)
- Landing page for GitHub Pages

---

## [0.3.0] — Phase 5: Language completeness

**Control flow, collections, string operations, modules.**

- `let` bindings with `if/else` expressions
- `each item in list { ... }` and `while cond { ... }` loops
- List literals `[1.0, 2.0, 3.0]` with `get`, `push`, `len`
- String operations: `index_of`, `substring`, `char_at`, `starts_with`, `ends_with`, `contains`, `split`, `join`, `trim`, `replace`
- Module system: `import std/string as str` with qualified calls (`str.trim(s)`)
- REPL integration tests, semantic check integration tests

---

## [0.2.0] — Phases 1–4: Core language, types, ML, ecosystem

**Probabilistic types, ML backend, knowledge graph, vector recall.**

- **Phase 1**: Fluid types with probabilistic superposition, confidence propagation, entity store queries (`find()`)
- **Phase 2**: Knowledge graph (`relate`), vector recall (semantic memory), full adapt system (sandbox/mutate/rollback), ML learn statement
- **Phase 3**: CLI (`mlog run/repl/check`), rustyline-based REPL with multiline support
- **Phase 4**: Code generation stubs, intermediate representation

---

## [0.1.0] — M1–M5: Seven pillars, basic interpreter

**The foundation — AI-native language with seven semantic primitives.**

- **M1**: Entity (simple, struct, instance), pure pattern, linear flow, built-in functions (`upper`, `lower`, `len`, etc.)
- **M2**: Struct entities, rule engine with priority and confidence-based flow branching
- **M3**: Learnable patterns (LLM backend trait + mock), prompt engineering, few-shot caching, `adapt` statement
- **M4**: Semantic memory (`memorize`/`recall`/`forget`), knowledge graph (`relate`), memory decay
- **M5**: Sandbox execution, `mutate` with rollback on degradation
- Pest PEG grammar, hand-written AST, tree-walking interpreter
- Golden-file test framework (`examples/*.expected`)
