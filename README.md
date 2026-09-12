<div align="center">

<img src="logo.jpg" alt="Metalogos" width="220"/>

# METALOGOS

**AI-native programming language with security by design. Written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.19.0-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-15%20blocking%20jobs-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)
[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

</div>

<div align="center">

<a href="https://youtube.com/shorts/Y8IFieOZQLo?feature=shared"><img src="https://img.youtube.com/vi/Y8IFieOZQLo/hqdefault.jpg" alt="Metalogos Presentation" width="400"/></a>

<sub>Short, Aug 12 2026 — visual pitch, not the security contract; see <a href="#security-by-design">Known boundaries</a> for what's actually guaranteed.</sub>

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
goal_set("launch v1.0", "2026-12-01")
todo_add("Fix cron edge case", "high")

// Call it like any other pattern
let result = Classify("where is my order?")
```

No frameworks. No boilerplate. No `import ai_sdk`. The language *is* the AI infrastructure.

---

## Why Metalogos

**In 30 seconds** — call an LLM like a function and get validated JSON back, then watch the compiler refuse to leak a secret:

```mlog
pattern Extract(x: String) -> String {
  let person = call_llm_schema("Extract the user as JSON", x,
    "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"]}")
  return json_get(person, "name")          // Struct field, not text scraping
}

pattern LeakApiKey() -> String {
  let key = env("FAKE_API_KEY")
  respond("200", "key is " + key)          // ← rejected: SECRET_LEAK
  return key
}
```

```text
$ mlog check api_leak.mlog
1 error:
  1: строка 1: [SECRET_LEAK] secret may be leaked — env() value passed to respond()
```

(Both outputs are real — this exact probe is what `mlog check` prints, exit code 1. See [REFERENCE.md §4.5](REFERENCE.md) for `call_llm_schema`, and [ADR-0133](docs/adr/0133-llm-schema-validator.md) for the validation contract.)

**Security by design** — not a library, a property of the language: static taint tracking turns secret leaks, SQL injection and raw LLM output into compile errors (Category A); file I/O is sandboxed; `exec()`/`env()` in serve routes are denied by default with stable diagnostic codes and an opt-in allowlist model; subprocesses land in the audit log. The full contract — including the honest list of what static analysis does NOT catch — is in [Security by Design](#2-security-by-design--zero-configuration) below.

**AI-native** — eight semantic primitives (Entity, Pattern, Flow, Memory, Rule, Learn, Adapt, Reflex) make AI operations first-class: learnable patterns teach from examples, Memory combines BM25 + vector search with rank fusion, `adapt` modifies the program's own patterns under a sandbox, Reflex distills LLM teachers into local models.

**MCP-native** — Metalogos speaks the integration standard of 2026-era AI agents in both directions, with the same security gates: the MCP client design is pinned in [ADR-0132](docs/adr/0132-mcp-client.md) (stdio transport, hand-rolled JSON-RPC, exec-gated server spawn, untrusted `UserInput` taint on tool output — the design is under owner review as of this release, the client implementation lands in naryad №268), and the reverse bridge (expose Metalogos `tool` constructs as MCP servers) follows it — [ADR-0054](docs/adr/0054-tool-abstraction.md) §Future Directions.

---

## Competitive Advantages

### 1. AI as Language, Not Library

In Python/JS, AI requires SDKs, prompt templates, API clients, and HTTP error handling. In Metalogos, a learnable pattern is a language construct. Calling an LLM is indistinguishable from calling a function. No `openai.chat()`, no `await model.generate()`. The language handles context, caching, retry, and fallback automatically.

### 2. Security by Design — Zero Configuration

OWASP Top 10 is addressed at the language level through a combination of compiler-enforced checks, opaque types, and static audit heuristics.

**Rejected by static checks** (Category A — `mlog check`, `mlog run`, `mlog serve`, and `mlog compile` all refuse to proceed for these data-flow shapes):
- **SQL injection** — `query()`/`db_execute()` with non-literal SQL is rejected; only parameterized queries compile
- **Plaintext secret leakage** — passing `env()` results to `respond()`/`write_file()`/`http_post()` body is rejected
- **XSS via LLM output** — passing `call_llm()`/`call_claude()` results to `respond()` without `render()`/`escape_html()` is rejected — for both direct variable flow and single-level inline nesting; see Known boundaries below

**Checked by `mlog audit`** (heuristic advisories — context-dependent, may have legitimate exceptions):
- Hardcoded secrets in source (heuristic; can false-positive on error messages)
- Missing sandbox for `adapt`/`mutate` (cross-file context needed)
- Missing `rate_limit` middleware (external infra may handle it)
- Missing CSRF middleware (not needed for token-authenticated APIs)
- Open redirect via user-controlled `respond_html()` (custom validation not recognized)
- Taint through `memorize`/`recall` persistence (file-level heuristic)
- Taint through trivial passthrough pattern indirection (single-param, `return param` only)

**Runtime exec & env gates** (opt-in flags, denied by default with `EXEC_NOT_PERMITTED` / `ENV_NOT_PERMITTED`):
- `exec()` / `exec_argv()` in process contexts (`mlog run`, `mlog check`, serve top level) require `METALOGOS_ALLOW_EXEC=1`
- `exec()` / `exec_argv()` in serve route bodies require `METALOGOS_SERVE_ALLOW_EXEC=1` — route handlers do **not** inherit `METALOGOS_ALLOW_EXEC` (replacement semantics, not AND; Naryad №253 Variant A). The serve banner prints the route-exec state at startup.
- `env()` in serve route bodies is denied by default with `ENV_NOT_PERMITTED` (Naryad №259) — route code must not read the process's secrets. Escape hatches (alternatives, not AND): `METALOGOS_SERVE_ALLOW_ENV=1` allows all env reads in route bodies, or `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` allows exactly the listed names. Outside serve `env()` stays ungated. The serve banner prints the route-env state at startup.

#### Known boundaries of static analysis

These checks use **intraprocedural taint tracking** — they follow `let`-assignment chains within a single pattern body. The following patterns are **not** detected at compile time:

| Pattern | Why not caught |
|---|---|
| LLM output passed via pattern call (interprocedural) | Taint does not cross pattern boundaries |
| LLM output stored via `memorize()` then read back via `recall()` | Data flow through persistence is not tracked |
| `query(format("...", x))` | `format()` output is not a literal string; check requires compile-time constant |
| `{{{ var }}}` (raw template substitution) | `template_render` with `raw=true` skips escaping by design — trusted author code only |

`mlog audit` provides **heuristic warnings** (not errors) for two narrow sub-cases:

| Warning | Scope | Example |
|---|---|---|
| `TAINT_PERSISTENCE` | Same-scope: `memorize(call_llm(...))` + `recall()` + `respond()` | `memorize call_llm("summarize")` then `let ctx = recall("q"); respond("200 OK", ctx)` |
| `TAINT_PASSTHROUGH` | Trivial passthrough pattern wrapping LLM output | `pattern Wrap(x: String) { return x }` then `respond("200 OK", Wrap(call_llm("...")))` |

These are file-level heuristics, not data-flow guarantees — they may false-positive in safe code and miss complex indirection.

### 3. Dual Execution Backend

Tree-walking interpreter (full language) + bytecode VM (47 instructions; experimental for full-language use — `match` (statement and `let`-binding expression) and block `if/else` **expression** not supported yet, see [ADR-0105](docs/adr/0105-vm-experimental-scope.md)). Programs both backends can run are checked by `crosscheck_backends` for TW↔VM output parity.

### 4. Typed Semantic Memory with Hybrid Search

More than a key-value store. Hierarchical memory (Memory Tree L0/L1/L2), typed records (persona, episodic, instruction, fact), FTS5 BM25 + cosine similarity, merged via Reciprocal Rank Fusion (k=60). A full retrieval system built into the language.

### 5. Self-Modification with Sandbox and Rollback

The `adapt` statement allows a program to modify its own patterns at runtime — with sandboxing, few-shot mutation, and automatic rollback. The rollback mechanism is real and tested. Quality metric is currently a fixed mock value (0.95), not a real accuracy computation — rollback logic exists but does not yet respond to actual quality degradation. See ADR-0112. Revisit point (recorded 2026-09-10 after an external audit): revisit only on a real `mutate` use case where the mock value creates a concrete problem (ADR-0112 addendum).

**Sandbox timeout caveat**: when a `sandbox` block specifies `timeout > 0`, both the calling thread's wait AND the underlying LLM request are cancelled at the deadline — on every call path. SmartRouter routes cancel via the HTTP client timeout (real TCP drop; Naryad №156); the legacy backend path cancels via `call_with_deadline` (Naryad №248): RealLlm drops the TCP connection at min(deadline, 120s), the mock sleeps min(delay, deadline). External on-demand abort (a language construct, or cancellation on client disconnect in server mode) is not supported — revisit when a real use case appears.

### 6. Complete Toolchain in One Binary

`mlog run`, `mlog serve`, `mlog compile`, `mlog repl`, `mlog check`, `mlog audit`, `mlog eval` — all in a single binary. The LSP server (`mlog-lsp`) and package manager (`mlogpkg`) are separate binaries in the same workspace. A VS Code extension with syntax highlighting is included in the repository.

---

## Eight Semantic Primitives

| Primitive | Purpose | Analogue in other languages |
|---|---|---|
| **Entity** | Typed data with identity, confidence, relations | Structs, objects, variables |
| **Pattern** | Transformations — pure, learnable (LLM), or hybrid | Functions, API calls |
| **Flow** | Declarative pipelines with confidence-based branching | Control flow, orchestrators |
| **Memory** | Typed semantic store with FTS5 BM25 + cosine RRF hybrid recall, decay | Databases, caches, vector stores |
| **Rule** | Probabilistic rules with priority and conflict resolution | If/else chains, business logic |
| **Learn** | Training as a language operation | ML frameworks, training scripts |
| **Adapt** | Runtime self-modification with sandbox and rollback | No direct analogue |
| **Reflex** | Local neural models — train, predict, persist, distill. LLM as teacher, local head as student (closes ADR-0112) | ML inference, distillation |

---

## Architecture

```
 .mlog source        Pest PEG          AST                Semantic            TW + VM backends
─────────────  ──>  ────────────  ──>  ───────────  ──>  ────────────  ──>  ────────────
 entity             parse tokens      29 Declaration    cross-reference     tree-walking
 pattern            syntax rules      15 Expr           validation          bytecode VM
 flow                                  12 Statement      opaque type       enforcement
 memory                                 4 MatchArm        span-aware
 rule                                                    error messages
 learn
 adapt
 reflex
```

### Implementation Stack

| Component | Technology | Lines |
|---|---|---|
| Parser | Pest 2.7 PEG grammar (~522 lines, 302 rules) | 2 176 |
| AST | 33 Declaration variants, 14 Expr, 12 Statement, 4 MatchArm, span tracking (ADR-0111) | 1 289 |
| Semantic analysis | Opaque types, arity checking, Category A audit (SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION, VISION_UNSIGNED_EXPORT, MODEL_WEIGHTS_UNSAFE), SVG XSS lint | 473 |
| Compiler | Bytecode, 395 builtins indexed | 1 516 |
| Bytecode format | 46 VM instructions | — |
| Tree-walking interpreter | Full feature support, 12 modules | ~4 400 |
| VM | Stack-based bytecode executor | 2 143 |
| Built-in functions | 395 functions across 37 modules | ~18 000 |
| HTTP server | Axum 0.8 + Tokio, security middleware | 2 433 |
| LLM backend | Trait + mock + real providers | 1 421 |
| Memory store | Typed memory with FTS5 BM25 + cosine RRF hybrid recall + KV store | 1 540 |
| Security audit | Static OWASP analysis | 1 075 |
| Embeddings | TF-IDF + OpenAI cosine similarity, FTS5 BM25 | 601 |
| **Total effective Rust LOC** | | **~59 000** |

### Project Structure

```
Metalogos-/
├── Cargo.toml                       # v0.19.0, workspace root
├── logo.jpg                          # Brand logo
├── README.md                         # This file
├── REFERENCE.md                      # Full builtin reference (~156 KB) — 100% of the registry (§6 index)
├── CHANGELOG.md                      # Version history (~177 KB)
├── FEATURE_INTAKE.md                 # Feature request tracking
├── MEMORY_ROADMAP.md                 # Memory system roadmap
├── Dockerfile                        # Docker build
├── index.html                        # Landing page / docs site
│
├── src/                              # Core compiler + interpreter (~59 000 LOC)
│   ├── main.rs                        # CLI: run/check/repl/compile/serve/eval/resume/test/audit
│   ├── grammar.pest                   # Pest PEG grammar (530 lines)
│   ├── ast.rs                         # AST definitions (29 Decl, 15 Expr, 12 Stmt) + Span tracking
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
│   └── builtins/                      # 395 built-in functions (37 modules)
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
│       └── lsp_integration.rs        # 7 integration tests
│
├── mlogpkg/                           # Package manager (workspace crate)
│   ├── Cargo.toml
│   ├── advisory-db.toml              # Local advisory DB (Naryad #198 Block 2)
│   ├── src/
│   │   └── main.rs                    # mlogpkg binary
│   └── tests/
│       ├── pkg_integration.rs         # original Phase 3.4 tests
│       ├── naryad_198_dependency_conflict.rs
│       ├── naryad_198_lockfile_determinism.rs
│       ├── naryad_198_audit_finds_known_vuln.rs
│       └── naryad_198_backward_compat.rs
│
├── tests/                             # 135 Rust test files
│   ├── fixtures/                      # PDF test fixtures
│   ├── golden.rs                      # Golden test runner
│   ├── vm_golden.rs                   # VM golden tests
│   ├── crosscheck_backends.rs          # TW vs VM parity (see ADR-0105 for known gaps)
│   ├── repl_integration.rs            # REPL tests
│   ├── definition_of_done.rs          # Project completeness validation
│   └── ...                            # and 130 more contract/feature test files
│
├── examples/                          # 213 .mlog programs (golden corpus)
│   ├── m1_hello.mlog                  # Hello World
│   ├── p6_full_app.mlog               # Full web app with routes
│   ├── p23_ml_learn.mlog              # ML learning
│   ├── dag_demo.mlog                  # DAG orchestration
│   ├── contracts/                     # Golden-file test contracts
│   └── debug/                          # Bug-reproduction files (not golden contracts)
│
├── std/                               # Standard library (4 .mlog files)
│   ├── string.mlog
│   ├── math.mlog
│   ├── collections.mlog
│   └── infographic.mlog
│
├── self-host/                         # Self-hosting experiments
│   ├── lexer.mlog                     # Lexer written in .mlog itself
│   ├── parser.mlog                    # Parser written in .mlog itself (Naryad #197)
│   └── std/                           # Copies of std/ for self-hosted execution
│
├── editors/vscode/                    # VS Code extension
│   ├── package.json
│   ├── language-configuration.json
│   └── syntaxes/mlog.tmLanguage.json  # TextMate syntax highlighting
│
├── assets/                            # Brand assets
│   └── qr_wallet.jpg                  # Crypto wallet QR code
│
├── benches/                           # Criterion benchmarks
│   └── core_benchmarks.rs
│
├── docs/
│   ├── book/                          # mdBook documentation (syntax, stdlib, tutorial)
│   └── adr/                           # 109 Architecture Decision Records (ADR-0001..0112)
│       ├── 0001-m1-architecture.md
│       ├── ...
│       └── 0111-ast-span-tracking.md
│
└── .github/workflows/                  # CI/CD
    ├── ci.yml                         # 15 blocking jobs (branch-freshness, fmt, clippy, test-lib, crosscheck, candle-tests, vision-tests, registry-arity-check, test-llm-cache-contract, minimal-build, test-integration, adr-check, module-size-guard, vscode-extension, cargo-audit)
    └── build.yml                      # Release build + artifact upload
```

---

## Key Features

### Security by Design

The compiler enforces structural security invariants — these are errors, not warnings:

```mlog
// SQL injection — non-literal query() is rejected by Category A
let user = query("SELECT * FROM users WHERE id = $1", [id])

// Secret leak — env() to respond()/print() is rejected by Category A
entity token: Secret = env("API_KEY")
respond(token)   // [SECRET_LEAK] via mlog audit / Category A checks
print(token)     // runtime error: print() refused: Secret values cannot be printed

// XSS via LLM — unsanitized LLM output to respond() is rejected by Category A
let reply = call_llm(prompt)
respond(reply)   // [HTML_INJECTION] — use render() or escape_html()

// Templates: render(Name, args...) substitutes {{ var }} with HTML-escaped values (naryad 115)
// {{{ var }}} skips escaping — trusted author code only, not caught by audit
```

### OWASP Top 10 Coverage

| OWASP Item | Mechanism | Boundary |
|---|---|---|
| A01 Broken Access Control | `requires=[role]` on routes, `require` assertions in patterns | — |
| A02 Cryptographic Failures | `Secret`/`Encrypted`/`Hash` opaque types; AES-256-GCM | `Secret` printable only in tests; `env()` to sink caught by Category A |
| A03 Injection | `Query` opaque type (parameterized only); `Html` type; template `{{ }}` auto-escapes | `{{{ raw }}}` skips escaping; interprocedural taint not tracked |
| A04 Insecure Design | Security-first language design; opaque types; static audit | See Known boundaries (section 2 above) |
| A05 Security Misconfiguration | CSP/HSTS/X-Frame-Options headers set by default | Developer can override headers |
| A06 Vulnerable Components | Minimal dependency tree; `cargo-audit` in CI; Dependabot | Transitive deps may have unpatched CVEs (see RUSTSEC ignores) |
| A07 Auth Failures | `hash_password`/`verify_password`; HMAC-SHA256 signed sessions | — |
| A08 Data Integrity Failures | CSRF double-submit on mutating routes; signed cookies | CSRF not needed for token-authenticated APIs |
| A09 Logging Failures | Audit log for `require` failures, `adapt` mutations, `unsafe_html` | — |
| A10 SSRF | LLM sandbox `forbidden: [network]`; outbound HTTP restricted | Sandbox is opt-in; `http_get`/`http_post` available outside sandbox |

### Two Execution Backends

- **Tree-walking interpreter** — full feature support, used for `mlog run` and `mlog serve`
- **Bytecode VM** — 46 instructions, stack-based, used for `mlog compile` + `mlog run file.mbc`
- **JIT** — experimental scaffold, not part of the build (see ADR-0073)

### 395 Built-in Functions

String ops, math, collections, type conversion, LLM/AI, HTTP, JSON, file I/O, KV store, session memory, encryption, authentication, HTTP server, templates, databases, Telegram/Discord bots, time/date/calendar, geolocation, weather, reminders, cron, goals, todos, memory tree, preferences, approval workflows, fuzzy matching, hashline editing, context compaction, budget awareness, replay logging, policy enforcement, PDF processing (classify, extract, OCR), typed semantic memory (FTS5 BM25 + cosine RRF), SMTP/IMAP email, CalDAV/CardDAV calendar and contacts, native SVG graphics, and more. See [REFERENCE.md](REFERENCE.md) for the full list.

### Span-Aware Error Messages (ADR-0111)

Every AST node carries source position (`start_line:start_col–end_line:end_col`).
Semantic errors include line numbers for easy debugging:

```mlog
entity User { name: String }
entity User { age: Int }   // line 2: duplicate entity type: User
```

Parser fills `Span` from `pest::Span` via `Span::from_pest()`;
`Expr::span()` and `Declaration::span()` provide uniform access.
Tuple-style variants (e.g. `StringLit(String)`) converted to struct
variants with named fields + `span` — zero change in execution logic.

### Cargo feature gates

Optional compile-time gates (`svg`, `chart`, `diagram`, `template`, `llm`, `server`;
default = all). Measured impact of building without the server stack: ~7% smaller
release binary — see [ADR-0104](docs/adr/0104-feature-gating-measured-impact.md).

### Native SVG/Graphics Subsystem

44 builtins, hand-rolled in pure Rust — zero external SVG/charting/
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
  `slide_16x9`, `social_og`, print A4), `svg_icon` (9 glyphs)
- **8 chart types** — bar, donut, line, scatter, area, heatmap, radar
  (multi-series), boxplot (real quartile math)
- **21 diagram types** — flowchart (topological layering, cycle
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

### Reflex — Local Neural Models (ADR-0112, ADR-0114, ADR-0117)

The Reflex pillar trains, predicts, persists, and distills local neural models — the LLM acts as a teacher, the local head as a student. Models are declared as first-class language constructs (`reflex Name { ... }`), opaque to `Value` (only a `ReflexId` handle enters the value system — weights never leak).

**Classification, not generation** — per ADR-0117 §3, `reflex` classifies into a closed-set label list (`labels: ["a", "b", ...]`). Free-form text generation is explicitly out of scope. This is the same boundary that applies to `reflex_seq` (sequence models) — symmetric ADR-0117 enforcement.

```mlog
// 1. Declare a classifier — input dim, dense layers, closed label set.
reflex SentimentClassifier {
  input: embedding(2)
  layers: [dense(8, relu), dense(2, softmax)]
  labels: ["positive", "negative"]
  seed: 42
}

// 2. Train on labeled data — returns Struct {loss, accuracy, metric, threshold_met}.
let result = reflex_train(SentimentClassifier, [
  [0.1, 0.2, 0.0],   // features + class_idx (last element)
  [0.8, 0.9, 1.0],
  // ... ≥10 samples for 80/20 holdout (ADR-0115)
], 200.0, "accuracy", 0.85)

// 3. Predict on new input — returns Fluid with label variants, sorted by confidence.
let prediction = reflex_predict(SentimentClassifier, [0.15, 0.25])
// prediction → Fluid{ "positive" (0.92), "negative" (0.08) }
```

**Distillation** — a `learnable pattern` can `distill_to` a reflex model: the LLM is called during the *teaching* phase, then the local head replaces it once confidence exceeds the `fallback_if` threshold.

```mlog
learnable pattern Classify(text: String) -> String {
  distill_to: SentimentClassifier
  fallback_if: confidence < 0.85
  call_llm(system_prompt, text)
}
```

**Persistence** — `reflex_save`/`reflex_load` serialize trained weights to the SQLite database configured by `memory { persist: "path.db" }` (ADR-0116). Shape mismatches between saved and current declarations are explicit errors, never silent corruption.

**Architecture blocks (ADR-0118, ADR-0119)** — `reflex_seq` declares sequence models for transformer-family layers: `attention` (multi-head with RoPE, Naryad #183), `rms_norm` / `swiglu` / `transformer_block` (Naryad #184). These require the optional `candle` feature (`cargo build --features candle`), not unconditional — when the feature is off, `reflex_seq` declarations produce a clean error naming the missing feature. Sequence models classify the *whole* sequence into one label (mean pooling + Dense head), not token-by-token generation (ADR-0117 §3 boundary, symmetric).

**Grouped-Query Attention (GQA, Naryad #188)** — `attention` accepts an optional third parameter for the number of KV heads:

```mlog
reflex_seq GqaModel {
  input: embedding(64)
  seq_len: 16
  layers: [attention(8, 64, 2)]   // 8 query heads, dim 64, 2 KV heads (GQA)
  labels: ["signal", "noise"]
  seed: 42
}
```

When the third parameter is omitted (`attention(8, 64)`), behaviour is identical to standard multi-head attention (Naryad #183) — `n_kv_heads` defaults to `n_heads`. When `n_kv_heads < n_heads`, K and V weights are smaller (`[dim, kv_dim]` instead of `[dim, dim]`) and repeated along the head axis during the attention computation (Llama 2/3 architecture). Constraints: `n_kv_heads > 0`, `n_kv_heads ≤ n_heads`, `n_heads % n_kv_heads == 0`.

**Stacked transformer blocks (Naryad #190)** — multiple `transformer_block` entries can be chained in the `layers` list. Each block gets its own independent, deterministically different weights (via `VarMap` prefixing — not identical copies):

```mlog
reflex_seq StackedTransformer {
  input: embedding(8)
  seq_len: 4
  layers: [
    transformer_block(2, 8, 16),
    transformer_block(2, 8, 16)   // different weights from block 0
  ]
  labels: ["a", "b"]
  seed: 42
}
```

Each layer receives `seed.wrapping_add(layer_index)` for deterministic weight initialization, and registers its parameters under unique `VarMap` names (`block0_attn_w_q`, `block1_attn_w_q`, etc.) so that `backward()` populates gradients for all blocks — gradients flow through the entire stack, not just the last layer.

### Vision — Generative Media (ADR-0122, ADR-0125)

**Weights run parked — no production PNG yet.** The Vision pillar (images, ADR-0122) is feature-gated (`--features vision`, which implies `candle`) and ships compiler-level provenance and supply-chain gates (ADR-0125); the real-weights run (runbook №237) has not been executed, so no production image has been generated.

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
| `test-lib` | **Blocking** | Unit + golden tests (564 pass) |
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
| **Self-hosted lexer** | Tokenizer written in .mlog itself (`self-host/lexer.mlog`) |
| **Self-hosted parser** | Parser written in .mlog itself (`self-host/parser.mlog`, Naryad #197) — parses a subset of the grammar sufficient for bootstrap (parses its own source). The full grammar remains the responsibility of the production Rust parser (`src/parser/`). |

---

## Self-hosted parser (Naryad #197)

`self-host/parser.mlog` is a Metalogos parser written in Metalogos itself. It
builds on top of the self-hosted lexer (`self-host/lexer.mlog`) — both files
are bootstrap-complete: each can process its own source.

### Supported subset (Block 1)

The first version supports the subset of the grammar required for bootstrap
(parsing parser.mlog itself). Constructs outside this subset are skipped via
the `SkipDecl` helper; the parser does NOT silently accept them as something
else, and a `// templ-decl` comment in the source marks which constructs are
intentionally excluded.

**Supported top-level declarations:**

| Declaration | Example |
|---|---|
| `pattern` | `pattern Foo(x: String) -> String { ... }` |
| `entity` (simple) | `entity greeting: String = "Hello"` |
| `flow` | `flow Main { input: Type = src -> Step1 -> output }` |
| `import` | `import std/string as str` |

**Supported statements:** `let`, `let mut`, assignment (`x = ...`), `if cond
{ ... } else if ... else { ... }`, `if cond then { ... }`, `if cond then X
else Y` (expression form), `while`, `each x in xs { ... }`, `each i, x in xs
{ ... }` (with index), `return`, bare expression statement, `break`,
`continue`.

**Supported expressions:** full precedence chain `or` / `and` / comparison
(`==`, `!=`, `<`, `>`, `<=`, `>=`) / additive / multiplicative, unary
minus, function call (`f(args)`), qualified call (`mod.fn(args)`), field
access (`obj.field`), index access (`arr[i]`), list literal (`[a, b, c]`),
struct literal (`{ k: v, ... }`), parenthesized expression, `if cond then
X else Y` expression form, and the literals STRING / NUMBER / INT / BOOL /
IDENT. Unary minus is desugared to `0.0 - X` to match the Rust AST's
representation (`src/parser/expr.rs`).

**Explicitly NOT supported** (deferred to a future naryad):

- All other top-level declarations: `entity Type { ... }` (record), `entity
  name: Type = { ... }` (instance), `rule`, `memorize`, `forget`, `relate`,
  `adapt`, `mutate`, `eval`, `test`, `type`, `llm`, `hook`, `sandbox`, `db`,
  `schema`, `skill_index`, `memory`, `conversation`, `context_budget`,
  `fluid`, `learnable pattern`, `tool`, `mlogserver`, `template`, `reflex`,
  `reflex_seq`, `reflex_gen`.
- `match` statement and `match` expression.
- `try` expression.
- Block `if/else` as an expression with side-effecting inner statements
  (Metalogos v0.19's scoping rule blocks mutations to outer `let mut`
  variables from inside a BlockIfElse expression; parser.mlog works around
  this by delegating to helper patterns — see `ParseElseBranch`,
  `ParseImportAlias`).

### Usage

```sh
# Parse a .mlog file (writes AST to stdout):
MLOG_PARSE_TARGET=path/to/file.mlog mlog run self-host/parser.mlog

# Bootstrap (parser.mlog parses itself):
MLOG_PARSE_TARGET=self-host/parser.mlog mlog run self-host/parser.mlog
```

### Contracts

Two integration tests verify the parser's correctness:

- `tests/naryad_197_parser_self_parses.rs` — bootstrap test: parser.mlog
  successfully parses its own source (~4 min runtime on a typical dev
  machine; the test has a 12-minute timeout).
- `tests/naryad_197_parser_matches_rust_parser.rs` — on a representative
  sample of 12 .mlog files covering the Block 1 subset, the AST produced by
  parser.mlog is structurally equivalent to the AST produced by the
  production Rust parser (`src/parser/`). The comparison normalises both
  sides to the same S-expr string format.

### Lexer bugs fixed in parser.mlog's local Tokenize copy

The self-hosted lexer (`self-host/lexer.mlog`) has several known issues
that prevented the parser from working directly off its output. The
parser's local `Tokenize` copy includes the following fixes (each
documented inline in `self-host/parser.mlog`):

1. **Whitespace handling**: the original lexer only recognized ASCII
   space, treating `\n`/`\t`/`\r` as quote chars. This produced phantom
   STRING tokens spanning multiple lines and swallowing entire
   declarations. Fix: treat all four whitespace chars as whitespace.
2. **Underscore in identifiers**: identifiers like `index_of` were split
   into `index`, `_`, `of` because the lexer's `abc` alphabet string
   omitted `_`. Fix: treat `_` as a letter, and allow it (plus digits)
   in identifier continuation.
3. **Multi-char operators**: `==`, `!=`, `<=`, `>=` were emitted as two
   single-char OPERATOR tokens, breaking comparison parsing. Fix: detect
   these as 2-char operators (mirrors the existing `->` handling).
4. **Line comments**: `// ...` comments were tokenized as code. Fix:
   detect `//` and skip to end of line.

The KEYWORD-classification bug (`index_of(kws, tok) > -1.0` does substring
matching) is NOT fixed at the lexer level — instead, the parser's `TokKind`
helper re-verifies KEYWORD/IDENT classification via exact-match against the
keyword list. This keeps the keyword list in one place (TokKind) rather
than duplicating it across multiple lexer-level checks.

---

## mlogpkg: dependency resolution + security audit (Naryad #198)

`mlogpkg` is the package manager for METALOGOS projects. As of Naryad #198 it
gains three new capabilities on top of the original `init`/`add`/`build`/`info`
commands:

1. **Full transitive dependency graph resolution** — `mlogpkg build` now
   walks the full dependency tree (not just direct deps), detecting
   version conflicts and cycles.
2. **`mlogpkg.lock` lockfile** — fixes the exact version of every
   dependency (direct + transitive) for reproducible builds. Same
   `mlog.toml` → same `mlogpkg.lock` byte-for-byte.
3. **`mlogpkg audit` command** — checks dependencies against a local
   advisory database of known vulnerabilities (Naryad #198 Block 2).

### Commands

```sh
mlogpkg init [--name NAME]     # create mlog.toml in current dir
mlogpkg add <pkg> [version]    # add a dependency (pre-flight resolves graph)
mlogpkg build                  # resolve full graph, write mlogpkg.lock, check sources
mlogpkg info                   # show project + lockfile status
mlogpkg audit                  # check deps against advisory DB
```

### mlogpkg.lock format

```toml
# This file is automatically generated by mlogpkg.
# Do not edit manually — run `mlogpkg build` to regenerate.
version = 1

[[package]]
name = "alpha"
version = "1.0.0"
source = "registry"

[[package]]
name = "beta"
version = "2.0.0"
source = "registry"
```

Packages are sorted alphabetically by name for determinism. The file is
TOML (consistent with `mlog.toml`), inspired by `Cargo.lock` and
`package-lock.json`.

### Version conflict detection

mlogpkg v1 does NOT support multiple concurrent versions of the same
package. If two dependencies require different versions of a common
transitive dep, `mlogpkg build` and `mlogpkg add` fail with an explicit
error:

```
error: dependency resolution failed: version conflict for 'shared':
'pkg_b' requires version '2.0.0', but 'pkg_a' already requires version '1.0.0'
(mlogpkg v1 does not support multiple concurrent versions of the same package)
```

This is a deliberate simplification — supporting concurrent versions
(cargo's "semver resolution") is a significantly more complex feature and
out of scope for v1.

### `mlogpkg audit` — limitation (Block 2)

⚠️ **The advisory database is LOCAL and MANUALLY MAINTAINED.** It is NOT
an integration with an external CVE database (RUSTSEC, NVD, GitHub Advisory
DB, etc.).

- The bundled DB lives at `mlogpkg/advisory-db.toml`.
- It is updated only when a new release of `mlogpkg` is published.
- Vulnerabilities are added as they are discovered and reported to the
  Metalogos team.
- Version matching is exact (no semver ranges in v1).

To override the bundled DB:

1. Set the `MLOGPKG_ADVISORY_DB` env var to point at a custom `.toml`
   file. Useful for tests and for projects that want to extend the
   bundled DB with project-specific advisories.
2. Or create `~/.mlog/advisory-db.toml` (user-level override).

For real-world security auditing, supplement `mlogpkg audit` with
external tools (e.g. `cargo audit` for Rust dependencies, OS-level
scanners for system packages).

### Registry

The local registry is at `~/.mlog/registry/<pkg-name>/`. Each package is
a directory containing:

- `mlog.toml` — package manifest (name, version, dependencies)
- `src/main.mlog` — source files

Override the registry path with the `MLOGPKG_REGISTRY` env var (useful
for tests).

### Contracts (Naryad #198)

Four integration tests verify the new behavior:

- `tests/naryad_198_dependency_conflict.rs` — two packages requiring
  incompatible versions of a common transitive dep → explicit error at
  `build` and `add` time.
- `tests/naryad_198_lockfile_determinism.rs` — same `mlog.toml` → same
  `mlogpkg.lock` byte-for-byte, in two separate dirs and on rebuild.
- `tests/naryad_198_audit_finds_known_vuln.rs` — `mlogpkg audit` finds
  and reports a known vulnerability from the advisory DB; passes when no
  vuln matches; respects version specificity.
- `tests/naryad_198_backward_compat.rs` — simple projects without
  transitive deps work exactly as before, plus the appearance of
  `mlogpkg.lock`.

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
| Effective Rust LOC | ~59 000 |
| Built-in Functions | 373 (37 modules) |
| Example Programs | 213 |
| Integration Tests | 70 test suites |
| Architecture Decision Records | 118 |
| Parser Rules | 288 (Pest PEG) |
| VM Instructions | 46 |
| Execution Backends | 2 (interpreter + bytecode VM) |
| Workspace Crates | 3 (mlog, mlog-lsp, mlogpkg) |
| Commits | See [GitHub](https://github.com/ShkodnikAI/Metalogos-/commits/main) — live value |
| License | MIT / Apache-2.0 |

---

## Development Process

Metalogos was built through iterative, verification-first development:
every feature started as a minimal example with an expected output,
followed by the smallest implementation that made it pass, followed by
review before merge. Architectural decisions are recorded as ADRs
(108 as of this writing, in `docs/adr/`) rather than left implicit —
including several honest "Rejected" decisions where a proposed feature
was deliberately not built, with the reasoning kept on record. The full
commit history and every ADR remain in this repository; detailed
internal work-order logs are not published separately.

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

All 8 milestones and 8+ phases complete, plus a full native SVG/graphics subsystem (naryads №77-92). 122+ development narads (work orders) delivered. 395 builtins, 135 test files, 213 example programs, 126 ADRs. See [GitHub](https://github.com/ShkodnikAI/Metalogos-/commits/main) for live commit count.

### Next

| Target | Description |
|---|---|
| **Phase 9** | Self-hosted compiler (lexer in `self-host/lexer.mlog`, parser in `self-host/parser.mlog` — Naryad #197, subset complete), mlogpkg ecosystem (Naryad #198: full dep graph + lockfile + local audit), production deployment |

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

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.

---

*Built with Rust. Designed by AI. For AI.*
