# ADR-0140: Diagnostic codes — addendum (no-reuse rule + SSOT-registry discipline)

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #288 (issue #302; ADR-only — без кода, аддендум к ADR-0131)
**Amends:** ADR-0131 (`Stable diagnostic codes for mlog check`) — accepted 2026-09-10, наряд №255
**Precedent:** ADR-0131 §Decision (format, единая конвенция, JSON-шейп), `BUILTIN_REGISTRY` (SSOT-реестр, наряд №170), ADR-0114 (`ReflexId` opaque handle — лекало для "registry owns the data, caller carries an index")

## Context

ADR-0131 (принят в наряде №255) зафиксировал основную конвенцию: `UPPER_SNAKE_CASE` коды, единая конвенция для `audit.rs` + `semantic.rs`, JSON-вывод `{code, message, span, severity}`. Постановочная воронка (issue #302, верифицировано постановщиком на main @ `e5eb2c8` — см. блок коллизии в issue) признала вопросы 1–3 (формат, единый реестр, JSON-вывод) **не подлежащими переоткрытию** — ответ дан в ADR-0131.

Однако в ADR-0131 **не зафиксированы** два операционных правила, без которых конвенция остаётся уязвимой к тихой деградации со временем:

1. **No-reuse rule (вечная бронь кодов).** Код, однажды назначенный ошибке/finding, **никогда не переиспользуется** под другим смыслом — даже после удаления ошибки из компилятора. Внешние инструменты (agent-парсеры, `zero fix --plan --json`-стиль repair tools, CI-гейты) могут полагаться на код молча; переиспользование под другим смыслом сломает их без видимой регрессии в самом компиляторе.
2. **SSOT-registry discipline (где живёт реестр, как ловится коллизия, как меняется).** ADR-0131 §Consequences упомянул "registry of all diagnostic codes" как принцип, но не зафиксировал ни место хранения, ни дисциплину поддержания. Без этого реестр либо не появится (конвенция есть, enforcement нет), либо появится в виде разрозненных `match`-рукавов по всему компилятору — то есть ровно той "второй системы", которую ADR-0131 избегал.

Этот аддендум закрывает оба пробела — без переоткрытия вопросов 1–3 и без supersede ADR-0131.

## Decision

### D1. No-reuse rule — коды бронируются навсегда

Код, однажды назначенный диагностике (через `check_id: "CODE"` в `audit.rs` или будущий `diagnostic_code` механизм в `semantic.rs`), **не переиспользуется под другим смыслом никогда**. Удаление ошибки из компилятора **не освобождает** код — он остаётся в реестре с пометкой `deprecated: true` (или `removed: true` с указанием версии удаления), но не переназначается.

**Обоснование.** Внешние инструменты не имеют способа узнать, что код X теперь значит другое — особенно если инструмент запускается в CI против разных версий языка одновременно. Тихое переназначение → тихое ложное срабатывание (или тихое пропускание реальной ошибки). Лучший сценарий — инструмент ломается громко на unfamiliar code; худший — работает молча неправильно.

**Прецедент.** Rust (`E0XXX`), Clang (`-Wxxxx`), TypeScript (`TSXXXX`) — все следуют этому правилу; Rust, например, явно сохраняет `E0XXX`-коды удалённых ошибок в `rustc_error_codes` с пометкой removed.

**Процедура вывода из обращения.** Удаление ошибки из компилятора сопровождается:
- Записью в реестре (см. D2) с пометкой `removed_in: <version>` + `replaced_by: Option<code>` (если ошибка была заменена на новую с другим кодом — например, разделена на две).
- Явной записью в CHANGELOG: `diagnostic code X removed (replaced by Y | deprecated with no replacement)`.
- Никакого переназначения того же строки под новый смысл.

### D2. SSOT-registry discipline — где живёт, как ловится, как меняется

**Место хранения.** Реестр всех диагностических кодов — отдельный модуль `src/diag_codes.rs` (новый, не входит в `audit.rs` — там только Category A/B codes, registry живёт отдельно, как `BUILTIN_REGISTRY` живёт в `registry.rs`, не в `core.rs`). Реестр — `const DIAG_CODES: &[DiagCodeSpec]` (лекало `BUILTIN_REGISTRY`, наряд №170):
```rust
pub struct DiagCodeSpec {
    pub code: &'static str,         // UPPER_SNAKE_CASE
    pub message_template: &'static str, // human-readable, may evolve freely
    pub severity: Severity,         // Error | Warning | Info
    pub category: &'static str,     // "security" | "semantic" | "vm" | ...
    pub removed_in: Option<&'static str>, // None = active; Some("v0.20") = removed
    pub replaced_by: Option<&'static str>, // Some("NEW_CODE") if renamed/split
}
```
Идентичный принцип SSOT: `spec!`-макрос или `const &[]`-литерал, **append-only** (как `BUILTIN_REGISTRY`) — коды не переупорядочиваются, удаление — через `removed_in`/`replaced_by`, не через исключение из списка.

**Как ловится коллизия.** Тест `tests/diag_codes_registry_check.rs` (будет создан в наряде реализации, не здесь):
- **Уникальность кодов** — ни одного дубликата `code` в реестре.
- **Cross-source consistency** — каждый `check_id: "X"` в `audit.rs` и каждый будущий `diagnostic_code: "Y"` в `semantic.rs`/`compiler.rs`/`vm.rs` обязан иметь matching entry в `DIAG_CODES`. Коллизия (код используется в source, но не в реестре, или в реестре, но не в source) — громкая ошибка CI.
- **No-reuse enforcement** — ни один `code` со статусом `removed_in: Some(_)` не используется в активном коде source (поиск через `grep -r "check_id:\s*\"$REMOVED_CODE\""` должен вернуть 0).

**Как меняется.** Добавление кода — append-only + commit-message convention `feat(diag-codes): add CODE for <description>`. Удаление — `removed_in` + `replaced_by` + CHANGELOG entry. **Переименование** (допустимо, но дорого) — `replaced_by: Some("NEW_CODE")`, оба кода в реестре, старый с `removed_in`, новый — активный; CHANGELOG фиксирует.

### D3. Категории кодов — единое пространство, но ярлык категории

Вопрос 2 постановки ("единый реестр или отдельные пространства имён для `semantic.rs`/`audit.rs`") — **не переоткрывается**. ADR-0131 выбрал единую конвенцию; этот аддендум подтверждает, но вводит поле `category` (`"security" | "semantic" | "vm" | "reflex" | "vision" | "voice" | ...`) в `DiagCodeSpec` для machine-readable различения категорий внутри одного реестра. Это решает первоначальное обоснование постановки ("семантически разные категории — разные пространства могут быть оправданы") через **подкатегоризацию внутри единого реестра**, а не через отдельные реестры.

**Пересмотр к "отдельным пространствам имён"** — только явным supersede ADR-0131 по решению владельца, не исполнителя. Текущая позиция: единый реестр + категория-поле достаточно; separation вводила бы ровно ту "вторую систему", которую ADR-0131 избегал.

### D4. Реестр известных кодов на момент аддендума (snapshot)

17 уникальных `check_id` в `audit.rs` на main `d3a1de5` (naryad №283 merge):

| Code | Category | Status |
|---|---|---|
| `CANARY_LEAK` | security (LLM) | active |
| `CSRF` | security (web) | active |
| `HTML_INJECTION` | security (web) | active |
| `MODEL_WEIGHTS_UNSAFE` | security (reflex) | active |
| `OPEN_REDIRECT` | security (web) | active |
| `RATE_LIMIT` | security (web) | active |
| `SANDBOX_COVERAGE` | security (sandbox) | active |
| `SECRETS` | security (sandbox) | active |
| `SECRET_LEAK` | security (audit) | active |
| `SQL_DYNAMIC` | security (audit) | active |
| `TAINT_PASSTHROUGH` | security (audit) | active |
| `TAINT_PERSISTENCE` | security (audit) | active |
| `UNTRUSTED_TRAINING_DATA` | security (reflex) | active |
| `VISION_POLICY_MISSING` | security (vision) | active |
| `VISION_PROMPT_USER_INPUT` | security (vision) | active |
| `VISION_UNSIGNED_EXPORT` | security (vision) | active |
| `VISION_UNSIGNED_EXPORT_RAW` | security (vision) | active |

Все 17 — Category A/B security; ADR-0131 расширяет конвенцию на general `semantic.rs` diagnostics. Реестр (D2) при создании автоматически включает эти 17 как базу; новые `semantic.rs`-коды добавляются append-only.

## Consequences

- **Внешние инструменты** (агент-парсеры, repair tools, CI-гейты, linters) могут полагаться на диагностический код как на **вечный контракт** — код не переназначается под другим смыслом; удаление ошибки не освобождает код для переиспользования.
- **Реестр `DIAG_CODES`** — единственный source of truth; коллизии ловятся на CI (наряд реализации, не этот ADR).
- **Поддержание реестра** — append-only + `removed_in`/`replaced_by` + CHANGELOG entry, идентично `BUILTIN_REGISTRY` дисциплине.
- **Категории внутри реестра** — через `category` поле, не отдельные пространства имён (подтверждение ADR-0131 §Decision).
- **Реализация** (применение кодов ко всем ошибкам `semantic.rs` + создание `src/diag_codes.rs` + `tests/diag_codes_registry_check.rs` + `mlog check --json` flag) — отдельный, следующий наряд после принятия этого ADR. Этот ADR фиксирует конвенцию, не реализацию.
- **ADR-0131 остаётся в силе** — аддендум только добавляет два операционных правила (no-reuse, SSOT-registry discipline), не пересматривает format/единая-конвенция/JSON-шейп.

## Addendum-specific precedents

- **Rust `rustc_error_codes`** — реестр всех error codes, удалённые коды сохраняются с пометкой, never reassigned. Лекало D1.
- **`BUILTIN_REGISTRY` (наряд №170)** — SSOT-реестр `spec!`-макроса, append-only, cross-source consistency через `registry_sync_check.rs`. Лекало D2.
- **`BUILTIN_REGISTRY` reserved numbers (ADR-README.md §Reserved)** — `ADR-0073`/`ADR-0075`/`ADR-0076` reserved to avoid reassignment. Лекало D1 (numbers reserved forever).
