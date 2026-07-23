# ADR-0064: Lifecycle Hooks Expansion (2 → 5)

**Status:** Implemented
**Date:** 2026-07-23
**Work Order:** O-2

## Context

Metalogos v0.10.0 had 2 lifecycle hook points (ADR-0045): `before_pattern` and `after_pattern`. These fire around every user-defined pattern invocation. However, many cross-cutting concerns need different trigger points:

- **Session initialization**: logging session start, loading initial state, setting up counters
- **Write auditing**: tracking all mutations (mem_set, mtree_store, db_execute, write_file, append_file)
- **Session cleanup**: flushing logs, emitting final metrics, cleanup on exit

[obsidian-mind](https://github.com/breferrari/obsidian-mind) (TypeScript, 3.5k★, MIT) implements 5 lifecycle hooks (on_session_start, on_write, on_session_end, before_step, after_step). Its hook pipeline provides a proven model for agent lifecycle management.

## Decision

Expand `HookPhase` from 2 to 5 variants:

| Hook | Trigger point | Variables | Analogue in obsidian-mind |
|------|--------------|-----------|--------------------------|
| `on_session_start` | Beginning of `run()`, after all declarations registered | (none) | `on_session_start` |
| `on_write` | Before every write builtin call | `target` (String), `args` (List) | `on_write` |
| `before_pattern` | Before pattern invocation (unchanged) | `pattern_name`, `args` | `before_step` |
| `after_pattern` | After pattern returns (unchanged) | `pattern_name`, `args`, `result`, `confidence` | `after_step` |
| `on_session_end` | End of `run()` | (none) | `on_session_end` |

### Implementation

- `on_session_start` uses a two-phase `run()`: first pass registers all hooks, fires session_start, then processes remaining declarations.
- `on_write` fires via `fire_on_write_hooks()` helper, called before write builtins at all 3 builtin dispatch sites (invoke, QualifiedCall, flow-step FnCall).
- Write builtins: `mem_set`, `mtree_store`, `db_execute`, `write_file`, `append_file`.
- All hook errors are silently ignored (advisory, not blocking) — consistent with ADR-0045.

### Grammar

New tokens: `ON_SESSION_START_KW`, `ON_WRITE_KW`, `ON_SESSION_END_KW`. Added to `step_ident` negative lookahead to prevent conflict with flow step names.

## Consequences

- **Positive:** Full lifecycle coverage for cross-cutting concerns. Session-level hooks enable initialization/cleanup patterns. Write hooks enable audit trails without per-call boilerplate.
- **Negative:** `on_session_start` requires a two-pass approach in `run()`, adding O(n) overhead for the pre-pass (negligible).
- **Neutral:** Backward compatible — new hook types are opt-in. Existing `before_pattern`/`after_pattern` hooks work unchanged.
