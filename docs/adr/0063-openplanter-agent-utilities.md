# ADR-0063: OpenPlanter-inspired Agent Utility Builtins

**Date:** 2026-07-21
**Status:** Accepted

## Context

Research of the [OpenPlanter](https://github.com/ShinMegamiBoson/OpenPlanter) repository (MIT-licensed recursive-language-model investigation agent) revealed several agent-utility patterns directly applicable to Metalogos as builtin functions. OpenPlanter is a Rust + Python project for OSINT-style data analysis with recursive sub-agent delegation, wiki knowledge graphs, content-verified editing, and budget-aware execution.

## Decision

Add 8 new builtins to `src/builtins.rs` across 4 categories:

### Fuzzy Matching (strsim / Jaro-Winkler)

| Builtin | Arity | Description |
|---------|-------|-------------|
| `fuzzy_match(a, b)` | 2 | Jaro-Winkler similarity score (0.0–1.0) |
| `fuzzy_find_best(query, candidates)` | 2 | Best match from candidate list → `FuzzyMatch{index, candidate, score}` struct |

Ported from OpenPlanter's `wiki/matching.rs::NameRegistry` which uses `strsim::jaro_winkler` with 0.85 threshold for entity resolution across heterogeneous datasets.

### Content-Verified Editing (CRC32 Hashlines)

| Builtin | Arity | Description |
|---------|-------|-------------|
| `hashline_read(text)` | 1 | Annotate each line with 2-char CRC32 hash prefix: `N:HH\|content` |
| `hashline_edit(text, edits)` | 2 | Hashline-verified editing with 3 ops: `set_line`, `replace_lines`, `insert_after` |

Ported from OpenPlanter's `tools.py::_line_hash()` and `hashline_edit()`. The CRC32 hash (whitespace-normalized, masked to 2 hex chars) prevents LLM agents from editing stale content when line numbers shift. Hash mismatch returns an error instead of silently corrupting data.

**Algorithm:**
1. `compute_line_hash(line)`: normalize whitespace → CRC32 → mask to `0xFF` → 2-char hex
2. `parse_line_ref("N:HH")`: split on `:` → validate line number + 2-char hash
3. Edit operations verify hash before applying changes

### Agent Utilities

| Builtin | Arity | Description |
|---------|-------|-------------|
| `compact_list(items, keep_first, keep_last)` | 3 | Context condensation: protect head/tail, collapse middle into `Compacted{compacted: true, removed_count: N}` |
| `budget_check(step, total_steps)` | 2 | Budget awareness → `BudgetStatus{step, total, remaining, pct_remaining, level}` where level is "ok" (≥50%), "warning" (≥25%), "critical" (<25%) |
| `replay_snapshot(data)` | 1 | Delta-encoded replay log helper → `ReplaySnapshot{seq: 0, count, snapshot}` (seq 0 = full snapshot, caller tracks delta on subsequent calls) |
| `policy_check(command)` | 1 | Shell command safety → `PolicyResult{allowed, reason}`. Blocks heredoc (`<<`) and interactive TUI programs (vim, nano, less, etc.) |

### Design Decisions

1. **Pure functions, no side effects.** All builtins follow the existing pattern of `fn(args: &[Value]) -> Result<Value, String>` with no mutable state. The KV store is not accessed; callers persist results as needed (same pattern as `recipe_save` in ADR-0062).

2. **Struct returns over tuple returns.** Multi-field results use `Value::Struct` with named fields (e.g., `FuzzyMatch.score`, `BudgetStatus.level`) for readability in .mlog programs.

3. **`compact_list` returns a new list** rather than mutating in place, consistent with Metalogos's functional style.

4. **`policy_check` is a pure validator** that returns a struct rather than throwing an error, allowing callers to decide how to handle blocked commands.

5. **`replay_snapshot` only creates seq-0 (full snapshot).** Delta encoding for seq 1+ is left to the caller (track previous count, slice new items). This keeps the builtin simple while enabling the full pattern.

## New Dependencies

- `strsim = "0.11"` — Jaro-Winkler similarity (MIT)
- `crc32fast = "1.4"` — Fast CRC32 hashing (MIT)

## Tests

20 unit tests added covering:
- `fuzzy_match`: identical, similar, different strings
- `fuzzy_find_best`: best match selection, empty list
- `hashline_read`: annotation format, empty input
- `hashline_edit`: set_line with hash verification, hash mismatch detection
- `compact_list`: no-compaction-needed case, middle compaction
- `budget_check`: ok/warning/critical levels, zero-total error
- `replay_snapshot`: basic snapshot creation
- `policy_check`: allowed commands, heredoc blocking, interactive blocking, whitespace trimming

## Limitations & Deferred Items

- `fuzzy_find_best` scans linearly; for large registries, a SQLite-backed index would be better (similar to OpenPlanter's `NameRegistry` + `strsim` combo)
- `hashline_edit` operates on in-memory strings; file I/O versions would need filesystem access
- `policy_check` does not track repeat command frequency (OpenPlanter's `_runtime_policy_check` blocks identical commands run >2 times at the same depth)
- No `compact_messages()` automatic integration in the VM evaluation loop (would require modifying the VM, not just adding builtins)
- `replay_snapshot` lacks the `child()` nesting pattern from OpenPlanter's `ReplayLogger`

## Examples

`examples/openplanter_demo.mlog` — demonstrates all 8 builtins.