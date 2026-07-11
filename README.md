<div align="center">

<img src="logo.jpg" alt="Metalogos" width="200"/>

# Metalogos

**AI-native programming language with security by design. Written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.9.1-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)

</div>

---

## What is Metalogos

Metalogos (mlog) is a programming language where AI operations — LLM calls, memory, learning, adaptation — are first-class language constructs, not library integrations. An LLM invocation is as natural as calling a function. Security constraints (XSS prevention, SQL injection prevention, secret opacity) are enforced at the language level, not through middleware.

The runtime is used as the execution engine for [FOSVED Office v2](FOSVED-office-v2/) — an AI office with 14 departments, each backed by mlog skills and patterns.

### Quick example

```mlog
// Learnable pattern: LLM call as a language construct
learnable pattern Classify(msg: String) -> String {
  prompt: "Classify as: question | complaint | greeting | urgent"
}

// Memory Tree: hierarchical knowledge with L0 → L1 → L2 compression
mtree_store("User prefers email over chat")
mtree_store("Project deadline is July 15")
mtree_summarize()  // L0 → L1 chunking

// Cron scheduler: run patterns on schedule
cron_run("0 9 * * 1-5", "MorningReport")  // weekdays at 09:00

// Goals and Todos
goal_set("launch v1.0", "2026-08-01")
todo_add("Fix cron edge case", "high")

// Call it like any other pattern
let result = Classify("where is my order?")
```

No frameworks. No boilerplate. No `import ai_sdk`. The language *is* the AI infrastructure.

---

## Seven Semantic Primitives

| Primitive | Purpose | Analogue in other languages |
|---|---|---|
| **Entity** | Typed data with identity, confidence, relations | Structs, objects, variables |
| **Pattern** | Transformations — pure, learnable (LLM), or hybrid | Functions, API calls |
| **Flow** | Declarative pipelines with confidence-based branching | Control flow, orchestrators |
| **Memory** | Semantic store with priority-based recall and decay | Databases, caches, vector stores |
| **Rule** | Probabilistic rules with priority and conflict resolution | If/else chains, business logic |
| **Learn** | Training as a language operation | ML frameworks, training scripts |
| **Adapt** | Runtime self-modification with sandbox and rollback | No direct analogue |

---

## Key Features

### Security by Design

Unsafe operations are syntactically impossible, not just discouraged:

```mlog
// XSS impossible — Html is opaque, auto-escaped in templates
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>   // <script> becomes &lt;script&gt;
}

// SQL injection impossible — parameterized only
let user = query("SELECT * FROM users WHERE id = $1", [id])

// Plaintext secrets impossible — Secret type blocks print/to_string
entity token: Secret = env("API_KEY")
print(token)   // Compile error

// Broken access control — routes require roles
route "/admin" method=GET requires=[admin] { ... }
```

### OWASP Top 10 Coverage

Every item in the OWASP Top 10 (2021) is addressed at the language level: type-safe HTML (A03), parameterized queries (A03), Secret/Encrypted/Hash opaque types (A02), role-based routes + `require` assertions (A01), HMAC-SHA256 sessions (A07), CSRF double-submit (A08), CSP/HSTS/X-Frame-Options headers (A05), LLM sandbox (A10), audit logging (A09).

### Three Execution Backends

- **Tree-walking interpreter** — full feature support, used for `mlog run` and `mlog serve`
- **Bytecode VM** — 44 instructions, stack-based, used for `mlog compile` + `mlog run file.mbc`
- **JIT** — Cranelift-based code generation (experimental)

### 142 Built-in Functions

String ops, math, collections, type conversion, LLM/AI, HTTP, JSON, file I/O, KV store, session memory, encryption, authentication, HTTP server, templates, databases, Telegram/Discord bots, time/date/calendar, geolocation, weather, reminders, cron, goals, todos, memory tree, preferences, approval workflows, and more. See [REFERENCE.md](REFERENCE.md) for the full list.

### Human Intelligence Layer

Persona system with memory trees, mood tracking, and human-like response generation — inspired by [OpenHuman](https://github.com/tinyhumansai/OpenHuman):

```mlog
human_create("Alice", "friendly, professional, curious")
human_remember("Alice", "project", "building AI assistant in Metalogos", 0.8)
human_mood("Alice", "excited", 0.9)
let reply = human_respond("Alice", "How is my project going?")
```

### Cron Scheduler

Real 5-field cron expressions, recurring and one-shot jobs, dispatches both builtins and user patterns:

```mlog
cron_run("*/30 * * * *", "HealthCheck")     // every 30 min
cron_run("0 9 * * 1-5", "MorningReport")     // weekdays 09:00
cron_list()   // list all jobs
```

### Memory Tree

Three-level hierarchical memory: L0 (raw entries) → L1 (chunk summaries) → L2 (global summary). Admission-gated storage, keyword-relevance retrieval with scoring, and automatic compression.

### Goals & Todos

Built-in goal tracking with deadlines and todo management with priorities — all persisted in KV store.

---

## Quick Start

```bash
# Build from source
git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo install --path .

# Run a program
mlog run examples/m1_hello.mlog

# Compile to bytecode, then run via VM
mlog compile examples/m1_hello.mlog
mlog run examples/m1_hello.mbc

# Interactive REPL
mlog repl

# Semantic check (no execution)
mlog check examples/p6_full_app.mlog

# Static security audit
mlog audit examples/p6_full_app.mlog

# Serve as web application
mlog serve app.mlog

# Run eval harness (test learnable patterns)
mlog eval examples/m3_classify.mlog
```

Pre-built Linux x86_64 binaries are available from [GitHub Actions](https://github.com/ShkodnikAI/Metalogos-/actions) artifacts.

---

## Architecture

```
 .mlog source        Pest PEG          AST              Semantic            Three Backends
─────────────  ──>  ────────────  ──>  ───────────  ──>  ────────────  ──>  ────────────
 entity             parse tokens      24 Declaration    cross-reference     tree-walking
 pattern            syntax rules      14 Expr           validation          bytecode VM
 flow                                  12 Statement      opaque type       JIT (Cranelift)
 memory                                 4 MatchArm        enforcement
 rule
 learn
 adapt
```

### Implementation Stack

| Component | Technology | Lines |
|---|---|---|
| Parser | Pest 2.7 PEG grammar (346 rules) | 2 176 |
| AST | 24 Declaration, 14 Expr, 12 Statement, 4 MatchArm | 731 |
| Compiler | Bytecode, 142 builtins indexed | 659 |
| Semantic analysis | Opaque types, arity checking, security audit | 446 |
| Interpreter | Tree-walking, full feature support | 4 281 |
| VM | 44 instructions, stack-based | 1 268 |
| Built-in functions | 142 function definitions | 5 655 |
| HTTP server | Axum 0.8 + Tokio, security middleware | 1 446 |
| LLM backend | Trait + mock + real providers | 1 421 |
| Memory store | Semantic memory with decay + KV store | 1 173 |
| Security audit | Static OWASP analysis | 1 075 |
| Embeddings | Cosine similarity search | 601 |
| **Total** | | **~23 200** |

### Project Structure

```
Metalogos-/
├── Cargo.toml                  # v0.9.1
├── src/
│   ├── grammar.pest             # PEG grammar (346 rules)
│   ├── ast.rs                   # AST definitions
│   ├── parser.rs                # Pest tokens -> AST
│   ├── semantic.rs              # Semantic analysis + opaque type enforcement
│   ├── interpreter.rs           # Tree-walking interpreter
│   ├── compiler.rs              # Bytecode compiler (142 builtins)
│   ├── bytecode.rs              # 44 VM instructions
│   ├── vm.rs                    # Bytecode VM executor
│   ├── jit.rs                   # JIT via Cranelift
│   ├── builtins.rs              # 142 built-in functions
│   ├── server.rs                # Axum HTTP server + cron scheduler
│   ├── llm.rs                   # LLM backend trait + providers
│   ├── memory_store.rs          # Semantic memory + KV store
│   ├── audit.rs                 # Static security audit
│   └── main.rs                  # CLI: run/repl/check/serve/compile/eval/audit
├── tests/                       # 33 integration test files (incl. phase23 v0.9.1 else-branch contracts)
├── examples/                    # 78 .mlog programs with golden tests
├── std/                         # Standard library (string, math, collections)
├── docs/adr/                    # 63 Architecture Decision Records
├── FOSVED-office-v2/            # Submodule: AI office with 14 departments
├── REFERENCE.md                 # Full builtin reference (RU)
└── CHANGELOG.md                 # Version history
```

---

## Version History (selected)

| Version | Highlights |
|---|---|
| **0.9.1** | Collection ops sync (142 builtins), BUILTIN_REGISTRY as single source of truth, line counts refreshed |
| **0.8.9** | Fix: else-branch parsing (else dropped silently), Fix: BlockIfElse mutation loss, 4 contract tests |
| **0.8.8** | Semantic fixes, compiler/VM builtin sync, variadic min-arity, 17 integration tests, chrono fix |
| **0.8.7** | Cron force_run bugfix, Memory Tree L2 global summary, mtree_stats |
| **0.8.6** | Memory Tree (L0/L1/L2), `call_pattern()`, cron user-pattern dispatch |
| **0.8.5** | Cron scheduler, goals/todos/preferences/approval builtins, 6 critical bugfixes |
| **0.8.4** | Telegram bot, extract_entities, memory_score, learn_preference, compress_html |
| **0.8.3** | map/zip/sort_by/filter/reduce, db_insert, matches_any |
| **0.8.2** | Telegram voice notes, inline keyboards, callbacks |
| **0.8.1** | Human Intelligence Layer (personas, mood, memory) |
| **0.8.0** | Time/date/calendar, weather, geolocation, reminders |
| **0.7.x** | HTTP server, encryption, auth, CSRF, OWASP, LLM cache, hooks |
| **0.6.x** | `let`/`if`/`each`/`while`, modules, break/continue, match |
| **0.4.x** | Bytecode VM + JIT (Cranelift) |
| **0.1–0.3** | Core: entity, pattern, flow, memory, rule, learn, adapt |

Full history: see [CHANGELOG.md](CHANGELOG.md).

---

## Roadmap

### Done (M1 — Phase 8.8)

All 8 milestones and 8+ phases complete. 21 development narads (work orders) delivered. 142 builtins, 33 test files, 78 golden-file examples, 63 ADRs.

### Next

| Target | Description |
|---|---|
| **Phase 9** | Self-hosted compiler, mlogpkg ecosystem, production deployment |

---

## Prior Art

- **Rust** — ownership model, opaque types, zero-cost abstractions
- **Haskell** — type-safe HTML (Yesod/Blaze), `newtype` for secrets
- **Pest** — PEG parser generator
- **Axum** — ergonomic async HTTP
- **Cranelift** — JIT code generation
- **Datalog / CLIPS** — declarative rule engines with priority
- **ACT-R** — memory activation and decay models
- **DSPy** — programmatic LLM orchestration
- **[OpenHuman](https://github.com/tinyhumansai/OpenHuman)** — persona system (inspiration for Human Intelligence Layer)

---

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE-2.0) at your option.

---

*Built with Rust. Designed by AI. For AI.*