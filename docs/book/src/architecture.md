# Architecture

## Architecture

```
 .mlog source        Pest PEG          AST                Semantic            TW + VM backends
─────────────  ──>  ────────────  ──>  ───────────  ──>  ────────────  ──>  ────────────
 entity             parse tokens      33 Declaration    cross-reference     tree-walking
 pattern            syntax rules      14 Expr           validation          bytecode VM
 flow                                  15 Statement      opaque type       enforcement
 memory                                 4 MatchArm        span-aware
 rule                                                    error messages
 learn
 adapt
 reflex
```

### Implementation Stack

| Component | Technology | Lines |
|---|---|---|
| Parser | Pest 2.7 PEG grammar (~595 lines, 328 rules) | 2 176 |
| AST | 33 Declaration variants, 14 Expr, 15 Statement, 4 MatchArm, span tracking (ADR-0111) | 1 289 |
| Semantic analysis | Opaque types, arity checking, Category A audit (SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION, VISION_UNSIGNED_EXPORT, MODEL_WEIGHTS_UNSAFE), SVG XSS lint | 473 |
| Compiler | Bytecode, 500 builtins indexed | 2 743 |
| Bytecode format | 47 VM instructions | — |
| Tree-walking interpreter | Full feature support, 12 modules | ~4 400 |
| VM | Stack-based bytecode executor | 2 143 |
| Built-in functions | 500 functions across 45 modules | ~18 300 |
| HTTP server | Axum 0.8 + Tokio, security middleware | 2 433 |
| LLM backend | Trait + mock + real providers | 1 421 |
| Memory store | Typed memory with FTS5 BM25 + cosine RRF hybrid recall + KV store | 1 540 |
| Security audit | Static OWASP analysis | 1 075 |
| Embeddings | TF-IDF + OpenAI cosine similarity, FTS5 BM25 | 601 |
| **Total effective Rust LOC** | | **~59 000** |

### Project Structure

```
Metalogos-/
├── Cargo.toml                       # v0.25.0, workspace root
├── logo.jpg                          # Brand logo
├── README.md                         # This file
├── AGENTS.md                         # Canonical methodology file for agent tools (industry-standard AGENTS.md spec — superseded AGENT.md)
├── CLAUDE.md                         # Bridge copy of AGENTS.md for Claude-compatible tools (synced manually — see issue #299)
├── GEMINI.md                         # Bridge copy of AGENTS.md for Gemini-compatible tools (synced manually — see issue #299)
├── REFERENCE.md                      # Full builtin reference (~282 KB) — 100% of the registry (§6 index + №316 classification)
├── CHANGELOG.md                      # Version history (~451 KB)
├── AI_USAGE.md                       # Disclosure: how generative AI is used in this project's development
├── FEATURE_INTAKE.md                 # Feature request tracking
├── MEMORY_ROADMAP.md                 # Memory system roadmap
├── Dockerfile                        # Docker build
├── index.html                        # Landing page / docs site
├── llms.txt                          # LLMs.txt v2 index for agent tools & RAG pipelines (issue #300)
│
├── src/                              # Core compiler + interpreter (~93 000 LOC)
│   ├── main.rs                        # CLI: run/check/repl/compile/serve/mcp-serve/eval/resume/test/audit
│   ├── grammar.pest                   # Pest PEG grammar (534 lines)
│   ├── ast.rs                         # AST definitions (29 Decl, 15 Expr, 12 Stmt) + Span tracking
│   ├── semantic.rs                    # Semantic analysis + opaque type enforcement
│   ├── compiler.rs                    # Bytecode compiler
│   ├── bytecode.rs                    # VM instruction set (46 instructions)
│   ├── vm.rs                          # Bytecode VM executor
│   ├── server.rs                      # Axum HTTP server + cron scheduler
│   ├── llm.rs                         # LLM backend trait + providers + streaming
│   ├── mcp_server.rs                  # MCP server (stdio JSON-RPC, exposes tool constructs)
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
│   ├── vision/                       # Vision pillar (feature-gated: images, ADR-0122)
│   ├── voice/                        # Voice pillar (feature-gated: speech, ADR-0143)
│   ├── video/                        # Video pillar (feature-gated: video, ADR-0147)
│   │
│   └── builtins/                      # 500 built-in functions (45 modules)
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
├── tests/                             # 228 Rust test files
│   ├── fixtures/                      # PDF test fixtures
│   ├── golden.rs                      # Golden test runner
│   ├── vm_golden.rs                   # VM golden tests
│   ├── crosscheck_backends.rs          # TW vs VM parity (see ADR-0105 for known gaps)
│   ├── repl_integration.rs            # REPL tests
│   ├── definition_of_done.rs          # Project completeness validation
│   └── ...                            # and 223 more contract/feature test files
│
├── examples/                          # 258 .mlog programs (golden corpus)
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
    ├── ci.yml                         # 19 blocking jobs (branch-freshness, fmt, clippy, test-lib, crosscheck, candle-tests, vision-tests, voice-tests, video-tests, doc-tests, registry-arity-check, test-llm-cache-contract, minimal-build, test-integration, ledger-golden, adr-check, module-size-guard, vscode-extension, cargo-audit)
    └── build.yml                      # Release build + artifact upload
```

---
