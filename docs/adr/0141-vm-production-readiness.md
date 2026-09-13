# ADR-0141: VM production-readiness — staged gap closure + parity-gated default flip

**Status:** Accepted (решение владельца 2026-09-14 — supersede оговорки ADR-0105)
**Date:** 2026-09-14
**Naryad:** #293 (issue #356, P0/adr — VM_COMPLETE)
**Amends / Supersedes (partial):** ADR-0105 (`Bytecode VM — experimental scope`) — supersede оговорки «Do not implement Match/BlockIfElse in the VM under this ADR — revisit only under a real strategic need»; ADR-0105 §Decision 1-4 остаются в силе (TW = guaranteed full-language backend; VM = experimental for full-language use до Stage 5).
**Precedent:** ADR-0088 (VM for `mlog serve`, opt-in default interpreter — флип дефолта остаётся за гейтами), Наряд №91 (TryEval — лекало закрытия одного VM-гэпа), ADR-0073 (JIT experimental scaffold).

## Context

ADR-0105 (Accepted 2026-08-21) зафиксировал два подтверждённых гэпа в bytecode VM: `Match` не компилируется вовсе; `Expr::BlockIfElse` (if/else как значение в `let`/`return`) не компилируется — громкая ошибка с наряда №129 (раньше тихо компилировалось в Unit). ADR-0105 прямо постановил: «Do not implement Match/BlockIfElse in the VM under this ADR — revisit only under a real strategic need (demonstrated load where VM throughput matters), not because leaving the gaps feels incomplete». FOSVED работает на TW; ADR-0088: дефолт `mlog serve` — interpreter, VM — opt-in.

Внешний аудит Metalogos 2026-09-13 (состояние main post-PR #353) предложил пересмотреть это решение. **Решение владельца 2026-09-14**: supersede оговорки ADR-0105 — стадийное закрытие гэпов; флип дефолта (ADR-0088) остаётся за гейтами: parity 100% + полный crosscheck + soak + реальная нагрузка.

Подробная инвентаризация гэпов и стоимость закрытия — `docs/research/vm-gaps-inventory.md` (этот ADR её резюмирует).

## Decision (staged plan)

### D1. Stage 0 — research (этот наряд, №293)

Ноль кода. Research inventory (`docs/research/vm-gaps-inventory.md`) + этот ADR + README update (VM experimental — отметить "staged closure in progress per ADR-0141"). Решение владельца зафиксировано.

### D2. Stage 1 — закрытие 4 гэпов по прецеденту №91

Каждый гэп — отдельный наряд, отдельный PR, отдельные тесты + crosscheck exclusion removal. Порядок (по эффекту на parity):

| Наряд (предлаг.) | Гэп | Стоимость (LOC) | crosscheck exclusion removal |
|---|---|---|---|
| №294 | Match statement + expression (`Statement::Match` + `match_expr`) | ~400 | `p_match_switch.mlog` |
| №295 | `Expr::BlockIfElse` (if/else как значение) | ~205 | (нет прямого exclusion; покрывается VM-excluded примерами в `p5_if_else.mlog`-style programs — добавить crosscheck если нужно) |
| №296 | Binop coercion (heterogeneous List+String) | ~80 | `p118_collection_utils.mlog` |
| №297 | PRNG state (`random_seed`/`random`) + Bool→String formatting | ~60 | `reflex_math.mlog` |

**Структура каждого наряда** (по лекалу №91 — TryEval):
1. Новая bytecode instruction в `src/bytecode.rs` (enum variant).
2. Compiler arm в `src/compiler.rs` (compile expression/statement → emit instruction).
3. VM dispatch в `src/vm.rs` — оба loops (`run` для main program + `execute_route_code` для route handlers).
4. Tests — `tests/naryad_<N>_*.rs` covering success path + edge cases + regression.
5. crosscheck_backends.rs — remove the corresponding exclusion (1 строка).

**ADR не требуется** для каждого гэпа — это расширение VM в рамках уже принятой семантики языка (Match/BlockIfElse/binop/random уже определены в грамматике и работают в TW). Расширение VM bytecode — implementation work, не архитектурное решение.

### D3. Stage 2 — parity gate

После Stage 1 (все 4 наряда смержены): `tests/crosscheck_backends.rs` без единого `continue;` VM-uncovered exclusion (кроме negative-test контрактов — `p50_unknown_fn`, `p2_wrong_types` — designed-to-fail, не parity concern). Если parity 100% — перейти к Stage 3. Если регрессии — дополнительные наряды для их закрытия перед переходом.

### D4. Stage 3 — soak

FOSVED на VM в стейджинге **1 sprint (≈2 недели)**, без panic/regression. Сейчас FOSVED работает на TW; VM opt-in только для экспериментов. Soak — обязательный период; если panic/regression — продлевать или откатывать.

### D5. Stage 4 — real-load benchmark

Benchmark на production-class .mlog файле (≥2000 строк, с LLM calls, DB, vision — representative FOSVED workload). VM должен показать:
- **≥2× latency improvement** (через bytecode dispatch + no AST traversal overhead), OR
- **Эквивалентная latency с memory/CPU win** (если latency не 2×, но memory footprint значительно меньше — приемлемо для restricted envs).

Без одного из этих условий — флип дефолта не делается (Stage 5 блокируется).

### D6. Stage 5 — флип дефолта ADR-0088 (только если Stage 2-4 зелёные)

Флип `METALOGOS_SERVE_BACKEND` default с `interpreter` на `vm`. Отдельный ADR (новый номер — `0142` или выше). С сохранением opt-out через `METALOGOS_SERVE_BACKEND=interpreter` для back-compat (старые деплойменты, edge cases, debugging).

**ADR-0088 status update**: `Implemented (default remains interpreter)` → `Implemented (default flipped to vm per ADR-0XXX, opt-out via METALOGOS_SERVE_BACKEND=interpreter)`. Отдельный amend-ADR — не делается в existing ADR-0088 (историческая точность).

### D7. ADR-0105 amend

ADR-0105 §Decision 1-4 остаются в силе:
- TW = guaranteed full-language backend (Stage 5 не отменяет — TW остаётся как opt-out).
- VM = experimental for full-language use — но статус меняется: "experimental" → "production-ready after Stage 1-4" (после закрытия гэпов).

Оговорка «Do not implement Match/BlockIfElse in the VM under this ADR» — **superseded** этим ADR (Stage 1 закрывает гэпы).

## Consequences

- **Stage 1 (наряды №294-№297)**: VM bytecode coverage растёт с ~95% до 100% language constructs. crosscheck_backends.rs становится без exclusions (для VM-uncovered constructs).
- **Stage 2-4**: parity + soak + benchmark — гейты перед флипом. Если любой fail — план пересматривается (возможно, оставить VM opt-in как есть, без флипа дефолта).
- **Stage 5 (если зелёный)**: `mlog serve` дефолт = VM. FOSVED получает автоматический perf boost. TW остаётся для debugging / back-compat.
- **Risk**: каждая стадия может вскрыть hidden гэпы (не охваченные в `docs/research/vm-gaps-inventory.md`). Это нормально — inventory верифицировано на `main fdfbfb7`, но не proof от future constructs.
- **No regression risk** для existing VM usage — Stage 1 только добавляет coverage (новые opcode + compiler arms); существующие bytecode остаются валидными.

## Addendum: Что НЕ делает этот ADR

- **Не закрывает гэпы в этом наряде** — Stage 0 = research only. Stage 1 = наряды №294-№297.
- **Не меняет дефолт бэкенда** — Stage 5 (отдельный ADR) после Stage 2-4.
- **Не удаляет ADR-0105** — только supersede оговорку «Do not implement…». ADR-0105 остаётся в силе для §Decision 1-4.
- **Не добавляет новые конструкции в язык** — Match/BlockIfElse/binop/random уже определены в грамматике и работают в TW. Расширение VM bytecode — implementation work.
