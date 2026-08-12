# Наряд MLG-6: Контакты (CardDAV + vCard) — адресная книга для Яны

Проект: Metalogos (ShkodnikAI/Metalogos-)
Слой: core (contacts-модуль builtin-функций)
Дата: 2026-08-13
Версия наряда: 1
База: HEAD 756bf08 (fix(CI): registry_sync_check + registry_arity_check)
Зависимость: наряды MLG-1..MLG-5 приняты; наряд MLG-6 расширяет их

## Цель прохода

Расширить Metalogos v0.16.0 контактным стеком для документооборота Яны:
1. **CardDAV-клиент** — подключение к Nextcloud, Radicale, DAViCal, Apple Contacts и др.
2. **vCard генерация/парсинг** — создание контактов, обработка входящих
3. **Управление контактами** — создание, чтение, обновление, удаление
4. **Поиск** — поиск по всем адресным книгам (FN + EMAIL)
5. **Покрыть тестами** — интеграционные + контрактные + arity

Грантовая стратегия: язык должен быть **самодостаточным** — без Python-прокси, чистый Rust.

## Текущее состояние (после MLG-5)

### Уже реализовано:
- PDF-стек (20 функций) — MLG-1..MLG-3
- Email-стек (7 функций) — MLG-4 (SMTP/IMAP)
- Calendar-стек (10 функций) — MLG-5 (CalDAV/iCal)
- Время, Cron, Reminders — базовые временные операции

### Что отсутствует:
| Что | Почему важно |
|-----|-------------|
| CardDAV-доступ | Яне нужна адресная книга для email и календаря |
| vCard генерация | Создание .vcf-файлов для обмена контактами |
| vCard парсинг | Обработка входящих vCard из email-вложений |
| card_create | Добавление нового контакта — базовая операция |
| card_contacts | Просмотр адресной книги с фильтрацией |
| card_search | Поиск контакта по имени/email для отправки письма |
| card_delete | Удаление контакта |
| card_update | Обновление информации о контакте |

## Спецификация функций

### CardDAV-сессии
| # | Функция | Arity | Описание | Крейт |
|---|---------|-------|----------|-------|
| 1 | `card_connect(url, user, pass)` | 3 | Подключение к CardDAV-серверу, возвращает session_id | reqwest |
| 2 | `card_list(session_id)` | 1 | Список доступных адресных книг | reqwest |

### Управление контактами
| # | Функция | Arity | Описание | Крейт |
|---|---------|-------|----------|-------|
| 3 | `card_contacts(addressbook_id, query)` | 2 | Контакты из адресной книги с фильтрацией | reqwest + hand-rolled |
| 4 | `card_read(contact_uid)` | 1 | Прочитать один контакт по URL | reqwest |
| 5 | `card_create(addressbook_id, fn, email [,tel, org, title, note])` | 4..7 | Создать контакт, возвращает UID | reqwest |
| 6 | `card_update(contact_uid, fields_json)` | 2 | Обновить поля контакта | reqwest |
| 7 | `card_delete(contact_uid)` | 1 | Удалить контакт | reqwest |
| 8 | `card_search(session_id, query)` | 2 | Поиск по всем адресным книгам (FN + EMAIL) | reqwest |

### vCard
| # | Функция | Arity | Описание | Крейт |
|---|---------|-------|----------|-------|
| 9 | `vcard_parse(text)` | 1 | Разобрать vCard-текст → JSON | hand-rolled (RFC 6350) |
| 10 | `vcard_generate(contact_json)` | 1 | Сгенерировать vCard-текст из JSON | hand-rolled (v4.0) |

**Итого: 10 функций**

## Зависимости (Cargo.toml)

```toml
# Наряд MLG-6: Contacts (CardDAV + vCard, pure Rust)
# vcard parsing is hand-rolled (no external crate needed); CardDAV uses reqwest
```

- `reqwest` — уже есть (HTTP для CardDAV/WebDAV запросов)
- vCard парсинг — hand-rolled (RFC 6350 line unfolding, property extraction, multi-value arrays)
- vCard генерация — hand-rolled (v4.0, line folding max 75 octets)

## Протоколы

- **CardDAV**: RFC 6352 — addressbook-query REPORT, PROPFIND для discovery
- **WebDAV**: RFC 4918 — PROPFIND, REPORT, PUT, DELETE, ETag/If-Match
- **vCard**: RFC 6350 — VERSION 4.0, line folding (75 octets), parameter syntax

## Тесты

### Inline-тесты (contacts.rs): 14
- gen_uid: формат UUID, version nibble, variant nibble
- vcard_to_json: basic, multi-email, with params
- unfold_vcard_lines: RFC 6350 §3.2 line unfolding
- vcard_escape_text: запятые, точки с запятой, переводы строк, обратные слеши
- xml_escape: &, <, >, "
- fold_vcard_line: короткие строки, длинные строки (> 75 octets)
- apply_vcard_updates: замена существующих, добавление новых свойств
- vcard_generate: basic, multi-email
- vcard_parse: basic
- vcard_roundtrip: generate → parse

### Интеграционные тесты (phase_mlg6_contacts.rs): 10
- Registry entries exist with correct arity
- Builtin count
- Category = "contacts"
- card_connect no-env (graceful failure)
- card_contacts no-env (error)
- card_create no-env (error)
- vcard_parse basic
- vcard_generate basic
- vcard roundtrip
- vcard_parse multi-email

## Запреты прохода

- **НЕ** ломать существующие функции (pdf_*, smtp_*, imap_*, cal_*, ical_*)
- **НЕ** добавлять Python-зависимости
- **НЕ** менять arity существующих функций
- **НЕ** менять порядок в BUILTIN_REGISTRY (bytecode-индексы)
- **НЕ** трогать grammar.pest

## Перед началом

1. Ветка: `git checkout -b feat/mlg6-contacts` (от HEAD main)
2. Проверить: `cargo build` компилируется, существующие тесты проходят
