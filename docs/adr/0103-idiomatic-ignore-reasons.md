# ADR 0103 — Idiomatic `#[ignore]` Reasons (Наряд №73 Block 3)

**Status**: Accepted
**Date**: 2026-08-14
**Milestone**: Наряд №73 — закрытие блока 3 (audit `#[ignore]` without reason)

## Context

Наряд №73 Block 3 требовал провести аудит всех `#[ignore]`-маркеров в тестах Metalogos без причины. Первичная проверка показала, что в коде **68** `#[ignore]`-атрибутов, распределённых по двум формам:

| Форма | Кол-во | Поведение cargo |
|-------|--------|-----------------|
| `#[ignore = "reason"]` (идиоматичная) | 2 | Reason виден в `cargo test` выводе, в `--list`, в CI-логах |
| `#[ignore] // reason` (неидиоматичная) | 66 | Cargo не показывал reason — выводил просто `... ignored`. Чтобы узнать причину, приходилось делать `git blame` |

Ни одного **полностью "голого"** `#[ignore]` без какого-либо объяснения в коде найдено не было — все 66 неидиоматичных случаев сопровождались `//` комментарием, объясняющим причину. Однако этот комментарий был невидим для инструментов (cargo, CI-агрегаторов, IDE-плагинов).

Пример **до**:

```rust
#[test]
#[ignore] // TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)
fn jit_p5_golden_example() { ... }
```

Вывод cargo: `test jit_p5_golden_example ... ignored` — без объяснения.

Пример **после**:

```rust
#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
fn jit_p5_golden_example() { ... }
```

Вывод cargo: `test jit_p5_golden_example ... ignored, TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)` — причина видна в логах и в CI.

## Decision

Принять **идиоматичную форму `#[ignore = "reason"]`** как единственно допустимую в кодовой базе Metalogos. Все `#[ignore] // reason` конвертированы в `#[ignore = "reason"]` с сохранением исходной формулировки причины.

Конвертация выполнена скриптом `/home/z/my-project/scripts/convert_ignore_to_idiomatic.py` (66 строк в 10 файлах). Скрипт идемпотентен: повторный запуск не изменяет уже сконвертированные строки.

### Категории причин (каталог)

Все 68 `#[ignore]` сгруппированы по 11 категориям:

| Категория | Кол-во | Файл(ы) | Что нужно для un-ignore |
|-----------|--------|---------|-------------------------|
| Legacy syntax (top-level stmts) | 21 | `tests/phase23_v084_v087_tests.rs` | Переписать тесты под `pattern`/`flow` синтаксис (top-level statements удалён в фазе 23) |
| Flaky in sandboxed env | 9 | `tests/memory_persist_e2e.rs` | Изолировать temp-dir для каждого теста |
| VM compiler feature gap | 7 | `tests/phase18_compiler_statements.rs`, `tests/phase19_22_constraints.rs` | Реализовать в VM-компиляторе: match with string literal arms, match contains, match with compare ops, match with starts_with arms, Ne compare in rules, process-style declarations |
| Semantic/template feature gap | 7 | `tests/phase19_22_constraints.rs`, `tests/template_integration.rs` | Реализовать: opaque Html type constraint, undefined variable detection, Interpreter::render_template, unknown template detection |
| JIT not integrated | 6 | `tests/jit_golden.rs` | Интегрировать JIT (ADR-0073) — `Vm::with_jit` пока недоступен |
| Needs HTTP server setup | 6 | `tests/server_json_body.rs` | Настроить запуск webhook-сервера в test setup |
| Parallel-test race (needs #[serial]) | 4 | `tests/llm_cache_contract.rs` | Добавить `#[serial_test::serial]` (как в naryad-71 fix) |
| External LLM API (manual run) | 3 | `src/llm.rs` | Не подлежит un-ignore — это ручные тесты против OpenAI/Anthropic/Ollama API. Запуск: `METALOGOS_MOCK_LLM=false METALOGOS_LLM_PROVIDER=openai METALOGOS_API_KEY=sk-xxx cargo test -- --ignored` |
| Secret type removed | 2 | `tests/phase19_22_constraints.rs` | Реализовать Secret type повторно (env() сейчас возвращает String, не Secret) |
| Semantic checker gap | 2 | `tests/phase19_22_constraints.rs` | Реализовать undefined variable detection в semantic checker |
| Self-hosting lexer | 1 | `tests/self_host_lexer.rs` | Решить product-decision (ADR-0023): Option A — продолжить, Option B — отменить, Option C — оставить как есть |

**Итого: 68 `#[ignore]`, из них:**
- **3** — permanent (manual LLM API tests) — не подлежат un-ignore
- **1** — ждёт product-decision по self-hosting (ADR-0023)
- **64** — waiting for implementation work (перечислено в таблице выше)

## Consequences

### Положительные

1. **CI-логи самодокументируемы**: причина ignore видна прямо в выводе `cargo test`, без `git blame`.
2. **Поиск по причинам**: можно делать `grep -r 'ignore = "JIT' tests/` для поиска всех JIT-блокированных тестов.
3. **Готовая roadmap для un-ignore**: каталог в этом ADR — это по сути todo-list для будущих нарядов по закрытию технического долга.
4. **Идемпотентность**: скрипт `convert_ignore_to_idiomatic.py` можно запускать в CI как lint (пока не добавлен — будущая работа).

### Отрицательные

1. **Длинные строки**: некоторые `#[ignore = "..."]` теперь длиннее 100 символов. `cargo fmt` это допускает (атрибуты не форматируются по ширине строки), но визуально шумно. Альтернатива (многострочный `#[ignore = "..."]`) не поддерживается Rust — атрибут должен быть на одной строке.
2. **Конвертация не решила underlying-проблему**: тесты всё ещё ignored. Это был audit + нормализация формы, а не fix тестов. Каждый `#[ignore]` остался в коде ровно с тем же поведением.

### Neutral

1. **Формулировки причин не редактировались**: оставлены как были в `//`-комментариях. Некоторые из них содержат `TODO:` префикс — это сохранено.
2. **2 уже-идиоматичных `#[ignore]` не тронуты**: в `tests/llm_cache_contract.rs:137` и `:173` — они уже были в правильной форме.

## Future work

1. ~~**CI guard**: добавить в `.github/workflows/ci.yml` шаг, который падает, если в PR добавлен `#[ignore]` без `= "..."`. Простейшая реализация — `grep -rE '#\[ignore\s*\]' tests/ src/ | grep -v '#'` должен быть пустым.~~
   **Done (Наряд №73 Block 3, 2026-08-14).** Реализовано как Rust-интеграционный тест `tests/ignore_reasons_lint.rs` (вместо shell-шага в CI — это идиоматичнее для codebase, где все invariant-чеки уже сделаны как cargo tests, см. `registry_sync_check.rs`). Тест сканирует все `.rs` файлы в `tests/`, `src/`, `examples/`, `self-host/`, `benches/`, и падает с детальным списком violations если находит bare `#[ignore]`. Пропускает комментарии и строковые литералы чтобы не ловить упоминания `#[ignore]` в docs/тест-данных.
2. **Un-ignore по категориям**: будущие наряды могут закрывать категории целиком. Например, "Наряд №N: closed parallel-test race category" — добавить `#[serial_test::serial]` к 4 тестам в `llm_cache_contract.rs` и снять `#[ignore]`.
   - **Подкатегория 'Parallel-test race' закрыта (Наряд №75, 2026-08-14).** Все 4 теста в `tests/llm_cache_contract.rs` теперь под `#[serial_test::serial]`, `#[ignore]` снят. См. PR `naryad-75-llm-cache-serial`.
3. **Quarterly audit**: раз в квартал перепроверять, не потерял ли какой-то `#[ignore]` актуальность (например, если JIT интегрировали — снять все 6 игноров из `jit_golden.rs`).
4. ~~**Product decision по self-hosting**: владелец должен выбрать Option A/B/C (см. ADR-0023) — это закроет последнюю категорию.~~
   **Done (Наряд №73 Block 3, 2026-08-14).** Decision: Option C (defer). См. обновлённый ADR-0023 — секция "Owner Decision". Тест `self_host_lexer_tokenizes_m1_hello` остаётся под `#[ignore]` с причиной, ссылающейся на ADR-0023 Option C. Категория в каталоге выше закрыта.
