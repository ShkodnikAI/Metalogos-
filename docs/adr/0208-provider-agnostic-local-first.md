# ADR-0208: Provider-Agnostic Local-First — the LLM Layer as a Provider-Agnostic Contract (the 8E-1 / C5 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-78 acceptance —
  the dispatch gh#1745 issues the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-02
- Deciders: the OWNER (the R38 train — 0.41 = 8E-1 (C5, OFFLINE_SUITE
  ≥ 8000 bp); the wave-78 dispatch gh#1745; the owner's dep verdict
  2026-11-02 «Рекомендация принимается» — the ride pair №883)
- Naryad: №878 (issue #1738, Wave 78 — W-G; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R56 GET-only
  fact-check over live main `93c18673aa` (2026-11-02)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.41 is this record's train slot),
  [ADR-0048](0048-smart-llm-routing.md) (the smart LLM routing — the
  failover/circuit-breaker parent this record contracts OVER, never
  rewrites), [ADR-0047](0047-llm-cache.md) (the LLM cache — the
  call-count parent), [ADR-0037](0037-real-llm-backend.md) (the real
  LLM backend — the provider ladder parent), [ADR-0203](0203-capability-token.md)
  (the capability token — the rights SSOT this layer never widens),
  [ADR-0169](0169-try-stable-error-codes.md) / №385 (the frozen 14-word
  refusal vocabulary — the layer refuses with the EXISTING classes),
  [ADR-0207](0207-provenance-native-io.md) (the provenance-native I/O —
  the egress gate of wave 77 this record does NOT duplicate: the
  coverage rule of №880 is about LLM fallback paths, not about the
  network transport), №4 (the SmartRouter origin), №156 (the real
  request cancellation), №248 (the deadline-based cancellation),
  №454 (the mock-NOT-default contract — the fail-loud legacy posture
  this record preserves 1:1), №487 (the stable mock-marker algorithm),
  №757 (the max_tokens/temperature limits), №819 (the per-provider
  model — `model → default_model` already landed; this record does not
  rename fields), №881 (the OFFLINE_SUITE machine fact this record
  defines — the implementation lands there)

## §1. Context — the LLM surface works, but the provider-agnostic contract is not fixed

The Metalogos LLM surface today is a WORKING multi-provider machine
(verified R56, main `93c18673aa`):

- the `llm {}` grammar block is alive: `providers`, `default_model`,
  `failover`, `circuit_breaker`, `timeout`, `max_tokens`,
  `temperature` (src/grammar.pest:15/24–25/39/40/48; the field set is
  fixed and free-ordered — №490);
- the runtime parents are alive: `Provider` (the vendor enum, llm.rs:240),
  `RealLlm` (the HTTP backend, llm.rs:327), `MockLlm` (the deterministic
  local backend, llm.rs:69), `ProviderHealth` (the per-provider health
  window + circuit breaker, llm.rs:1177), `SmartRouter` (the
  failover router over provider slots, llm.rs:1494);
- the tests that pin the surface: `llm_cache_contract`,
  `naryad_248_llm_cancellation`, `naryad_757_llm_limits`,
  `naryad_819_provider_model`; the CHANGELOG carries failover ×3.

What is MISSING (the same honest-gap posture as ADR-0207 §1):

- **The contract is implicit.** A call today reaches a provider through
  the `LlmBackend` trait and the SmartRouter slots, but nothing
  machine-checkable fixes WHAT a provider is: the capabilities (can it
  stream? does it honor a deadline? does it report token usage?), the
  limits (the model, the max_tokens ceiling, the timeout), and the
  health semantics (circuit_breaker as CONTRACT behavior, not an
  implementation detail of one router) live scattered across types. A
  consumer cannot ask «what does this provider guarantee?» without
  reading the implementation.
- **The local-first posture is not recorded.** The machine already
  runs fully offline when configured so: `MockLlm` answers
  deterministically (the non-echo `ock-llm:<8 hex>]` marker of №454/
  №487), the VM/interpreter lanes run without any network. But the
  posture «work without external network is the NORMAL mode, not an
  emergency» is nowhere written: nothing names the local/stub backend
  a first-class PROVIDER, and nothing defines the offline-coverage
  fact the plan fixes as the 0.41 gate.
- **OFFLINE_SUITE — 0 hits** (PLAN-SUMMARY / CHANGELOG / gen_metrics,
  verified R56).

The consequence: subphase 8E-1 closes exactly that gap — the LLM layer
becomes a provider-agnostic CONTRACT with the local-first default, and
the offline coverage becomes a machine fact with a floor.

## §2. Decision — a call addresses the contract, the provider is an interchangeable implementation

1. **The provider-agnostic contract.** The LLM layer is defined by an
   explicit provider contract (the type lands as №879): capabilities +
   limits + health semantics. A call to an LLM construct addresses the
   CONTRACT — «give me a completion within these limits, honoring this
   health state» — never a vendor name. `Provider` (the vendor enum),
   `RealLlm`, and `MockLlm` become IMPLEMENTATIONS of the contract;
   `SmartRouter` becomes the contract's dispatcher (failover over
   interchangeable ranks). The contract NAME is deliberately distinct
   from the existing `enum Provider` (the dedup guard — the name is
   fixed in this record; the implementation detail is №879's).
2. **The local provider is a first-class provider.** `MockLlm` — the
   deterministic, non-echo, offline backend — is re-declared as the
   LOCAL PROVIDER of the contract (not a test-only stub): it implements
   the same capabilities/limits/health semantics, it is traceable
   (the trace names it honestly), and it answers with the frozen
   deterministic marker. A new dedicated local default (the LocalProvider
   rank — №879's detail) serves the local-first default path; the two
   names stay apart mechanically.
3. **Local-first default.** The absence of configured external
   providers is the NORMAL mode: `llm {}` with no provider entries
   (and the empty providers list) resolves to the local provider by
   default — the network is an OPTIMIZATION, not a precondition. An
   explicit external provider is an explicit declaration (the existing
   `llm { providers: [...] }` grammar — unchanged, grammar.pest:24–25).
   This is a SMART-ROUTER-LANE posture: the legacy no-llm{} path keeps
   the №454 contract EXACTLY (mock only when `METALOGOS_MOCK_LLM=1|true`
   is explicit; otherwise the real backend fails loudly without
   credentials) — the local-first default never silently re-introduces
   the mock-by-default leak №454 closed, because the local rank lives
   inside an EXPLICIT `llm {}` declaration and its answers carry the
   non-echo deterministic marker.
4. **The surface does not move.** The `llm {}` grammar, the field set
   (№490), the `model → default_model` resolution (№819), the
   failover-`auto` semantics, the circuit_breaker threshold meaning,
   and the trace/Ledger shapes stay AS THEY ARE: the contract layer is
   a re-declaration OVER the living parents (the 1:1 migration — §6).
   The observable behavior of every existing LLM test is preserved;
   the four surface tests stay green WITHOUT golden edits.
5. **The refusal posture does not grow.** The 14-word vocabulary stays
   frozen (№385/ADR-0169): the layer's refusals ride the EXISTING
   classes — `LLM_PROVIDER_UNAVAILABLE` (the connect-failure class,
   llm.rs:930) for the unavailable provider surface, `LLM_TIMEOUT`
   for the deadline breaches. No new DenyEvent word, no new code.

## §3. The portable-kernel boundary — what the provider-agnostic layer is and is not

- **IN:** the contract types (capabilities, limits, the health
  semantics view), the local-first default rule (the explicit-llm{}
  local rank), the failover semantics re-read OVER the contract (the
  same behavior, now named as contract dispatch), the circuit_breaker
  as a CONTRACT behavior (threshold → open → half-open recovery — the
  ProviderHealth machine, unchanged, now part of the contract's
  vocabulary), and the DEFINITION of the OFFLINE_SUITE machine fact
  (§5; the implementation is №881).
- **OUT:** new provider integrations and vendor names (no new HTTP
  surfaces, no new SDK branches — the provider ladder of ADR-0037
  stays closed to additions in this subphase), the network transport
  policies (the egress gate of ADR-0207/№874 governs the shared I/O
  boundary — the coverage rule of №880 is about LLM FALLBACK PATHS,
  never a second egress gate), the media/perception domains (their own
  subphases), the grant mechanics (the capability layer ADR-0203 is
  untouched — the LLM contract never issues, widens or consumes a
  right), the field renames (№819 landed; `model → default_model`
  stays).
- **The refusal vocabulary does not grow** (§2.5).

## §4. The consumption contracts — what №879–№884 land

- **№879 — the contract in code:** the provider-contract type over the
  living parents (`Provider`/`RealLlm`/`MockLlm`/`ProviderHealth`/
  `SmartRouter` — the llm.rs:240/327/69/1177/1494 facts); the
  local-first default (the empty providers → the local rank); the 1:1
  migration (the SmartRouter lane keeps the observable behavior; the
  four surface tests green WITHOUT golden edits); the negative
  controls: an undeclared provider refuses loudly with the EXISTING
  class, the circuit_breaker threshold is honored by the contract, and
  the configuration serialization double-run is byte-for-byte.
- **№880 — the coverage rule:** the fail-closed provider-dependency
  gate — an LLM call addressed to an external provider WITHOUT a
  covered local fallback path refuses BEFORE the network with the
  EXISTING class; the e2e trace online → deterministic
  refusal/timeout (stub) → failover chain → local fallback → result;
  the backend-agnostic posture (vm and interpreter give one outcome —
  the #1735 misdispatch class does not reproduce here).
- **№881 — the machine fact OFFLINE_SUITE:** the share (bp) of the
  REGISTERED LLM-surface checks that run without external network
  (the local/stub provider, the deterministic fixtures — offline);
  the registry is the explicit machine source (the gate_039_goals.txt
  form — the №862 roster posture); the gate_039 floor 8000 bp,
  only-up, with the tamper traps (the gate_035/036/037/038 лекало —
  the №875 form).
- **№882 — the baseline ride:** the type_signature /
  sensitive_surface baselines re-pinned under the llm contract types
  («the floor follows the fact» — the №863/№870/№876 form); the
  advisory `::warning` never appears on main.
- **№883 — the deps ride:** rusqlite 0.40.2 → 0.41.0 (the dependabot
  PR #1658; the owner's verdict 2026-11-02) with the full CI; the
  rustyline blocker (#1544) re-checked on the execution date (the
  ride pair).
- **№884 — the weekly record:** the wave-78 entry by the №789
  generator, strictly last (the N-rule).

## §5. The OFFLINE_SUITE definition — the 0.41 machine fact (the implementation is №881)

- **OFFLINE_SUITE** = the share (in basis points) of the registered
  LLM-surface checks that EXECUTE WITHOUT EXTERNAL NETWORK: the check
  drives the LLM surface (the grammar, the router, the failover loop,
  the limits, the contract) through a local/stub provider or a
  deterministic fixture — no vendor endpoint is contacted, no
  credential is consulted.
- **The denominator** is the explicit registry of the LLM-surface
  checks (the machine source — the file the checker reads, the №862
  roster form); a check is counted in the numerator ONLY when its
  registry row carries the offline fact (offline=yes) — the
  self-declaration without the registration row is uncovered by
  construction.
- **The floor** is 8000 bp (the wave-78 dispatch constant, gh#1745:
  0.41 = 8E-1 (C5, OFFLINE_SUITE ≥ 8000 bp)) — only-up, with the
  floor-twin and the tamper traps (the №875 form): a lowered floor
  refuses loudly; a forged roster row lowers the share (the
  classification/machine source is the registry file, and the traps
  prove the counter catches the forgery).
- **The honest boundary:** OFFLINE_SUITE measures the TEST/CI coverage
  of the LLM surface, not the runtime behavior; a rung that dials a
  real vendor in production is outside this fact's scope (the runtime
  network posture is the egress gate's domain, ADR-0207).

## §6. The migration discipline — 1:1, byte-identical surfaces

- №879 lands the contract OVER the existing types: no grammar change,
  no field rename, no trace-shape change, no golden edit (the
  №838/№854/№860/№867/№873 1:1 pattern); the four surface tests
  (`llm_cache_contract`, `naryad_248_llm_cancellation`,
  `naryad_757_llm_limits`, `naryad_819_provider_model`) stay green
  with ZERO golden edits.
- The parents are consumed by reference: the grammar stays the surface
  SSOT (№490), the router stays the failover SSOT (№4/№156), the
  health window stays the circuit SSOT, the mock marker stays the
  deterministic SSOT (№454/№487), the cache stays the call-count SSOT
  (ADR-0047). The contract is a NAMED VIEW over them; a drift between
  the view and a parent is a loud verification failure, never a
  silent overwrite.
- The backend-agnostic posture (the #1735 lesson) holds: the contract
  and the coverage rule of №880 dispatch identically on the vm and the
  interpreter lanes — the e2e trace of №880 runs green on both.

## §7. Acceptance

- Merges №878–№884 in Metalogos- with the green CI of the repository
  on every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The surface gate: the `llm {}` grammar, the field set, the trace
  shape and the existing LLM goldens unchanged — the four surface
  tests green WITHOUT golden edits; the 14-word vocabulary unchanged;
  the wave-77 egress surface not duplicated.
- The trace gate: the e2e online → refusal → failover → local fallback
  green on BOTH backends; the configuration serialization double-run
  byte-for-byte.
- The 0.41 gate: OFFLINE_SUITE in `gen_metrics --check`, floor 8000 bp,
  only-up; the tamper traps live.
- The baseline hygiene: the advisory `::warning` absent from main (the
  №882 re-pin, the «the floor follows the fact» form of №569).
