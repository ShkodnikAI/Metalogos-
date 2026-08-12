# Наряд MLG-5: Календарь (CalDAV + iCal) — полноценное расписание для Яны

Проект: Metalogos (ShkodnikAI/Metalogos-)
Слой: core (calendar-модуль builtin-функций)
Дата: 2026-08-13
Версия наряда: 1
База: HEAD aa06a3f (fix(CI): clippy clean — too_many_arguments allow)
Зависимость: наряды MLG-1..MLG-4 приняты; наряд MLG-5 расширяет их

## Цель прохода

Расширить Metalogos v0.14.0 календарным стеком для документооборота Яны:
1. **CalDAV-клиент** — подключение к Nextcloud, Google Calendar, Radicale и др.
2. **iCal генерация/парсинг** — создание приглашений, обработка входящих
3. **Управление событиями** — создание, чтение, обновление, удаление
4. **Free/Busy** — проверка занятости перед назначением встреч
5. **Покрыть тестами** — интеграционные + контрактные + arity

Грантовая стратегия: язык должен быть **самодостаточным** — без Python-прокси, чистый Rust.

## Текущее состояние (после MLG-4)

### Уже реализовано:
- PDF-стек (20 функций) — MLG-1..MLG-3
- Email-стек (7 функций) — MLG-4 (SMTP/IMAP)
- Время: `now()`, `format_date()`, `date_parts()`, `days_between()`, `add_days()`, `add_hours()`, `weekday_name()`, `is_leap_year()`, `days_in_month()`
- Cron: `cron_add()`, `cron_list()`, `cron_remove()`, `cron_run()`, `cron_mark_fired()`
- Reminders: `remind()`, `remind_recurring()`, `cancel_remind()`, `list_reminders()`, `check_reminders()`

### Что отсутствует:
| Что | Почему важно |
|-----|-------------|
| CalDAV-доступ | Яне нужно читать/писать в общий календарь (Nextcloud, Google) |
| iCal генерация | Создание .ics-файлов для приглашений на встречи |
| iCal парсинг | Обработка входящих приглашений из email-вложений |
| cal_create | Назначение встреч — ключевая функция для агента |
| cal_events | Просмотр расписания — проверка занятости |
| cal_freebusy | Автоматический подбор времени без конфликтов |
| cal_delete | Отмена встреч |
| cal_update | Перенос встреч, изменение описания |

## Спецификация функций

### CalDAV-сессии
| # | Функция | Arity | Описание | Крейт |
|---|---------|-------|----------|-------|
| 1 | `cal_connect(url, user, pass)` | 3 | Подключение к CalDAV-серверу, возвращает session_id | reqwest |
| 2 | `cal_list(session_id)` | 1 | Список доступных календарей | reqwest |

### Управление событиями
| # | Функция | Arity | Описание | Крейт |
|---|---------|-------|----------|-------|
| 3 | `cal_events(calendar_id, start, end)` | 3 | События в диапазоне дат (YYYY-MM-DD) | reqwest + ical |
| 4 | `cal_read(event_uid)` | 1 | Прочитать одно событие по UID | reqwest |
| 5 | `cal_create(cal_id, summary, start, end [,desc, location, attendees_json])` | 4..7 | Создать событие, возвращает UID | reqwest |
| 6 | `cal_update(event_uid, fields_json)` | 2 | Обновить поля события | reqwest |
| 7 | `cal_delete(event_uid)` | 1 | Удалить событие | reqwest |

### Free/Busy + iCal
| # | Функция | Arity | Описание | Крейт |
|---|---------|-------|----------|-------|
| 8 | `cal_freebusy(calendar_id, start, end)` | 3 | Запросить занятость в диапазоне | reqwest |
| 9 | `ical_parse(text)` | 1 | Разобрать iCal-текст → Struct | ical |
| 10 | `ical_generate(event_json)` | 1 | Сгенерировать iCal-текст из JSON | ручной |

**Итого: 10 функций**

## Зависимости (Cargo.toml)

```toml
# Наряд MLG-5: Calendar (CalDAV + iCal, pure Rust)
ical = "0.8"
chrono-tz = "0.10"
```

- `reqwest` — уже есть (HTTP для CalDAV/WebDAV запросов)
- `chrono` — уже есть (время/даты)
- `ical` — парсинг iCalendar (RFC 5545)
- `chrono-tz` — поддержка часовых поясов

## Архитектура

```
src/builtins/calendar.rs
├── CAL_SESSIONS: Lazy<Mutex<HashMap<String, CalSession>>>  — CalDAV-сессии
├── CalSession { url, user, pass, client: reqwest::blocking::Client }
├── cal_connect()   → создать сессию, проверить PROPFIND
├── cal_list()      → PROPFIND calendar-home-set
├── cal_events()    → CalDAV REPORT calendar-query
├── cal_read()      → GET по href события
├── cal_create()    → PUT нового .ics ресурса
├── cal_update()    → PUT с If-Match (ETag)
├── cal_delete()    → DELETE с If-Match
├── cal_freebusy()  → CalDAV free-busy-query REPORT
├── ical_parse()    → ical::IcalParser → Value::Struct
└── ical_generate() → ручная генерация VEVENT/VCALENDAR
```

CalDAV-протокол реализован поверх reqwest (блокирующий клиент):
- **PROPFIND** — обнаружение calendar-home-set (RFC 4791 §6.2)
- **REPORT calendar-query** — фильтрация событий по диапазону (RFC 4791 §7.8)
- **REPORT free-busy-query** — запрос занятости (RFC 4791 §7.10)
- **PUT** — создание/обновление событий
- **DELETE** — удаление

iCal-генерация — ручная (формат простой, RFC 5545):
```
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Metalogos//MLG-5//RU
BEGIN:VEVENT
UID:...
DTSTART:20260813T100000Z
DTEND:20260813T110000Z
SUMMARY:Встреча с инвестором
END:VEVENT
END:VCALENDAR
```

## Регистрация (3 шага)

### 1. registry.rs — добавить spec! в конец BUILTIN_REGISTRY
```rust
// Наряд MLG-5: Calendar (CalDAV + iCal)
spec!("cal_connect", 3, "calendar"),
spec!("cal_list", 1, "calendar"),
spec!("cal_events", 3, "calendar"),
spec!("cal_read", 1, "calendar"),
spec!("cal_create", 4, 7, "calendar"),
spec!("cal_update", 2, "calendar"),
spec!("cal_delete", 1, "calendar"),
spec!("cal_freebusy", 3, "calendar"),
spec!("ical_parse", 1, "calendar"),
spec!("ical_generate", 1, "calendar"),
```

### 2. mod.rs — добавить funcs.insert() в Builtins::new()
```rust
// Наряд MLG-5: Calendar (CalDAV + iCal)
funcs.insert("cal_connect".to_string(), builtin_cal_connect as BuiltinFn);
funcs.insert("cal_list".to_string(), builtin_cal_list as BuiltinFn);
funcs.insert("cal_events".to_string(), builtin_cal_events as BuiltinFn);
funcs.insert("cal_read".to_string(), builtin_cal_read as BuiltinFn);
funcs.insert("cal_create".to_string(), builtin_cal_create as BuiltinFn);
funcs.insert("cal_update".to_string(), builtin_cal_update as BuiltinFn);
funcs.insert("cal_delete".to_string(), builtin_cal_delete as BuiltinFn);
funcs.insert("cal_freebusy".to_string(), builtin_cal_freebusy as BuiltinFn);
funcs.insert("ical_parse".to_string(), builtin_ical_parse as BuiltinFn);
funcs.insert("ical_generate".to_string(), builtin_ical_generate as BuiltinFn);
```

### 3. mod.rs — добавить mod calendar; use calendar::*;
```rust
pub(crate) mod calendar;
use calendar::*;
```

## Тестирование

### Интеграционные тесты (`tests/phase_mlg5_calendar.rs`)

| Тест | Описание | Требует env |
|------|----------|-------------|
| test_registry_mlg5_entries_exist | Все 10 функций зарегистрированы с правильной arity | нет |
| test_mlg5_builtin_count | Общее количество builtin'ов выросло на 10 | нет |
| test_mlg5_category_calendar | Все 10 в категории "calendar" | нет |
| test_cal_connect_no_env | cal_connect без CALDAV_URL → graceful error | нет |
| test_cal_events_no_env | cal_events без сессии → graceful error | нет |
| test_cal_create_no_env | cal_create без сессии → graceful error | нет |
| test_ical_parse_basic | ical_parse простого VEVENT | нет |
| test_ical_generate_basic | ical_generate из JSON | нет |
| test_ical_roundtrip | generate → parse → совпадение полей | нет |
| test_ical_parse_multi_event | Парсинг VCALENDAR с 2+ VEVENT | нет |

**Live-тесты** (только при наличии CALDAV_URL, CALDAV_USER, CALDAV_PASS в env):
- test_live_cal_connect_list
- test_live_cal_create_read_delete
- test_live_cal_freebusy

## Версия

- Cargo.toml: `0.14.0` → `0.15.0`
- CHANGELOG.md: добавить секцию `## [0.15.0] — 2026-08-13`

## Git flow

1. Ветка: `feat/mlg5-calendar` (от `feat/mlg4-email`)
2. Commit: `Наряд MLG-5: Календарь (CalDAV+iCal) — cal_connect, cal_list, cal_events, cal_read, cal_create, cal_update, cal_delete, cal_freebusy, ical_parse, ical_generate`
3. Push: `git push origin feat/mlg5-calendar`
4. PR: в GitHub с описанием наряда

## Критерии приёмки

- [ ] 10 функций зарегистрированы в registry.rs с правильной arity
- [ ] 10 handlers в mod.rs (funcs.insert)
- [ ] calendar.rs компилируется без ошибок
- [ ] cargo fmt --check ✅
- [ ] cargo clippy --all-targets ✅ (0 ошибок, 0 warnings)
- [ ] cargo check ✅
- [ ] cargo test --lib ✅
- [ ] cargo test --test phase_mlg5_calendar ✅ (10/10)
- [ ] cargo test --test phase_mlg4_email ✅ (10/10)
- [ ] cargo test --test phase_mlg3_pdf_office ✅ (13/13)
- [ ] CHANGELOG.md обновлён
- [ ] Версия 0.15.0 в Cargo.toml
- [ ] Push в feat/mlg5-calendar
