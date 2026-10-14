<div align="center">

<img src="logo.jpg" alt="Metalogos" width="220"/>

# METALOGOS

**AI-native programming language with security by design. Written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.93.1-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/v0.30.1-blue.svg)](https://github.com/ShkodnikAI/Metalogos-/releases)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](#license)
[![CI](https://img.shields.io/badge/CI-28%20blocking%20jobs-brightgreen.svg)](https://github.com/ShkodnikAI/Metalogos-/actions)
[![Open Collective](https://img.shields.io/opencollective/all/metalogos?label=Backers&logo=open-collective&color=7fadf2)](https://opencollective.com/metalogos)

</div>

<div align="center">

<a href="https://youtube.com/shorts/Y8IFieOZQLo?feature=shared"><img src="https://img.youtube.com/vi/Y8IFieOZQLo/hqdefault.jpg" alt="Metalogos Presentation" width="400"/></a>

<sub>Short, Aug 12 2026 — visual pitch, not the security contract; see <a href="docs/limitations.md">Known boundaries</a> for what's actually guaranteed.</sub>

</div>

## What is Metalogos

Metalogos (mlog) is an open source programming language where AI operations — LLM calls, memory, learning, adaptation — are first-class language constructs, not library integrations. Security constraints (secret opacity, taint tracking, injection prevention) are enforced at the language level, at compile time. Built for backend and tooling developers who wire LLMs into real products, researchers who want AI calls as typed constructs, and teams that need provenance for every AI-driven decision.

## Quick Start

**Try without building.** The [releases](https://github.com/ShkodnikAI/Metalogos-/releases) carry a prebuilt Linux x86_64 `mlog` binary with a `SHA256SUMS` manifest and build info attached:

```bash
git clone https://github.com/ShkodnikAI/Metalogos-.git && cd Metalogos-
curl -LO https://github.com/ShkodnikAI/Metalogos-/releases/latest/download/mlog-linux-x86_64
sha256sum mlog-linux-x86_64        # compare with SHA256SUMS attached to the release
chmod +x mlog-linux-x86_64 && ./mlog-linux-x86_64 run examples/m1_hello.mlog
./mlog-linux-x86_64 audit examples/m1_hello.mlog
```

**Build from source** — Rust **1.93.1+** (the verified MSRV of the locked tree, recorded next to `rust-version` in `Cargo.toml`; an older toolchain refuses with a clear resolver error):

```bash
cargo build --release
./target/release/mlog run examples/m1_hello.mlog
```

Every subcommand: [Quick Start — the Full CLI Tour](docs/book/src/quickstart.md).

## A Taste

```text
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

An AI-native "hello": a learnable pattern with a fallback, run in a flow — all in the language itself. The full tour with the VM/TW parity explanation lives in the [tutorial](docs/book/src/tutorial.md).

## Three Core Ideas

1. **Security contexts are strict by default.** In `mlog serve` every unmarked thread runs in the serve-route context: `env()` denied without an allowlist, `exec()` behind the server flag, sensitive file paths refused outright — a forgotten permission is a loud denial, never silent process rights.
2. **Labels and provenance, end to end.** Values carry classification labels; sources are tainted (`env`, user input, LLM output, MCP tool data); sinks check clearance at compile time and in the VM. A secret reaching `respond()` is a compile error, not a code-review finding.
3. **Degraded means loud.** AI subsystems degrade — backends fall down ladders, learnables fall back to patterns — and every degradation is a typed, audible event (`Degraded(t)`). Silent quality loss is treated as a defect.

## What's Inside

| Capability | In one line |
|---|---|
| AI as language, not library | a learnable pattern is a construct — calling an LLM is indistinguishable from calling a function |
| Security by design | non-literal SQL, plaintext secrets reaching sinks, and unsanitized LLM output are compile-time refusals; `exec`/`env`/MCP are denied by default |
| Dual execution backend | a tree-walking interpreter and a bytecode VM, checked for output parity on every CI run |
| Typed semantic memory | hierarchical typed records, FTS5 BM25 + cosine similarity, fused with Reciprocal Rank Fusion |
| Self-modification | `adapt` mutates its own patterns under a sandbox, with rollback driven by measured accuracy |
| One-binary toolchain | run / serve / mcp-serve / compile / repl / check / audit / eval in a single binary |

**Honest status.** The language core — parser, both backends, audit, serve — is the worked, tested surface; Vision has never been validated against real model weights (a hardware-bound No-Go) and Voice's generative tier is a scaffold; there is no independent security audit. The maintained, per-feature truth lives in [docs/limitations.md](docs/limitations.md) and [docs/REALITY.md](docs/REALITY.md); the deep dive behind the table is [docs/newcomer-contract.md](docs/newcomer-contract.md).

## Documentation

| Document | What it holds |
| --- | --- |
| [docs/book](docs/book/src/tutorial.md) | Tutorial, syntax reference, stdlib, overview, architecture, features, dev guide |
| [docs/newcomer-contract.md](docs/newcomer-contract.md) | The deep dive behind "What's Inside" — walkthroughs, check tables, boundaries |
| [REFERENCE.md](REFERENCE.md) | Full builtin reference — 100% of the registry |
| [docs/limitations.md](docs/limitations.md) | The honest boundaries — enforced, not aspirational |
| [docs/threat-model.md](docs/threat-model.md) | The security model: checks, gates, audit classes |
| [docs/privacy.md](docs/privacy.md) | The engineering privacy policy — what is stored, where, how it is deleted |
| [CHANGELOG.md](CHANGELOG.md) | Every wave, every change |
| [SECURITY.md](SECURITY.md) | How to report vulnerabilities |
| [docs/feature-gates.md](docs/feature-gates.md) | The feature-flag surface and the heavy tier per gate |
| [AI_USAGE.md](AI_USAGE.md) | How generative AI is used in this project's development |

Repo-root map: `REFERENCE.md` (builtins), `CHANGELOG.md` (history), `SECURITY.md` (disclosure), `docs/` (book, adr, limitations, threat-model, privacy).

## Metrics (generated)

<!-- BEGIN GENERATED METRICS (scripts/gen_metrics.py — do not edit inside) -->
| Metric | Value (generated — do not hand-edit) |
| ------ | ------------------------------------- |
| Version | 0.30.1 |
| Built-in Functions | 526 functions across 45 modules |
| Typed Signatures | 337/526 (64.06%) — precise 227/526 (43.15%) |
| SVG/Graphics | 44 builtins, hand-rolled in pure Rust |
| Grammar | 339 rules |
| Architecture Decisions | 184 ADRs |
| Example Programs | 253 .mlog programs |
| Reference | REFERENCE.md (~313 KB) — 100% registry coverage |
| Changelog | CHANGELOG.md (~57 KB) — every wave documented |
<!-- END GENERATED METRICS -->

Validated on every CI run: `scripts/gen_metrics.py --check` plus the independent recomputation in `tests/readme_consistency.rs`.

## Feature Gates

The full gate table (defaults, what each gate opens, the heavy tier) and the core-media invariant live in [docs/feature-gates.md](docs/feature-gates.md). Headline: the core language builds with `cargo build --no-default-features` — a blocking CI job (`core-media-gate`).

## The Newcomer Contract (2-minute read)

The compressed contract moved to its home (the README links, it does not duplicate): **[docs/newcomer-contract.md](docs/newcomer-contract.md)** holds the six-claim walkthrough — AI as language, security by design, the dual backend, typed semantic memory, self-modification with rollback, the one-binary toolchain — with the gate semantics and the exact boundaries of every claim.

## Project Links

- **Roadmap and status** — [docs/PLAN-SUMMARY.md](docs/PLAN-SUMMARY.md) and [docs/REALITY.md](docs/REALITY.md), plus the ADR index ([docs/adr/](docs/adr/))
- **Version history (selected milestones)** — [docs/book/src/dev-guide.md](docs/book/src/dev-guide.md)
- **Prior art and influences** — [docs/book/src/overview.md](docs/book/src/overview.md)

## Support the Project

Metalogos is developed by a solo developer — your support keeps the project alive and growing. Transparent funding on Open Collective: see exactly where every dollar goes — [opencollective.com/metalogos](https://opencollective.com/metalogos).

Direct cryptocurrency support (from anywhere, regardless of banking restrictions): **USDT (TRC-20, Tron)** — `TU6adaaFxJdmvXRT8fhu9w3NQJeNueUyJk`, QR: <img src="assets/qr_wallet.jpg" width="120">.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.

*Built with Rust. Designed by AI. For AI.*
