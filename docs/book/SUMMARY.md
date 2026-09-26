# METALOGOS Documentation

Welcome to the official documentation for **METALOGOS** — an AI-native programming language built on seven pillars: Entity, Pattern, Flow, Memory, Rule, Learn, and Adapt.

## What is METALOGOS?

METALOGOS is a programming language designed from the ground up for AI-assisted software development. It natively supports:

- **Fluid Types** — confidence-based type collapse at the point of use (does not propagate through calls; see ADR-0089)
- **Learnable Patterns** — functions powered by LLMs
- **Adaptive Mutations** — patterns that improve from feedback
- **Semantic Memory** — persistent knowledge with decay
- **Knowledge Graphs** — entities connected by relations
- **Rule Engine** — priority-based inference with conflict resolution

## Getting Started

If you're new to METALOGOS, start with the [Tutorial](./tutorial.md) — it walks you from a simple "Hello, World!" program all the way to adaptive patterns with machine learning.

## Guides

- [Tutorial: Hello to Adapt](./tutorial.md) — hands-on guide from basics to advanced features
- [Syntax Reference](./syntax.md) — complete language syntax reference
- [Standard Library Reference](./stdlib.md) — built-in modules: string, math, collections, JSON, HTTP, crypto, memory, SVG, and more (359 builtins across 22 modules)

### The Full Picture (from the README, naryad #460)

- [Overview](./src/overview.md) — what Metalogos is and why it exists
- [Competitive Advantages](./src/advantages.md) — the honest comparison table
- [Eight Semantic Primitives](./src/primitives.md) — the constructs behind the pillars
- [Architecture](./src/architecture.md) — crates, modules, the VM, the file tree
- [Key Features](./src/features.md) — the feature-by-feature tour
- [Quick Start — the Full CLI Tour](./src/quickstart.md) — every `mlog` subcommand explained
- [Development Guide](./src/dev-guide.md) — CI/CD, ecosystem, technology stack
- [Self-Hosting & Packaging](./src/selfhost.md) — the №197 parser and the №198 mlogpkg
## Architecture

- [ADR Index](../adr/README.md) — Architecture Decision Records (0001–0111)

## Tooling

- **`mlog run <file.mlog>`** — execute a .mlog program
- **`mlog repl`** — interactive REPL with persistent state
- **`mlog check <file.mlog>`** — semantic analysis without execution
- **`mlog-lsp`** — LSP server for editor integration (diagnostics, go-to-definition, hover)
- **`mlogpkg init|add|build`** — package manager for .mlog projects
