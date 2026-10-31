# ADR-0205: Typed Approval — the Human Mandate as a Typed Carrier of Consent and Oversight (the 8B-2 / C11 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-75 acceptance — the
  dispatch gh#1699 issued the subphase's naryads the same week; the wave
  acceptance fixes the ADR status as Accepted)
- Date: 2026-10-31
- Deciders: the OWNER (the R38 revision (2) — C11 raised into the 8B
  stream as the 0.38 train slot with the APPROVAL_COVERAGE 10000 bp
  floor; the wave-75 dispatch gh#1699)
- Naryad: №859 (issue #1693, Wave 75 — W-D; ADR before the first code of
  the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R53 GET-only
  fact-check over live main `8e40e02e` (2026-10-31)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.38 is this record's train slot),
  [ADR-0203](0203-capability-token.md) (the capability-token — the grant
  contour this record rides OVER, never instead),
  [ADR-0155](0155-grant-algebra.md) (the grant algebra — the parent
  runtime), [ADR-0204](0204-portable-refusal-algebra.md) (the portable
  refusal algebra — the verdict value this record returns, never
  extends), [ADR-0167](0167-action-ledger-v1.md) (the Action Ledger v1 —
  the trace binding this record must not perturb),
  [ADR-0197](0197-vm-single-executor.md) (the TW/VM parity posture — the
  №829 lesson), №392 (the DenyEvent surface — the frozen 14-word
  vocabulary), №588 (the honest-limits protocol — the form this
  record's §5 follows), №845 (the coverage rule — the fail-closed gate
  form), №860–№864 (the naryads that land on top of this record)

## §1. Context — the approval surface lives, but the mandate is not a type

The human-oversight parents are alive on main and stay untouched:

- the route-level access control `requires=[admin]`
  (`src/ast.rs:621`, the `RouteDecl` doc comment — the parse tests pin
  the grammar): a coarse, per-route attribute at the HTTP boundary;
- the typed degradation `Degraded(t)` (`src/builtins/backends.rs:58`
  and `:230`, the №336/ADR-0165 lineage): a backend ladder exhaustion
  returns a typed, inspectable value — never a panic, never a silent
  mock;
- the honest-limits protocol (№588, the ADR-0125/№241 form): every
  capability page carries what the surface does NOT do;
- the capability line 8B-1 ([ADR-0203](0203-capability-token.md),
  `src/capability.rs`; the grant contour `src/grants.rs`): the typed
  carrier over the grant algebra, the fail-closed gate on irreversible
  carriers (№845), the GRANT_COVERAGE machine fact with its 10000 bp
  floor (№846, gate_034).

The approval surface exists too — as verbs, not as a value: the builtin
`ask_approval` (`src/builtins_classification.rs:420` — Sink / Network /
Irreversible, «sends an approval request to the human»; the registry
spec `src/builtins/registry.rs:588`) and the office goal status
`awaiting_approval` (`src/builtins/office/goals.rs:229`). What the
human answered, there is no place to hold: no actor, no digest of what
exactly was approved, no validity window, no revocation, and nothing
that can fail-closed an irreversible carrier on «no live mandate».

The mandate AS A TYPE does not exist: `TypedApproval`,
`typed-approval`, `APPROVAL_COVERAGE` — 0 hits on `8e40e02e`. The
coverage machinery for the consent layer has the same shape the
capability line had before №843: real parents, no typed contract, no
counted fact — so nothing can hold a floor.

R38 (2) raised C11 into the 8B stream: **0.38 = 8B-2 (C11, typed
consent & human oversight, APPROVAL_COVERAGE 10000 bp)** — one subject
domain with the capability line, a layer over grants, not a parallel
one. The predecessor gate is GREEN on main (SEMA_EXTERNAL = 3,
gate_035, the 0.37 subphase record accepted) — the subphase may open
(the №630 pattern).

## §2. Decision — the mandate is a type, the consent is data, the refusal is algebra

1. **The mandate is a typed carrier of the human consent/oversight
   decision.** It carries: who approved (the `actor`), what exactly was
   approved (the `target_digest` — the digest of the goal/action the
   approval binds), when it was granted (`granted_at`), how long it
   lives (`expires_at`) — and the revocation state. The exact Rust
   shape is №860's; this record fixes the field SEMANTICS.
2. **The mandate is a condition of action for a grant of the
   human-oversight class — NOT a new grant mechanism and NOT a
   replacement.** The grant contour (ADR-0155/ADR-0203) stays the
   single rights machinery: `grant_issue` / `grant_revoke`, the
   once/n/unlimited classes, the TTL, the attenuation law. Where a
   grant answers «may this capability act», the mandate answers «did
   the human consent to THIS action». A grant without a mandate is a
   right nobody asked the human about; a mandate without a grant is
   consent nobody can act on — where both apply, the gate demands both
   (the composition form of №839: the license class × the profile
   permit).
3. **`requires=[admin]` stays the route-level attribute** — the coarse
   access control at the HTTP boundary. The mandate does not rewrite
   it and does not move into it; the mandate is the in-language,
   per-action consent record beneath the route gate (the №861
   boundary: no route rewrite).
4. **Oversight under degradation follows the `Degraded(t)` posture:**
   a consent refusal is a typed, loud VALUE — the same «typed result,
   never a panic, never a silent mock» discipline the backend ladder
   already runs (№336/ADR-0165).
5. **The refusal is a value of the W-C algebra**
   ([ADR-0204](0204-portable-refusal-algebra.md)): the mandate's
   `verify` returns `Allow | Deny { code, cause }`; the cause class
   comes from the frozen 14-word vocabulary — an irreversible action
   without a live mandate is the existing `IRREVERSIBLE_NO_GRANT`
   class (the live dogfood refusal of №856). The algebra is CONSUMED,
   never extended: no new codes (№385, ADR-0169 — the vocabulary is
   frozen).
6. **Fail-closed (the №845 posture):** a human-oversight carrier
   without a live mandate refuses BEFORE the action runs; a silent
   pass is forbidden. The composition of the human-oversight class is
   an explicit registry transaction in the PR that lands it — no
   silent shelves (the №845 form).

## §3. The typed surface — the constructors and the negative controls

The type lands pure, without IO (the portable-kernel form of ADR-0204
§3 — the same «no clock, no network, no filesystem» boundary; the host
passes the moment in):

- the constructors: **issue** (the human decision becomes a mandate),
  **verify** (is it live: the window holds, the digest matches, it is
  not revoked — returns the algebra verdict), **revoke** (the explicit
  early withdrawal — the human consent is withdrawable by the human);
- the negative controls are part of the contract, not tests-only:
  (а) a mandate on a foreign digest → Deny; (б) an expired or revoked
  mandate → Deny; both refusals happen BEFORE the action executes
  (the «2 negative controls» standard — the memanto
  v0.2.29→v0.2.30 lesson).

## §4. The coverage rule and the machine fact — what №861–№862 land

- **The coverage rule (№861):** a human-oversight carrier without a
  live mandate refuses before execution — the fail-closed gate of the
  №845 shape, wired on both backends (the TW/VM parity, ADR-0197).
  The e2e trace request → approve → execute → revoke/expire → deny
  runs on both backends (the №847 trace form); the Ledger records
  keep the existing shape (ADR-0167) — the ledger-golden and
  conformance suites pass WITHOUT golden edits.
- **APPROVAL_COVERAGE (№862):** the machine fact — the share of
  human-oversight carriers covered by the fail-closed approval gate,
  in bp, counted over the live registry (the №846 form: the fact rides
  with its checker). The floor is `goal_approval_coverage_bp 10000`
  in `scripts/ci/gate_036_goals.txt` — the plan §23 constant, only-up;
  the tamper traps live (the №857 form: placeholder, deletion,
  definition-substitution, floor-lowering).

## §5. Honest limits — what the mandate is NOT

- **Not a signature under a document.** The `target_digest` binds the
  action/goal the carrier names — nothing more. No cryptographic
  signature semantics, no document-binding claim, no counterparty.
- **Not a legal act.** No legal-effect claim of any kind. This is
  configuration of constraints, not legal advice (the R38 (6)
  revision discipline).
- **Not an audit mechanism.** The Ledger only writes the trace
  (ADR-0167); the mandate does not retroactively prove anything — it
  answers the live question «is the consent valid NOW», nothing
  about the past beyond what the Ledger already records.
- **Not IO, not network, not UI.** The type is pure (the ADR-0204 §3
  kernel boundary); `ask_approval` remains the request surface, the
  mandate is the record of the answer (№860), the enforcement is the
  gate (№861) — this record and the code naryads add no interaction
  channel with the human.
- **Not a replacement of the grant contour.** Capabilities and grants
  (ADR-0203/ADR-0155) stay the rights machinery with their own ledger,
  quotas and revocation; the mandate layers the consent condition over
  the human-oversight class only. Carriers outside the class do not
  change (the №861 boundary).
- **Not new refusal vocabulary.** The 14 words stay frozen (№385);
  the cause is the existing `IRREVERSIBLE_NO_GRANT` class; the
  GRANted-`GRANT_*` detail wording remains the producer's `human`
  payload exactly as №390 folded it.
- **Publication is the owner's act.** The 0.38 release, the floor
  posture changes and the market part are outside the wave's naryads
  (§16.0-7).

## §6. The migration discipline — 1:1, byte-identical surfaces

- №860 lands the type OVER the existing constructors and surfaces: no
  existing constructor changes shape or semantics (the №838/№854
  1:1 migration pattern); the DenyEvent surface and the Ledger
  records stay byte-identical — the parity test of the №854 form
  keeps pinning the 14 words, the ledger-golden and conformance
  suites pass with ZERO golden edits.
- The existing approval words keep their semantics: `ask_approval`
  stays «sends an approval request to the human»,
  `awaiting_approval` stays the office goal status — the wave builds
  OVER them, never renames them, never moves them.
- The TW/VM parity holds on every wired carrier (the №829 lesson,
  ADR-0197): a rule enforced on one backend and not on the other is a
  defect, not a feature.

## §7. Acceptance

- Merges №859–№864 in Metalogos- with the green CI of the repository
  on every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The DenyEvent surface and the Ledger records unchanged: the
  ledger-golden, the conformance and the 14-word parity suites (№854)
  green WITHOUT golden edits.
- The trace gate: the e2e request → approve → execute → deny green on
  both backends.
- The 0.38 gate: APPROVAL_COVERAGE in `gen_metrics --check`, floor
  10000 bp, only-up; the tamper traps live.
- The coverage hygiene: the advisory `::warning` gone from main (the
  №863 re-pin, the «the floor follows the fact» form of №569).
