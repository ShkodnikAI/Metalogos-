# ADR Index — Architecture Decision Records

## Numbering rule

Before creating a new ADR, check for duplicates:

```bash
ls docs/adr/ | sed 's/-.*//' | sort | uniq -d
```

Must be empty. If not empty, resolve collisions before proceeding.

Numbers are assigned sequentially. The current accepted maximum is `0177-*`
(ADR-0177 №461 domain freeze; ADR-0176 №453 plan-v2 memory contract). When
numbering a new ADR, remember the tasking-template line: **"domain X is
frozen (ADR-0177) — the naryad is not opened"** (the freeze mechanics, §7
of the ADR). The freeze was lifted by the owner on 2026-09-27 (gh#680,
the lift record 5851623870) — the template line is historical now; the
domain-boundary rules continue in the crate-split roadmap
(docs/refactoring-split-plan.md).

## Reserved numbers (do not reassign)

The following ADR numbers are referenced in source code, tests, or past Naryad
reports and must not be changed:

- **ADR-0073** (`jit-experimental`) — referenced in `README.md`, `CHANGELOG.md`,
  `tests/crosscheck_backends.rs`
- **ADR-0075** (`tw-vm-divergence`) — referenced in `CHANGELOG.md`,
  `tests/crosscheck_backends.rs`, past Naryad reports
- **ADR-0076** (`vm-dispatch-paths`) — referenced in `tests/vm_golden.rs`

## Reserved for plan v2 §19 (booking 0155–0161, naryad №319 / issue #406)

Booked 2026-09-14 by naryad №319. Content — the filling naryads of plan v2 phases,
not №319. The plan's original block 0151–0158 shifted +3: 0151–0153 were taken by
real ADRs (№309/№320/№412) before the booking.

| # | File | Theme | Filling naryad |
|---|---|---|---|
| 0155 | `0155-grant-algebra.md` | Grant algebra — permissions for irreversible operations | №339 — filled 2026-09-17 by naryad #389 (issue #483) |
| 0156 | `0156-tw-vm-jit-parity.md` | TW/VM/JIT parity contract | №328 |
| 0157 | `0157-ledger-profile-prov-intoto.md` | Ledger profile — PROV/in-toto alignment | №343 |
| 0158 | `0158-declassify-boundaries.md` | Declassify boundaries | №326 |
| 0159 | `0159-sim-first-stl.md` | Sim-first verification — STL semantics | №354 |
| 0160 | `0160-identifier-naming-convention.md` | Identifier naming convention | (not assigned) |
| 0161 | `0161-compat-profile.md` | Legacy compatibility profile | №325 |

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
| 0025 | Loops: `each` (data-first) + `while` (fallback) | Accepted |
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
| 0048 | Smart-LLM-Routing — Naryad №4 | Implemented |
| 0049 | Session Memory (temporary conversation memory) | Accepted |
| 0050 | Eval Harness — Automatic Evaluation of Learnable Patterns | Implemented |
| 0051 | inspect() — Pattern Metadata Builtin | Implemented |
| 0052 | Event Stream — Unified Log of All Operations | Implemented |
| 0053 | Conversation State — Managed Dialog Context | Implemented |
| 0054 | Tool Abstraction — External Services as Language Constructs | Implemented |
| 0056 | Lifecycle Control — checkpoint/resume for long-running tasks | Implemented |
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
| 0078 | Metalogos Runtime Fixes (Naryad №12) | Accepted |
| 0079 | Positional taint check for http_post body | Accepted |
| 0080 | Module size policy | accepted |
| 0081 | VM-for-serve feasibility assessment | assessed (not switching yet) |
| 0082 | Phase 5 Language Completeness | Accepted |
| 0083 | Unary Minus Fix | Accepted |
| 0084 | Taint tracking — assignment propagation | accepted |
| 0085 | Type-Safe HTML Templates | Accepted |
| 0086 | Performance baseline benchmarks | accepted |
| 0087 | Full UTF-8 Audit — Naryad №11 | Implemented |
| 0088 | VM Backend for `mlog serve` | Implemented (Naryad №40, extended by №41) |
| 0089 | Confidence Semantics — Actual State | accepted |
| 0090 | Rule Priority Semantics — First-Wins | accepted |
| 0093 | Memory Typology & FTS5 Hybrid Search | Accepted |
| 0094 | Type-Aware Recall & Hybrid Search (Memory Phase 3) | Accepted |
| 0095 | Builtin Arity Range | Accepted |
| 0096 | Replace `block_in_place` with `spawn_blocking` in route handlers | Accepted (implemented) |
| 0097 | Replace block_in_place with spawn_blocking | Accepted |
| 0098 | Registry–Dispatcher Sync | accepted |
| 0099 | Regular Expression Builtins (Naryad №54) | PROPOSED |
| 0100 | LSP Position Resolution via Text Search (Variant B) | Accepted |
| 0101 | Deferred Route Response (post-respond continuation) | Accepted (contract phase |
| 0102 | Native SVG Graphics & Diagrams | Accepted (MVP scope |
| 0103 | Idiomatic `#[ignore]` Reasons (Naryad #73 Block 3) | Accepted |
| 0104 | Cargo feature gating — measured binary impact | Accepted |
| 0105 | Bytecode VM — experimental scope (not full-language equivalent) | Accepted |
| 0106 | `Option`/`Result` — not introduced, soft-failure remains the error model | Rejected |
| 0107 | A separate `Int` type — not introduced without functional necessity | Rejected |
| 0108 | Generics — not introduced, reaffirms the decision of ADR-0011 | Rejected (reaffirmed) |
| 0109 | `imap` 3.0.0-alpha.15 — intentional pre-release dependency | Accepted |
| 0110 | Language Enrichment Protocol | Accepted |
| 0111 | Inline span tracking in AST nodes | Accepted |
| 0112 | `adapt` quality metric — current mock, not an implemented function | Accepted + IMPLEMENTED (реализовано в наряде №375, 2026-09-16 |
| 0113 | Pattern-name collision warnings on `run` / `serve` | Accepted |
| 0114 | `Value::Reflex` as an opaque handle, not a tensor type | Accepted |
| 0115 | What "accuracy" means for `Reflex` | Accepted |
| 0116 | Weight persistence for `Reflex` — SQLite BLOB, not a new file format | Accepted |
| 0117 | Semantics of distillation — mode switching, backward compatibility, and why generation stays out of scope | Accepted |
| 0118 | `candle` as the tensor/autograd dependency for architecture blocks | Accepted |
| 0119 | Extending the layer abstraction for sequence-processing blocks | Accepted |
| 0120 | Opening text generation — amends `ADR-0117` §3 by explicit owner decision | Accepted |
| 0121 | Closing the VM-parity gap for `Reflex` — VM-owned state, not shared `RuntimeContext` | Accepted |
| 0122 | Vision pillar scope — inference-first over open weights, images before video | Accepted |
| 0123 | Vision wedge — Z-Image-Turbo primary, FLUX.2 [klein] fallback | Accepted |
| 0124 | `Value::Vision` as opaque handle + `VisionRegistry` — Reflex patterns, VM-owned state | Accepted |
| 0125 | Provenance and supply-chain gates for generated media | Accepted |
| 0131 | Stable diagnostic codes for `mlog check` — extending the existing `audit.rs` convention, not a new one | Accepted |
| 0132 | MCP client — hand-rolled JSON-RPC over stdio, stateless, output with taint `UserInput` | Accepted (approved by the owner 2026-09-12; taint kind of MCP output |
| 0133 | `call_llm_schema` — structured LLM output through a hand-rolled JSON-Schema subset validator | Accepted |
| 0134 | sqlite-vec as a KNN accelerator for semantic recall — spike #271 verdict: Go | Accepted (verdict gate of dispatch #316: resolved by the implementer per the spike 2026-09-12, as the gate mechanics prescribe) |
| 0135 | Semantic cache (cache_semantic) + LRU bound for the ADR-0047 cache | Accepted |
| 0136 | redact(text, mode) — PII/secrets as a taint sanitizer | Accepted (stop-gate SG-2 approved by the owner 2026-09-12) |
| 0137 | LLM streaming — `llm_stream_open/next/close` over `reqwest::blocking` | Accepted |
| 0138 | Per-call LLM traces — file-based JSONL with OpenTelemetry GenAI field names | Accepted |
| 0139 | SMFS — memory export as a virtual read-only FS (`sm:`) | Proposed (draft of spike #282; spike verdict |
| 0140 | Diagnostic codes — addendum (no-reuse rule + SSOT-registry discipline) | Accepted |
| 0141 | VM production-readiness — staged gap closure + parity-gated default flip | Accepted (owner decision 2026-09-14 |
| 0142 | Error protocol — structural errors through try-extended semantics (candidate B) | Accepted + IMPLEMENTED (owner decision 2026-09-14 |
| 0143 | Voice pillar — scope (TTS, zero-shot cloning, voice-design) | Accepted |
| 0144 | Voice value-registry — `Value::Audio(AudioId)` opaque handle + VoiceRegistry | Accepted |
| 0145 | Voice security gates — consent, provenance, privacy, taint | Accepted |
| 0146 | Voice wedge — Chatterbox Multilingual V3 primary, Kokoro-82M warm-up | Accepted |
| 0147 | Video pillar — scope (T2V/I2V primary, v2v phase-gated) | Accepted |
| 0148 | Video value-registry — `Value::Video(VideoId)` opaque handle + VideoRegistry | Accepted |
| 0149 | Video security gates — likeness consent, provenance, taint, adult policy | Accepted |
| 0150 | Video wedge — Wan 2.2 primary, CogVideoX-5B warm-up | Accepted |
| 0151 | Video I2V pipeline — first/last-frame anchors, RIFE-class interpolation, AV sidecar mux | Accepted |
| 0152 | C2PA mini-slice — Art. 50 synthetic marking on egress (no clearance lattice) | Accepted |
| 0153 | Video DiT text path wired — prompt embedding genuinely conditions the denoiser | Accepted |
| 0154 | Label lattice for taint kinds — (conf, integrity, consent-scope) | Accepted |
| 0155 | Grant algebra — permissions for irreversible operations | Implemented (naryads #389–#393 landed the algebra in main: `grant_issue`/`grant_subgrant`/`grant_revoke`/`grant_use`/`db_execute_with_grant` with scope/TTL/quota enforcement, the action bridge over the №316 sink SSOT, the exhaustive DenyEvent reasons with `on_deny` handlers, and the signed Action Ledger v1. Anchors: `tests/naryad_390_grants.rs`, `tests/grant_algebra_fuzz.rs`, `tests/naryad_391_bridge.rs`, `tests/naryad_392_deny_event.rs`, `tests/naryad_393_ledger.rs`; independently accepted by the wave-3 audit and the external audit 2026-09-19; exercised end-to-end by the kitchen-camera e2e and the #395 dogfood run. Originally Accepted 2026-09-17 |
| 0156 | TW/VM/JIT label parity — LabelJoin/SinkCheck in the bytecode | Accepted |
| 0157 | Ledger profile — PROV/in-toto alignment | Accepted (fills the reserved booking of 2026-09-14) |
| 0158 | Declassify boundaries | reserved |
| 0159 | Sim-first verification — SafetyBounds as an STL-formula monitor | Accepted (filled 2026-09-22 by naryad №354, wave 10 |
| 0160 | Identifier naming convention | reserved |
| 0161 | Legacy compatibility profile (`profile legacy`) | Accepted |
| 0162 | Unified media handles and the media store (lazy materialization, refcount, at-rest sealing) | Accepted |
| 0163 | Backend registry — classes, SHA-pin contract, license classes and the distribution gate | Accepted |
| 0164 | Perception AST — HandleSource/Lift/Sink/ProvBind and the origin chain | Accepted |
| 0165 | BackendSelect — the backend ladder and Degraded(t), typed degradation | Accepted |
| 0166 | The C2PA contour of media handles — read/write manifests and the generation guarantee | Accepted |
| 0167 | Action Ledger v1 — signed append-only journal of actions | Accepted |
| 0168 | MCP server transports — stdio + HTTP/SSE, bearer auth, compiled tool-policy | Accepted |
| 0169 | Stable `try` error codes — origin-stamped classification | Accepted |
| 0170 | Persistence taint layer 2 — locally-bound key prefixes; points-to deferred | Accepted |
| 0171 | `mlog serve` default backend flip — the VM becomes the default (Stage 5 executed) | Accepted |
| 0172 | Session model — wake/interrupt/duty over a process-global registry | Implemented (Accepted 2026-09-22 by №348; Implemented 2026-09-22 |
| 0173 | The derived-from graph and cascading forgetting over Memory<K> | Accepted |
| 0174 | Directed audio effects (`listen`/`speak`) and the duplex channel — barge-in over the session priority ladder | Accepted |
| 0175 | The tick context — the cron dispatch executes in the program context | Accepted |
| 0176 | The plan-v2 memory contract — the SSOT of the memory surface | Accepted |
| 0177 | The domain freeze until 0.27 — no new subsystems, the core first | Lifted |
| 0178 | The generative contour boundary — experimental, scoped, exit-ready | Accepted |
| 0179 | The 0.28 Release Gate — the Absolute-Goals Criteria (v2) | IMPLEMENTED (2026-10-05) |
| 0180 | The Soft-Failure Naming Rule — "Silence Is Visible in the Name" | ACCEPTED |
| 0181 | The 0.29 Release Gate — the Absolute-Goals Criteria (owner-fixed 2026-10-04) | Accepted (the parameters fixed by the OWNER, 2026-10-04) |
| 0182 | The media-handle family and the backend-registry interface — the domain-line reopening contour | Accepted |
| 0183 | The migration-rollback boundary — schema evolution stays ADDITIVE-ONLY; the rollback path is backup restore, not a language surface | Accepted |
| 0184 | The fate of the depth-≥2 early answer — `RESPOND_SWALLOWED` stays FOREVER (variant Б); the propagation-to-any-depth (variant А) is not built | Accepted |
| 0185 | The container form of `memory_forget` — DEFERRED to the memory-phase planning (not rejected); the managed by-query forgetting is the accepted surface | Accepted |
| 0186 | The 0.30 Release Gate — the Owner-Fixed Criteria (Accepted; the §5 mechanical transition landed) | Accepted (the executor's draft №615 |
| 0187 | The Memory-Phase Plan — the Phase-4 slot, the container-form decision slot (ADR-0185 revisit), the parity posture | Proposed** (the executor's draft plan |
| 0188 | The Template Semantics Composition — the §7 lane plan: the compile-time port re-priced, the phased recommendation (the OWNER's choice slot) | Accepted |
| 0189 | The release train — the Monday cadence, the window, and the honest-content rule | Accepted |
| 0190 | metalogos-examples — the showcase charter (three apps with e2e, for the language consumer) | Accepted |
| 0191 | The 0.31 Release Gate — the sensitive-surface parameter (OWNER-FIXED; the divergence closed by the owner's verdict 2026-10-13) | Accepted** (`owner_fixed: true` |
| 0192 | The `html` / `xml` capability features — html5ever / xml5ever behind off-by-default flags | Accepted** (the owner's dispatch #1222 |
| 0193 | The 0.32 Typed-Source Goal — the FIXED-LIST closure form (ACCEPTED 2026-10-19; the owner's verbatim) | Accepted** (2026-10-19 |
| 0194 | The query-parity decision package — the db_ops error paths and the query result contract (ACCEPTED 2026-10-20; the owner's verbatim — packages A(c)+B(i)) | Accepted** (2026-10-20 |
| 0195 | The 0.34 Release-Gate Criteria — the draft variants (ACCEPTED 2026-10-20; the owner's verbatim — the §5 package as submitted) | Accepted** (2026-10-20 |
| 0196 | The MCP Policy Gateway — the flagship pilot (the C1 (b) decision, ACCEPTED 2026-10-23 by the owner's verbatim) | Accepted** (2026-10-23 |
| 0197 | The VM Is the Only Executor of User Commands; TW Is the Reference Oracle (the C5 (b) decision, ACCEPTED 2026-10-23 by the owner's verbatim) | Accepted** (2026-10-23 |
| 0198 | The Error Model — the Loud Failure + `_or` + `try`; `Result<T>`/`Option<T>` in the Type Layer Without New `Value` Variants (the C6 R-b decision, ACCEPTED 2026-10-23 by the owner's verbatim; SUPERSEDES ADR-0106) | Accepted** (2026-10-23 |
| 0199 | The Typed-Signature Metric Reads by Maturity Tier — the STABLE-Tier Share Is the PRIMARY Definition; the Full-Registry Figure Stays Beside (the C3-a T3 decision) | Accepted** (the C3-a T3 owner decision of 2026-10-23, |
| 0200 | The Provable Tier-Lowering Line — the Reflex Attestation Plan, the Owner Decisions 1–6, and the W1–W9 + E1–E4 Roadmap | Accepted** (the owner decisions of 2026-10-27 |
| 0201 | Phase 8 "Verifiable Compliance & Trust" — the Through-Line, the Subphase Train 0.35–0.47, and the ADR Booking 0201–0215 | Accepted** (the owner decision of 2026-10-28 |
| 0202 | ModelPassport — the Passport of a Model as a Typed Artifact (the License-Typed Model Layer, the 8D-1 Subphase Record) | Accepted** (the owner decision of 2026-10-28 |
| 0203 | Capability-Token — the Typed Carrier of a Grant and the Single Consumption Contract (the 8B-1 Subphase Record) | Accepted** (the owner decision of 2026-10-28 |
| 0204 | Portable Refusal Algebra — the Refusal Algebra as a Portable Artifact (the 8F-1 / C6 Subphase Record) | Accepted** (the owner decision of 2026-10-28 |
| 0205 | Typed Approval — the Human Mandate as a Typed Carrier of Consent and Oversight (the 8B-2 / C11 Subphase Record) | Proposed** (the ACCEPTED path is the wave-75 acceptance |
| 0206 | Compliance Evidence Compiler — the Attestation Map as a Typed, Verifiable, Sellable Artifact (the 8A-1 / C1 Subphase Record) | Proposed** (the ACCEPTED path is the wave-76 acceptance |
| 0216 | The `rollback_if` Metric Is a Registry Name — the Gate Reads the Metric's Own Value and Carries the Majority Baseline (the W2′ Grammar Semantics) | Accepted** (the наряд-mandated ADR-запись |
