# ADR-0206: Compliance Evidence Compiler — the Attestation Map as a Typed, Verifiable, Sellable Artifact (the 8A-1 / C1 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-76 acceptance —
  the dispatch gh#1714 issued the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-01
- Deciders: the OWNER (the R38 revision (3) — C1 moved to the 0.39
  train slot: «превращает мандаты в продаваемый артефакт после
  завершения линии 8B»; the wave-76 dispatch gh#1714)
- Naryad: №866 (issue #1708, Wave 76 — W-E; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R54 GET-only
  fact-check over live main `67b005fe` (2026-11-01)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.39 is this record's train slot),
  [ADR-0196](0196-mcp-policy-gateway.md) (the MCP policy gateway — the
  flagship C1 (b) pilot and its digest-pinning), №828 (the G5
  digest-pinning — the determinism pattern this record adopts),
  [ADR-0167](0167-action-ledger-v1.md) (the Action Ledger v1 — the
  trace binding this record must not perturb),
  [ADR-0203](0203-capability-token.md) (the capability token — a
  referenced parent, never copied),
  [ADR-0205](0205-typed-approval.md) (the typed mandate — a referenced
  parent, never copied),
  [ADR-0202](0202-model-passport.md) (the model passport — a referenced
  parent, never copied), [ADR-0163](0163-backend-registry-licenses.md)
  (the license classes),
  [ADR-0157](0157-ledger-profile-prov-intoto.md) (the
  in-toto/DSSE profile of the Ledger — the transport profile this
  record aggregates OVER, never rewrites),
  [ADR-0200](0200-provable-lowering.md)
  (the Reflex attestation — the C4 name boundary this record keeps),
  №320 (the C2PA/Art.50 slice), №385 (the frozen 14-word vocabulary),
  №565 (the structural reproducibility — the raw-rows rule for the
  machine fact), №857/№862 (the machine-fact-with-its-checker form),
  №867–№871 (the naryads that land on top of this record)

## §1. Context — the parents write verifiable traces, but the sellable artifact does not exist

The compliance parents are alive on main and stay untouched:

- the MCP policy gateway (`metalogos-server/src/mcp_policy.rs`,
  [ADR-0196](0196-mcp-policy-gateway.md); the irreversible-tool
  policy, the G1–G5 gates, the digest-pinning of №828);
- the Action Ledger (`src/ledger.rs`,
  [ADR-0167](0167-action-ledger-v1.md); the ACTION paths journal
  grant/deny/irreversible events; the golden trace
  `examples/w72_capability_trace.ledger.golden` pins the record shape);
- the capability token (`src/capability.rs`, №844) and the grant
  algebra (`src/grants.rs`, ADR-0155) — the typed rights machinery
  with its own ledger, quotas and revocation;
- the typed mandate (`src/approval.rs`, №860) — the consent decision
  as a value with the live-verify gate (№861);
- the model passport (`src/model_passport.rs`, №838) — the
  license-typed weights artifact with its SHA-256 pins;
- the C2PA/Art.50 slice (№320) — the content-provenance posture.

What does NOT exist is the artifact an external auditor consumes. The
parents WRITE verifiable traces, but proving compliance to a partner
today means lifting the whole language: there is no map from a claim
about the toolchain to the digest of its evidence, no freshness
verdict, no deterministic export. `AttestationMap` / `ATTEST_MAP` —
0 hits on `67b005fe`. The word *attest* is alive in a DIFFERENT
domain: `src/nn/attest.rs` (the Reflex distillation certificates, the
C4 line №825, the surface frozen by №832) — this record builds a
SEPARATE contour on top and keeps the names apart (§5); renaming the
existing attest artifacts is forbidden.

## §2. Decision — the attestation is a value, the map is append-only, the artifact is a deterministic serialization

1. **An attestation is a typed carrier of a compliance claim.** It
   carries: who attests (`actor`), what the claim is about (the
   subject digest), the claim itself (a code from the EXISTING
   vocabularies — no new claim language is minted by this record),
   the digest of the evidence (`evidence_digest` — the SHA-256 of the
   referenced artifact), the verifier (the named check a third party
   can re-run), the freshness window (`issued_at` / `expires_at`),
   and the revocation state. The exact Rust shape is №867's; this
   record fixes the field SEMANTICS.
2. **The attestation map is append-only.** Issue adds, revoke MARKS —
   a revocation never deletes the row (the consent-ledger posture of
   №335, the loud-bookkeeping discipline). The map aggregates the
   existing traces; it never mints rights, never widens a sink, never
   weakens a gate.
3. **An attestation REFERENCES its parents, it does not copy them.**
   The evidence of a claim is a Ledger record, a live mandate, a
   passport, a capability token or a gateway digest — named by kind
   plus digest. The parent surfaces stay the SSOT; the map is an
   untrusted, verifiable VIEW over them (the capability-token posture
   of №844: the token is a cache, the ledger is the SSOT).
4. **The sellable artifact is the deterministic serialization of the
   map.** Sorted rows, canonical field order, a digest pinned to
   every row, a digest over the whole package (the G5 form of №828:
   a row without a digest is not a row of the map). Determinism is a
   CONTRACT, verified by the double-run byte-identity (№868): two
   compilations of the same inputs produce the same bytes, or the
   compiler is broken.
5. **Fail-closed verification (the №845/№861 posture):** a map row
   whose evidence digest does not match the referenced artifact, a
   row past its freshness window, a reused revoked attestation — the
   consumer refuses; nothing degrades silently into «compliant».

## §3. The portable-kernel boundary — what the compliance module is and is not

The module lands pure, without IO (the portable-kernel form of
ADR-0204 §3 / ADR-0205 §3 — no clock, no network, no filesystem in
the value layer; the host passes the moment and the digests in):

- **IN:** the types (the attestation, the map), the freshness rule
  (the window plus the comparison date — the G5 digest-pinning form),
  the deterministic serialization (canonical rows, per-row digests,
  the package digest), the negative-control constructors (the №860
  form: the refusals are part of the contract, not tests-only).
- **OUT:** the network (no new transport surfaces — the package is a
  file/string the caller carries), new grant mechanics (the grant
  contour ADR-0155/ADR-0203 is untouched — the map never issues,
  subgrants or consumes rights), legal texts (the legal-claim layer
  is the 8H-1 subphase, 0.46 — this record compiles EVIDENCE, not
  legal opinion).
- **The refusal vocabulary does not grow.** The 14 words stay frozen
  (№385/ADR-0169); the map creates no new deny paths — it aggregates
  existing traces. The construction refusals (a missing evidence
  digest, a dangling parent reference, a revoked-row reuse) are
  constructor errors, not new DenyEvent classes.

## §4. The consumption contracts — what №867–№869 land

- **№867 — the type in code:** `Attestation` + the append-only map in
  a dedicated kernel module (the approval.rs/capability.rs form);
  the negative controls (the №860 form): an attestation without an
  evidence digest refuses at construction; a dangling parent
  reference refuses; a closed/revoked attestation refuses reuse. The
  migration is 1:1: the event surface, the Ledger records and the
  golden files stay byte-identical.
- **№868 — the evidence-package compiler:** from the map plus the
  live in-repo sources it compiles the deterministic package (sorted
  rows, canonical serialization, per-row digests, the package
  digest); the e2e trace action → Ledger → attestation → map row →
  package; the double-run byte-identity is the acceptance gate; the
  README section tells the external auditor how to consume the
  package (no market claims — the ADR-0201 §5 boundary).
- **№869 — the machine fact ATTEST_MAP:** the count of the
  attestation rows in the compiled map, counted from the RAW map
  rows structurally (the №565 lesson: reproducibility is structural
  — never from a summary number); the fact rides with its checker
  (the №857/№862 form), the floor is `goal_attest_map 30` in
  `scripts/ci/gate_037_goals.txt` — the start floor, only-up (the
  №841 form; the owner fixes the goals at the acceptance dispatch);
  the tamper traps live (the gate_036 form: the summary substitution,
  the deletion, the definition substitution, the floor lowering).

## §5. Honest limits — what the attestation map is NOT

- **Not a legal opinion and not a certification.** The map compiles
  verifiable evidence about a toolchain; it makes no legal-effect
  claim of any kind (the R38 (6) discipline). The 8H-1 legal-claim
  layer is a later subphase.
- **Not tamper-PROOF.** The digests make mismatches LOUD (the
  ADR-0167 boundary verbatim: tamper-EVIDENT, not tamper-PROOF). An
  attacker who controls both the artifact and the map can re-digest;
  the counter-signature and the external anchoring are future
  subphases, not this one.
- **Not a signature under a document.** The evidence digest binds the
  artifact bytes — nothing more. No cryptographic signature
  semantics, no counterparty, no document-binding claim.
- **Not IO, not network, not UI.** The value layer is pure (the
  portable-kernel boundary of §3); the package leaves the process as
  a plain file/string the caller owns.
- **Not a new refusal surface.** The map aggregates; it never gates a
  builtin, never widens a sink, never adds a DenyEvent class (№385 —
  the 14-word vocabulary is frozen).
- **Not the C4 attestation.** `src/nn/attest.rs` (the Reflex
  distillation certificates, №825) stays the model-provenance domain;
  this record's `Attestation`/`AttestationMap` live in the compliance
  kernel module and NEVER rename, move or wrap the C4 surface (the
  №832 freeze). The two domains share a root word, not a contract.
- **Publication and distribution are the owner's acts.** The npm/crate
  publication of the package format and any market claim are outside
  the wave's naryads (§16.0-7, the ADR-0201 §5 boundary).

## §6. The migration discipline — 1:1, byte-identical surfaces

- №867 lands the type OVER the existing surfaces: no existing
  constructor, builtin, Ledger record or DenyEvent changes shape or
  semantics (the №838/№854/№860 1:1 migration pattern); the
  ledger-golden and conformance suites pass with ZERO golden edits.
- The parents are consumed by reference: the Ledger stays the trace
  SSOT (ADR-0167), the mandates stay the consent SSOT (ADR-0205),
  the passports stay the weights SSOT (ADR-0202), the gateway stays
  the policy SSOT (ADR-0196). The map is an untrusted view; a drift
  between the view and a parent is a LOUD verification failure,
  never a silent overwrite.
- The TW/VM parity posture (ADR-0197, the №829 lesson) holds wherever
  the map's values are exercised from the language side; the kernel
  module itself is backend-neutral (pure values).

## §7. Acceptance

- Merges №866–№871 in Metalogos- with the green CI of the repository
  on every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The surface gate: the Ledger records, the DenyEvent surface and the
  golden files unchanged — the ledger-golden and conformance suites
  green WITHOUT golden edits; the 14-word vocabulary unchanged.
- The trace gate: the e2e action → Ledger → attestation → map row →
  package green; the double compiler run byte-identical.
- The 0.39 gate: ATTEST_MAP in `gen_metrics --check`, floor 30,
  only-up; the tamper traps live.
- The baseline hygiene: the advisory `::warning` absent from main
  (the №870 re-pin, the «the floor follows the fact» form of №569).
