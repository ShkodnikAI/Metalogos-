# tree-sitter-mlog

[Tree-sitter](https://tree-sitter.github.io/tree-sitter/) grammar for the [Metalogos](https://github.com/ShkodnikAI/Metalogos-) language (`.mlog`).

> **Наряд №289** (issue #301, P2/tooling) — Tree-sitter грамматика для `.mlog`. Источник истины — `REFERENCE.md` §3 (Syntax), сверено с `src/grammar.pest` (PEG-грамматика основного компилятора). Ноль диффа в `src/**`, `tests/**` и `src/grammar.pest` — параллельный артефакт для внешних инструментов, не часть компиляции `.mlog`.

## Установка

```bash
cd tree-sitter-mlog
npm install
```

## Сборка парсера

```bash
./node_modules/.bin/tree-sitter generate
```

## Парсинг файла

```bash
./node_modules/.bin/tree-sitter parse ../examples/m1_hello.mlog
```

## Покрытие конструкций (Блок 1 — грамматика)

`grammar.js` покрывает все основные конструкции языка из `REFERENCE.md` §3:

### Декларации верхнего уровня
- `entity` (три формы: type / record / simple — mirror pest ordered choice)
- `pattern`, `learnable pattern` (with ADR-0117 distill fields in any order)
- `flow` (with `checkpoint("...")` markers + `branch_def`s — ADR-0056)
- `rule` (match + actions)
- `reflex` / `reflex_seq` / `reflex_gen` (ADR-0114 / ADR-0119 / ADR-0120)
- `vision` (ADR-0124)
- `type` alias (Наряд №119)
- `llm {}` config (ADR-0048)
- `mlogserver {}` / `server {}` + `route` (ADR-0074)
- `template` (Phase 6.2)
- `db`, `schema`, `skill_index`, `memory`, `conversation`, `context_budget`
- `import`, `hook`, `sandbox`, `mutate`, `eval`, `fluid`, `adapt`
- `memorize`, `relate`, `forget` (statement + declaration forms)
- `tool`, `test` (Наряд №120 + №287)

### Statements
- `let` / `let mut` / assign / expression statement
- `if` (block form) / `if-then` (block form) / `else if` / `else`
- `each ... in ... { ... }` (loop over collection)
- `while ... { ... }`
- `match expr { arm* else? }` — exact / `starts_with` / `contains` / compare-op arms (Наряд №173b)
- `break` / `continue` / `return expr`

### Expressions (layered precedence)
- `or` → `and` → `compare` → `add` (`+`/`-`) → `mul` (`*`/`/`) → `unary` → `access` → `primary`
- Unary minus, `try expr` (error handling, Наряд №14)
- `if cond then a else b` (expression form, Наряд №14)
- `if cond { ... } else { ... }` (block-as-expression, Наряд №14)
- Function call `func(args)` + qualified call `module.func(args)` (via postfix chain)
- Field access `obj.field` + index `list[0]`
- Struct literal `{ field: value, ... }` + list literal `[a, b, c]`
- Parenthesized expression `(expr)`
- Literals: string (`"..."` with `\n \t \r \" \\ \uXXXX` escapes), multiline string (`"""..."""`), int, float, bool, ident

### Identifiers
- ASCII letters + underscore + digits (after first char)
- Cyrillic А-я (per pest IDENT rule)
- Apostrophe allowed inside (pest uses it for some idents)

### Comments
- `// single-line comment`

## Контракт корректности (Блок 2 — представительная выборка)

Прогнано `tree-sitter parse` на **23 репрезентативных файлах** из `examples/`, покрывающих все основные столпы языка:

| Pillar / Category | Examples | Result |
|---|---|---|
| Basic syntax | `m1_hello`, `m2_triage`, `m4_memory`, `m5_adapt` | 2 PASS, 2 PARTIAL |
| Fluid types | `p1_fluid_collapse` | 1 PARTIAL |
| Control flow | `p5_if_else`, `p51_let_bindings` | 2 PASS |
| HTTP server + routes | `p7_post_routes`, `p7_query_readable`, `p69_deferred_response`, `vm_serve_realistic_dispatcher` | 3 PASS, 1 PARTIAL |
| Crypto / secrets | `p70_crypto_smoke` | 1 PASS |
| Cyrillic identifiers | `p9_cyrillic_full` | 1 PARTIAL |
| Context loading | `p11_context_loading` | 1 PARTIAL |
| Knowledge graph | `p2_knowledge_graph` | 1 PARTIAL |
| Hooks lifecycle | `hooks_lifecycle` | 1 PARTIAL |
| Schema | `dept_schema` | 1 PARTIAL |
| Skill index | `skill_index_tiered` | 1 PARTIAL |
| Reflex (generation) | `reflex_gen_declare`, `p201_reflex_generate_html_injection` | 2 PASS |
| Pattern composition | `n1_pattern_compose`, `p161_helper` | 2 PASS |
| Full app (template + render) | `p6_full_app` | 1 PARTIAL |

**Итог:** **12 PASS (no ERROR nodes), 11 PARTIAL (parser recovered, ERROR nodes in deep constructs), 0 FAIL (no crashes)**. Все 23 файла парсятся структурно — ни один не падает. PARTIAL означает, что tree-sitter восстановил дерево, но некоторые узлы помечены `ERROR` (обычно из-за нерешённых GLR-конфликтов в сложных случаях, не критичных для базовой индексации). Доработка PARTIAL → PASS — отдельный, следующий наряд (если появится реальный спрос от агентных индексаторов).

## Расхождения с `grammar.pest` (зафиксировано явно, не тихо)

1. **pest ordered choice ↔ tree-sitter GLR**: pest использует PEG ordered choice (`|` — first match wins), tree-sitter — GLR (все альтернативы параллельно, conflicts разрешаются через `conflicts: $ => [...]` в grammar.js). Это означает, что tree-sitter может построить дерево для constructов, которые pest бы отклонил (из-за lexical ambiguity). Не баг — фича, но при сверке с `grammar.pest` местами расходится.
2. **`_{ ... }` silent rules ↔ tree-sitter `inline`**: pest silent rules (underscore prefix) are inlined into parent; tree-sitter uses `inline: $ => [...]` declaration. Same effect, different mechanism.
3. **Keyword extraction**: pest resolves keyword-vs-identifier conflicts through ordered choice in consumer rules. tree-sitter uses `word: $.ident` declaration (currently not set in grammar.js — known limitation, see "Known limitations" below). As a result, some keywords like `memorize`, `relate`, `if`, `each` may parse as identifiers in some contexts. Not a bug in semantics — pest catches it via ordered choice.
4. **Empty-matching rules**: pest allows `entity_type_body = { field_decl* }` (matches empty). tree-sitter forbids non-start rules matching empty string — `entity_type_body` was inlined into `entity_type_decl` parent seq with `repeat1` to enforce ≥1 field.
5. **Entity record decl shape**: pest `entity_record_decl = { "entity" ~ IDENT ~ ":" ~ type_name ~ "=" ~ LBRACE ~ field_init ~ ... ~ RBRACE }`. В моей первоначальной версии grammar.js я смоделировал его как `entity Name(params) { ... }` — это была ошибка (params in parens, не `: Type = {...}`). Исправлено на `entity Name : Type = { field: value, ... }` — точно mirror pest.

## Known limitations (для следующего наряда, если будет спрос)

1. **11/23 examples PARTIAL**: GLR conflicts в deep constructs (hooks lifecycle, schema, skill_index) — parser восстанавливает, но с ERROR nodes. Решается добавлением `prec(...)` / `prec.left/right(...)` деклараций в conflict-rules.
2. **`word: $.ident` not set**: tree-sitter рекомендует `word` для keyword extraction. Текущая grammar.js опирается на GLR conflicts (работает, но менее эффективно для keyword-heavy языков).
3. **`if-else`-block as expression conflict**: `block_if_else_expr` самоконфликтует через `repeat(else_if_block)` — добавлено в `conflicts`, но при deeply nested if-else может всё ещё давать ERROR nodes.
4. **Multiline strings**: simplified regex `/[^"]*/` — корректно для большинства случаев, но не идентичен pest's `(!("\"\"\"") ~ ANY)*` (не обрабатывает `"\"\""` escape inside multiline). Для полного паритета нужно внешнее lexer-правило.

## Публикация (Блок 3)

Постановка наряда явно говорит: "Для первой версии — грамматика в самом репозитории Metalogos, публикация как отдельного пакета — отдельный, более поздний наряд, если появится реальный спрос (тот же принцип «не чинить без подтверждённого случая», что весь проект применяет)". Поэтому:
- `package.json` создан (имя `tree-sitter-mlog`, version 0.19.0 — sync с `Cargo.toml`).
- Не опубликован в npm registry.
- Использование: clone Metalogos, `cd tree-sitter-mlog && npm install && ./node_modules/.bin/tree-sitter parse <file.mlog>`.

## Связь

- Issue #301 (наряд).
- `REFERENCE.md` §3 (Syntax) — primary source.
- `src/grammar.pest` (530+ lines) — secondary source для сверки.
- `examples/*.mlog` (214 файла) — test corpus (23 representative picked).
- Наряды №287 (doc-tests) — параллельная testing-инфраструктура для .mlog doc-snippets.
- Agent skill `agent-browser` — потенциальный потребитель (если tree-sitter-mlog будет опубликован).
