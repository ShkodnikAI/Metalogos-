# Metalogos (mlog)

> An AI-native programming language with security enforced at the compiler level. Built in Rust.

[![GitHub stars](https://img.shields.io/github/stars/ShkodnikAI/Metalogos-?style=social)](https://github.com/ShkodnikAI/Metalogos-)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/blob/main/LICENSE-MIT)
[![Rust](https://img.shields.io/badge/Rust-1.79%2B-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/ShkodnikAI/Metalogos-?style=flat-square)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/ShkodnikAI/Metalogos-/ci.yml?style=flat-square&label=CI)](https://github.com/ShkodnikAI/Metalogos-/actions)

[📖 Documentation](https://github.com/ShkodnikAI/Metalogos-/tree/main/docs) · [🚀 Quick Start](#quick-start) · [📦 Releases](https://github.com/ShkodnikAI/Metalogos-/releases) · [💬 Discussions](https://github.com/ShkodnikAI/Metalogos-/discussions)

---

## What is Metalogos?

Metalogos is a programming language where **security is not a library — it is the language itself**.

- **XSS, SQL injection, secret leakage** — syntactically impossible, not just caught by linters
- **AI operations** (LLM calls, memory, learning) — first-class language constructs, not bolt-on frameworks
- **Built in Rust** — memory-safe, zero-cost abstractions, fearless concurrency
- **Production-ready** — 27K LOC, 246 built-ins, 2 execution backends, 78 ADRs

```mlog
// LLM call as a natural language construct
learnable pattern Classify(msg: String) -> String {
  prompt: "Classify as: question | complaint | greeting | urgent"
}

let result = Classify("where is my order?")
// Returns: "question"
```

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🛡️ **Security by Design** | XSS, SQLi, secret leakage, broken access control — impossible at compile time |
| 🤖 **AI-Native** | LLM calls, memory, learning, adaptation — first-class language primitives |
| 🔒 **Opaque Types** | `Secret`, `Encrypted`, `Html` — enforced by the type system, not conventions |
| 🧠 **Semantic Memory** | Hierarchical memory with FTS5 BM25 + cosine RRF hybrid recall |
| 🔄 **Backward Iteration** | `Adapt` primitive — reversible self-modification with sandbox and rollback |
| ⚡ **Multiple Backends** | Tree-walking interpreter + Bytecode VM (44 instructions) + JIT scaffold |
| 🔍 **Built-in Audit** | `mlog audit` — static OWASP analysis before deployment |
| 🦀 **Rust Foundation** | ~27,000 lines of Rust, MIT/Apache-2.0, fully open source |

---

## 🚀 Quick Start

### Install

```bash
# From crates.io (when published)
cargo install metalogos

# Or download pre-built binary from releases
curl -L https://github.com/ShkodnikAI/Metalogos-/releases/latest/download/metalogos-linux-x86_64.tar.gz | tar xz

# Or build from source
git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo build --release
```

### Run Your First Program

```bash
# Run an example
mlog run examples/hello.mlog

# Check security before deployment
mlog audit examples/web_server.mlog

# Start the HTTP server
mlog serve examples/api.mlog
```

### Write a Secure Program

```mlog
// This program is secure by construction
entity apiKey: Secret = env("API_KEY")

route "/data" method=GET requires=[authenticated] {
  let users = query("SELECT * FROM users WHERE id = $1", [id])
  response.json(users)
}

// Compile-time guarantees:
// ✅ apiKey cannot be printed or serialized
// ✅ Only parameterized queries compile
// ✅ Route requires authentication at declaration
```

---

## 📦 Installation

### Prerequisites

- **Rust** 1.79+ ([install](https://rustup.rs))
- **Linux** (pre-built binaries available)
- **macOS / Windows** — build from source

### From Binary (Linux x86_64)

```bash
curl -LO https://github.com/ShkodnikAI/Metalogos-/releases/latest/download/metalogos-linux-x86_64.tar.gz
tar xzf metalogos-linux-x86_64.tar.gz
sudo mv metalogos /usr/local/bin/
```

### From Source

```bash
git clone https://github.com/ShkodnikAI/Metalogos-.git
cd Metalogos-
cargo build --release
# Binary: target/release/metalogos
```

---

## 🛡️ Security Guarantees

Metalogos makes entire classes of vulnerabilities **unexpressible**:

| Vulnerability | Traditional Fix | Metalogos Approach |
|-------------|-----------------|------------------|
| XSS | Sanitize output | `Html` type is opaque — strings cannot reach browser |
| SQL Injection | Use ORM / prepared statements | Only parameterized queries compile |
| Secret Leakage | Scan for hardcoded keys | `Secret` type blocks `print()`, `to_string()`, serialization |
| Broken Access Control | Middleware checks | Routes declare roles at definition; violations are compile errors |
| OWASP Top 10 | SAST/DAST tools | All 10 categories addressed at the language level |

```bash
# Run the security audit
mlog audit your_program.mlog

# Output:
# ✅ No XSS vulnerabilities detected
# ✅ No SQL injection vectors
# ✅ All secrets properly typed
# ✅ All routes have access control
# ✅ OWASP Top 10: 10/10 covered
```

---

## 🤖 AI-Native Programming

```mlog
// Memory Tree: hierarchical knowledge with automatic compression
mtree_store("User prefers email over chat")
mtree_store("Project deadline is July 15")
mtree_summarize()  // L0 → L1 chunking

// Cron scheduler
cron_run("0 9 * * 1-5", "MorningReport")

// Goals and Todos
goal_set("launch v1.0", "2026-08-01")
todo_add("Fix cron edge case", "high")
```

---

## 🏗️ Architecture

```
.mlog source → Pest PEG Parser (180 rules) → AST (24 Decl, 14 Expr, 12 Stmt)
    → Semantic Analysis (opaque types, arity, security audit)
    → Two Backends:
        ├── Tree-walking Interpreter (full features, 4,395 LOC)
        └── Bytecode VM (44 instructions, stack-based, 1,268 LOC)
```

**Key Metrics:**
- ~27,000 lines of Rust
- 246 built-in functions (14 modules)
- 92 example programs with golden-file tests
- 35 integration test suites
- 78 Architecture Decision Records
- Version 0.12.0

---

## 📁 Repository Structure

```
Metalogos-
├── src/                    # Core implementation
│   ├── parser/             # Pest PEG grammar
│   ├── ast/                # AST definitions
│   ├── semantic/           # Type checker & security audit
│   ├── interpreter/        # Tree-walking backend
│   ├── vm/                 # Bytecode compiler & VM
│   └── builtins/           # 246 built-in functions
├── examples/               # 92 .mlog example programs
├── tests/                  # 35 integration test files
├── docs/                   # Documentation & 78 ADRs
│   └── adr/                # Architecture Decision Records
└── .github/                # CI/CD, issue templates
```

---

## 🤝 Contributing

We welcome contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

- 🐛 [Report a bug](https://github.com/ShkodnikAI/Metalogos-/issues/new?template=bug_report.md)
- 💡 [Request a feature](https://github.com/ShkodnikAI/Metalogos-/issues/new?template=feature_request.md)
- 🔒 [Security issues](SECURITY.md)

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE).

---

## 💖 Support

Metalogos is developed by a solo developer. If you find it useful, consider supporting the project:

[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

- **[Open Collective](https://opencollective.com/metalogos)** — Transparent funding
- **USDT (TRC-20)** — Direct crypto support: scan QR or use wallet address

<img src="https://raw.githubusercontent.com/ShkodnikAI/metalogos-grants/main/media/qr_usdt_wallet.jpg" width="120" alt="USDT QR Code">

*Thank you to all our backers!*
