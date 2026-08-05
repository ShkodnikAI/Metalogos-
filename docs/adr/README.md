# ADR Index — Architecture Decision Records

## Numbering rule

Before creating a new ADR, check for duplicates:

```bash
ls docs/adr/ | sed 's/-.*//' | sort | uniq -d
```

Must be empty. If not empty, resolve collisions before proceeding.

Numbers are assigned sequentially. The current maximum is in `0094-*`.

## Reserved numbers (do not reassign)

The following ADR numbers are referenced in source code, tests, or past Наряд
reports and must not be changed:

- **ADR-0073** (`jit-experimental`) — referenced in `README.md`, `CHANGELOG.md`,
  `tests/crosscheck_backends.rs`
- **ADR-0075** (`tw-vm-divergence`) — referenced in `CHANGELOG.md`,
  `tests/crosscheck_backends.rs`, past Наряд reports
- **ADR-0076** (`vm-dispatch-paths`) — referenced in `tests/vm_golden.rs`

## Index

| # | Title | Status |
|---|-------|--------|
| 0001 | M1 Architecture | accepted |
| 0002 | Rule Engine | accepted |
| 0003 | Learnable Semantics | accepted |
| 0004 | Memory | accepted |
| 0005 | Adapt | accepted |
| 0006 | Fluid Types | accepted |
| 0007 | Confidence Propagation | accepted |
| 0008 | Entity Store | accepted |
| 0009 | Semantic Analysis | accepted |
| 0010 | Codegen IR | accepted |
| 0011 | Type Inference | accepted |
| 0012 | Vector Recall | accepted |
| 0013 | ML Backend | accepted |
| 0014 | Knowledge Graph | accepted |
| 0015 | Full Adapt | accepted |
| 0016 | CLI + REPL | accepted |
| 0017 | Stdlib | accepted |
| 0018 | LSP | accepted |
| 0019 | Mlogpkg | accepted |
| 0020 | Bytecode VM | accepted |
| 0021 | VM Full Coverage | accepted |
| 0022 | JIT | accepted |
| 0023 | Self-Hosting | accepted |
| 0024 | Let / If / Else | accepted |
| 0025 | Iteration | accepted |
| 0026 | String Ops | accepted |
| 0027 | Modules | accepted |
| 0028 | HTTP Server (original) | superseded by 0074 |
| 0029 | Type-Safe HTML | accepted |
| 0030 | Parameterized Queries | accepted |
| 0031 | Encryption Opaque Types | accepted |
| 0032 | Session Management | accepted |
| 0033 | CSRF Protection | accepted |
| 0034 | Auth | accepted |
| 0035 | Bot Integration | accepted |
| 0036 | OWASP Validation | accepted |
| 0037 | Real LLM Backend | accepted |
| 0038 | Real Encryption | accepted |
| 0039 | Real Auth | accepted |
| 0040 | Real Embeddings | accepted |
| 0041 | Memory Persistence | accepted |
| 0042 | Real Sandbox | accepted |
| 0043 | Unicode Fix | accepted |
| 0044 | Route Pattern Invocation | accepted |
| 0045 | Hooks | accepted |
| 0046 | Context Loading | accepted |
| 0047 | LLM Cache | accepted |
| 0048 | Smart LLM Routing | accepted |
| 0049 | Session Memory | accepted |
| 0050 | Eval Harness | accepted |
| 0051 | Inspect | accepted |
| 0052 | Event Stream | accepted |
| 0053 | Conversation State | accepted |
| 0054 | Tool Abstraction | accepted |
| 0056 | Lifecycle Control | accepted |
| 0057 | Security Audit | accepted |
| 0058 | Tiered Skill Index | accepted |
| 0059 | Struct Entity Reuse | accepted |
| 0060 | Schema as Code | accepted |
| 0061 | Webhook Routing Diagnosis | accepted |
| 0062 | AgentSkillOS Recipe DAG | accepted |
| 0063 | OpenPlanter Agent Utilities | accepted |
| 0064 | Obsidian Mind Lifecycle Hooks | accepted |
| 0065 | Config Load YAML | accepted |
| 0066 | Stable Graph for Memory | accepted |
| 0067 | Blocking IO in Async Handlers | accepted |
| 0068 | DB Execute Parameter Binding | accepted |
| 0069 | Slice Builtin for Lists | accepted |
| 0070 | Parser Returns Result | accepted |
| 0071 | Integration Test Triage Block3 | accepted |
| 0072 | BUILTIN_REGISTRY / Dispatcher Sync | accepted |
| 0073 | JIT Backend — Experimental | accepted |
| 0074 | HTTP Server — Axum | accepted |
| 0075 | TW vs VM Divergence List | accepted |
| 0076 | VM Dispatch Path Coverage | accepted |
| 0077 | Model Routing | accepted |
| 0078 | Runtime Fixes | accepted |
| 0079 | Secret Leak HTTP POST Positional | accepted |
| 0080 | Module Size Policy | accepted |
| 0081 | VM-for-Serve Feasibility | assessed |
| 0082 | Language Completeness | accepted |
| 0083 | Unary Minus Fix | accepted |
| 0084 | Taint Assignment Propagation | accepted |
| 0085 | Type-Safe HTML Templates | accepted |
| 0086 | Performance Baseline Benchmarks | accepted |
| 0087 | Full UTF-8 Audit | accepted |
| 0088 | VM Serve Backend | accepted |
| 0089 | Confidence Semantics — Actual State | accepted |
| 0090 | Rule Priority Semantics — First-Wins | accepted |
| 0093 | Memory Typology & FTS5 Hybrid Search | accepted |
| 0094 | Type-Aware Recall & Hybrid Search (Phase 3) | accepted |
