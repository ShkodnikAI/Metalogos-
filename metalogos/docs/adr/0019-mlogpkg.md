# ADR 0019: Package Manager — mlogpkg (Phase 3.4)

**Date:** 2026-06-01
**Status:** accepted
**Supersedes:** —
**Superseded by:** —

## Context

METALOGOS Phase 3.3 (LSP) is closed. The language now has: CLI, REPL, standard library with import mechanism, and LSP server. The final Phase 3 deliverable requires a package manager to manage .mlog projects, their dependencies, and build workflow.

Without a package manager, users must manually manage imports, track dependency versions, and coordinate source files. A package manager provides:
- Project initialization with a standard manifest format
- Dependency tracking and resolution
- Build workflow that validates all source files
- Lock files for reproducible builds

## Decision

### 1. Separate `mlogpkg` crate

Create a dedicated binary crate (`mlogpkg/`) as a Cargo workspace member. It depends on the core `metalogos` crate for semantic checking.

**Workspace structure (updated):**
```
metalogos/              (root Cargo.toml — workspace)
  Cargo.toml            [workspace: metalogos, mlog-lsp, mlogpkg]
  src/                  (core lib + binary: mlog)
  mlog-lsp/             (LSP server: mlog-lsp)
  mlogpkg/              (package manager: mlogpkg)
    Cargo.toml
    src/main.rs         (init, add, build, info commands)
    tests/              (integration tests)
  std/                  (standard library modules)
  docs/book/            (mdbook documentation)
  editors/vscode/       (VS Code extension manifest)
```

### 2. Manifest format: `mlog.toml`

```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2024"

[dependencies]
some-pkg = "0.3.0"
```

The format mirrors Rust's `Cargo.toml` for familiarity. Key fields:
- `package.name` — project identifier
- `package.version` — semantic version
- `package.edition` — language edition (default "2024")
- `dependencies` — map of package name to version constraint

### 3. Commands

| Command | Description |
|---------|-------------|
| `mlogpkg init [name]` | Creates `mlog.toml` and `src/main.mlog` scaffold |
| `mlogpkg add <pkg> [version]` | Adds dependency to `mlog.toml`, checks local registry |
| `mlogpkg build` | Resolves dependencies, checks all `.mlog` sources, writes `mlog.lock` |
| `mlogpkg info` | Shows project metadata and dependencies |

### 4. Local registry

Packages are stored in `~/.mlog/registry/<pkg-name>/`. Each package directory contains its own `mlog.toml` manifest and source files. There is no remote server — packages are installed manually by copying files into the registry directory.

This is intentionally minimal for Phase 3. A remote registry server (with publish/fetch) is deferred to a future phase.

### 5. Lock file: `mlog.lock`

Written after successful `mlogpkg build`. Records resolved dependency versions for reproducible builds. Format:

```json
{
  "packages": {
    "some-pkg": {
      "version": "0.3.0",
      "source": "registry:some-pkg"
    }
  }
}
```

### 6. Build workflow

The `mlogpkg build` command:
1. Reads `mlog.toml`
2. Finds the entry point (`src/main.mlog`)
3. Resolves dependencies from the local registry
4. Collects all `.mlog` source files recursively
5. Runs semantic analysis (`metalogos::check_program`) on each file
6. Writes `mlog.lock` with resolved versions
7. Reports success or failure with error count

### 7. Integration testing

Tests exercise the CLI commands via subprocess invocation:
- `init` creates `mlog.toml` with correct fields
- `init` creates `src/main.mlog` scaffold
- `init` fails if `mlog.toml` already exists
- `build` succeeds on a fresh initialized project
- `build` detects semantic errors in source files
- `build` fails without `mlog.toml`
- `info` displays project metadata

## Consequences

### Positive
- Standard project structure for all METALOGOS projects
- Dependency tracking with version resolution
- Reproducible builds via lock files
- Familiar manifest format for Rust developers
- Build step validates all source files with semantic analysis

### Negative
- Local registry requires manual package installation (no `publish`/`fetch`)
- No version constraint resolution — uses the exact version from the registry package's manifest
- No workspace/multi-package support in this phase

### Future
- Remote registry server with `mlogpkg publish` / `mlogpkg fetch`
- Semantic version constraint resolution (^, ~, >= ranges)
- Workspace support (monorepo with multiple packages)
- Package signing and verification
- `mlogpkg test` — run tests across packages
- `mlogpkg doc` — generate documentation from .mlog sources
