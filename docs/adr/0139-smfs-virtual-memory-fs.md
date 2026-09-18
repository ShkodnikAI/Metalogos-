# ADR-0139: SMFS — memory export as a virtual read-only FS (`sm:`)

**Status:** Proposed (draft of spike #282; spike verdict — GO)
**Date:** 2026-09-13
**Naryad:** #282 (spike, issue #331; prototype — draft PR #349, branch `naryad-282-smfs-profile`, not merged into main)
**Related:** ADR-0041 (memory persistence), ADR-0093/0094 (memory typology / type-aware recall), #281 (user_profile), ADR-0131 (diagnostic codes), ADR-0136 (redact), #284 (canary), #252/#254 (sandbox), ADR-0096 (concurrency)

## Context

supermemory SMFS confirms the "memory as a filesystem" niche: the memory container is mounted as a directory, with a virtual read-only `profile.md` at the root ("cat profile.md" instead of walking the files), and a claimed ~3× token saving versus traversal. In Metalogos the container profile already exists — the deterministic `user_profile` (#281) — but it is reachable only by a direct builtin call, and its Struct form does not fit into file-oriented scenarios (templates, LLM context "as if from files", navigation scripts).

Spike #282 (report `docs/research/naryad-282-smfs-spike.md`) verified on a prototype: whether the language's memory can be served through the **existing** file builtins without extending the sandbox per #131/#252/#254 — and whether this yields a measurable context reduction.

## Decision (design for production implementation)

1. **The virtual `sm:` namespace** — read-only export of kv memory on top of `user_profile` (#281):

   | Path | Operation | Result |
   |---|---|---|
   | `sm:` | `list_dir` | mounted DBs (in production — a registry of mount points, not a root scan) |
   | `sm:<db>` | `list_dir` | containers |
   | `sm:<db>/<container>` | `list_dir` | `profile.md` + buckets |
   | `sm:<db>/<container>/profile.md` | `read_file` | deterministic digest (trimming + pointers to the full records) |
   | `sm:<db>/<container>/<bucket>/<key>` | `read_file` | the full record value |

2. **Read-only mounting**: `write_file`/`append_file`/`delete_file` on `sm:*` — a loud `[SMFS_READ_ONLY]` error (an ADR-0131 code). Writing to memory remains with `memorize`/`kv_set` (a single writer; sugar writes — a separate decision, out of scope).

3. **Two-way prefix reservation**: `sm:*` paths are handled only virtually (intercepted before `sandbox_path`); real `sm:*` files are unreachable and uncreatable through the builtins. `..` — a loud `[SANDBOX_VIOLATION]`; invalid forms — a loud `[SMFS_BAD_PATH]`/`SMFS_IS_DIR`/`SMFS_IS_FILE`.

4. **The sandbox is not extended**: virtual paths do not touch disk (except the normal sandboxed opening of the profile DB); an active sandbox `forbidden=["filesystem"]` takes precedence over the interception — virtual access counts as filesystem access.

5. **Determinism**: the digest is rendered without an LLM (reuses the #281 cache); an LLM summary is a separate production loop (streaming #275, bounded-per-chunk), not part of this decision.

## Consequences

- **Positive**: the spike measurement — context reduction **3.66×** versus traversing all records (Go criterion ≥ 2×); a "memory-native filesystem" narrative for the grant; memory access from any file scenario without new builtins; registry/arities/bytecode indexes unchanged.
- **Positive**: canary detection (#284) survives through the export (prototype test); the export creates no new taint channel relative to `user_profile`.
- **Negative/limits**: the prefix reservation makes real `sm:*` files unreachable through the builtins (backward compatibility — a deliberate price, announced loudly); the digest parameter (160 chars) requires tuning; no gain on memory made of short records; the token proxy chars/4 without a tokenizer.
- **Follow-ups**: a registry of mount points (replacing the root scan); taint marks on records created from tainted expressions; a follow-up hotfix for the `eval_expr`-path sandbox gate (a spike finding, §5 of the report — a pre-existing gap, fixed on the spike branch).
- **Revisit point**: production writes appearing through memory-paths (SMFS semantics "a write generates a memory") — requires a separate ADR with cache/ledger/taint gates.
