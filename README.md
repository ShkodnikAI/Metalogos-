# Metalogos (mlog)

> AI-native programming language with security by design. Built in Rust.

[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

---

## 💖 Support Metalogos

Metalogos is developed by a solo developer. Your support helps keep the project alive and growing.

### Open Collective (Primary)

Transparent funding for the project. See exactly where every dollar goes.

[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

[opencollective.com/metalogos](https://opencollective.com/metalogos)

### Cryptocurrency (USDT TRC-20)

Direct support from anywhere in the world, regardless of banking restrictions:

| Scan to donate | Wallet Address |
|----------------|----------------|
| <img src="https://raw.githubusercontent.com/ShkodnikAI/metalogos-grants/main/media/qr_usdt_wallet.jpg" width="150"> | `USDT (TRC-20): [YOUR_WALLET_ADDRESS]` |

**Network:** TRC-20 (Tron)

### Sponsorship Tiers

| Tier | Open Collective | Crypto (USDT) | Benefits |
|------|----------------|---------------|----------|
| ☕ Supporter | $5/mo | 50 USDT/mo | Name in SPONSORS.md, early releases, Discord |
| 🚀 Contributor | $25/mo | 250 USDT/mo | + Vote on features, beta access, monthly updates |
| 🏆 Sponsor | $100/mo | 1000 USDT/mo | + Logo on website, 1hr consulting, priority support |
| 🏢 Corporate | $500/mo | 5000 USDT/mo | + Enterprise support, private registry, roadmap calls |

*Thank you to all our backers!*

---

## 🚀 Quick Start

```bash
# Install
cargo install metalogos

# Run a program
mlog run examples/hello.mlog

# Security audit
mlog audit myapp.mlog

# Start HTTP server
mlog serve app.mlog
```

---

## 🛡️ Security by Design

Metalogos makes entire classes of vulnerabilities **syntactically impossible**:

- **XSS** → `Html` type is opaque; strings cannot reach the browser unescaped
- **SQL Injection** → Only parameterized queries compile
- **Secret Leakage** → `Secret` type blocks `print()`, `to_string()`, serialization
- **Broken Access Control** → Routes declare roles at definition; violations are compile-time errors
- **OWASP Top 10** → All 10 categories covered at the language level

---

## 🤖 AI-Native

- LLM calls are **first-class language constructs** — no SDK imports, no boilerplate
- Hierarchical semantic memory with hybrid recall (FTS5 BM25 + cosine RRF)
- Sandboxed execution with automatic audit logging
- Every AI decision is logged, inspectable, and reversible

---

## 📊 Project Metrics

| Metric | Value |
|--------|-------|
| Lines of Code | ~27,000 (Rust) |
| Built-in Functions | 246 (14 modules) |
| Example Programs | 92 |
| Integration Tests | 35 test suites |
| Architecture Decision Records | 78 |
| Execution Backends | 2 (interpreter + bytecode VM) |
| License | MIT / Apache-2.0 |

---

## 📁 Repository Structure

```
Metalogos-
├── src/              # Core language implementation
├── examples/         # 92 example programs
├── tests/            # Integration tests
├── docs/             # Documentation and ADRs
│   └── adr/          # 78 Architecture Decision Records
└── .github/          # CI/CD, issue templates, funding
```

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE).

---

*Metalogos — The future of programming is secure by design.*
