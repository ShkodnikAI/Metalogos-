# ADR-0176: The plan-v2 memory contract — the SSOT of the memory surface

**Status:** Accepted — Variant B (the minimal contract). The owner's
decision of 2026-09-25 ("Variant B") was received in the office chat
(trace `1a0d69cf6e3a0925`) and relayed into the issue #662 thread
(comment 5826252041); it matches the executor's recommendation (§3).
**Date:** 2026-09-25
**Naryad:** #453 (issue #662; Wave 15 addendum, the audit 24.09 P1-4)
**Pillar:** Phase 4 memory (cross-cutting: the typed lane, consent, the
poison cascade, the ledger family)
**Consumers:** REALITY §6.9/§6.10 (the Memory row — the last named gap of
the row is THIS decision), docs/limitations.md (the Memory boundaries row
syncs after the decision), №442/№445 (the front doors — unchanged), the
next recount (Wave 16)

## 1. Context

The audit 24.09 (P1-4) named the residue of the Memory row: the plan-v2
memory CONTRACT is unknown because plan v2 is absent from the repository —
a decision/SSOT gap, not a code gap (REALITY §6.9, the №446 row: "the
plan-v2 memory contract is still unknown (plan absent) — the ONLY remaining
named gap of the row"). The code substrate is real and CI-pinned:

- the typed `Memory<K>` lane with consent fail-closed (№350/№413) and the
  consent revocation cascade;
- the FTS5 + vector retrieval integration behind the `recall` front door
  (№442 — consent-gated, provenance-bearing, ledgered);
- the derived→poisoned cascade with the retained VETO (№351/ADR-0173) and
  the quarantine sink-gates (`MEMORY_POISONED`);
- the TTL auto-forget sweep and the ACT-R decay/boost ranking (№445,
  `memory_retain_ttl` — the canon `retain(memory, ttl)`);
- the `[MEM]` provenance stamps on every read surface;
- the `memory.*` ledger family + `irreversible.memory_forget` (№393/№415,
  the ADR-0167 append-only discipline).

The canon (plan v2) is held by the coordinator in the private office repo
(§16.0-7); this repository cannot contain it. The gap is therefore not
"write the plan" — it is "fix the memory contract at the level this
repository can honestly speak", in one of two forms. The naryad fixes the
frame: the memory CODE does not change (the substrate is real); the P2
parks stay parked; the office canon is not touched by the executor; the
variant is chosen by the OWNER in the issue thread.

## 2. Variant A — explicit defer

**Decision (A):** the plan-v2 memory contract is OUT OF SCOPE until the
owner explicitly returns to it. This ADR records the deferral itself as
the decision: the SSOT question is answered with "deliberately open, owned
by the coordinator".

**Return conditions** (what lifts the defer):
1. plan v2 (or its memory section) lands in the repository in any form;
2. OR the owner issues a naryad to fix the contract independently of the
   plan (this ADR then becomes the placeholder that naryad supersedes).

**Consequences:**
- REALITY: the Memory row's named gap closes as "resolved-deferred" — the
  decision EXISTS (the ambiguity is gone), the boundary is documented in
  limitations (the memory surface is defined by the shipped code + the
  P2 parks; no plan-level contract is claimed).
- limitations: the Memory boundaries row gains "the contract is
  deliberately deferred (ADR-0176 variant A) — the shipped behavior is the
  de-facto contract until the plan lands".
- Risks: the de-facto contract drifts as waves add surfaces (recall
  extensions, new ledger families) with nothing to diff against; the %
  credit rests on a negative decision (auditors may read a deferred
  contract as an unresolved one).

**Honest % arithmetic (protocol №318 — proof at the next recount):** the
Memory row 70 → **80** (the last named gap closes as a decision — the same
~10pp per named gap granularity §6.9 used: 60 → 70 for the third of four);
the total +20% × 10pp = **+2.0pp → 85 → 87%**. The movement is
reproducible: `grep -n "ADR-0176" docs/limitations.md` (the boundary row)
+ the REALITY row citing this ADR as the decision anchor.

## 3. Variant B — the minimal contract (RECOMMENDED)

**Decision (B):** the repository fixes the memory contract EXPLICITLY, at
the level of what is shipped and CI-pinned today. The SSOT statement:

1. **The single memory SSOT is the typed `Memory<K>` lane** (№350) —
   `memory_put`/`memory_read`/`memory_keys`/`memory_export`/`forget`/
   `memory_retain_ttl`/`recall`/`recall_top_k`. The legacy session-memory
   call forms (`memorize("fact", priority)` / `forget(query, days)`, №72)
   remain supported as the compat surface; the KV call form
   (`memorize(key, value)`, the №386/№405 vocabulary) aliases `kv_set`.
2. **Consent is fail-closed on every read/export surface** (№413):
   `MEMORY_RECALL_CONSENT_REQUIRED` on recall, `MEMORY_FORGET_CONSENT_REQUIRED`
   on forget — a refusal is a ledger record, never a silent skip.
3. **The poison cascade is the integrity backbone**: forget poisons the
   derived closure (ADR-0173), poisoned values materialize nowhere
   (`MEMORY_POISONED` on read/export; the recall lane skips quarantine);
   a re-grant never resurrects quarantined content.
4. **Forgetting is real**: TTL auto-forget (the №445 sweep,
   `memory.ttl_expired`) + the ACT-R activation ranking (base × priority ×
   exp(−decay × staleness)) in the typed lane; `retain(memory, ttl)` is the
   canon primitive.
5. **Provenance**: `[MEM]` stamps on reads; the `memory.*` ledger family
   (put/read/recall/forget/denied/ttl_expired/retain_ttl) per ADR-0167.
6. **Boundaries (deliberately NOT in the contract)**: the static poison
   lattice and points-to (P2-1 park); `find`/`inspect` (named stubs); the
   real-weights lines (hardware gate); the vector store as an independent
   SSOT (it serves recall through the typed lane, not beside it); the
   FTS5/InMemory engines are storage details behind the lane interface —
   swappable without a contract change.

**Consequences:**
- REALITY: the Memory row's named gap closes as "the contract exists" —
  the same 70 → **80** row movement, total 85 → **87%** (the same +2.0pp;
  the honest arithmetic is identical to variant A at the next recount —
  the difference is NOT the number, it is what backs it).
- limitations: the Memory boundaries row syncs to cite the contract
  sections (each boundary line gets its ADR anchor).
- Future waves gain a diff target: any memory-surface change reviews
  against §3's six points instead of against silence.
- Risks: the contract could ossify a moving surface (mitigated: the six
  points name interfaces and guarantees, not implementations — the
  FTS5/InMemory engines are explicitly declared swappable); the vector
  lane's "serves through, not beside" wording may need revisiting if a
  future wave makes vec_search a first-class SSOT (named as an open
  question below).

**Why B is recommended:** the substrate is real, CI-pinned and
mutation-verified (the №445/№442 contract tests); a contract that documents
what EXISTS is the honest artifact, while a defer leaves the row's %
backed by a negative. The naryad's own risk table hints the same: variant
B's "risks" are documentation risks, variant A's are accounting risks.

## 4. Open questions to the owner (resolved with the decision)

The gate asked for the variant choice; the owner's decision ("Variant B",
the office chat 2026-09-25, relayed as the issue #662 comment 5826252041)
did not contradict the stated defaults, so each question is resolved
conservatively to this ADR's own default:

1. **A or B** — resolved: **B** (the owner's decision).
2. (B only) the vector lane — resolved to the §3 default: "serves through
   the typed lane, not beside it" (№442 built `recall` strictly through
   the lane); naming `vec_search` a second, parallel SSOT stays a
   future-wave decision (named in §3 risks).
3. (B only) the compat call forms — resolved to the §3 default: they stay
   OUTSIDE the six numbered points, described inside point 1 as the
   compat surface (exactly as shipped today).
4. The recount timing — resolved to the naryad canon: the §6.9 named gap
   closes at the NEXT recount (Wave 16, after №450); REALITY is not
   edited by this decision.

## 5. Decision record

- Chosen variant: **B — the minimal contract** (the owner's decision,
  2026-09-25)
- The issue comment anchoring the decision: issue #662 comment 5826252041
  (the chat decision, trace `1a0d69cf6e3a0925`, relayed by the executor)
- The limitations sync commit: this PR's squash (PR #668) — the Memory
  boundaries row lands in the same merge
- The recount section that credits the movement: the Wave-16 recount
  (REALITY §6.9 — the last named gap of the Memory row closes there)
