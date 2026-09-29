# ADR-0180: The Soft-Failure Naming Rule — "Silence Is Visible in the Name"

- Status: ACCEPTED (implemented by №481 env/env_or, №507 read_file_or, №514 to_float/to_int loud + to_float_or/to_int_or)
- Date: 2026-09-29
- driven by: №514 (gh#798); the consolidated audit 28.09 C-10; the №481 precedent (audit 25.09 §3.9)

## 1. Context

The language had no ONE rule for "when a builtin may stay silent". The
same shape of missing data was answered differently by different
builtins: `env()` returned `""` (loud since №481), `to_float("abc")`
returned `0.0`, `read_file()` of a missing file returned `""` (the №254
contract, preserved — №507 added the `_or` twin). The audit's vector:
`let amount = to_float(json_body().amount); charge(user, amount)` — the
string `"12,50"` silently became `0.0`, and the program charged zero
without an error. Silence that the caller did not ask for is a masked
failure; silence the caller ASKED for (by calling `*_or`) is an explicit
fallback. The difference must live in the NAME.

## 2. Decision (the rule)

1. **LOUD is the default.** A builtin whose operation fails — a missing
   variable, a missing file, a non-numeric string, a closed connection —
   refuses loudly with a stable coded error (`TYPE_MISMATCH`,
   `ENV_NOT_FOUND`, `IO_ERROR`, ...), naming the input and pointing at
   its `_or` twin when one exists.
2. **Silence is opt-in by name.** A builtin that returns a default on
   failure carries the `_or` suffix (`env_or`, `read_file_or`,
   `to_float_or`, `to_int_or` — the same rule, one family). Every
   firing of the fallback is announced on the audit stderr (the №326
   op-log posture: the FACT, never the value), so the explicit silence
   is still observable.
3. **Explicit silence covers DATA, not type errors.** `_or` swallows
   the documented data-level failure (unparseable string, missing
   variable/file). A programming error — a non-scalar argument, a wrong
   default type — stays loud in the `_or` twin as well.
4. **A conversion is not a soft failure.** Total mappings that never
   fail (`Bool → 1.0/0.0`, float truncation) are not silence and stay
   unchanged in both the loud and the `_or` twin.
5. **Exceptions are documented, not silent.** A preserved soft site
   (e.g. the `read_file` №254 contract) carries its owning naryad/ADR
   reference in code and in the inventory below. New soft sites are
   FORBIDDEN — the `_or` form is the only way in.

## 3. The inventory (silent/loud, by group) — the fact at the №514 landing

### math (src/builtins/math.rs)

| Builtin | Failure | Behavior | Authority |
|---|---|---|---|
| `to_float("abc")` | non-numeric string | LOUD `[TYPE_MISMATCH]` (was soft `0.0` — the C-10 vector) | №514 |
| `to_int("42abc")` | non-numeric string | LOUD `[TYPE_MISMATCH]` (was soft `0.0`) | №514 |
| `to_float_or(v, d)` / `to_int_or(v, d)` | non-numeric string | explicit default + `[TO_FLOAT_OR]`/`[TO_INT_OR]` on stderr | №514 |
| `float(s)` | invalid string | always LOUD (parse error) — no `_or` twin needed | pre-existing |
| `to_float(Bool)` / `to_int(Bool)` | — | conversion `1.0/0.0` (never fails) — not silence | ADR-0180 §2.4 |
| `abs/min/max/clamp/round/exp/ln/sqrt/pow/tanh/sigmoid/softmax` | wrong arity/type | LOUD (argument errors are never soft) | pre-existing |

### io (src/builtins/io.rs)

| Builtin | Failure | Behavior | Authority |
|---|---|---|---|
| `env(name)` | missing variable | LOUD `[ENV_NOT_FOUND]` (№481; the serve-route №259 gate unchanged) | №481 |
| `env_or(name, d)` | missing variable | explicit default + `[ENV_OR]` on stderr | №481 |
| `read_file(path)` | missing file | SOFT `""` — the deliberate №254 streaming contract, preserved | №254 (documented exception, §2.5) |
| `read_file(path)` | exists but unreadable | LOUD `[IO_ERROR]` | №481 |
| `read_file_or(path, d)` | missing file | explicit default + `[READ_FILE_OR]` | №507 |
| `write_file/append_file/delete_file` | any I/O failure | LOUD `[IO_ERROR]` / sandbox `[SANDBOX_VIOLATION]` | pre-existing |
| `print` | — | never fails on data | pre-existing |

### memory (src/builtins/memory.rs, session/kv paths)

| Builtin | Failure | Behavior | Authority |
|---|---|---|---|
| `mem_get(key)` | missing key | SOFT `""` — the empty-string default IS the documented contract of the KV shape (absence = empty, checked by `==`) | pre-existing (documented exception, §2.5) |
| `session_get(sid, key)` | missing key | SOFT `""` — same KV-shape contract | pre-existing (documented exception) |
| `mem_set/session_set` | — | total (returns the value) | pre-existing |

The memory exceptions stay soft because absence-of-key is a
first-class, routinely-queried STATE in bot programs (the №513 examples
`p6_kv_memory`/`p8_session_memory` rely on it), the value shape is
always a string, and every read without a default would force an
awkward `mem_has` preamble. The `*_or` rule still applies to NEW
memory-side readers (e.g. a typed `mem_get_or` is the sanctioned way
forward if a non-string memory type lands).

### http / llm (src/builtins/http.rs, llm paths)

| Builtin | Failure | Behavior | Authority |
|---|---|---|---|
| `http_get/http_post` (no try) | connection/refused | LOUD runtime error | pre-existing |
| `try http_get(...)` | connection/refused | the explicit `try` form — the error VALUE is returned to the program; silence is visible in the call form | №240 (try contracts) |
| `http_download(path)` | non-2xx body | the body/status is the value (№261 C4); refusal paths loud | №261 |
| `call_llm` | provider error | LOUD (`LLM_PROVIDER_UNAVAILABLE`/`LLM_TIMEOUT`); the №757 truncation is visible via `llm_last_finish_reason()` | №481-class + №757 |

## 4. Consequences

- Programs using `to_float`/`to_int` on non-numeric input BREAK — loudly,
  at the exact line, with the fix named in the message. This is the
  intended 0.28 breaking change (the CHANGELOG documents it): the old
  behavior was the defect.
- Migration is mechanical: the silent call `to_float(x)` becomes
  `to_float_or(x, 0.0)` — byte-identical behavior, visible in the name.
- The inventory is part of the contract: a PR adding or changing a
  silent/loud site updates §3 (the review question is now written down).
