# ADR-0016: CLI + REPL + Semantic Check (Phase 3)

**Status:** Accepted
**Date:** 2026-06-01
**Milestone:** Phase 3 — Developer tooling

## Context

METALOGOS had a single CLI command `mlog run <file>` that executed a `.mlog` program from start to finish. The interpreter was stateless — each invocation created a fresh `Interpreter::new()` and discarded all state (entities, patterns, memory, relations, adapted patterns) after completion. There was no way to interact incrementally with the language, no way to validate programs without running them, and no developer ergonomics beyond the binary entry point.

Phase 3 (CLI/REPL/LSP/package manager) in the Metalogos Build Ladder roadmap targets developer tooling. This ADR covers the first three items: a complete CLI with subcommands, an interactive REPL with persistent state, and a semantic analysis mode that reports errors without execution.

## Decision

### 1. CLI: clap with derive macros

We extended the existing clap-based CLI from one subcommand (`Run`) to three:

```
mlog run <file>     — execute a .mlog program (unchanged semantics)
mlog repl            — start interactive REPL session
mlog check <file>   — run semantic analysis without execution
```

The `[[bin]]` section in `Cargo.toml` was made explicit: `name = "mlog"`, `path = "src/main.rs"`.

**Rationale:** clap `derive` was already a dependency (Phase 1). Adding subcommands is zero-cost. No additional CLI framework needed.

### 2. REPL: rustyline with state persistence

The REPL (`mlog repl`) uses `rustyline` for line editing, history (saved to `~/.mlog_history`), and readline. Two modes:

- **Interactive (tty):** rustyline with `mlog>` prompt, history, `exit`/`quit` commands. Output prefixed with `=> `.
- **Piped (non-tty):** reads lines from stdin silently, evaluates each. Used by integration tests.

State persistence is achieved by keeping a single `Interpreter` instance alive across all REPL iterations. A new `feed_line(interp, line)` function in `lib.rs` parses a single line into declarations and feeds them to the existing interpreter — entities, patterns, memory, relations, and adapted patterns survive between inputs.

**tty detection:** Uses `libc::isatty(0)` on Unix (no extra `atty` crate). Falls back to `METALOGOS_FORCE_PIPE=1` env var for test control.

**Rationale:** rustyline is lightweight (no async, no event loop), widely used in Rust CLIs (rustc, cargo subcommands). `reedline` was considered (used by nushell) but is heavier and overkill for our needs. The `Interpreter::run()` method already accepts `Vec<Declaration>`, so `feed_line()` is a thin wrapper — no interpreter restructuring needed.

### 3. Semantic Check: `src/semantic.rs` with `AnalysisResult`

A new `semantic` module provides `check_program(source) -> AnalysisResult`:

- Two-pass analysis: (1) collect all declarations, (2) cross-reference validation.
- Validates: entity type references, field initializers, rule targets, adapt/mutate targets, flow pipeline steps, branch targets, duplicate names.
- `AnalysisResult { errors: Vec<String>, warnings: Vec<String> }` with `format()` for display.
- `mlog check` exits with code 1 if errors found, 0 if clean.

**Rationale:** Validation without execution is essential for CI, editors, and quick feedback. The two-pass approach is simple and sufficient for the current grammar. No borrow-checker issues because we operate on immutable declaration references, not the mutable interpreter state.

## Consequences

**Positive:**
- Developers can interact with METALOGOS incrementally — define entities, patterns, then flows, all in one session.
- `mlog check` enables CI integration and quick validation without running potentially slow LLM calls.
- REPL history persists across sessions via `~/.mlog_history`.
- `feed_line()` is a public API that enables future tooling (IDE integration, LSP, notebooks).

**Neutral:**
- REPL state is in-memory only — not persisted to disk between sessions. This is intentional for now; a future `mlog save`/`mlog load` could serialize interpreter state.
- Semantic check is structural only (no type inference). Phase 2's type inference could be integrated later.

**Negative:**
- rustyline adds ~8 crate dependencies (nix, rustix, etc.). Acceptable for a CLI tool.
- `libc` added for `isatty()`. Could be removed if a pure-Rust alternative is preferred.

## Contract Tests

| Test | File | Validates |
|------|------|-----------|
| REPL 3-line integration | `tests/repl_integration.rs` | entity + pattern + flow incremental eval |
| Check OK program | `tests/check_integration.rs` | valid program produces no errors |
| Check undefined type | `tests/check_integration.rs` | reports unknown type error |
| Check adapt target | `tests/check_integration.rs` | reports missing learnable pattern |
| Check duplicate | `tests/check_integration.rs` | reports duplicate entity type |
| Check format | `tests/check_integration.rs` | "OK: no issues found" for clean program |
| Golden p3_repl_stdin | `examples/p3_repl_stdin.mlog` | pipe 3 lines → output |

**Total: 11 tests green** (8 golden + 4 semantic unit + 5 check integration + 1 REPL integration, with overlap counted once).
