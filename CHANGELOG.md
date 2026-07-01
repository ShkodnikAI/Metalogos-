# Changelog

All notable changes to the Metalogos project.

## [0.8.1] — 2026-07-01

**Phase 8.1: Human Intelligence Layer — система персон, память, настроение, человекоподобные AI-ответы.**
**Вдохновлено [OpenHuman](https://github.com/tinyhumansai/OpenHuman) — ключевые концепции (memory tree, persona, mood) реализованы как нативные builtins Metalogos.**

### Новые builtins (8 функций, итого 116)

**Human Intelligence Layer (8):**
- **`human_create(name, traits)`** — создать персону с чертами характера. Хранится в KV (персистентно). Возвращает {name, traits, created_at, memory_count}
- **`human_mood(persona, mood?, intensity?)`** — получить/установить эмоциональное состояние. Настроение влияет на тон LLM-ответов. Mood: happy/sad/focused/creative/neutral/excited и т.д. Intensity 0.0–1.0
- **`human_remember(persona, key, content, importance?)`** — сохранить воспоминание. Importance 0.0–1.0 (по умолчанию 0.5). Хранится в KV с метаданными
- **`human_forget(persona, key?)`** — удалить воспоминание по ключу ("ok"/"not_found") или все воспоминания персоны (возвращает количество)
- **`human_recall(persona, query, limit?)`** — поиск по памяти с composite scoring: 50% релевантность (keyword match) + 30% важность + 20% свежесть (half-life ~1 неделя). Возвращает отсортированный список {Memory} с полем score
- **`human_respond(persona, message, context?)`** — генерация человекоподобного ответа через LLM с учётом характера, настроения и релевантных воспоминаний. Автоматически вызывает human_recall
- **`human_personas()`** — список всех персон с текущим настроением и количеством воспоминаний
- **`human_delete(persona)`** — удаление персоны + всех её воспоминаний. Возвращает {deleted_memories, status}

### Архитектура

- Построено на существующих примитивах Metalogos: KV-хранилище (kv_set/kv_get) для персистентности, call_llm для генерации
- Нулевые новые зависимости — только serde_json (уже в проекте)
- Поддержка SQLite-персистенции (write-through через существующий KV_SQLITE)
- Mock-режим по умолчанию (METALOGOS_LLM_MOCK=true) — работает без LLM-провайдера

### Документация

- REFERENCE.md: новый раздел 4.22 «Human Intelligence Layer» с полным описанием всех 8 функций, структур возврата, алгоритма скоринга и примерами
- README.md: новый раздел «What's New in v0.8.1» с примерами, обновлён раздел «Why Metalogos» (новый pillar «Human Intelligence Layer»), обновлены Prior Art и Architecture
- CHANGELOG.md: данная запись

### Версия

- Bump 0.8.0 → 0.8.1

---

## [0.8.0] — 2026-07-01

**Phase 8: Время, дата, календарь, геолокация, погода (бесплатно), напоминания.**

### Новые builtins (15 функций, итого 108)

**Время, дата, календарь (8):**
- **`format_date(fmt, timestamp?)`** — форматирование даты/времени. Шаблоны: `%Y/%y/%m/%d/%H/%I/%M/%S/%p/%A/%a/%B/%b/%j/%w/%W/%F/%T/%R`
- **`date_parts(timestamp?)`** — возвращает struct {Date} со всеми компонентами: year, month, day, hour, minute, second, weekday, weekday_name, month_name, day_of_year, week_number, timestamp
- **`days_between(ts1, ts2)`** — абсолютная разница в днях
- **`days_in_month(year, month)`** — дни в месяце с учётом високосных годов
- **`is_leap_year(year)`** — проверка високосного года
- **`add_days(timestamp, days)`** — прибавить/вычесть дни
- **`add_hours(timestamp, hours)`** — прибавить/вычесть часы
- **`weekday_name(timestamp)`** — полное название дня недели

**Геолокация (2):**
- **`geo_ip(ip?)`** — геолокация по IP через ip-api.com (бесплатно, без ключа). Возвращает {ip, city, region, country, country_code, lat, lon, isp, timezone}
- **`geo_distance(lat1, lon1, lat2, lon2, unit?)`** — расстояние по гаверсинусу. Единицы: km (по умолч.), mi, nm, m

**Погода (2, Open-Meteo — БЕСПЛАТНО, БЕЗ API-КЛЮЧА):**
- **`weather(city_or_lat, lon?)`** — текущая погода. Автоматическое разрешение города через Open-Meteo Geocoding. Возвращает {temp, feels_like, humidity, description, wind_speed, wind_direction, pressure, cloud_cover, is_day, city}
- **`weather_forecast(city_or_lat, days?)`** — прогноз на 1–16 дней. Возвращает список DayForecast: {date, temp_max, temp_min, precipitation, description, wind_speed_max, sunrise, sunset, uv_index}
- Заменён OpenWeatherMap (требовал API-ключ) на Open-Meteo (никаких ключей)
- WMO weather codes → человекочитаемые описания (Clear sky, Partly cloudy, Rain, Snow, Thunderstorm и т.д.)

**Напоминания (5):**
- **`remind(message, timestamp, data?)`** — одноразовое напоминание, возвращает ID
- **`remind_recurring(message, interval_seconds, data?)`** — повторяющееся (86400 = день, 604800 = неделя)
- **`cancel_remind(id)`** — отмена ("ok" / "not_found")
- **`list_reminders()`** — список активных
- **`check_reminders()`** — просроченные (одноразовые деактивируются, повторяющиеся сдвигаются)

### Документация

- REFERENCE.md полностью переработан: новые разделы 4.17–4.20 (время, геолокация, погода, напоминания)
- README.md: новый раздел "What's New in v0.8.0", обновлены таблицы и roadmap
- Обновлённые docs/book/src/ (syntax.md, stdlib.md)

### Версия

- Bump 0.7.10 → 0.8.0

---

## [0.7.10] — 2026-06-18

**4 критических багфикса + документация.**

### Новое в языке

- **Логические операторы `and` / `or`** — короткое замыкание (short-circuit). Работают во вложенных условиях, интерпретаторе, компиляторе байткода и VM. `and` возвращает `false` если левый операнд falsy, иначе вычисляет правый. `or` возвращает `true` если левый truthy, иначе вычисляет правый
- **`if expr then { ... }` как statement** — ранее `then` работал только в тернарном выражении `if cond then a else b`. Теперь блочная форма `if x > 5 then { ... }` корректно парсится как оператор
- **Unicode escape `\uXXXX`** — строковые литералы поддерживают `\u0041` → `A` и любые валидные Unicode code points. Парсер (grammar + unescape) и рантайм

### Исправления

- **Unit equality** — `Unit == ""`, `Unit == 0.0`, `Unit == "test"` больше не крашат. `Unit == Unit` → `true`, `Unit == anything_else` → `false`. Работает в интерпретере и VM
- **query_scalar dereference** — `ValueRef::Integer(n)` и `ValueRef::Real(f)` в rusqlite обработчике: убран ошибочный `*n` / `*f` (E0614, значения уже Copy)

### Документация

- REFERENCE.md обновлён: добавлены `and`/`or` в таблицу операторов, `\uXXXX` в escape-последовательности, `if-then` блочная форма, исправлен пример `json_get` + Unit comparison

---

## [0.7.9] — 2026-06-15

**Наряд №24: аудит-доработка — новые builtins, исправления, конвергенция бинарников.**

### Новые builtins

- **`git_push(message)`** — `git add/commit/push` через subprocess. Использует `GITHUB_TOKEN` и `GITHUB_REPO` env vars (Наряд 24 A3)
- **`web_search(query, num_results)`** — поиск через SerpAPI. Использует `SERPAPI_KEY` env. Возвращает raw JSON (Наряд 24 A4)
- **`make_list(a, b, c, ...)`** — создание списка из вариативных аргументов. Устраняет race condition от write_file/read_file при возврате нескольких значений (Наряд 24 B2)

### Исправления

- **Graceful unknown function** — вызов несуществующей функции возвращает `Ok(String("[ERROR: unknown function '...']"))` вместо `Err(...)` и краша. Критично для fosved-v2 (Наряд 24 B1)
- **LLM timeout 30→120с** — увеличен таймаут в `call_claude()` (builtins.rs) и `RealLlm` (llm.rs) для сложных запросов (Наряд 24 B3)
- **json_get с числовыми индексами массивов** — `json_get(data, "items.0.title")` теперь работает: числовые сегменты path трактуются как индексы `Value::List` (Наряд 24 B4)
- **send_message: реальная Telegram API отправка** — при наличии `TELEGRAM_BOT_TOKEN` env отправляет сообщение через Telegram API. Поддерживает отрицательные channel ID (как `i64` в JSON). Без токена — audit stub (Наряд 24 B5)

### Статистика

- 100 уникальных builtins (89 публичных + 11 `__`-префиксных внутренних)
- Бинарник: 12 MB (Linux x86_64)

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