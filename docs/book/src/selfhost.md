# Self-Hosting & Packaging

## Self-hosted parser (Naryad #197)

`self-host/parser.mlog` is a Metalogos parser written in Metalogos itself. It
builds on top of the self-hosted lexer (`self-host/lexer.mlog`) — both files
are bootstrap-complete: each can process its own source.

### Supported subset (Block 1)

The first version supports the subset of the grammar required for bootstrap
(parsing parser.mlog itself). Constructs outside this subset are skipped via
the `SkipDecl` helper; the parser does NOT silently accept them as something
else, and a `// templ-decl` comment in the source marks which constructs are
intentionally excluded.

**Supported top-level declarations:**

| Declaration | Example |
|---|---|
| `pattern` | `pattern Foo(x: String) -> String { ... }` |
| `entity` (simple) | `entity greeting: String = "Hello"` |
| `flow` | `flow Main { input: Type = src -> Step1 -> output }` |
| `import` | `import std/string as str` |

**Supported statements:** `let`, `let mut`, assignment (`x = ...`), `if cond
{ ... } else if ... else { ... }`, `if cond then { ... }`, `if cond then X
else Y` (expression form), `while`, `each x in xs { ... }`, `each i, x in xs
{ ... }` (with index), `return`, bare expression statement, `break`,
`continue`.

**Supported expressions:** full precedence chain `or` / `and` / comparison
(`==`, `!=`, `<`, `>`, `<=`, `>=`) / additive / multiplicative, unary
minus, function call (`f(args)`), qualified call (`mod.fn(args)`), field
access (`obj.field`), index access (`arr[i]`), list literal (`[a, b, c]`),
struct literal (`{ k: v, ... }`), parenthesized expression, `if cond then
X else Y` expression form, and the literals STRING / NUMBER / INT / BOOL /
IDENT. Unary minus is desugared to `0.0 - X` to match the Rust AST's
representation (`src/parser/expr.rs`).

**Explicitly NOT supported** (deferred to a future naryad):

- All other top-level declarations: `entity Type { ... }` (record), `entity
  name: Type = { ... }` (instance), `rule`, `memorize`, `forget`, `relate`,
  `adapt`, `mutate`, `eval`, `test`, `type`, `llm`, `hook`, `sandbox`, `db`,
  `schema`, `skill_index`, `memory`, `conversation`, `context_budget`,
  `fluid`, `learnable pattern`, `tool`, `mlogserver`, `template`, `reflex`,
  `reflex_seq`, `reflex_gen`.
- `match` statement and `match` expression.
- `try` expression.
- Block `if/else` as an expression with side-effecting inner statements
  (Metalogos v0.19's scoping rule blocks mutations to outer `let mut`
  variables from inside a BlockIfElse expression; parser.mlog works around
  this by delegating to helper patterns — see `ParseElseBranch`,
  `ParseImportAlias`).

### Usage

```sh
# Parse a .mlog file (writes AST to stdout):
MLOG_PARSE_TARGET=path/to/file.mlog mlog run self-host/parser.mlog

# Bootstrap (parser.mlog parses itself):
MLOG_PARSE_TARGET=self-host/parser.mlog mlog run self-host/parser.mlog
```

### Contracts

Two integration tests verify the parser's correctness:

- `tests/naryad_197_parser_self_parses.rs` — bootstrap test: parser.mlog
  successfully parses its own source (~4 min runtime on a typical dev
  machine; the test has a 12-minute timeout).
- `tests/naryad_197_parser_matches_rust_parser.rs` — on a representative
  sample of 12 .mlog files covering the Block 1 subset, the AST produced by
  parser.mlog is structurally equivalent to the AST produced by the
  production Rust parser (`src/parser/`). The comparison normalises both
  sides to the same S-expr string format.

### Lexer bugs fixed in parser.mlog's local Tokenize copy

The self-hosted lexer (`self-host/lexer.mlog`) has several known issues
that prevented the parser from working directly off its output. The
parser's local `Tokenize` copy includes the following fixes (each
documented inline in `self-host/parser.mlog`):

1. **Whitespace handling**: the original lexer only recognized ASCII
   space, treating `\n`/`\t`/`\r` as quote chars. This produced phantom
   STRING tokens spanning multiple lines and swallowing entire
   declarations. Fix: treat all four whitespace chars as whitespace.
2. **Underscore in identifiers**: identifiers like `index_of` were split
   into `index`, `_`, `of` because the lexer's `abc` alphabet string
   omitted `_`. Fix: treat `_` as a letter, and allow it (plus digits)
   in identifier continuation.
3. **Multi-char operators**: `==`, `!=`, `<=`, `>=` were emitted as two
   single-char OPERATOR tokens, breaking comparison parsing. Fix: detect
   these as 2-char operators (mirrors the existing `->` handling).
4. **Line comments**: `// ...` comments were tokenized as code. Fix:
   detect `//` and skip to end of line.

The KEYWORD-classification bug (`index_of(kws, tok) > -1.0` does substring
matching) is NOT fixed at the lexer level — instead, the parser's `TokKind`
helper re-verifies KEYWORD/IDENT classification via exact-match against the
keyword list. This keeps the keyword list in one place (TokKind) rather
than duplicating it across multiple lexer-level checks.

---

## mlogpkg: dependency resolution + security audit (Naryad #198)

`mlogpkg` is the package manager for METALOGOS projects. As of Naryad #198 it
gains three new capabilities on top of the original `init`/`add`/`build`/`info`
commands:

1. **Full transitive dependency graph resolution** — `mlogpkg build` now
   walks the full dependency tree (not just direct deps), detecting
   version conflicts and cycles.
2. **`mlogpkg.lock` lockfile** — fixes the exact version of every
   dependency (direct + transitive) for reproducible builds. Same
   `mlog.toml` → same `mlogpkg.lock` byte-for-byte.
3. **`mlogpkg audit` command** — checks dependencies against a local
   advisory database of known vulnerabilities (Naryad #198 Block 2).

### Commands

```sh
mlogpkg init [--name NAME]     # create mlog.toml in current dir
mlogpkg add <pkg> [version]    # add a dependency (pre-flight resolves graph)
mlogpkg build                  # resolve full graph, write mlogpkg.lock, check sources
mlogpkg info                   # show project + lockfile status
mlogpkg audit                  # check deps against advisory DB
```

### mlogpkg.lock format

```toml
# This file is automatically generated by mlogpkg.
# Do not edit manually — run `mlogpkg build` to regenerate.
version = 1

[[package]]
name = "alpha"
version = "1.0.0"
source = "registry"

[[package]]
name = "beta"
version = "2.0.0"
source = "registry"
```

Packages are sorted alphabetically by name for determinism. The file is
TOML (consistent with `mlog.toml`), inspired by `Cargo.lock` and
`package-lock.json`.

### Version conflict detection

mlogpkg v1 does NOT support multiple concurrent versions of the same
package. If two dependencies require different versions of a common
transitive dep, `mlogpkg build` and `mlogpkg add` fail with an explicit
error:

```
error: dependency resolution failed: version conflict for 'shared':
'pkg_b' requires version '2.0.0', but 'pkg_a' already requires version '1.0.0'
(mlogpkg v1 does not support multiple concurrent versions of the same package)
```

This is a deliberate simplification — supporting concurrent versions
(cargo's "semver resolution") is a significantly more complex feature and
out of scope for v1.

### `mlogpkg audit` — limitation (Block 2)

⚠️ **The advisory database is LOCAL and MANUALLY MAINTAINED.** It is NOT
an integration with an external CVE database (RUSTSEC, NVD, GitHub Advisory
DB, etc.).

- The bundled DB lives at `mlogpkg/advisory-db.toml`.
- It is updated only when a new release of `mlogpkg` is published.
- Vulnerabilities are added as they are discovered and reported to the
  Metalogos team.
- Version matching is exact (no semver ranges in v1).

To override the bundled DB:

1. Set the `MLOGPKG_ADVISORY_DB` env var to point at a custom `.toml`
   file. Useful for tests and for projects that want to extend the
   bundled DB with project-specific advisories.
2. Or create `~/.mlog/advisory-db.toml` (user-level override).

For real-world security auditing, supplement `mlogpkg audit` with
external tools (e.g. `cargo audit` for Rust dependencies, OS-level
scanners for system packages).

### Registry

The local registry is at `~/.mlog/registry/<pkg-name>/`. Each package is
a directory containing:

- `mlog.toml` — package manifest (name, version, dependencies)
- `src/main.mlog` — source files

Override the registry path with the `MLOGPKG_REGISTRY` env var (useful
for tests).

### Contracts (Naryad #198)

Four integration tests verify the new behavior:

- `tests/naryad_198_dependency_conflict.rs` — two packages requiring
  incompatible versions of a common transitive dep → explicit error at
  `build` and `add` time.
- `tests/naryad_198_lockfile_determinism.rs` — same `mlog.toml` → same
  `mlogpkg.lock` byte-for-byte, in two separate dirs and on rebuild.
- `tests/naryad_198_audit_finds_known_vuln.rs` — `mlogpkg audit` finds
  and reports a known vulnerability from the advisory DB; passes when no
  vuln matches; respects version specificity.
- `tests/naryad_198_backward_compat.rs` — simple projects without
  transitive deps work exactly as before, plus the appearance of
  `mlogpkg.lock`.

---
