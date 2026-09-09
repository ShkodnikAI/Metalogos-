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
| №212 | R3 | end-to-end клин (flow-sampler + VAE), первое изображение из .mlog; **Go/No-Go gate**. Code-complete: weights.rs + tokenizer.rs + TextEncoder::from_weights + VAE decoder (tiny golden pinned) + ZImageTransformer + FlowMatchEuler sampler. Env-gated tests SKIP loudly when MLOG_VISION_WEIGHTS_DIR unset — real-weights run + Go/No-Go report at `docs/research/naryad-212-go-no-go.md` (pending actual weights execution). DiT tiny golden запинен в №231 (PR #229). Architecture fidelity fixed in №232 (12 расхождений с diffusers исправлены), PR #230. RoPE/guard/mid-attn/cap pos_ids → №233 (PR #231). Mid-attn placement + guard truth-up + doc-sync → №234 (PR #232). VAE structure truth-up → №235 (PR #233). Generator single-source → №236 (PR #234). See PR #227 | merged (code), PR #227; Go/No-Go pending real-weights run; DiT golden №231→№232→№233 |
| №213 | R4 | `vision { }`-декларации, парсер, semantic, taint(prompts), dispatch | reserved |
| №214 | R5 | security-гейты (ADR-0125), manifest, LSB-watermark, checksum-пиннинг | reserved |
| №215 | R6 | vision_edit, LoRA-адаптеры, vision_save/load (SQLite BLOB) | reserved |
| №216 | R7 (опц.) | LoRA-дообучение через candle autograd | reserved |
| №217–219 | V1–V3 | видео-фаза (после отдельного research-цикла, ADR-0126 reserved) | reserved |
| №230 | maintenance (Vision R2 hotfix) | golden-пиннинг: PRNG SSOT (замена локальной копии на `crate::nn::attention::generate_uniform_f32`) + stream-гигиена (per-parameter derivation без перекрытий) → затем пиннинг const GOLDEN_* после 3 бит-в-бит прогонов (порядок обязателен: PRNG меняет все значения); panic-free инварианты; пере-якорение константного теста. R3 (№212) стартует только после №230 | merged, PR #225 |
| №237 | R3.7 | real-weights run prep: `tools/fetch_vision_weights.sh` (manifest-driven, `curl -L -C -`, reference = манифест-SHA → HF LFS oid, sha-verified SKIP, POST-DOWNLOAD REFUSAL, `--dry-run`, `--only`); манифест №212: секция «Как сверять с источником» (HF LFS oid = SHA-256, расхождение = громкий отказ) + layout-размеры truth-up (реально 16 файлов = 32.85 GB, HF API 2026-09-09) + 4 tokenizer SHA заполнены реальными значениями; runbook Go/No-Go `naryad-237-real-weights-runbook.md` (предпрогон ≥40 GB диск / ≥64 GB RAM по F32-политике ~62 GB peak; точные команды env-gated тестов №212; фиксация PNG/таймингов/детерминизма; критерии Go/No-Go дословно). **src нулевой дифф по дизайну** (код GO-ready после №236); real-weights прогон PARKED до железа (решение владельца 2026-09-09) | merged, PR #235 |
| №238 | R4.1 | `vision { }`-декларации — первый срез R4 «Язык» (№213 остаётся резервом остальных срезов R4: taint, dispatch). grammar: `vision_decl` в реестре L8, имя STRING (план §3), 7 полей, громкие unknown-field/duplicate-field с позицией, `vision_ident_val` = IDENT+'-' ради ADR-0124 `gguf-q4`; AST: VisionDecl + VisionPolicy{Safe} + VisionProfile{Fp16,Fp8,GgufQ4}; parser: все поля required (тихих дефолтов нет), значения вне enum = parse-ошибки; semantic: `KNOWN_VISION_MODELS` SSOT в `src/vision/mod.rs` (не feature-gated, рядом с VisionRegistry), steps≥1 (≠8 → audit-warning), width/height ×16 в 256..=4096, дубликат имени = ошибка. Block 0 = fix-forward №237 (runbook golden `860c85b3`, TE 8.05 GB). Диспетчеризация и builtins НЕ тронуты (R4.2); минимальные no-op arms в compiler/execution/modules — вынуждены исчерпывающими матчами (отклонение от diff-инварианта задокументировано в PR). 12 новых тестов (6 parser + 6 semantic); локально fmt/clippy/lib-тесты зелёные, интеграционные — предел контейнера, CI авторитетен | merged, PR #236 |
| №240 | R4.2 | dispatch: `vision { }` → VM → builtins + taint-интеграция (prompts). `Program::vision_decls` (лекало reflex_decls, поля 1:1 с AST через `CompiledVisionDecl::from_ast`), pass1 populate / pass2 без байткода, `Vm::load_program` + interpreter declaration pass регистрируют имя → параметры; `vision_registry` на контексте (Mutex на Interpreter, plain на VM), артефакт `VisionArtifact` = PNG-буфер (R1 `()` снят). `vision_generate(decl, prompt)` — **2-арг** (арность 3→2, урок №234, контракт план §3): резолв декларации с перечнем, runtime re-check `model ∈ KNOWN_VISION_MODELS`, weights из `MLOG_VISION_WEIGHTS_DIR` (нет env/компонента → громкий Err — честный отказ среды, не стаб), клин tokenizer → Qwen3-4B → Z-Image DiT + flow_match_euler_sample (steps+seed из декларации, сигмы = steps+1) → VAE → PNG (encode_png, общий с save_png) → registry → `Value::Vision(id)`; фикс 1024×1024 (латент из DiT-конфига), иное разрешение = громкий Err (R5). `vision_list` — реальные хэндлы сорт по id; `vision_export` = реальные PNG-байты + unsigned-WARN (гейты = R5); `vision_edit/save/load` — стабы R6 не тронуты. Перехваты в interpreter (eval + invoke) и VM (лекало reflex), диспетч-функции общие. Taint: UserInput во 2-й позиции vision_generate → audit-warning VISION_PROMPT_USER_INPUT (лекало n201; arg 0 не флагается; taint на Value::Vision не вводится). Тесты: 13 не-gated (naryad_240_vision_dispatch) + env-gated .mlog e2e без #[ignore] (naryad_240_vision_mlog_e2e) — закрывает обещание №237 Block 3.1; 387 builtins, KNOWN_VISION_MODELS, goldens, ADR-0124 — не тронуты | PR #238 |
| №241 | R5 | security-гейты (ADR-0125) + Provenance MVP: гейты категории A — `VISION_UNSIGNED_EXPORT` (audit Error через `audit_category_a` + runtime backstop с тем же check-id), `MODEL_WEIGHTS_UNSAFE` (audit Error, переиспользуемо в audit.rs — SSOT для Voice), `VISION_POLICY_MISSING` (audit Warning; parser relax policy-only по ADR-0125 SSOT — остальные 6 полей required, enum policy не тронут), raw-ворнинг `VISION_UNSIGNED_EXPORT_RAW` (имя фиксирует №241 — ADR его не задаёт). Provenance: `vision_generate` подписывает всегда — LSB-watermark (MLGV+model-hash32, RGB LSB) + `VisionManifest` (model-id+weights-SHA/`unpinned`, seed, prompt-hash, policy/`unspecified`, timestamp, SHA итогового PNG); `vision_export` = PNG + sidecar `.manifest.json` (unsigned-WARN №240 снят); `vision_export_raw` — явный opt-out (байты as-is, без sidecar). `vision_fetch_weights` — SSRF-guard (`check_url_ssrf`, пиннинг резолвов), allowlist default-deny (`MLOG_VISION_WEIGHTS_ALLOWLIST`), manifest.json-class only, SHA-256 pinning (WeightsManifest переиспользован). Реестр 387→389; 4 контракта-теста категории A = 3 новых + taint №240; сети в тестах нет, веса не нужны | PR #239 |
| №242 | R6.1 | SQLite-персистенция артефактов (первая треть R6, план §7.1; R6 разрезан громко: №242 = save/load, №243 = vision_edit, №244 = LoRA). `src/vision/store.rs` — таблица `vision_artifacts` (name PK / png BLOB / manifest_json NULL⇔None / saved_at RFC 3339; лекало `init_kv_persist`), API save/load/list (тесты и loud-диагностика, builtin-списка поверх БД нет), **дословный manifest-roundtrip** (`Some(m)` → JSON → `Some(m')` по полям, timestamp не перегенерируется; None → NULL → None; битый JSON = громкий Err — тихая деградация в unsigned запрещена), **коллизия имени = громкий Err** (upsert/delete вне скоупа, тихая перезапись = потеря provenance-цепочки), PNG только BLOB в БД программы. Перехваты `vision_save`/`vision_load` в interpreter (eval+invoke) и VM — state-carrying №240/№241 + db_conn; no-db громкий Err с подсказкой `db { url: ... }`; id — сессионный хэндл (монотонный, не персистится), персистентный ключ = name. Last-resort стабы truth-up (лекало export_raw_stub, №242 вместо устаревшей «214/215» R0-эпохи). Roundtrip-контракт (плановая приёмка R6 «roundtrip-тест»): PNG + sidecar байт-в-байт после save→load в новый рег; backstop `VISION_UNSIGNED_EXPORT` жив после персистенции. Реестр 389 не меняется; edit/goldens/weights.rs/ADR-0124/KNOWN_VISION_MODELS не тронуты | PR #240 |

**Free ranges:** буфер №206–208 исчерпан (№206 merged, №207/№208 — долг №206);
№220–229 — резерв Voice-пиллара (`Metalogos_Voice_Pillar_Plan.md`);
№230 — Vision R2 hotfix (golden pinning + PRNG SSOT), №231+ — свободны
(мейнтенанс/хотфиксы до резервирования нового пиллара).

**Rule:** перед стартом любого наряда исполнитель сверяется с этой картой и с
update-нотацией ADR-0121. Новый пиллар обязан зарезервировать диапазон номеров
в своём scope-ADR до первого наряда — не в приватных планах.
