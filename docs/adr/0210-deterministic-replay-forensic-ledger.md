# ADR-0210: Deterministic Replay & Forensic Ledger — the Decision Trace as a Typed, Append-Only, Verifiable Artifact (the 8G / C8 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-80 acceptance —
  the dispatch gh#1779 issued the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-04
- Deciders: the OWNER (the R38 train — 0.43 = 8G (C8, REPLAY_MATCH
  ≥ 10000 bp); the wave-80 dispatch gh#1779; the «Почисти если нужно
  хвосты и заливай следующую волну» directive of 2026-11-03)
- Naryad: №891 (issue #1773, Wave 80 — W-I; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38 — the C8 narrowing
  decision R38(5): the fact is REPLAY_MATCH, the deterministic
  DECISION TRACE — «побитовое воспроизведение» is struck as
  unfulfillable, the honest-limits protocol №588); the R58 GET-only
  fact-check over live main `d309cd6` (2026-11-04)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.43 is this record's train slot),
  [ADR-0167](0167-ledger-dsse.md) (the Action Ledger and its DSSE
  profile — the decision trace LIVES IN the Ledger ACTION contour and
  never builds a parallel store), [ADR-0208](0208-provider-agnostic-local-first.md)
  (the provider-agnostic contract — the model/provider VERSIONS a
  trace pins are read through the ProviderContract, №879),
  [ADR-0169](0169-stable-try-codes.md) (the frozen 14-word refusal
  vocabulary), [ADR-0204](0204-portable-refusal-algebra.md) (the
  refusal algebra — the deny causes a trace records),
  [ADR-0203](0203-capability-token.md) (the rights SSOT — grant
  references), [ADR-0202](0202-model-passport.md) (the passport — the
  model identity seam), [ADR-0207](0207-provenance-native-io.md) (the
  I/O provenance — the ADJACENT contour this record never duplicates),
  [ADR-0209](0209-jurisdiction-aware-policy-compilation.md) (the
  jurisdiction kernel — a policy-compile decision is also a decision),
  №385 (the frozen refusal vocabulary), №828/№868 (the digest-pinning
  and canonical-row forms), №874 (the fire_on_deny form),
  №885–№890 (the wave-79 kernel form this record follows),
  №892–№896 (the naryads that land on top of this record)

## §1. Context — the decision facts exist, but the DECISION TRACE as an artifact does not

The forensic substrates are alive on main (verified R58, `d309cd6`):
the Action Ledger writes ACTION rows through its verify+DSSE contour
(`src/ledger.rs`, ADR-0167), the refusals carry the frozen 14-word
DenyEvent vocabulary (№385/ADR-0169), the degradation facts are typed
(`Degraded(t)` — `src/builtins/backends.rs:58/230/263`), the
model/provider versions live in the ProviderContract (№879, ADR-0208),
and the grant references are typed (ADR-0155). The wave-79 kernel
added the latest decision producer: the jurisdiction policy
compilation (`src/jurisdiction.rs`) — whose allow/deny/limit
compilation is itself a decision.

The honest gap this record closes:

- **There is no typed DECISION TRACE.** The facts a forensic auditor
  needs — WHAT was decided, on WHICH inputs (by digest, not raw
  content), under WHICH model/provider versions, with WHICH grant,
  with WHICH refusal/degradation, ending in WHICH recorded output —
  exist only as scattered rows in different contours. Nothing in the
  type system says what a decision trace IS; an artifact assembled by
  hand from five contours cannot be digest-pinned, cannot be appended
  honestly, and cannot be verified against what was actually recorded.
- **The replay expectation is dishonest if stated as bit-for-bit.** A
  model cannot be replayed bit-for-bit when the provider deprecates
  weights (the honest-limits protocol №588; the R38(5) revision struck
  the bit-for-bit claim). The honest contract is REPLAY_MATCH: the
  TRACE of the decision is deterministic and verifiable — the same
  inputs, the same versions, the same grant, the same refusal class,
  the same recorded output digest — while the raw model output bytes
  are explicitly OUT of scope.
- **The coverage is not fail-closed.** Nothing refuses an irreversible
  decision made WITHOUT a written trace; the coverage claim would be
  a declaration, not a machine check.

## §2. Decision — the trace is typed data; the log is append-only; the verification is a comparison, not a reproduction

The C8 record adopts four decisions:

1. **The trace is a typed artifact.** `DecisionTrace` is a struct:
   the inputs (digests/identifiers — NEVER raw prompts or payloads:
   the privacy boundary), the model/provider versions (read through
   the ProviderContract — №879), the grant references (ADR-0155), the
   refusals (the frozen №385 classes), the degradations
   (`Degraded(t)` markers), the recorded outputs (digests), the
   timestamp. Every field is part of the canonical row; the canonical
   row is digest-pinned (the №828/№868 form — SHA-256 over the
   `\x1f`-joined fields).
2. **The log is append-only and lives IN the Ledger contour.**
   `DecisionTraceLog` follows the AttestationMap discipline
   (ADR-0206 §2.2): issuing adds, nothing deletes; a supersede is a
   NEW row. The trace rows ride the EXISTING Ledger ACTION contour
   (ADR-0167) — this record consumes the Ledger, never builds a
   parallel action store (the dedup guard).
3. **The verification is a COMPARISON.** `verify(trace, log)` checks
   the trace against what was recorded — the inputs match, the
   versions match, the grant matches, the refusal class matches, the
   output digest matches. It never re-runs the model and never claims
   byte-reproduction of an LLM answer (the №588 honest boundary).
4. **The coverage is fail-closed at the consumption site.** An
   irreversible decision WITHOUT a live trace refuses BEFORE the
   effect and journals a Ledger deny row (the №874 fire_on_deny form,
   the existing refusal classes — №385 stays frozen).

## §3. The parents are consumed by reference — what this record does NOT redefine

- `ledger.rs` (ADR-0167) stays the SSOT of the action store; the
  trace rows ride its ACTION contour and its verify discipline; this
  record adds no second store.
- The ProviderContract (№879/ADR-0208) is the only version source; a
  trace never hard-codes a model name.
- The refusal vocabulary stays frozen (№385/ADR-0169, 14 words); the
  trace records CLASSES, never mints new ones.
- The egress provenance gate (ADR-0207, wave 77) stays the I/O
  boundary contour — provenance records WHAT crossed the boundary,
  the decision trace records WHY the decision happened; adjacent, not
  duplicate.
- The capability/grant SSOT (ADR-0203/ADR-0155) is referenced by id —
  never re-decided by the trace.
- No new crates; the kernel uses the crate's own `serde`/`sha2`.

## §4. The consumption contracts — what №892–№896 land

- **№892 (the kernel):** `src/decision_trace.rs` — `DecisionTrace`,
  `DecisionTraceLog` (append-only, the digest-pinned canonical rows),
  `verify` (the comparison contract), the Ledger consumption ride;
  the negative controls (a trace without inputs refuses; a trace
  without versions over an LLM carrier refuses; a verify against a
  substituted output refuses) — the double-run byte-identity.
- **№893 (the coverage rule):** the fail-closed consumption checks
  (an irreversible decision without a live trace refuses + the Ledger
  deny row — fire_on_deny №874) + the e2e trace action → record →
  verify on the language roads (TW + VM where the carrier passes the
  language) — the double-run byte-identity.
- **№894 (the machine fact):** `REPLAY_MATCH` in `scripts/gen_metrics.py`
  + the `gate_041` floor (10000 bp, only-up, the floor-twin + the
  tamper traps — the gate_038–040 лекало); the share of the
  registered decision carriers whose traces are WRITTEN and
  VERIFIED — the wiring proof is the source presence, the count is
  CHECKED, not declared.
- **№895 (the re-pin ride):** the five surface-gate baselines re-verify
  under the replay types (the №863/№889 form; an honest no-drift is a
  valid verdict).
- **№896 (docs, strictly last):** the weekly wave record + the
  sanitized 0.43 phase line + the restored parked-features RED record
  (the honest finding of the 04.11 consolidation — the ADR-0199 §4
  rule line missed in the 0.38.1 window).

## §5. Honest limits — what the decision trace is NOT

- **Not bit-for-bit replay.** The trace proves the DECISION was made
  on the recorded inputs/versions/grant and ended in the recorded
  output digest; it does NOT reproduce model bytes (№588 — the model
  cannot be replayed bit-for-bit when weights deprecate).
- **Not a prompt store.** The inputs are digests/identifiers; the raw
  prompt/payload stays OUT of the trace (the privacy boundary; the
  redaction SSOT stays elsewhere).
- **Not a second action store.** The Ledger (ADR-0167) stays the SSOT
  of actions; the trace rides its contour.
- **Not a new refusal vocabulary.** The trace records the existing
  classes; №385 stays frozen.

## §6. The migration discipline — 1:1, byte-identical surfaces

The grammar, the builtin registry and the golden files do not change:
a program that compiled and ran before the decision-trace layer
compiles and runs identically after it. The kernel is a pure library
module + the Ledger consumption ride; the fail-closed checks land at
the consumption site (№893) and refuse loudly, pre-effect, naming the
missing trace. The double-run serialization byte-identity is the
acceptance form of №892/№893.

## §7. Acceptance

- ADR-0210 lands as Proposed and is fixed Accepted by the wave-80
  acceptance (the dispatch gh#1779 acceptance comment).
- №892–№896 land green on the merge commits (the blocking CI on every
  merge, the gh#1599 rule).
- `REPLAY_MATCH` in `gen_metrics --check` ≥ 10000 bp (only-up) with
  the tamper traps alive (the gate_041 record).
- The coverage is machine-true: every registered decision carrier's
  trace is written AND verified (the fail-closed checks of №893).
- The surfaces stay 1:1: no grammar, registry, or golden change; the
  refusal vocabulary stays at 14 words (№385); the Ledger stays the
  single action store.
