# METALOGOS — Справочник языка (Reference)

> **Версия:** 0.7.10 (Phase 7.10)
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
5. [Байткод VM и JIT](#5-байткод-vm-и-jit)
6. [Объявления верхнего уровня](#6-объявления-верхнего-уровня)
7. [Stdlib](#7-stdlib-стандартная-библиотека)
8. [Changelog](#8-changelog-кратко)

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
let payload = { chat_id: "123", text: "hello", urgent: true }  // struct literal
```

### 3.3. Операторы

| Категория | Операторы |
|-----------|-----------|
| Арифметические | `+`, `-`, `*`, `/` |
| Сравнение | `==`, `!=`, `>`, `<`, `>=`, `<=` |
| Логические (short-circuit) | `and`, `or` |
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

**If-then-else (тернарное выражение):**
```mlog
let label = if score >= 90.0 then "A" else "B"
```

**If-then (блочный оператор):**
```mlog
if x > 10.0 then {
  print("big")
} else if x > 5.0 then {
  print("medium")
} else {
  print("small")
}
```

**Each (цикл по коллекции):**
```mlog
each item in items {
  print(item)
}
```

**Each with index (цикл с индексом):**
```mlog
each i, item in items {
  print(to_string(i) + ": " + item)
}
```

**While (цикл с условием):**
```mlog
while count < 10.0 {
  let count = count + 1.0
}
```

**Break / Continue:**
```mlog
each item in items {
  if item == "stop" then { break }
  if item == "skip" then { continue }
  print(item)
}
```

**Match (сопоставление с образцом):**
```mlog
match command {
  "start" then { print("starting") }
  starts_with "stop" then { print("stopping") }
  contains "help" then { print("helping") }
  > 100.0 then { print("too big") }
  else { print("unknown") }
}
```

**Try (перехват ошибок):**
```mlog
let result = try risky_operation()
// Если risky_operation() вернёт ошибку, result = Unit
```

**Return:**
```mlog
return result
```

### 3.5. Строковые литералы

```mlog
let s = "hello world"
let with_escape = "line1\nline2\ttabbed"
```

Поддерживаемые escape-последовательности: `\"`, `\\`, `\n`, `\t`, `\r`, `\uXXXX` (Unicode code point, например `\u0041` = `A`).

### 3.6. Идентификаторы

Идентификаторы поддерживают ASCII, `_` и кириллицу (А-я):
```mlog
let имя = "Metalogos"
let счетчик = 0.0
pattern Приветствие(кто: String) -> String { ... }
```

---

## 4. Встроенные функции (Builtins)

Все 93 встроенные функции регистрируются в `src/builtins.rs` и доступны как в tree-walking интерпретаторе, так и в байткод VM. JIT-компилятор через Cranelift также поддерживает все builtins.

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
| `starts_with(s, prefix)` | `String, String -> Bool` | Bool | Проверяет, начинается ли строка с префикса. Также доступна как VM-инструкция `StartsWith` и в `match` как `starts_with "..."` |
| `ends_with(s, suffix)` | `String, String -> Bool` | Bool | Проверяет, заканчивается ли строка суффиксом |
| `contains(s, needle)` | `String, String -> Float` | Float | Возвращает `1.0` если содержит, `0.0` если нет. Также доступна как VM-инструкция `Contains` |
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
| `make_list(a, b, c, ...)` | `Any, ... -> List` | List | Создаёт список из произвольного числа аргументов. Потокобезопасная альтернатива write_file/read_file для возврата нескольких значений из pattern (Наряд 24) |

**Примеры:**
```mlog
let items = [10.0, 20.0, 30.0]
get(items, 1.0)        // 20.0
push(items, 40.0)      // [10.0, 20.0, 30.0, 40.0]
first(items)           // 10.0
last(items)            // 30.0
len(items)             // 3.0
reverse(items)         // [30.0, 20.0, 10.0]
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
| `call_llm(prompt, input)` | `String, String -> String` | String | Вызывает LLM-бэкенд. По умолчанию возвращает мок: `"[MOCK: prompt \| input]"`. Реальный вызов при `METALOGOS_LLM_MOCK=false`. Таймаут 120с |
| `call_claude(api_key, model, system_prompt, user_message)` | `String, String, String, String -> String` | String | Прямой вызов Anthropic Claude Messages API (v1/messages). Возвращает `content[0].text`. Таймаут 120с |
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
| `http_post_multipart(url, fields, files)` | `String, String\|Struct, String\|List -> String` | String | Multipart POST. `fields` — JSON-строка или Struct с текстовыми полями. `files` — JSON-массив `[{name, path, mime}]` или List of Structs |
| `http_post_multipart(url, fields, files, headers)` | `String, String\|Struct, String\|List, String\|Struct -> String` | String | Multipart POST с заголовками |

**Примеры:**
```mlog
// Простой POST
let resp = http_post("https://api.example.com/data", json_encode(payload))

// POST с Bearer-токеном
let resp = http_post("https://api.example.com/data", body, "application/json", env("API_TOKEN"))

// POST с кастомными заголовками
let headers = { "X-Custom": "value", "Authorization": "Bearer token123" }
let resp = http_post("https://api.example.com/data", body, "application/json", headers)

// GET
let data = http_get("https://api.example.com/users")
let data = http_get("https://api.example.com/users", env("API_TOKEN"))

// Multipart POST — отправка голосового в Telegram
let result = http_post_multipart(
  "https://api.telegram.org/bot" + token + "/sendVoice",
  { chat_id: chat_id },
  "[{\"name\":\"voice\",\"path\":\"/tmp/voice.ogg\",\"mime\":\"audio/ogg\"}]",
  token
)

// Multipart POST — Whisper transcription
let result = http_post_multipart(
  "https://api.groq.com/openai/v1/audio/transcriptions",
  "{\"model\":\"whisper-large-v3\"}",
  "[{\"name\":\"file\",\"path\":\"/tmp/voice.ogg\",\"mime\":\"audio/ogg\"}]",
  env("GROQ_KEY")
)
```

### 4.7. JSON

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `parse_json(text)` | `String -> Struct\|List\|String\|Float\|Bool\|Unit` | Any | Парсит JSON-строку. Объекты → Struct с `type_name: "Json"`, массивы → List, `null` → Unit |
| `json_encode(value)` | `Any -> String` | String | Сериализует значение в JSON-строку. Поддерживает String, Float, Bool, Unit→null, List→array, Struct→object |
| `json_get(obj, field_path)` | `Struct, String -> Value` | Value | Безопасный доступ к полю (возвращает Unit если нет поля). Unit корректно сравнивается через `==`/`!=` с любым типом. Поддерживает dot-path: `"voice.file_id"`. **Поддерживает числовые индексы массивов:** `"items.0.title"` (Наряд 24) |
| `json_get(obj, field_path, default)` | `Struct, String, Value -> Value` | Value | С дефолтным значением при отсутствии поля. Также поддерживает числовые индексы |
| `has_field(obj, field_path)` | `Struct, String -> Float` | Float | `1.0` если поле существует, `0.0` если нет. Поддерживает dot-path |
| `escape_json(text)` | `String -> String` | String | Экранирует спецсимволы для встраивания в JSON |

**Примеры:**
```mlog
let data = parse_json("{\"name\": \"Alice\", \"age\": 30}")
let name = json_get(data, "name")           // "Alice"
let missing = json_get(data, "email")        // Unit
let missing = json_get(data, "email", "none") // "none"

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
| `query_param(name)` | `String -> String` | String | Получает query-параметр из URL |

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
| `query(sql, params)` | `String, List -> Query` | Query | Создаёт параметизированный SQL-запрос (непрозрачный тип, инъекция синтаксически невозможна) |
| `db_execute(sql)` | `String -> Unit` | Unit | Выполняет SQL-запрос без возврата данных (INSERT, UPDATE, DELETE) |

> **Безопасность:** `query()` возвращает непрозрачный тип `Query` — его нельзя напечатать,
> конкатенировать со строкой или передать как строку. SQL-инъекция невозможна на уровне типа.

### 4.16. Боты (Telegram/Discord)

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `send_message(chat_id, text)` | `String\|Float, String -> String` | String | Отправляет сообщение в Telegram. При `TELEGRAM_BOT_TOKEN` env — реальная отправка через API. Поддерживает отрицательные channel ID (числа в JSON). Без токена — логирует в `[AUDIT]` |
| `whisper_transcribe(file_id, bot_token, whisper_key, provider)` | `String, String, String, String -> String` | String | STT через Whisper API: скачивает голосовое из Telegram, отправляет в OpenAI/Groq. `provider`: `"openai"` (по умолч.) или `"groq"` |
| `tts_send(text, voice, bot_token, chat_id)` | `String, String, String, String -> String` | String | TTS через OpenAI API + отправка аудио в Telegram (`sendAudio`). Требует `OPENAI_API_KEY` |

### 4.17. Интеграции и автоматизация

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `git_push(message)` | `String -> String` | String | `git add . && git commit && git push` через subprocess. Требует `GITHUB_TOKEN` и `GITHUB_REPO` env. Возвращает `"ok"` или `"nothing to commit"` |
| `web_search(query, num_results)` | `String, Float -> String` | String | Поиск через SerpAPI. Требует `SERPAPI_KEY` env. Возвращает raw JSON. `num_results` по умолчанию 10 |
| `exec(command)` | `String -> String` | String | Выполняет shell-команду. В server mode отключён без `METALOGOS_ALLOW_EXEC=1` |

### 4.18. Прочее

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `print(s)` | `String -> String` | String | Выводит строку в stdout, возвращает её же |
| `now()` | `-> Float` | Float | Текущий Unix-timestamp в секундах |
| `str(value)` | `Any -> String` | String | Преобразует любое значение в строку |
| `to_string(value)` | `Any -> String` | String | Аналог `str()` (Float без `.0` для целых) |
| `type_of(value)` | `Any -> String` | String | Возвращает имя типа значения: `"String"`, `"Float"`, `"Bool"`, `"List"`, `"Struct"`, `"Unit"`, `"Html"`, `"Query"`, `"Secret"`, `"Encrypted"`, `"Hash"`, `"Session"`, `"Fluid"`, `"HttpResponse"` |
| `format(template)` | `String -> String` | String | Позиционная интерполяция: `format("Hello {0}, you are {1}", name, age)` |

**Пример — безопасная работа с `json_get` (Unit comparison):**
```mlog
// Двухаргументная форма: возвращает Unit если поле отсутствует
let voice = json_get(data, "voice")
if voice == "" {
  // voice — Unit, сравнение Unit == "" = false
  // Безопасно: не крашит
}
// С дефолтом — проще:
let file_id = json_get(data, "voice.file_id", "")
if file_id != "" {
  send_voice(file_id)
}
```

---

## 4.18. Scoping: область видимости `let`

**Важно:** `let` внутри `if`, `while`, `each` блоков создаёт **новую лексическую переменную**, видимую только внутри этого блока. Она **не затрагивает** внешнюю переменную с тем же именем.

```mlog
let handled = 0.0
if user_id != owner_id {
  let handled = 1.0   // ← это НОВАЯ переменная, видна только здесь
}
// Внешний handled всё ещё 0.0!
```

**Как изменить внешнюю переменную:** используйте прямое присваивание (без `let`):
```mlog
let handled = 0.0
if user_id != owner_id {
  handled = 1.0   // ← присваивание в существующую переменную
}
// Теперь handled == 1.0
```

---

## 5. Байткод VM и JIT

Metalogos имеет три бэкенда выполнения:

| Бэкенд | Команда | Описание |
|--------|---------|----------|
| Tree-walking | `mlog run file.mlog` | Интерпретатор AST, полный набор функций |
| Bytecode VM | `mlog compile file.mlog` затем `mlog run file.mbc` | 44 инструкции, стековая машина |
| JIT (Cranelift) | Автоматически при наличии Cranelift | Нативный код через Cranelift |

### 5.1. Инструкции VM (44)

**Константы и переменные:** `Const`, `LoadGlobal`, `LoadGlobalByName`, `StoreGlobal`, `LoadLocal`, `StoreLocal`

**Функции:** `RegisterPattern`, `RegisterLearnable`, `CallBuiltin(arity, idx)`, `CallPattern(arity, idx)`, `Return`

**Арифметика и сравнение:** `Add`, `Sub`, `Mul`, `Div`, `Contains`, `CmpGt`, `CmpLt`, `CmpGe`, `CmpLe`, `CmpEq`, `CmpNe`

**Структуры и коллекции:** `MakeStruct(type_name, fields)`, `GetField(name)`, `IndexAccess`, `MakeList(count)`, `ListLen`, `Pop`, `StartsWith`

**Fluid types:** `MakeFluid(variant_count)`

**Управление:** `Jump(addr)`, `JumpIfNot(addr)`, `JumpIfLow(confidence, addr)`, `Halt`

**Память:** `Collapse(name)`, `Memorize(priority)`, `Recall`, `Forget(decay)`

**LLM:** `LlmCall(arity, learnable_idx)`

**Adapt/Relate/Mutate:** `Adapt(pattern_name)`, `Relate`, `Mutate { .. }`

**Пайплайны и правила:** `FlowExec { .. }`, `ExecuteRules`

### 5.2. Компиляция управляющих конструкций

Все 12 видов statements компилируются в байткод:
`LetBinding`, `Assign`, `Return`, `ExprStmt`, `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match` (все 4 arm-варианта), `Break`, `Continue`.

Циклы используют `LoopCtx` — контекст с адресом условия (для `continue`) и списком патчей (для `break`).

---

## 6. Объявления верхнего уровня

### 6.1. Pattern (функция)

```mlog
pattern Name(param1: Type, param2: Type) -> ReturnType {
  // тело
  return result
}
```

### 6.2. Learnable Pattern (AI-функция)

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

### 6.3. Entity (структура данных)

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

### 6.4. Flow (пайплайн)

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

### 6.5. Rule (правило)

```mlog
rule If(status contains "error") then alert.level = "high" with priority = 10
```

### 6.6. Server / MlogServer (HTTP-сервер)

```mlog
server {
  port: 8080
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

Доступные middleware: `session`, `csrf`, `security_headers`.

### 6.7. Template (HTML-шаблон)

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

### 6.8. Import (модули)

```mlog
import std/string as str
import std/math
import ./my_utils
import pkg/utils as u

// Квалифицированный вызов
str.trim("  hello  ")
math.abs(-5.0)
```

### 6.9. Memory (память)

```mlog
// In-memory (по умолчанию)
memory { }

// С персистентностью в SQLite
memory { persist: "./data/memory.db" }

// С KV-конфигурацией
memory { kv: { type: key_value, persist: true } }
```

### 6.10. DB (база данных)

```mlog
db {
  url: env("DATABASE_URL")
  pool_size: 10
  migrate: "./migrations"
}
```

### 6.11. LLM (конфигурация провайдеров)

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

### 6.12. Hook (хуки паттернов)

```mlog
hook before_pattern { print("calling: " + pattern_name) }
hook after_pattern { print("result: " + to_string(result) + " confidence: " + to_string(confidence)) }
```

Переменные внутри хука: `pattern_name`, `args`, `result` (только after), `confidence` (только after).

### 6.13. Tool (абстракция инструментов)

```mlog
tool telegram {
  send(chat_id: String, text: String) -> String {
    http_post("https://api.telegram.org/bot" + token + "/sendMessage",
      json_encode({ chat_id: chat_id, text: text }))
  }
}
```

Вызов: `telegram.send("123", "hello")`.

### 6.14. Eval (тестирование паттернов)

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

### 6.15. Sandbox, Mutate, Adapt, Memorize, Forget, Relate

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

### 6.16. Conversation (конфигурация)

```mlog
conversation {
  ttl: 1800,
  max_messages: 50,
  compress_after: 20
}
```

### 6.17. Fluid Types (вероятностные типы)

```mlog
fluid x = String["answer"][0.9] or String["question"][0.1]
```

---

## 7. Stdlib (стандартная библиотека)

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

## 8. Changelog (кратко)

| Версия | Дата | Что нового |
|--------|------|------------|
| **0.7.10** | 2026-06-18 | +and/or логические операторы (short-circuit), Unit == / != без краша, +\uXXXX escape, +if-then блочный оператор, query_scalar fix |
| **0.7.9** | 2026-06-15 | Наряд 24: +git_push, +web_search, +make_list, graceful unknown fn, LLM timeout 120с, json_get массивы, send_message реальный API, 100 builtins |
| **0.7.8** | 2026-06-15 | BlockIfElse expression в bytecode, format() arity fix |
| **0.7.7** | 2026-06-14 | Phase 7.7: break/continue, Match (StartsWith/Contains/Compare), компилятор полн. покрытие, 44 VM-инструкций |
| **0.7.1** | 2026-06-10 | Phase 7.1–7.2: inspect, контекст, события, conversation state, LLM cache, model routing |
| **0.7.3** | 2026-06-12 | Phase 7.3–7.4: контекстная компрессия, lifecycle, Tool, Hook, DoD |
| **0.7.5** | 2026-06-13 | Phase 7.5–7.6: memory persistence, tokens, eval harness, session memory, audit |
| **0.6.0** | 2025-06-03 | Phase 6: HTTP-сервер, шаблоны, БД, шифрование, auth, CSRF, bot integration |
| **0.5.0** | — | Phase 5: let/if, each/while/break/continue, match, List, строки, модули, bytecode VM, JIT |
| **0.3.0** | — | Phases 1-4: fluid types, knowledge graph, vector recall, CLI, LSP, mlogpkg, codegen |
| **0.1.0** | — | M1-M5: entity, rule, learnable pattern, semantic memory, sandbox, adapt |

Полный CHANGELOG см. в файле [`CHANGELOG.md`](CHANGELOG.md).
Архитектурные решения см. в [`docs/adr/`](docs/adr/).