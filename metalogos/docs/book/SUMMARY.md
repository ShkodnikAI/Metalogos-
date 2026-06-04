# METALOGOS Documentation

Welcome to the official documentation for **METALOGOS** — an AI-native programming language built on seven pillars: Entity, Pattern, Flow, Memory, Rule, Learn, and Adapt.

## What is METALOGOS?

METALOGOS is a programming language designed from the ground up for AI-assisted software development. It natively supports:

- **Fluid Types** — superposition of types with confidence scores
- **Confidence Propagation** — uncertainty flows through data pipelines
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
- [Standard Library Reference](./stdlib.md) — built-in modules: string, math, collections

## Architecture

- [ADR Index](./adr-index.md) — Architecture Decision Records (0001–0019)

## Tooling

- **`mlog run <file.mlog>`** — execute a .mlog program
- **`mlog repl`** — interactive REPL with persistent state
- **`mlog check <file.mlog>`** — semantic analysis without execution
- **`mlog-lsp`** — LSP server for editor integration (diagnostics, go-to-definition, hover)
- **`mlogpkg init|add|build`** — package manager for .mlog projects
