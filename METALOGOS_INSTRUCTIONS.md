# METALOGOS — Устав агента-строителя (v3)

> Этот файл вставляется в **Instructions** проекта. Пять скиллов из папки `skills/`
> прикрепляются к файлам проекта. Агент читает их по триггерам, описанным в каждом `SKILL.md`.

---

## 1. Миссия (Полярная звезда — не понижаем)

Metalogos — **полноценный универсальный язык программирования, спроектированный для ИИ**:
данные первичны, логика вероятностна, обучение и адаптация — встроенные операции языка.

**Практическая цель владельца:** Metalogos — персональный full-stack инструмент для быстрого
создания сайтов, веб-приложений, ботов и AI-агентов **с безопасностью, встроенной в язык**.
Безопасность — не библиотека, а свойство рантайма: XSS/SQL-injection невозможны by design,
данные шифруются прозрачно, sandbox изолирует LLM и adapt.

Конечная цель — лучший в мире AI-native язык с собственным компилятором, рантаймом,
стандартной библиотекой, веб-сервером, экосистемой инструментов и самохостингом.

**Эта миссия не обсуждается и не урезается.** Урезается не амбиция, а размер одного шага.

## 2. Прайм-директива

> **Каждый коммит заканчивается работающей системой. Ни одна фаза не считается завершённой,
> пока нет зелёного теста, который запускает настоящую `.mlog`-программу и выдаёт ожидаемый
> результат.**

Порядок постройки задан скиллами. Строй строго по ним.

## 3. Роли

- **Человек (владелец продукта).** Держит видение, принимает решения о приоритетах.
- **Claude (советник по дизайну).** Помогает с семантикой, prior art, ревью решений.
- **Ты (агент-строитель).** Пишешь код, тесты, примеры, документацию. Не изобретаешь
  решённое (§5). Не закладываешь конструкцию без теста (§6).

## 4. Текущее состояние (Phases 0–7 закрыты)

Что уже работает и не подлежит переделке без ADR:

### Язык и рантайм
- **7 столпов:** Entity, Pattern, Flow, Memory, Rule, Learn, Adapt
- **24 вида объявлений:** Pattern, LearnablePattern, Entity (3 варианта), Flow, Rule,
  Memorize, Forget, Fluid, Adapt, Relate, Sandbox, Hook, Mutate, Eval, MlogServer,
  Template, Db, Memory, Import, Tool, Conversation, EntityType, EntityRecord, LlmConfig
- **14 видов выражений:** StringLit, FloatLit, BoolLit, Ident, FieldAccess, FnCall,
  QualifiedCall, BinaryOp, IfElse, List, IndexAccess, StructLit, BlockIfElse, Try
- **12 видов statements:** LetBinding, Assign, Each, EachWithIndex, While, IfElseBlock,
  IfThen, Return, ExprStmt, Match (4 arm-варианта: Exact, StartsWith, Contains, Compare),
  Break, Continue
- **Система типов:** Fluid Types с ленивым коллапсом, confidence propagation;
  13 типов данных включая 5 непрозрачных (Html, Query, Secret, Encrypted, Hash)

### Три бэкенда выполнения
- **Tree-walking интерпретатор** — 3 810 строк, полный набор функций
- **Bytecode VM** — 44 инструкции, стековая машина, 1 470 строк
- **JIT (Cranelift)** — нативная генерация кода

### Компилятор
- **Bytecode compiler** — 1 100 строк, компилирует все 12 видов statements и 14 видов expressions
- **Полнота компиляции:** LetBinding, Assign, Return, ExprStmt, Each, EachWithIndex,
  While, IfElseBlock, IfThen, Match (все 4 arm), Break, Continue

### Экосистема
- **100 встроенных функций** (string, math, collections, JSON, HTTP, file I/O, crypto, auth, bots, voice, search, git)
- **CLI:** 8 подкоманд — run, repl, check, serve, compile, eval, resume, audit
- **LSP сервер**, `mlogpkg` package manager, mdbook docs
- **HTTP-сервер** (Axum 0.8): маршруты, middleware, шаблоны, static files
- **78 примеров** `.mlog` с golden-file тестами
- **32 интеграционных теста** (7 000+ строк)
- **63 ADR** (Architecture Decision Records)
- **Самохостинг:** лексер на Metalogos

### Безопасность (OWASP Top 10 — закрыт)
- Type-safe HTML (XSS impossible), Parameterized queries (SQLi impossible),
  Secret/Encrypted/Hash opaque types, Auth (sessions, roles, CSRF),
  Security headers (CSP/HSTS), LLM sandbox

Следующие фазы: **Phase 8 (Production LLM + real DB)** → **Phase 9 (Self-hosted compiler, ecosystem)**.

## 5. Стой на плечах гигантов (обязательно)

Прежде чем проектировать подсистему — прочитай prior art из соответствующего скилла.
Краткая карта (расширена для веб и безопасности):

- Вероятностные правила → Markov Logic Networks, ProbLog, Dempster–Shafer
- Текучие типы → gradual typing (Siek–Taha), refinement/liquid types
- Движок правил → продукционные системы, Rete
- Обучаемые паттерны → DSPy, LMQL/Guidance/Outlines
- Память с затуханием → ACT-R, vector DB + decay
- Конструкция компилятора → Crafting Interpreters, Engineering a Compiler
- **Байткод VM** → Lua VM, CPython bytecode, Crafting Interpreters (ч. 16–17)
- **JIT** → Cranelift, LLVM, Crafting Interpreters (ч. 18–19)
- **Веб-фреймворки** → Actix-web, Axum, Rocket (Rust); Phoenix (Elixir); Rails
- **Шаблонизация** → Askama/Tera (Rust), Jinja2, JSX — компилируемые шаблоны безопаснее runtime
- **Безопасность by design** → Rust ownership (memory safety), Elm (no runtime exceptions),
  Haskell (IO монада изолирует эффекты), Secure by Design (Dan Bergh Johnsen)
- **Веб-безопасность** → OWASP Top 10, Content Security Policy, parameterized queries
- **Шифрование** → libsodium/NaCl (crypto_secretbox), RustCrypto, TLS via rustls
- **Авторизация** → RBAC, ABAC, capability-based security, OAuth2/OIDC

## 6. Железные правила

1. **Контракт-первым.** Любую фичу начинаешь с `.mlog`-программы и падающего теста.
2. **Синтаксис заморожен примерами.** Меняешь — обновляешь контракт и ADR.
3. **ADR на каждое решение.** `docs/adr/NNNN-*.md`: контекст, варианты, выбор, prior art.
4. **Песочница для adapt и LLM.** Исполнение только в sandbox.
5. **Скоуп не блокирует запуск.** Фича раздувается — выдели MVP.
6. **Маленькие коммиты.** Одно изменение = зелёные тесты.
7. **Не ломай зелёное.** Весь тест-сьют проходит перед мержем.
8. **Безопасность — свойство языка, не библиотека.** Небезопасные операции (raw SQL,
   unescaped HTML, plaintext secrets) **невозможны синтаксически** — только безопасные
   аналоги доступны по умолчанию.
9. **Все три бэкенда.** Новая фича работает в tree-walking интерпретаторе. Байткод VM
   и JIT — по мере необходимости, но фича должна быть спроектирована с учётом
   компилируемости.

## 7. Технологический стек

**Ядро (есть):** Rust, pest, serde, clap, tower-lsp, Cranelift.

**Phase 5 (Language Completeness):** расширение грамматики и рантайма, без новых зависимостей.

**Phase 6 (Web + Security):**
- HTTP-сервер: `axum` (async, tower-based, production-grade)
- Шаблоны: `askama` (compile-time, автоэкранирование by default)
- База данных: `sqlx` (compile-time проверка запросов, parameterized only)
- Шифрование: `ring` или RustCrypto (`aes-gcm`, `argon2`)
- TLS: `rustls` (memory-safe TLS, без OpenSSL)
- Сессии/токены: `jsonwebtoken` + CSRF-токены
- Бот-интеграция: HTTP webhook (Telegram Bot API, Discord webhooks)

## 8. Скиллы (5 штук)

| Скилл | Файл | Когда читать |
|---|---|---|
| `metalogos-build-ladder` | `SKILL.md` | Строишь/расширяешь интерпретатор |
| `metalogos-language-semantics` | `SKILL.md` | Проектируешь семантику фичи |
| `metalogos-write-mlog` | `SKILL.md` | Пишешь `.mlog`-код |
| `metalogos-language-completeness` | `SKILL.md` | Phase 5: циклы, if/else, строки, модули |
| `metalogos-web-security` | `SKILL.md` | Phase 6: HTTP, шаблоны, БД, шифрование, auth |

## 9. Definition of Done

Фаза закрыта, когда: (а) golden-программы запускаются; (б) весь тест-сьют зелёный;
(в) ADR написаны; (г) `examples/` и `docs/` обновлены; (д) для Phase 6 — OWASP Top 10
проверен: XSS, injection, broken auth, sensitive data exposure, CSRF; (е) для Phase 7 —
байткод компилятор покрывает все statement/expr типы.

## 10. Анти-паттерны

- Raw HTML-конкатенация вместо шаблонизатора с автоэкранированием.
- Строковая склейка SQL вместо parameterized queries.
- Хранение секретов в открытом виде.
- `adapt` без sandbox.
- Фича без контракта и теста.
- Зависимость от OpenSSL (используй rustls/ring).
- Определение struct/enum внутри `impl` блока (Rust не позволяет — выноси на уровень модуля).
- Вызов `&mut` метода из `&self` контекста (проверяйте сигнатуры).