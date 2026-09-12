# ADR-0134: sqlite-vec как KNN-ускоритель semantic recall — вердикт спайка №271: Go

**Status:** Accepted (вердикт-гейт диспатча #316: решён исполнителем по спайку 2026-09-12, как утверждено механикой гейтов)
**Date:** 2026-09-12
**Naryad:** #271 (issue #307); спайк-отчёт — `docs/research/naryad-271-sqlite-vec-spike.md`
**Precedent:** ADR-0104 (feature-gating с измеренным влиянием), ADR-0116 (SQLite как носитель состояния памяти), FEATURE_INTAKE §4-C/§5, MEMORY_ROADMAP Phase 4

## Context

MEMORY_ROADMAP Phase 4 (L2 scenario grouping: `scenarios` / `scenario_members`, `group_scenarios()`, `recall_from_scenario()`) спроектирована вокруг центроидных эмбеддингов и KNN. Текущая реализация semantic recall — полный скан: SELECT всех строк `memories`, decode `embedding BLOB` (LE f32) и скалярный `cosine_similarity` (`src/embeddings.rs`) в 4 местах `src/memory_store.rs`. На 100K записей это 112 мс на запрос (замер спайка, 2 vCPU Xeon) — на границе интерактивности и тормоз роста для Phase 4.

sqlite-vec (asg017, MIT, крейт `sqlite-vec 0.1.9` — последняя стабильная; 2.8M загрузок) — каноничный SQLite-ускоритель векторного поиска: vec0 virtual table, C-исходник ~100 KB, статическая линковка через cc. Постановка №271 требовала спайка с объективными Go-критериями: дельта бинарника < 2 MB, KNN 10K×384 < 50 ms, 3 ОС без ручных флагов. Код спайка остаётся на ветке `naryad-271-sqlite-vec` (draft PR #334 не мержится); в main попадают только этот ADR и отчёт.

## Decision

### D1. Go — sqlite-vec принимается как KNN-движок для semantic recall

Все критерии выполнены с запасом: дельта бинарника **0.15 MB** (probe-замер с реальным использованием; критерий < 2 MB), KNN 10K×384 k=10 — **4.41 ms** против 8.78 ms текущего пути (критерий < 50 ms), вставка ~81–84 тыс. векторов/с, Linux/macOS/Windows собираются без ручных флагов (Linux локально + smoke; macOS/Windows — джобы `--features portable --all-targets` на ветке-спайке). Корректность: top-1 vec0 совпадает с полным скалярным сканом на 1K/10K/100K (smoke + встроенная верификация бенчмарка). №272 реализует `embed` / `vec_store` / `vec_search` поверх sqlite-vec.

### D2. Интеграция — статическая регистрация, без `load_extension`

Расширение регистрируется как auto-extension (`sqlite3_auto_extension` + `sqlite3_vec_init`) до открытия соединения; feature `load_extension` rusqlite не вводится. Это исключает динамическую загрузку `.so`/`.dll` целиком — весь vec0-код статически линкуется в бинарник, платформенных проблем загрузки расширений нет. Гибридный recall сохраняется: BM25 остаётся на FTS5, vec0 заменяет только cosine-половину (RRF-слияние не меняется).

### D3. Фича `vec` — off-by-default, measured impact по ADR-0104

`vec = ["dep:sqlite-vec"]`, вне `default`/`full`. При мерже №272 фича включается в `portable` (дельта 0.15 MB это позволяет; кросс-ОС CI покрывает её каждым прогоном). Числа спайка фиксируются в FEATURES-учёте по образцу ADR-0104: +0.15 MB бинарника, +1 зависимость, ~2× на KNN.

### D4. Честная граница: brute-force, не ANN; wasm-граница зафиксирована

sqlite-vec 0.1.9 — линейный скан с SIMD, не ANN-индекс; выигрыш ~2× (SIMD-ядро C + скан внутри SQLite без материализации таблицы в Rust). Браузерный путь (`wasm32-unknown-unknown`) через rusqlite невозможен — cc-тулчейна для wasm нет, в build.rs libsqlite3-sys ветки нет (есть только ветка wasm32-wasip1, требующая wasi-sdk); сам sqlite-vec wasm-совместим и доступен для Go-стека Playground через SQLite-WASM (согласовано с вердиктом №278). Точка пересмотра: 100K+ записей или появление ANN/квантования в upstream sqlite-vec.

## Consequences

- Положительные: Phase 4 получает in-DB KNN без выгрузки таблицы; recall на 100K — 58 мс вместо 112 мс; память остаётся одним файлом SQLite (ADR-0116 не нарушается); зависимость +1 (лимит FEATURE_INTAKE §4-C — 5, не превышен).
- Отрицательные/риски: C-зависимость в дереве сборки (cc), как уже принято с libsqlite3-sys; вертикаль скорости ограничена brute-force природой vec0 (осознанно, D4).
- Нейтральные: спайк-код (bench + smoke) остаётся на ветке-спайке как артефакт доказательства; №272 переносит контракт smoke-теста в постоянный CI при включении фичи.
