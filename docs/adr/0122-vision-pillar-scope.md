# ADR-0122: Vision pillar scope — inference-first over open weights, images before video

**Status:** Accepted
**Date:** 2026-09-07
**Naryad:** #209 (R0, research phase)
**Mandate:** owner directive of 2026-09-07 to begin implementation of
`Metalogos_Vision_Pillar_Plan.md`; this ADR formalizes that plan's scope decisions.

## Context

The owner requested a fourth generative capability pillar (images, later video) on top of
the existing `Reflex` pillar (ADR-0114..0121). The September 2026 state of open image
generation is consolidated around one architecture family: DiT/MMDiT backbones trained
with flow matching, distilled to few-step inference (8 NFE is normal, not an optimization).
Training such backbones from scratch is datacenter economics: pretraining corpora at
this scale do not exist in open access, and the best open training data is
non-commercially licensed.

At the same time, `candle` already carries full-pipeline precedents (SD 1.5/2.1, SDXL,
Wuerstchen): the "text encoder → denoiser → VAE decode" cycle in Rust has been walked
by someone. The realistic unit of value for Metalogos is the layer **above** the weights:
inference, adaptation, orchestration — and, as the differentiator, compiler-level
provenance and supply-chain security (ADR-0125).

## Decision

1. **Inference-first, no pretraining.** The pillar runs open checkpoints and may adapt
   them (LoRA-class, phase R7); it never pretrains base models. This mirrors Reflex's
   "real training, real accuracy — at local-machine scale" honesty contract (ADR-0115,
   ADR-0112).
2. **Images before video.** Image generation reaches reproducible quality first
   (phases R1–R6, наряды №210–215); video is a separate research cycle with its own ADR
   (**ADR-0126 reserved**). Video inference in pure Rust on consumer GPUs (3D VAE,
   temporal attention) is a distinct risk class and is not promised in this cycle.
3. **Explicit non-scope: NCII capability targets.** The language whose brand is
   security-by-design with compiler gates cannot ship a pillar whose primary community
   use-case is non-consensual synthetic imagery of real people. This is not enforced by
   prose but by mechanism: policy declarations and provenance gates (ADR-0125) make
   honest use explicit and dishonest use loud.
4. **Feature `vision`, off by default** (precedent: `candle`/ADR-0118), with a dedicated
   CI job (precedent: наряд №200's candle-CI job) and crosscheck exceptions referencing
   this ADR (precedent: n187).
5. **All pillar code lives in `src/vision/`, split from day one** under the existing
   module-size-guard discipline; it must not repeat the `diagrams.rs` god-file history.

## Consequences

- ADR-0123 (wedge choice), ADR-0124 (value/registry design), ADR-0125 (provenance gates)
  carry the technical detail; this ADR holds only scope.
- Reserves ADR numbers 0123–0126 for the Vision block; final numbers fixed in one pass
  with the Voice pillar plan (which reserves 0127+) to avoid parallel-ADR collisions —
  the same collision class as наряды 195–197 vs 200–203.
- Crosscheck exclusions for vision examples will grow in R1 and shrink only when
  examples genuinely pass on both backends, mirroring ADR-0121's staging discipline.

## Naryad map — single source of truth for pillar numbering

Reservation lesson applied: the №195–197 vs №200–203 collision and the
ADR-0121 stage-slot drift (update note there) both happened because the
numbering lived in private plan documents. It now lives here, in the repo.

| Наряд | Фаза | Содержание | Статус |
|---|---|---|---|
| №206 | maintenance | test-triage: 44 advisory-провала устранены, `test-integration` → blocking (PR #219). Реальные фиксы: Eq/Ne rollback-операторы инвертированы с №148 (hooks.rs), `!=` отсутствовал в `parse_compare_op` (grammar had NEQ, Rust-arm нет), import_path trim, CARGO_MANIFEST_DIR для std_root. Долг: 47 `#[ignore]` с причинами — погашается №207/№208 | merged, PR #219 |
| №207 | maintenance | `run_test_server_with_backend_in_dir(source, backend, base_dir)` — один параметр на оба пути резолва (TW: `set_base_dir`, VM: `with_std_root`); обёртка `run_test_server_with_backend` сохранена. Итог по тестам: активны 2 TW-теста (n161 `block3_tw_serves_imported_pattern`, dept `tw_serves_all_dept_branches_correctly`); 5 VM-зависимых тестов игноры с n207/n208-якорями. **Ключевое открытие: VM не умеет вызывать user-паттерны из тел маршрутов (HTTP 500)** — см. коррекцию №208 | merged, PR #221 (+ fix-forward) |
| №208 | maintenance | **Скоп скорректирован по итогам №207:** корневая причина — VM не диспетчеризует user-паттерны из тел маршрутов (HTTP 500, доказано n161 Block 3, verbatim "status 500 != 200") + дивергенции query_param/json_body/respond. Цель: VM route-body = TW (naryad_160 — 24 теста, n161 Block 3 VM — 3, dept_parity VM — 2, vm_golden — 2; итого 31 un-ignore) | reserved (долг №206) |
| №209 | R0 | research (wedge) + ADR-0122..0125 | merged, PR #214 |
| №210 | R1 | каркас: feature `vision`, `src/vision/`, Value::Vision + VisionRegistry, SSOT loud-stubs, vision-tests CI job | merged, PR #218 |
| №211 | R2 | текст-энкодер на Reflex-блоках (Qwen3-архитектура): TextEncoderConfig + TextEncoder::new(seed) + forward → [seq, hidden], RoPE/QK-norm/GQA/causal; `vision` влечёт `candle`. **Доставлено с дефектами** (выявлены верификацией, исправлены fix-forward): golden-пиннинг не выполнен (хэши не запинены), config-пиннинг Qwen3-4B сфабрикован (40/8-64-6912-32768 вместо реальных 32/8-128-9728-40960), локальная PRNG-копия расходится с SSOT-контрактом `src/nn`, перекрытие seed-потоков | merged, PR #223 (+ fix-forward); остаток долга → №230 |
| №212 | R3 | end-to-end клин (flow-sampler + VAE), первое изображение из .mlog; **Go/No-Go gate** | reserved |
| №213 | R4 | `vision { }`-декларации, парсер, semantic, taint(prompts), dispatch | reserved |
| №214 | R5 | security-гейты (ADR-0125), manifest, LSB-watermark, checksum-пиннинг | reserved |
| №215 | R6 | vision_edit, LoRA-адаптеры, vision_save/load (SQLite BLOB) | reserved |
| №216 | R7 (опц.) | LoRA-дообучение через candle autograd | reserved |
| №217–219 | V1–V3 | видео-фаза (после отдельного research-цикла, ADR-0126 reserved) | reserved |
| №230 | maintenance (Vision R2 hotfix) | golden-пиннинг: PRNG SSOT (замена локальной копии на `crate::nn::attention::generate_uniform_f32`) + stream-гигиена (per-parameter derivation без перекрытий) → затем пиннинг const GOLDEN_* после 3 бит-в-бит прогонов (порядок обязателен: PRNG меняет все значения); panic-free инварианты; пере-якорение константного теста. R3 (№212) стартует только после №230 | reserved — следующий |

**Free ranges:** буфер №206–208 исчерпан (№206 merged, №207/№208 — долг №206);
№220–229 — резерв Voice-пиллара (`Metalogos_Voice_Pillar_Plan.md`);
№230 — Vision R2 hotfix (golden pinning + PRNG SSOT), №231+ — свободны
(мейнтенанс/хотфиксы до резервирования нового пиллара).

**Rule:** перед стартом любого наряда исполнитель сверяется с этой картой и с
update-нотацией ADR-0121. Новый пиллар обязан зарезервировать диапазон номеров
в своём scope-ADR до первого наряда — не в приватных планах.
