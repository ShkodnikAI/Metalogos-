---
name: metalogos-web-security
description: >
  Build the Metalogos web platform with security as a language-level property. USE THIS SKILL
  whenever implementing HTTP server, routing, templates, database access, authentication,
  session management, encryption, CSRF protection, or any web-facing feature. Trigger it for
  Phase 6 work, or whenever you're about to generate HTML, touch a database, handle user input,
  store secrets, or expose a network endpoint. It defines security invariants that CANNOT be
  violated — unsafe operations are syntactically impossible, not just discouraged.
---

# Metalogos — Phase 6: Web Platform + Security

**Принцип:** безопасность — не библиотека, а **свойство рантайма**. Небезопасные операции
(raw SQL, unescaped HTML, plaintext secrets) **не существуют в языке**. Доступны только
безопасные аналоги. Это как Rust с памятью — не «будь осторожен», а «невозможно ошибиться».

---

## Архитектура безопасности — 6 уровней

### Уровень 1: Типобезопасный HTML (XSS невозможен)

**Проблема:** конкатенация строк в HTML → XSS.
**Решение:** тип `Html` — непрозрачный, строится только через шаблонизатор с автоэкранированием.

```mlog
// ЭТО НЕВОЗМОЖНО — нет оператора String → Html:
entity page: Html = "<div>" + user_input + "</div>"   // ОШИБКА КОМПИЛЯЦИИ

// ЭТО ЕДИНСТВЕННЫЙ ПУТЬ:
template Page(title: String, body: String) -> Html {
  <html>
    <head><title>{{ title }}</title></head>
    <body>{{ body }}</body>        // автоэкранирование: < → &lt;
  </html>
}
```

**Prior art:** Askama (Rust, compile-time templates), Yesod (Haskell, type-safe HTML),
Elm (virtual DOM, no raw HTML injection).

**Реализация:** `template` — новая конструкция, компилируется через Askama в Rust.
Тип `Html` — opaque, к нему нет `+`, нет `to_string`, нет прямого создания из String.
Единственная операция: `render(template, args)` → Http response body.
Для случаев, когда raw HTML нужен (виджеты): `unsafe_html(s)` — требует `sandbox` блок
и записывается в audit log.

### Уровень 2: Параметризованные запросы (SQL injection невозможен)

**Проблема:** `"SELECT * FROM users WHERE id = " + id` → injection.
**Решение:** тип `Query` — строится только через `query` конструкцию с параметрами.

```mlog
// ЭТО НЕВОЗМОЖНО — нет оператора String → Query:
entity q: Query = "SELECT * FROM users WHERE id = " + id   // ОШИБКА КОМПИЛЯЦИИ

// ЭТО ЕДИНСТВЕННЫЙ ПУТЬ:
let user = query("SELECT * FROM users WHERE id = $1", [user_id])
let users = query("SELECT * FROM users WHERE age > $1 AND city = $2", [min_age, city])
```

**Prior art:** sqlx (Rust, compile-time query validation), Yesod Persistent,
parameterized queries в каждой серьёзной ORM.

**Реализация:** `query(sql_literal, params)` — встроенная конструкция, не паттерн.
`sql_literal` — только строковый литерал (не переменная!), проверяется в semantic analysis.
Параметры подставляются через prepared statements. Тип `Query` — opaque.

### Уровень 3: Шифрование данных (plaintext secrets невозможны)

**Проблема:** пароли и токены хранятся в открытом виде.
**Решение:** типы `Secret` и `Encrypted` — непрозрачные, к ним нет `print` и `to_string`.

```mlog
entity api_key: Secret = env("API_KEY")              // из переменной окружения
entity password: Secret = input_secret("Password: ")  // из stdin без эхо

// ЭТО НЕВОЗМОЖНО:
print(api_key)           // ОШИБКА: Secret не поддерживает print/to_string/output
let s = to_string(password)  // ОШИБКА: Secret → String запрещён

// Для хранения:
let hash = hash_password(password)            // argon2, возвращает Hash
let ok = verify_password(password, hash)      // сравнение без раскрытия

// Для шифрования данных:
let key: Secret = generate_key()
let encrypted: Encrypted = encrypt(data, key)       // aes-256-gcm
let decrypted: String = decrypt(encrypted, key)     // обратно
```

**Prior art:** Haskell `newtype` для секретов (не показывает через Show), Vault (HashiCorp),
AWS Secrets Manager, `secrecy` crate в Rust (zeroize on drop).

**Реализация:** `Secret` и `Encrypted` — opaque типы. `Secret` реализует `Zeroize` (обнуление
при drop). `print`, `to_string`, `output`, конкатенация с String — ошибка semantic analysis.
`env("KEY")` → Secret. Шифрование: `ring` (aes-256-gcm) или `aes-gcm` crate.

### Уровень 4: Аутентификация и авторизация

```mlog
// Определение ролей
entity Role { name: String, permissions: List }
entity admin: Role = { name: "admin", permissions: ["read", "write", "delete"] }
entity viewer: Role = { name: "viewer", permissions: ["read"] }

// Защита маршрута
route "/admin/users" method=GET requires=[admin] {
  let users = query("SELECT * FROM users", [])
  render(AdminPanel, users)
}

// Проверка прав в паттерне
pattern DeleteUser(user_id: String, actor: Session) -> Html {
  require actor.role has "delete"    // иначе → 403 Forbidden
  query("DELETE FROM users WHERE id = $1", [user_id])
  render(Success, "User deleted")
}
```

**Prior art:** RBAC (role-based access control), ABAC, capability-based security,
Phoenix plugs (Elixir), Rails before_action.

**Реализация:** `requires=[roles]` на маршруте — проверка до вызова обработчика.
`require expr` — assertion, при провале → 403 + audit log. `Session` — opaque тип
из middleware (см. уровень 5).

### Уровень 5: Сессии, CSRF, куки

```mlog
// Middleware автоматически:
// 1. Проверяет/создаёт сессию (HttpOnly, Secure, SameSite=Strict)
// 2. Проверяет CSRF-токен для POST/PUT/DELETE
// 3. Устанавливает Security Headers (CSP, X-Frame-Options, etc.)

server {
  middleware: [session, csrf, security_headers]
  
  route "/" method=GET {
    render(Home)
  }
  
  route "/login" method=POST {
    let credentials = form_data()    // автоматический CSRF-check
    let user = authenticate(credentials.email, credentials.password)
    if user then session_login(user) else render(LoginFailed)
  }
}
```

**Prior art:** Express.js middleware, Axum extractors/layers, Phoenix plugs,
Django middleware, Rails rack middleware.

**Реализация:** через Axum tower layers. Session: signed cookie (HMAC-SHA256) или
server-side store (SQLite). CSRF: double-submit cookie pattern. Security headers:
`Content-Security-Policy`, `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`,
`Strict-Transport-Security`.

### Уровень 6: Sandbox для LLM и Adapt

**Уже существует** (Phase 2/M5), но усиливаем для веб-контекста:
- LLM-ответы **никогда** не вставляются в HTML напрямую → проходят через `template`
  (автоэкранирование).
- `adapt` в продакшн-режиме требует `sandbox` с `forbidden: [network, write_permanent]`.
- Rate limiting на LLM-вызовы: `rate_limit: 10/minute` в конфигурации.

---

## Веб-конструкции языка

### `server` — HTTP-сервер

```mlog
server {
  port: 8080
  host: "0.0.0.0"
  tls: { cert: "cert.pem", key: env("TLS_KEY") }   // rustls, не OpenSSL
  middleware: [session, csrf, security_headers, rate_limit(100)]
  
  static: "./public"     // статические файлы
  
  route "/" method=GET { render(Home) }
  route "/api/users" method=GET requires=[admin] { ... }
  route "/webhook/telegram" method=POST { ... }
}
```

### `template` — типобезопасный HTML

```mlog
template Layout(title: String, content: Html) -> Html {
  <!DOCTYPE html>
  <html>
    <head>
      <title>{{ title }}</title>
      <meta charset="utf-8">
    </head>
    <body>{{ content | safe }}</body>   // content уже Html → пропустить экранирование
  </html>
}

template UserCard(user: User) -> Html {
  <div class="card">
    <h2>{{ user.name }}</h2>           // автоэкранирование
    <p>Confidence: {{ user.confidence }}</p>
  </div>
}
```

### `route` — маршрутизация

```mlog
route "/users/:id" method=GET {
  let user = query("SELECT * FROM users WHERE id = $1", [params.id])
  if user then render(UserCard, user) else respond(404, "Not found")
}

route "/api/classify" method=POST {
  let text = json_body().text
  let result = Classify(text)        // learnable pattern!
  respond_json({ category: result, confidence: result.confidence })
}
```

### `db` — соединение с БД

```mlog
db {
  url: env("DATABASE_URL")           // Secret — не строковый литерал
  pool_size: 10
  migrate: "./migrations"            // автоматические миграции
}
```

---

## Лестница постройки Phase 6

### 6.1 — HTTP-сервер (минимальный)
`server` + `route` + `respond` (plain text). Контракт: `mlog serve app.mlog` слушает порт,
GET "/" → "Hello from Metalogos". Используй Axum.

### 6.2 — Шаблоны (типобезопасный HTML)
`template` конструкция. Тип `Html` — opaque. Автоэкранирование. Контракт: template с
пользовательским вводом `<script>alert(1)</script>` → экранированный вывод.

### 6.3 — База данных (parameterized queries)
`db` + `query()`. Тип `Query` — opaque. Контракт: попытка конкатенации String в Query →
ошибка компиляции. SQLite через sqlx.

### 6.4 — Шифрование (Secret + Encrypted)
Типы `Secret`, `Encrypted`, `Hash`. `env()` → Secret. `hash_password` / `verify_password`.
`encrypt` / `decrypt`. Контракт: `print(secret)` → ошибка компиляции.

### 6.5 — Аутентификация (session + CSRF + roles)
Middleware, `session_login`/`session_logout`, CSRF-проверка, `requires=[role]`.
Контракт: POST без CSRF-токена → 403.

### 6.6 — Бот-интеграция (Telegram + Discord)
`webhook` конструкция для приёма сообщений. `send_message(chat_id, text)`.
Learnable-паттерн как обработчик → AI-бот на 20 строках .mlog.

### 6.7 — Валидация: полноценное веб-приложение
Приложение с авторизацией, CRUD, AI-классификацией, ботом — всё на .mlog.
Прогон OWASP Top 10 чек-листа.

---

## OWASP Top 10 — как Metalogos закрывает каждый пункт

| # | Угроза | Защита в Metalogos |
|---|---|---|
| A01 | Broken Access Control | `requires=[role]` на маршрутах, `require` в паттернах |
| A02 | Cryptographic Failures | `Secret` opaque тип, шифрование через `ring`, без plaintext |
| A03 | Injection | `Query` opaque тип, только parameterized, SQL литерал — не переменная |
| A04 | Insecure Design | Безопасность by design — unsafe операции не существуют синтаксически |
| A05 | Security Misconfiguration | Security headers middleware включен по умолчанию |
| A06 | Vulnerable Components | `mlogpkg` lock-файл, проверка зависимостей |
| A07 | Auth Failures | `hash_password`/`verify_password` (argon2), rate limiting |
| A08 | Data Integrity Failures | CSRF middleware по умолчанию, signed sessions |
| A09 | Logging Failures | Audit log для `unsafe_html`, `adapt`, `require` failures |
| A10 | SSRF | LLM-вызовы только в sandbox с `forbidden: [network]` |
