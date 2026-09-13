# Наряд №294 (issue #357) — VISION_REALW — формальный No-Go

> **Дата:** 2026-09-14 (UTC+8).
> **Вердикт:** **No-Go** — исполнение runbook №237 невозможно в текущей среде; железо-гейт не пройден.
> **Дата пересмотра:** при выделении железа (≥64 GB RAM, ≥40 GB диск, GPU-контур). Открытый пункт вне репо — решение владельца по выделению.
> **Решение владельца 2026-09-14** (в issue #357 body): «условный Go — при выделении машины по preflight runbook №237 прогон исполняется дословно; без машины — формальный No-Go с явной датой пересмотра. "Tiny model" аудита отклонён».

## Контекст

Runbook №237 (`docs/research/naryad-237-real-weights-runbook.md`, 226 строк) готов к исполнению дословно:
- Манифест весов закреплён и проверяем (16 файлов, 32.85 GB; SHA-сверка против HF LFS oid; post-download refusal при несовпадении; `--dry-run`, `--only`).
- `tools/fetch_vision_weights.sh` готов.
- Go/No-Go критерии сформулированы дословно.
- Preflight: ≥40 GB диск, ≥64 GB RAM под F32-политику, ~62 GB пик.
- Код GO-ready после №236/№243 (ноль диффа в `src/**`).
- Env-gated тесты №212/№243 SKIP loudly когда `MLOG_VISION_WEIGHTS_DIR` unset — это рабочее поведение, не блокер.

## Preflight проверка (2026-09-14, контейнер агента)

| Требование runbook | Фактическое наличие | Статус |
|---|---|---|
| ≥64 GB RAM | 4.1 GB total (3.2 GB free) | ❌ FAIL (нужно 64 GB) |
| ≥40 GB диск | 9.9 GB total (7.1 GB free) | ❌ FAIL (нужно 40 GB; только под веса — 32.85 GB) |
| GPU-контур (CUDA, для candle) | /dev/nvidia* не существует | ❌ FAIL (нет GPU) |
| `MLOG_VISION_WEIGHTS_DIR` | unset | ⏸ SKIP (env-gated; рабочее поведение) |

**Итог preflight**: 3 из 3 железо-требований НЕ пройдены. Runbook №237 §preflight блокирует исполнение.

## Причины No-Go

1. **RAM**: 4.1 GB vs нужно 64 GB (F32-политика для 62 GB пика). 16× меньше минимума. Без GPU + candle offload, F16 также не поднимется (нужно ~32 GB RAM).
2. **Диск**: 9.9 GB total vs нужно 40 GB+ (только под веса — 32.85 GB). 4× меньше минимума. cargo-артефакты + клон репо уже занимают ~2 GB.
3. **GPU**: нет `/dev/nvidia*` устройств. Candle может работать на CPU, но для inference Z-Image-Turbo (4B параметров) на CPU без существенного RAM (см. п.1) — неприемлемо медленно (часы на один шаг).

## Что НЕ сделано (потому что невозможно без железа)

- ❌ Загрузка 16 файлов манифеста (32.85 GB) — `tools/fetch_vision_weights.sh` не запущен (нечего загружать в 7 GB).
- ❌ SHA-верификация скачанных файлов против HF LFS oid — нет скачанных файлов.
- ❌ Env-gated тесты №212 (`naryad_212_wedge_e2e`) / №243 (`mlog_vision_edit_export_e2e`) с `MLOG_VISION_WEIGHTS_DIR` — SKIP loudly (ожидаемое поведение; не исполнено по существу).
- ❌ Capture: golden PNG, тайминги, детерминизм (повторный прогон — байт-в-байт).
- ❌ Снятие PARKED в ADR-0122 / README / threat-model — Parked статус остаётся (No-Go → Parked не снимается).

## Что доступно без железа

- ✅ Код vision pillar — feature-gated (`--features vision`, implies `candle`), не включается в default build. CI на PR: `vision-tests (blocking)` прогоняет только tiny-golden тесты (без real weights). Это рабочее состояние.
- ✅ Runbook, манифест, `tools/fetch_vision_weights.sh` — готовы к исполнению при выделении железа.
- ✅ Tiny-golden pinned модели (#231, #232) — работают на tiny pinned весах (мегабайты, не гигабайты), CI green. Это не заменяет real-weights run, но подтверждает архитектурную корректность.

## Вердикт

**No-Go** — с явной датой пересмотра: при выделении железа (≥64 GB RAM, ≥40 GB диск, GPU-контур) прогон исполняется дословно по runbook №237. Без железа — pillar остаётся PARKED (как и было с решения владельца 2026-09-09).

## Что меняется в репо

Минимальные docs-only изменения (ноль диффа в `src/**`):
1. **ADR-0122 map row #237** — добавить запись о No-Go вердикте 2026-09-14 + дата пересмотра (явная, не "TBD"). Parked статус остаётся.
2. **README** — `Weights run parked` строка обновлена: ссылка на этот отчёт + явная дата No-Go вердикта.
3. **docs/threat-model.md** — если упоминается PARKED — ссылка на этот отчёт.
4. **CHANGELOG** — entry для №294.

## Связь

- Issue #357 (наряд).
- Runbook: `docs/research/naryad-237-real-weights-runbook.md`.
- ADR-0122 (Vision pillar scope — map row #237).
- Наряд №237 (runbook prep — merged PR #235, 2026-09-09).
- Наряды №212/№243 (env-gated тесты, SKIP loudly при unset `MLOG_VISION_WEIGHTS_DIR` — рабочее поведение, не блокер).
- Решение владельца 2026-09-14 (в issue #357 body): «условный Go; без машины — формальный No-Go с явной датой пересмотра. "Tiny model" аудита отклонён».
