# Наряд — Metalogos M2: else-ветка не исполняется

Проект: Metalogos (ShkodnikAI/Metalogos-)
Слой: 1 (интерпретатор молча пропускает код пользователя)
Дата: 2026-07-04
Версия наряда: 1
База: HEAD 2e89b81; дефект подтверждён запуском на 0.7.10 и 0.8.8
Зависимость: наряд M0 (restore-build) принят. С нарядом M1 не пересекается по файлам-строкам, но при параллельном исполнении — согласовать merge.

## Цель прохода

Ветка `else` (и мутации переменных внутри веток if/else-выражений) исполняется во всех формах if.

## Контекст дефекта (все факты проверены запуском)

Программа:
```mlog
let c = x == "no"        // false
if c then { return "then" } else { return "else" }
return "after"
```
Вывод: `after`. Ветка else не исполнилась. То же — в многострочной форме, то же — с мутациями вместо return. Обе версии бинарника.

Корень: src/parser.rs:1281-1317 (сборка `if_then_stmt`). Цикл распознаёт else-контент двумя способами: `Rule::statement` при `in_else == true` и текстовое сравнение `child.as_str().trim() == "else"`. Оба не срабатывают: по грамматике (src/grammar.pest:209-210) statements ветки else вложены в узел `Rule::else_block` и не являются прямыми детьми `if_then_stmt`, а `as_str()` у `else_block` — весь текст `else { ... }`, не слово `else`. Ветка `Rule::else_block` в match не обрабатывается → `else_body = None` → узел собирается как `Statement::IfThen`, else отбрасывается без ошибки и без предупреждения.

Второй дефект той же области: src/interpreter.rs:3508-3525 (`Expr::BlockIfElse`) исполняет каждую ветку на клоне окружения (`let mut local_env = env.clone()`), из-за чего мутации `let` внутри веток if/else-выражения теряются.

## Запреты прохода

- Не трогать грамматику выражений (скобки, приоритет) — наряд M1.
- Не добавлять новые формы if (if-выражения со statement-блоками как значение let — отдельная очередь).
- Не рефакторить parse_single_statement целиком — только ветка else.
- Ничего не удалять.

## Перед началом

1. Ветка: `git checkout -b fix/else-branch` (от результата M0).
2. Лог: `docs/naryads/2026-07-04-else-branch/report.md`.
3. Контракты в `examples/contracts/else_branch/` (создать до правок, зафиксировать текущий вывод):

`e1_oneline_return.mlog` (ожидаемо `else`, сейчас `after`):
```mlog
pattern T(x: String) -> String {
  let c = x == "no"
  if c then { return "then" } else { return "else" }
  return "after"
}
flow Main { input: String = "abc" -> T -> output }
```
`e2_multiline_mutation.mlog` (ожидаемо `else`, сейчас пустая строка):
```mlog
pattern T(x: String) -> String {
  let r = ""
  let c = x == "no"
  if c then {
    let r = "then"
  } else {
    let r = "else"
  }
  return r
}
flow Main { input: String = "abc" -> T -> output }
```
`e3_then_still_works.mlog` (ожидаемо `then` — регресс-защита):
```mlog
pattern T(x: String) -> String {
  let r = ""
  let c = x != "no"
  if c then { let r = "then" } else { let r = "else" }
  return r
}
flow Main { input: String = "abc" -> T -> output }
```
`e4_else_if.mlog` (ожидаемо `second`):
```mlog
pattern T(x: String) -> String {
  let a = x == "no"
  let b = x == "abc"
  if a then { return "first" } else if b then { return "second" } else { return "third" }
  return "after"
}
flow Main { input: String = "abc" -> T -> output }
```

## Блок A. Парсер

### A.1. Обработка Rule::else_block

- Где: src/parser.rs:1287-1317, цикл `for child in &it_children`.
- Что не так: нет ветки match для `Rule::else_block`; текстовый детектор `child.as_str().trim() == "else"` мёртв.
- Что сделать: добавить в match ветку:
```rust
Rule::else_block => {
    let eb: Vec<Statement> = children_of(child).iter()
        .filter(|c| c.as_rule() == Rule::statement)
        .map(|c| parse_single_statement(c.clone()))
        .collect();
    else_body = Some(eb);
}
```
Ветки `Rule::statement` c флагом `in_else` и текстовый детектор оставить (не удалять), но они станут недостижимыми — пометить комментарием.
- Как проверить: `mlog run examples/contracts/else_branch/e1_oneline_return.mlog` → `else`; e4 → `second`.

### A.2. Тот же дефект в block_if_else_expr

- Где: src/parser.rs, сборка `Rule::block_if_else_expr` (район строки 1537 и выражение-ветка; локализовать `grep -n "block_if_else_expr" src/parser.rs`).
- Что не так: проверить тем же методом — обрабатывается ли `Rule::else_block` как ребёнок; по симметрии с A.1 вероятен тот же пропуск.
- Что сделать: если пропуск подтверждается — та же правка, что A.1. Если else_block обрабатывается — зафиксировать в отчёте «дефект отсутствует» с номером строки.
- Как проверить: контракт с if/else-выражением в позиции значения (создать e5 по образцу e1, но в форме `let v = if c { ... } else { ... }`, если форма парсится; если не парсится — записать в отчёт и пропустить).

## Блок B. Интерпретатор

### B.1. Мутации в ветках Expr::BlockIfElse теряются

- Где: src/interpreter.rs:3508-3525.
- Что не так: `let mut local_env = env.clone()` для каждой ветки; изменения переменных внешней области, сделанные внутри ветки, исполняются на копии и отбрасываются.
- Что сделать: исполнять ветки на исходном `env` (как это уже делает `Statement::IfElseBlock` в src/interpreter.rs:3393-3412 через `eval_block!(…, env)`). Если прямое использование `env` ломает borrow — извлечь исполнение ветки в ту же схему, что у `Statement::IfElseBlock`.
- Как проверить: e2, e3 проходят; `cargo build --release` без ошибок.

## Smoke-test после всего

```
cargo build --release
for c in e1 e2 e3 e4; do ./target/release/mlog run examples/contracts/else_branch/${c}_*.mlog; done
# ожидаемо: else / else / then / second
cargo test --release 2>&1 | tail -3
```

## Влияние на потребителей (для сведения владельца, не задача исполнителя)

FOSVED-office-v2 содержит 31 else-блок в .mlog (app.mlog — 25, dept/admin.mlog — 3, dept/chain.mlog — 2, dept/scheduler.mlog — 1). Все эти ветки сейчас мертвы и молча пропускаются. После принятия M2 и обновления бинарника FOSVED они оживут — поведение изменится. Прогон тотального теста FOSVED обязателен после апгрейда бинарника.

## Структура отчёта исполнителя (report.md)

```
# Отчёт: else-branch
Дата, ветка, коммиты.
## Контракты до правок: [вывод e1-e4]
## A.1: [строки правки]
## A.2: [подтверждён/отсутствует, строки]
## B.1: [строки правки, способ]
## Контракты после: [вывод e1-e4]
## Smoke: [дословно]
## Затруднения: [список или «нет»]
```

## Чек-лист сдачи

- [ ] Контракты e1-e4 созданы и до правок давали after / (пусто) / then / after
- [ ] A.1 выполнена, e1 → else, e4 → second
- [ ] A.2 проверена, результат зафиксирован
- [ ] B.1 выполнена, e2 → else, e3 → then
- [ ] cargo build и cargo test — успех, итог в отчёте
- [ ] report.md заполнен
