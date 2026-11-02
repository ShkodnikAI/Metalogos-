# ADR-0207: Provenance-Native I/O — the I/O Boundary as a Provenance Carrier (the 8C / C3 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-77 acceptance —
  the dispatch gh#1729 issued the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-02
- Deciders: the OWNER (the R38 train — 0.40 = 8C (C3, PROVENANCE_SHARE
  10000 bp); the wave-77 dispatch gh#1729)
- Naryad: №872 (issue #1723, Wave 77 — W-F; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R55 GET-only
  fact-check over live main `89d06850` (2026-11-02)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.40 is this record's train slot),
  [ADR-0166](0166-media-c2pa-contour.md) (the C2PA contour of media
  handles — the media-manifest parent this record consumes, never
  rewrites), [ADR-0152](0152-c2pa-mini-slice.md) (the Art.50 synthetic
  marking), [ADR-0164](0164-perception-ast-origin-chain.md) (the
  perception origin chain — HandleSource/Lift/Sink/ProvBind), 
  [ADR-0167](0167-action-ledger-v1.md) (the Action Ledger v1 — the
  trace binding this record must not perturb),
  [ADR-0157](0157-ledger-profile-prov-intoto.md) (the in-toto/DSSE
  profile of the Ledger — the transport profile this record aggregates
  OVER), [ADR-0202](0202-model-passport.md) (the model passport — a
  referenced parent), [ADR-0203](0203-capability-token.md) (the
  capability token — a referenced parent), [ADR-0196](0196-mcp-policy-gateway.md)
  (the MCP policy gateway and the G1–G5 gates),
  №828 (the G5 digest-pinning — the determinism pattern this record
  adopts), [ADR-0206](0206-compliance-evidence-compiler.md) (the
  attestation map — the compliance domain this record stays apart
  from), №337 (the media-manifest test guard), №385 (the frozen
  14-word vocabulary), №860/№867 (the typed-carrier form this record
  follows), №874–№877 (the naryads that land on top of this record)

## §1. Context — the values cross the I/O boundaries without a machine link to their source

Every value that enters or leaves a Metalogos program crosses one of
the shared I/O boundaries — `read_file`, `write_file`, `http_get`,
`http_post`, `respond`, `print` — and today the crossing carries the
VALUE only. Nothing machine-checkable ties the bytes that left to the
bytes that arrived: a consumer cannot ask «where did this value come
from, and is it still the value that was recorded?». The honest state
of the art on main (verified R55, `89d06850`):

- provenance lives in POINT domains, each with its own surface: the
  C2PA media manifests (`src/builtins/media.rs` — the
  `<path>.manifest.json` sidecar contour of №337, ADR-0152/ADR-0166),
  the perception origin chain (ADR-0164, HandleSource/ProvBind), the
  taint/redact layer (ADR-0136/ADR-0154), the Ledger's in-toto/PROV
  export profile (ADR-0157), the model passports (ADR-0202);
- the shared I/O layer has NO provenance record at all:
  `ProvenanceManifest` / `PROVENANCE_SHARE` — 0 hits on `89d06850`;
- the carriers themselves are alive and classified: `read_file`
  (Source, Internal), `write_file` (Sink, Internal), `respond` (Sink,
  Public), `http_get` (Source, Network), `http_post` (Sink, Network),
  `print` (Sink, Public) — the classification SSOT
  `src/builtins_classification.rs` pins every role;
- the fail-closed I/O posture already exists in slices: `fs_gate.rs`
  (the file facade — the only raw `File::open`), the RESPOND_SWALLOWED
  swallow-guard (№601/ADR-0184), the file-ingest gate (№455). None of
  them records provenance; none makes a missing record a refusal.

The consequence: the compliance line of Phase 8 (the passports, the
mandates, the attestations) produces verifiable claims about ACTIONS,
but the I/O substrate those actions read from and write to stays
opaque. Subphase 8C closes exactly that gap: the I/O boundary itself
becomes a provenance carrier.

## §2. Decision — provenance is origin digest + channel + revision, recorded at the boundary

1. **Provenance is a typed fact about a boundary crossing.** It
   carries: the origin digest (the SHA-256 of the value bytes that
   crossed — the G5 digest-pinning form of №828), the channel (file /
   network / respond / print — the carrier's own channel class), the
   direction (ingress / egress), the revision (a monotone sequence of
   the record within the process — later records of the same bytes are
   revisions, never replacements), and the fact of record (the moment
   and the row digest that pin the record itself). The exact Rust
   shape is №873's; this record fixes the field SEMANTICS.
2. **An I/O carrier is a boundary crosser with a MANDATORY
   provenance record.** The record happens AT THE BOUNDARY — inside
   the carrier's own path, after the value exists and before (for
   egress) the side effect completes. The carrier signatures do not
   change: the record is a side statement of the crossing, not a new
   argument surface (the 1:1 migration, §6).
3. **The provenance map is append-only and the records are
   content-addressed.** A crossing adds a row; nothing edits or
   deletes a row (the loud-bookkeeping posture of №335, the attestation
   map form of ADR-0206 §2.2). Two records of the same bytes do not
   contradict — they are two attestations of the same origin digest
   (the media-consistency guard, №873).
4. **The freshness rule is digest equality at the use site (the G5
   form).** The origin digest is pinned when the bytes cross; a use
   that must prove its origin re-hashes the observed bytes and
   compares. A substituted digest — the bytes changed between the
   read and the use without crossing a recorded boundary — refuses
   (the digest-pinning negative control of №828, landed as the
   verify seam of №873).
5. **Fail-closed egress (the coverage rule — №874):** an egress
   carrier whose boundary record is missing or refused does not
   perform the side effect; the refusal is loud, names «provenance
   missing», and journals itself through the EXISTING Ledger ACTION
   path (`ledger::record`, the fire_on_deny form of ADR-0167 §3.4).
   An ingress carrier whose record is missing refuses ON USE, not on
   read — «молча пропустить» is forbidden (the №584/№601 posture).

## §3. The portable-kernel boundary — what the provenance module is and is not

The value layer lands pure (the portable-kernel form of ADR-0204 §3 /
ADR-0205 §3 / ADR-0206 §3 — no clock, no network, no filesystem in
the carrier itself; the host passes the moment and the bytes in):

- **IN:** the types (the manifest, the append-only map), the channel
  and direction vocabularies (closed enums — a new channel lands as an
  explicit ADR/registry transaction), the freshness rule (origin
  digest + the verification date — the G5 digest-pinning form of
  №828), the deterministic serialization (canonical field order, the
  row digest over every field), the negative-control constructors (the
  №860 form: the refusals are part of the contract, not tests-only).
- **OUT:** the network as a transport (no new HTTP/ws surfaces — the
  carriers keep their existing transports; this record adds records,
  not routes), new grant mechanics (the grant contour ADR-0155 /
  ADR-0203 is untouched — provenance never issues, widens or consumes
  a right), the media domain (the C2PA contour of №337 is CONSUMED —
  the two record forms agree on the digest and never rewrite each
  other), legal texts (the legal-claim layer is the 8H-1 subphase,
  0.46).
- **The refusal vocabulary does not grow.** The 14 words stay frozen
  (№385/ADR-0169); the egress refusal rides the EXISTING
  `SINK_CLEARANCE` class (the generic sink-clearance word — a sink
  that cannot prove its provenance record is a sink without
  clearance) with the typed detail «provenance missing» in the
  refusal text and the Ledger detail. No new DenyEvent class, no new
  deny word.

## §4. The consumption contracts — what №873–№877 land

- **№873 — the type in code:** `ProvenanceManifest` + the append-only
  map in a dedicated kernel module (the approval.rs/compliance.rs
  form); the explicit carrier registry (`PROVENANCE_CARRIERS` — the
  wave-77 transaction opens with the six shared-boundary carriers:
  `read_file`, `write_file`, `http_get`, `http_post`, `respond`,
  `print`; the totality test pins every name against the
  classification SSOT — a non-carrier name reddens); the boundary
  record wired inside the carriers (1:1 — no signature, no golden,
  no Ledger-shape change); the negative controls: a record-less
  boundary demand refuses «provenance missing»; a substituted origin
  digest refuses (the G5 form); the media-manifest consistency guard
  (the №337 digest forms agree — two records of the same bytes never
  contradict).
- **№874 — the coverage rule:** the fail-closed egress gate (§2.5)
  composed inside the egress carriers BEFORE the side effect; the
  ingress use-check (§2.5) as the verify seam; the e2e trace
  action → ingress (recorded) → processing → egress (recorded) →
  manifest on BOTH backends (TW/VM parity — the №829 lesson); the
  negative controls: an egress without the record refuses and
  journals (the Ledger ribbon carries «provenance missing»); the
  legit path with the full record passes without degradation.
- **№875 — the machine fact PROVENANCE_SHARE:** the share of the
  registered carriers whose boundary record is LIVE-wired, bp,
  counted over the explicit registry (the APPROVAL_COVERAGE form of
  №862: the roster is the machine source, the composition site is
  fixed, the fact rides with its checker); the gate_038 floor 10000
  bp, only-up, the tamper traps by the gate_036/037 лекало; the
  wiring rides the EXISTING docs-metrics job (the cell does not grow
  — №535).
- **№876 — the baseline ride:** the type_signature /
  sensitive_surface baselines re-pinned under the provenance module's
  merge fact («the floor follows the fact» — the №863 form); the
  advisory ::warning never appears on main.
- **№877 — the weekly record:** the wave-77 entry by the №789
  generator, strictly last (the N-rule).

## §5. Honest limits — what the provenance layer is NOT

- **Not a taint rewrite.** The label lattice (ADR-0154) and the
  taint propagation (ADR-0136) keep governing CONFIDENTIALITY; the
  provenance layer records ORIGIN. A value can be perfectly tainted
  and perfectly provenanced at once; neither subsumes the other.
- **Not the media provenance rewrite.** The C2PA contour (ADR-0152 /
  ADR-0166, №337) stays the media-domain SSOT with its sidecar
  manifests; this record consumes its digest form (the consistency
  guard) and never writes a second manifest format over media bytes.
  The media carriers (`media_save`, `vision_export`, …) are NOT in
  the wave-77 registry — extending the registry is an explicit
  future transaction.
- **Not tamper-PROOF.** The origin digest makes substitutions LOUD
  (the ADR-0167 boundary verbatim: tamper-EVIDENT, not
  tamper-PROOF). An attacker who controls the process can re-record;
  the external anchoring is the Ledger's own posture (ADR-0167 §6),
  not this record's.
- **Not a grant, not a policy.** The provenance record never
  authorizes, never widens, never replaces the capability gate
  (№845), the approval gate (№861) or the file-ingest gate (№455).
  The gates keep their surfaces; the record rides beside them.
- **Not the compliance attestation.** The attestation map
  (ADR-0206/№867) compiles evidence CLAIMS for an external auditor;
  the provenance manifest records I/O crossings for the runtime. The
  two share the digest-pinning pattern, not the contract; the names
  stay apart (`Attestation` vs `ProvenanceManifest` — the dedup
  guard of №873 keeps the boundary mechanical).
- **Not a new refusal surface.** The egress refusal reuses the
  frozen `SINK_CLEARANCE` class; no new DenyEvent word lands (№385).
- **Not IO in the value layer.** The kernel module is pure; the
  boundary wiring lives in the carriers (the executor seam), the
  process map is the host's store (the approval-store seam form).

## §6. The migration discipline — 1:1, byte-identical surfaces

- №873 lands the type OVER the existing carriers: no carrier
  signature, no registry spec, no Ledger record shape, no golden file
  changes (the №838/№854/№860/№867 1:1 migration pattern); the
  ledger-golden and conformance suites pass with ZERO golden edits;
  the happy path adds NO Ledger records (the record-on-refusal-only
  posture keeps the goldens byte-identical).
- The parents are consumed by reference: the media manifests stay the
  media SSOT (№337), the Ledger stays the trace SSOT (ADR-0167), the
  passports stay the weights SSOT (ADR-0202), the capability layer
  stays the rights SSOT (ADR-0203), the digest-pinning gateway stays
  the policy SSOT (ADR-0196/№828). The provenance map is an
  untrusted, verifiable VIEW over the crossings; a drift between the
  view and a parent is a LOUD verification failure, never a silent
  overwrite.
- The TW/VM parity posture (ADR-0197, the №829 lesson) holds: the
  boundary record happens inside the carriers both backends dispatch
  through — the e2e trace of №874 runs green on both lanes by
  construction.

## §7. Acceptance

- Merges №872–№877 in Metalogos- with the green CI of the repository
  on every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The surface gate: the carrier signatures, the Ledger records, the
  DenyEvent surface and the golden files unchanged — the
  ledger-golden and conformance suites green WITHOUT golden edits;
  the media golden byte-for-byte; the 14-word vocabulary unchanged.
- The trace gate: the e2e ingress → processing → egress → manifest
  green on BOTH backends; the «provenance missing» refusal visible in
  the Ledger ribbon; the manifest serialization double-run
  byte-identical.
- The 0.40 gate: PROVENANCE_SHARE in `gen_metrics --check`, floor
  10000 bp, only-up; the tamper traps live.
- The baseline hygiene: the advisory `::warning` absent from main
  (the №876 re-pin, the «the floor follows the fact» form of №569).
