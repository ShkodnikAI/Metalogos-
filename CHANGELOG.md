# Changelog

All notable changes to the Metalogos project.

## [Unreleased]

### Module size policy (Наряд №38)
- ADR-0080: module size policy — production files ≤2,000 lines, tests exempt.
  Supersedes the 800-line rule from №37.
- Interpreter: extracted `execution.rs` (1,645 lines) from `mod.rs` (2,178 → 539).
  Moved `run()`, `eval_expr()`, `eval_statements()`, `eval_binop()`, `invoke()`.
- Builtins: extracted `office.rs` (1,724 lines) from `server.rs` (2,579 → 865).
  Moved human, goal, todo, recipe, DAG, semantic search, config builtins.
- Builtin form audit: confirmed №37 split preserved `fn builtin_xxx()` form
  in all 8 extracted modules. No closure re-registration occurred.

### Code quality (Наряд №38)
- Clippy: zero warnings on `--all-targets` (was: compilation failure).
  Fixed unused imports, missing struct fields, private function access,
  bool_assert_comparison, len_zero, unnecessary_mut, cloned_ref_to_slice_refs,
  useless_conversion, redundant_closure across 15 files.
- CI: `clippy` job promoted from advisory to blocking. `continue-on-error` removed.
- Session store test helpers (`reset_session_store`, `session_key_count`,
  `session_store_count`) made `pub` for integration test access.

### VM feasibility assessment (Наряд №38)
- ADR-0081: VM-for-serve feasibility with FOSVED-office-v2 data.
  22/23 .mlog files pass `mlog check`. 4 files blocked by missing `And`/`Or`
  short-circuit evaluation in VM (92 combined occurrences). Single well-scoped
  fix needed before `mlog serve` can switch to VM backend.

### Code quality (Наряд №37)
- Clippy: zero warnings (was 192). Categories fixed: get(0)→first(), doc formatting,
  redundant closures, unnecessary mut/return/clone, new_without_default (11 types),
  dead_code cleanup, matches!/sort_by_key/clamp/flatten/Entry API, and more.
- CI: `fmt` gate restored to green with `cargo fmt` commit.
- ADR: resolved 8 numbering collisions (0076 duplicate, 071 missing leading zero).
  Renamed 0076-vm-dispatch-paths.md → 0077-vm-dispatch-paths.md.
- ADR-0075: clarified 58/58 crosscheck — zero both-error cases (all 58 are genuine
  both-success matches).
- Builtins: split builtins.rs (10,838 lines) into 15 modules: mod, registry, core,
  string, math, collections, crypto, llm, http, json, io, memory, cron, server, tests.
  mod.rs reduced to 581 lines.
- Interpreter: split interpreter.rs (5,073 lines) into 10 modules: mod, values, types,
  events, db, conversations, learnable, modules, flow, memory, hooks.
  mod.rs reduced to 2,172 lines.
- Parser: split parser.rs (4,921 lines) into 5 modules: mod, helpers, expr, stmt,
  decl, tests. mod.rs reduced to 128 lines.
- Documentation: added docs/refactoring-split-plan.md with per-function module mapping.
- No logic changes in any split — pure code moves.

### VM backend (Наряд №36)
- VM: crosscheck 58/58 — all golden examples match between tree-walking interpreter
  and bytecode VM. Zero mismatches, zero VM errors. `assert!(mismatches.is_empty())`
  now enabled in crosscheck test.
- VM: `find()` entity store query handler added — searches globals for structs
  matching type, field, and comparison operator.
- VM: `resolve_skill_index()` handler added — skill_index declarations now compiled
  into Program (CompiledSkillIndex/CompiledSkillTier/CompiledSkillTriggerRule).
- VM: database support — `db_conn`, `db_insert`, `query_scalar`, `query`, `db_execute`
  handlers added. DB URL extracted from `db` declaration at compile time.
- VM: schema DDL generation — `schema` declarations now generate
  `CREATE TABLE IF NOT EXISTS` SQL, executed at VM startup.
- VM: `context: recall(text, limit=N)` and `context: auto` now work in VM.
  Added `CompiledContextMode` enum (None/Auto/Recall/Literal) and `recall_top()`
  for multi-entry memory retrieval with `format_context_block` formatting.
- Compiler: `call_builtin` and `execute_code` changed to `&mut self` for DB support.
  Name cloning resolves borrow conflicts in CallBuiltin dispatch.
- ADR-0075 updated: all 9 remaining cases resolved. Crosscheck assertion enabled.

### VM backend (Наряд №35)
- VM: `eval_cmp()` now handles String-String comparisons (was Float-only via
  `as_float()`). `"" == ""` now correctly returns true. Fixes while/each loops
  that checked `result == ""` — crosscheck 45/58 → 48/58 (3 cases).
- VM: `MakeStruct` and `Contains` added to `execute_code()` (were only in `run()`).
  Pattern calls from flow pipelines now correctly handle struct literals.
  Fixes dag_demo.mlog — crosscheck 48/58 → 49/58 (1 case).
- VM: `CmpNe` added to `eval_cmp()` numeric path (was `_ => false`).
- ADR-0075 updated: 4 cases resolved in №35, 9 remaining documented with root causes.
  Crosscheck threshold raised to 49/58.
- Remaining VM divergences: memory subsystem (3), rule/find (1), flow source
  expression BinOp limitation (1), skill_index (1), DB builtins (2), modules (1).

### VM backend (Наряд №34)
- Compiler: While, Each, EachWithIndex, Assign, IfThen, IfElseBlock, Break,
  Continue, ExprStmt now compiled to bytecode (were silently dropped).
- Compiler: function-level scoping for LetBinding — `let` inside blocks overwrites
  outer variable, matching interpreter semantics (per p30_scope_let).
- Compiler: Expr::List now emits MakeList(count) (was broken — pushed Float(len)).
- VM: implemented MakeList, ListLen, Pop, StartsWith in both run() and
  execute_code() (were unimplemented!/silently skipped).
- VM: is_truthy() now handles Value::Bool correctly (Bool(true) was always false).
- Crosscheck TW vs VM baseline raised from 37/58 to 45/58 (8 cases closed).
  ADR-0075 documents all 21 remaining divergences.

### Documentation
- README: "Three Execution Backends" → "Two Execution Backends". JIT declared
  experimental (scaffold only, see ADR-0073). Cranelift removed from Prior Art.
- ADR-0075: full list of 21 TW vs VM divergences with categories and root causes.
- ADR-0076: performance baseline benchmarks (parser 178µs, interpreter 272µs,
  compiler 218µs, VM 36µs — VM 7.5× faster).

### Надёжность
- Парсер возвращает Result<_, ParseError> вместо аварийного завершения.
  27 вызовов std::process::abort() убраны, ошибка разбора теперь даёт
  диагностику с позицией line:col и код возврата 1 (ADR-0070)
- Golden test runner собирает ВСЕ failures перед panic — сломанные примеры
  не маскируют последующие тесты (Блок 2)
- p31_* error contracts покрыты автоматическими тестами (Блок 2)
- dag_demo.mlog исправлен: Demo() → Demo(input: String) (arity mismatch)

### Диагностика
- Триаж 92 integration test failures: 8 категорий (Блок 3, ADR-0071).
  219/311 integration tests pass. Ключевые группы: missing builtins (Phase 23),
  BUILTIN_REGISTRY gaps (8 Telegram/Voice entries), VM unimplemented (5 instructions),
  server-dependent (11 tests), immutable variable (4 tests).

## [0.12.0] - 2026-07-30

**Production hardening (наряды №29 и №30).**

### Безопасность
- .env вычищен из истории git и из всех веток
- HMAC-ключ сессий читается из METALOGOS_HMAC_KEY (раньше генерировался
  при каждом старте — сессии слетали при рестарте)
- CSRF-токены получили TTL 15 минут и фоновую очистку (раньше росли без границ)
- SECRET_LEAK: обнаружение секрета в теле http_post по позиции аргумента
  (ADR-0064) — заголовки остаются штатной авторизацией
- unsafe-блоков: 5 -> 1 (остался только Cranelift JIT, задокументирован)

### Надёжность
- Сессии, CSRF и rate limits переведены на DashMap
- Конкурентная обработка запросов: вызовы интерпретатора обёрнуты в
  tokio::task::block_in_place, лок планировщика сокращён (ADR-0067)
- Граф памяти переведён на StableDiGraph: удаление узла больше не портит
  индексы остальных (ADR-0066)
- Типизированные ошибки: RuntimeError через thiserror, хелпер lock_or_err

### Язык
- +slice(list, start, end) — срез списка, семантика зеркалит substring (ADR-0069)
- db_execute принимает необязательный список параметров — паритет с query()
  (ADR-0068). Склейка SQL больше не единственный способ
- Семантика зафиксирована golden-контрактами: let во вложенном блоке
  присваивает внешней переменной; присваивание требует let mut;
  kv_get на отсутствующем ключе возвращает пустую строку

### Тесты и CI
- Unit-тесты: 233 -> 373
- GitHub Actions: блокирующие test-lib и fmt, advisory test-integration и clippy
- Устранена гонка env-переменных в параллельных тестах llm.rs
- Cargo.lock взят под контроль версий, сборки воспроизводимы

### Сборка
- Dockerfile: rust 1.85, запуск от непривилегированного пользователя

### Известные ограничения
- 92 из 310 интеграционных тестов красные (накопленный долг, триаж — наряд №31)
- 191 clippy-предупреждение (джоб advisory)
- Fluid Types и confidence propagation не покрыты тестами
- BUILTIN_REGISTRY и Builtins::new() рассинхронизированы: 67 вызываемых
  функций отсутствуют в реестре, 44 записи реестра не имеют обработчика

## [0.11.0] — 2026-07-23

**Lifecycle hooks + YAML config (Наряд O-2).**

Расширение lifecycle hooks с 2 до 5 точек и поддержка YAML в config_load. Концепции вдохновлены [obsidian-mind](https://github.com/breferrari/obsidian-mind) (TypeScript, 3.5k★, MIT — код НЕ копировался, только архитектурные концепции).

### Lifecycle hooks (2 → 5)

- `hook on_session_start { ... }` — срабатывает один раз в начале `run()`, после регистрации всех деклараций.
- `hook on_write { ... }` — срабатывает перед каждым мутирующим билтином (mem_set, mtree_store, db_execute, write_file, append_file). Переменные: `target` (String), `args` (List).
- `hook on_session_end { ... }` — срабатывает один раз в конце `run()`.
- Существующие `before_pattern` / `after_pattern` без изменений (ADR-0045).

### config_load — поддержка YAML

- `config_load(path)` теперь автоматически определяет формат по расширению: `.yaml`/`.yml` → YAML, иначе → JSON.

### Новые зависимости

- `serde_yaml = "0.9"` — парсинг YAML конфигов.

### Изменённые файлы

- `src/grammar.pest` — 3 новых токена (on_session_start, on_write, on_session_end), расширен hook_kind, step_ident negative lookahead
- `src/ast.rs` — HookPhase: 2 → 5 вариантов (OnSessionStart, OnWrite, OnSessionEnd)
- `src/parser.rs` — parse_hook_decl: обработка 5 точек
- `src/interpreter.rs` — 3 новых поля, two-phase run(), fire_on_write_hooks() в 3 точках вызова
- `src/builtins.rs` — config_load: YAML поддержка + yaml_to_json_value() helper
- `Cargo.toml` — версия 0.11.0, serde_yaml
- `docs/adr/0064-obsidian-mind-lifecycle-hooks.md` — АДР
- `docs/adr/0065-config-load-yaml.md` — АДР
- `examples/hooks_lifecycle.mlog` — демо всех 5 lifecycle hooks

## [0.10.0] — 2026-07-23

**Vault/memory builtins inspired by [obsidian-mind](https://github.com/breferrari/obsidian-mind) (MIT — код НЕ копировался, только архитектурные концепции).**

### Новые builtins (3)

**Семантический поиск:**

- `semantic_search(query, documents, top_k)` — семантический поиск по списку документов. Возвращает список `SearchResult{index, text, score}`. Использует EmbeddingManager: OpenAI text-embedding-3-small если `METALOGOS_EMBEDDING_API_KEY` задан, иначе TF-IDF fallback. Вдохновлён QMD semantic search из obsidian-mind.

**Конфигурация и валидация:**

- `config_load(path)` — загрузка JSON-файла конфигурации в struct. Имя типа берётся из имени файла (stem). Вдохновлён vault-manifest.json — coordination point pattern из obsidian-mind.
- `vault_validate(config, required_fields)` — проверка, что struct содержит все указанные обязательные поля. Возвращает `ValidationResult{valid, missing}`. Вдохновлён frontmatter_required из obsidian-mind.

### Изменённые файлы

- `src/builtins.rs` — 3 новых builtin (semantic_search, config_load, vault_validate), импорт EmbeddingManager, BUILTIN_REGISTRY entries

## [0.9.6] — 2026-07-23

**Narad ML-1: host key in mlogserver + json_get NULL fix.**

### Bug fixes

- **mlogserver `host:` key** (баг №2): блок `mlogserver` теперь принимает опциональный ключ `host: "127.0.0.1"` для биндинга на указанный адрес вместо жёстко зашитого `0.0.0.0`. Закрывает гонку портов на Render. Обратная совместимость: отсутствие `host:` → дефолт `"0.0.0.0"`.
- **`json_get` SQL NULL** (баг №1): `json_get(row, key, default)` теперь возвращает `default`, когда значение поля — SQL NULL (`Value::Unit`). Раньше возвращал `Unit`, что вызывало `type mismatch` при конкатенации `String + Unit`. Двухаргументная форма (без default) не изменена.

### Изменённые файлы

- `src/grammar.pest` — правило `mlogserver_host`, `"host"` в `step_ident` исключениях
- `src/ast.rs` — поле `host: Option<String>` в `MlogServerDecl`
- `src/parser.rs` — разбор `host` в `parse_mlogserver_decl`
- `src/server.rs` — биндинг на `config.host` с fallback `"0.0.0.0"`
- `src/builtins.rs` — проверка `Value::Unit` в 3-аргументной ветке `json_get`

## [0.9.5] — 2026-07-21

**OpenPlanter-inspired: Agent utility builtins (ADR-0063).**

Концепции заимствованы из https://github.com/ShinMegamiBoson/OpenPlanter (MIT — код НЕ копировался, только идеи).

### Новые зависимости

- `strsim = "0.11"` — Jaro-Winkler нечёткое сравнение строк
- `crc32fast = "1.4"` — быстрая CRC32-хеширование

### Новые builtins (8)

**Нечёткое сравнение (fuzzy matching):**

- `fuzzy_match(a, b)` — Jaro-Winkler сходство двух строк (0.0..1.0). Основано на OpenPlanter `wiki/matching.rs::NameRegistry`.
- `fuzzy_find_best(query, candidates)` — лучший матч из списка кандидатов → `FuzzyMatch{index, candidate, score}`.

**Контент-верифицированное редактирование (hashlines):**

- `hashline_read(text)` — аннотировать строки 2-символьным CRC32-хешем: `N:HH|content`. Предотвращает LLM-редактирование устаревшего контента.
- `hashline_edit(text, edits)` — редактирование с верификацией хешей. 3 операции: `set_line`, `replace_lines`, `insert_after`. Ошибка при несовпадении хеша.

**Утилиты агента:**

- `compact_list(items, keep_first, keep_last)` — контекстная компактификация: защита головных/хвостовых элементов, среда схлопывается в `Compacted{compacted: true, removed_count: N}`. Аналог OpenPlanter `compact_messages()`.
- `budget_check(step, total_steps)` — осведомлённость о бюджете → `BudgetStatus{step, total, remaining, pct_remaining, level}`. Уровни: "ok" (≥50%), "warning" (≥25%), "critical" (<25%).
- `replay_snapshot(data)` — дельта-логирование: seq 0 = полный снапшот → `ReplaySnapshot{seq, count, snapshot}`. Аналог OpenPlanter `ReplayLogger`.
- `policy_check(command)` — проверка безопасности shell-команды → `PolicyResult{allowed, reason}`. Блокирует heredoc (`<<`) и интерактивные программы (vim, nano, less и т.д.).

### Изменённые файлы

- `src/builtins.rs` — 8 новых builtin'ов + 2 helper'а + 20 тестов (~590 строк).
- `Cargo.toml` — версия 0.9.5, зависимости `strsim`, `crc32fast`.
- `docs/adr/0063-openplanter-agent-utilities.md` — ADR.
- `examples/openplanter_demo.mlog` — демонстрация всех 8 builtin'ов.

## [0.9.4] — 2026-07-16

**AgentSkillOS-inspired: Recipe system + DAG orchestration builtins (ADR-0062).**

Концепции заимствованы из https://github.com/ynulihao/AgentSkillOS (MIT — код НЕ копировался, только идеи).

### Новые builtins (5)

- `recipe_save(name, description, skills, plan)` — построить рецепт (struct с key + recipe), для сохранения через `kv_set`. Возвращает `{key: "__recipe:<name>", recipe: {...}}`.
- `recipe_search(query)` — placeholder для семантического поиска рецептов. Возвращает пустой список (требует embedding infrastructure).
- `recipe_list()` — placeholder для списка рецептов. Возвращает пустой список (требует KV access из builtin context).
- `dag_phases(dag)` — извлечь параллельные фазы выполнения из DAG. Вход: список `{id, depends_on}`. Выход: список фаз (списков ID). Kahn's algorithm + детекция циклов.
- `topo_sort(dag)` — топологическая сортировка DAG. Тот же формат входа. Выход: плоский список ID в порядке зависимостей.

### Изменённые файлы

- `src/builtins.rs` — 5 новых builtin'ов + 13 тестов (~300 строк).
- `docs/adr/0062-agentskillos-recipe-dag.md` — ADR с описанием архитектуры.
- `examples/dag_demo.mlog` + `.expected` — golden test для dag_phases/topo_sort.

### Ограничения

- `recipe_search`/`recipe_list` — placeholders, полная реализация требует доступа к KV-хранилищу из builtin context.
- Нет семантического поиска рецептов (требует embeddings).

## [0.9.3] — 2026-07-12

**sqz-inspired builtins и declaration (P1+P2+P3).**

Концепции заимствованы из https://github.com/ojuschugh1/sqz (ELv2 — код НЕ копировался, только идеи).

### P1 — Строковые/списковые утилиты (10 builtin'ов)

- `squeeze(s, chars)` — схлопнуть идентичные соседние символы (аналог Ruby String#squeeze).
- `dedup(list)` — удалить дубликаты, сохраняя порядок первого вхождения. Сравнение через JSON для сложных типов.
- `condense(list)` — схлопнуть идентичные соседние строки с подсчётом повторов (формат: элемент, "×N").
- `strip(s, chars)` — удалить символы с обоих концов строки (аналог Python str.strip).
- `chomp(s)` — удалить один trailing newline (\n или \r\n, аналог Ruby String#chomp).
- `repeat(s, n)` — повторить строку n раз. Проверка: n >= 0, целый.
- `pad_left(s, n, fill)` / `pad_right(s, n, fill)` — дополнить строку символом fill до длины n.
- `lines(s)` — разбить на список строк по \n, без trailing пустого элемента.
- `words(s)` — разбить на список слов по whitespace.

### P2 — TOON encoding + content-addressed refs

- `toon_encode(value)` — кодировать любое значение в TOON (Token-Optimized Object Notation). Префикс `TOON:`, ключи без кавычек, non-ASCII → `\u{XXXX}`. Lossless.
- `toon_decode(s)` — декодировать TOON обратно в Value. Recursive descent parser. Проверка префикса, валидация JSON-like синтаксиса.
- `ref(content)` — SHA-256 хэш, сохранить в KV-хранилище (`__ref:HASH`), вернуть hex-строку (64 символа). Idempotent (INSERT OR IGNORE).
- `deref(hash)` — восстановить содержимое по хэшу. Валидация формата (64 hex символов), ошибка если не найден.

### P3 — Token awareness

- `token_count(text)` — оценка количества токенов: кириллица chars/2, латиница chars/4, порог 50%.
- `context_budget` — новое объявление верхнего уровня: `context_budget { pattern: "name", limit: 4096 }`. Хранит токенный бюджет для learnable pattern'ов в `Interpreter.context_budgets` HashMap.

### Изменённые файлы

- `src/builtins.rs` — 15 новых функций + 52 теста.
- `src/grammar.pest` — правило `context_budget_decl`.
- `src/ast.rs` — `ContextBudgetDecl` struct + `Declaration::ContextBudget` variant.
- `src/parser.rs` — `parse_context_budget_decl`.
- `src/interpreter.rs` — обработка `ContextBudget` в `run()` и `clone_definitions_into()`, поле `context_budgets`.
- `src/compiler.rs` — `ContextBudget` в catch-all arms (pass1 + pass2).

### Тесты

- 52 новых теста в `mod tests_sqz_builtins`. Все pass.
- Итого: 196 passed, 3 failed (pre-existing), 3 ignored.

## [0.9.2] — 2026-07-12

**Заплатка: исправление 5 ошибок компиляции E0004 (non-exhaustive patterns) после Problem A/B/C/D/E.**

- `compiler.rs`: `BinOp::And`/`Or` — добавлена явная ветка с ошибкой компиляции (short-circuit evaluation не реализован в VM bytecode, требуется tree-walking interpreter).
- `vm.rs` main loop: `Instruction::MakeList`, `ListLen`, `Pop`, `StartsWith` — добавлены ветки `unimplemented!` с поясняющим сообщением (VM bytecode support отложен).
- `vm.rs` `eval_branch_condition`: `ConditionOp::Ne` — реализована семантика `!=` (по аналогии с `Eq`).
- `vm.rs` `eval_rule_condition`: `&ConditionOp::Ne` — реализована семантика `!=` (по аналогии с `Eq`).
- `vm.rs` `eval_binop` Float branch: `BinOp::And`/`Or` — добавлена ветка, возвращающая runtime-ошибку (булева логика некорректна для Float operands).

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

### Problem C — Schema-as-code (ADR-0060)

- Новая декларация `schema name { table T { ... } }` — DECLARE таблиц прямо в .mlog файлах
- Auto-migration при старте: `CREATE TABLE IF NOT EXISTS` (additive-only, никогда не drop/alter)
- Поддерживаемые типы: Int, Float, String, Text, Bool, DateTime
- Модификаторы: primary_key, auto_increment, nullable, references(table.field)
- Дефолты: default("value"), default(now())
- Интеграционные тесты: schema + db_insert + query round-trip, additive migration
- **Ограничение**: schema DDL и db_insert работают только в tree-walking режиме (требуют SQLite connection). VM/JIT путь отложен.

### Problem A — Tiered Skill Index (ADR-0058)

- Новая декларация `skill_index name { tier N always [...] | tier N when_matches [...] budget: N tokens truncation: mode }`
- AST: SkillIndexDecl, SkillTier, SkillTriggerRule, TruncationMode
- Grammar: 12 new PEG rules (skill_index_decl, skill_tier, tier_always_list, tier_matches_list, etc.)
- Parser: 2 new parse functions
- Interpreter: `skill_indices` HashMap, `resolve_skill_index` + `fit_to_budget` builtins
- `fit_to_budget` MVP: pass-through (полная реализация с file I/O отложена)
- 5 интеграционных тестов: базовая загрузка, trigger matching, budget/truncation, error handling, 3 tiers
- STOP Trigger #4 задокументирован: бюджет per-model, не глобальная константа (известное ограничение MVP)

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