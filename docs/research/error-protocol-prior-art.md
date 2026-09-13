# Error Protocol — Prior Art & Current State Inventory

> **Наряд №298** (issue #362, P1/adr — ERR_PROTOCOL). Источник: внешний аудит Metalogos 2026-09-13. Решение владельца 2026-09-14: GO на research + ADR; приоритет — кандидат (б) расширение try-семантики (№91).

## 1. Текущее состояние ошибочных путей Metalogos

### 1.1. Громкие runtime-ошибки (default)
Все runtime-ошибки — `Result<_, String>` (строка-сообщение), возвращаются громко:
- `Err("call_llm() failed: ...")` — стандартный паттерн.
- `Err("division by zero")` — арифметические.
- `Err("field 'x' not found on struct")` — доступа.
- Паттерн: каждая ошибка — String, не структурированная.

### 1.2. try-механика (Наряд №91, TryEval)
- `try expr` — возвращает `Unit` при ошибке, `Value` при успехе.
- Реализация: `Expr::Try { expr, span }` → compiled to `Instruction::TryEval(inner_code)`.
- VM dispatch (обоих loops): если inner_code возвращает `Err` → push `Value::Unit`; иначе push `Value`.
- Ограничение: `try` возвращает Unit при ошибке — **теряет информацию об ошибке** (код/сообщение/позиция). Нет способа узнать, какая именно ошибка произошла.

### 1.3. Soft-failure конвенции (ADR-0106)
- `respond("404", "not found")` — HTTP-уровневый soft-failure (не crash).
- `kv_get("missing")` → `""` — empty-string fallback.
- `recall("missing")` → `""` — soft-failure.
- `query_param("missing")` → `""` — soft-failure.
- Конвенция: builtins возвращают `Unit`/`""`/`false` при отсутствии, не `Err`.
- Громкие ошибки только для «настоящих» ошибок (тип-несовпадение, деление на ноль, network failure).

### 1.4. EOF/End markers
- `llm_stream_next` → `"__end__"` (Наряд №275, ADR-0137) — end-of-stream marker.
- `""` — keep-alive ping.
- Конвенция: special sentinel values для soft-EOF.

### 1.5. JSON-шейп диагностик (ADR-0131/0140)
- `mlog check --json` → массив `{code, message, span, severity}` (Наряд №255, ещё не реализован).
- `check_id` — UPPER_SNAKE_CASE stable codes (ADR-0131: `SECRET_LEAK`, `SQL_DYNAMIC`, etc.).
- ADR-0140 (Наряд №288): no-reuse rule + SSOT-registry discipline для диагностических кодов.
- Синергия: если error-struct будет иметь `{code: String, message: String}`, коды уже стабильны через ADR-0131/0140.

## 2. Prior Art — error protocols без Result

### 2.1. Go — error values (multiple return values)
```go
result, err := doSomething()
if err != nil {
    return err
}
```
- Error — отдельное value, не тип-контейнер.
- `error` interface (`Error() string`).
- **Минус для Metalogos**: multiple return values не в языке.

### 2.2. Lua — nil + error value (pcall)
```lua
local ok, err = pcall(function() ... end)
if not ok then ... end
```
- `pcall` — protected call, returns `(success, value_or_error)`.
- **Сходство с Metalogos**: `try` уже работает как pcall (возвращает Unit при ошибке).
- **Отличие**: Lua возвращает error-string, Metalogos — теряет её.

### 2.3. Erlang — tagged tuples
```erlang
{ok, Value} = doSomething(),
{error, Reason} = ...
```
- Tagged tuples — pattern-matching on tags.
- **Минус для Metalogos**: tagged unions не в языке (Structs есть, но pattern-matching на variant — нет).

### 2.4. Elm — Result type (но с explicitness)
```elm
case result of
    Ok value -> ...
    Err error -> ...
```
- **Отвергнуто ADR-0106**: Result type = Option/Result, явно rejected.

### 2.5. Python — exception-based
```python
try:
    result = do_something()
except SpecificError as e:
    ...
```
- Exceptions with class hierarchy.
- **Минус для Metalogos**: exceptions не в языке (errors are values, not exceptions).

## 3. Кандидаты для ADR

### (а) Стандартный error-struct + ?-оператор раннего возврата
```mlog
let result = try_expr(call_llm("..."))
# result = Struct { ok: Bool, value: String, error: Struct{code, message} }
# ?-оператор: early return on error
let r = call_llm("...") ?  # if error → return error-struct from current scope
```
- **Breaking surface**: новый оператор `?` (грамматика); новый error-struct shape; изменение сигнатур builtins (возврат Struct вместо String при ошибке — обратная несовместимость).
- **Синергия**: коды (`code` field) = ADR-0131/0140 stable codes.

### (б) Расширение try-семантики до структурных ошибок (ПРИОРИТЕТ владельца)
```mlog
let result = try call_llm("...")  # возвращает Struct { ok, value, error } вместо Unit
if result.ok then {
    respond("200", result.value)
} else {
    respond("500", result.error.message)
}
```
- **Breaking surface**: `try` возвращает `Struct` вместо `Unit` при ошибке — обратная несовместимость для кода, который проверяет `try result == Unit`. Менее инвазивный, чем (а) — нет нового оператора.
- **Синергия**: try уже реализован (№91); extension, не новая конструкция.
- **Совместимость**: `if try_result == Unit` — можно backward-compat: если старый код проверяет `Unit`, новый код возвращает Struct → `Unit`-проверка провалится (ломает старый код). Альтернатива: `try?` — новый оператор, старый `try` остаётся как есть (возвращает Unit). Но `try?` = новый оператор = (а).

### (в) Статус-кво + документированный паттерн
- `try` остаётся как есть (Unit при ошибке).
- Error-handling pattern документирован: громкие ошибки для critical-path; soft-failure (`""`, `Unit`, `false`) для optional-path; `try` для «ошибка не критична, но известна».
- **Breaking surface**: ноль.
- **Минус**: не решает проблему «много громких runtime-ошибок» для больших agentic-приложений.

## 4. Сценарии Fosved

### 4.1. Chain (pattern A → pattern B → pattern C)
```
pattern A() -> String { let r = call_llm("..."); return B(r) }
pattern B(x: String) -> String { ... }
```
- Если `call_llm` возвращает Err → A возвращает Err → chain ломается.
- С текущей моделью: каждый pattern возвращает `Result<String, String>` (в интерпретаторе — `Result<Value, String>`).
- С кандидатом (б): `try call_llm(...)` → Struct → pattern может вернуть Struct → caller pattern проверяет `.ok` → если не ok, return error-struct дальше.

### 4.2. Dept-handler (route → pattern → LLM)
```
route "/classify" method=GET {
    let r = try call_llm("classify", query_param("text"))
    if r == Unit then { respond("500", "LLM call failed") }
    else { respond("200", r) }
}
```
- С кандидатом (б): `let r = try call_llm("...")` → Struct { ok, value, error } → `if r.ok then { respond("200", r.value) } else { respond("500", r.error.message) }`.

## 5. Оценка breaking-поверхности

| Кандидат | Грамматика | Builtins | Старый код | Усилия |
|---|---|---|---|---|
| (а) error-struct + `?` | новый оператор | новые сигнатуры | ломается | ~наряд |
| (б) try → Struct | без изменений | без изменений | `try x == Unit` ломается | ~наряд |
| (в) статус-кво | без изменений | без изменений | без изменений | 0 |

**Приоритет владельца**: кандидат (б). Финальный выбор — после ADR.
