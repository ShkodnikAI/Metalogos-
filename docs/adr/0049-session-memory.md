# ADR-0049: Session Memory (временная память разговора)

**Статус**: Accepted
**Дата**: 2025-06-09
**Наряд**: №5

## Контекст

METALOGOS имеет глобальную KV-память (`mem_set`/`mem_get`/`mem_delete`), которая
персистентна через SQLite при `memory { persist: "..." }`. Это подходит для
глобальных конфигураций и долгосрочных данных.

Но при построении чат-ботов и веб-приложений нужна **временная** память,
привязанная к конкретной сессии (chat_id, user_id). Эта память должна:

- Быть изолированной между сессиями (chat_id A не видит данные chat_id B)
- Сбрасываться при рестарте сервера (by design — сессионные данные)
- Не персистироваться (в отличие от глобальной `mem_*`)

## Решение

Три новых builtin-функции с session-scoped хранилищем:

```
session_set(session_id: String, key: String, value: String) -> String
session_get(session_id: String, key: String) -> String
session_clear(session_id: String) -> Unit
```

### Хранилище

```rust
static SESSION_STORE: OnceLock<Mutex<HashMap<String, HashMap<String, String>>>>
```

- Внешний ключ = `session_id` (например `"chat-42"`, `"user-alice"`)
- Внутренний HashMap = ключ-значение внутри сессии
- **In-memory only** — нет SQLite, нет файла, нет персистенции
- Сброс при рестарте = просто исчезновение (HashMap очищается)

### Отличие от mem_set/mem_get

| Аспект | `mem_set`/`mem_get` | `session_set`/`session_get` |
|--------|----------------------|----------------------------|
| Scope | Глобальный | Per-session (session_id) |
| Персистенция | SQLite write-through | In-memory only |
| Рестарт | Данные сохраняются | Данные теряются |
| Изоляция | Нет — общая для всех | Да — разделена по session_id |

### Использование в .mlog

```
// Сохранить контекст разговора
session_set(chat_id, "last_topic", "billing")
session_set(chat_id, "message_count", "5")

// Прочитать позже в другом запросе
let topic = session_get(chat_id, "last_topic")

// Очистить при завершении сессии
session_clear(chat_id)
```

## Контракт-тесты

10 тестов в `tests/session_memory_contract.rs`:

1. **set→get roundtrip** — записал, прочитал, совпадает
2. **set returns value** — session_set возвращает сохранённое значение
3. **missing key** — session_get несуществующего ключа → пустая строка
4. **missing session** — session_get несуществующей сессии → пустая строка
5. **session isolation** — данные сессии A не видны из сессии B
6. **session_clear** — после clear все ключи сессии пустые
7. **restart empties** — reset_session_store() → все данные исчезли
8. **multiple keys** — несколько ключей в одной сессии сосуществуют
9. **overwrite** — перезапись ключа заменяет старое значение
10. **no persistence** — нет SQLite, чисто in-memory

## Файлы

| Файл | Изменение |
|------|-----------|
| `src/builtins.rs` | +3 builtin functions, SESSION_STORE static, helpers |
| `tests/session_memory_contract.rs` | NEW — 10 contract tests |
| `docs/adr/0049-session-memory.md` | NEW — этот документ |

## Последствия

- **Нет изменений в grammar/AST/parser** — это обычные function calls
- **Нет изменений в interpreter** — builtin dispatch уже обрабатывает все FnCall
- **Потокобезопасность**: `std::sync::Mutex` (same model как KV_STORE)
- **Обратная совместимость**: существующие `mem_set`/`mem_get` без изменений
- **Глобальное состояние**: `OnceLock<Mutex<...>>` — shared across all interpreters (by design для server mode)
