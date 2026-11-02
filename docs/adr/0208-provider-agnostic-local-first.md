# ADR-0208: Provider-Agnostic Local-First — the LLM Layer as a Provider-Agnostic Contract (the 8E-1 / C5 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-78 acceptance —
  the dispatch gh#1745 issued the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-02
- Deciders: the OWNER (the R38 train — 0.41 = 8E-1 (C5, OFFLINE_SUITE
  ≥8000 bp); the wave-78 dispatch gh#1745; the owner's deps decision of
  2026-11-02 «Рекомендация принимается» rides in №883)
- Naryad: №878 (issue #1738, Wave 78 — W-G; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R56 GET-only
  fact-check over live main `93c18673` (2026-11-02)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.41 is this record's train slot),
  [ADR-0203](0203-capability-token.md) (the capability token — the
  capability contour this record defers to, never redefines),
  [ADR-0202](0202-model-passport.md) (the model passport — the
  per-model identity layer this record stays apart from),
  [ADR-0207](0207-provenance-native-io.md) (the provenance-native I/O —
  the 0.40 subphase whose egress gate this record does NOT duplicate),
  №385 (the frozen 14-word deny dictionary — no new codes under this
  record), [ADR-0168](0168-mcp-server-transports.md) (the D3
  zero-new-crates form this record keeps),
  [ADR-0163](0163-backend-registry-licenses.md) (the license class —
  the portable kernel parent), №819 (the `model → default_model` rename already
  landed — this record renames nothing), №874 (the fail-closed gate
  form this record's coverage rule follows — backend-agnostic),
  №875 (the machine-fact + gate form OFFLINE_SUITE follows),
  №879–№884 (the naryads that land on top of this record)

## §1. Context — the LLM surface works, but the provider is not a contract

The language surface ships a live LLM layer today: the `llm {}` grammar
block (src/grammar.pest:15 — `llm { providers: [...], default_model:
"...", failover: auto, circuit_breaker: 3, timeout: 30 }`; the field
alternatives at src/grammar.pest:24–25 and the failover /
circuit_breaker productions at src/grammar.pest:39–40), and the
implementation core src/llm.rs (3853 lines) with the live parents —
`enum Provider` (src/llm.rs:240), `RealLlm` (src/llm.rs:327),
`MockLlm` (src/llm.rs:69), `ProviderHealth` (src/llm.rs:1177),
`SmartRouter` (src/llm.rs:1494) — and the failover / circuit-breaker
machinery spread across the module. The test base holds
llm_cache_contract, naryad_248_llm_cancellation, naryad_757_llm_limits,
naryad_819_provider_model.

What the layer does NOT have is the contract line drawn. Today a call
into the LLM surface is shaped by what a concrete provider is, not by
what the surface needs from A provider: the capabilities, the limits
and the health semantics live as implementation details of the
concrete types, and `MockLlm` sits in the code as a test stand-in
rather than as a first-class local provider. The local-first postulate
— that work WITHOUT an external network is the normal operating mode,
not a degraded one — is nowhere fixed: nothing states that an
unconfigured `llm {}` block must resolve to a local provider by
default, and nothing states that reaching an external provider is an
optimization the configuration opts INTO.

The R56 fact-check confirms the gap is real and the ground is clean:
OFFLINE_SUITE has 0 hits across main (PLAN-SUMMARY, CHANGELOG,
gen_metrics); the `model → default_model` rename already landed (№819)
so this subphase renames nothing; the ADR-0208 slot is free (the
0203–0214 train booking holds 0201–0207 as taken; 0216 sits outside the
booking) and gate_039 is free (the gate files run to 038).

## §2. Decision — the call addresses the contract; local is the default

**The provider-agnostic contract.** A call into the LLM surface
addresses a PROVIDER CONTRACT — the triple of (a) capabilities (what
the provider can serve), (b) limits (the timeout / max_tokens /
circuit_breaker semantics the grammar already carries), (c) health
semantics (what ProviderHealth tracks, in contract terms) — and not a
vendor name. `enum Provider`, `RealLlm`, `MockLlm` become
implementations OF that contract; they keep their names, their
signatures and their observable behavior (the migration is 1:1, §6).
The local/deterministic provider is a first-class implementation —
`MockLlm` is the local provider of the surface, not a test-only
stub; the contract names it as such.

**The local-first postulate.** The absence of external provider
configuration is the NORMAL mode: an `llm {}` block (or its absence)
with no `providers:` declaration resolves to the local provider by
default. Declaring an external provider is an explicit opt-IN — the
network is an optimization the configuration asks for, never a
prerequisite the surface assumes. The grammar (src/grammar.pest:24–25)
does not change: `providers: []` (the empty list) and the absent field
both mean «local only».

**What this record does NOT decide.** It fixes the contract line and
the default posture; it lands no provider integration, no vendor name,
no network policy (see §3). The observable behavior of the existing
LLM tests is frozen — the §7 gate holds the goldens unedited.

## §3. The portable-kernel boundary — what the contract layer is and is not

**Enters the portable kernel:**

- The provider contract type — capabilities + limits + health
  semantics, with the local provider as a first-class implementation.
- The failover semantics AS CONTRACT BEHAVIOR — the failover chain
  (src/grammar.pest:39) and the circuit_breaker threshold
  (src/grammar.pest:40) are contract-level semantics every
  implementation honors, not provider-level accidents.
- The local-first default rule — the unconfigured/empty `providers:`
  resolution to the local provider (the §2 posture, machine-checked by
  the negative controls of №879).
- The OFFLINE_SUITE fact-machine definition (§4.д) — the definition
  lives in this record; its implementation is №881's.

**Stays OUT of this record's kernel:**

- New provider integrations and vendor names — the contract is
  provider-agnostic BY POINT; adding concrete external providers is
  outside the subphase and outside this record.
- Network policies and egress — the provenance egress gate of №874
  (wave 77) already governs the network boundary; this record does not
  duplicate it. The coverage rule of №880 is about LLM fallback PATHS,
  not about the network perimeter.
- Media / perception domains (the ADR-0166 / ADR-0164 contours) —
  separate domains with their own carriers.
- The capability contour — ADR-0203 governs it; the contract defers to
  it, never redefines it.

## §4. The consumption contracts — what №879–№883 land

- **№879** lands the contract in code: the contract type over the live
  parents (Provider/RealLlm/MockLlm at src/llm.rs:240/327/69), the
  local-first default at the SmartRouter entry (src/llm.rs:1494), the
  1:1 migration (§6), the negative controls (the unregistered provider
  call → the loud refusal in the existing class; the circuit_breaker
  threshold honored THROUGH the contract; the double-run
  serialization byte-identity).
- **№880** lands the coverage rule: the fail-closed gate — an LLM call
  addressed to an external provider without a covered local fallback
  path refuses BEFORE the network, in the existing refusal class (the
  №385 frozen dictionary; no new deny codes); the e2e trace
  request → external refusal/timeout (stub) → failover chain → local
  fallback → result, green on both backends, double-run
  byte-identical.
- **№881** lands the machine fact: OFFLINE_SUITE in gen_metrics.py —
  the share (bp) of the registered LLM-surface checks executable
  without external network; gate_039 — the 8000 bp floor, only-up,
  with the tamper traps (the №875 pattern).
- **№882** re-pins the type_signature / sensitive_surface baselines
  under the llm-types the code lands (the «the floor follows the fact»
  form of №863/№870/№876).
- **№883** rides the deps: rusqlite 0.40.2 → 0.41.0 (the dependabot PR
  #1658, the owner's 2026-11-02 decision) and the rustyline #1544
  re-check — the ride-pair, outside this record's kernel but inside
  the wave.

### §4.д The OFFLINE_SUITE definition (the machine fact this record fixes)

OFFLINE_SUITE is the share (in basis points, 0–10000) of the
REGISTERED LLM-surface checks that execute without reaching an
external network — the registry is the wave's gate_039 goals file
(№881), a check is offline=yes when it runs against the local
provider, a deterministic stub, or a deterministic fixture, and
offline=no when the check requires a live external endpoint to
execute. The floor is the owner's 8000 bp (the wave-78 dispatch);
the gate is only-up. The definition is the record's; the registry, the
counter and the tamper traps are №881's — the definition does not
decide the roster, the roster decides the fact.

## §5. Honest limits — what this record is NOT

- Not a provider integration plan: no vendor, no endpoint, no SDK
  enters the tree under this record (the D3 no-new-crates form of
  ADR-0168 holds for the whole wave).
- Not a network-policy record: the egress perimeter belongs to the
  wave-77 provenance layer (№874); if the two touch, the provenance
  gate wins — the LLM fallback paths route INSIDE it.
- Not a capability change: the ADR-0203 contour is referenced, not
  redefined; no new capability class, no new grant shape.
- Not a grammar change: the `llm {}` productions stand as they are
  (src/grammar.pest:15/24–25/39–40/48); the default_model field is not
  renamed (№819 settled it).
- Not a behavior change: the 1:1 discipline (§6) is the acceptance
  gate, not an aspiration — the existing LLM tests pass WITHOUT
  golden edits, the refusal vocabulary stays at 14 words (№385), the
  configuration serialization is byte-identical across runs.
- The record fixes the OFFLINE_SUITE DEFINITION, not its value: the
  8000 bp floor is the owner's wave constant; the fact at the moment
  of landing is whatever the registered roster says — the floor only
  ever moves up.

## §6. The migration discipline — 1:1, byte-identical surfaces

- The existing call sites route through the contract without
  observable change: the SmartRouter path (src/llm.rs:1494) keeps its
  dispatch order, its health accounting (ProviderHealth,
  src/llm.rs:1177) and its refusal texts.
- The existing LLM tests are the migration's verifier:
  llm_cache_contract, naryad_248_llm_cancellation, naryad_757_llm_limits,
  naryad_819_provider_model pass WITHOUT golden edits — a test edit to
  make a green test pass is a red flag, not a migration step.
- The configuration serialization double-run is byte-identical (the
  №874/№869 form) — the contract adds no nondeterminism to the
  config output.
- The local-first default changes NOTHING for the configurations that
  already declare providers; it only gives the unconfigured surface a
  defined, loud, local answer where before the behavior was
  configuration-dependent.

## §7. Acceptance

- Merges №878–№884 in Metalogos- with the green CI of the repository
  on every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The surface gate: the `llm {}` grammar, the Provider/RealLlm/
  MockLlm/ProviderHealth/SmartRouter signatures, the existing LLM
  goldens and the 14-word refusal dictionary unchanged — the existing
  LLM tests green WITHOUT golden edits; the egress surface of wave 77
  not duplicated.
- The trace gate: the e2e online → refusal/timeout → failover → local
  fallback green on both backends (№880); the double-run serialization
  byte-identical.
- The 0.41 gate: OFFLINE_SUITE in `gen_metrics --check`, floor 8000
  bp, only-up, the tamper traps live (№881).
- The baseline hygiene: the advisory `::warning` absent from main (the
  №882 re-pin, the «the floor follows the fact» form of №569).
- The deps ride: rusqlite 0.41.0 merged inside the wave with the full
  CI (№883); #1544 closed or honestly re-dated.
