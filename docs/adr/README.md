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
| 0001 | M1 Architecture | Accepted |
| 0002 | M2 — Rule Engine, Confidence, Flow Branching | Accepted |
| 0003 | M3 — Learnable Patterns and LLM Integration | Accepted |
| 0004 | M4 — Memory: Memorize, Recall, Forget, Decay | Accepted |
| 0005 | M5 — Adapt: Few-Shot Self-Modification | Accepted |
| 0006 | Phase 1 — Fluid Types: Lazy Collapse of Type Superpositions | Accepted |
| 0007 | Confidence Propagation through Patterns | Implemented |
| 0008 | Entity Store | Implemented |
| 0009 | Semantic Analysis | Implemented |
| 0010 | Codegen and IR (Intermediate Representation) | Implemented (Phase 1 |
| 0011 | Type Inference, Error Recovery, and Branch Overlap Detection | Partially Implemented |
| 0012 | Vector Recall (Embedding-Based Semantic Similarity) | Implemented (Phase 2.2 |
| 0013 | ML Backend (PyO3 Bridge for Fine-Tuning Learnable Patterns) | Implemented (Phase 2.3 |
| 0014 | Knowledge Graph (Semantic Memory via Graph Structure) | Implemented (Phase 2 Final) |
| 0015 | Full Adapt (Mutate + Sandbox + Rollback) | Implemented (Phase 2 Final) |
| 0016 | CLI + REPL + Semantic Check (Phase 3) | Accepted |
| 0017 | Standard Library (stdlib) and Import Mechanism | Accepted |
| 0018 | LSP Server (Phase 3.3) | accepted |
| 0019 | Package Manager — mlogpkg (Phase 3.4) | accepted |
| 0020 | Bytecode VM for METALOGOS | Accepted |
| 0021 | VM Full Feature Coverage — Phase 4.2 | Accepted |
| 0022 | JIT Compilation via Cranelift — Phase 4.3 | Accepted |
| 0023 | Self-Hosting: First Lexer Component (Phase 4.4) | Accepted (implementation pending |
| 0024 | let bindings + if/else expressions | Accepted |
| 0025 | Циклы: `each` (data-first) + `while` (fallback) | Принято |
| 0026 | String Operations as Builtins | Accepted |
| 0027 | Module System with Namespaces | Accepted |
| 0028 | HTTP Server with Axum | Accepted |
| 0029 | Type-Safe HTML via Opaque Html Type | Accepted |
| 0030 | Parameterized Queries via Opaque Query Type | Accepted |
| 0031 | Secret, Encrypted, Hash Opaque Types | Accepted |
| 0032 | Session Management with HMAC-SHA256 Signed Cookies | Accepted |
| 0033 | CSRF Protection via Double-Submit Cookie Pattern | Accepted |
| 0034 | Role-Based Access Control (RBAC) | Accepted |
| 0035 | Bot Integration via Webhook Routes | Accepted |
| 0036 | OWASP Top 10 Compliance via Language-Level Security | Accepted |
| 0037 | Real LLM Backend | Accepted |
| 0038 | Real Encryption (Phase 7.3) | Accepted |
| 0039 | Real Sessions, CSRF, and Rate Limiting (Phase 7.4) | Accepted |
| 0040 | Real Embeddings and Vector Recall (Phase 7.2) | Accepted |
| 0041 | Memory Persistence via SQLite (Phase 7.6) | Accepted |
| 0042 | Real Sandbox Enforcement | Accepted |
| 0043 | Unicode Fix — Cyrillic String Handling | Implemented |
| 0044 | Route Pattern Invocation Fix | Implemented |
| 0045 | Hooks — before_pattern / after_pattern | Implemented |
| 0046 | Context Auto-Loading in Learnable Patterns | Implemented (extended with `auto`/`none`/literal variants) |
| 0047 | LLM Response Caching for Learnable Patterns | Implemented |
| 0048 | Smart-LLM-Routing — Наряд №4 | Implemented |
| 0049 | Session Memory (временная память разговора) | Accepted |
| 0050 | Eval Harness — Automatic Evaluation of Learnable Patterns | Implemented |
| 0051 | inspect() — Pattern Metadata Builtin | Implemented |
| 0052 | Event Stream — Unified Log of All Operations | Implemented |
| 0053 | Conversation State — Managed Dialog Context | Implemented |
| 0054 | Tool Abstraction — External Services as Language Constructs | Implemented |
| 0056 | Lifecycle Control — checkpoint/resume для долгих задач | Implemented |
| 0057 | Static Security Audit (`mlog audit`) | Accepted |
| 0058 | Tiered Skill Index — Structured Skill Loading | Accepted |
| 0059 | Struct via Entity Reuse — No New Keyword | Accepted |
| 0060 | Schema-as-Code — Additive-Only Table DDL in .mlog | Accepted |
| 0061 | Webhook Routing Diagnosis — No Language Gap, Architectural Root Cause | Accepted |
| 0062 | AgentSkillOS-inspired Recipe System + DAG Orchestration | Accepted |
| 0063 | OpenPlanter-inspired Agent Utility Builtins | Accepted |
| 0064 | Lifecycle Hooks Expansion (2 → 5) | Implemented |
| 0065 | config_load YAML Support | Implemented |
| 0066 | Use StableDiGraph for MemoryGraph | Accepted |
| 0067 | Blocking I/O in Async Handlers | Accepted |
| 0068 | Parameterised db_execute | Accepted |
| 0069 | slice() builtin for lists | Accepted |
| 0070 | Parser returns Result instead of abort() | Accepted |
| 0071 | Integration Test Triage (Block 3) | Accepted |
| 0072 | BUILTIN_REGISTRY / dispatcher synchronization | accepted |
| 0073 | JIT backend status — declared experimental | accepted |
| 0074 | HTTP Server — Axum | Accepted |
| 0075 | TW vs VM divergence list (21 cases) | accepted |
| 0076 | VM Dispatch Path Coverage | accepted |
| 0077 | Cost-Aware Model Routing for Learnable Patterns | Implemented |
| 0078 | Metalogos Runtime Fixes (Наряд №12) | Accepted |
| 0079 | Positional taint check for http_post body | Accepted |
| 0080 | Module size policy | accepted |
| 0081 | VM-for-serve feasibility assessment | assessed (not switching yet) |
| 0082 | Phase 5 Language Completeness | Accepted |
| 0083 | Unary Minus Fix | Accepted |
| 0084 | Taint tracking — assignment propagation | accepted |
| 0085 | Type-Safe HTML Templates | Accepted |
| 0086 | Performance baseline benchmarks | accepted |
| 0087 | Full UTF-8 Audit — Наряд №11 | Implemented |
| 0088 | VM Backend for `mlog serve` | Implemented (Наряд №40, extended by №41) |
| 0089 | Confidence Semantics — Actual State | accepted |
| 0090 | Rule Priority Semantics — First-Wins | accepted |
| 0093 | Memory Typology & FTS5 Hybrid Search | Accepted |
| 0094 | Type-Aware Recall & Hybrid Search (Memory Phase 3) | Accepted |
| 0095 | Builtin Arity Range | Accepted |
| 0096 | Replace `block_in_place` with `spawn_blocking` in route handlers | Accepted (implemented) |
| 0097 | Replace block_in_place with spawn_blocking | Accepted |
| 0098 | Registry–Dispatcher Sync | accepted |
| 0099 | Regular Expression Builtins (Наряд №54) | PROPOSED |
| 0100 | LSP Position Resolution via Text Search (Variant B) | Accepted |
| 0101 | Deferred Route Response (post-respond continuation) | Accepted (contract phase |
| 0102 | Native SVG Graphics & Diagrams | Accepted (MVP scope |
| 0103 | Idiomatic `#[ignore]` Reasons (Наряд №73 Block 3) | Accepted |
| 0104 | Cargo feature gating — measured binary impact | Accepted |
| 0105 | Bytecode VM — experimental scope (not full-language equivalent) | Accepted |
| 0106 | `Option`/`Result` — не вводить, soft-failure остаётся моделью ошибок | Rejected |
| 0107 | Отдельный тип `Int` — не вводить без функциональной необходимости | Rejected |
| 0108 | Generics — не вводить, подтверждает решение ADR-0011 | Rejected (reaffirmed) |
| 0109 | `imap` 3.0.0-alpha.15 — intentional pre-release dependency | Accepted |
| 0110 | Протокол обогащения языка | Accepted |
| 0111 | Inline span tracking in AST nodes | Accepted |
| 0112 | Метрика качества `adapt` — текущий mock, не реализованная функция | Accepted |
| 0113 | Pattern-name collision warnings on `run` / `serve` | Accepted |
