<div align="center">

<img src="logo.jpg" alt="Metalogos" width="220"/>

# METALOGOS

**AI-native programming language with security by design. Written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.17.0-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-6%20blocking%20jobs-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)
[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

</div>

<div align="center">

<a href="https://youtube.com/shorts/Y8IFieOZQLo?feature=shared"><img src="https://img.youtube.com/vi/Y8IFieOZQLo/hqdefault.jpg" alt="Metalogos Presentation" width="400"/></a>

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

OWASP Top 10 is covered at the language level, not through middleware.

**Eliminated by the compiler** (these are structural errors — `mlog check`, `mlog run`, `mlog serve`, and `mlog compile` all refuse to proceed):
- **SQL injection is impossible** — `query()` with non-literal SQL is a compile-time error; only parameterized queries are allowed
- **Plaintext secret leakage is impossible** — passing `env()` results to `respond()`/`write_file()` is a compile-time error
- **XSS via LLM output is impossible** — passing `call_llm()`/`call_claude()` results to `respond()` without `render()`/`escape_html()` is a compile-time error

**Checked by `mlog audit`** (these are heuristic advisories — context-dependent, may have legitimate exceptions):
- Hardcoded secrets in source (heuristic; can false-positive on error messages)
- Missing sandbox for `adapt`/`mutate` (cross-file context needed)
- Missing `rate_limit` middleware (external infra may handle it)
- Missing CSRF middleware (not needed for token-authenticated APIs)
- Open redirect via user-controlled `respond_html()` (custom validation not recognized)

### 3. Dual Execution Backend

Tree-walking interpreter + bytecode VM (46 instructions) with identical semantics. Every program is verified by `crosscheck_backends` — a test ensuring both backends return the same result. This is unique for a language of this size.

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
| Parser | Pest 2.7 PEG grammar (~400 lines, 180 rules) | 2 176 |
| AST | 27 Declaration variants, 14 Expr, 12 Statement, 4 MatchArm | 731 |
| Semantic analysis | Opaque types, arity checking, Category A audit (SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION), SVG XSS lint | 473 |
| Compiler | Bytecode, 359 builtins indexed | 659 |
| Bytecode format | 46 VM instructions | — |
| Tree-walking interpreter | Full feature support, 12 modules | ~4 400 |
| VM | Stack-based bytecode executor | 2 143 |
| Built-in functions | 359 functions across 22 modules | ~9 500 |
| HTTP server | Axum 0.8 + Tokio, security middleware | 2 357 |
| LLM backend | Trait + mock + real providers | 1 421 |
| Memory store | Typed memory with FTS5 BM25 + cosine RRF hybrid recall + KV store | 1 540 |
| Security audit | Static OWASP analysis | 1 075 |
| Embeddings | TF-IDF + OpenAI cosine similarity, FTS5 BM25 | 601 |
| **Total effective Rust LOC** | | **~58 000** |

### Project Structure

```
Metalogos-/
├── Cargo.toml                       # v0.17.0, workspace root
├── logo.jpg                          # Brand logo
├── README.md                         # This file
├── REFERENCE.md                      # Full builtin reference (50 KB)
├── CHANGELOG.md                      # Version history (45 KB)
├── FEATURE_INTAKE.md                 # Feature request tracking
├── MEMORY_ROADMAP.md                 # Memory system roadmap
├── Dockerfile                        # Docker build
├── CNAME                             # Custom domain
├── index.html                        # Landing page / docs site
│
├── src/                              # Core compiler + interpreter (~58 000 LOC)
│   ├── main.rs                        # CLI: run/repl/check/serve/compile/eval/audit
│   ├── grammar.pest                   # Pest PEG grammar (384 lines)
│   ├── ast.rs                         # AST definitions (27 Declaration variants)
│   ├── semantic.rs                    # Semantic analysis + opaque type enforcement
│   ├── compiler.rs                    # Bytecode compiler
│   ├── bytecode.rs                    # VM instruction set (46 instructions)
│   ├── vm.rs                          # Bytecode VM executor
│   ├── server.rs                      # Axum HTTP server + cron scheduler
│   ├── llm.rs                         # LLM backend trait + providers
│   ├── memory_store.rs               # Semantic memory + KV store (SQLite)
│   ├── memory_graph.rs               # Knowledge graph (petgraph)
│   ├── audit.rs                       # Static security audit
│   ├── embeddings.rs                  # TF-IDF + OpenAI cosine similarity
│   ├── error.rs                       # Error types
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
│   └── builtins/                      # 359 built-in functions (22 modules)
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
├── tests/                             # 52 Rust test files
│   ├── fixtures/                      # PDF test fixtures
│   ├── golden.rs                      # Golden test runner (66/70 pass)
│   ├── vm_golden.rs                   # VM golden tests
│   ├── crosscheck_backends.rs          # TW vs VM parity verification
│   ├── repl_integration.rs            # REPL tests
│   ├── definition_of_done.rs          # Project completeness validation
│   └── ...                            # Contract + feature tests (52 files)
│
├── examples/                          # 174 .mlog programs
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
├── assets/                            # Brand assets
│   ├── logo.svg
│   └── qr_wallet.jpg                  # Crypto wallet QR code
│
├── benches/                           # Criterion benchmarks
│   └── core_benchmarks.rs
│
├── docs/
│   └── adr/                           # 100 Architecture Decision Records (ADR-0001..0103)
│       ├── 0001-m1-architecture.md
│       ├── ...
│       └── 0103-idiomatic-ignore-reasons.md
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

The compiler enforces structural security invariants — these are errors, not warnings:

```mlog
// SQL injection impossible — non-literal query() is a compile-time error
let user = query("SELECT * FROM users WHERE id = $1", [id])

// Secret leak impossible — env() to respond() is a compile-time error
entity token: Secret = env("API_KEY")
respond(token)   // Compile error: [SECRET_LEAK]

// XSS via LLM impossible — unsanitized LLM output to respond() is a compile-time error
let reply = call_llm(prompt)
respond(reply)   // Compile error: [HTML_INJECTION] — use render() or escape_html()
```

Heuristic security checks (hardcoded secrets, sandbox coverage, rate limiting, CSRF, open redirect) are advisory — run `mlog audit` for a full report.

### OWASP Top 10 Coverage

Every item in the OWASP Top 10 (2021) is addressed at the language level: type-safe HTML (A03), parameterized queries (A03), Secret/Encrypted/Hash opaque types (A02), role-based routes + `require` assertions (A01), HMAC-SHA256 sessions (A07), CSRF double-submit (A08), CSP/HSTS/X-Frame-Options headers (A05), LLM sandbox (A10), audit logging (A09).

### Two Execution Backends

- **Tree-walking interpreter** — full feature support, used for `mlog run` and `mlog serve`
- **Bytecode VM** — 46 instructions, stack-based, used for `mlog compile` + `mlog run file.mbc`
- **JIT** — experimental scaffold, not part of the build (see ADR-0073)

### 359 Built-in Functions

String ops, math, collections, type conversion, LLM/AI, HTTP, JSON, file I/O, KV store, session memory, encryption, authentication, HTTP server, templates, databases, Telegram/Discord bots, time/date/calendar, geolocation, weather, reminders, cron, goals, todos, memory tree, preferences, approval workflows, fuzzy matching, hashline editing, context compaction, budget awareness, replay logging, policy enforcement, PDF processing (classify, extract, OCR), typed semantic memory (FTS5 BM25 + cosine RRF), SMTP/IMAP email, CalDAV/CardDAV calendar and contacts, native SVG graphics, and more. See [REFERENCE.md](REFERENCE.md) for the full list.

### Native SVG/Graphics Subsystem

42 builtins, hand-rolled in pure Rust — zero external SVG/charting/
rendering dependencies. Every text-carrying function is covered by a
dedicated static security lint (`SVG_AUTO_ESCAPE_BUILTINS` /
`SVG_NO_ESCAPE_BUILTINS`), the same "unsafe by construction" discipline
as the rest of the language:

```mlog
let style = color_palette("energy", "dark")
let chart = chart_bar(revenue_data, style)
let flow  = diagram_flowchart(nodes, edges, style)
let poster = InfographicPoster("Q3 Revenue", "energy", stats, narrative)
```

- **Primitives** — `svg_rect`, `svg_circle`, `svg_path`, `svg_text`,
  `svg_group`, `svg_canvas` (+ named presets: `doc_inline`,
  `slide_16x9`, `social_og`, print A4), `svg_icon` (10 glyphs)
- **9 chart types** — bar, donut, line, scatter, area, heatmap, radar
  (multi-series), boxplot (real quartile math)
- **22 diagram types** — flowchart (topological layering, cycle
  detection), tree, org chart, sequence, timeline, Gantt, state machine,
  ER, swimlane, Venn (2/3-circle), quadrant, pyramid, and more
- **`color_palette`** — HSL-cascade generator, 5 intents × 2 modes
- **`template_render`** — `{{ var }}`, `{{#if}}/{{else}}`, `{{#each}}`,
  hand-written recursive parser, no templating crate
- **Anti-overlap engine** — iterative label-collision resolution,
  wired into `diagram_timeline`
- **`infographic_qa`** — advisory contrast/saturation/density checks
- **`html_render`** — headless-browser screenshot, hardened `exec()`
  underneath (real timeout + kill, file audit log, no shell
  interpretation); network isolation is a documented caller
  responsibility, not an OS-level guarantee
- **`std/infographic.mlog`** — `InfographicPoster`, `InfographicDashboard`,
  `InfographicComparison`, `InfographicTimeline`

Delivered across naryads №77–92 (see [CHANGELOG.md](CHANGELOG.md)).
Found and fixed one critical, previously-invisible bug along the way:
the bytecode VM discarded `try`'s result on the success path since
naryad №14 — masked for the project's entire history because every
existing `try`-using test only checked the error path.

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
| Effective Rust LOC | ~58 000 |
| Built-in Functions | 359 (22 modules) |
| Example Programs | 174 |
| Integration Tests | 52 test suites |
| Architecture Decision Records | 100 |
| Parser Rules | ~180 (Pest PEG) |
| VM Instructions | 46 |
| Execution Backends | 2 (interpreter + bytecode VM) |
| Workspace Crates | 3 (mlog, mlog-lsp, mlogpkg) |
| Commits | 653 |
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

All 8 milestones and 8+ phases complete, plus a full native SVG/graphics subsystem (naryads №77-92). 92+ development narads (work orders) delivered. 359 builtins, 52 test files, 174 golden-file examples, 100 ADRs. 653 commits.

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

---

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE-2.0) at your option.

---

*Built with Rust. Designed by AI. For AI.*
