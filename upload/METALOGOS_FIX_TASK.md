# ЗАДАНИЕ: Исправление Metalogos для Fosved Office v2

## Контекст

Metalogos v0.4.0 — AI-язык программирования на Rust. Используется как runtime для Telegram-бота Fosved Office v2 (12 отделов + LLM-координация). Исходники в `/home/z/my-project/metalogos-src/`, бинарник в `/home/z/my-project/bin/mlog`.

Текущее состояние: бот работает через Python-прокси (`llm_proxy.py`) как workaround для багов Металогоса. Цель — починить язык, чтобы прокси стал не нужен.

**ВАЖНО**: Исходники в `metalogos-src/` — это СТАРАЯ версия. Бинарник `bin/mlog` (11MB) собран из НОВОЙ версии с частью фиксов. Нужно работать с исходниками и пересобрать бинарник.

---

## БАГ 1 [CRITICAL] — Отсутствует builtin `query_param()`

**Симптом**: В `app.mlog` строка 63: `let doc_id = query_param("id")`. Бутина не существует в исходниках (`builtins.rs`).

**Где исправить**: `src/builtins.rs` + `src/interpreter.rs` + `src/server.rs`

**Что нужно**:
1. Добавить builtin `query_param(name: String) -> String` в `builtins.rs`
2. Interpreter должен хранить текущий query string (map name->value)
3. В `server.rs::execute_route_body()` — распарсить query string из URI и передать в interpreter
4. Проблема: `execute_route_body()` сейчас НЕ получает URI — сигнатура:
   ```rust
   async fn execute_route_body(state, body_stmts, _headers, raw_body)
   ```
   Нужно добавить параметр `query_string: &str` и передать его из `route_handler()`.

**Реализация**:
- Добавить поле `server_query_params: HashMap<String, String>` в `Interpreter`
- Добавить `pub fn set_server_query_params(&mut self, params: HashMap<String, String>)`
- В `execute_route_body` распарсить `uri.query()` и вызвать `interp.set_server_query_params()`
- В `interpreter.rs` при вызове функции с именем `"query_param"` — искать в `self.server_query_params`
- Зарегистрировать заглушку в `builtins.rs` (реальная логика в interpreter.rs)

**Контракт**:
```
query_param("id") -> "abc123"     // из ?id=abc123
query_param("missing") -> ""       // пустая строка если нет параметра
```

---

## БАГ 2 [CRITICAL] — Отсутствует builtin `respond_html()`

**Симптом**: В `app.mlog` строка 69: `return respond_html("200", html)`. Бутина не существует.

**Где исправить**: `src/builtins.rs`

**Что нужно**: Бuiltin `respond_html(status_code: String, html_content: String) -> Html`
- Парсит status_code (число из строки "200")
- Возвращает `Value::Html(html_content)`

**Важно**: `Value::Html` уже существует в `interpreter.rs` (строка 79) и обрабатывается в `server.rs::value_to_response()` (строка 814):
```rust
Value::Html(html) => AxumHtml(html).into_response()
```
Тип уже поддерживается сервером, нужен только builtin для создания.

**Реализация**:
```rust
fn builtin_respond_html(args: &[Value]) -> Result<Value, String> {
    let status_str = expect_string_arg("respond_html", args, 0)?;
    let html = expect_string_arg("respond_html", args, 1)?;
    // Status is informational only — AxumHtml always returns 200
    // Could validate but HTML content-type is set by AxumHtml wrapper
    Ok(Value::Html(html))
}
```
Зарегистрировать: `funcs.insert("respond_html".to_string(), builtin_respond_html as BuiltinFn);`

---

## БАГ 3 [HIGH] — CSP заголовок блокирует Telegram WebApp

**Симптом**: Миниаппы Telegram WebApp требуют загрузку скрипта `https://telegram.org/js/telegram-web-app.js`, но CSP заголовок `script-src 'self'` это блокирует.

**Где исправить**: `src/server.rs` строка 205

**Текущий код**:
```rust
HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"),
```

**Нужно**:
```rust
HeaderValue::from_static("default-src 'self' https://telegram.org; script-src 'self' https://telegram.org; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' https://api.telegram.org"),
```

**Обоснование**:
- `script-src` — нужен `https://telegram.org` для `telegram-web-app.js`
- `connect-src` — нужен `https://api.telegram.org` для `sendData()` из WebApp
- `img-src data:` — для возможных inline изображений в отчётах
- `default-src` — нужен `https://telegram.org` для frame-embedding

---

## БАГ 4 [HIGH] — `call_llm()` по умолчанию в MOCK-режиме

**Симптом**: `call_llm(prompt, input)` возвращает `[MOCK: prompt | input]` вместо реального LLM-ответа. Env-переменная `METALOGOS_LLM_MOCK` по умолчанию `true`.

**Где исправить**: `src/builtins.rs` строки 1018-1031 + `src/llm.rs` строки 516-518

**Текущий код** (`builtins.rs`):
```rust
let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
    .map(|v| v == "true" || v == "1")
    .unwrap_or(true); // Default: mock mode ON
```

**Варианты исправления**:
- **Вариант A**: Изменить дефолт на `false` (breaking change для тестов)
- **Вариант B**: Добавить отдельный env `METALOGOS_LLM_MOCK` но в `call_llm` builtin проверить ЕЩЁ один флаг `METALOGOS_LLM_REAL` который включает реальный режим
- **Вариант C (рекомендуемый)**: Не трогать дефолт, но в `entrypoint.sh` установить `export METALOGOS_LLM_MOCK=false`

**Рекомендация**: Вариант C — минимальные изменения, не ломает тесты. Но также стоит добавить в документацию.

---

## БАГ 5 [HIGH] — Нет цепочки fallback провайдеров LLM

**Симптом**: `llm.rs::RealLlm` поддерживает только ОДИН провайдер за раз. Fosved нужна цепочка: GLM 4.6 -> GLM 5.1 -> DeepSeek -> Groq -> Claude.

**Где исправить**: `src/llm.rs`

**Текущая архитектура**:
- `Provider` enum: Anthropic | OpenAI | Ollama
- `RealLlm` — один провайдер, один ключ, одна модель
- `create_llm_backend()` — создаёт ОДИН backend

**Что нужно**:
1. Добавить провайдеры: `GLM`, `DeepSeek`, `Groq` (все OpenAI-compatible API)
2. Реализовать `FallbackLlm` — обёртку над `Vec<RealLlm>`:
   ```rust
   struct FallbackLlm {
       backends: Vec<RealLlm>,
   }
   impl LlmBackend for FallbackLlm {
       fn call(&self, prompt: &str, input: &str) -> Result<String, String> {
           let mut errors = Vec::new();
           for backend in &self.backends {
               match backend.call(prompt, input) {
                   Ok(response) => return Ok(response),
                   Err(e) => { errors.push(format!("{}: {}", backend.name(), e)); }
               }
           }
           Err(format!("All LLM providers failed: {}", errors.join(" | ")))
       }
   }
   ```
3. Конфигурация через env-переменные (уже используются в прокси):
   - `GLM_46_API_KEY`, `GLM_46_MODEL=glm-4-plus`, `GLM_46_URL=https://open.bigmodel.cn/api/paas/v4`
   - `GLM_51_API_KEY`, `GLM_51_MODEL=glm-z1-plus`
   - `DEEPSEEK_API_KEY`, `DEEPSEEK_MODEL=deepseek-chat`, `DEEPSEEK_URL=https://api.deepseek.com/v1`
   - `GROQ_API_KEY`, `GROQ_MODEL=llama-3.3-70b-versatile`, `GROQ_URL=https://api.groq.com/openai/v1`
   - `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL=claude-sonnet-4-20250514`

**Ключевой момент**: GLM, DeepSeek, Groq — все используют OpenAI-compatible формат (`/v1/chat/completions` с `Authorization: Bearer`). Можно реализовать один `call_openai_compatible()` метод.

---

## БАГ 6 [HIGH] — `call_llm()` не разделяет system/user сообщения

**Симптом**: `call_llm(prompt, input)` склеивает в `"prompt\n\nInput: input"`. Нельзя задать системный промпт отдельно от пользовательского сообщения. Для Fosved критично: каждый отдел имеет свой системный промпт.

**Где исправить**: `src/llm.rs` + `src/builtins.rs`

**Текущий код** (`llm.rs` строки 274):
```rust
"content": format!("{}\n\nInput: {}", prompt, input)
```

**Что нужно**: Изменить `LlmBackend::call()` сигнатуру:
```rust
fn call(&self, system: &str, user: &str) -> Result<String, String>;
```
И в JSON-запросе правильно разделить:
```json
{
  "messages": [
    {"role": "system", "content": "...system prompt..."},
    {"role": "user", "content": "...user message..."}
  ]
}
```

Для Anthropic — использовать отдельное поле `"system"` вместо сообщения с ролью system.

**Обратно-совместимость**: Оставить старую сигнатуру как `call_legacy()` для learnable patterns (у которых только prompt + input).

---

## БАГ 7 [MEDIUM] — Blocking HTTP на async runtime (tokio)

**Симптом**: `http_post()` и `call_llm()` используют `reqwest::blocking::Client` внутри tokio async runtime. Это блокирует потоки tokio, что при concurrent-запросах ведёт к thread starvation и таймаутам.

**Где исправить**: `src/builtins.rs` (`builtin_http_post`), `src/llm.rs` (`RealLlm`)

**Текущий код** (`builtins.rs` строка 752):
```rust
let client = reqwest::blocking::Client::builder()
    .timeout(std::time::Duration::from_secs(30))
    .build()
```

**Что нужно**:
- Обернуть blocking-вызовы в `tokio::task::block_in_place()` (если runtime позволяет) ИЛИ
- Переписать на async `reqwest::Client` и использовать `tokio::runtime::Handle::current().block_on()` ИЛИ
- Проще всего: использовать `std::thread::spawn` + channel для неблокирующего ожидания

**Вариант с минимальными изменениями**: В `execute_route_body()` (который уже async) обернуть весь вызов интерпретатора в `tokio::task::spawn_blocking`:
```rust
let result = tokio::task::spawn_blocking(move || {
    // создать interpreter, выполнить stmts, вернуть Response
}).await.map_err(|e| format!("Task join error: {}", e))?;
```
Это изолирует ALL blocking-операции интерпретатора от tokio runtime.

---

## БАГ 8 [MEDIUM] — `http_post()` не возвращает статус-код при ошибках 4xx/5xx

**Симптом**: При `status >= 400` `http_post()` возвращает `Err(...)`, что крашит весь route handler. Нельзя обработать ошибку gracefully.

**Где исправить**: `src/builtins.rs` строки 790-794

**Текущий код**:
```rust
if status >= 400 {
    return Err(format!("http_post() returned status {}: {}", status, resp_body));
}
```

**Что нужно**: Вернуть `Value::HttpResponse { status, body: resp_body }` вместо ошибки. Пусть вызывающий код сам решает, как обрабатывать.

**Альтернатива**: Добавить параметр `throw_on_error: Bool` (5-й аргумент). Если false — возвращать HttpResponse при любой ошибке.

---

## БАГ 9 [MEDIUM] — Нет builtin `http_get()` с заголовками

**Симптом**: `http_get(url)` не поддерживает передачу заголовков (Authorization и т.д.). Аналогично старому `http_post()` до фикса.

**Где исправить**: `src/builtins.rs` (`builtin_http_get`)

**Текущий код**: Только `url` как аргумент, нет заголовков.

**Что нужно**: Добавить опциональный 2-й параметр (Bearer token или Struct с заголовками), аналогично `http_post()`.

---

## БАГ 10 [LOW] — Несогласованность версий исходников и бинарника

**Симптом**: В `metalogos-src/` есть фиксы UTF-8 (char-based `len`/`substring`/`index_of`), фиксы `} else {` в парсере, фиксы `http_post()` с 4-м параметром. Но отсутствуют `query_param()` и `respond_html()`. Бинарник `bin/mlog` имеет свою версию.

**Что нужно**:
1. Определить, какая версия исходников актуальна (вероятно, бинарник собран из более новой версии)
2. Убедиться, что ВСЕ фиксы из бинарника есть в исходниках
3. Добавить недостающие фиксы (баги 1-9 из этого документа)
4. Пересобрать бинарник: `cargo build --release`
5. Заменить `/home/z/my-project/bin/mlog`

---

## БАГ 11 [LOW] — `call_llm()` имеет env-переменную `METALOGOS_LLM_MOCK` но прокси использует `METALOGOS_LLM_MOCK` — разные имена

**Симптом**: В `builtins.rs` строка 1019: `METALOGOS_LLM_MOCK`. В `llm.rs` строка 516: тоже `METALOGOS_LLM_MOCK`. Ок, одинаково. Но для включения реального режима нужен `METALOGOS_LLM_MOCK=false`.

**Что нужно**: Добавить поддержку `METALOGOS_LLM_PROVIDER` для выбора провайдера и `METALOGOS_API_KEY` для ключа (уже есть в `llm.rs` но не используется в `call_llm` builtin правильно).

---

## ПРИОРИТЕТ ИСПРАВЛЕНИЙ

| Порядок | Баг | Причина приоритета |
|---------|-----|-------------------|
| 1 | Баг 1: `query_param()` | Блокирует /report и /miniapp роуты |
| 2 | Баг 2: `respond_html()` | Блокирует HTML-ответы |
| 3 | Баг 3: CSP для Telegram | Блокирует миниаппы |
| 4 | Баг 7: Blocking на tokio | Блокирует concurrent-запросы |
| 5 | Баг 5: Fallback LLM | Нужен для убирания прокси |
| 6 | Баг 6: System/user split | Нужен для отделов |
| 7 | Баг 4: Mock mode default | Нужен для убирания прокси |
| 8 | Баг 8: http_post errors | Улучшение стабильности |
| 9 | Баг 9: http_get headers | Полезно но не критично |
| 10 | Баг 10-11: Version sync | Housekeeping |

---

## ФАЙЛЫ ДЛЯ ИСПРАВЛЕНИЯ

- `src/builtins.rs` — баги 1, 2, 4, 8, 9
- `src/server.rs` — баги 1, 3, 7
- `src/interpreter.rs` — баг 1 (query_params storage)
- `src/llm.rs` — баги 5, 6
- `Cargo.toml` — проверить зависимости (все ли нужные crate есть)
- `entrypoint.sh` — баг 4 (env vars)

---

## ТЕСТИРОВАНИЕ

После каждого фикса:
1. `cargo build --release` — проверить компиляцию
2. `cargo test` — запустить все тесты
3. Проверить конкретный фикс:
   - Баг 1-2: `mlog run test_bot.mlog` (если есть тест)
   - Баг 3: Запустить сервер, проверить CSP header в response
   - Баг 5-6: `METALOGOS_LLM_MOCK=false METALOGOS_LLM_PROVIDER=openai METALOGOS_API_KEY=... cargo test -- --ignored`
4. Итоговый тест: заменить `bin/mlog`, задеплоить, проверить `/status` и `/test-llm` в Telegram

---

## ЗАМЕТКИ ПО АРХИТЕКТУРЕ

### Что РАБОТАЕТ в текущем исходнике:
- `len()`, `substring()`, `index_of()`, `char_at()` — Unicode-aware (char-based) ✅
- `__replace()` — корректно работает с UTF-8 ✅
- `} else {` — поддерживается в грамматике ✅
- `import` — загружает patterns в интерпретатор ✅
- `clone_definitions_into()` — копирует patterns, learnables, structs, rules ✅
- `json_body()` — работает в server context через `server_json_body` ✅
- `memorize()`/`recall()`/`forget()` — работают как callable functions ✅
- `http_post()` — поддерживает 4-й параметр (Bearer token или Struct headers) ✅
- `call_claude()` — работает с Anthropic API ✅
- `parse_json()` — парсит JSON в Value ✅
- `http_get()` — базовая реализация есть ✅
- `now()` — возвращает Unix timestamp ✅

### Что НЕ работает:
- `query_param()` — отсутствует ❌
- `respond_html()` — отсутствует ❌
- `call_llm()` — mock по умолчанию, нет fallback, нет system/user разделения ❌
- CSP — блокирует внешние скрипты ❌
- Blocking HTTP на async runtime ❌