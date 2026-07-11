# Changelog

All notable changes to the Metalogos project.

## [0.9.1] — 2026-07-12

**Наряд 4-примитивов: Problems B + D (Problem B: aggregation, Problem D: webhook diagnosis).**

### Problem B — Aggregation over list of structs (ADR-0059)

- **`map()` в VM** — `map(list, "pattern_name")` теперь работает во всех трёх бэкендах (tree-walking, bytecode/VM, JIT). Ранее — только tree-walking.
- **`map`, `zip`, `sort_by`, `filter`, `reduce` добавлены в BUILTIN_REGISTRY** — ранее отсутствовали, компилятор не мог создать `CallBuiltin` для них.
- **`IndexAccess` в execute_code** — паттерны в VM теперь могут использовать `list[N]` и `struct["key"]` (раньше инструкция обрабатывалась только в main loop).
- **`entity` как struct** — STOP Trigger #1 подтверждён: `entity TypeName { ... }` полностью покрывает потребность в `struct`. Новый ключевой код не добавлен (ADR-0059).

### Problem D — Webhook routing diagnosis (ADR-0061)

- Диагностика: `Hook` (ADR-0045) — AOP для паттернов, не для HTTP. `route` — полноценный HTTP-роутер, достаточный для Telegram webhook. Корень бага — архитектурный (reverse_proxy.py маршрутизирует `/webhook/*` в Python, mlog-обработчик физически недостижим).
- Golden test: `telegram_webhook_route.mlog` — проверяет `parse_json` + `json_get` на mock Telegram update JSON.

---

## [0.9.0] — 2026-07-07

**Unified Builtin Registry — Single Source of Truth refactoring.**

### Architecture

- **`BuiltinSpec` struct + `BUILTIN_REGISTRY` const** — 135 builtins with name, arity, and category in a single master table (`builtins.rs`)
- **Helper functions** — `builtin_names()`, `builtin_indices()`, `builtin_name_set()`, `builtin_arity_map()`, `is_builtin()`, `builtin_count()` — all derived from the registry
- **compiler.rs** — hardcoded 26-entry builtin array replaced with `builtin_indices()` call
- **vm.rs** — hardcoded 26-entry `builtin_names` vec replaced with `builtin_names()` call
- **semantic.rs** — hardcoded 28-entry `builtin_names` set replaced with `builtin_name_set()` call
- **Debug sync check** — `Builtins::check_registry_sync()` asserts (in debug builds) that every non-stateful registry entry has a handler in `Builtins::new()`
- **Duplicate `env` registration removed** (was inserted twice at lines 28 and 70)
- **Before**: adding 1 builtin required editing 5 files; **After**: 1 row in `BUILTIN_REGISTRY` + 1 insert in `Builtins::new()`

### Registry categories

135 builtins organized into categories: string, convert, list, math, std, web, json, crypto, auth, db, llm, memory, io, time, bot, voice, stateful, graph, mtree, cron, test, encoding, stub, fluid, system

---

## [0.7.8] — 2026-06-15

**Наряд №17 closure: BlockIfElse expression in bytecode compiler, format() arity fix.**

### Bytecode compiler

- **`Expr::BlockIfElse` full bytecode compilation** — `if cond { ... } else { ... }` as expression now compiles to a proper conditional jump chain with result slot, instead of emitting `Const(Unit)` placeholder (Наряд 17 Б.1)
- New `compile_body_expr` method — compiles statement blocks in expression context, storing the last expression's value into a result local slot
- `format()` arity corrected from `-1` (variadic) to `1` (template-only) in semantic arity checks

### Bug fixes

- Block if/else expression in VM path no longer silently returns `Unit`; the value of the last expression in the matched branch is correctly propagated to the stack

---

## [0.7.7] — 2026-06-14

**Phase 7.7: Break/Continue, Match arms, compiler full-coverage, security constraints.**

### Language

- **`break` and `continue`** statements in `each`, `each_with_index`, and `while` loops (Наряд 17)
- **`MatchArm::StartsWith`** — bytecode instruction `StartsWith` + VM execution + compiler codegen (Наряд 17)
- **`MatchArm::Compare`** — threshold-based match arms with full compiler support
- **`Statement::IfElseBlock`** — multi-branch `if/else if/else` as statement with full compiler coverage (Наряд 18)
- **`Expr::BlockIfElse`** — block if/else as expression in interpreter (Наряд 14)
- **`Expr::Try`** — try/catch expression, catches errors and returns `Unit` (Наряд 14)

### Bytecode compiler

- Full statement compilation: `LetBinding`, `Assign`, `Return`, `ExprStmt`, `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match`, `Break`, `Continue` (Наряд 18)
- Loop context (`LoopCtx`) for break/continue jump patching — continue jumps back to condition, break jumps to loop end
- `Match` with `Exact`, `StartsWith`, `Contains`, `Compare` arms — all compiled to conditional jump chains
- Global variable slots, `StoreGlobal` instruction (Наряд 22)
- 44 total VM instructions in the bytecode instruction set

### VM

- `StartsWith` instruction — string prefix check, pushes 1.0 (true) or 0.0 (false)
- `StoreGlobal` instruction — write to global variable slot
- `execute_code` method with `&mut self` for mutable global state in pattern execution
- `IndexAccess`, `ListLen`, `MakeList`, `MakeStruct`, `GetField` — collection and struct support

### Semantic analysis

- Opaque type enforcement across all statement types: `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match` (all 4 arm variants)
- Tool declaration body analysis
- Static security audit (`mlog audit`) coverage for new statement forms

### Security constraints (Наряды 19–22)

- `inspect` builtin — introspect variable values without violating opaque types (Наряд 19)
- Context loading from `Entity`/`Memory`/`Fluid` declarations before pattern execution (Наряд 20)
- Event streaming: `emit`/`on` event hooks (Наряд 20)
- Conversation state: `Conversation` declaration with TTL and message limits (Наряд 21)
- LLM response cache with configurable TTL (Наряд 21)
- Model routing: `LlmConfig` declaration with provider failover (Наряд 21)
- Context compression for long conversations (Наряд 21)
- Tool abstraction: `Tool` declaration with typed methods (Наряд 22)
- `Hook` declaration: before/after pattern hooks (Наряд 22)
- Session memory: `session_set`/`session_get`/`session_clear` builtins (Наряд 22)

### Infrastructure

- 32 integration test files (7 000+ lines of tests)
- 63 Architecture Decision Records
- CI pipeline: build + release binary (Linux x86_64)

---

## [0.7.5] — 2026-06-13

**Phase 7.5–7.6: Memory persistence, tokens, eval harness, session memory, audit.**

- Memory persistence e2e tests (JSON file-based storage)
- JWT-style token generation and verification
- Eval harness for testing learnable patterns with golden-file assertions
- `session_set`/`session_get`/`session_clear` session memory builtins
- Audit parser integration tests
- Server JSON body parsing for POST routes

---

## [0.7.3] — 2026-06-12

**Phase 7.3–7.4: Context compression, lifecycle, tool abstraction, hooks, DoD.**

- Context compression for long conversations
- Lifecycle control for flows and patterns
- Tool abstraction (`Tool` declaration)
- `Hook` declaration for before/after pattern execution
- Definition of Done framework with automated checks

---

## [0.7.1] — 2026-06-10

**Phase 7.1–7.2: Inspect, context loading, events, conversation state, LLM cache, model routing.**

- `inspect()` builtin for safe value introspection
- Context loading from entity/memory/fluid declarations
- Event streaming (`emit`/`on`)
- `Conversation` declaration with TTL and message limits
- LLM response cache with configurable TTL
- `LlmConfig` declaration for multi-provider model routing

---

## [0.6.0] — 2025-06-03

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

- `p6_full_app.mlog` — 170-line full-stack application demonstrating all 6 security levels

---

## [0.5.0] — Phase 5: Language completeness

**Control flow, collections, string operations, modules, bytecode VM, JIT.**

- `let` bindings with `if/else` expressions
- `each item in list { ... }` and `while cond { ... }` loops
- `break` and `continue` in loops
- `match` expression with `exact`, `starts_with`, `contains`, `compare` arms
- List literals `[1.0, 2.0, 3.0]` with `get`, `push`, `len`, `first`, `last`, `reverse`
- String operations: `index_of`, `substring`, `char_at`, `starts_with`, `ends_with`, `contains`, `split`, `join`, `trim`, `replace`
- Module system: `import std/string as str` with qualified calls (`str.trim(s)`)
- Bytecode VM: 44 instructions, stack-based execution
- JIT compiler via Cranelift
- Self-hosted lexer
- REPL integration tests, semantic check integration tests

---

## [0.3.0] — Phases 1–4: Core language, types, ML, ecosystem

**Probabilistic types, ML backend, knowledge graph, vector recall, LSP, packages.**

- **Phase 1**: Fluid types with probabilistic superposition, confidence propagation, entity store queries (`find()`)
- **Phase 2**: Knowledge graph (`relate`), vector recall (semantic memory), full adapt system (sandbox/mutate/rollback), ML learn statement
- **Phase 3**: CLI (`mlog run/repl/check`), LSP server, `mlogpkg` package manager, mdbook documentation
- **Phase 4**: Bytecode VM, JIT compiler (Cranelift), self-hosted lexer, IR generation

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