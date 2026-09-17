# Error Protocol — Prior Art & Current State Inventory

> **Naryad #298** (issue #362, P1/adr — ERR_PROTOCOL). Source: external Metalogos audit 2026-09-13. Owner decision 2026-09-14: GO on research + ADR; priority — candidate (b), extension of try semantics (#91).

## 1. Current state of Metalogos error paths

### 1.1. Loud runtime errors (default)
All runtime errors are `Result<_, String>` (a message string), returned loudly:
- `Err("call_llm() failed: ...")` — the standard pattern.
- `Err("division by zero")` — arithmetic errors.
- `Err("field 'x' not found on struct")` — field-access errors.
- Pattern: every error is a String, unstructured.

### 1.2. try mechanics (Naryad #91, TryEval)
- `try expr` — returns `Unit` on error, `Value` on success.
- Implementation: `Expr::Try { expr, span }` → compiled to `Instruction::TryEval(inner_code)`.
- VM dispatch (both loops): if inner_code returns `Err` → push `Value::Unit`; otherwise push `Value`.
- Limitation: `try` returns Unit on error — **loses the error information** (code/message/position). There is no way to learn which error actually occurred.

### 1.3. Soft-failure conventions (ADR-0106)
- `respond("404", "not found")` — an HTTP-level soft-failure (not a crash).
- `kv_get("missing")` → `""` — empty-string fallback.
- `recall("missing")` → `""` — soft-failure.
- `query_param("missing")` → `""` — soft-failure.
- Convention: builtins return `Unit`/`""`/`false` on absence, not `Err`.
- Loud errors only for "real" errors (type mismatch, division by zero, network failure).

### 1.4. EOF/End markers
- `llm_stream_next` → `"__end__"` (Naryad #275, ADR-0137) — end-of-stream marker.
- `""` — keep-alive ping.
- Convention: special sentinel values for soft-EOF.

### 1.5. JSON shape of diagnostics (ADR-0131/0140)
- `mlog check --json` → an array of `{code, message, span, severity}` (Naryad #255, not yet implemented).
- `check_id` — UPPER_SNAKE_CASE stable codes (ADR-0131: `SECRET_LEAK`, `SQL_DYNAMIC`, etc.).
- ADR-0140 (Naryad #288): the no-reuse rule + SSOT-registry discipline for diagnostic codes.
- Synergy: if the error struct has `{code: String, message: String}`, the codes are already stable via ADR-0131/0140.

## 2. Prior Art — error protocols without Result

### 2.1. Go — error values (multiple return values)
```go
result, err := doSomething()
if err != nil {
    return err
}
```
- Error — a separate value, not a type container.
- `error` interface (`Error() string`).
- **Minus for Metalogos**: multiple return values are not in the language.

### 2.2. Lua — nil + error value (pcall)
```lua
local ok, err = pcall(function() ... end)
if not ok then ... end
```
- `pcall` — protected call, returns `(success, value_or_error)`.
- **Similarity to Metalogos**: `try` already works like pcall (returns Unit on error).
- **Difference**: Lua returns the error string; Metalogos loses it.

### 2.3. Erlang — tagged tuples
```erlang
{ok, Value} = doSomething(),
{error, Reason} = ...
```
- Tagged tuples — pattern-matching on tags.
- **Minus for Metalogos**: tagged unions are not in the language (Structs exist, but pattern-matching on a variant does not).

### 2.4. Elm — Result type (but with explicitness)
```elm
case result of
    Ok value -> ...
    Err error -> ...
```
- **Rejected by ADR-0106**: Result type = Option/Result, explicitly rejected.

### 2.5. Python — exception-based
```python
try:
    result = do_something()
except SpecificError as e:
    ...
```
- Exceptions with class hierarchy.
- **Minus for Metalogos**: exceptions are not in the language (errors are values, not exceptions).

## 3. Candidates for the ADR

### (a) Standard error-struct + ?-operator for early return
```mlog
let result = try_expr(call_llm("..."))
# result = Struct { ok: Bool, value: String, error: Struct{code, message} }
# ?-operator: early return on error
let r = call_llm("...") ?  # if error → return error-struct from current scope
```
- **Breaking surface**: a new operator `?` (grammar); a new error-struct shape; changed builtin signatures (returning a Struct instead of a String on error — backward incompatible).
- **Synergy**: the codes (`code` field) = ADR-0131/0140 stable codes.

### (b) Extending try semantics to structural errors (owner PRIORITY)
```mlog
let result = try call_llm("...")  # returns Struct { ok, value, error } instead of Unit
if result.ok then {
    respond("200", result.value)
} else {
    respond("500", result.error.message)
}
```
- **Breaking surface**: `try` returns a `Struct` instead of `Unit` on error — backward incompatible for code that checks `try result == Unit`. Less invasive than (a) — no new operator.
- **Synergy**: try is already implemented (#91); an extension, not a new construct.
- **Compatibility**: `if try_result == Unit` — backward compat is possible: if old code checks `Unit` and the new code returns a Struct → the `Unit` check will fail (breaks old code). Alternative: `try?` — a new operator, the old `try` stays as is (returns Unit). But `try?` = a new operator = (a).

### (c) Status quo + a documented pattern
- `try` stays as is (Unit on error).
- The error-handling pattern is documented: loud errors for the critical path; soft-failure (`""`, `Unit`, `false`) for the optional path; `try` for "the error is not critical but is known".
- **Breaking surface**: zero.
- **Minus**: does not solve the "many loud runtime errors" problem for large agentic applications.

## 4. Fosved scenarios

### 4.1. Chain (pattern A → pattern B → pattern C)
```
pattern A() -> String { let r = call_llm("..."); return B(r) }
pattern B(x: String) -> String { ... }
```
- If `call_llm` returns Err → A returns Err → the chain breaks.
- With the current model: every pattern returns `Result<String, String>` (in the interpreter — `Result<Value, String>`).
- With candidate (b): `try call_llm(...)` → Struct → the pattern may return the Struct → the caller pattern checks `.ok` → if not ok, the error-struct is returned onward.

### 4.2. Dept-handler (route → pattern → LLM)
```
route "/classify" method=GET {
    let r = try call_llm("classify", query_param("text"))
    if r == Unit then { respond("500", "LLM call failed") }
    else { respond("200", r) }
}
```
- With candidate (b): `let r = try call_llm("...")` → Struct { ok, value, error } → `if r.ok then { respond("200", r.value) } else { respond("500", r.error.message) }`.

## 5. Breaking surface assessment

| Candidate | Grammar | Builtins | Old code | Effort |
|---|---|---|---|---|
| (a) error-struct + `?` | new operator | new signatures | breaks | ~one naryad |
| (b) try → Struct | unchanged | unchanged | `try x == Unit` breaks | ~one naryad |
| (c) status quo | unchanged | unchanged | unchanged | 0 |

**Owner priority**: candidate (b). The final choice — after the ADR.
