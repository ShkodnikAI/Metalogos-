<div align="center">

<img src="logo.jpg" alt="Metalogos" width="200"/>

# Metalogos

**The first programming language designed by AI, for AI. Security built into the language.**

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.8.3-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)

</div>

---

## What's New in v0.8.3

**Reverse-iteration primitives** — list aggregation, DB inserts, skill matching. Built by reverse-iterating from real Fosved V2 pain points.

```mlog
// map — apply pattern to every list element
entity Actor { name: String, A: Float, R: Float }
let actors = [{ name: "РФ", A: 0.6, R: 0.7 }, { name: "UA", A: 0.7, R: 0.4 }]

pattern Potential(a: Actor) -> Float {
  return a.A * a.R
}

let scores = map(actors, "Potential")          // [0.42, 0.28]
let ranked = sort_by(zip(actors, scores), "b", 1.0)  // descending by score
print(ranked[0].a.name)                        // "РФ"

// filter — find structs by field value
let big = filter(actors, "A", 0.65)

// db_insert — parameterized INSERT (safe by design)
db { url: "sqlite::memory:" }
db_insert("users", { name: "Alice", role: "admin" })
let row = query("SELECT * FROM users WHERE role = 'admin'")
print(row[0].name)  // "Alice"

// matches_any — case-insensitive substring matching
let hit = matches_any("проанализируй рынок нефти", ["рынок", "акци", "валют"])
// hit = 1.0

// estimate_tokens — rough heuristic
let t = estimate_tokens("Hello world")  // 3.0
```

---

## What's New in v0.8.2

**Telegram Bot Platform + Reminder Scheduler** — voice notes, inline keyboards, callback handling, background cron.

```mlog
// Voice notes (tts_send now sends as voice bubble, not audio file)
tts_send("Привет!", "alloy", bot_token, chat_id)
// Optional 5th arg "audio" for audio player mode instead

// Inline keyboards
let keyboard = {
  inline_keyboard: [
    [{ text: "Да", callback_data: "yes" }, { text: "Нет", callback_data: "no" }]
  ]
}
send_message(chat_id, "Выберите:", keyboard)

// Handle callbacks in webhook route
route "/webhook" method=POST {
  let data = json_body()
  let cb = data.callback_query
  if cb != "" {
    answer_callback_query(cb.id, "Выбрано: " + cb.data)
    if cb.data == "yes" {
      edit_message_text(cb.message.chat.id, cb.message.message_id, "Отлично!", keyboard)
    }
  }
  respond("ok")
}

// Background scheduler — reminders auto-fire every 5 seconds in server mode
remind_recurring("Ежедневный отчёт", 86400.0, "DailyReport")
// Server: mlog serve app.mlog → [scheduler] due recurring: [Ежедневный отчёт] DailyReport
```

---

## What's New in v0.8.1

**Human Intelligence Layer** — persona system, memory tree, mood tracking, human-like AI responses. Inspired by [OpenHuman](https://github.com/tinyhumansai/OpenHuman).

```mlog
// Create a persona with personality traits
human_create("Alice", "friendly, professional, curious, speaks Russian")

// Store memories with importance scoring
human_remember("Alice", "user_name", "Sergei from Minsk", 0.9)
human_remember("Alice", "preference", "prefers concise responses", 0.7)
human_remember("Alice", "project", "building AI assistant in Metalogos", 0.8)

// Set mood — influences LLM response tone
human_mood("Alice", "excited", 0.9)

// Recall relevant memories (keyword search with scoring)
let memories = human_recall("Alice", "project AI", 3)
each mem in memories {
  print(mem.key + ": " + mem.content + " (score: " + to_string(mem.score) + ")")
}

// Generate human-like response using persona + mood + memories + LLM
let reply = human_respond("Alice", "How is my project going?")
print(reply)

// Manage multiple personas
let all = human_personas()
human_delete("Alice")  // cleanup persona + all its memories
```

**All built on existing Metalogos primitives** — KV store for persistence, `call_llm` for generation. Zero new dependencies. Works with any LLM provider.

---

## What's New in v0.8.0

**Time, Calendar, Weather, Geolocation, Reminders** — all built-in, no API keys needed.

```mlog
// Free weather via Open-Meteo (no API key!)
let w = weather("Minsk")
print(format("{}°C, {}", w.temp, w.description))

// 7-day forecast
let forecast = weather_forecast("London", 7)
each day in forecast {
  print(day.date + ": " + to_string(day.temp_min) + ".." + to_string(day.temp_max) + "°C, " + day.description)
}

// Date formatting and calendar
print(format_date("%A, %d %B %Y"))  // "Tuesday, 01 July 2026"
let dp = date_parts(now())
print("Week " + to_string(dp.week_number) + " of " + to_string(dp.year))

// Geolocation by IP (free)
let loc = geo_ip()
print(loc.city + ", " + loc.country)  // "Minsk, Belarus"

// Reminders with recurrence
let id = remind("Call client", add_hours(now(), 2.0))
let daily = remind_recurring("Daily standup", 86400.0)
let due = check_reminders()
```

---

## 30-Second Example

An AI-powered message triage system with learnable patterns, confidence-based branching, semantic memory, and runtime self-improvement — in under 10 lines:

```mlog
// Learnable pattern: LLM call as a first-class language operation
learnable pattern Classify(msg: String) -> String {
  prompt: "Classify as: question | complaint | greeting | urgent"
}

// Semantic memory with priority-based recall
memorize "user prefers email over chat" with priority=0.9

// Runtime self-improvement: add examples to the model on the fly
adapt Classify add_example("where is my order?", "complaint")

// Confidence-based flow branching
flow Triage {
  input: String = "where is my order?"
  -> Classify -> output
}
```

No frameworks. No boilerplate. No `import ai_sdk`. The language *is* the AI infrastructure.

---

## Why Metalogos

### Seven Pillars Instead of Functions

Most languages give you functions, classes, and modules — tools for organizing computation. Metalogos gives you **seven semantic primitives** that map to how AI systems actually reason:

| Pillar | What It Does | Replaces |
|---|---|---|
| **Entity** | Typed data with identity, confidence, and relations | Variables, structs, objects |
| **Pattern** | Transformations — pure, learnable (LLM), or hybrid | Functions, API calls |
| **Flow** | Declarative pipelines with confidence-based branching | Control flow, orchestrators |
| **Memory** | Semantic store with priority-based recall and decay | Databases, caches, vector stores |
| **Rule** | Probabilistic rules with priority and conflict resolution | If/else chains, business logic |
| **Learn** | Training as a language operation | ML frameworks, training scripts |
| **Adapt** | Runtime self-modification with sandbox and rollback | — *(no analogue)* |

### Security by Design

In most languages, security is a library you choose to use. In Metalogos, unsafe operations **do not exist**. This is Rust's ownership model applied to web security:

```mlog
// XSS impossible — Html is opaque, built only via auto-escaping templates:
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>   // <script> becomes &lt;script&gt;
}

// SQL injection impossible — Query is opaque, parameters only:
let user = query("SELECT * FROM users WHERE id = $1", [id])

// Plaintext secrets impossible — Secret is opaque, cannot be printed:
entity token: Secret = env("API_KEY")
print(token)   // Compile error: Secret does not support print

// Broken access control impossible — routes require roles:
route "/admin" method=GET requires=[admin] { ... }
```

### AI-Native

Learnable patterns are not a library call — they are a **language construct**. An LLM invocation is as natural as calling a function, with automatic prompt engineering, few-shot caching, sandboxed execution, and runtime adaptation:

```mlog
learnable pattern Classify(text: String) -> Category {
  prompt: "Classify as: question | complaint | greeting | urgent"
}
// Call it like any other pattern:
flow Pipeline { input -> Classify -> Respond -> output }
```

### Human Intelligence Layer

Inspired by [OpenHuman](https://github.com/tinyhumansai/OpenHuman), Metalogos has a built-in **persona system** with memory trees, mood tracking, and human-like response generation. Create multiple AI personas, each with their own personality, emotional state, and persistent memory — all as simple function calls:

```mlog
human_create("Support Bot", "patient, helpful, technical")
human_remember("Support Bot", "faq_login", "User must reset password via email link", 0.9)
human_mood("Support Bot", "focused", 0.8)
let answer = human_respond("Support Bot", "I can't log in")
```

---

## Quick Start

```bash
# Download pre-built binary from GitHub Releases (Linux x86_64)
# Or build from source:

git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo install --path .

# Run a program
mlog run examples/m1_hello.mlog
# Output: HELLO, METALOGOS!!

# Compile to bytecode, then run
mlog compile examples/m1_hello.mlog
mlog run examples/m1_hello.mbc

# Interactive REPL
mlog repl

# Semantic check (no execution)
mlog check examples/p6_full_app.mlog

# Static security audit
mlog audit examples/p6_full_app.mlog

# Serve a web application
mlog serve examples/p6_full_app.mlog
# Listening on 0.0.0.0:8080

# Run eval harness (test learnable patterns)
mlog eval examples/m3_classify.mlog
```

---

## Security — OWASP Top 10

Metalogos addresses every item in the OWASP Top 10 (2021) at the **language level**, not through middleware or best practices:

| # | Threat | How Metalogos Prevents It |
|---|---|---|
| A01 | **Broken Access Control** | `requires=[role]` on routes, `require` assertions in patterns — enforced at runtime |
| A02 | **Cryptographic Failures** | `Secret` opaque type — no `print`, no `to_string`; `encrypt`/`decrypt` via AES-256-GCM |
| A03 | **Injection** | `Query` opaque type — parameterized only, SQL is a literal not a variable |
| A04 | **Insecure Design** | Security by design — unsafe operations are syntactically impossible, not just discouraged |
| A05 | **Security Misconfiguration** | Security headers (CSP, HSTS, X-Frame-Options) middleware enabled by default |
| A06 | **Vulnerable Components** | Minimal dependency tree; `mlogpkg` lockfile (planned) |
| A07 | **Authentication Failures** | `hash_password` / `verify_password` (Argon2id); HMAC-SHA256 signed sessions |
| A08 | **Data Integrity Failures** | CSRF double-submit pattern on all state-changing routes; signed session cookies |
| A09 | **Logging Failures** | Audit log for `require` failures, `adapt` mutations, and `unsafe_html` usage |
| A10 | **SSRF** | LLM calls execute in sandbox with `forbidden: [network]`; outbound HTTP restricted |

**Six security levels, each building on the last:**

1. **Type-safe HTML** — XSS impossible (`Html` opaque type, auto-escaping templates)
2. **Parameterized queries** — SQL injection impossible (`Query` opaque type)
3. **Encryption primitives** — plaintext secrets impossible (`Secret`, `Encrypted`, `Hash`)
4. **Authentication & authorization** — sessions, roles, `require` assertions
5. **CSRF & security headers** — HMAC-signed cookies, double-submit pattern, CSP/HSTS
6. **LLM sandbox** — no direct HTML injection from AI, rate limiting, network isolation

---

## Examples

| Example | What It Shows | Lines |
|---|---|---|
| [`m1_hello.mlog`](examples/m1_hello.mlog) | Entity + pattern + flow — "Hello, Metalogos!" | 4 |
| [`m2_triage.mlog`](examples/m2_triage.mlog) | Struct entities + rules + confidence branching | 17 |
| [`m3_classify.mlog`](examples/m3_classify.mlog) | Learnable pattern (LLM call) in a flow pipeline | 9 |
| [`m4_memory.mlog`](examples/m4_memory.mlog) | Semantic memory with `memorize` / `recall` | 8 |
| [`m5_adapt.mlog`](examples/m5_adapt.mlog) | Runtime self-improvement with `adapt` | 11 |
| [`p1_fluid_types.mlog`](examples/p1_fluid_types.mlog) | Fluid types: probabilistic superposition | 5 |
| [`p2_knowledge_graph.mlog`](examples/p2_knowledge_graph.mlog) | Knowledge graph with `relate` | 12 |
| [`p6_full_app.mlog`](examples/p6_full_app.mlog) | Full web app — auth, CRUD, AI classify, bot webhooks | 170 |
| [`v05_final_integration.mlog`](examples/v05_final_integration.mlog) | Full integration: strings, files, LLM, memory, time, weather | 30 |

All examples have golden-file tests (`.expected` / `.error` files). Run them with `cargo test`.

---

## Architecture

Metalogos has **three execution backends**: a tree-walking interpreter, a bytecode VM, and a JIT compiler (Cranelift).

```
 .mlog source        Pest PEG          AST              Semantic            Three Backends
─────────────  ──▶  ────────────  ──▶  ───────────  ──▶  ────────────  ──▶  ────────────
 entity             parse tokens      24 declaration    cross-reference     tree-walking
 pattern            syntax rules      14 Expr           validation          bytecode VM
 flow                                  12 Statement      opaque type       JIT (Cranelift)
 memory                                 4 MatchArm        enforcement
 rule
 learn
 adapt
```

**Implementation stack:**

| Component | Technology |
|---|---|
| Parser | [Pest 2.7](https://pest.rs/) — PEG grammar (335 rules) |
| AST / Interpreter | Hand-written Rust (~19 700 lines) |
| Bytecode compiler | 1 220 lines, 44 VM instructions, stack-based |
| JIT | [Cranelift](https://cranelift.dev/) — code generation |
| Web server | [Axum 0.8](https://github.com/tokio-rs/axum) + [Tokio](https://tokio.rs/) |
| Crypto | `hmac`, `sha2`, `aes-gcm` (AES-256-GCM) |
| CLI | [Clap 4.5](https://github.com/clap-rs/clap) |
| Tests | Golden-file (78 examples with `.expected`/`.error`) + 32 integration test files (7 000+ lines) |
| Builtins | `builtins.rs` — 141 built-in functions (4 700+ lines) |

```
Metalogos-/
├── Cargo.toml              # Single-crate project, version 0.8.3
├── src/
│   ├── grammar.pest         # PEG grammar (335 rules)
│   ├── ast.rs               # AST: 24 Declaration, 14 Expr, 12 Statement, 4 MatchArm variants
│   ├── parser.rs            # Pest tokens → AST (1 860 lines)
│   ├── semantic.rs          # Semantic analysis, opaque type enforcement, security audit (1 370 lines)
│   ├── interpreter.rs       # Tree-walking interpreter (3 810 lines)
│   ├── compiler.rs          # Bytecode compiler (1 220 lines)
│   ├── bytecode.rs          # 44 VM instructions
│   ├── vm.rs                # Bytecode VM executor (1 470 lines)
│   ├── jit.rs               # JIT compiler via Cranelift
│   ├── builtins.rs          # 118 built-in functions (3 500+ lines)
│   ├── server.rs            # Axum HTTP server + security middleware (1 280 lines)
│   ├── llm.rs               # LLM backend trait + mock + real providers (1 420 lines)
│   ├── memory_store.rs      # Semantic memory with decay + KV store (1 170 lines)
│   ├── audit.rs             # Static security audit (1 060 lines)
│   ├── embeddings.rs        # Embedding generation + cosine similarity (600 lines)
│   └── main.rs              # CLI: run / repl / check / serve / compile / eval / resume / audit
├── examples/                # 78 .mlog programs with golden tests
├── tests/                   # 32 integration test files (7 000+ lines)
├── stdlib/                  # Standard library modules (std/string, std/math, std/collections)
└── docs/adr/                # 63 Architecture Decision Records
```

---

## Roadmap

### Done

| Phase | Milestone | Status |
|---|---|---|
| M1 | Entity + Pattern + Flow | Done |
| M2 | Struct entities + Rules + Confidence branching | Done |
| M3 | Learnable patterns (LLM backend) | Done |
| M4 | Semantic memory + Knowledge graph | Done |
| M5 | Adapt + Sandbox + Mutate with rollback | Done |
| Phase 1 | Fluid types + Confidence propagation | Done |
| Phase 2 | Vector recall + Full adapt + ML learn | Done |
| Phase 3 | LSP, `mlogpkg` package manager, mdbook docs | Done |
| Phase 4 | Bytecode VM + JIT (Cranelift), self-hosted lexer | Done |
| Phase 5 | `let`/`if`/`each`/`while`, List type, Modules, Break/Continue, Match | Done |
| Phase 6.1–6.2 | HTTP server + Type-safe HTML templates | Done |
| Phase 6.3 | Parameterized database queries | Done |
| Phase 6.4 | Encryption: Secret / Encrypted / Hash | Done |
| Phase 6.5 | Authentication: sessions + CSRF + roles | Done |
| Phase 6.6 | Bot integration: Telegram + Discord webhooks | Done |
| Phase 6.7 | OWASP Top 10 validation + full web app | Done |
| Phase 7.1 | Inspect builtin, context loading, event streaming | Done |
| Phase 7.2 | Conversation state, LLM cache, model routing | Done |
| Phase 7.3 | Context compression, lifecycle control | Done |
| Phase 7.4 | Tool abstraction, hooks, definition of done | Done |
| Phase 7.5 | Memory persistence (e2e), JWT-style tokens, eval harness | Done |
| Phase 7.6 | Session memory, audit parse tests, server JSON body | Done |
| Phase 7.7 | Break/Continue, Match (StartsWith/Contains/Compare), compiler full-coverage, constraints | Done |
| Phase 7.8 | BlockIfElse expression bytecode compilation, format() arity fix | Done |
| Phase 8.0 | Time/Date/Calendar, Geolocation, Weather (Open-Meteo, free), Reminders with recurrence | Done |
| Phase 8.1 | Human Intelligence Layer: personas, memory tree, mood, human-like AI responses | Done |
| Phase 8.2 | Telegram Bot Platform: voice notes, inline keyboards, callbacks, reminder scheduler | Done |
| Phase 8.3 | Reverse-iteration primitives: map/zip/sort_by/filter/reduce, db_insert, matches_any, estimate_tokens | Done |

### Next

| Phase | Target |
|---|---|
| Phase 9 | Self-hosted compiler, mlogpkg ecosystem, production deployment |

---

## Prior Art

Metalogos stands on the shoulders of proven systems:

- **Rust** — ownership model, opaque types, zero-cost abstractions
- **Haskell** — type-safe HTML (Yesod/Blaze), `newtype` for secrets
- **Pest** — elegant PEG parser generator
- **Axum** — ergonomic async HTTP
- **Cranelift** — fast JIT code generation
- **Datalog / CLIPS** — declarative rule engines with priority
- **ACT-R** — memory activation and decay models
- **DSPy** — programmatic LLM orchestration
- **[OpenHuman](https://github.com/tinyhumansai/OpenHuman)** — persona system, memory tree, mood tracking (inspiration for Human Intelligence Layer)

---

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE-2.0) at your option.

---

*Built with Rust. Designed by AI. For AI.*