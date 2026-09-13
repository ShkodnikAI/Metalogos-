# ADR-0142: Error protocol — structural errors through try-extended semantics (candidate B)

**Status:** Accepted (решение владельца 2026-09-14 — GO на research + ADR; приоритет кандидат (б) — расширение try-семантики; финальный выбор кандидата — после ADR)
**Date:** 2026-09-14
**Naryad:** #298 (issue #362, P1/adr — ERR_PROTOCOL)
**Amends:** ADR-0106 (Option/Result — not introduced, soft-failure remains the error model) — error-struct ≠ Option/Result типы; ADR-0106 не пересматривается, только аннотируется примечанием.
**Precedent:** Наряд №91 (TryEval — try expr → Unit on error), ADR-0131/0140 (stable diagnostic codes — UPPER_SNAKE_CASE, no-reuse rule), ADR-0106 (soft-failure model).

## Context

Metalogos использует громкие runtime-ошибки (`Result<Value, String>`) — осознанный дефолт (ADR-0106). `try` (Наряд №91) возвращает `Unit` при ошибке, теряя информацию об ошибке. Для больших agentic-приложений (Fosved: dept-handlers, chain patterns) это реальная боль — нет способа структурированно обработать ошибку (код, сообщение, позиция) без теряемой информации.

Option/Result-типы ЯВНО отвергнуты (ADR-0106, Rejected). Наряд — НЕ «добавить Result» (запрещено без supersede), а research + ADR: стандартизированный error-protocol через расширение существующих конструкций (`try` №91).

Research-док: `docs/research/error-protocol-prior-art.md` — prior art (Go error values, Lua pcall, Erlang tagged tuples, Elm Result), текущее состояние Metalogos, 3 кандидата, сценарии Fosved, оценка breaking-поверхности.

## Decision (3 кандидата, приоритет (б))

### Кандидат (а) — стандартный error-struct + `?`-оператор раннего возврата
```mlog
let r = call_llm("...") ?  # early return on error
```
- Новый оператор `?` (грамматика).
- Новый error-struct shape: `{ code: String, message: String, span: Option<Struct> }`.
- Изменение сигнатур builtins — возврат Struct вместо String при ошибке.
- **Breaking surface**: грамматика + builtins + старый код.
- **Отвергнут** владельцем как приоритет (но не исключён — финальный выбор после ADR).

### Кандидат (б) — расширение try-семантики до структурных ошибок (ПРИОРИТЕТ)
```mlog
let r = try call_llm("...")
# r = Struct { ok: Bool, value: String, error: Option<Struct> }
if r.ok then { respond("200", r.value) } else { respond("500", r.error.message) }
```
- `try` возвращает `Struct { ok: Bool, value: Value, error: Option<Struct{code, message}> }` вместо `Unit` при ошибке.
- Грамматика — БЕЗ ИЗМЕНЕНИЙ (try уже в языке).
- Builtins — БЕЗ ИЗМЕНЕНИЙ.
- Старый код: `if try_result == Unit` — ломается (Unit ≠ Struct). Обратная несовместимость.
- **Совместимость**: можно ввести `try` как есть (возвращает Unit) и `try_struct` (или `try?`) как новый вариант (возвращает Struct). Но это = кандидат (а).
- **Альтернатива**: изменить `try` на возвращение Struct, с backward-compat: `if result == Unit` → `Unit` is NOT `Struct { ok: false }` → old code breaks. Migration: `if result.ok` replaces `if result == Unit`.
- **Breaking surface**: только старый код, который проверяет `try x == Unit`. Минимальный.
- **Синергия**: `code` field = ADR-0131/0140 stable codes (UPPER_SNAKE_CASE).
- **Усилия**: ~1 наряд для реализации (TryEval → TryEvalStruct opcode, VM dispatch, interpreter try-eval → Struct, tests).

### Кандидат (в) — статус-кво + документированный паттерн
- `try` остаётся как есть (Unit при ошибке).
- Error-handling pattern документирован.
- **Breaking surface**: ноль.
- **Отвергнут** владельцем (не решает проблему).

### D1. Выбранный кандидат: (б) — расширение try-семантики
- `try expr` → при ошибке возвращает `Struct { ok: false, value: Unit, error: Some(Struct{ code: "RUNTIME_ERROR", message: "..." }) }`; при успехе — `Struct { ok: true, value: <result>, error: None }`.
- `code` field — stable per ADR-0131/0140 convention (UPPER_SNAKE_CASE, no-reuse). Для runtime errors — новый namespace category `"runtime"` (vs existing `"security"`/`"semantic"`).
- `message` field — human-readable, may change freely (the code is the contract, not the prose — ADR-0131 principle).
- `span` field — optional, for source-position (future; not in v1).

### D2. Backward compatibility
- Старый код: `if try_result == Unit` → ломается (Unit ≠ Struct).
- Migration path: `if result.ok` (или `if not result.ok` для error-branch).
- ADR-0106 аннотируется: error-struct ≠ Option/Result типы; soft-failure model остаётся (empty string / Unit / false для optional paths); `try` с Struct — для critical-path error-handling, не замена soft-failure.

### D3. Реализация — ОТДЕЛЬНЫЙ наряд
- Этот ADR фиксирует решение; реализация (opcode TryEvalStruct, VM dispatch, interpreter, tests) — отдельный наряд после принятия ADR.
- Не реализовывать в этом наряде (контракт: research + ADR only).

## Consequences

- `try` перестаёт возвращать `Unit` при ошибке — возвращает `Struct`. Старый код, проверяющий `try x == Unit`, ломается. Migration: `if result.ok`.
- Error-struct с `code` field — синергия с ADR-0131/0140 stable codes. Коды для runtime errors — новые (namespace `"runtime"`), но follow ту же UPPER_SNAKE_CASE + no-reuse дисциплину.
- Soft-failure model (ADR-0106) остаётся для optional paths (`kv_get`/`recall`/`query_param` → `""`). `try` с Struct — для critical-path error-handling.
- ADR-0106 не отменяется, только аннотируется (error-struct ≠ Option/Result; soft-failure остаётся; `try` — extension, не замена).
- Реализация — отдельный наряд (~1 наряд по прецеденту №91).

## Addendum: ADR-0106 annotation

ADR-0106 §Decision (Option/Result — Rejected) остаётся в силе. Error-struct (кандидат (б)) — это не Option/Result тип; это:
- Расширение существующей `try`-механики (Наряд №91) до возврата структурированной информации об ошибке.
- Struct value (уже есть в языке), не новый type.
- Soft-failure model (empty string / Unit / false) остаётся для optional paths.
- Critical-path error-handling через `try` → Struct — отдельный паттерн, не замена soft-failure.
- ADR-0106 «soft-failure remains the error model» — верно для optional paths; для critical-path errors — `try` → Struct добавляет structured error-handling without introducing Result.
