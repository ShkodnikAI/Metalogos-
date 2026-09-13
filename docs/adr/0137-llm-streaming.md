# ADR-0137: LLM streaming — `llm_stream_open/next/close` over `reqwest::blocking`

**Status:** Accepted
**Date:** 2026-09-13
**Naryad:** #275 (issue #311, dispatch #316; спайк-отчёт — `docs/research/naryad-275-streaming-spike.md`)
**Precedent:** ADR-0048 (SmartRouter — стрим встраивается НАД ним, не дублирует выбор), ADR-0096 (block-in-place single-core — стрим не вводит async/таски), ADR-0101 (deferred response — не пересекается), ADR-0114 (`Value::Reflex` opaque handle — лекало `Value::LlmStream`), ADR-0124 (`Value::Vision` — лекало registry в `crate::llm`), ADR-0138 (per-call LLM traces — стрим пишет ОДНУ строку трейса на завершённый вызов), №263 (карта состояний с границей — `LLM_STREAM_REGISTRY` лимитирован).

## Context

Все LLM-вызовы в Metalogos сегодня блокирующие «все-или-ничего»: `call_llm` / `SmartRouter::call` ждут полный ответ, `resp.text()` целиком, таймаут 30s (SmartRouter) / 120s (`call_claude`). Рантайм — single-core block-in-place (ADR-0096), serve — axum/tokio + `spawn_blocking` (ADR-0096 §2, та же модель). Для UX FOSVED (Telegram: длинные ответы департаментов, голосовой контур) прогрессивная выдача — заметное улучшение, но она не должна ломать модель параллелизма.

Issue #311 формулирует спайк-гейт СГ-3 (вердикт-гейт, утверждён владельцем 2026-09-12): спайк решает сам, ADR-0137 пост-фактум. Критерии Go: SSE через `reqwest blocking` без переписывания бэкендов; стрим НАД SmartRouter; каждый `llm_stream_next` блокирует ≤ 1 чанка (ADR-0096 конструктивно); TW/VM parity достижима. Критерии No-Go: нужен rewrite бэкендов или колбэки/таски → честный No-Go, идея → Tier 3 (Python llm_proxy FOSVED, FO-013). Бэкенды без стрима возвращают явную ошибку `STREAM_UNSUPPORTED`, не тихий full-answer.

## Decision (вердикт спайка: GO)

### D1. API — итераторный стиль, никаких колбэков

```mlog
let s = llm_stream_open(prompt, input?)        // -> Struct { handle: LlmStream, model: String, provider: String }
let chunk = llm_stream_next(s.handle)          // -> String (delta) | "" (keep-alive / end-of-stream marker)
let final = llm_stream_close(s.handle)         // -> Struct { tokens: Float, latency_ms: Float, status: String }
```

- `llm_stream_open(prompt: String, input?: String) -> Struct { handle, model, provider }`.
  Выбирает лучшего доступного провайдера через тот же механизм, что `SmartRouter::call` (candidates sorted by `health_score`, circuit breaker, failover=auto только на этапе open). Возвращает opaque handle в `LLM_STREAM_REGISTRY`.
- `llm_stream_next(handle: LlmStream) -> String` — один blocking `Read::read` + парсинг одной SSE-дельты. Возвращает `delta`-строку, или `""` для keep-alive ping, или специальный маркер конца (конвенция конца — `"__end__"`, выбран по образцу существующих soft-failure EOF-паттернов; зафиксировано здесь).
- `llm_stream_close(handle: LlmStream) -> Struct { tokens, latency_ms, status, provider, model }` — drop response, агрегирует usage из финального SSE-event'а, пишет **одну** строку JSONL-трейса (ADR-0138 §D4 — «one line per completed call, not per chunk»), возвращает финальные метаданные.

### D2. Opaque handle — `Value::LlmStream(LlmStreamId)`

`LlmStreamId = u32` (новый тип, лекало `ReflexId`/`VisionId`). Индекс в `LLM_STREAM_REGISTRY` (процесс-глобальная `Lazy<Mutex<HashMap<u32, LlmStreamState>>>`), lives в `crate::llm` (так же как `GLOBAL_SMART_ROUTER`/`GLOBAL_LLM_USAGE` — оба бэкенда ходят через `crate::llm`).

`Value::LlmStream(LlmStreamId)` — новый `Value`-вариант. `Display` = `[LlmStream#<id>]`. `Debug` — ручной, как для `Reflex` (ADR-0114), печатает только `provider` и `model`, не payload. `Serialize`/`Deserialize` — ручные, как для `SecretString`/`Reflex`: сериализуется как маркер `"[LlmStream]"`, чтобы нельзя было выгрузить активный stream в постоянное хранилище. `type_name` = `"LlmStream"`.

### D3. Transport — `reqwest::blocking` + `impl Read` + самописный SSE-парсер

**Отклонение от формулировки issue #311 (зафиксировано громко):** в issue сказано «reqwest blocking `chunk()`». У `reqwest::blocking::Response` **нет** метода `chunk()` (это метод async-`Response`). Вместо него используется `impl std::io::Read for Response` — blocking `read(&mut [u8; N])` читает «один буфер-фулл» из TCP-потока и возвращает управление. Это **семантически эквивалентно** «incremental chunked SSE-чтение» и удовлетворяет духу Go-критерия (без переписывания бэкендов, без колбэков, без async).

Один вызов `llm_stream_next`:
1. Один `Read::read(&mut [u8; 8192])` — blocking, ≤ 8192 байт из TCP-буфера (или меньше).
2. Append в `line_buffer` состояния потока.
3. Парсинг одной полной SSE-дельты из line-buffer (формат: `data: <json>\n\n` или `event: ...\ndata: <json>\n\n`).
4. Если в буфере нет полной дельты — повторить `Read::read` (всё ещё «≤ 1 syscall на chunk», но логически «одна дельта»).
5. Возврат `delta`-строки (или `""` для keep-alive ping; `"__end__"` для конца).

### D4. Stream — параллельный путь, не замена single-shot

`SmartRouter::call` (блокирующий single-shot) **остаётся без изменений**. `call_llm` / learnables / `call_claude` / `call_llm_schema` — не трогаются. Стрим — отдельная API-поверхность:

```rust
impl SmartRouter {
    pub fn stream_open(&self, prompt, input, model_override, timeout) -> Result<LlmStreamState, String>;
    //              ^-- тот же candidates/circuit-breaker/resolved_model, что call()
    //                  но POST body имеет "stream": true (для OpenAI/Anthropic)
    //                  и возвращает Response + инкрементальный SseParser
    pub fn stream_next(state: &mut LlmStreamState) -> Result<String, String>;
    pub fn stream_close(state: LlmStreamState) -> LlmStreamFinal;
}
```

Не-стрим-провайдеры (mock, будущие non-SSE) → `STREAM_UNSUPPORTED` на этапе `stream_open`. Не тихий full-answer.

### D5. Failover в стриме — один провайдер на стрим

Failover в стриме концептуально невозможен (нельзя «переподключиться к другому провайдеру в середине стрима» без потери уже полученных чанков). `stream_open` выбирает лучшего доступного провайдера и держится за него до `close`. Если провайдер упал в середине — `next` возвращает ошибку, пользователь вызывает `close`, при следующем `open` circuit breaker пометит провайдера больным и обойдёт его. Это **не нарушает** SmartRouter-контракт ADR-0048 (failover применяется на этапе выбора, не в середине вызова).

### D6. Лимит одновременных стримов (урок №263)

`LLM_STREAM_REGISTRY` — `HashMap<u32, LlmStreamState>` с **явным верхним пределом** (64 по умолчанию, `METALOGOS_LLM_STREAM_MAX` env override). Превышение → громкая ошибка `STREAM_LIMIT_REACHED`. Это урок наряда №263 (карты состояния без границ переполняются). В serve route-телах лимит защищает от утечки ресурсов при не-закрытых стримах.

### D7. Ликвидация утечек

Не-закрытый стрим при выходе из скопа / ошибке — принудительное закрытие на уровне интерпретатора (лексало: как `Session`/`Conversation` живут в interpreter и drop-аются при выходе из скопа). В serve route-телах лимит (D6) + явный drop при выходе из `spawn_blocking` closure (ADR-0096 §2 — interpreter клонируется в closure, дропается после `.await`).

### D8. Trace — одна строка на завершённый стрим (ADR-0138 §D4)

`llm_stream_close` пишет **одну** `trace_llm_call` строку:
- `latency_ms` = open→close (время полного стрима).
- `gen_ai.usage.input_tokens` / `gen_ai.usage.output_tokens` — агрегированные из финального SSE-event'а (Anthropic: `message_delta` с `usage`; OpenAI: финальный `data: [DONE]` с `usage` в `stream_options.include_usage`; ollama: в каждом чанке — берём из финального).
- `cache` = `"miss"`.
- `provider_alias` = alias провайдера, на котором открыли стрим.

`next`-вызовы **не трейсить per-chunk** — это зафиксировано в ADR-0138 §D4: «streams will emit one line per completed call with summarized usage (the issue's contract), not per chunk». Streaming — это один LLM-call, разложенный во времени.

### D9. TW/VM parity

`LLM_STREAM_REGISTRY` lives в `crate::llm` (как `GLOBAL_SMART_ROUTER` / `GLOBAL_LLM_USAGE`). Body билтина один — оба бэкенда вызывают `crate::llm::stream_via_smart_router` (по лекалу `call_via_smart_router`). Handle = `u32` индекс, не owned-данные — `Send`-bound удовлетворяется тривиально (как для `ReflexId`/`VisionId`).

## Consequences

- ✅ Single-core block-in-place семантика ADR-0096 сохранена — никаких async/тасков/колбэков.
- ✅ Single-shot LLM-путь не меняется — `call_llm`/learnables/`call_claude`/`call_llm_schema` без регрессий.
- ✅ Trace-контракт ADR-0138 §D4 соблюдён — одна строка на стрим.
- ✅ SmartRouter-контракт ADR-0048 не нарушен — failover работает на этапе open.
- ✅ Opaque-handle pattern ADR-0114 переиспользован — `Value::LlmStream(LlmStreamId)`, registry в `crate::llm`.
- ✅ Карта состояния с границей (№263) — `LLM_STREAM_REGISTRY` лимитирован.
- ⚠️ Отклонение от формулировки issue #311: не literally `chunk()`, а `impl Read` + SSE-парсер. Зафиксировано громко в D3.
- ⚠️ Stream — отдельная API-поверхность, не заменяет `call_llm`. Это сознательное решение — предотвращает регрессии застрахованных путей.
- ⚠️ Failover в середине стрима концептуально невозможен (D5) — пользователь сам решает закрыть стрим при ошибке и открыть новый.

## Addendum: Открыто для будущих ADR

- Потоковая отправка в HTTP-ответ (`respond_stream` в serve) — отдельный ADR, не входит в №275. №275 даёт только `llm_stream_*` builtins; интеграция с serve deferred-response (ADR-0101) — отдельная задача.
- Stream для MCP-клиентов (ADR-0132) — не пересекается; MCP stateless, stream там не нужен.
- Stream для `call_llm_schema` (ADR-0133) — не входит в v1 №275; schema-валидация требует полного ответа для валидации JSON-схемы.
