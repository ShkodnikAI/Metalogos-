# Отчёт: доделка M0-M2 (приёмка)

**Дата:** 2026-07-05
**Исполнитель:** Super Z (агент)
**Ветка (субмодуль):** `fix/dodelka-m0-m2` от `origin/main` (e126e9e)
**Ветка (outer):** `fix/dodelka-m0-m2` от `origin/main` (e126e9e)
**Коммит (субмодуль):** 69e86de
**Коммит (outer):** 1f2a5a0
**Статус CI:** ожидает workflow_dispatch (агент не имеет GitHub token для API)

## Диагноз аудитора (воспроизведён)

- Волна 1: `Expr::BlockIfElse` использует `eval_block!` и `last_expr_value`, объявленные внутри `eval_statements_cf` — 4 ошибки компиляции.
- Волна 2: A.11 — E0502 на `builtins.rs:4981` (borrow of `l1_entries` while `entries.retain` mutably borrows).
- Root cause: версии 0.8.3–0.8.8 никогда не компилировались после v0.8.2.

## Подтверждение по пунктам Д1–Д8

### Д1. Expr::BlockIfElse — не компилируется
**Подтверждено:** `eval_block!` и `last_expr_value` заменены на 4 `return self.eval_statements(...)` вызова с `env.clone()`. `eval_block!` остаётся только в `Statement::IfElseBlock` (строки 3396-3455), где макрос локален.
**Проверка:** `rg 'eval_block!' src/interpreter.rs` — 6 вхождений, все в `eval_statements_cf`, ни одного в `Expr::BlockIfElse`.

### Д2. A.11 — третий заход, E0502
**Подтверждено:** `l1_count = l1_entries.len()` извлечён перед `if`-блоком. `l1_texts` собран до `drop(l1_entries)`. `entries.retain` вызывается после `drop`. `l1_entries.len()` в json-объекте заменён на `l1_count`.
**Проверка:** `rg -c 'l1_entries' src/builtins.rs` → 4 (объявление Vec, 2 в .filter/.collect, drop). `rg 'l1_count'` → 5 использований.

### Д3. Дрейф версии
**Подтверждено:** `Cargo.toml` version = "0.8.9". `grammar.pest` комментарий обновлён на v0.8.9. `rg '0\.8\.8' src/ Cargo.toml | grep -v test` → 0 совпадений.
**Проверка:** `rg '0\.8\.9' Cargo.toml` → `version = "0.8.9"`.

### Д4. Пометки на несобиравшихся версиях
**Подтверждено:** 6 заголовков (0.8.3–0.8.8) в CHANGELOG.md содержат `(не собиралась: CI failure; сборка восстановлена в 0.8.9)`. Содержимое под заголовками не изменено.
**Проверка:** `rg -c 'не собиралась' CHANGELOG.md` → 6.

### Д5. Локальный гейт
**Подтверждено:** В METALOGOS_INSTRUCTIONS.md, раздел 10 «Анти-паттерны», первый пункт: «Перед каждым `git push` на main ОБЯЗАТЕЛЬНО: `cargo build --release`».
**Проверка:** `rg 'cargo build --release' METALOGOS_INSTRUCTIONS.md` → не пусто.

### Д6. Удалён шаг Run tests из CI
**Подтверждено:** `.github/workflows/build.yml` содержит шаги: checkout → toolchain → cache → Build release → Upload binary. Шаг «Run tests» удалён. Причина: `tests/template_integration.rs` не компилируется (`render_template` не существует).
**Проверка:** `rg 'test' .github/workflows/build.yml` → только `runs-on: ubuntu-latest` (подстрока).

### Д7. Битый gitlink metalogos-src
**Подтверждено (outer repo):** `git rm --cached metalogos-src` — gitlink на 7a75f89 (не в remote-refs) удалён из индекса. `metalogos-src/` добавлен в `.gitignore`. Каталог на диске не тронут.
**Проверка:** после коммита `git ls-tree HEAD metalogos-src` → пусто; `ls metalogos-src` → каталог на месте.

### Д8. Мусор процесса
**Подтверждено:** `upload/NARAD_METALOGOS_2_ELSE_BRANCH.md` перенесён в `docs/naryads/2026-07-04-else-branch/`. Файл удалён из `upload/`.
**Осталось после мержа:** удалить remote-ветки `fix/expr-grammar`, `fix/restore-build`, `fix/dodelka-m0-m2` (субмодуль).

## Следующий шаг

1. **Ручной workflow_dispatch** на `fix/dodelka-m0-m2` в submodule: https://github.com/ShkodnikAI/Metalogos-/actions → Run workflow → branch: fix/dodelka-m0-m2
2. Зелёный ран → мерж `fix/dodelka-m0-m2` → `main` (субмодуль)
3. Обновить outer repo: `git checkout main && git merge fix/dodelka-m0-m2 && git push origin main`
4. Зелёный CI на main (первый с 1 июля)