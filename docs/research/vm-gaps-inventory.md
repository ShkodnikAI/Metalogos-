# VM Gaps Inventory — закрытие гэпов Match / BlockIfElse / match_expr

> **Наряд №293** (issue #356, P0/adr — VM_COMPLETE). Источник: внешний аудит Metalogos 2026-09-13. Верифицировано на main `fdfbfb7` (post-№292 merge).
> **Решение владельца 2026-09-14**: supersede оговорки «Do not implement…» ADR-0105 — стадийное закрытие гэпов; флип дефолта (ADR-0088) остаётся за гейтами: parity 100% + полный crosscheck + soak + реальная нагрузка.

## 1. Известные гэпы (verified на main `fdfbfb7`)

### 1.1. `Match` statement (compiler.rs:1373)

```rust
Statement::Match { .. } => {
    return Err("compile: Match statement not yet supported in VM bytecode \
         (use tree-walking interpreter)"
        .into());
}
```

**Семантика** (REFERENCE §3.4 + `src/interpreter/execution.rs`): `match expr { arm* else? }` — 4 вида arm:
- exact: `"val" then { stmts }`
- prefix: `starts_with "pre" then { stmts }`
- substring: `contains "sub" then { stmts }`
- compare: `> expr then { stmts }` (сравнение scrutinee с expr, любой из `>`/`<`/`>=`/`<=`/`==`/`!=`)
- `else { stmts }` — fallback.

Match как **statement** — выполняет выбранную ветку ради side-effects, результат не используется. Match как **expression** (`let x = match y { ... }`) — см. §1.3.

**Стоимость закрытия** (по прецеденту №91 — TryEval):

| Компонент | Estimate | Детали |
|---|---|---|
| Новая bytecode instruction | ~30 строк | `Match { scrutinee_code, arms: Vec<MatchArm>, else_code: Option<Vec<Instruction>> }` в `src/bytecode.rs`. `MatchArm { kind: MatchArmKind, value_code: Vec<Instruction>, body_code: Vec<Instruction> }`. `MatchArmKind { Exact, StartsWith, Contains, Compare(BinOp) }`. |
| Compiler (compile Match statement) | ~80 строк | В `compile_statement`: для каждого arm — compile scrutinee + compile value-expr + compile body. Match expression → Const(value) + branch-to-arm по result. |
| Compiler (compile Match as expression для `let`/`return`) | ~80 строк | Аналогично, но body возвращает Value — нужен новый opcode `MatchReturn` или стек-based Result через существующий `Return` + scope-aware подход. |
| VM dispatch — execute_arm | ~60 строк | В обоих dispatch loops (`run` для main program, `execute_route_code` для route handlers): для каждого arm — проверить условие, если match — выполнить body; если все провалились — выполнить else (или return Unit). |
| Tests | ~150 строк | New `tests/naryad_<N>_vm_match.rs` — все 4 arm kinds + else + match as statement + match as expression + recursion inside arm body. |
| crosscheck_backends.rs — remove `p_match_switch.mlog` exclusion | 1 строка | `if name == "p_match_switch.mlog" { continue; }` → удалить. |
| **Итого** | **~400 строк** | Прецедент №91 (TryEval) — ~250 строк. Match сложнее (4 arm kinds, compare с 6 операторами) — ~400. |

### 1.2. `Expr::BlockIfElse` (compiler.rs:902)

```rust
Expr::BlockIfElse { .. } => {
    return Err("compile: block if/else expression not yet supported \
         in VM bytecode (use tree-walking interpreter)"
        .into());
}
```

**Семантика** (REFERENCE §3.4 + `src/ast.rs:1483`): `if cond { stmts } else { stmts }` как **значение** (в `let`/`return`/аргументе). Value — последнее выражение в выбранной ветке (Unit если нет non-Unit expr).

**Важно**: `Statement::IfElseBlock` (block if/else как оператор) — **уже поддержан** VM (Narяд №129). Только expression-форма не поддержана.

**Стоимость закрытия**:

| Компонент | Estimate | Детали |
|---|---|---|
| Новая bytecode instruction | ~15 строк | `BlockIfElse { cond_code: Vec<Instruction>, then_code: Vec<Instruction>, else_ifs: Vec<(Vec<Instruction>, Vec<Instruction>)>, else_code: Option<Vec<Instruction>> }`. Альтернатива: переиспользовать `JumpIfFalse`/`Jump` + `Pop` — но структурированный opcode проще. |
| Compiler (compile BlockIfElse expr) | ~50 строк | В `compile_expr_with_locals`: для каждой ветки — compile condition + compile body (последний stmt возвращает value через `Return`-free path). Last-expression-in-block → value semantics (в TW это работает через `eval_block` — VM нужен аналог). |
| VM dispatch | ~40 строк | В обоих loops: evaluate cond → jump-to-matching-branch → execute → leave value on stack. |
| Tests | ~100 строк | New `tests/naryad_<N>_vm_block_if_else_expr.rs` — простое/вложенное/else-if/нет else (Unit). |
| **Итого** | **~205 строк** | Меньше чем Match — нет arm-kind вариативности, но value-semantics-of-last-stmt сложнее (TW eval_block) |

### 1.3. `match_expr` (`let x = match y { ... }`) — TW-only, наряд №173b

`match_expr` — это `Match` statement используемый в `let`/`return` позиции. На самом деле это **часть гэпа 1.1** — если закрыть Match statement в VM, нужно закрыть и его expression-форму. Постановка №173b добавила match_expr только в TW; для VM он будет закрыт автоматически при реализации Match-as-expression в гэпе 1.1.

**Стоимость**: включена в §1.1 (compiler Match as expression, ~80 строк) — отдельной работы не требуется.

## 2. Скрытые гэпы — инвентаризация (grep `unimplemented`/`not yet supported`)

### 2.1. compiler.rs — 2 явных гэпа

```
$ grep -nE "unimplemented|not yet supported|not supported|TODO\(vm\)|FIXME\(vm\)" src/compiler.rs
902:                return Err("compile: block if/else expression not yet supported \
1374:                    return Err("compile: Match statement not yet supported in VM bytecode \
```

Только два гэпа — оба из §1. Других `unimplemented`/`not yet supported` в compiler.rs **нет**.

### 2.2. vm.rs — 0 явных гэпов

```
$ grep -nE "unimplemented|not yet supported|not supported|TODO\(vm\)|FIXME\(vm\)" src/vm.rs
(empty)
```

`vm.rs` не содержит `unimplemented!()` или `not yet supported` — все неиспользуемые opcode-arms обрабатываются через `=> {}` no-op или `panic!("unknown opcode: {:?}")` (для настоящих unknown opcodes).

### 2.3. crosscheck_backends.rs — 3 исключения

```
$ grep -n "continue;" tests/crosscheck_backends.rs | head -5
```

| File | Причина | Гэп |
|---|---|---|
| `p_match_switch.mlog` | Exercises `match` statement | §1.1 — закрыть Match → убрать исключение |
| `p118_collection_utils.mlog` | `unique`/`chunk`/`sort` results через string `+` — heterogeneous types | VM `eval_binop` rejects heterogeneous; TW auto-coerces. Отдельный гэп binop coercion. |
| `reflex_math.mlog` | `random_seed`/`random` (TW-only — VM has no PRNG state); Bool→String formatting ("true" in TW, "1" in VM) | 2 гэпа: PRNG state + Bool→String formatting parity. |

### 2.4. Скрытые гэпы (вывод из §2.3)

После закрытия §1.1 (Match) остаются 2 скрытых гэпа:
- **Binop coercion** — heterogeneous List + String concatenation. VM eval_binop strict, TW lenient. Стоимость: ~80 строк (ослабить eval_binop + 6-10 contract tests).
- **PRNG state** — `random_seed`/`random` — TW-only. Стоимость: ~50 строк (добавить `RandomState` в Vm struct, seed propagation, deterministic mode for tests).
- **Bool→String formatting** — `"true"` vs `"1"`. Стоимость: ~10 строк (форматирование в vm.rs).

**Итого скрытых гэпов**: 3 (binop coercion, PRNG, Bool→String).

## 3. Прецедент №91 — TryEval

Наряд №91 закрыл `Expr::Try` (`try expr`) в VM. Шаблон:
1. Новая bytecode instruction `TryEval(Vec<Instruction>)` в `src/bytecode.rs` (10 строк — enum variant).
2. Compiler: `Expr::Try { expr: inner, .. }` → compile inner в sub-vec, emit `TryEval(inner_code)` (5 строк).
3. VM dispatch — оба loops (`run` + `execute_route_code`): для `TryEval(inner_code)` — evaluate inner, catch error → push Unit; success → push value (по 10 строк на loop = 20 строк).
4. Tests: `tests/naryad_91_*` (~30 строк — success path + error path).
5. crosscheck_backends.rs — remove exclusion (1 строка).

**Всего ~250 строк для одной инструкции**. Match сложнее (4 arm kinds × 6 операторов × branch logic) → ~400 строк. BlockIfElse проще (нет arm kinds) но value-semantics-of-last-stmt сложнее → ~205 строк.

## 4. Итоговая стоимость закрытия

| Гэп | Стоимость (LOC) | Нарядов |
|---|---|---|
| §1.1 Match statement + expression | ~400 | 1 наряд (~№294) |
| §1.2 BlockIfElse expression | ~205 | 1 наряд (~№295) |
| §2.4 Binop coercion (heterogeneous types) | ~80 | 1 наряд (~№296) |
| §2.4 PRNG state | ~50 | 1 наряд (~№297) |
| §2.4 Bool→String formatting parity | ~10 | мини-наряд (можно в один с PRNG) |
| crosscheck_backends.rs cleanups | 3 строки | в каждом из выше |
| **Итого** | **~745 LOC** | **~4 наряда** |

Все 4 наряда — прецедент №91 по структуре: instruction + compiler + VM dispatch + tests. Не требуют ADR (расширение VM, не новая семантика языка — ADR-0105 оговорка снимается решением владельца).

## 5. Критерий «стратегической нужды» (для флипа дефолта ADR-0088)

Флип дефолта `METALOGOS_SERVE_BACKEND=interpreter` → `vm` — только при **всех** условиях:

1. **Parity 100%** — все 3 crosscheck exclusions сняты (p_match_switch, p118_collection_utils, reflex_math). Это означает: все 4 гэпа из §4 закрыты.
2. **Полный crosscheck зелёный** — `tests/crosscheck_backends.rs` без единого `continue;` (кроме negative-test контрактов типа p50_unknown_fn, p2_wrong_types — которые designed-to-fail).
3. **Soak период** — 1 sprint (≈2 недели) работы FOSVED на VM бэкенде в стейджинге, без panic/regression. Сейчас FOSVED работает на TW; VM opt-in только для экспериментов.
4. **Реальная нагрузка** — benchmark на production-class .mlog файле (≥2000 строк, с LLM calls, DB, vision — representative FOSVED workload). VM должен показать ≥2× latency improvement или эквивалентную latency с memory/CPU win.

Без ВСЕХ четырёх условий флип дефолта не делается — ADR-0088 `Implemented (default remains interpreter)` stays.

## 6. План стадий (решение владельца 2026-09-14 — supersede ADR-0105 оговорки)

**Stage 0** (этот наряд — №293): research + ADR-0141 + inventory (этот документ). Ноль кода.

**Stage 1** (наряды №294–№297): закрытие 4 гэпов по прецеденту №91. Каждый — отдельный наряд, отдельный PR, отдельные тесты + crosscheck exclusion removal. Порядок: Match (самый большой эффект — p_match_switch) → BlockIfElse → binop coercion → PRNG + Bool→String.

**Stage 2**: parity gate — после всех 4 нарядов, `tests/crosscheck_backends.rs` без `continue;` exclusions для VM-uncovered constructs. Если parity 100% — перейти к Stage 3.

**Stage 3**: soak — FOSVED на VM в стейджинге 1 sprint. Без panic/regression → перейти к Stage 4.

**Stage 4**: real-load benchmark — representative FOSVED workload на VM. ≥2× latency improvement или эквивалентная latency с memory/CPU win.

**Stage 5** (только если Stage 2-4 зелёные): флип дефолта ADR-0088 `interpreter` → `vm`. Отдельный ADR (новый номер — `0142` или выше). С сохранением opt-out через `METALOGOS_SERVE_BACKEND=interpreter` для back-compat.

## 7. Риски

1. **Match expression value-semantics** — TW `eval_block` возвращает последнее non-Unit значение. VM bytecode не имеет concept-of-block-as-value; нужен pattern: последний stmt в блоке → `SetLocal`/`Push`. Риск: тонкости с `if-then-else` внутри block (statement vs expression).
2. **BlockIfElse vs Statement::IfElseBlock overlap** — statement-form уже работает, expression-form — нет. В компиляторе нужно различать контекст (`let x = if ...` vs `if ... { stmts }`). В TW это различается в execution.rs; VM компилятор должен делать то же различение.
3. **Binop coercion** — ослабление strict typing в VM `eval_binop` может сломать существующие VM tests (которые полагаются на strict). Все heterogeneous-binop tests нужно переделать — `assert!(result.is_err())` → `assert_eq!(result, ...)`.
4. **PRNG determinism** — VM должен deterministic mode для tests (seed propagation через `Vm::set_random_seed`). Если random state global — гонки между request handlers в serve.
5. **Soak regressions** — VM может иметь edge cases на production-load, которые не ловятся на examples. 1 sprint — минимальный период; если регрессии — продлевать.

## 8. Альтернативы

- **Не закрывать гэпы, оставить VM experimental** (оригинальная позиция ADR-0105). Решение владельца 2026-09-14 — supersede этой оговорки, так что альтернатива отвергнута.
- **Закрывать все гэпы одним мега-нарядом** — отвергнуто: риск review-load, regression-batch. По прецеденту №91 — один наряд на гэп.
- **Флип дефолта без soak** — отвергнуто: ADR-0088 уже зарегистрировал риск "static checks vs production". Soak обязателен.

## 9. Что НЕ делать в этом наряде (№293)

- Не компилировать Match/BlockIfElse (Stage 1 — наряды №294–№297).
- Не менять дефолт бэкенда (Stage 5 — после Stage 2-4).
- Не «тихо» переоткрывать ADR-0105 — только явный supersede владельцем (сделано 2026-09-14, зафиксировано в этом документе + ADR-0141).
