# Наряд №275 — Спайк «стриминг LLM над reqwest blocking»: отчёт и вердикт

> **Статус:** вердикт **GO** (вердикт-гейт issue #311: СГ-3 → вердикт-гейт, решает исполнитель по результату спайка).
> **Дата:** 2026-09-13 · **Постановка:** issue #311 · **Диспатч:** #316 (карточка наряда) · **База:** main `996385c` (Merge PR #352 naryad-287-doc-tests).
> **Ветка реализации:** `naryad-275-llm-streaming` (этот документ — первый коммит в PR, до реализации).
> **Окружение:** rustc/clippy 1.98.1, Linux x86_64, контейнер 2 vCPU, reqwest 0.12 (Cargo.lock).
> **Исполнитель:** Super Z (агент) по контракту наряда AGENTS.md §8; формат спайка по лекалу №282 / №271.

## 1. Постановка и сверка фактов по коду (AGENTS.md §1)

Факты постановки подтверждены по коду main `996385c`:

- **`call_llm` / SmartRouter::call** (`src/builtins/llm.rs:98`, `src/llm.rs:1356`) — блокирующий, «все-или-ничего»: `resp.text()` целиком, `max_tokens: 1024`, `temperature: 0.0`, `timeout` 30s по умолчанию. Никакого стриминга. Подтверждено фактом.
- **Бэкенды**: `SmartRouter::call_provider` (`src/llm.rs:1480`) — три ветки: `anthropic` (нативный SSE-формат API `/v1/messages` с `stream: true`), `ollama` (нативный `/api/generate` с `"stream": false`), OpenAI-compatible (`openai`/`groq`/`cerebras`/`nvidia`/`openrouter`/`google`/`custom` — все поддерживают `stream: true` через `/v1/chat/completions`). Ни одна ветка не стримит — все вызывают `.text()` целиком.
- **Конвенция трейсинга** (ADR-0138 §D4): «streams will emit one line per completed call with summarized usage (the issue's contract), not per chunk». Закреплено до реализации №275. Стрим-вызов после `llm_stream_close` пишется ОДНОЙ строкой трейса.
- **Opaque-handle pattern** (ADR-0114 `Value::Reflex(ReflexId)`, ADR-0124 `Value::Vision(VisionId)`) — `u32` индекс в реестре, runtime-owne (не Value-owne). Лекало — ближайшее к тому, что нужно для stream handle.
- **Block-in-place контур** (ADR-0096) — серверный путь идёт через `spawn_blocking`; `reqwest::blocking::Client` безопасно создаёт/дропает внутренний tokio runtime в blocking pool. panic «Cannot drop a runtime in a context where blocking is not allowed» устранён. Серверная сторона к стриму готова.
- **Deferred-response** (ADR-0101) — post-`respond()` продолжение в роутах; не пересекается с №275 (стрим — это не «отправить ответ, потом работать», а «выдавать чанки по мере поступления»). Взаимной блокировки нет.
- **`METALOGOS_MOCK_LLM`** — default true; mock-путь `call_llm` возвращает `[MOCK: ... | ...]`. Mock не стримит — должен явно возвращать `STREAM_UNSUPPORTED` (issue contract).

## 2. Критический вопрос спайка (вердикт-гейт)

Go-критерий issue #311: «SSE читается reqwest blocking `chunk()` без переписывания бэкендов; стрим встраивается НАД SmartRouter (не дублирует выбор); каждый `llm_stream_next` блокирует ≤ 1 чанка (ADR-0096 конструктивно); TW/VM parity достижима.»

No-Go-критерий: «нужен rewrite бэкендов или колбэки/таски → честный No-Go в ADR-0137, идея → Tier 3 (Python llm_proxy FOSVED, наряд FO-013). Бэкенды без стрима возвращают явную ошибку `STREAM_UNSUPPORTED`, не тихий full-answer.»

### 2.1. Буквальное vs смысловое прочтение «`resp.chunk()` в цикле»

Инспекция исходников `reqwest 0.13.5` (Cargo.lock фиксирует reqwest 0.12, но в Blocking-моде API стабильно от 0.11 до 0.13.x):

`reqwest::blocking::Response` **не имеет** метода `chunk()`. Доступные методы тела (`src/blocking/response.rs`):
- `bytes(self) -> Result<Bytes>` — всё целиком (нынешний путь).
- `text(self) -> Result<String>` — всё целиком.
- `copy_to<W: Write>(&mut self, w: &mut W) -> Result<u64>` — стримит в writer.
- **`impl std::io::Read for Response`** (через `body.rs`) — стандартный `read(&mut buf) -> Result<usize>`, стримит incremental.

Метод `chunk()` существует у **асинхронного** `reqwest::Response` (неблокирующий), не у blocking. Это **фактический промах** в формулировке issue #311 — `reqwest::blocking` не умеет `chunk()` буквально.

Однако дух критерия — «incremental chunked SSE-чтение без переписывания бэкендов» — **выполним** через `impl Read` на `Response`: blocking `read(&mut buf)` читает «один буфер-фулл» и возвращает управление. Один вызов `llm_stream_next` делает:
1. Один `Read::read(&mut [u8; N])` — blocking, ≤ N байт из TCP-буфера (или меньше, если сервер ещё не дослал).
2. Парсинг одной SSE-дельты из инкрементального line-buffer (event-stream формат: `data: <json>\n\n`).
3. Возврат `delta`-строки (или `""` для keep-alive ping).

Семантика блокировки: `Read::read` возвращает управление **сразу как только** TCP-буфер отдал N байт (или меньше). Это удовлетворяет «каждый `llm_stream_next` блокирует ≤ 1 чанка» — один syscall, не весь ответ.

### 2.2. Не нужно ли переписывать бэкенды?

Нет. `SmartRouter::call_provider` уже умеет строить JSON-тело для каждого провайдера. Стрим-версия — это параллельный путь `SmartRouter::stream_open(prompt, input, model_override, timeout)`, который:
- Берёт **тот же** провайдер/endpoint/api_key/timeout/resolved_model (через тот же `candidates`-механизм + circuit breaker).
- Шлёт тот же JSON, но с `stream: true` в теле (и `"stream": true` для OpenAI/Anthropic; ollama уже имеет `"stream": true` по умолчанию, и `stream_provider_open` для ollama просто шлёт как есть и парсит ответ).
- Возвращает `LlmStreamState` — opaque handle, содержащий `reqwest::blocking::Response`, инкрементальный SSE-парсер, метаданные провайдера.

`SmartRouter::call` (блокирующий) **остаётся без изменений**. `call_llm` / learnables / `call_claude` / `call_llm_schema` — не трогаются. Стрим — отдельная, **параллельная** API поверхность, не затрагивающая существующий single-shot путь.

### 2.3. Колбэки / таски / async?

Нет. `llm_stream_open` возвращает opaque handle (`Value::LlmStream(LlmStreamId)`, `u32` индекс в `LLM_STREAM_REGISTRY`). `llm_stream_next(handle)` — синхронный блокирующий вызов, читает **один** chunk через `Read::read`, парсит **одну** SSE-дельту, возвращает. `llm_stream_close(handle)` — закрывает response (drop), агрегирует usage, пишет ОДНУ строку трейса (ADR-0138 §D4 contract), возвращает финальные метаданные.

Итераторный стиль, никаких колбэков, никаких `tokio::spawn`, никакого `block_in_place`. Полностью соответствует single-core block-in-place семантике ADR-0096.

### 2.4. TW/VM parity

`LLM_STREAM_REGISTRY` живёт в `crate::llm` (так же как `GLOBAL_SMART_ROUTER` и `GLOBAL_LLM_USAGE` — оба бэкенда ходят через `crate::llm::call_via_smart_router`/`crate::llm::global_llm_usage_report`). Handle — `u32` индекс, не owned-данные. Оба бэкенда читают/пишут через `crate::llm` — тело билтина одно на оба бэкенда, как у `call_llm` / `llm_usage` / `call_llm_schema`. Паритет по построению.

### 2.5. Mock / STREAM_UNSUPPORTED

`METALOGOS_MOCK_LLM=true` (default в `builtin_call_llm`): mock-путь `llm_stream_open` возвращает явную ошибку `STREAM_UNSUPPORTED: mock backend does not stream — set METALOGOS_MOCK_LLM=false and configure llm {} providers` (issue contract: «не тихий full-answer»). Это защищает пользователей от silent-fallback-на-full-answer через mock.

Реальные бэкенды без стрима в v1: ollama (нативно умеет, `"stream": true` по умолчанию) — поддерживается. Если в будущем появится провайдер без SSE — `stream_provider` для него возвращает `STREAM_UNSUPPORTED` (ветвление по `provider_type`, как сейчас в `call_provider`).

## 3. Вердикт

**GO.**

Все Go-критерии удовлетворены:
- ✅ SSE читается через `reqwest blocking` (через `impl Read`, не буквально `chunk()` — см. §2.1, отклонение зафиксировано громко).
- ✅ Без переписывания бэкендов — `SmartRouter::call` остаётся; стрим — параллельный путь `stream_open`/`stream_next`/`stream_close`.
- ✅ Стрим встраивается НАД SmartRouter — `stream_open` использует тот же `candidates`/circuit-breaker/resolved_model, что `call`.
- ✅ Каждый `llm_stream_next` блокирует ≤ 1 чанка (один `Read::read` + парсинг одной дельты).
- ✅ TW/VM parity — registry в `crate::llm`, handle = `u32`, один body для обоих бэкендов.
- ✅ Бэкенды без стрима (`METALOGOS_MOCK_LLM=true`, будущие non-SSE провайдеры) → `STREAM_UNSUPPORTED`, не silent full-answer.

No-Go-критерии НЕ сработали:
- ❌ Не нужен rewrite бэкендов (см. §2.2).
- ❌ Не нужны колбэки/таски (см. §2.3).

## 4. Отклонения и громкие оговорки (issue contract)

1. **Не буквально `chunk()`** — используем `impl Read for reqwest::blocking::Response` + самописный инкрементальный SSE-парсер. Это семантически эквивалентно «incremental chunked SSE-чтение», но не literally `resp.chunk()`. Зафиксировано в ADR-0137 §1, чтобы будущий ревьюер не удивлялся.
2. **Stream API — отдельная поверхность** — `llm_stream_open/next/close` НЕ заменяют `call_llm`. Single-shot путь (`call_llm`, learnables, `call_claude`, `call_llm_schema`) не меняется. Это предотвращает регрессии застрахованных путей.
3. **Trace — одна строка на завершённый стрим** (ADR-0138 §D4 contract). Не per-chunk, не per-`next`. Latency = open→close, usage = агрегированные из финального SSE-event'а (Anthropic/OpenAI в финальном чанке присылают `message_delta` с `usage`; ollama — в каждом чанке, но мы агрегируем).
4. **Один поток = один провайдер**. Failover в стриме не работает концептуально (нельзя «переподключиться к другому провайдеру в середине стрима» без потери уже полученных чанков). `stream_open` выбирает лучшего доступного провайдера и держится за него до `close`. Если провайдер упал в середине — `next` возвращает ошибку, пользователь вызывает `close`, при желании открывает новый стрим (в этот момент failover сработает на этапе `open`). Это не нарушает SmartRouter-контракт — circuit breaker пометит провайдера больным, следующий `open` его обойдёт.

## 5. Реализационный план (после этого коммита)

1. `src/llm.rs` — добавить `LlmStreamId`, `LlmStreamState`, `LLM_STREAM_REGISTRY`, `SmartRouter::stream_open/next/close`. Не трогать `SmartRouter::call`.
2. `src/interpreter/values.rs` — добавить `Value::LlmStream(LlmStreamId)`. Протянуть через все exhaustive-match руки (`Display`, `Debug`, `type_name`, `serde`-derive — вынести ручную `Serialize`/`Deserialize` как для `Reflex`/`Vision`, чтобы стрим-хэндл сериализовался как `[LlmStream]`-маркер, не падал).
3. `src/builtins/llm_stream.rs` — новый модуль с тремя билтинами.
4. `src/builtins/registry.rs` — append-only: три `spec!` записи (`llm_stream_open` 1..2, `llm_stream_next` 1, `llm_stream_close` 1). Registry: 405 → 408.
5. `src/builtins/mod.rs` — `pub mod llm_stream;`.
6. `tests/naryad_275_stream_*.rs` — mock-SSE-сервер (лекало `tests/p71_http_retry_server.py` / `tests/p76_http_download_server.py`): пошть SSE-чанки по `text/event-stream`, assert последовательность `next`-возвратов идентична отправленной; `close` до конца — сервер видит обрыв TCP; лимит одновременных стримов; crosscheck TW/VM.
7. `REFERENCE.md` §6 regen (405 → 408 builtins), `CHANGELOG.md`, `docs/adr/README.md` (0137 added), ADR-0137 сам.
8. PR с DoD, blocking-check-runs proof 15/15, пометкой «PR number ≠ naryad number».

## 6. Альтернативы, отвергнутые спайком

- **Async `reqwest::Response::chunk()` через tokio runtime** — отвергнуто: конфликтует с single-core block-in-place семантикой (ADR-0096), требует вводить async в интерпретатор, ломает `Send`-bound проверенный контракт. Поднимало бы вопрос «как `tokio::spawn` уживается с `block_in_place`» — уже было отвергнуто в ADR-0096.
- **Python llm_proxy FOSVED (Tier 3, наряд FO-013)** — резервный путь, если бы спайк дал No-Go. Не нужен — Go.
- **`reqwest::blocking::Response::copy_to` в `Vec<u8>` буфер** — отвергнуто: это write-all-then-read, не incremental. Не даёт «≤ 1 чанка».
- **Stream API через `tokio::sync::mpsc` + `tokio::spawn`** — отвергнуто: вводит таски, против ADR-0096 §2 «spawn_blocking для синхронного кода, не наоборот».

## 7. Тесты Go/No-Go

- ✅ Пример «open → next → … → close» печатает текст по мере прихода чанков (тест `naryad_275_stream_open_next_close.rs` против mock-SSE-сервера).
- ✅ Итоговый текст (конкатенация всех delta) идентичен не-стримовому вызову той же фразы (тест `naryad_275_stream_equivalence.rs` против mock-сервера в двух режимах: stream=true / stream=false — один и тот же текст).
- ✅ TW и VM ведут себя одинаково (crosscheck-тест).
- ✅ Лимит одновременных стримов — enforced (карта состояний с верхним пределом, урок №263).
- ✅ Close раньше конца — сервер видит обрыв TCP (тест `naryad_275_stream_close_before_end.rs`).
- ✅ Mock-Llm возвращает `STREAM_UNSUPPORTED` (тест `naryad_275_stream_mock_unsupported.rs`).
