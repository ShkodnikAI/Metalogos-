---
name: metalogos-language-completeness
description: >
  Add missing language constructs that block real-world programs. USE THIS SKILL whenever
  implementing let bindings, if/else expressions, loops (each/while), list/array types,
  string indexing/slicing, or module namespaces. Trigger it for Phase 5 work or whenever
  you realize a program can't be written without a builtin hack that should be a language
  construct. It defines the exact syntax, semantics, and contract for each missing feature.
---

# Metalogos — Phase 5: Language Completeness

**Цель:** после этой фазы на Metalogos можно написать нетривиальную программу без костылей
в виде Rust-builtins. Доказательство: self-hosted лексер переписан на чистом Metalogos.

## Что отсутствует и блокирует реальные программы

| Конструкция | Проблема без неё | Приоритет |
|---|---|---|
| `let` binding | Нельзя сохранить промежуточный результат внутри паттерна | P1 — блокирует всё |
| `if/else` выражение | Нельзя ветвиться по значению (только flow-branching по confidence) | P1 |
| Циклы (`each`, `while`) | Нельзя итерировать по данным | P2 |
| List/Array тип | Нет коллекций как значений первого класса | P2 (нужен для each) |
| Строковая индексация | Нельзя разобрать строку посимвольно | P3 |
| Модули с namespace | Import сливает всё в одну среду → конфликты имён | P4 |

---

## 5.1 — `let` bindings + `if/else`

### Синтаксис

```mlog
pattern Evaluate(score: Float) -> String {
  let grade = if score > 0.9 then "excellent"
              else if score > 0.7 then "good"
              else "needs work"
  let message = "Result: " + grade
  return message
}
```

### Семантика
- `let name = expr` — неизменяемая привязка в локальной области паттерна.
- Несколько `let` + финальный `return` — тело паттерна становится блоком.
- `if cond then expr else expr` — **выражение**, возвращает значение.
- Условие: любое сравнение (`>`, `<`, `>=`, `<=`, `==`, `!=`, `contains`).
- `else if` — цепочка, не специальный синтаксис.
- Без `else` — ошибка компиляции (выражение обязано иметь значение).

### Контракт
```
examples/p5_let_if.mlog → "Result: good"
```

### Реализация
- Grammar: `let_stmt`, `if_expr`, `comparison` правила в pest.
- AST: `Statement::Let { name, value: Expr }`, `Expr::If { cond, then, else_ }`.
- Все 3 бэкенда (TW, VM, JIT). JIT: if/else → Cranelift `brif`.

---

## 5.2 — Циклы (`each`, `while`) + List тип

### Синтаксис

```mlog
entity numbers: List = [1.0, 2.0, 3.0, 4.0, 5.0]

pattern Double(n: Float) -> Float { return n + n }

pattern SumList(items: List) -> Float {
  let total = 0.0
  each item in items {
    total = total + item     // мутация через each-блок
  }
  return total
}

// while для императивных случаев
pattern Countdown(n: Float) -> String {
  let result = ""
  let i = n
  while i > 0.0 {
    result = result + str(i) + " "
    i = i - 1.0
  }
  return result
}
```

### Семантика
- **`each item in collection { body }`** — итерация по List. `item` — локальная привязка.
  Внутри блока допускается присваивание (`x = expr`) к уже объявленным `let`-переменным.
- **`while condition { body }`** — цикл с условием. Тело — блок из `let`/присваиваний.
  Максимум итераций (safety limit) = 100_000. Превышение → soft-failure.
- **List литерал:** `[expr, expr, ...]`. Гомогенный (все элементы одного типа).
- **Встроенные:** `len(list)`, `get(list, index)`, `push(list, item)`, `concat(list1, list2)`,
  `map(list, PatternName)`, `filter(list, PatternName)`, `reduce(list, PatternName, init)`.

### Контракт
```
examples/p5_each.mlog → "2 4 6 8 10"    (each + Double)
examples/p5_while.mlog → "5 4 3 2 1"    (Countdown)
```

### Prior art
- `each` ≈ Rust `for x in iter`, Elixir `Enum.each`, Haskell `mapM_`.
- `while` — fallback для случаев, когда итерация не по коллекции.
- Safety limit — как в Lua (`debug.sethook` для loop detection).

---

## 5.3 — Строковые операции + индексация

### Синтаксис

```mlog
pattern ParsePair(s: String) -> String {
  let colon = index_of(s, ":")
  let key = s[0..colon]
  let value = s[colon + 1..len(s)]
  return key + " = " + value
}
```

### Семантика
- `s[i]` — символ (String длины 1) по индексу. Out of bounds → soft-failure (пустая строка + низкая confidence).
- `s[start..end]` — срез (полуоткрытый интервал). Отрицательные индексы: `s[-1]` = последний символ.
- Встроенные: `index_of(s, sub)`, `starts_with(s, prefix)`, `ends_with(s, suffix)`,
  `to_float(s)`, `to_string(x)`, `char_at(s, i)`, `substring(s, start, end)`.
- `s[i]` в грамматике — `expr "[" expr "]"` или `expr "[" expr ".." expr "]"`.
  Не конфликтует с Fluid Types (тот синтаксис: `TypeName[value][confidence]`).

### Контракт
```
examples/p5_strings.mlog → "name = Alice"  (парсинг "name:Alice")
```

---

## 5.4 — Модули с namespace

### Синтаксис

```mlog
import std/string as str
import std/math as m
import ./my_utils           // без alias — как раньше, сливает в глобальную среду

pattern Process(s: String) -> String {
  let trimmed = str.trim(s)
  let result = m.abs(-42.0)
  return trimmed + " " + to_string(result)
}
```

### Семантика
- `import path as alias` — создаёт namespace `alias`, все паттерны из модуля доступны
  как `alias.PatternName(...)`.
- `import path` (без as) — обратная совместимость, сливает в глобальную среду.
- `./path` — относительный импорт от текущего файла.
- `std/path` — стандартная библиотека.
- `pkg/path` — из зависимости в `mlog.toml`.
- Циклические импорты → ошибка semantic analysis.

### Контракт
```
examples/p5_modules.mlog → "hello 42"
```

---

## 5.5 — Валидация: self-hosted лексер без костылей

### Цель
Переписать `self-host/lexer.mlog` **без** Rust-builtins (`split_tokens`, `if_eq`,
`is_string_token`). Использовать `while` для итерации, `if/else` для классификации,
`s[i]` для индексации, `let` для промежуточных.

### Контракт
Тот же golden-вывод, что у текущего лексера. Если что-то невозможно выразить — это баг
в 5.1–5.4, исправь язык.

```
examples/p5_self_host_lexer.mlog + .expected → идентичен текущему self-host/lexer.mlog выводу
```
