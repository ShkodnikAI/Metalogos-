# НАРЯД №12: Доработка Metalogos Runtime
## Ветка: `fix/metalogos-runtime`

## Контекст

Metalogos v0.4.0 — AI-язык программирования на Rust (бинарник `bin/mlog`, 11MB, ELF64, not stripped).
Проект Fosved Office v2 использует Metalogos в `mlog serve` режиме для Telegram-бота с 12 отделами.

Проблема: в route handlers (`mlogserver {}` блок) **недоступны** встроенные функции и пользовательские паттерны.
Это блокирует: вызов LLM, работу 12 отделов, ответы Яны.

Исходников Metalogos нет в открытом доступе — бинарник единственный артефакт.
Необходимо декомпилировать, реконструировать проблемные модули, пропатчить и пересобрать.

## Что найдено в бинарнике (strings + nm + objdump)

### Архитектура
- **Парсер**: `metalogos::parser::MlogParser` (PEG через `pest`)
- **AST**: `metalogos::ast` — Declaration, PatternDecl, RouteDecl, MlogServerDecl, Expr, Statement
- **Компилятор**: `metalogos::bytecode` — CompiledFn, CompiledLearnableInfo, BranchDef, Instruction
- **VM**: `metalogos::vm::Vm` → `Vm::execute_code()` @ 0x2f26f0
- **Интерпретатор**: `metalogos::interpreter::Interpreter`:
  - `Interpreter::new()` @ 0x286b10
  - `Interpreter::run()` @ 0x2873c0
  - `Interpreter::invoke()` @ 0x28a230
  - `Interpreter::eval_expr_with_env()` @ 0x282130
  - `Interpreter::eval_statements()` @ 0x27f9e0
- **Сервер**: `metalogos::server`:
  - `ServerState` — Axum state shared across routes
  - `route_handler` — Axum handler function
  - `merge_interpreter()` @ 0x2d8350 — КЛЮЧЕВАЯ ФУНКЦИЯ, копирует глобальные переменные из основного Interpreter в route VM
- **Builtins**: `metalogos::builtins` — все в `src/builtins.rs`:
  - `builtin_http_post()` @ 0x2a9880 — 3 параметра (url, body, content_type), БЕЗ auth headers
  - `builtin_starts_with()` @ 0x2aaee0
  - `builtin_len()` @ 0x2a5160
  - и ~40 других builtins

### Ключевые баги

1. **Builtins не резолвятся в route handlers**
   - `merge_interpreter()` копирует HashMap<String, Value> (глобальные переменные)
   - НЕ копирует: таблицу builtins, таблицу паттернов, функции call_llm
   - Результат: `Handler error: undefined pattern or builtin: <name>`

2. **http_post() — 3 параметра, без авторизации**
   - `http_post(url, body, content_type)` — нет 4-го параметра для заголовков
   - Невозможно вызвать LLM API с Authorization header
   - Сообщение об ошибке: `http_post() requires at least 1 argument (url)`

3. **__replace паникует на кириллице (Cyrillic)**
   - Паника при вызове `__replace()` на строках с не-ASCII символами

4. **Нет METALOGOS_OPENAI_BASE_URL**
   - call_llm() harcoded: `api.openai.com`, `api.anthropic.com`, `localhost` (ollama)
   - Нет поддержки кастомного base URL

## Задачи (по порядку)

### Фаза 1: Декомпиляция и анализ
1. Установить Rust toolchain (`rustup`)
2. Использовать `cargo-decompiler` или ручную декомпиляцию через `objdump` + `nm`
3. Восстановить структуру `src/server.rs` (merge_interpreter, route_handler, ServerState)
4. Восстановить структуру `src/vm.rs` (Vm struct, execute_code, builtin resolution)
5. Восстановить структуру `src/builtins.rs` (http_post signature, call_llm)

### Фаза 2: Исправление багов

**Баг 1 — Builtins в route handlers:**
- В `merge_interpreter()` или в `route_handler` → добавить копирование:
  - `builtins: HashMap<String, BuiltinFn>` (таблица всех встроенных функций)
  - `patterns: HashMap<String, CompiledFn>` (таблица пользовательских паттернов)
  - `learnables: Vec<CompiledLearnableInfo>`
- Или проще: передать ссылку на основной Interpreter в route handler (Arc<Interpreter>)

**Баг 2 — http_post с заголовками:**
- Добавить 4-й опциональный параметр: `http_post(url, body, content_type, headers?)`
- `headers` — String в формате `"Authorization: Bearer xxx\nContent-Type: application/json"`
- В `builtin_http_post()` — парсить headers и добавлять к reqwest RequestBuilder

**Баг 3 — __replace UTF-8:**
- Найти `builtin_replace()` @ 0x2a7d70
- Заменить byte-by-byte replace на char-by-char (String::chars())

**Баг 4 — METALOGOS_OPENAI_BASE_URL:**
- В `call_llm()`: проверить env `METALOGOS_OPENAI_BASE_URL`, если не пусто — использовать вместо `api.openai.com`

### Фаза 3: Сборка и тестирование
1. Создать минимальный Cargo.toml с нужными зависимостями:
   - axum, hyper, tokio, reqwest, rustls, serde, serde_json, rusqlite, pest
2. Собрать: `cargo build --release`
3. Создать тестовый `test.mlog`:
   ```
   mlogserver {
     route "/test" method=GET {
       let r = call_llm("Say OK", "test")
       let t = http_post("http://example.com", "hello", "text/plain")
       let s = __replace("Привет мир", "мир", "земля")
       return respond("200 ok r=" + r + " t=" + t + " s=" + s)
     }
   }
   ```
4. Запустить: `./target/release/mlog serve test.mlog`
5. Проверить: `curl localhost:10000/test`

### Фаза 4: Деплой
1. Скопировать новый бинарник в `/home/z/my-project/bin/mlog`
2. Вернуться в ветку `main` и замержить
3. Обновить `app.mlog` — убрать прокси, вернуть `call_llm()` и `local_patterns`
4. Тест на Render

## Артефакты
- Исходный бинарник: `/home/z/my-project/bin/mlog` (11MB, not stripped)
- nm dump: `nm /home/z/my-project/bin/mlog`
- strings dump: `strings /home/z/my-project/bin/mlog`
- Анализ merge_interpreter: objdump @ 0x2d8350..0x2d89dc

## Метрики успеха
- [ ] `call_llm()` работает в route handler
- [ ] `http_post()` принимает 4 параметра (с заголовками)
- [ ] `__replace()` работает с кириллицей
- [ ] `METALOGOS_OPENAI_BASE_URL` поддерживается
- [ ] Тестовый `test.mlog` проходит все 4 проверки
- [ ] Yana отвечает через встроенный `call_llm()` без прокси
