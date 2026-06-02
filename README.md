<div align="center">

# Metalogos

**The first programming language designed by AI, for AI. Security built into the language.**

[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.4.0-blue.svg)](https://github.com/ShkodnikAI/Metalogos-)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)

</div>

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

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo install --path .

# Run a program
mlog run examples/m1_hello.mlog
# Output: HELLO, METALOGOS!!

# Interactive REPL
mlog repl

# Semantic check (no execution)
mlog check examples/p6_full_app.mlog

# Serve a web application
mlog serve examples/p6_full_app.mlog
# Listening on 0.0.0.0:8080
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

All examples have golden-file tests (`.expected` files). Run them with `cargo test`.

---

## Architecture

```
 .mlog source        Pest PEG          AST              Semantic            Interpreter
─────────────  ──▶  ────────────  ──▶  ───────────  ──▶  ────────────  ──▶  ────────────
 entity             parse tokens      17 declaration    cross-reference     tree-walking
 pattern            syntax rules     Expr variants     validation          evaluation
 flow                                  Statement         opaque type       built-in fns
 memory                                 Value            enforcement        effects
 rule                                                             
 learn                                                         
 adapt
```

**Implementation stack:**

| Component | Technology |
|---|---|
| Parser | [Pest 2.7](https://pest.rs/) — PEG grammar |
| AST / Interpreter | Hand-written Rust (~5 000 lines) |
| Web server | [Axum 0.8](https://github.com/tokio-rs/axum) + [Tokio](https://tokio.rs/) |
| Crypto | `hmac`, `sha2`, `aes-gcm` (AES-256-GCM) |
| CLI | [Clap 4.5](https://github.com/clap-rs/clap) |
| Tests | Golden-file (`examples/*.expected`) + unit + integration |

```
Metalogos-/
├── Cargo.toml              # Single-crate project
├── src/
│   ├── grammar.pest         # PEG grammar (~200 lines)
│   ├── ast.rs               # AST definitions
│   ├── parser.rs            # Pest tokens → AST
│   ├── semantic.rs          # Semantic analysis (opaque types, references)
│   ├── interpreter.rs       # Tree-walking interpreter
│   ├── builtins.rs          # 40+ built-in functions
│   ├── server.rs            # Axum HTTP server + security middleware
│   ├── llm.rs               # LLM backend trait + mock
│   └── main.rs              # CLI: run / repl / check / serve
├── examples/                # 36 .mlog programs with golden tests
├── tests/                   # Golden-file runner, integration tests
└── docs/adr/                # 26 Architecture Decision Records
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
| Phase 5 | `let`/`if`/`each`/`while`, List type, Modules | Done |
| Phase 6.1–6.2 | HTTP server + Type-safe HTML templates | Done |
| Phase 6.3 | Parameterized database queries | Done |
| Phase 6.4 | Encryption: Secret / Encrypted / Hash | Done |
| Phase 6.5 | Authentication: sessions + CSRF + roles | Done |
| Phase 6.6 | Bot integration: Telegram + Discord webhooks | Done |
| Phase 6.7 | OWASP Top 10 validation + full web app | Done |

### Next

| Phase | Target |
|---|---|
| Phase 3 | LSP, `mlogpkg` package manager, mdbook docs |
| Phase 4 | Bytecode VM / JIT, self-hosted compiler |
| Phase 7 | Production LLM backend (OpenAI / Anthropic), real database (SQLite/Postgres) |

---

## Prior Art

Metalogos stands on the shoulders of proven systems:

- **Rust** — ownership model, opaque types, zero-cost abstractions
- **Haskell** — type-safe HTML (Yesod/Blaze), `newtype` for secrets
- **Pest** — elegant PEG parser generator
- **Axum** — ergonomic async HTTP
- **Datalog / CLIPS** — declarative rule engines with priority
- **ACT-R** — memory activation and decay models
- **DSPy** — programmatic LLM orchestration

---

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE-2.0) at your option.

---

*Built with Rust. Designed by AI. For AI.*
