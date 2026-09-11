# ADR-0132: MCP-клиент — ручной JSON-RPC поверх stdio, stateless, вывод с taint `UserInput`

**Status:** Proposed (черновик — awaiting owner approval; утверждение = стоп-гейт 1 диспатча №267–279, разблокирует №268)
**Date:** 2026-09-11
**Naryad:** №267 (research, issue #303) — реализация в №268 (issue #304)
**Research base:** `docs/research/naryad-267-mcp-recon.md` (все факты сняты 2026-09-11: crates.io API, GitHub API, спецификация `modelcontextprotocol/modelcontextprotocol@2026-07-28`, grep по `src/` v0.19.0)

## Context

Metalogos отрезан от главного интеграционного стандарта AI-агентов: MCP в кодовой базе отсутствует полностью, при этом конструкция `tool` (ADR-0054) реализована, LLM-контур (`call_llm` → SmartRouter) и исходящий HTTP (SSRF-пакет №261) есть, а exec-гейты №253-А дают языковой контроль над запуском процессов. ADR-0054 §Future Directions фиксирует только обратный мост (expose tools AS MCP-сервер); прямой мост (Metalogos-программа **вызывает** инструменты внешних MCP-серверов) не спроектирован. Грантовый контекст: секция Proposal_Restack «MCP с языковым security-контролем» — уникальный эдж: ни один mainstream MCP-клиент не имеет taint-статики и exec-гейтов уровня языка.

Актуальная ревизия спецификации — **2026-07-28** — разделила протокол на modern (per-request `_meta`, без `initialize`) и legacy (handshake, ≤ 2025-11-25). Официальный Rust SDK `rmcp` существует, активен и зрел (3.3.0, Apache-2.0).

## Decision Drivers

1. Dependency-дисциплина FEATURE_INTAKE §5: warning на 2 новых крейтах за версию, **hard-лимит 5**.
2. Модель исполнения: TW/VM синхронны, билтины блокирующие; в serve обработчики исполняются в `spawn_blocking` (ADR-0096) — вне async-контекста.
3. Security-модель: exec-гейты №253-А, taint-система (`TaintKind`), audit log — переиспользовать, не дублировать.
4. Interop: максимум существующих MCP-серверов при минимальной протокольной поверхности.

## Decision

### D1. Транспорт — stdio, ревизия спеки 2026-07-28 как reference

`std::process::Child`, newline-delimited JSON-RPC 2.0 по stdin/stdout (спека: сообщения MUST NOT содержать встроенных `\n`; stderr — только логи; shutdown закрытием потока). Отклонено: streamable HTTP — тянет OAuth-контур спеки (`oauth2`, `jsonwebtoken`), SSE, session-менеджмент; Future по реальному use-case удалённого сервера.

Граница interop: клиент v1 говорит **legacy** (`initialize` → `notifications/initialized` → `tools/list`/`tools/call`) — покрывает серверы ревизий 2024-11-05…2025-11-25 и dual-era серверы. Modern-only серверы не поддерживаются (по матрице совместимости спеки legacy-клиент с modern-only сервером несовместим — осознанный разрыв). Modern-era, MRTR, subscriptions, прогресс — Future.

### D2. Реализация — ручной JSON-RPC-клиент, 0 новых зависимостей

`std::process` + `serde_json` (оба уже в дереве), ~300–400 строк.

**Отклонённая альтернатива — `rmcp` 3.3.0 (официальный SDK):**
- зависимости: 9 новых крейтов даже для одного stdio-транспорта (`rmcp`, `futures`, `indexmap`, `tokio-util`, `tracing`, `pin-project-lite`, `process-wrap`, `which`, `pastey`) — **превышение hard-лимита 5**; HTTP-фича добавила бы 15+, включая `reqwest 0.13` рядом с нашей 0.12 (двойной TLS-стек);
- async-модель: tokio-async (подтверждено: `tokio ^1` — неопциональная зависимость ядра) — каждый билтин потребовал бы async-моста из блокирующего контекста (`Handle::block_on`/одноразовый `Runtime`), постоянная интеграционная сложность и вложенный-runtime риск класса, разобранного ADR-0096;
- что SDK даёт честно: сопровождение эволюции протокола апстримом. Цена выбора — эволюция теперь наша работа; принято, т.к. v1-поверхность (D4) узка и покрывается контракт-тестами. Переоткрыть решение при расширении на HTTP/OAuth: там dependency-математика другая.

### D3. Security-дизайн — reuse, не новая политика

- **exec-гейт:** `mcp_call`/`mcp_list_tools` вызывают SSOT `exec_gate(context)` (№253-А) перед spawn: `Process` → `METALOGOS_ALLOW_EXEC=1`, `ServeRoute` → `METALOGOS_SERVE_ALLOW_EXEC=1` (замена, не AND). Код отказа — существующий `EXEC_NOT_PERMITTED`. Каждая spawn-запись — в `METALOGOS_AUDIT_LOG_PATH` с полем `mcp`.
- **Taint-род вывода — `UserInput` (reuse).** Вывод `mcp_call` получает `TaintKind::UserInput` — существующие проверки Category-A/B работают без единого изменения: `UNTRUSTED_TRAINING_DATA` блокирует `reflex_train` на MCP-данных, пайплайны в `respond()`/`write_file()`/`http_post()` покрыты. **Отклонённая альтернатива — новый `ToolOutput`:** честнее по имени, но требует новой ветки «род × sink» в каждой существующей проверке — риск пропуска ветки без единой политики, которая различала бы роды. Заводить `ToolOutput` только одновременно с первой такой политикой (Future). *Явно вынесено владельцу: выбор рода — часть утверждения этого ADR.*
- **`METALOGOS_MCP_ALLOWLIST`:** comma-separated (trim/пустые игнорируются — конвенция №259), точное совпадение argv[0]. Unset — не сужает (действует только exec-гейт); пустая строка — deny all MCP; непустая — только перечисленные, отказ `MCP_NOT_ALLOWLISTED` (код по конвенции ADR-0131). Третий allowlist Metalogos после `METALOGOS_ENV_ALLOWLIST` (№259) и `MLOG_VISION_WEIGHTS_ALLOWLIST` (ADR-0125).
- **Доверие полей:** server info и tool-метаданные (включая descriptions) — без taint; descriptions — текст третьей стороны для LLM-контекста, поверхность промпт-инъекции фиксируется честно как вне зоны taint-системы (программа включает их в контекст явно). Аргументы вызова — доверенные (статически проверенный код .mlog). Вывод — недоверенный (см. выше).

### D4. Scope v1 — tools only, stateless, 2 билтина

- `mcp_call(command, args_json, tool, arguments_json) -> string` и `mcp_list_tools(command, args_json) -> string`. Ресурсы/prompts/sampling, нотификации списков, subscriptions — не входят.
- **Stateless**: spawn → handshake → вызов → shutdown на каждый вызов. Главный аргумент — security-атрибуция один-к-одному: один вызов = один exec-гейт = одна audit-запись (грантовая демонстрация). Цена: +spawn/handshake (~10–50 ms) на вызов. **Отклонённая альтернатива — stateful** (`mcp_start`/`mcp_stop` + реестр дескрипторов): дешевле на длинных цепочках, но требует реестра процессов, очистки сирот, гейт «размывается» по времени; Future — revisit по реальным бенчмаркам №268.

## Consequences

- Положительные: 0 новых зависимостей; единообразие с http_*-билтинами по модели исполнения; exec-гейты и taint работают с первого дня; allowlist даёт развёртываниям минимальную поверхность; audit-лог демонстрирует security-контроль для гранта.
- Отрицательные / принятые риски: протокольная эволюция — наша ответственность; modern-only серверы вне v1; spawn-оверхед на вызов; tool-descriptions как вектор промпт-инъекции закрыт только процедурно (документировано), не технически.
- Реализация (№268) обязана: контракт-тесты на fixture-stdio-сервере (handshake, framing, isError, обрыв потока, garbage в stdout), интеграцию `exec_gate`/audit/allowlist, taint-пин статикой (`reflex_train` на MCP-выводе — ошибка), обновление threat-model и REFERENCE.

## Go/No-Go

**GO** для №268 при утверждении настоящего ADR владельцем (включая D3 taint-род). Оценка 3–5 дней подтверждена разведкой; блокеров нет.
