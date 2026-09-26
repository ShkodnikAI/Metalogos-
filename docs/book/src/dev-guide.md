# Development Guide — CI/CD, Ecosystem, Technology Stack

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
