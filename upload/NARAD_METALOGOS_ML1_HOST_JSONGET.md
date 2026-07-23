# Наряд — Metalogos: ML-1, ключ `host:` в mlogserver + фикс json_get на SQL NULL

Репозиторий: `ShkodnikAI/Metalogos-` (НЕ FOSVED-office-v2)
Исполнитель: агент-язык (Metalogos)
Дата: 2026-07-23
Версия наряда: 1
Версия языка: 0.9.5 (последний коммит d931483)
Тип: правка компилятора. Обязательны: правка исходников → сборка → замена bin/mlog в Office → коммит+пуш обоих репо → запись в тех-документацию.

## Две задачи прохода

- **Задача A (баг №2):** блок `mlogserver` принимает ключ `host:`, сервер биндится на указанный адрес вместо жёстко зашитого `0.0.0.0`. Закрывает гонку определения порта на Render (mlog торчит наружу и перехватывает primary port).
- **Задача B (баг №1):** `json_get(row, key, default)` возвращает `default`, когда значение поля — SQL NULL (сейчас возвращает Unit, что роняет конкатенацию `String + Unit`). Убирает необходимость COALESCE-обхода по всему Office.

## Установленные факты (проверены аудитором на коде 0.9.5, не перепроверять)

Задача A — 4 точки:
- `src/grammar.pest:55` — правило `mlogserver_port`, рядом нет `mlogserver_host`.
- `src/grammar.pest:54` — `mlogserver_body` перечисляет разрешённые секции.
- `src/grammar.pest:268` — `step_ident` содержит список зарезервированных слов; `host` туда добавить, иначе слово станет ломать идентификаторы.
- `src/ast.rs:106` — `struct MlogServerDecl { port, middleware, routes }`, поля host нет.
- `src/parser.rs:104` — `parse_mlogserver_decl`, парсит port и middleware.
- `src/server.rs:278,280` — println и `bind(format!("0.0.0.0:{}", port))`. `config: MlogServerDecl` уже доступен здесь (port берётся из `config.port` на строке 202), поэтому host прокидывается тем же config без изменения сигнатур.

Задача B — 1 точка:
- `src/builtins.rs:1609` — в ветке `json_get` с default, после навигации пути, `Ok(current.clone())` возвращает найденное значение даже если это Unit. SQL NULL становится Unit ещё в `interpreter.rs` (строки 997/1073/1121, `ValueRef::Null => Value::Unit`) — эти строки НЕ трогать, они корректны для 2-аргументной формы. Чинить в json_get.

## Запреты прохода

- Задача B: НЕ менять маппинг `ValueRef::Null => Value::Unit` в interpreter.rs. Только поведение json_get при наличии default.
- Задача A: `host` — опциональный. Отсутствие `host:` в блоке → дефолт `"0.0.0.0"` (обратная совместимость: все существующие .mlog без host работают как раньше).
- НЕ менять номер порта по умолчанию, сигнатуры run_server/build_state.
- НЕ трогать другие builtins, другие правила грамматики.
- Каждая задача — отдельный коммит (A и B раздельно, чтобы откатывались независимо).

## Перед началом

1. `git fetch && git rev-parse HEAD` → зафиксировать (ожидание d931483 или новее).
2. `cargo build --release --bin mlog` на чистом дереве → убедиться, что базовая сборка зелёная ДО правок. Если красная — стоп, доклад (проблема не наша).
3. **`cargo test` НЕ используется как гейт** — на 0.9.5 тестовый крейт не компилируется (15 ошибок: отсутствует `PartialEq` на `Value`, устаревший паттерн `Value::Struct(fields)`, вызов удалённого `json_to_mlog_value`). Это установленный факт, не чинить в рамках ML-1. Вместо теста-гейта используются smoke-проверки A.5/B.2 и корпус из D.3.
4. Ветка `feat/ml1-host-and-jsonget`.

## Задача A — ключ host в mlogserver

### A.1. Грамматика (grammar.pest)
- Строка 54, `mlogserver_body`: добавить `mlogserver_host?` в последовательность, после `mlogserver_port?`:
  `mlogserver_body = { mlogserver_port? ~ mlogserver_host? ~ mlogserver_middleware? ~ route_decl* ~ WHITESPACE* }`
- После строки 55 добавить правило:
  `mlogserver_host = { "host" ~ COLON ~ STRING_LITERAL }`
- Строка 268, `step_ident`: добавить `"host"` в список исключений (в любое место внутри перечисления зарезервированных слов).

### A.2. AST (ast.rs)
- Строка 106, `MlogServerDecl`: добавить поле `pub host: Option<String>,`.

### A.3. Парсер (parser.rs)
- В `parse_mlogserver_decl` (строка 104): после блока разбора port добавить разбор host:
```rust
let host: Option<String> = body_children.iter()
    .find(|c| c.as_rule() == Rule::mlogserver_host)
    .and_then(|c| find_child_str(&children_of(c), Rule::STRING_LITERAL))
    .map(|s| s.trim_matches('"').to_string());
```
- В конструкторе `MlogServerDecl { port, middleware, routes }` добавить `host`.
- Если STRING_LITERAL приходит с кавычками — снять их (trim_matches, как показано). Проверить по образцу, как это делает parse_route_decl для path.

### A.4. Сервер (server.rs)
- Около строки 202, где `let port = config.port;` — добавить:
  `let host = config.host.clone().unwrap_or_else(|| "0.0.0.0".to_string());`
- Строка 278: `println!("mlog serve: listening on {}:{}", host, port);`
- Строка 280: `let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;`
- Строка 313 (тестовый серверный bind на 0.0.0.0:0) — НЕ трогать.

### A.5. Проверка задачи A
- `cargo build --release` — зелёная.
- Тест-файл вне репо `/tmp/host_test.mlog`:
```
mlogserver {
  port: 10011
  host: "127.0.0.1"
  route "/" method=GET { return respond("200 ok") }
}
```
  `./target/release/mlog serve /tmp/host_test.mlog &` → в выводе `listening on 127.0.0.1:10011`.
  `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:10011/` → 200.
  `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:10011/ --interface 0.0.0.0` не обязателен; достаточно проверить, что на 127.0.0.1 отвечает.
- Обратная совместимость: тот же файл БЕЗ строки `host:` → `listening on 0.0.0.0:10011`, 200. Оба варианта в отчёт дословно.

## Задача B — json_get на NULL

### B.1. builtins.rs
- В `builtin_json_get`, ветка `if args.len() >= 3` (default передан), строка ~1609, перед финальным `Ok(current.clone())`:
```rust
// SQL NULL / missing value maps to Value::Unit; treat as "use default"
if matches!(current, Value::Unit) {
    return Ok(default_val);
}
Ok(current.clone())
```
- 2-аргументную ветку (без default) НЕ трогать — там Unit корректен.

### B.2. Проверка задачи B
- Тест-файл `/tmp/jsonget_test.mlog` с `db { url: "sqlite::memory:" }`:
```
db { url: "sqlite::memory:" }
pattern T(x: String) -> String {
  let _ = db_execute("CREATE TABLE t (a TEXT, b TEXT)")
  let _ = db_execute("INSERT INTO t (b) VALUES ('bee')")
  let rows = query("SELECT a, b FROM t", [])
  let out = ""
  each row in rows {
    let av = json_get(row, "a", "DEFLT")
    let out = out + "[" + av + "]"
  }
  return out
}
flow Main { input: String = "z" -> T -> output }
```
  `./target/release/mlog run /tmp/jsonget_test.mlog` → ожидание `[DEFLT]`, БЕЗ `type mismatch`. Дословный вывод в отчёт.
- Регресс-контроль: тот же вызов, но поле НЕ null (`INSERT INTO t (a,b) VALUES ('foo','bee')`) → `[foo]`. Оба в отчёт.

## Сборка и доставка бинарника

### D.1. Финальная сборка
- `cargo build --release --bin mlog`, зафиксировать путь `target/release/mlog` и размер.
- `./target/release/mlog --version` → 0.9.5 (или обновлённая, если версию поднимаете — указать в отчёте).
- `cargo test` НЕ запускать как гейт (не компилируется, см. «Перед началом»).

### D.3. Регресс-корпус вместо тестов (обязательно)
Поскольку `cargo test` недоступен, регрессию ловим прогоном реальных программ. На новом бинарнике выполнить и приложить дословный вывод:
1. Smoke языка: любой существующий пример из репозитория (например `m1_hello.mlog`) → работает как раньше.
2. `A.5` (host с ключом и без) — оба варианта.
3. `B.2` (json_get с NULL и с непустым значением) — оба варианта.
4. Проверка, что старые .mlog без `host:` и без изменений в json_get ведут себя как до правок — на любом втором примере из репозитория.
Любое расхождение с ожидаемым — стоп, доклад.

### D.2. Замена бинарника в Office + ОБЯЗАТЕЛЬНАЯ проверка совместимости

Курс владельца: прод переходит на новый бинарник 0.9.5. Но переключение проходит через гейт совместимости — `app.mlog` писался под 0.8.9, а между версиями Metalogos синтаксис менялся (факт: на 0.7.10 `app.mlog` даёт parse error на `let ... = read_file(...)`, строка 411 — синтаксис `let` расходится между версиями). Значит 0.9.5 тоже может не принять часть кода.

Порядок:
1. Скопировать `target/release/mlog` (0.9.5) в `FOSVED-office-v2/bin/mlog`, ветка `chore/mlog-0.9.5`.
2. **Гейт совместимости** — на новом бинарнике:
   `./bin/mlog check app.mlog --root app.mlog 2>&1` → вывести ВСЕ строки `parse error` и `error:` (не `undefined function`, не `expects N arguments` — это шум кросс-модульной проверки).
   - Ноль parse error → app.mlog синтаксически совместим, продолжаем.
   - Есть parse error → СТОП. Составить список: файл, строка, конструкция, сообщение. Это отдельный наряд миграции app.mlog под 0.9.5. НЕ править app.mlog в рамках ML-1, НЕ мёржить, НЕ деплоить. Доложить список аудитору.
   ПРИМЕЧАНИЕ: риск трёх новых reserved слов (`schema`, `skill_index`, `context_budget`) аудитором УЖЕ проверен — ноль упоминаний в app.mlog и во всех dept/*.mlog. Гейт нужен для выявления прочих, не предсказанных расхождений.
3. Если гейт чист — прогнать также `./bin/mlog check dept/*.mlog --root app.mlog` по каждому файлу отдела: те же критерии.
4. Дословный вывод гейта — в отчёт.

Office-ветка с новым bin/mlog НЕ мёржится и НЕ деплоится без разрешения аудитора, даже при чистом гейте — деплой пойдёт отдельным шагом с прод-проверкой (и порт-гонка баг №2 к тому моменту уже закрыта host-ключом, что и была цель).

## Блок M — мёрж (ТОЛЬКО по разрешению аудитора)

Не выполнять, пока аудитор не написал: «ML-1 принят, мёрж разрешён».
- Metalogos: PR ветки `feat/ml1-host-and-jsonget` в main, мёрж, сообщить хеш.
- Office: ветка с новым bin/mlog остаётся НЕмёрженой до отдельного наряда-верификации на 0.9.5.

## Тех-документация (обязательно, до сдачи)

Обновить в репозитории Metalogos:
- Описание блока `mlogserver`: добавить ключ `host: String` (опциональный, дефолт "0.0.0.0"), пример.
- CHANGELOG / ADR: запись про два фикса — host в mlogserver и json_get NULL→default. Указать номер наряда ML-1 и дату.
- Если есть файл со списком builtins/поведением json_get — отметить, что при default значении SQL NULL возвращает default.

## Структура отчёта (вне репо; содержимое в чат)

```
# Отчёт ML-1
Базовый хеш Metalogos. Ветка. cargo test ДО (число).
## A.1-A.4: для каждого файла — фрагмент до/после
## A.5: вывод serve с host и без host (оба), коды curl
## B.1: фрагмент до/после
## B.2: вывод run с null-полем ([DEFLT]) и с непустым ([foo])
## D.1: cargo build статус, cargo test ПОСЛЕ (число, сравнение с ДО), --version
## D.2: хеш Office-коммита с новым bin/mlog (ветка, НЕ мёржена)
## Документация: что обновлено, ссылки на файлы
## Итог
```

## Чек-лист сдачи

- [ ] Базовая сборка зафиксирована ДО правок (cargo test НЕ гейт — не компилируется)
- [ ] D.3: регресс-корпус прогнан, дословные выводы в отчёте
- [ ] A: host добавлен в 4 файлах (grammar/ast/parser/server) + в step_ident
- [ ] A: host опционален, без него дефолт 0.0.0.0 (обратная совместимость проверена)
- [ ] A: serve с host="127.0.0.1" реально биндится на 127.0.0.1
- [ ] B: json_get с default на NULL возвращает default, без type mismatch
- [ ] B: json_get на непустом значении возвращает значение (регресс-контроль)
- [ ] interpreter.rs ValueRef::Null маппинг НЕ тронут
- [ ] bin/mlog в Office обновлён на 0.9.5, отдельная ветка
- [ ] Гейт совместимости: app.mlog + dept/*.mlog проверены на 0.9.5, вывод parse error в отчёте
- [ ] При parse error — app.mlog НЕ правился, составлен список для отдельного наряда миграции
- [ ] Office-ветка НЕ мёржена и НЕ задеплоена (ждёт разрешения аудитора)
- [ ] Тех-документация обновлена (mlogserver host + json_get)
- [ ] Два отдельных коммита (A и B) для независимого отката
- [ ] Мёрж Metalogos только после разрешения аудитора
