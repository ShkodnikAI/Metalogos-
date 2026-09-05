# METALOGOS — Справочник языка (Reference)

> **Версия:** 0.17.0
> **Единый источник истины** для разработчиков, пишущих на Металогосе.
> Содержит полный список встроенных функций с сигнатурами, типами, описанием и примерами,
> а также справочник по синтаксису, типам данных и CLI.

---

## Содержание

1. [CLI](#1-cli-команды)
2. [Типы данных](#2-типы-данных)
3. [Синтаксис](#3-синтаксис)
4. [Встроенные функции (Builtins)](#4-встроенные-функции-builtins)
   - [Строки](#41-строковые-функции)
   - [Числа и математика](#42-числа-и-математика)
   - [Коллекции (List)](#43-коллекции-list)
   - [Преобразование типов](#44-преобразование-типов)
   - [LLM и AI](#45-llm-и-ai)
   - [HTTP](#46-http)
   - [JSON](#47-json)
   - [Файловый I/O](#48-файловый-io)
   - [Память (KV-хранилище)](#49-память-kv-хранилище)
   - [Сессионная память](#410-сессионная-память)
   - [Шифрование и безопасность](#411-шифрование-и-безопасность)
   - [Аутентификация](#412-аутентификация)
   - [HTTP-сервер (mlogserver)](#413-http-сервер-mlogserver)
   - [Шаблоны (Templates)](#414-шаблоны-templates)
   - [Базы данных](#415-базы-данных)
   - [Боты (Telegram/Discord)](#416-боты-telegramdiscord)
   - [Прочее](#417-прочее)
   - [PDF (pdf-inspector)](#418-pdf-наряд-48-pdf-inspector)
   - [SVG-графика и диаграммы](#419-svg-графика-и-диаграммы-наряды-77-92-adr-0102)
   - [Email, календарь, контакты](#420-email-календарь-контакты-наряды-mlg-456)
5. [Объявления верхнего уровня](#5-объявления-верхнего-уровня)
6. [Stdlib (стандартная библиотека)](#6-stdlib-стандартная-библиотека)
7. [Changelog](#7-changelog-кратко)

---

## 1. CLI команды

Бинарник `mlog` поддерживает следующие команды:

| Команда | Описание |
|---------|----------|
| `mlog run <file.mlog>` | Выполнить программу (или `.mbc` байткод) |
| `mlog repl` | Интерактивная сессия (REPL) с историей команд |
| `mlog check <file.mlog>` | Семантический анализ без выполнения |
| `mlog serve <file.mlog>` | Запустить HTTP-сервер из `mlogserver`/`server` блока |
| `mlog compile <file.mlog>` | Скомпилировать `.mlog` в `.mbc` байткод |
| `mlog eval <file.mlog>` | Запустить `eval`-блоки (тестирование learnable patterns) |
| `mlog resume <file.mlog> --flow=<name> --from=<checkpoint>` | Возобновить flow с контрольной точки |
| `mlog audit <file.mlog>` | Статический security-аудит без выполнения |
| `mlog test <file.mlog> [--filter=substring]` | Запустить `test`-блоки (юнит-тесты) |

### Переменные окружения

| Переменная | Описание |
|------------|----------|
| `METALOGOS_LLM_MOCK` | `true` (по умолчанию) — моковые LLM-ответы; `false` — реальные вызовы |
| `METALOGOS_FORCE_PIPE` | `1` — принудительно использовать piped-режим REPL (для тестов) |

---

## 2. Типы данных

| Тип | Описание | Пример литерала |
|-----|----------|-----------------|
| `String` | Строка (UTF-8, Unicode-aware) | `"Привет мир"` |
| `Float` | Число с плавающей точкой (все числа) | `42.0`, `3.14`, `-1.0` |
| `Bool` | Булево значение | `true`, `false` |
| `List` | Список значений | `[1.0, 2.0, "three"]` |
| `Struct` | Именованная структура с полями | `{ name: "Alice", age: 25.0 }` |
| `Html` | непрозрачный тип для безопасного HTML (авто-escaping) | — |
| `Query` | непрозрачный тип для SQL-запросов (инъекция невозможна) | — |
| `Secret` | непрозрачный тип для секретов (не печатается, не сериализуется) | — |
| `Encrypted` | непрозрачный тип для зашифрованных данных | — |
| `Hash` | непрозрачный тип для хешей паролей | — |
| `Session` | непрозрачный тип для сессий | — |
| `Unit` | Пустое значение (аналог `null`/`void`) | — |
| `Fluid` | Вероятностный тип (суперпозиция вариантов с confidence) | — |

> **Примечание:** В Металогосе нет отдельного типа `Int` — все числа являются `Float`.
> Целые числа записываются как `42.0`. Для преобразования строки в целое используйте `to_int()`.

---

## 3. Синтаксис

### 3.1. Комментарии

```mlog
// однострочный комментарий
```

### 3.2. Переменные и привязки

```mlog
let x = 42.0
let name = "Metalogos"
let items = [1.0, 2.0, 3.0]
let result = if x > 10.0 then "big" else "small"   // let с if-выражением
```

**Мутабельные переменные (`let mut`):** Начиная с Наряд №14, переменные по умолчанию иммутабельны. Для повторного присваивания используйте `let mut`:

```mlog
let mut counter = 0.0
while counter < 10.0 {
  counter = counter + 1.0   // OK — counter объявлена как mut
}
let x = 5.0
x = 10.0   // ОШИБКА: "cannot assign to immutable variable: x"
```

**Область видимости `let`:** `let` создаёт или перезаписывает переменную в **текущем** окружении. Внутри блоков (`if`, `each`, `while`, `match`) `let` ведёт себя как перезапись — он модифицирует переменную из внешнего окружения, а не создаёт локальную теневую копию. Это значит, что `let x = 999.0` внутри `if` изменит `x` для всего последующего кода, включая код после блока.

```mlog
let x = 1.0
if x == 1.0 {
    let x = 999.0   // перезаписывает внешний x
}
// здесь x == 999.0
```

Если нужно локальное переопределение, которое не влияет на внешний `x`, используйте другое имя или шаблон с `let tmp_x = ...`. Контракт: `examples/p30_scope_let.mlog` + `.expected`. Это поведение может измениться в будущих версиях (планируется переход на лексическое scoping с блоковой изоляцией).

**Мутабельные переменные (`let mut`, Наряд №14):** Переменные по умолчанию иммутабельны. Для присваивания используйте `let mut` (контракт: `examples/p30_assign_mut.mlog` + `.expected`, `examples/p30_assign_immutable.mlog` + `.error`):
```mlog
let mut counter = 0.0
counter = counter + 1.0   // OK
let x = 5.0
x = 10.0                  // ОШИБКА: cannot assign to immutable variable: x
```

### 3.3. Операторы

| Категория | Операторы |
|-----------|-----------|
| Арифметические | `+`, `-`, `*`, `/` |
| Сравнение | `==`, `!=`, `>`, `<`, `>=`, `<=` |
| Унарный минус | `-expr` |
| Доступ к полю | `obj.field` |
| Доступ по индексу | `list[0]` |
| Вызов функции | `func(arg1, arg2)` |
| Квалифицированный вызов | `module.func(arg)` |

### 3.4. Управляющие конструкции

**If-else (блочная форма):**
```mlog
if x > 10.0 {
  print("big")
} else if x > 5.0 {
  print("medium")
} else {
  print("small")
}
```

**If-then-else (выражение):**
```mlog
let label = if score >= 90.0 then "A" else "B"
```

**Each (цикл по коллекции):**
```mlog
each item in items {
  print(item)
}
```

**While (цикл с условием):**
```mlog
while count < 10.0 {
  let count = count + 1.0
}
```

**Match (сопоставление с образцом, Наряд №14):**
```mlog
match command {
  "start" then { print("starting") }
  starts_with "stop" then { print("stopping") }
  contains "help" then { print("helping") }
  > 100.0 then { print("too big") }
  else { print("unknown") }
}
```
Поддерживаются 4 вида arm: точное совпадение (`"val" then {}`), префикс (`starts_with "pre" then {}`), подстрока (`contains "sub" then {}`), сравнение (`> expr then {}` с любым из `>`, `<`, `>=`, `<=`, `==`, `!=`). Match возвращает значение последнего expression в выбранной ветви.

> **TW-only:** `match` (как statement) и `match` как expression в `let`-binding
> (`let x = match y { ... }`, Наряд №173b) работают только в tree-walking
> interpreter. Bytecode VM их не поддерживает — см. [ADR-0105](docs/adr/0105-vm-experimental-scope.md).

**If-else block как expression (Наряд №14):**
```mlog
let label = if score >= 90.0 { "A" } else { "B" }
let dept_color = if x == "osp" { "#FF0000" } else if x == "lz" { "#00FF00" } else { "#999999" }
```

**Try expression (error handling, Наряд №14):**
```mlog
let result = try http_post("https://api.example.com", body, "application/json")
// При ошибке: result = Unit, в stderr: [try] caught error: ...
```

**Return:**
```mlog
return result
```

**Require (RBAC, Наряд №14):** Проверяет роль текущего пользователя (только в HTTP контексте с session middleware).
```mlog
let _ = require("admin")   // При отказе: execution прерывается с ошибкой "access denied"
```

### 3.5. Строковые литералы

```mlog
let s = "hello world"
let with_escape = "line1\nline2\ttabbed"
```

Поддерживаемые escape-последовательности: `\"`, `\\`, `\n`, `\t`, `\r`.

### 3.6. Идентификаторы

Идентификаторы поддерживают ASCII, `_` и кириллицу (А-я):
```mlog
let имя = "Metalogos"
let счетчик = 0.0
pattern Приветствие(кто: String) -> String { ... }
```

---

## 4. Встроенные функции (Builtins)

> **Coverage note (v0.17):** This section documents **~47%** of the 357 registered builtins (166 of 357).
> The remaining 191 functions (pdf, cron, graph, time, bot, encoding, std helpers, etc.)
> are not yet documented here. REFERENCE.md is **not exhaustive** — see
> `src/builtins/registry.rs` for the authoritative list.
>
> Registered builtins live in `src/builtins/` (34 domain modules, ~29K lines).
> The registry (`spec!()` macro) is the single source of truth for names, arities,
> and categories — compiler, VM, and semantic analysis all derive from it.

Все встроенные функции зарегистрированы в едином реестре `BUILTIN_REGISTRY` (файл `src/builtins/registry.rs`).

### 4.1. Строковые функции

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `upper(s)` | `String -> String` | String | Преобразует строку в верхний регистр |
| `lower(s)` | `String -> String` | String | Преобразует строку в нижний регистр |
| `trim(s)` | `String -> String` | String | Убирает пробелы по краям |
| `replace(s, old, new)` | `String, String, String -> String` | String | Заменяет все вхождения `old` на `new`. Unicode-aware, работает с кириллицей и эмодзи |
| `split(s, sep)` | `String, String -> List` | List | Разделяет строку по разделителю. Пустой разделитель разбивает посимвольно |
| `join(items, sep)` | `List, String -> String` | String | Объединяет элементы списка через разделитель. По умолчанию `","` |
| `index_of(s, needle)` | `String, String -> Float` | Float | Возвращает позицию (символьную, не байтовую) первого вхождения или `-1.0` |
| `substring(s, start, end)` | `String, Float, Float -> String` | String | Извлекает подстроку по символьным индексам. Soft-failure: пустая строка при out-of-bounds |
| `char_at(s, index)` | `String, Float -> String` | String | Возвращает символ по индексу. Пустая строка при out-of-bounds |
| `starts_with(s, prefix)` | `String, String -> Bool` | Bool | Проверяет, начинается ли строка с префикса |
| `ends_with(s, suffix)` | `String, String -> Bool` | Bool | Проверяет, заканчивается ли строка суффиксом |
| `contains(s, needle)` | `String, String -> Float` | Float | Возвращает `1.0` если содержит, `0.0` если нет |
| `reverse(s)` | `String -> String` | String | Разворачивает строку (посимвольно) |
| `length(s)` | `String -> Float` | Float | Длина строки в символах (Unicode-aware). Аналог `len()` |
| `len(s)` | `String\|List -> Float` | Float | Длина строки (символы) или списка (элементы) |
| `escape_html(s)` | `String -> String` | String | Экранирует HTML-спецсимволы: `& < > " '` |
| `escape_json(s)` | `String -> String` | String | Экранирует JSON-спецсимволы: `" \ \n \t \r` |

**Примеры:**
```mlog
let s = "Привет, мир!"
upper(s)              // "ПРИВЕТ, МИР!"
lower(s)              // "привет, мир!"
trim("  hello  ")     // "hello"
replace(s, "мир", "свет")  // "Привет, свет!"
split("a,b,c", ",")   // ["a", "b", "c"]
join([1.0, 2.0], "-") // "1-2"
index_of(s, "мир")    // 8.0
substring(s, 0.0, 6.0) // "Привет"
char_at(s, 8.0)       // "м"
starts_with(s, "Прив") // true
ends_with(s, "!")      // true
contains(s, "ет")      // 1.0
reverse("abc")         // "cba"
length("Привет")       // 6.0
len([1.0, 2.0, 3.0])  // 3.0
escape_html("<script>alert('xss')</script>")
  // "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
escape_json("hello\"world\n")  // "hello\\\"world\\n"
```

### 4.2. Числа и математика

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `abs(n)` | `Float -> Float` | Float | Абсолютное значение |
| `min(a, b)` | `Float, Float -> Float` | Float | Минимум из двух чисел |
| `max(a, b)` | `Float, Float -> Float` | Float | Максимум из двух чисел |
| `clamp(val, lo, hi)` | `Float, Float, Float -> Float` | Float | Ограничивает значение диапазоном `[lo, hi]` |
| `round(n)` | `Float -> Float` | Float | Округление до ближайшего целого |
| `to_float(s)` | `String\|Float\|Bool -> Float` | Float | Преобразует в Float. Soft-failure: `0.0` |
| `to_int(s)` | `String\|Float\|Bool -> Float` | Float | Преобразует в целое (усекает дробную часть). Soft-failure: `0.0` |
| `float(s)` | `String\|Float -> Float` | Float | Аналог `to_float()`, но с ошибкой при невалидной строке |

**Примеры:**
```mlog
abs(-5.5)          // 5.5
min(3.0, 7.0)      // 3.0
max(3.0, 7.0)      // 7.0
clamp(15.0, 0.0, 10.0)  // 10.0
round(3.7)         // 4.0
to_float("3.14")   // 3.14
to_int("42abc")    // 0.0 (soft-failure)
to_int(3.9)        // 3.0
```

### 4.3. Коллекции (List)

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `get(list, index)` | `List, Float -> Value` | Value | Получить элемент по индексу. Ошибка при out-of-bounds |
| `push(list, item)` | `List, Value -> List` | List | Добавить элемент в конец (возвращает новый список) |
| `first(list)` | `List -> Value` | Value | Первый элемент. Пустая строка если список пуст |
| `last(list)` | `List -> Value` | Value | Последний элемент. Пустая строка если список пуст |
| `len(list)` | `List -> Float` | Float | Количество элементов |
| `length(list)` | `List -> Float` | Float | Аналог `len()` |
| `reverse(list)` | `List -> List` | List | Разворачивает список |
| `map(list, "pattern")` | `List, String -> List` | List | Применяет паттерн к каждому элементу (требует `import std/collections`) |
| `zip(a, b)` | `List, List -> List` | List | Попарное объединение в `Pair{a, b}` |
| `sort_by(list, "field", desc)` | `List, String, Float -> List` | List | Сортировка структур по полю (desc=1.0 → убывание) |
| `filter(list, "field", value)` | `List, String, Value -> List` | List | Фильтрация: field == value |
| `reduce(list, "field", init)` | `List, String, Float -> Float` | Float | Сумма значений поля по списку |
| `slice(list, start, end)` | `List, Float, Float -> List` | List | Срез списка [start, end). Soft-failure: start >= len → пустой список, end > len → clamp, start >= end → пустой список (ADR-0069) |
| `dedup(list)` | `List -> List` | List | Удаляет дубликаты, сохраняя порядок первого вхождения |

**Примеры:**
```mlog
let items = [10.0, 20.0, 30.0]
get(items, 1.0)        // 20.0
push(items, 40.0)      // [10.0, 20.0, 30.0, 40.0]
first(items)           // 10.0
last(items)            // 30.0
len(items)             // 3.0
reverse(items)         // [30.0, 20.0, 10.0]
slice(items, 1.0, 3.0) // [20.0, 30.0]
dedup([1.0, 2.0, 2.0])  // [1.0, 2.0]

import std/collections
let scored = map(actors, "ComputePotential")
let paired = zip(actors, scored)
let ranked = sort_by(paired, "b", 1.0)
```

### 4.4. Преобразование типов

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `str(value)` | `Any -> String` | String | Преобразует любое значение в строку |
| `to_string(value)` | `Any -> String` | String | Аналог `str()`. Float без `.0` для целых |
| `float(value)` | `String\|Float -> Float` | Float | Преобразует в число (с ошибкой) |
| `to_float(value)` | `String\|Float\|Bool -> Float` | Float | Преобразует в число (soft-failure: 0.0) |
| `to_int(value)` | `String\|Float\|Bool -> Float` | Float | Преобразует в целое (soft-failure: 0.0) |

### 4.5. LLM и AI

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `call_llm(prompt, input)` | `String, String -> String` | String | Вызывает LLM-бэкенд. По умолчанию возвращает мок: `"[MOCK: prompt \| input]"`. Реальный вызов при `METALOGOS_LLM_MOCK=false` |
| `call_claude(api_key, model, system_prompt, user_message)` | `String, String, String, String -> String` | String | Прямой вызов Anthropic Claude Messages API (v1/messages). Возвращает `content[0].text` |
| `llm_usage()` | `-> Struct` | Struct `{LlmUsage}` | Статистика использования LLM: `total_calls`, `total_tokens`, `total_errors`, `providers` (список `{alias, calls, tokens, errors, avg_latency_ms, health_score}`) |
| `confidence(fluid_value)` | `Fluid -> Float` | Float | Возвращает максимальный confidence вероятностного типа. Для конкретных значений возвращает `1.0` |

**Пример:**
```mlog
let result = call_llm("Translate to English", "Привет мир")
// По умолчанию: "[MOCK: Translate to English | Привет мир]"

let claude_response = call_claude(env("ANTHROPIC_KEY"), "claude-sonnet-4-20250514", "You are helpful.", "Hello!")
```

### 4.6. HTTP

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `http_post(url, body)` | `String, String -> String` | String | POST-запрос. Content-Type по умолчанию `application/json`. Таймаут 30с. Ошибка при статусе >= 400 |
| `http_post(url, body, content_type)` | `String, String, String -> String` | String | POST с указанным Content-Type |
| `http_post(url, body, content_type, headers)` | `String, String, String, String\|Struct -> String` | String | POST с заголовками. Если 4-й аргумент String — устанавливается `Authorization: Bearer <token>`. Если Struct — устанавливаются заголовки из полей |
| `http_get(url)` | `String -> String` | String | GET-запрос. Таймаут 30с. Ошибка при статусе >= 400 |
| `http_get(url, headers)` | `String, String\|Struct -> String` | String | GET с заголовками (Bearer token или Struct) |
| `http_post_multipart(url, fields, files)` | `String, Struct, Struct -> String` | String | Multipart POST. `fields` — текстовые поля (Struct), `files` — файловые поля (Struct, значения — пути к файлам). Таймаут 120с |

**Примеры:**
```mlog
// Простой POST
let resp = http_post("https://api.example.com/data", json_encode(payload))

// POST с Bearer-токеном
let resp = http_post("https://api.example.com/data", body, "application/json", env("API_TOKEN"))

// POST с кастомными заголовками
let headers = { "X-Custom": "value", "Authorization": "Bearer token123" }
let resp = http_post("https://api.example.com/data", body, "application/json", headers)

// Multipart POST (загрузка файла)
let fields = {"model": "whisper-1"}
let files = {"file": "/tmp/voice.ogg"}
let resp = http_post_multipart("https://api.openai.com/v1/audio/transcriptions", fields, files)

// GET
let data = http_get("https://api.example.com/users")
let data = http_get("https://api.example.com/users", env("API_TOKEN"))
```

### 4.6.1. Голосовой pipeline

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `whisper_transcribe(file_id, bot_token, api_key, provider)` | `String, String, String, String -> String` | String | Скачивает голосовое сообщение из Telegram по `file_id`, отправляет на транскрипцию в Whisper API. `provider`: `"openai"` (по умолчанию) или `"groq"`. Возвращает распознанный текст |
| `tts_send(text, voice, bot_token, chat_id)` | `String, String, String, String -> String` | String | Генерирует речь через OpenAI TTS API (`tts-1` модель) и отправляет аудио в Telegram чат. Требует `OPENAI_API_KEY`. Голоса: `"alloy"`, `"echo"`, `"fable"`, `"onyx"`, `"nova"`, `"shimmer"` |

### 4.7. JSON

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `parse_json(text)` | `String -> Struct\|List\|String\|Float\|Bool\|Unit` | Any | Парсит JSON-строку. Объекты → Struct с `type_name: "Json"`, массивы → List, `null` → Unit |
| `json_encode(value)` | `Any -> String` | String | Сериализует значение в JSON-строку. Поддерживает String, Float, Bool, Unit→null, List→array, Struct→object |
| `json_get(obj, field_path)` | `Struct, String -> Value` | Value | Доступ к полю по dot-path. Возвращает **реальное значение** (в т.ч. String). Если поле отсутствует или SQL NULL — возвращает Unit. Поддерживает dot-path: `"voice.file_id"` |
| `json_get(obj, field_path, default)` | `Struct, String, Value -> Value` | Value | С дефолтным значением при отсутствии поля или SQL NULL (v0.9.6) |
| `has_field(obj, field_path)` | `Struct, String -> Float` | Float | `1.0` если поле существует, `0.0` если нет. Поддерживает dot-path |
| `dict_get(dict, key, default)` | `Struct, String, Any -> Any` | Any | Доступ к ключу dict-style. Возвращает `default` если ключ отсутствует. Не поддерживает dot-path (для этого есть `json_get`) |
| `dict_set(dict, key, value)` | `Struct, String, Any -> Struct` | Struct | Возвращает **новый** Struct с обновлённым ключом. Исходный dict не мутируется |
| `dict_has(dict, key)` | `Struct, String -> Bool` | Bool | `true` если ключ существует в dict. Прямая проверка (без dot-path) |
| `dict_keys(dict)` | `Struct -> List` | List | Возвращает список всех ключей dict |
| `dict_values(dict)` | `Struct -> List` | List | Возвращает список всех значений dict (порядок соответствует `dict_keys`) |
| `escape_json(text)` | `String -> String` | String | Экранирует спецсимволы для встраивания в JSON |

**Примеры:**
```mlog
let data = parse_json("{\"name\": \"Alice\", \"age\": 30}")
let name = json_get(data, "name")           // "Alice"
let missing = json_get(data, "email")        // Unit (поле отсутствует)
let missing = json_get(data, "email", "none") // "none" (дефолт)

let nested = parse_json("{\"a\": {\"b\": 42}}")
json_get(nested, "a.b")                      // 42.0
has_field(nested, "a.b")                     // 1.0

let encoded = json_encode({ key: "value", n: 42.0 })
// "{\"key\":\"value\",\"n\":42.0}"
```

### 4.8. Файловый I/O

> **Важно:** Все файловые операции песочницей ограничены рабочей директорией.
> Абсолютные пути и `..` (path traversal) отклоняются.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `read_file(path)` | `String -> String` | String | Читает файл. Soft-failure: пустая строка при ошибке |
| `write_file(path, content)` | `String, String -> String` | String | Пишет файл (перезапись). Возвращает `"ok"` или `""` при ошибке |
| `append_file(path, content)` | `String, String -> String` | String | Добавляет в конец файла. Возвращает `"ok"` или `""` |
| `delete_file(path)` | `String -> String` | String | Удаляет файл. Возвращает `"ok"` или `""` |
| `file_exists(path)` | `String -> Bool` | Bool | Проверяет существование файла |
| `list_dir(path)` | `String -> List` | List | Список файлов в директории. Без аргумента — текущая директория |

**Примеры:**
```mlog
write_file("data.txt", "hello world")  // "ok"
let content = read_file("data.txt")   // "hello world"
append_file("data.txt", "\nmore")     // "ok"
file_exists("data.txt")               // true
let files = list_dir(".")             // ["data.txt", ...]
delete_file("data.txt")               // "ok"
```

### 4.9. Память (KV-хранилище)

Глобальное in-memory KV-хранилище. При `memory { persist: "path.db" }` — также записывает в SQLite (write-through).

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `kv_set(key, value)` | `String, String -> Unit` | Unit | Записывает пару ключ-значение |
| `kv_get(key)` | `String -> String` | String | Читает значение (пустая строка если нет ключа) |
| `kv_delete(key)` | `String -> Unit` | Unit | Удаляет ключ |
| `kv_exists(key)` | `String -> Bool` | Bool | Проверяет существование ключа |
| `kv_list()` | `-> List` | List | Возвращает список всех ключей |
| `mem_set(key, value)` | `String, String -> String` | String | Аналог `kv_set`, но возвращает записанное значение |
| `mem_get(key)` | `String -> String` | String | Аналог `kv_get` |
| `mem_delete(key)` | `String -> String` | String | Аналог `kv_delete`, возвращает удалённое значение |

### 4.9.1. Семантическая память с типами и гибридным поиском (ADR-0093, ADR-0094)

Типизированная семантическая память с SQLite-персистентностью, FTS5 BM25
ключевым индексом, косинусной схожестью и слиянием через Reciprocal Rank
Fusion (k=60). Каждая запись несёт тип-тег для дифференцированного поиска.

**Типы памяти:** `persona`, `episodic`, `instruction`, `fact`

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `memorize(text, priority, type)` | `String, Float, String -> Unit` | Unit | Сохраняет факт с приоритетом (0.0–1.0) и типом. Пример: `memorize("любит острую еду", 0.9, "persona")` |
| `recall_top_k(query, k, type)` | `String, Float, String -> String` | String (JSON) | Возвращает top-K записей, отсортированных по RRF-скору. JSON-массив: `[{text, type, priority, score, created_at}]`. Пустой тип — поиск по всем типам |

**Скоринг:** Reciprocal Rank Fusion (k=60). Результаты BM25 и косинусной схожести
(с временным затуханием и приоритетом) ранжируются отдельно, затем сливаются:
`score = 1/(60+bm25_rank) + 1/(60+cosine_rank)`. RRF устойчив к различиям
в распределении скоров между сигналами.

**Примеры:**
```mlog
// Сохранение с типом
memorize("пользователь предпочитает email", 0.9, "persona")
memorize("дедлайн проекта 15 июля", 0.8, "fact")
memorize("всегда приветствовать на русском", 1.0, "instruction")

// Поиск top-5 фактов
let results = recall_top_k("предпочтения пользователя", 5.0, "persona")

// Поиск по всем типам
let all = recall_top_k("проект", 10.0, "")
```

### 4.9.2. Memory Tree (mtree) — многоуровневая память (наряд №63)

Иерархическая память L0 (raw) → L1 (cluster summary) → L2 (high-level).
Графовая структура с поиском путей и извлечением подграфов.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `mtree_store(text, source?)` | `String, String? -> String` | String (id) | Сохраняет текст как L0-узел. `source` по умолчанию `"user"`. Возвращает ID узла. Score для admission вычисляется по длине/уникальности |
| `mtree_retrieve(query, limit?)` | `String, Float? -> String` | String (JSON) | Поиск по графу памяти. `limit` по умолчанию 5. Возвращает JSON-массив узлов с метаданными |
| `mtree_forget(id)` | `String -> Float` | Float | Удаляет узел по ID. Возвращает `1.0` если существовал, `0.0` если нет |
| `mtree_summarize()` | `-> Unit` | Unit | L0 → L1 → L2 кластеризация. Группирует L0-узлы в L1-summaries, L1 в L2. Idempotent — повторный вызов пересчитывает summaries |
| `mtree_stats()` | `-> Dict` | Dict | Возвращает `MemoryStats { l0_count, l1_count, l2_count, total_nodes, edges }` |

### 4.9.3. Граф памяти: пути и подграфы

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `graph_query(query, limit?, level_filter?)` | `String, Float?, String? -> String` | String (JSON) | Поиск по графу с опциональным фильтром уровня (`"L0"`, `"L1"`, `"L2"`). `limit` по умолчанию 5 |
| `graph_path(from_id, to_id)` | `String, String -> String` | String (JSON) | Кратчайший путь между двумя узлами. Возвращает JSON-массив узлов в пути. Пустой массив если пути нет |
| `graph_neighbors(id)` | `String -> String` | String (JSON) | Соседи узла (прямые связи). JSON-массив узлов |
| `subgraph_extract(root_id, depth)` | `String, Float -> String` | String (JSON) | Извлекает подграф от `root_id` на глубину `depth`. JSON с узлами и рёбрами |
| `subgraph_nodes(root_id, depth)` | `String, Float -> String` | String (JSON) | Только узлы подграфа (без рёбер). JSON-массив узлов |
| `subgraph_json(root_id, depth)` | `String, Float -> String` | String (JSON) | Полный подграф как JSON (nodes + edges). Готов для визуализации |
| `trace_start(label)` | `String -> String` | String (id) | Начинает trace-сегмент с меткой. Возвращает trace_id |
| `trace_end(trace_id)` | `String -> Dict` | Dict | Завершает trace-сегмент. Возвращает `TraceResult { id, label, duration_ms }` |

### 4.10. Сессионная память

Временное in-memory хранилище, привязанное к session_id. Не персистентно — сбрасывается при перезапуске `mlog serve`.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `session_set(session_id, key, value)` | `String, String, String -> String` | String | Сохраняет значение в сессии |
| `session_get(session_id, key)` | `String, String -> String` | String | Читает значение из сессии (пустая строка если нет) |
| `session_clear(session_id)` | `String -> String` | String | Удаляет все данные сессии. Возвращает `"ok"` |

### 4.11. Шифрование и безопасность

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `env(key)` | `String -> String` (→ `Secret` в entity context) | String/Secret | Читает переменную окружения. Пустая строка если не найдена |
| `generate_key()` | `-> Secret` | Secret | Генерирует 256-битный случайный ключ (64 hex-символа) |
| `encrypt(data, key)` | `String, Secret -> Encrypted` | Encrypted | Шифрует AES-256-GCM со случайным 96-бит nonce. Ключ — 64 hex-символа |
| `decrypt(encrypted, key)` | `Encrypted, Secret -> String` | String | Расшифровывает AES-256-GCM. Ошибка при неверном ключе |
| `hash_password(password)` | `String -> Hash` | Hash | Хеширует пароль (Argon2id с рандомной солью) |
| `verify_password(password, hash)` | `String, Hash -> Bool` | Bool | Проверяет пароль (constant-time сравнение) |
| `sha256(text)` | `String -> String` | String | SHA-256 хеш, hex-представление (64 символа). Unicode-aware: хеширует UTF-8 байты |
| `hmac_sha256(key, message)` | `String, String -> String` | String | HMAC-SHA256 с ключом, hex-представление (64 символа) |
| `hex_encode(text)` | `String -> String` | String | Кодирует строку в hex (UTF-8 байты → hex символы) |
| `hex_decode(hex_str)` | `String -> String` | String | Декодирует hex в строку. Невалидный UTF-8 → `<binary: N bytes>` placeholder |
| `require(condition)` | `Bool -> Unit` | Unit | Runtime-assertion. Ошибка если `false` |
| `require(condition, message)` | `Bool, String -> Unit` | Unit | Assertion с сообщением об ошибке |

**Примеры:**
```mlog
entity db_url: Secret = env("DATABASE_URL")
let key = generate_key()
let encrypted = encrypt("secret data", key)
let decrypted = decrypt(encrypted, key)  // "secret data"

let hash = hash_password("mypassword")     // Hash (opaque)
verify_password("mypassword", hash)        // true
verify_password("wrong", hash)             // false

require(user.role == "admin")              // паника если не admin
require(age >= 18.0, "Access denied")      // с сообщением
```

### 4.12. Аутентификация

> **Примечание:** В режиме `mlog run` эти функции возвращают мок-значения.
> Реальная работа только в контексте `mlog serve` (Axum-сервер).

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `authenticate(email, password)` | `String, Secret\|String -> Unit` | Unit | Аутентификация пользователя. В server-контексте проверяет credentials |
| `session_login(user_id)` | `String -> Session` | Session | Создаёт сессию для пользователя |
| `session_logout(session)` | `Session -> Unit` | Unit | Уничтожает сессию |

### 4.13. HTTP-сервер (mlogserver)

Функции для использования внутри route-обработчиков `mlogserver`/`server` блоков.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `respond(status_line)` | `String -> HttpResponse` | HttpResponse | Формирует HTTP-ответ. Формат: `"200 OK"`, `"404 Not Found"` и т.д. |
| `respond_html(status, html)` | `String, String -> HttpResponse` | HttpResponse | HTML-ответ с указанным статусом |
| `form_data()` | `-> Struct {FormData}` | Struct | Парсит данные из `application/x-www-form-urlencoded` тела запроса |
| `json_body()` | `-> Struct {JsonBody}` | Struct | Парсит JSON из тела запроса |
| `query_param(name)` | `String -> String` | String | Получает query-параметр из URL. `curl "localhost:8080/search?q=hello" → query_param("q") == "hello"`. Пустая строка если параметр отсутствует. |

**Пример:**
```mlog
mlogserver {
  port: 8080
  route "/hello" method=GET {
    respond("200 OK")
  }
  route "/api/data" method=POST {
    let data = json_body()
    let name = json_get(data, "name", "unknown")
    respond_html("200", "<h1>Hello " + escape_html(name) + "</h1>")
  }
  route "/search" method=GET {
    let q = query_param("q")
    respond("200 " + q)
  }
}
```

### 4.14. Шаблоны (Templates)

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `render(template_name, key1, val1, key2, val2, ...)` | `String, String, Any, ... -> Html` | Html | Рендерит шаблон с подстановкой переменных `{{ var }}`. Количество аргументов после имени шаблона должно быть чётным (пары ключ/значение) |

**Пример:**
```mlog
template Page(title: String, body: String) -> Html {
  <html><head><title>{{ title }}</title></head><body>{{ body }}</body></html>
}

// В route handler:
let page = render(Page, "title", "My Page", "body", "Hello!")
return page
```

### 4.15. Базы данных

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `query(sql, params)` | `String, List -> List` | List | SQL-запрос. SELECT возвращает List[Row{...}], остальное — строку с числом затронутых строк |
| `db_execute(sql)` | `String -> Unit` | Unit | Выполняет SQL-запрос без возврата данных |
| `db_insert(table, struct)` | `String, Struct -> Float` | Float | Параметризованный INSERT. Возвращает last_insert_rowid (Problem C) |

**Schema-as-code** (ADR-0060) — декларация таблиц прямо в .mlog:

```mlog
db { url: "sqlite::memory:" }

schema my_dept {
  table analysis {
    id: Int primary_key auto_increment
    topic: String
    status: String default("drafted")
  }
}
```

Типы: Int->INTEGER, Float->REAL, String/Text->TEXT, Bool->INTEGER, DateTime->TEXT. Модификаторы: primary_key, auto_increment, nullable, references(table.field). Дефолты: default("value"), default(now()). Миграция: additive-only (CREATE TABLE IF NOT EXISTS).
### 4.16. Боты (Telegram/Discord)

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `send_message(chat_id, text)` | `String\|Float, String -> Unit` | Unit | Отправляет сообщение в чат (Telegram/Discord). В interpreter mode — логирует в `[AUDIT]` |

### 4.17. Прочее

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `print(s)` | `String -> String` | String | Выводит строку в stdout, возвращает её же |
| `base64_encode(s)` | `String -> String` | String | Кодирует строку в Base64 (standard alphabet). Unicode-aware: кодирует UTF-8 байты |
| `base64_decode(s)` | `String -> String` | String | Декодирует Base64. Ошибка если невалидный Base64 или не UTF-8 |
| `toon_encode(value)` | `Any -> String` | String | Кодирует значение в TOON (Token-Optimized Object Notation). Префикс `TOON:`. Любой Value → строка |
| `toon_decode(s)` | `String -> Any` | Value | Декодирует TOON-строку обратно в Value. Recursive descent parser |
| `assert_eq(actual, expected)` | `Any, Any -> Any` | Any | Runtime-equality assertion. Возвращает actual при успехе, паникует с `actual != expected` |
| `assert_contains(haystack, needle)` | `Any, Any -> Unit` | Unit | Паникует если строковое представление `needle` не найдено в `haystack` |

### 4.17.1. OpenPlanter-inspired system builtins

> Источник: наряд №64 (OpenPlanter agent utilities). Используются в
> агент-флоу для бюджет-aware выполнения, снимков состояния и
> безопасного exec.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `budget_check(step, total_steps)` | `Float, Float -> Dict` | Dict | Возвращает `BudgetStatus { step, total_steps, remaining, fraction, over_budget }`. Ошибка если `total_steps == 0` |
| `replay_snapshot(data)` | `List -> Dict` | Dict | Сериализует список значений в JSON-снимок. Возвращает `ReplaySnapshot { seq, items, json, created_at }`. seq=0 — полный снимок, seq=N — delta |
| `policy_check(command)` | `String -> Dict` | Dict | Проверяет команду на соответствие policy: heredoc `<<`, pipe `|`, background `&`, redirect `>`. Возвращает `PolicyResult { command, allowed, reason }` |

### 4.17.2. Fluid-type контроль бюджета

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `budget_check(step, total_steps)` | `Float, Float -> Dict` | Dict | (См. 4.17.1 — указан в категории `system`, также используется в fluid-пайплайнах для контроля лавины правок) |

### 4.17.3. Cron-планировщик (наряд №35)

Персистентный cron-планировщик для отложенных задач. Jobs сохраняются в
JSON и переживают перезапуск процесса. Выражения — стандартный 5-field
cron (`min hour dom month dow`).

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `cron_add(cron_expr, prompt)` | `String, String -> String` | String (id) | Добавляет cron-job. `cron_expr` — 5-field cron (например `"0 9 * * 1-5"` — каждый будний день в 9:00). `prompt` — что запускать. Возвращает ID job'а |
| `cron_list()` | `-> List` | List | Список всех cron-jobs. Каждый элемент — `CronJob { id, cron_expr, prompt, force_run, run_count, last_run, created_at }` |
| `cron_remove(id)` | `String -> Float` | Float | Удаляет job по ID. Возвращает `1.0` если удалён, `0.0` если не найден |
| `cron_run(id)` | `String -> Float` | Float | Принудительно запускает job вне расписания (ставит `force_run=true`). Возвращает `1.0` если найден, `0.0` если нет |
| `cron_mark_fired(id)` | `String -> Float` | Float | Помечает job как выполненный: сбрасывает `force_run`, инкрементирует `run_count`, обновляет `last_run`. Возвращает `1.0` если найден, `0.0` если нет |

### 4.17.4. Время и даты

Все функции работают с Unix timestamps (Float, секунды с эпохи 1970-01-01).
Локальная таймзона используется для `format_date` / `weekday_name` /
`date_parts` (на сервере — таймзона процесса).

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `time()` | `-> Float` | Float | Текущий Unix timestamp (секунды с эпохи). Высокая точность (sub-second) |
| `sleep(seconds)` | `Float -> Unit` | Unit | Блокирует текущий поток на `seconds` секунд. Использовать осторожно в `mlog serve` — блокирует обработку запроса |
| `format_date(fmt?, timestamp?)` | `String?, Float? -> String` | String | Форматирует timestamp по strftime-строке. `fmt` по умолчанию `"%Y-%m-%d %H:%M:%S"`. `timestamp` по умолчанию — текущий момент |
| `date_parts(timestamp?)` | `Float? -> Dict` | Dict | Возвращает `DateParts { year, month, day, hour, minute, second, weekday }`. `timestamp` по умолчанию — текущий момент |
| `days_between(ts1, ts2)` | `Float, Float -> Float` | Float | Абсолютная разница между двумя timestamps в днях. `|ts1 - ts2| / 86400` |
| `days_in_month(year, month)` | `Float, Float -> Float` | Float | Число дней в месяце. `month` — 1-12. Ошибка если месяц вне диапазона. Учитывает високосные годы |
| `is_leap_year(year)` | `Float -> Bool` | Bool | `true` если год високосный (Gregorian rules) |
| `add_days(timestamp, days)` | `Float, Float -> Float` | Float | Прибавляет `days` к timestamp. Отрицательные `days` — вычитание. `ts + days * 86400` |
| `add_hours(timestamp, hours)` | `Float, Float -> Float` | Float | Прибавляет `hours` к timestamp. Отрицательные `hours` — вычитание. `ts + hours * 3600` |
| `weekday_name(timestamp)` | `Float -> String` | String | Название дня недели (локализованное через chrono::Local). Например `"Monday"` |

### 4.18. PDF (Наряд №48, pdf-inspector)

Нативная обработка PDF на Rust через `pdf-inspector` crate. Классификация,
извлечение текста в Markdown, регионный анализ и OCR-фоллбэк.
Нулевой IPC, <200ms на текстовых PDF.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `pdf_classify(path)` | `String -> Dict` | Dict | Классификация PDF: TextBased / Scanned / ImageBased / Mixed. Ключи: type, confidence, pages_needing_ocr, page_count |
| `pdf_to_markdown(path)` | `String -> Dict` | Dict | Полный pipeline: классификация + извлечение текста + Markdown. Ключи: markdown, page_count, pdf_type, has_tables, confidence, processing_time_ms |
| `pdf_extract_regions(path, filter)` | `String, String -> List` | List | Извлечение текстовых регионов с координатами. Список словарей: text, needs_ocr, ocr_reason, page, x, y |
| `pdf_ocr(path)` | `String -> Dict` | Dict | OCR-фоллбэк для сканов (требует `--features pdf-ocr` и системный Tesseract). Ключи: markdown, ocr_confidence, pages_processed |

**Примеры:**
```mlog
// Классификация PDF
let info = pdf_classify("report.pdf")
// → { type: "TextBased", confidence: 0.95, pages_needing_ocr: [], page_count: 12 }

// Извлечение текста в Markdown
let result = pdf_to_markdown("report.pdf")
let md = json_get(result, "markdown", "")

// Для сканов — OCR
let ocr = pdf_ocr("scan.pdf")
let text = json_get(ocr, "markdown", "")
```

> **Примечание:** `pdf_ocr` требует сборки с флагом `--features pdf-ocr` и установленного
> системного пакета `tesseract-ocr` с CJK training data.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `now()` | `-> Float` | Float | Текущий Unix-timestamp в секундах |
| `str(value)` | `Any -> String` | String | Преобразует любое значение в строку |
| `to_string(value)` | `Any -> String` | String | Аналог `str()` (Float без `.0` для целых) |

---

### 4.19. SVG-графика и диаграммы (наряды №77–92, ADR-0102)

44 встроенные функции, написанные вручную на чистом Rust — без единой
внешней SVG/chart/rendering-библиотеки. Все диспетчеризуются через
общий путь (не спец-кейс), паритет TW/VM для путей, которые оба бэкенда умеют исполнять (см. ADR-0105; `match`/блочный if-else в VM пока нет)
и проверен `crosscheck`. Полная история решений — ADR-0102 и наряды
№77–92.

> **Безопасность:** каждая функция классифицирована в `svg_security_lint`
> (`src/semantic.rs`) — текстовые аргументы либо автоматически
> экранируются рантаймом (`SVG_AUTO_ESCAPE_BUILTINS`, предупреждение
> при `mlog check`), либо структурные аргументы (`d`, `viewbox`,
> `transform` — мини-языки, которые нельзя безопасно экранировать)
> дают жёсткую **ошибку** компиляции при подозрении на инъекцию
> (`SVG_NO_ESCAPE_BUILTINS`). Наряд №92 проверил все 44 функции —
> пробелов не найдено.

#### Палитра

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `color_palette(intent, mode)` | `String, String -> Struct` | `DiagramStyle` | HSL-каскадная палитра. `intent` ∈ {calm, tension, energy, authority, warmth}, `mode` ∈ {light, dark}. Вывод — 5 токенов (`paper`, `ink`, `accent`, `muted`, `rule`) |
| `diagram_style(paper, ink, accent, muted, rule)` | `String×5 -> Struct` | `DiagramStyle` | Валидатор для вручную заданных токенов (без генерации через `color_palette`) |

#### SVG-примитивы

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `svg_rect(x, y, w, h, fill, stroke)` | `Float×4, String×2 -> String` | String | Прямоугольник |
| `svg_circle(cx, cy, r, fill, stroke)` | `Float×3, String×2 -> String` | String | Круг |
| `svg_line(x1, y1, x2, y2, stroke, width)` | `Float×4, String, Float -> String` | String | Линия |
| `svg_text(x, y, text, font_size, fill, anchor)` | `Float×2, String, Float, String×2 -> String` | String | Текст (экранируется автоматически) |
| `svg_path(d, fill, stroke)` | `String×3 -> String` | String | Произвольный путь. `d` — структурный аргумент, **не экранируется**, ошибка компиляции при инъекции |
| `svg_group(children, transform)` | `List, String -> String` | String | Группа с опциональной трансформацией (`transform` структурный, как `d`) |
| `svg_canvas(w, h, viewbox, children)` | `Float×2, String, List -> String` | String | Корневой `<svg>` (`viewbox` структурный) |
| `svg_canvas_preset(preset_name, viewbox, children)` | `String, String, List -> String` | String | То же, с именованным размером холста: `doc_inline` (960×600), `slide_16x9` (1280×720), `social_og` (1200×632), `print_a4_landscape`, `print_a4_portrait` |
| `svg_icon(name, x, y, size, fill)` | `String, Float×3, String -> String` | String | Готовая иконка. `name` ∈ {server, laptop, phone, database, cloud, arrow-right, check, warning, user, document} |
| `svg_sketchy_filter(id, roughness)` | `String, Float -> String` | String | SVG-фильтр «рукописного» стиля (`id` структурный) |
| `svg_generate(kind, intent, w, h)` | `String, String, Float×2 -> String` | String | Процедурный фон. `kind` ∈ {flow, grid, noise}. Детерминированный (одинаковый `intent` → идентичный результат, без `rand`/системных часов) |

#### Чарты

| Функция | Форма данных | Особенность |
|---------|-------------|-------------|
| `chart_bar(data, style)` | `List<Struct{label, value}>` | Базовый столбчатый |
| `chart_donut(data, style)` | `List<Struct{label, value}>` | Дуги через `svg_path` |
| `chart_line(data, style)` | `List<Struct{label, value}>` | Соединённые точки |
| `chart_area(data, style)` | `List<Struct{label, value}>` | Заливка под линией |
| `chart_scatter(data, style)` | `List<Struct{x, y, label?}>` | Независимое масштабирование обеих осей — **другая форма данных**, чем остальные |
| `chart_heatmap(data, style)` | `List<List<Float>>` | HSL-интерполяция цвета по значению; без текста — сознательно вне security-lint |
| `chart_radar(data, style)` | `Struct{axes: List<String>, series: List<Struct{name, values}>}` | Многосерийный, полярные координаты, фиксированная палитра до 5 серий |
| `chart_boxplot(data, style)` | `List<Struct{label, values: List<Float>}>` | Реальные квартили (linear interpolation / R-7 метод), усы 1.5×IQR, выбросы |

#### Диаграммы — иерархии и потоки

| Функция | Форма данных | Особенность |
|---------|-------------|-------------|
| `diagram_tree(data, style)` | `Struct{label, children: List<Struct>}` (рекурсивно) | Раздельная раскладка поддеревьев |
| `diagram_org_chart(data, style)` | То же + опциональный `title` | Тонкая обёртка над `diagram_tree` |
| `diagram_flowchart(data, style)` | `Struct{nodes, edges}` | Топологическая сортировка по слоям; **цикл — ошибка** с явным текстом |
| `diagram_layers(data, style)` | `List<Struct{label, description?}>` | Горизонтальные полосы |

#### Диаграммы — временные и процессные

| Функция | Форма данных | Особенность |
|---------|-------------|-------------|
| `diagram_sequence(data, style)` | `Struct{actors: List<String>, messages: List<Struct{from, to, label?}>}` | Вертикальные lifeline, `actors` — список строк, не структур |
| `diagram_timeline(data, style)` | `List<Struct{date, label, description?}>` | Anti-overlap engine применяется автоматически (наряд №87) |
| `diagram_gantt(data, style)` | `List<Struct{task, start, duration}>` | Условные единицы времени, без привязки к календарю |
| `diagram_process(data, style)` | `List<Struct{label, description?}>` | Строго линейно — **не путать** с `diagram_flowchart` |
| `diagram_loop(data, style)` | `List<Struct{label, description?}>` | Замкнутый цикл, полярные координаты, минимум 3 шага |

#### Диаграммы — множества и сравнения

| Функция | Форма данных | Особенность |
|---------|-------------|-------------|
| `diagram_venn(data, style)` | `Struct{circles: List<Struct{label, value?}>, overlap_label?}` | Строго 2 или 3 круга, фиксированная симметричная геометрия — общий N-круговой Venn вне объёма |
| `diagram_quadrant(data, style)` | `Struct{x_axis_label, y_axis_label, items: List<Struct{label,x,y}>}` | `x`,`y` ∈ [-1.0, 1.0] |
| `diagram_pyramid(data, style)` | `List<Struct{label, value?}>` | Первый элемент — верхний узкий слой (не основание) |
| `diagram_nested(data, style)` | `List<Struct{label, value?}>` | Концентрические кольца, первый элемент — внешнее |
| `diagram_medallion(data, style)` | `List<Struct{icon?, label, value?}>` | `icon` — имя из `svg_icon`, валидация переиспользована |

#### Диаграммы — данные и состояния

| Функция | Форма данных | Особенность |
|---------|-------------|-------------|
| `diagram_er(data, style)` | `Struct{entities: List<Struct{name, fields: List<String>}>, relations}` | Простая сетка 3-в-ряд, не анализ связей для позиционирования |
| `diagram_state(data, style)` | `Struct{states: List<String>, transitions, initial?}` | **Циклы и self-loop валидны** (в отличие от flowchart) |
| `diagram_swimlane(data, style)` | `Struct{lanes: List<String>, steps}` | Позиция по `order`, не по индексу |
| `diagram_data_flow(data, style)` | `Struct{nodes, edges}` | Циклы валидны (обратная связь данных) |
| `diagram_high_level(data, style)` | `Struct{nodes, edges}` | Без иконок, ациклический (топологическая сортировка) |
| `diagram_architecture(data, style)` | `Struct{nodes: List<Struct{id,label,icon?}>, edges}` | То же + `svg_icon` для узлов |

#### Композиция и качество

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `template_render(template, data)` | `String, Struct -> Html` | Html | Отдельный движок (не расширение `render()`): `{{ var }}` (экранируется), `{{{ var }}}` (без экранирования), `{{#if}}/{{else}}`, `{{#each}}`. Содержимое шаблона — доверенный код автора, не сканируется lint'ом |
| `infographic_qa(svg)` | `String -> Struct{passed, warnings, checks_run}` | Struct | Advisory-проверка: контраст (WCAG, порог 4.5), дисциплина насыщенности (>2 цветов с S>60% — предупреждение), плотность элементов. `passed: false` — совет, не блокировка |
| `html_render(html, width, height)` | `String, Float×2 -> String` | String | Скриншот через headless-браузер (`METALOGOS_BROWSER_BIN`, без дефолтного пути). Единственная функция серии, порождающая внешний процесс — через `exec_restricted` (argv, не shell). Сетевая изоляция не гарантируется на уровне ОС — вход должен быть самодостаточным HTML |

`std/infographic.mlog` — паттерны верхнего уровня, компонующие
перечисленное: `InfographicPoster`, `InfographicDashboard` (KPI-карточки
+ сетка чартов 2×2), `InfographicComparison` (side-by-side, требует
одинаковый `chart_type` слева и справа), `InfographicTimeline`
(обёртка над `diagram_timeline`).

---

### 4.20. Email, календарь, контакты (наряды MLG-4/5/6)

27 функций, все на чистом Rust (`lettre`/`imap` для почты, hand-rolled
CalDAV/CardDAV клиент для календаря и контактов). Две разные модели
работы с учётными данными — не путать при использовании.

#### Email (SMTP/IMAP) — учётные данные через переменные окружения

**Не принимают host/user/pass как аргументы.** Конфигурация читается
из окружения при каждом вызове: `SMTP_HOST`, `SMTP_PORT` (по умолчанию
587), `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM` (по умолчанию — `SMTP_USER`);
`IMAP_HOST`, `IMAP_PORT`, `IMAP_USER`, `IMAP_PASS`. Отсутствие
обязательной переменной — понятная ошибка (`smtp_send: SMTP_HOST env
not set`), не молчаливый сбой.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `smtp_send(to, subject, body, attachments_json?, from?, reply_to?)` | `String×3, String?×3 -> String` | String | Отправка plain-text письма, TLS/STARTTLS. Арность 3..6 |
| `smtp_send_html(to, subject, html_body, attachments_json?)` | `String×3, String? -> String` | String | HTML-письмо. Арность 3..4 |
| `imap_list(folder, limit, offset?)` | `String, Float, Float? -> String` | String (JSON) | Список писем — envelope + флаги. Арность 2..3 |
| `imap_read(message_id)` | `String -> String` | String | Полное письмо: заголовки, тело, вложения |
| `imap_search(folder, query)` | `String, String -> String` | String (JSON) | Поиск по тексту (IMAP `TEXT` criteria) |
| `imap_mark_read(message_id)` | `String -> String` | String | Пометка как прочитанного |
| `imap_move(message_id, target_folder)` | `String, String -> String` | String | Перемещение (RFC 6851 `MOVE`, откат на `COPY`+`DELETE`) |

#### Календарь (CalDAV/iCal) — сессия через `cal_connect`

**Модель session-based**, не переменные окружения. `cal_connect`
возвращает `session_id`, который передаётся во все последующие вызовы —
ближе к тому, как работают `whisper_transcribe`/другие stateful
интеграции, чем к email-функциям выше.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `cal_connect(url, user, pass)` | `String×3` (пароль принимает `Secret`, наряд №70) `-> String` | String | `session_id`. PROPFIND, discovery `calendar-home-set` |
| `cal_list(session_id)` | `String -> String` | String (JSON) | Список календарей (PROPFIND Depth:1) |
| `cal_events(session_id, start_date, end_date)` | `String×3 -> String` | String (JSON) | События в диапазоне (CalDAV REPORT `calendar-query`, RFC 4791 §7.8) |
| `cal_read(event_url)` | `String -> String` | String | Одно событие по URL |
| `cal_create(calendar_id, summary, start, end, description?, location?, attendees_json?)` | `String×4, String?×3 -> String` | String | Создаёт `.ics`, возвращает UID. Арность 4..7 |
| `cal_update(event_url, fields_json)` | `String×2 -> String` | String | GET+PUT с `ETag`/`If-Match` |
| `cal_delete(event_url)` | `String -> String` | String | DELETE с `If-Match` |
| `cal_freebusy(session_id, start_date, end_date)` | `String×3 -> String` | String (JSON) | CalDAV REPORT `free-busy-query`, RFC 4791 §7.10 |
| `ical_parse(text)` | `String -> String` | String (JSON) | Разбор iCalendar-текста (RFC 5545) |
| `ical_generate(json)` | `String -> String` | String | Сборка `VEVENT`+`VCALENDAR` из JSON |

#### Контакты (CardDAV/vCard) — та же session-модель

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `card_connect(url, user, pass)` | `String×3` (`Secret`-совместимо) `-> String` | String | `session_id`. PROPFIND, discovery `addressbook-home-set` |
| `card_list(session_id)` | `String -> String` | String (JSON) | Список адресных книг |
| `card_contacts(session_id, addressbook_id)` | `String×2 -> String` | String (JSON) | Контакты книги (CardDAV REPORT `addressbook-query`, RFC 6352 §8.6) |
| `card_read(contact_url)` | `String -> String` | String | Один контакт по URL |
| `card_create(addressbook_id, fn, email, tel?, org?, title?, note?)` | `String×3, String?×4 -> String` | String | Создаёт `.vcf`, возвращает UID. Арность 3..7 |
| `card_update(contact_url, fields_json)` | `String×2 -> String` | String | GET+PUT с `ETag`/`If-Match` |
| `card_delete(contact_url)` | `String -> String` | String | DELETE с `If-Match` |
| `card_search(session_id, query)` | `String×2 -> String` | String (JSON) | Поиск по всем книгам (`FN` + `EMAIL`) |
| `vcard_parse(text)` | `String -> String` | String (JSON) | Разбор vCard-текста (RFC 6350) |
| `vcard_generate(json)` | `String -> String` | String | Сборка vCard v4.0 из JSON |

> **Конфигурация подключения** — в отличие от email-функций выше,
> `cal_connect`/`card_connect` **не имеют** встроенного резерва на
> переменные окружения — все три аргумента (`url`, `user`, `pass`)
> обязательны при каждом вызове. Соглашение об именах `CALDAV_URL`/
> `CALDAV_USER`/`CALDAV_PASS`, `CARDDAV_URL`/`CARDDAV_USER`/
> `CARDDAV_PASS` — рекомендация для `.mlog`-кода, который сам читает
> их через `env()` и передаёт в `connect()`, не поведение самого билтина.

---

## 5. Объявления верхнего уровня

### 5.1. Pattern (функция)

```mlog
pattern Name(param1: Type, param2: Type) -> ReturnType {
  // тело
  return result
}
```

### 5.2. Learnable Pattern (AI-функция)

```mlog
learnable pattern Classify(text: String) -> Category {
  prompt: "Classify this message. Return JSON: {category, confidence}"
  context: auto
  model: "gpt-4"
  max_tokens: 100
  cache: true
  cache_ttl: 5.0 minutes
  max_context_tokens: 4000
}
```

Поля `context`:
- `context: recall("query", limit = 5)` — семантический поиск в памяти
- `context: auto` — автоматический выбор стратегии
- `context: none` — без контекста
- `context: "literal text"` — литеральный контекст

### 5.3. Entity (структура данных)

```mlog
// Определение типа
entity User {
  id: String,
  name: String,
  role: String = "viewer"    // значение по умолчанию
}

// Экземпляр (record)
entity alice: User = { id: "1", name: "Alice", role: "admin" }

// Простой entity (single value)
entity db_url: Secret = env("DATABASE_URL")
```

### 5.4. Flow (пайплайн)

```mlog
flow ProcessMessage {
  input: String = "Hello world"
  -> Normalize -> Classify -> checkpoint("classified") -> Format -> output

  Classify {
    result.confidence > 0.8 -> HighConfidenceHandler
    result.confidence < 0.3 -> LowConfidenceHandler
  }
}
```

### 5.5. Rule (правило)

```mlog
rule If(status contains "error") then alert.level = "high" with priority = 10
```

### 5.6. Server / MlogServer (HTTP-сервер)

```mlog
server {
  port: 8080
  host: "127.0.0.1"
  middleware: [session, csrf, security_headers]

  route "/" method=GET {
    respond("200 OK")
  }

  route "/admin" method=GET requires=[admin] {
    respond("200 Secret")
  }
}
```

Ключевое слово `server` — синоним `mlogserver`.

Ключи: `port` (Int, дефолт 8080), `host` (String, опционально, дефолт `"0.0.0.0"`), `middleware`, `route`.
Доступные middleware: `session`, `csrf`, `security_headers`.

### 5.7. Template (HTML-шаблон)

```mlog
template Page(title: String, body: String) -> Html {
  <!DOCTYPE html>
  <html>
  <head><title>{{ title }}</title></head>
  <body>{{ body }}</body>
  </html>
}
```

Тип возврата `Html` — непрозрачный, обеспечивает автоматическое экранирование XSS.

### 5.8. Import (модули)

```mlog
import std/string as str
import std/math
import ./my_utils
import pkg/utils as u

// Квалифицированный вызов
str.trim("  hello  ")
math.abs(-5.0)
```

### 5.9. Memory (память)

```mlog
// In-memory (по умолчанию)
memory { }

// С персистентностью в SQLite
memory { persist: "./data/memory.db" }

// С KV-конфигурацией
memory { kv: { type: key_value, persist: true } }
```

### 5.10. DB (база данных)

```mlog
db {
  url: env("DATABASE_URL")
  pool_size: 10
  migrate: "./migrations"
}
```

### 5.11. LLM (конфигурация провайдеров)

```mlog
llm {
  providers: [
    { alias: openai, provider: openai, key: env("OPENAI_KEY") },
    { alias: claude, provider: anthropic, key: env("ANTHROPIC_KEY"), url: "https://api.anthropic.com" }
  ],
  default_model: "gpt-4",
  failover: auto,
  circuit_breaker: 3,
  timeout: 30
}
```

### 5.12. Hook (lifecycle hooks, ADR-0045 + ADR-0064)

5 lifecycle points (inspired by obsidian-mind):

| Hook | Точка срабатывания | Переменные |
|------|-------------------|-------------|
| `hook on_session_start { ... }` | Начало `run()` | (нет) |
| `hook on_write { ... }` | Перед write-билтинами | `target`, `args` |
| `hook before_pattern { ... }` | Перед вызовом паттерна | `pattern_name`, `args` |
| `hook after_pattern { ... }` | После возврата паттерна | `pattern_name`, `args`, `result`, `confidence` |
| `hook on_session_end { ... }` | Конец `run()` | (нет) |

Write-билтины (триггерят `on_write`): `mem_set`, `mtree_store`, `db_execute`, `write_file`, `append_file`.

```mlog
hook on_session_start { mem_set("start_time", now_iso()) }
hook on_write { print("WRITE: " + target) }
hook before_pattern { print("calling: " + pattern_name) }
hook after_pattern { print("result: " + to_string(result)) }
hook on_session_end { print("Session done") }
```

Ошибки в хуках игнорируются (advisory, не blocking).

### 5.13. Tool (абстракция инструментов)

```mlog
tool telegram {
  send(chat_id: String, text: String) -> String {
    http_post("https://api.telegram.org/bot" + token + "/sendMessage",
      json_encode({ chat_id: chat_id, text: text }))
  }
}
```

Вызов: `telegram.send("123", "hello")`.

### 5.14. Eval (тестирование паттернов)

```mlog
eval Classify {
  dataset: [
    ("Hello", "greeting"),
    ("Fix bug #123", "task"),
    ("Please help", "question")
  ],
  metric: accuracy,
  threshold: 0.8
}
```

Запуск: `mlog eval file.mlog`.

### 5.15. Sandbox, Mutate, Adapt, Memorize, Forget, Relate

```mlog
// Sandbox (ограничение выполнения)
sandbox safe_executor {
  allowed: [upper, lower, trim, split],
  forbidden: [http_post, http_get, write_file],
  timeout: 5
}

// Mutate (адаптация с откатом)
mutate Classify {
  add_example("new input", "new output")
  rollback_if: accuracy < 0.7
}

// Adapt (добавление примера)
adapt Classify add_example("input", "output")

// Memorize / Forget (семантическая память)
memorize "important fact" with priority = 0.9
forget "outdated fact" after 30.0 days

// Relate (знаковый граф)
relate entity1 to entity2 as "relationship"
```

**Sandbox `timeout` caveat**: `timeout > 0` makes the calling thread stop *waiting* at the deadline (preemptive via `mpsc::recv_timeout`). The background HTTP request to the LLM provider may still be in flight — only the wait is cancelled, not the request itself. Full request cancellation (`reqwest::AbortHandle`) is a separate наряд.

### 5.16. Conversation (конфигурация)

```mlog
conversation {
  ttl: 1800,
  max_messages: 50,
  compress_after: 20
}
```

### 5.17. Fluid Types (вероятностные типы)

```mlog
fluid x = String["answer"][0.9] or String["question"][0.1]
```

### 5.18. Type Aliases (псевдонимы типов)

**Наряд №119.** Псевдонимы типов позволяют создать короткое имя для существующего типа. Псевдоним полностью наследует семантику целевого типа, включая защиту непрозрачных типов (например, Secret).

```mlog
type Token = Secret
type UserName = String

entity api_key: Token = "sk-..."    // получит защиту Secret
entity name: UserName = "Alice"      // эквивалентно String
```

Цепочки псевдонимов разрешаются автоматически (максимальная глубина — 10):

```mlog
type A = B
type B = String
// A → B → String
```

Циклические псевдонимы обнаруживаются на этапе запуска и вызывают ошибку:

```mlog
type X = Y
type Y = X   // ОШИБКА: cyclic type alias
```

Синтаксис не конфликтует с `type` внутри `memory { kv: { type: key_value } }` — PEG грамматика различает их по контексту.

### 5.19. Test (юнит-тесты)

```mlog
test "название теста" {
  let result = PatternName(args)
  assert_eq(result, expected)
}
```

Запуск: `mlog test file.mlog`. Флаг `--filter=substring` фильтрует тесты по подстроке в названии.

---

## 6. Stdlib (стандартная библиотека)

Стандартная библиотека находится в `std/` и подключается через `import`:

### std/string

```mlog
import std/string as str

str.trim(s: String) -> String       // trim whitespace
str.replace(s, old, new) -> String  // replace all occurrences
str.split(s, sep) -> List           // split by separator
str.join(items, sep) -> String      // join list into string
```

### std/math

```mlog
import std/math

math.abs(n: Float) -> Float         // absolute value
math.min(a, b) -> Float             // minimum
math.max(a, b) -> Float             // maximum
math.clamp(val, lo, hi) -> Float    // clamp to range
math.round(n) -> Float              // round to nearest
```

### std/collections

```mlog
import std/collections

collections.first(items: List) -> String   // first element
collections.last(items: List) -> String    // last element
collections.push(items, item) -> List      // append to list
```

> **Примечание:** Все функции stdlib также доступны как top-level builtins (без импорта):
> `trim()`, `replace()`, `split()`, `join()`, `abs()`, `min()`, `max()`, `clamp()`, `round()`, `first()`, `last()`.

---

## 7. Changelog (кратко)

| Версия | Дата | Что нового |
|--------|------|------------|
| **0.12.0** | 2025-08 | PDF builtins (pdf-inspector), typed memory (FTS5 BM25 + cosine RRF), modular builtins structure |
| **0.11.0** | 2025-07 | obsidian-mind: 5 lifecycle hooks, config_load YAML (ADR-0064, ADR-0065) |
| **0.10.0** | 2025-06 | obsidian-mind: semantic_search, config_load, vault_validate |
| **0.9.5** | 2025-06 | OpenPlanter: fuzzy, hashline, compact, budget, replay, policy (ADR-0063) |
| **0.9.4** | 2025-06 | AgentSkillOS: recipe system, DAG orchestration (ADR-0062) |
| **0.9.3** | 2025-06 | sqz: string/list/token utilities (ADR-0058+) |
| **0.9.1** | 2025-06 | Collection ops sync, BUILTIN_REGISTRY SSOT |
| **0.8.9** | 2025-06 | Fix: else-branch, BlockIfElse mutation, 4 contract tests |
| **0.8.0** | 2025-05 | Time, weather, geo, reminders, HTTP server, encryption, auth, CSRF, OWASP |
| **0.7.x** | 2025-05 | Telegram bot, memory tree L0/L1/L2, cron, goals/todos |
| **0.6.x** | 2025-05 | let/if/each/while, modules, break/continue, match |
| **0.4.0** | 2025-06-03 | Phase 6: HTTP-сервер, шаблоны, БД, шифрование, auth, CSRF, bot integration, 40+ builtins |
| **0.3.0** | — | Phase 5: let/if, each/while, List literals, строковые операции, модули, REPL |
| **0.2.0** | — | Phases 1-4: fluid types, knowledge graph, vector recall, CLI, codegen |
| **0.1.0** | — | M1-M5: entity, rule, learnable pattern, semantic memory, sandbox, adapt |

Полный CHANGELOG см. в файле [`CHANGELOG.md`](CHANGELOG.md).
Архитектурные решения см. в [`docs/adr/`](docs/adr/).