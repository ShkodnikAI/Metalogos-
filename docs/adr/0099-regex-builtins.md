# ADR-0099: Regular Expression Builtins (Наряд №54)

## Status: PROPOSED

## Context

The METALOGOS language needs regex support for text processing patterns used in the
FOSVED office (llm_proxy.py uses ~30 regex patterns for TTS cleaning, HTML stripping,
Markdown escaping, date normalization, and JSON extraction).

Prior to this ADR, the language had `contains`, `starts_with`, `ends_with`, `split`,
`replace` — but no pattern matching with character classes, quantifiers, alternation,
or capture groups. Users needing regex were forced to shell out to external tools.

## Decision

### 1. Crate Choice: `regex` (Rust standard)

**Chosen:** `regex = "1"` (crates.io, Rust's standard regex library)

**Alternatives considered:**
- `fancy-regex`: Supports lookahead/lookbehind, but uses backtracking → exponential worst-case
- `pcre2`: C binding, supports full Perl-compatible regex, but adds a C dependency
- Hand-rolled: Too much work, correctness risk

**Rationale:** The `regex` crate provides linear-time matching guarantees (no exponential
backtracking), is pure Rust (no C deps), and covers 90% of real-world regex use cases.
The 10% it doesn't cover (lookahead/lookbehind) have straightforward workarounds
(capture groups, two-pass replace, char-by-char iteration).

### 2. Builtin Functions: Three primitives

| Function | Signature | Returns |
|----------|-----------|---------|
| `regex_match` | `(pattern, text) → Bool` | Full-match semantics (implicit `^...$`) |
| `regex_captures` | `(pattern, text) → List` | `[full_match, group1, group2, ...]` or `[]` |
| `regex_replace` | `(pattern, text, replacement) → String` | All matches replaced; supports `$1`, `$name` |

**Why no `regex_find` / `regex_split`:** Not needed for current office patterns.
Can be added in a follow-up onomat if demand arises.

### 3. Soft-Failure Semantics

Invalid regex patterns must NOT panic. Instead:
- `regex_match(invalid, text)` → `false`
- `regex_captures(invalid, text)` → `[]` (empty list)
- `regex_replace(invalid, text, repl)` → `text` (unchanged)

This is consistent with the interpreter's existing error model where builtin failures
produce degraded results rather than crashes.

### 4. Compilation Cache

Regex compilation is expensive (~microseconds). A global `LazyLock<Mutex<Cache>>` holds
up to 32 compiled patterns in an LRU eviction policy.

**Why LRU?** Office workloads reuse the same patterns repeatedly (HTML stripping, TTS cleaning).
LRU keeps hot patterns compiled and evicts cold ones at capacity.

**Why `Mutex` not `RwLock`?** Regex compilation is fast enough that read-write contention
is negligible. `Mutex` is simpler and has no writer starvation risk.

**Thread safety:** The cache is global (process lifetime), shared across all interpreter
instances and HTTP requests. This is correct because `Regex` is `Send + Sync`.

### 5. Lookahead/Lookbehind Limitation

The Rust `regex` crate does NOT support lookahead (`(?=...)`) or lookbehind (`(?<=...)`,
`(?<!...)`). This is a fundamental trade-off for linear-time matching.

**Crosscheck result (llm_proxy.py, 30 patterns):**
- 27 patterns (90%) work as-is in Rust regex
- 3 patterns (10%) use lookbehind and need workarounds:
  - `(?<=\S)\s*[.!?]\s*[.!?]+` → use capture groups
  - `(?<!\\)([_*...])` → two-pass replace or char iteration
  - `\\(\\[emoji_range])` → literal double-backslash match

This is documented for future porting reference.

## Consequences

### Positive
- Text processing power parity with Python regex for office patterns
- Linear-time guarantee prevents ReDoS attacks
- Zero new C dependencies (pure Rust)
- Compilation cache makes repeated use efficient

### Negative
- No lookahead/lookbehind support (documented, 90% coverage)
- 32-entry cache may be insufficient for highly dynamic pattern sets (unlikely in practice)
- Regex errors are silent (soft-failure) — may hide bugs in user patterns

### Neutral
- Binary size increase: ~+400KB (regex crate + regex-syntax + regex-automata)
- Three new entries in BUILTIN_REGISTRY (indices stable, appended at end)
- Three new entries in registry_arity_check (exhaustive coverage)

## Implementation

Files changed:
- `Cargo.toml` — added `regex = "1"`
- `src/builtins/regex.rs` — new module (3 builtins + LRU cache)
- `src/builtins/mod.rs` — module registration + handler inserts
- `src/builtins/registry.rs` — 3 new `spec!` entries
- `tests/registry_arity_check.rs` — 3 new arity test cases
- `src/builtins/tests.rs` — 12 unit tests (4 match + 4 captures + 4 replace)
- `examples/p54_regex.mlog` + `.expected` — golden contract (6 test cases)
- `examples/p54_regex_crosscheck.txt` — office pattern compatibility analysis

## Verification

- `cargo test --lib test_regex` — 12/12 pass
- `cargo test --test registry_arity_check` — 1/1 pass (includes 3 new)
- `cargo test --test golden all_golden_tests_pass` — p54 passes
- `cargo fmt --check` — clean
- `cargo clippy --lib` — clean (no warnings)
