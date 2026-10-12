<div align="center">

<img src="logo.jpg" alt="Metalogos" width="220"/>

# METALOGOS

**AI-native programming language with security by design. Written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.93.1-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.30.0-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-28%20blocking%20jobs-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)
[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

</div>

<div align="center">

<a href="https://youtube.com/shorts/Y8IFieOZQLo?feature=shared"><img src="https://img.youtube.com/vi/Y8IFieOZQLo/hqdefault.jpg" alt="Metalogos Presentation" width="400"/></a>

<sub>Short, Aug 12 2026 — visual pitch, not the security contract; see <a href="docs/limitations.md">Known boundaries</a> for what's actually guaranteed.</sub>

</div>

---

## What is Metalogos

Metalogos (mlog) is an open source programming language where AI operations — LLM calls, memory, learning, adaptation — are first-class language constructs, not library integrations. Security constraints (secret opacity, taint tracking, injection prevention) are enforced at the language level, at compile time.

**Who it is for.** Backend and tooling developers who wire LLMs into real products and want the security checks in the compiler, not in code review; researchers who want AI calls, memory, and self-modification as typed language constructs; teams that need provenance and an audit trail for every AI-driven decision.

**Security by design, degraded-loud, provenance-first.** See the [three core ideas](#three-core-ideas) below, then take the [guided tour](docs/book/src/tutorial.md).

## A Taste

```text
// An AI-native "hello": learn a pattern from examples, run it with a
// fallback, and keep the provenance trail — all in the language itself.
entity greeting: String = "Hello, Metalogos!"

learnable Greet {
  learn: Pattern(input: String) -> String {
    return "Hi, " + input + "!"
  }
  fallback: Pattern(input: String) -> String {
    return greeting + " (degraded)"    // degraded-loud, never silent
  }
}

flow Main {
  input: String = "world" -> Greet -> output
}
```

The full tour with the VM/TW parity explanation lives in the [tutorial](docs/book/src/tutorial.md).

## Installation

**Try without building.** The [releases](https://github.com/ShkodnikAI/Metalogos-/releases) carry a prebuilt Linux x86_64 `mlog` binary with a `SHA256SUMS` checksum manifest and build info attached — install, verify, run, and audit in five lines:

```bash
git clone https://github.com/ShkodnikAI/Metalogos-.git && cd Metalogos-
curl -LO https://github.com/ShkodnikAI/Metalogos-/releases/latest/download/mlog-linux-x86_64
sha256sum mlog-linux-x86_64        # compare with SHA256SUMS attached to the release
chmod +x mlog-linux-x86_64 && ./mlog-linux-x86_64 run examples/m1_hello.mlog
./mlog-linux-x86_64 audit examples/m1_hello.mlog
```

**Build from source** — Rust **1.93.1+**, the only requirement. The floor is the verified MSRV of the locked dependency tree (each step of the toolchain walk is an evidence-backed dependency requirement recorded next to `rust-version` in `Cargo.toml`, and `rust-toolchain.toml` tracks the stable channel with the clippy/rustfmt components the CI lints require); an older toolchain refuses with a clear resolver error:

```bash
cargo build --release
./target/release/mlog run examples/m1_hello.mlog
```

Every subcommand: [Quick Start — the Full CLI Tour](docs/book/src/quickstart.md).

## Three Core Ideas

**1. Security contexts are strict by default.** In `mlog serve` every unmarked thread runs in the serve-route context: `env()` is denied without an allowlist, `exec()` needs the server flag, file ingest refuses sensitive paths outright — including cron ticks and MCP tools. A forgotten permission at a new launch point is a loud denial, not silent process rights. Escape hatches exist, are explicit, and are printed at startup as danger flags.

**2. Labels and provenance, end to end.** Values carry classification labels (public/internal/private/secret); sources are tainted (`env`, user input, LLM output, MCP tool data); sinks check clearance at compile time and at runtime in the VM. A secret reaching `respond()` is a compile error, not a code-review finding. Provenance is recorded in the ledger, not in comments.

**3. Degraded means loud.** AI subsystems degrade — backends fall down ladders, learnables fall back to patterns, forecast rungs refuse without the feature. Every degradation is a typed, audible event (`Degraded(t)`); silent quality loss is treated as a defect. The honest-boundary discipline (`docs/limitations.md`) is enforced by consistency tests in CI.

## What's Inside

| Capability | In one line |
|---|---|
| AI as language, not library | a learnable pattern is a construct — calling an LLM is indistinguishable from calling a function |
| Security by design | non-literal SQL, plaintext secrets reaching sinks, and unsanitized LLM output are compile-time refusals; `exec`/`env`/MCP are denied by default |
| Dual execution backend | a tree-walking interpreter and a bytecode VM, checked for output parity on every CI run |
| Typed semantic memory | hierarchical typed records, FTS5 BM25 + cosine similarity, fused with Reciprocal Rank Fusion |
| Self-modification | `adapt` mutates its own patterns under a sandbox, with rollback driven by measured accuracy |
| One-binary toolchain | run / serve / mcp-serve / compile / repl / check / audit / eval in a single binary |

**Honest status.** The language core — parser, both backends, audit, serve — is the worked, tested surface. The media pillars are not at the same readiness: Vision has never been validated against real model weights (a hardware-bound No-Go; the exercised paths are mock-first), and Voice is a scaffold — its consent/provenance surfaces are real, the generative tier is not. The project has no independent security audit. The maintained, per-feature truth lives in [docs/limitations.md](docs/limitations.md) and [docs/REALITY.md](docs/REALITY.md).

The [deep dive behind this table](docs/newcomer-contract.md) holds the security walkthroughs, the exact static-analysis boundaries, and the check tables.

## Documentation

| Document | What it holds |
| --- | --- |
| [docs/book](docs/book/src/tutorial.md) | Tutorial, syntax reference, stdlib, overview, architecture, features, dev guide |
| [docs/newcomer-contract.md](docs/newcomer-contract.md) | The deep dive behind "What's Inside" — walkthroughs, check tables, boundaries |
| [REFERENCE.md](REFERENCE.md) | Full builtin reference — 100% of the registry |
| [docs/limitations.md](docs/limitations.md) | The honest boundaries — enforced, not aspirational |
| [docs/privacy.md](docs/privacy.md) | The engineering privacy policy — what is stored, where, how it is deleted (verdict-marked) |
| [docs/threat-model.md](docs/threat-model.md) | The security model: checks, gates, audit classes |
| [CHANGELOG.md](CHANGELOG.md) | Every wave, every change |
| [SECURITY.md](SECURITY.md) | How to report vulnerabilities |
| [AI_USAGE.md](AI_USAGE.md) | How generative AI is used in this project's development |

Key files at the repo root:

```text
├── REFERENCE.md                      # Full builtin reference — 100% of the registry
├── CHANGELOG.md                      # Version history (see metrics above)
├── SECURITY.md                       # Vulnerability disclosure policy
└── docs/                             # book/, adr/, limitations, threat-model, privacy
```

## Metrics (generated)

<!-- BEGIN GENERATED METRICS (scripts/gen_metrics.py — do not edit inside) -->
| Metric | Value (generated — do not hand-edit) |
| ------ | ------------------------------------- |
| Version | 0.30.0 |
| Built-in Functions | 516 functions across 45 modules |
| Typed Signatures | 317/516 (61.43%) — precise 222/516 (43.02%) (№467/№560) |
| SVG/Graphics | 44 builtins, hand-rolled in pure Rust |
| Grammar | 339 rules |
| Architecture Decisions | 183 ADRs |
| Example Programs | 246 .mlog programs |
| Reference | REFERENCE.md (~303 KB) — 100% registry coverage |
| Changelog | CHANGELOG.md (~691 KB) — every wave documented |
<!-- END GENERATED METRICS -->

Validated on every CI run: `scripts/gen_metrics.py --check` plus the independent recomputation in `tests/readme_consistency.rs`.

## Feature Gates

| Feature | Default | What it gates | The heavy tier |
| --- | --- | --- | --- |
| *(none — core)* | ✓ | the language: parser, compiler, the TW/VM backends, audit, semantic | — |
| `svg` / `chart` / `diagram` / `template` | on | the document/graphics family (pure Rust) | — |
| `llm` | on | the LLM client surface (HTTP, no local inference) | — |
| `server` | off | the HTTP/SSE serve stack (axum/tokio) | — |
| `candle` | off | the tensor backend for the generative stacks | candle-core/nn |
| `vision` | off | the image pillar: dit/vae/sampler/encoders (implies `candle`) | tokenizers, image |
| `voice` | off | the voice pillar: TTS/clone/design (implies `candle`) | — |
| `video` | off | the video pillar: T2V/I2V/interp/mux (implies `candle`) | — |
| `vec` | off | the vector contour (sqlite-vec) | — |
| `timesfm` | off | the off-process timesfm-2.5 rung of the `timeseries` ladder | off-process |

The media-path invariant: **core never imports the media processing** — the interpreter/compiler/VM/audit touch only the handle/registry tier (`MediaStore`, `MediaHandle`, `MediaKind`, `VisionRegistry`, `VisionId`, `VideoId`, `VoiceId`, `AudioId`...), enforced by the `core-media-gate` CI job (`scripts/ci/core_media_gate.py`). The core builds without media: `cargo build --no-default-features` is a blocking CI job.

## The Newcomer Contract (2-minute read)

The compressed contract — the [deep dive](docs/newcomer-contract.md) holds the walkthroughs, the gate semantics, and the exact boundaries for every claim below.

### 1. AI as Language, Not Library

In Python/JS, AI requires SDKs, prompt templates, API clients, and HTTP error handling. In Metalogos, a learnable pattern is a language construct. Calling an LLM is indistinguishable from calling a function. The language handles context, caching, retry, and fallback automatically.

### 2. Security by Design — Zero Configuration

OWASP Top 10 is addressed at the language level through a combination of compiler-enforced checks, opaque types, and static audit heuristics. The headline refusals: non-literal SQL, plaintext secrets reaching sinks, and unsanitized LLM output reaching `respond()` are compile-time errors; `exec()`, `env()` (in serve routes), and MCP tool calls are denied by default behind explicit, banner-printed opt-in flags. The full walkthrough with live transcripts: [the deep dive](docs/newcomer-contract.md).

### 3. Dual Execution Backend

Tree-walking interpreter (full language) + bytecode VM. All VM Stage 1 gaps are **CLOSED** ([ADR-0141](docs/adr/0141-vm-production-readiness.md) Stage 1; the maintained truth in [docs/limitations.md](docs/limitations.md)). Programs both backends can run are checked by `crosscheck_backends` for TW↔VM output parity. `mlog serve` ships **the VM by default** ([ADR-0171](docs/adr/0171-serve-default-flip-vm.md)); the tree-walking interpreter remains the guaranteed full-language opt-out via `METALOGOS_SERVE_BACKEND=interpreter`.

### 4. Typed Semantic Memory with Hybrid Search

More than a key-value store. Hierarchical memory (Memory Tree L0/L1/L2), typed records (persona, episodic, instruction, fact), FTS5 BM25 + cosine similarity, merged via Reciprocal Rank Fusion (k=60). A full retrieval system built into the language.

### 5. Self-Modification with Sandbox and Rollback

The `adapt` statement allows a program to modify its own patterns at runtime — with sandboxing, few-shot mutation, and automatic rollback. The quality metric is **REAL in real mode**: the mutated pattern is measured on a golden-task battery, and keep/rollback responds to measured accuracy. The 0.95 stub remains ONLY in mock mode (`METALOGOS_MOCK_LLM=1|true`) — it exercises the rollback mechanism, it is not a quality signal. Details: [the deep dive](docs/newcomer-contract.md) and [docs/limitations.md](docs/limitations.md).

### 6. Complete Toolchain in One Binary

`mlog run`, `mlog serve`, `mlog mcp-serve`, `mlog compile`, `mlog repl`, `mlog check`, `mlog audit`, `mlog eval` — all in a single binary. The LSP server (`mlog-lsp`) and package manager (`mlogpkg`) are separate binaries in the same workspace. A VS Code extension with syntax highlighting is included in the repository.

## Quick Start

```bash
mlog repl                              # interactive session
mlog serve app.mlog                    # routes + cron + MCP from one file
mlogpkg build                          # package a .mlog project
```

The complete subcommand tour (VM backend selection, MCP transports, eval harness, LSP): [docs/book/src/quickstart.md](docs/book/src/quickstart.md).

## Project Links

- **Roadmap and status** — [docs/PLAN-SUMMARY.md](docs/PLAN-SUMMARY.md) and [docs/REALITY.md](docs/REALITY.md), plus the ADR index ([docs/adr/](docs/adr/))
- **Version history (selected milestones)** — [docs/book/src/dev-guide.md](docs/book/src/dev-guide.md)
- **Prior art and influences** — [docs/book/src/overview.md](docs/book/src/overview.md)

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

---

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.

---

*Built with Rust. Designed by AI. For AI.*
