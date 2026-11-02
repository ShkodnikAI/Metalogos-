# ADR-0208: Provider-Agnostic Local-First — the LLM Layer as a Provider-Agnostic Contract (the 8E-1 / C5 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-78 acceptance —
  the dispatch gh#1745 issued the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-02
- Deciders: the OWNER (the R38 train — 0.41 = 8E-1 (C5, OFFLINE_SUITE
  ≥ 8000 bp); the wave-78 dispatch gh#1745; the «Заливай следующую
  волну» directive of 2026-11-02)
- Naryad: №878 (issue #1738, Wave 78 — W-G; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R56 GET-only
  fact-check over live main `93c18673aa` (2026-11-02)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.41 is this record's train slot),
  [ADR-0207](0207-provenance-native-io.md) (the provenance-native I/O
  — the egress-gate contour of wave 77; this record CONSUMES it and
  never duplicates the network policy),
  [ADR-0203](0203-capability-token.md) (the capability contour — the
  rights SSOT this record never widens), [ADR-0169](0169-stable-try-codes.md)
  (the frozen 14-word refusal vocabulary), [ADR-0197](0197-vm-only-executor.md)
  (the VM-is-the-executor posture — the backend-agnostic rule of №880
  rides it), ADR-0047/ADR-0048 (the mock counter and the per-call
  model override — the existing seams the contract keeps), №454 (the
  mock SSOT predicate — the explicit opt-in this record extends with
  the local-first default), №819 (the per-provider `model:` resolution
  — landed; this record renames nothing), №385 (the frozen refusal
  vocabulary), №872 (the ADR-before-code form this record follows),
  №879–№884 (the naryads that land on top of this record)

## §1. Context — the LLM surface works, but the contract is implicit and the local posture is undefined

The `llm{}` surface is alive and stable on main (verified R56,
`93c18673aa`): the grammar pins the field set
(`src/grammar.pest:15/24–25/39/40/48` — `providers`, `default_model`,
`failover`, `circuit_breaker`, plus the №757 `max_tokens`/
`temperature`), the implementation is a 3853-line module
(`src/llm.rs`) with the routing machinery this subphase builds ON:
`Provider` (the provider roster enum, llm.rs:240), `RealLlm` (the
HTTP backend, llm.rs:327), `MockLlm` (the deterministic backend,
llm.rs:69), `ProviderHealth` (the circuit/health tracker, llm.rs:1177)
and `SmartRouter` (the failover loop, llm.rs:1494). The tests are
green: `llm_cache_contract`, `naryad_248_llm_cancellation`,
`naryad_757_llm_limits`, `naryad_819_provider_model`; the CHANGELOG
carries the failover line three times over.

The honest gaps this record closes:

- **The contract is implicit.** A call is addressed to *whatever the
  router happens to hold* — the capabilities, the limits and the
  health semantics of «a provider» exist only as scattered method
  facts (`default_model()`, `endpoint()`, `requires_api_key()`, the
  `ProviderHealth` scores). Nothing in the type system says what a
  provider IS; nothing separates the contract (what a call may demand)
  from the implementations (who may answer).
- **The local posture is a test flag, not a posture.** `MockLlm` is
  reachable ONLY through the explicit `METALOGOS_MOCK_LLM=1|true` opt-in
  (№454 — the anti-silent-mock lesson); without `llm{}` and without the
  flag the legacy default is the real backend failing loudly at the
  first call. The system has no defined answer to «can I run without
  any external network?» — the local mode exists, but it is framed as
  a testing seam, never as a first-class operating mode.
- **No machine fact pins the offline coverage.** The suite that
  exercises the LLM surface runs on the deterministic local fixtures
  (the mock, the loopback stubs) — but no counter MEASURES that; a
  future regression that drags the LLM tests onto the external network
  would drift silently.

## §2. Decision — the contract is the address; the local mode is the default

1. **The LLM layer is a provider-agnostic contract, fixed in code as
   `ProviderContract`** (the name is the ADR's — the dedup guard of
   №879 keeps it apart from the existing `enum Provider`, which stays
   the implementation roster). A call addresses the CONTRACT: the
   capabilities (what the answering side can do — the deterministic
   answer, the per-call model override, the deadline cancellation),
   the limits (the ceilings a call is bound by — the timeout, the
   token ceiling, the retry count) and the health semantics (the
   circuit-breaker posture the grammar's `circuit_breaker` field
   configures). A provider — Anthropic, OpenAI, Ollama, a future one —
   is a REPLACEABLE implementation of the contract, never the address.
   The local provider (`MockLlm`, the deterministic non-echo backend
   of №454/№487) is a FULL-RIGHT member of the implementation roster,
   not a test stub.
2. **Local-first: the absence of external configuration is the NORMAL
   operating mode.** A program with no `llm { providers: … }` block
   and no explicit external-provider declaration runs on the local
   provider — the deterministic answer, zero network. The network is
   an OPTIMIZATION layered on top by explicit declaration (`llm{}` or
   the `METALOGOS_LLM_PROVIDER` env — the existing declaration
   surfaces, unchanged); it is never a precondition of the surface
   working. The №454 loudness lesson is preserved: the local answer
   carries the deterministic marker of the local provider (`mock_response`) in the value
   itself — a local-first answer is VISIBLE as such, never a silent
   stand-in for an external voice.
3. **The failover chain is a contract behavior, not a provider
   behavior.** The `failover: auto` cycle (grammar.pest:39) and the
   `circuit_breaker` threshold (grammar.pest:40) are SEMANTICS OF THE
   CONTRACT — every implementation is bound by the same loop, the same
   health accounting (`ProviderHealth` stays the tracker) and the same
   exhaustion refusal classes. The coverage rule of №880 extends the
   chain with the covered local fallback: every external call either
   has a covered local fallback path or an honest refusal — the
   fail-closed posture of wave 77 (№874) applied to the LLM layer,
   with the EXISTING refusal classes (the 14-word vocabulary of №385
   does not grow).
4. **The portable-kernel boundary (what the contract layer is and is
   not):**
   - **IN:** the `ProviderContract` type (capabilities + limits +
     health semantics, the deterministic report form), the
     local-first default rule, the failover semantics as a contract
     behavior, the OFFLINE_SUITE fact definition (below).
   - **OUT:** new provider integrations and vendor names (the roster
     `enum Provider` is closed; a new provider is an explicit future
     transaction), the network policy (the egress-gate contour of
     №874/ADR-0207 is consumed, not duplicated — this record adds no
     HTTP surface and no sink rule), the media/perception domains
     (their own backends, their own lines), the grant mechanics (the
     capability contour ADR-0203 stays the rights SSOT — a provider
     contract never issues or widens a right).
5. **The OFFLINE_SUITE machine fact (the definition; the
   implementation is №881):** the share, in basis points, of the
   REGISTERED LLM-surface checks that execute without the external
   network — the local provider, the deterministic mock fault seam
   (№385), the loopback stub servers (the №819/n812 fixture form) all
   count as offline; only checks that require reaching a host outside
   the process/loopback count as online. The registry is an explicit
   file (`scripts/ci/gate_039_goals.txt` — the gate_035–038 roster
   form), the counter reads the registry (the №862 roster form: the
   roster is the machine source, the count is checked, not declared),
   the gate_039 floor is the wave-78 dispatch constant — 8000 bp,
   only-up.

## §3. The parents are consumed by reference — what this record does NOT redefine

- **The `llm{}` grammar is untouched.** The field set, the order
  freedom (№490) and the parse refusals stay; `model → default_model`
  (№819) is landed and not renamed; the per-provider `model:` chain
  (№819) stays the resolution order.
- **The SmartRouter is the loop's SSOT.** The health-score ordering,
  the per-rung model resolution, the exhaustion classification (the
  №385 stamps at the origin) stay exactly as they are; the contract
  layer DESCRIBES the loop, it does not re-route it.
- **The ProviderHealth is the health SSOT.** The contract's health
  semantics expose the tracker's posture (the threshold, the
  availability rule); no second health state machine lands.
- **The refusal vocabulary is frozen.** The exhaustion and the
  unavailable refusals ride the existing `LLM_PROVIDER_UNAVAILABLE` /
  `LLM_TIMEOUT` classes (№385/ADR-0169); no new deny word, no new
  DenyEvent class.
- **The capability contour is the rights SSOT.** A provider
  declaration never grants a network right by itself; the egress
  policy of wave 77 (№874) keeps governing the I/O carriers.

## §4. The consumption contracts — what №879–№884 land

- **№879 — the contract in code:** the `ProviderContract` type
  (capabilities + limits + health semantics) with
  `Provider`/`RealLlm`/`MockLlm` as its implementations; the
  local-first default in the backend composition (the absence of the
  external declaration → the local provider); the migration 1:1 (no
  signature, no grammar, no golden change — the four named suites
  green WITHOUT edits); the negative controls (the unaddressable
  provider refuses loudly through the existing class, the circuit
  threshold honored by the contract, the double-run configuration
  serialization byte-identical).
- **№880 — the coverage rule:** the fail-closed gate over the
  provider dependency — every external call has a covered local
  fallback path or an honest refusal; the e2e trace (the request →
  the deterministic external refusal/timeout → the failover chain →
  the local fallback → the result) green on BOTH backends (the
  №829/ADR-0197 parity lesson), the double run byte-identical; no new
  deny codes.
- **№881 — the machine fact:** the OFFLINE_SUITE registry
  (`scripts/ci/gate_039_goals.txt` — the offline=yes/no field per
  registered check), the counter in `gen_metrics.py` (the GENERATED
  row, the №875 form), the gate_039 floor 8000 bp only-up, the tamper
  traps by the gate_036/037/038 лекало; the wiring rides the EXISTING
  docs-metrics job (the cell does not grow — №535).
- **№882 — the baseline ride:** the type_signature /
  sensitive_surface baselines re-pinned under the llm-contract types
  («the floor follows the fact» — the №863/№870/№876 form); the
  advisory `::warning` never returns to main.
- **№883 — the deps ride:** the rusqlite 0.40.2→0.41.0 bump (the
  dependabot PR #1658, the owner's 02.11 acceptance) inside the wave;
  the rustyline blocker (#1544) re-checked at the execution date.
- **№884 — the weekly record:** the wave-78 entry by the №789
  generator, strictly last (the N-rule).

## §5. Honest limits — what the contract layer is NOT

- **Not a provider abstraction for new vendors.** The roster is
  closed; the contract makes the EXISTING implementations
  interchangeable, it does not invite new integrations (a new provider
  is an explicit ADR/registry transaction, the same discipline as the
  channel vocabulary of ADR-0207 §3).
- **Not a network gate.** The coverage rule governs the LLM FALLBACK
  paths; the egress gate of №874 keeps governing the I/O carriers. A
  provider HTTP call is not a provenance carrier and this record adds
  no record to it.
- **Not a quality claim about the local answers.** The local provider
  answers deterministically and honestly marks itself; the OFFLINE_SUITE
  fact measures WHERE THE TESTS RUN, not how good the local answers
  are. A program that needs a real external voice still declares one —
  local-first is a DEFAULT, never a ceiling on what can be declared.
- **Not a behavior rewrite of the router.** The 1:1 migration keeps
  the observable behavior of every existing test and golden; the ONE
  intentional posture change (the local-first default where the
  legacy default was the loud real backend) is the ADR's decision,
  recorded here — the №454 loudness is preserved by the visible
  marker, and every explicit declaration (the mock flag, the env
  provider, the `llm{}` block) keeps its exact current meaning.
- **Not a new refusal surface.** The unaddressable-provider and
  exhaustion refusals ride the existing classes; the 14 words stay
  frozen.

## §6. The migration discipline — 1:1, byte-identical surfaces

- №879 lands the contract OVER the existing types: no `llm{}` grammar
  change, no `Provider` roster change, no SmartRouter signature
  change, no golden change (the №838/№854/№860/№867/№873 1:1
  pattern); the four named LLM suites pass with ZERO edits; the
  serialization double-run of the contract report is byte-identical.
- The local-first default is the ONE composition-site change: the
  backend composition (the legacy seam `create_llm_backend` and the
  empty-chain seam of the router) resolves to the local provider when
  no external declaration exists; every declared configuration —
  the mock flag, the env provider, the `llm{}` block — keeps its
  exact current semantics.
- The TW/VM parity posture (ADR-0197, the №829 lesson) holds: the
  coverage rule lives in the SHARED builtin/router seam both backends
  dispatch through — the e2e trace of №880 runs green on both lanes
  by construction.

## §7. Acceptance

- Merges №878–№884 in Metalogos- with the green CI of the repository
  on every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The surface gate: the `llm{}` grammar, the `Provider` roster, the
  SmartRouter signatures, the refusal vocabulary and the golden files
  unchanged — the four named LLM suites green WITHOUT edits; the
  refusal dictionary at 14 words.
- The trace gate: the e2e online → external refusal → failover →
  local fallback green on BOTH backends; the configuration
  serialization double-run byte-identical.
- The 0.41 gate: OFFLINE_SUITE in `gen_metrics --check`, floor 8000
  bp, only-up; the tamper traps live.
- The baseline hygiene: the advisory `::warning` absent from main
  (the №882 re-pin, the «the floor follows the fact» form of №569).
- The deps ride: rusqlite 0.41.0 merged inside the wave (the full CI);
  #1544 closed by acceptance or honestly re-dated.
