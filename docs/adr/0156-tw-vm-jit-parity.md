# ADR-0156: TW/VM/JIT label parity — LabelJoin/SinkCheck in the bytecode

**Status:** Accepted
**Date:** 2026-09-15
**Naryad:** №328 (issue #422, plan v2 §16.3 / Волна 1 · Фаза 1)
**Supersedes:** the reserved stub written by №319 (booking note in git history)

## 1. Decision

The static gates (№323 inference, №325 sink clearance, №327 decisions) are the SSOT of every verdict. №328 lowers that knowledge into the bytecode so the VM runtime twin agrees by construction and any divergence is a loud error:

- `Instruction::LabelJoin { dst, src }` — joins the runtime label of `src` into `dst` (componentwise, ADR-0154 §2.4). A `src` starting with `@` names a №316 Source builtin; the VM seeds the runtime label from the same mapping the static pass uses (Secret sources → `(private, trusted)`, every other source → `(public, untrusted)`).
- `Instruction::SinkCheck { fn_name, arg, line }` — the runtime twin of the №325 gate: the runtime label of `arg` must clear the sink (`public`; exec additionally refuses untrusted). A violation is a distinct runtime error `[SINK_CLEARANCE_RUNTIME]` + an audit event line on stderr.

The compiler lowers source-backed `let`/assignments into `LabelJoin` and every sink call site (identifiers and direct-source arguments) into `SinkCheck` — in both compile paths (top-level statements and pattern bodies).

## 2. The dispatch-gap rule

The JIT compiler is not in the tree yet. `bytecode::is_jit_eligible` is the SSOT predicate the future dispatcher must consult: label instructions are explicitly OUTSIDE the JIT-eligible class (arithmetic-only). When a JIT dispatcher appears, it is required to reject label-bearing functions with a distinct error naming this ADR — never to skip them silently. Pinned by test.

## 3. Parity matrix (Фаза 1)

| Verdict | TW (run) | VM (compile+run) | JIT |
|---|---|---|---|
| Static gate verdicts (№325/№327) | Error at compile | Error at compile | label fns excluded from the eligible class — explicit reject |
| Runtime label tracking | env-tracked by the №323 contracts | `LabelJoin` over the runtime label env | not eligible (explicit error, future) |
| Sink verdict at runtime | via the same compile gate | `SinkCheck` runtime twin, loud divergence error | not eligible (explicit error, future) |

Golden verdicts: the run and compile paths agree on both rejecting and accepting programs (pinned by test).

## 4. Verification (№328)

`cargo test naryad_328` — 8 tests: the dispatch-gap predicate, the VM runtime twin, runtime source labels matching the static mapping, the componentwise join, golden run/compile verdict agreement, compiler emission into the bytecode, no-stub grep.
