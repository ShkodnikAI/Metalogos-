<div align="center">

<img src="assets/logo.png" alt="Metalogos" width="220"/>

# METALOGOS

**Open source AI-native programming language with security by design. Written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.12.0-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-4%20jobs-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)
[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

</div>

---

## What is Metalogos

Metalogos (mlog) is an open source programming language where AI operations — LLM calls, memory, learning, adaptation — are first-class language constructs, not library integrations. An LLM invocation is as natural as calling a function. Security constraints (XSS prevention, SQL injection prevention, secret opacity) are enforced at the language level, not through middleware.

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

## Competitive Advantages

### 1. AI as Language, Not Library

In Python/JS, AI requires SDKs, prompt templates, API clients, and HTTP error handling. In Metalogos, a learnable pattern is a language construct. Calling an LLM is indistinguishable from calling a function. No `openai.chat()`, no `await model.generate()`. The language handles context, caching, retry, and fallback automatically.

### 2. Security by Design — Zero Configuration

OWASP Top 10 is covered at the language level, not through middleware. XSS is impossible (Html is an opaque type with auto-escaping). SQL injection is impossible (parameterized queries only). Plaintext secret leakage is impossible (Secret type blocks print/to_string). There is no way to bypass these constraints — they are built into the semantics.

### 3. Dual Execution Backend

Tree-walking interpreter + bytecode VM (44 instructions) with identical semantics. Every program is verified by `crosscheck_backends` — a test ensuring both backends return the same result. This is unique for a language of this size.

### 4. Typed Semantic Memory with Hybrid Search

More than a key-value store. Hierarchical memory (Memory Tree L0/L1/L2), typed records (persona, episodic, instruction, fact), FTS5 BM25 + cosine similarity, merged via Reciprocal Rank Fusion (k=60). A full retrieval system built into the language.

### 5. Self-Modification with Sandbox and Rollback

The `adapt` statement allows a program to modify its own patterns at runtime — with sandboxing, quality metrics, and automatic rollback on degradation. No analogues exist in mainstream languages.

### 6. Complete Toolchain in One Binary

`mlog run`, `mlog serve`, `mlog compile`, `mlog repl`, `mlog check`, `mlog audit`, `mlog eval` — all in a single binary. The LSP server (`mlog-lsp`) and package manager (`mlogpkg`) are separate binaries in the same workspace. A VS Code extension with syntax highlighting is included in the repository.

---

## Seven Semantic Primitives

| Primitive | Purpose | Analogue in other languages |
|---|---|---|
| **Entity** | Typed data with identity, confidence, relations | Structs, objects, variables |
| **Pattern** | Transformations — pure, learnable (LLM), or hybrid | Functions, API calls |
| **Flow** | Declarative pipelines with confidence-based branching | Control flow, orchestrators |
| **Memory** | Typed semantic store with FTS5 BM25 + cosine RRF hybrid recall, decay | Databases, caches, vector stores |
| **Rule** | Probabilistic rules with priority and conflict resolution | If/else chains, business logic |
| **Learn** | Training as a language operation | ML frameworks, training scripts |
| **Adapt** | Runtime self-modification with sandbox and rollback | No direct analogue |

---

## Architecture

```
 .mlog source        Pest PEG          AST              Semantic            Two Backends
─────────────  ──>  ────────────  ──>  ───────────  ──>  ────────────  ──>  ────────────
 entity             parse tokens      27 Declaration    cross-reference     tree-walking
 pattern            syntax rules      14 Expr           validation          bytecode VM
 flow                                  12 Statement      opaque type       enforcement
 memory                                 4 MatchArm
 rule
 learn
 adapt
```

### Implementation Stack

| Component | Technology | Lines |
|---|---|---|
| Parser | Pest 2.7 PEG grammar (384 lines, ~180 rules) | 2 176 |
| AST | 27 Declaration variants, 14 Expr, 12 Statement, 4 MatchArm | 731 |
| Semantic analysis | Opaque types, arity checking, security audit | 473 |
| Compiler | Bytecode, 246+ builtins indexed | 659 |
| Bytecode format | 44 VM instructions | — |
| Tree-walking interpreter | Full feature support, 12 modules | ~4 400 |
| VM | Stack-based bytecode executor | 2 143 |
| Built-in functions | 246+ functions across 14 modules | ~9 500 |
| HTTP server | Axum 0.8 + Tokio, security middleware | 2 357 |
| LLM backend | Trait + mock + real providers | 1 421 |
| Memory store | Typed memory with FTS5 BM25 + cosine RRF hybrid recall + KV store | 1 540 |
| Security audit | Static OWASP analysis | 1 075 |
| Embeddings | TF-IDF + OpenAI cosine similarity, FTS5 BM25 | 601 |
| **Total effective Rust LOC** | | **~30 500** |

### Project Structure

```
Metalogos-/
├── Cargo.toml                       # v0.12.0, workspace root
├── assets/                            # Brand assets
│   ├── logo.png                     # Brand logo (PNG)
│   ├── logo.svg                     # Brand logo (SVG)
│   └── qr_wallet.jpg               # Crypto wallet QR code
├── README.md                         # This file
├── REFERENCE.md                      # Full builtin reference (50 KB)
├── CHANGELOG.md                      # Version history (45 KB)
├── FEATURE_INTAKE.md                 # Feature request tracking
├── MEMORY_ROADMAP.md                 # Memory system roadmap
├── Dockerfile                        # Docker build
├── CNAME                             # Custom domain
├── index.html                        # Landing page / docs site
│
├── src/                              # Core compiler + interpreter (~30 500 LOC)
│   ├── main.rs                        # CLI: run/repl/check/serve/compile/eval/audit
│   ├── grammar.pest                   # Pest PEG grammar (384 lines)
│   ├── ast.rs                         # AST definitions (27 Declaration variants)
│   ├── semantic.rs                    # Semantic analysis + opaque type enforcement
│   ├── compiler.rs                    # Bytecode compiler
│   ├── bytecode.rs                    # VM instruction set (44 instructions)
│   ├── vm.rs                          # Bytecode VM executor
│   ├── jit.rs                         # JIT scaffold (experimental, ADR-0073)
│   ├── server.rs                      # Axum HTTP server + cron scheduler
│   ├── llm.rs                         # LLM backend trait + providers
│   ├── memory_store.rs               # Semantic memory + KV store (SQLite)
│   ├── memory_graph.rs               # Knowledge graph (petgraph)
│   ├── audit.rs                       # Static security audit
│   ├── embedding.rs / embeddings.rs  # TF-IDF + OpenAI cosine similarity
│   ├── ml.rs                          # ML backend
│   ├── error.rs                       # Error types
│   ├── codegen.rs                     # Code generation
│   ├── ir.rs                          # Intermediate Representation
│   │
│   ├── parser/                        # Pest tokens -> AST
│   │   ├── mod.rs, expr.rs, stmt.rs, decl.rs, helpers.rs, tests.rs
│   │
│   ├── interpreter/                   # Tree-walking interpreter
│   │   ├── mod.rs                     # Interpreter entry
│   │   ├── execution.rs               # Execution engine + builtin dispatch
│   │   ├── flow.rs                    # Control flow (if/while/break/continue)
│   │   ├── values.rs                  # Runtime values
│   │   ├── types.rs                   # Type system + opaque types
│   │   ├── modules.rs                 # Module system
│   │   ├── hooks.rs                   # 5 lifecycle hooks
│   │   ├── events.rs                  # Event stream
│   │   ├── memory.rs                  # Memory operations
│   │   ├── conversations.rs           # Conversation state
│   │   ├── db.rs                      # SQLite database access
│   │   └── learnable.rs               # Learnable pattern support
│   │
│   └── builtins/                      # 246+ built-in functions (14 modules)
│       ├── mod.rs                     # Builtin dispatch
│       ├── registry.rs               # BUILTIN_REGISTRY (SSOT for all builtins)
│       ├── core.rs                    # print, let, type, inspect, sleep
│       ├── string.rs                  # String operations
│       ├── math.rs                    # Math operations
│       ├── collections.rs            # List/map operations
│       ├── json.rs                    # JSON parsing/serialization
│       ├── io.rs                      # File I/O
│       ├── http.rs                    # HTTP client + sleep builtin
│       ├── llm.rs                     # LLM call builtins
│       ├── server.rs                  # HTTP server (serve, route, respond)
│       ├── crypto.rs                  # AES-GCM, Argon2, HMAC-SHA256, hashing
│       ├── memory.rs                  # Memory/graph builtins
│       ├── cron.rs                    # Cron scheduler
│       ├── office.rs                  # Office document handling
│       ├── pdf.rs                     # PDF processing (pdf-inspector)
│       └── tests.rs                   # Builtin unit tests
│
├── mlog-lsp/                          # LSP server (workspace crate)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                    # LSP server binary
│       ├── lib.rs                     # LSP implementation (diagnostics, goto-def, hover)
│   └── tests/
│       └── lsp_integration.rs        # 8 integration tests
│
├── mlogpkg/                           # Package manager (workspace crate)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs                    # mlogpkg binary
│   └── tests/
│       └── pkg_integration.rs
│
├── tests/                             # 37 Rust test files
│   ├── fixtures/                      # PDF test fixtures
│   ├── golden.rs                      # Golden test runner (66/70 pass)
│   ├── vm_golden.rs                   # VM golden tests
│   ├── jit_golden.rs                  # JIT golden tests
│   ├── crosscheck_backends.rs          # TW vs VM parity verification
│   ├── repl_integration.rs            # REPL tests
│   ├── definition_of_done.rs          # Project completeness validation
│   └── ...                            # Contract + feature tests (35 files)
│
├── examples/                          # 116 .mlog programs
│   ├── m1_hello.mlog                  # Hello World
│   ├── p6_full_app.mlog               # Full web app with routes
│   ├── p23_ml_learn.mlog              # ML learning
│   ├── dag_demo.mlog                  # DAG orchestration
│   └── contracts/                     # Golden-file test contracts
│
├── std/                               # Standard library (3 .mlog files)
│   ├── string.mlog
│   ├── math.mlog
│   └── collections.mlog
│
├── self-host/                         # Self-hosting experiments
│   ├── lexer.mlog                     # Lexer written in .mlog itself
│   └── std/                           # Copies of std/ for self-hosted execution
│
├── editors/vscode/                    # VS Code extension
│   ├── package.json
│   ├── language-configuration.json
│   └── syntaxes/mlog.tmLanguage.json  # TextMate syntax highlighting
│
├── assets/                            # Brand assets (logo.png, logo.svg, qr_wallet.jpg)
│
├── benches/                           # Criterion benchmarks
│   └── core_benchmarks.rs
│
├── docs/
│   └── adr/                           # 91 Architecture Decision Records (ADR-0001..0090)
│       ├── 0001-m1-architecture.md
│       ├── ...
│       └── 0090-rule-priority-semantics.md
│
├── scripts/                           # Development utilities (7 files)
│   ├── add_builtins.py                # Builtin registration helpers
│   ├── dl_artifact.py                 # CI artifact downloader
│   └── generate_o2_order.js           # Narad execution order generator
│
└── .github/workflows/                  # CI/CD
    ├── ci.yml                         # test-lib (blocking), test-integration, fmt, clippy
    └── build.yml                      # Release build + artifact upload
```

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

### Two Execution Backends

- **Tree-walking interpreter** — full feature support, used for `mlog run` and `mlog serve`
- **Bytecode VM** — 44 instructions, stack-based, used for `mlog compile` + `mlog run file.mbc`
- **JIT** — declared experimental, scaffold only (see ADR-0073)

### 246+ Built-in Functions

String ops, math, collections, type conversion, LLM/AI, HTTP, JSON, file I/O, KV store, session memory, encryption, authentication, HTTP server, templates, databases, Telegram/Discord bots, time/date/calendar, geolocation, weather, reminders, cron, goals, todos, memory tree, preferences, approval workflows, fuzzy matching, hashline editing, context compaction, budget awareness, replay logging, policy enforcement, PDF processing (classify, extract, OCR), typed semantic memory (FTS5 BM25 + cosine RRF), and more. See [REFERENCE.md](REFERENCE.md) for the full list.

### Human Intelligence Layer

Persona system with memory trees, mood tracking, and human-like response generation — inspired by [OpenHuman](https://github.com/tinyhumansai/OpenHuman):

```mlog
human_create("Alice", "friendly, professional, curious")
human_remember("Alice", "project", "building AI assistant in Metalogos", 0.8)
human_mood("Alice", "excited", 0.9)
let reply = human_respond("Alice", "How is my project going?")
```

### Cron Scheduler

Fuzzy matching (Jaro-Winkler), content-verified hashline editing (CRC32), context compaction, budget awareness, replay logging, shell policy enforcement:

```mlog
fuzzy_match("metalogos", "metalogus")           // 0.96
fuzzy_find_best("Mikhail", ["Michele", "Mikael"])  // FuzzyMatch{index:1, candidate:"Mikael", score:0.82}
hashline_read(code)                                  // "1:3f|fn main() {"
hashline_edit(text, [{op:"set_line", ref:"3:ab", content:"..."}])
compact_list(messages, 2, 4)                          // protect head/tail, compress middle
budget_check(8, 10)                                   // BudgetStatus{level:"warning", pct_remaining:20}
policy_check("vim file.txt")                          // PolicyResult{allowed:false, reason:"blocked: interactive..."}
replay_snapshot(events)                              // ReplaySnapshot{seq:0, count:5, snapshot:"..."}
```

Cron scheduler, recurring and one-shot jobs, dispatches both builtins and user patterns:

```mlog
cron_run("*/30 * * * *", "HealthCheck")     // every 30 min
cron_run("0 9 * * 1-5", "MorningReport")     // weekdays 09:00
cron_list()   // list all jobs
```

### Memory Tree

Three-level hierarchical memory: L0 (raw entries) → L1 (chunk summaries) → L2 (global summary). Admission-gated storage, keyword-relevance retrieval with scoring, and automatic compression.

### Typed Memory with Hybrid Search (ADR-0072, ADR-0073)

Memory entries carry a type tag (`persona`, `episodic`, `instruction`, `fact`) for differentiated recall. SQLite-backed persistence with FTS5 BM25 keyword index + cosine similarity, merged via Reciprocal Rank Fusion (k=60). Top-K recall with type filtering:

```mlog
memorize("user likes spicy food", 0.9, "persona")
let results = recall_top_k("food preferences", 5, "persona")
```

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

### LSP Server

```bash
# Build LSP server (requires workspace)
cargo install --path mlog-lsp

# Use with VS Code — extension included in editors/vscode/
# Or run standalone:
mlog-lsp
```

### Package Manager

```bash
cargo install --path mlogpkg
mlogpkg init my-project
mlogpkg add dependency@1.0
```

---

## CI/CD

<div align="center">

| Job | Status | Type |
|---|---|---|
| `test-lib` | **Blocking** | Unit + golden tests (401 pass) |
| `test-integration` | Advisory | Integration tests (continue-on-error) |
| `fmt` | **Blocking** | `cargo fmt --check` |
| `clippy` | **Blocking** | `cargo clippy -- -D warnings` |

</div>

Release builds run on push to main — produces `mlog-linux-x86_64` binary artifact.

---

## Ecosystem

| Component | Description |
|---|---|
| **mlog** | Core compiler, interpreter, VM, server, REPL |
| **mlog-lsp** | LSP server — diagnostics, goto-definition, hover |
| **mlogpkg** | Package manager for .mlog projects |
| **VS Code extension** | Syntax highlighting + language configuration |
| **Self-hosted compiler** | Lexer written in .mlog itself (self-host/) |

---

## Technology Stack

| Category | Technologies |
|---|---|
| Language | Rust 1.85+ (edition 2021) |
| Parser | Pest 2.7 PEG grammar |
| CLI | clap 4.5 (derive) |
| REPL | rustyline 14 |
| Web server | Axum 0.8 + Tokio + Tower |
| HTTP client | reqwest 0.12 (rustls-tls) |
| Database | rusqlite 0.31 (bundled SQLite) |
| Crypto | AES-GCM, Argon2, HMAC-SHA256 |
| Serialization | serde, serde_json, bincode, serde_yaml, toml |
| Graph | petgraph 0.7 |
| Concurrency | dashmap 6.1 |
| LSP | tower-lsp 0.20 |
| Cron | chrono 0.4 |
| Benchmarks | criterion 0.5 |
| PDF | pdf-inspector 0.1 (optional tesseract OCR) |

---

## Project Metrics

| Metric | Value |
|---|---|
| Effective Rust LOC | ~30 500 |
| Built-in Functions | 246+ (14 modules) |
| Example Programs | 116 |
| Integration Tests | 37 test suites |
| Architecture Decision Records | 91 |
| Parser Rules | ~180 (Pest PEG) |
| VM Instructions | 44 |
| Execution Backends | 2 (interpreter + bytecode VM) |
| Workspace Crates | 3 (mlog, mlog-lsp, mlogpkg) |
| Commits | 463 |
| License | MIT / Apache-2.0 |

---

## Version History (selected)

| Version | Highlights |
|---|---|
| **0.12.0** | Production hardening: security, reliability, slice(), db_execute params, CI |
| **0.11.0** | obsidian-mind inspired: 5 lifecycle hooks, config_load YAML support |
| **0.10.0** | obsidian-mind inspired: semantic_search, config_load, vault_validate |
| **0.9.5** | OpenPlanter-inspired: fuzzy matching, hashline editing, compact_list, budget_check, replay_snapshot, policy_check |
| **0.9.4** | AgentSkillOS: recipe system, DAG orchestration |
| **0.9.3** | sqz-inspired string/list/token utilities |
| **0.9.1** | Collection ops sync (142 builtins), BUILTIN_REGISTRY SSOT |
| **0.8.9** | Fix: else-branch parsing, Fix: BlockIfElse mutation loss |
| **0.8.5** | Cron scheduler, goals/todos/preferences/approval builtins |
| **0.8.4** | Telegram bot, extract_entities, memory_score |
| **0.8.1** | Human Intelligence Layer (personas, mood, memory) |
| **0.8.0** | Time/date/calendar, weather, geolocation, reminders |
| **0.7.x** | HTTP server, encryption, auth, CSRF, OWASP, LLM cache, hooks |
| **0.6.x** | `let`/`if`/`each`/`while`, modules, break/continue, match |
| **0.4.x** | Bytecode VM |
| **0.1–0.3** | Core: entity, pattern, flow, memory, rule, learn, adapt |

Full history: see [CHANGELOG.md](CHANGELOG.md).

---

## Roadmap

### Done (M1 — Phase 8.8)

All 8 milestones and 8+ phases complete. 24+ development narads (work orders) delivered. 246+ builtins, 37 test files, 116 golden-file examples, 91 ADRs. 463 commits.

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
- **Datalog / CLIPS** — declarative rule engines with priority
- **ACT-R** — memory activation and decay models
- **DSPy** — programmatic LLM orchestration
- **[OpenHuman](https://github.com/tinyhumansai/OpenHuman)** — persona system (inspiration for Human Intelligence Layer)

---

## Support the Project

Metalogos is developed by a solo developer. Your support helps keep the project alive and growing.

### Open Collective (Primary)

Transparent funding for the project. See exactly where every dollar goes.

[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

[opencollective.com/metalogos](https://opencollective.com/metalogos)

### Cryptocurrency (USDT TRC-20)

Direct support from anywhere in the world, regardless of banking restrictions:

| Scan to donate | Wallet Address |
|----------------|----------------|
| <img src="assets/qr_wallet.jpg" width="150"> | `USDT (TRC-20): TU6adaaFxJdmvXRT8fhu9w3NQJeNueUyJk` |

**Network:** TRC-20 (Tron)

### Sponsorship Tiers

| Tier | Open Collective | Crypto (USDT) | Benefits |
|------|----------------|---------------|----------|
| Supporter | $5/mo | 50 USDT/mo | Name in SPONSORS.md, early releases, Discord |
| Contributor | $25/mo | 250 USDT/mo | + Vote on features, beta access, monthly updates |
| Sponsor | $100/mo | 1000 USDT/mo | + Logo on website, 1hr consulting, priority support |
| Corporate | $500/mo | 5000 USDT/mo | + Enterprise support, private registry, roadmap calls |

*Thank you to all our backers!*

---

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE-2.0) at your option.

---

*Built with Rust. Designed by AI. For AI.*
