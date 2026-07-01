# METALOGOS — Справочник языка (Reference)

> **Версия:** 0.8.1 (Phase 8.1)
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
   - [Время, дата, календарь](#417-время-дата-календарь)
   - [Геолокация](#418-геолокация)
   - [Погода](#419-погода)
   - [Напоминания и таймеры](#420-напоминания-и-таймеры)
   - [Интеграции и автоматизация](#421-интеграции-и-автоматизация)
   - [Human Intelligence Layer](#422-human-intelligence-layer-openhuman-inspired)
   - [Прочее](#423-прочее)
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

Все 108 встроенных функций регистрируются в `src/builtins.rs` и доступны как в tree-walking интерпретаторе, так и в байткод VM. JIT-компилятор через Cranelift также поддерживает все builtins.

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

### 4.17. Время, дата, календарь

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `now()` | `-> Float` | Float | Текущий Unix-timestamp в секундах (alias: `time()`) |
| `format_date(fmt, timestamp?)` | `String, Float? -> String` | String | Форматирует timestamp (или текущее время) по шаблону. Поддерживает: `%Y` (год), `%y` (2 цифры), `%m` (месяц), `%d` (день), `%H` (часы 24), `%I` (часы 12), `%M` (минуты), `%S` (секунды), `%p` (AM/PM), `%A` (день недели), `%a` (сокр.), `%B` (месяц), `%b` (сокр.), `%j` (день года), `%w` (weekday 0=Mon), `%W` (неделя), `%F` (YYYY-MM-DD), `%T` (HH:MM:SS), `%R` (HH:MM) |
| `date_parts(timestamp?)` | `Float? -> Struct {Date}` | Struct | Возвращает struct: `year`, `month`, `day`, `hour`, `minute`, `second`, `weekday`, `weekday_name`, `month_name`, `day_of_year`, `week_number`, `timestamp` |
| `days_between(ts1, ts2)` | `Float, Float -> Float` | Float | Абсолютная разница в днях между двумя timestamps |
| `days_in_month(year, month)` | `Float, Float -> Float` | Float | Количество дней в месяце (1-12). Учитывает високосные года |
| `is_leap_year(year)` | `Float -> Bool` | Bool | Проверяет, является ли год високосным |
| `add_days(timestamp, days)` | `Float, Float -> Float` | Float | Прибавляет/вычитает дни к timestamp |
| `add_hours(timestamp, hours)` | `Float, Float -> Float` | Float | Прибавляет/вычитает часы к timestamp |
| `weekday_name(timestamp)` | `Float -> String` | String | Полное название дня недели: "Monday".."Sunday" |

**Примеры:**
```mlog
let ts = now()
print(format_date("%Y-%m-%d %H:%M:%S", ts))  // "2026-07-01 14:30:00"
print(format_date("%d.%m.%Y"))               // текущая дата: "01.07.2026"

let dp = date_parts(ts)
print(dp.weekday_name)   // "Tuesday"
print(dp.week_number)    // "27"

print(days_in_month(2026.0, 2.0))   // 28.0
print(is_leap_year(2024.0))          // true
print(add_days(ts, 7.0))            // timestamp через 7 дней
print(days_between(ts, add_days(ts, 3.0)))  // 3.0
print(weekday_name(ts))             // "Tuesday"
```

### 4.18. Геолокация

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `geo_ip(ip?)` | `String? -> Struct {GeoLocation}` | Struct | Геолокация по IP. Без аргумента — текущий IP. Использует ip-api.com (бесплатно, без ключа). Поля: `ip`, `city`, `region`, `country`, `country_code`, `lat`, `lon`, `isp`, `timezone` |
| `geo_distance(lat1, lon1, lat2, lon2, unit?)` | `Float, Float, Float, Float, String? -> Float` | Float | Расстояние по формуле гаверсинуса. `unit`: `"km"` (по умолч.), `"mi"`, `"nm"`, `"m"` |

**Примеры:**
```mlog
// Определить местоположение по IP
let loc = geo_ip()
print(loc.city)          // "Minsk"
print(loc.country_code)  // "BY"
print(loc.lat)           // 53.9

// Расстояние между городами
let d = geo_distance(53.9, 27.57, 55.75, 37.62)  // Минск — Москва
print(d)                  // ~690 km
let d_mi = geo_distance(53.9, 27.57, 55.75, 37.62, "mi")
print(d_mi)              // ~429 mi
```

### 4.19. Погода

Погодные функции используют **Open-Meteo API** — полностью бесплатно, **без API-ключа**, без регистрации. Город автоматически разрешается в координаты через Open-Meteo Geocoding.

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `weather(city_or_lat, lon?)` | `String \| Float, Float? -> Struct {Weather}` | Struct | Текущая погода. `weather("Minsk")` или `weather(53.9, 27.57)`. Поля: `temp`, `feels_like`, `temp_min`, `temp_max`, `humidity`, `description`, `wind_speed`, `wind_direction`, `pressure`, `cloud_cover`, `is_day`, `city`, `country` |
| `weather_forecast(city_or_lat, lon_or_days?, days?)` | `String \| Float, Float?, Float? -> List` | List | Прогноз на 1–16 дней. `weather_forecast("Minsk")` (7 дней), `weather_forecast("Minsk", 3)`, `weather_forecast(53.9, 27.57, 14)`. Каждый элемент — struct `DayForecast`: `date`, `temp_max`, `temp_min`, `precipitation`, `weather_code`, `description`, `wind_speed_max`, `sunrise`, `sunset`, `uv_index` |

**Примеры:**
```mlog
// Текущая погода
let w = weather("Minsk")
print("Температура: " + to_string(w.temp) + "°C")
print("Ощущается: " + to_string(w.feels_like) + "°C")
print(w.description)              // "Partly cloudy"
print("Влажность: " + to_string(w.humidity) + "%")
print("Ветер: " + to_string(w.wind_speed) + " km/h")
print("День: " + to_string(w.is_day))  // 1.0 = день, 0.0 = ночь

// По координатам
let w2 = weather(40.71, -74.01)   // New York

// Прогноз на 3 дня
let forecast = weather_forecast("Minsk", 3)
each day in forecast {
  print(day.date + ": " + to_string(day.temp_min) + ".." + to_string(day.temp_max) + "°C, " + day.description)
}

// Полный прогноз (по умолчанию 7 дней, макс 16)
let week = weather_forecast("London", 14)
each day in week {
  print(day.date + " | " + day.description + " | UV: " + day.uv_index)
}
```

### 4.20. Напоминания и таймеры

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `remind(message, timestamp, data?)` | `String, Float, String? -> String` | String | Одноразовое напоминание. Возвращает ID. `timestamp` — когда сработать (Unix). `data` — произвольные данные |
| `remind_recurring(message, interval_seconds, data?)` | `String, Float, String? -> String` | String | Повторяющееся напоминание. `interval_seconds` — период в секундах (86400 = день, 604800 = неделя) |
| `cancel_remind(id)` | `String -> String` | String | Отменяет напоминание. Возвращает `"ok"` или `"not_found"` |
| `list_reminders()` | `-> List` | List | Список активных напоминаний. Каждый элемент — struct `Reminder`: `id`, `message`, `fire_at`, `interval`, `next_fire`, `data`, `created_at`, `type` |
| `check_reminders()` | `-> List` | List | Возвращает просроченные напоминания (DueReminder). Одноразовые деактивируются, повторяющиеся сдвигаются на следующий период |

**Примеры:**
```mlog
// Напоминание через 1 час
let in_1h = add_hours(now(), 1.0)
let id = remind("Перезвонить клиенту", in_1h, "phone:+375291234567")

// Ежедневное напоминание (каждые 24 часа)
let daily_id = remind_recurring("Ежедневный отчёт", 86400.0, "report:daily")

// Еженедельное напоминание
let weekly_id = remind_recurring("Плёнка", 604800.0)

// Проверить сработавшие
let due = check_reminders()
each r in due {
  print("НАПОМИНАНИЕ: " + r.message + " (data: " + r.data + ")")
}

// Отменить
cancel_remind(daily_id)

// Список всех активных
let active = list_reminders()
each r in active {
  print(r.type + ": " + r.message + " next=" + format_date("%Y-%m-%d %H:%M", r.next_fire))
}
```

### 4.21. Интеграции и автоматизация

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `git_push(message)` | `String -> String` | String | `git add . && git commit && git push` через subprocess. Требует `GITHUB_TOKEN` и `GITHUB_REPO` env. Возвращает `"ok"` или `"nothing to commit"` |
| `web_search(query, num_results)` | `String, Float -> String` | String | Поиск через SerpAPI. Требует `SERPAPI_KEY` env. Возвращает raw JSON. `num_results` по умолчанию 10 |
| `exec(command)` | `String -> String` | String | Выполняет shell-команду. В server mode отключён без `METALOGOS_ALLOW_EXEC=1` |

### 4.22. Human Intelligence Layer (OpenHuman-inspired)

Система персон (вдохновлённая [OpenHuman](https://github.com/tinyhumansai/OpenHuman)) — персоны с чертами характера, память, отслеживание настроения, человекоподобные AI-ответы. Построена на существующих примитивах Metalogos (KV-хранилище для персистентности, `call_llm` для генерации). Без новых зависимостей, без API-ключей сверх провайдера LLM.

#### `human_create(name, traits)` → Struct {Persona}

Создаёт или обновляет персону с именем и описанием характера. Хранит в KV под `human_persona:{name}`.

```mlog
human_create("Alice", "friendly, professional, curious, speaks Russian")
human_create("Support Bot", "patient, helpful, technical, concise")
```

Возвращает:
```
Struct {Persona} {
  name: String,          // имя персоны
  traits: String,        // описание характера
  created_at: Float,     // timestamp создания
  memory_count: Float    // количество сохранённых воспоминаний
}
```

#### `human_mood(persona, mood?, intensity?)` → Struct {Mood}

Получает или устанавливает эмоциональное состояние персоны. Настроение влияет на тон генерируемых ответов.

- **1 аргумент** — получить текущее настроение.
- **2+ аргумента** — установить настроение. `intensity` от 0.0 до 1.0 (по умолчанию 0.5).

Примеры mood: `"happy"`, `"sad"`, `"focused"`, `"creative"`, `"neutral"`, `"excited"`, `"calm"`, `"anxious"`.

```mlog
// Установить настроение
human_mood("Alice", "excited", 0.9)

// Получить текущее настроение
let m = human_mood("Alice")
print(m.mood + " (intensity: " + to_string(m.intensity) + ")")
```

Возвращает:
```
Struct {Mood} {
  persona: String,       // имя персоны
  mood: String,          // текущее настроение
  intensity: Float,      // интенсивность 0.0–1.0
  updated_at: Float      // timestamp последнего обновления
}
```

#### `human_remember(persona, key, content, importance?)` → String

Сохраняет воспоминание в дерево памяти персоны. `importance` от 0.0 до 1.0 (по умолчанию 0.5) — выше значит вспоминается первым.

```mlog
human_remember("Alice", "user_name", "Sergei from Minsk", 0.9)
human_remember("Alice", "preference", "prefers concise responses", 0.7)
human_remember("Alice", "project", "building AI assistant in Metalogos", 0.8)
human_remember("Alice", "meeting_2026_07_01", "Discussed roadmap for Phase 9", 0.6)
```

Возвращает: `"ok"`.

#### `human_forget(persona, key?)` → String | Float

Удаляет воспоминания. С 2 аргументами — конкретное по ключу (возвращает `"ok"` / `"not_found"`). С 1 аргументом — ВСЕ воспоминания персоны (возвращает количество удалённых).

```mlog
human_forget("Alice", "meeting_2026_07_01")  // "ok" или "not_found"
human_forget("Alice")  // удалит все, вернёт количество (например, 3.0)
```

#### `human_recall(persona, query, limit?)` → List of Struct {Memory}

Поиск воспоминаний персоны по ключевым словам. Возвращает список, отсортированный по composite score (50% релевантность + 30% важность + 20% свежесть). Поле `score` отражает итоговый рейтинг.

```mlog
let memories = human_recall("Alice", "project AI", 3)
each mem in memories {
  print(mem.key + ": " + mem.content)
  print("  importance=" + to_string(mem.importance) + " score=" + to_string(mem.score))
}
```

Возвращает список:
```
Struct {Memory} {
  key: String,           // ключ воспоминания
  content: String,       // содержание
  importance: Float,     // важность 0.0–1.0
  created_at: Float,     // timestamp создания
  access_count: Float,   // количество обращений
  relevance: Float,      // релевантность запросу 0.0–1.0
  score: Float           // composite score (итоговый рейтинг)
}
```

**Алгоритм скоринга:**
- **Релевантность (50%)** — доля совпавших слов из запроса в content/key
- **Важность (30%)** — напрямую из `importance` при сохранении
- **Свежесть (20%)** — экспоненциальное затухание с периодом полураспада ~1 неделя (168 часов)

#### `human_respond(persona, message, context?)` → String

Генерирует человекоподобный ответ используя характер, настроение и воспоминания персоны + LLM. Автоматически вызывает `human_recall` для поиска релевантных воспоминаний.

```mlog
// Без дополнительного контекста
let reply = human_respond("Alice", "Как дела с моим проектом?")
print(reply)

// С контекстом разговора
let reply2 = human_respond("Alice", "А что насчёт дедлайна?", "Мы обсуждали Phase 9")
print(reply2)
```

В mock-режиме (`METALOGOS_LLM_MOCK=true`, по умолчанию) возвращает: `[Alice (mood: excited): Как дела с моим проектом?]`. При реальном LLM-провайдере генерирует полный ответ в характере персоны с учётом настроения и памяти.

#### `human_personas()` → List of Struct {PersonaSummary}

Список всех созданных персон с текущим настроением и количеством воспоминаний.

```mlog
let all = human_personas()
each p in all {
  print(p.name + " (" + p.mood + "): " + to_string(p.memory_count) + " memories")
}
```

#### `human_delete(persona)` → Struct {DeleteResult}

Удаляет персону и все её воспоминания.

```mlog
let result = human_delete("Old Bot")
print(result.status)  // "deleted" или "not_found"
print(to_string(result.deleted_memories))  // количество удалённых воспоминаний
```

Возвращает:
```
Struct {DeleteResult} {
  deleted_memories: Float,  // количество удалённых воспоминаний
  status: String            // "deleted" или "not_found"
}
```

**Пример — полноценный чат-бот с персональностью:**
```mlog
// Инициализация
human_create("Assistant", "helpful, technical, friendly, speaks Russian")
human_remember("Assistant", "system", "Metalogos v0.8.1 — AI-native язык программирования", 1.0)
human_remember("Assistant", "owner", "Sergei, создатель Metalogos", 0.9)
human_mood("Assistant", "focused", 0.8)

// Сохраняем факт из разговора
human_remember("Assistant", "user_question", "User asked about Phase 9 roadmap", 0.5)

// Генерируем ответ
let answer = human_respond("Assistant", "Что ты знаешь о Metalogos?")
print(answer)

// Проверяем что запомнили
let mem = human_recall("Assistant", "Metalogos", 1)
print("Recalled: " + first(mem).content)
```

### 4.23. Прочее

| Функция | Сигнатура | Возврат | Описание |
|---------|-----------|---------|----------|
| `print(s)` | `String -> String` | String | Выводит строку в stdout, возвращает её же |
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
| **0.8.0** | 2026-07-01 | +Время/дата/календарь (format_date, date_parts, days_between, add_days, add_hours, weekday_name, is_leap_year, days_in_month), +Геолокация (geo_ip, geo_distance), +Погода Open-Meteo бесплатно без ключа (weather, weather_forecast), +Напоминания с рекурренцией (remind, remind_recurring, cancel_remind, list_reminders, check_reminders), 108 builtins |
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