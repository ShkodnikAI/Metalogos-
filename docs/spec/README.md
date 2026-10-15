# The METALOGOS Core Specification — the normative layer

**Status: normative for the topics it records.** This directory is the
specification of how the METALOGOS language MUST behave — the «спецификация
— эталон» machine (the unified audit of 48301708, §6.2 Recommendation 1).
Before a topic is recorded here, the implementation (either backend) was
the de-facto reference; from the moment a norm lands, the SPEC is the
reference and both backends are its checked-in proofs.

## What is normative here

- A **norm** is a numbered statement of required behavior with a stable
  identifier (`S-VAL-###` — the value-semantics topic; later topics get
  their own prefixes). The norm states WHAT the language must do, in one
  sentence, without narrating either implementation.
- Every norm carries an **implementation anchor** (the function that
  realizes it, per backend) and a **conformance test** (the checked-in
  pair under `tests/conformance/` that proves it on BOTH backends).
- A norm exists ONLY where both backends already agree (each pair is run
  on the TW interpreter and the VM by the blocking
  `conformance (blocking)` CI job; a cross-backend divergence is a RED).
  Where the backends diverge, there is NO norm — the divergence is a
  loud finding that needs its own naryad (never a silent fit).

## What is NOT here

- This is NOT the builtins reference (REFERENCE.md keeps that role) and
  NOT a style guide. No new language constructs are introduced by this
  layer — the specification RECORDS existing behavior; changing the
  behavior is a semantics-change naryad (the owner's gate), and the norm
  text changes only after the change lands on both backends.
- The floor vocabulary: `String`, `Float`, `Bool`, `Unit` (the precise
  scalar set), the composites `List`, `Struct`, and the opaque values
  (Html, Query, Secret, ...). The stable refusal codes are frozen
  contracts (№385/ADR-0169).

## The conformance contract

Each norm's test is a pair `<id>.mlog` + `<id>.expected` in
`tests/conformance/`:

- the `.mlog` file is a self-contained program exercising the norm;
- the `.expected` file pins the record:
  `STATUS: ok` + `OUT: <exact output>`, or `STATUS: error` +
  `CODE: <stable refusal code>` (the wording is deliberately not pinned —
  the stable code is);
- the runner (`tests/conformance.rs`) executes every pair on both
  backends through the production paths and demands the cross-backend
  agreement FIRST, then the `.expected` match.

## Topics

| Topic | File | Norms |
|---|---|---|
| 1. Comparisons and truthiness | [values.md](values.md) | S-VAL-001 … S-VAL-013 |
| 2. Blocks and control flow | [blocks.md](blocks.md) | S-BLK-001 … S-BLK-014 |
| 3. Errors and the try result | [errors.md](errors.md) | S-ERR-001 … S-ERR-006 |
| 4. Routes and the HTTP egress | [routes.md](routes.md) | S-RT-001 … S-RT-006 |
| 5. State and memory | [state.md](state.md) | S-ST-001 … S-ST-003 |
| 6. Tools and the abstraction | [tools.md](tools.md) | S-TOOL-001 … S-TOOL-007 |

**The normative core: 49 norms** (13 value-semantics + 14 blocks/control
flow + 6 errors/try + 6 routes/HTTP-egress + 3 state/memory + 7 tools),
every one
probed on both backends per the №645 protocol; the probe of topic 2
made three honest findings recorded in its «Honest limits»
(№652-a/№652-b/№652-c — the candidate repair naryads, all three
repaired by №658/№659); the probes of topics 3, 4 and 5 found NO
cross-backend divergence — the try shape and the code classification
are shared-code parity by construction, and the route/state refusals
are shared-code builtins and semantic checks (the topics' «Honest
limits» cite the mirrored by-design boundaries); the probe of topic 6
(№696, the №684 contract) found NO cross-backend divergence in the
language-facing tool facts — its honest findings are the codeless
negative-path refusals (the «Honest limits» of tools.md — the №385
additive lane would unlock the refusal norms).
