# ADR-0209: Jurisdiction-Aware Policy Compilation — the Jurisdiction Limit-Configuration as a Typed, Verifiable Artifact (the 8D-2 / C10 Subphase Record)

- Status: **Accepted** (the wave-79 acceptance — OFFLINE_SUITE
  10000 bp at the floor 8000, gate_039; the dispatch gh#1764 closed
  2026-11-03 with the acceptance comment)
- Date: 2026-11-03
- Deciders: the OWNER (the R38 train — 0.42 = 8D-2 (C10,
  JURISDICTION_MATRIX ≥ 3); the wave-79 dispatch gh#1764; the «Заливай
  следующую волну» directive of 2026-11-03)
- Naryad: №885 (issue #1758, Wave 79 — W-H; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38 — the C10 frame
  decision: «конфигурация ограничений, не юридический совет»; the KPI
  replacement: 100% of the package rules carry a norm reference and a
  verification date); the R57 GET-only fact-check over live main
  `2e4725a83d97` (2026-11-03)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.42 is this record's train slot),
  [ADR-0196](0196-mcp-policy-gateway.md) (the MCP policy compilation —
  the C10 parent: the jurisdiction narrowing rides the SAME
  compile-site, never a parallel one), docs/feature-gates.md (the
  feature-flag surface — the second C10 parent, №676),
  [ADR-0202](0202-model-passport.md) (the model passport — the
  license-typed artifact the jurisdiction limits scope),
  [ADR-0206](0206-compliance-evidence-compiler.md) (the attestation
  map — the append-only store form this record follows),
  [ADR-0203](0203-capability-token.md) (the capability contour — the
  rights SSOT this record never widens),
  [ADR-0207](0207-provenance-native-io.md) (the egress gate of wave 77
  — consumed, never duplicated), [ADR-0169](0169-stable-try-codes.md)
  (the frozen 14-word refusal vocabulary), №385 (the frozen refusal
  vocabulary), №828/№868 (the digest-pinning and the canonical-row
  forms), №886–№890 (the naryads that land on top of this record)

## §1. Context — the policy surface decides globally, with no jurisdiction axis

The policy surface is alive and stable on main (verified R57,
`2e4725a83d97`): `docs/feature-gates.md` pins the feature-flag surface
(the `svg`/`chart`/`llm`/`server`/`candle`/... table — the front-door
record of what the binary can reach), and the MCP policy compiler
(`metalogos-server/src/mcp_policy.rs` — `ToolPolicy` :35,
`compile_policy` :60, ADR-0168 §4) derives, per tool method, the audited
sink calls, the ingress points, the clearance-relevant arguments and the
irreversible flag — one GLOBAL answer, jurisdiction-blind. The adjacent
typed carriers exist: the model passport (ADR-0202), the capability
token (ADR-0203), the attestation map (ADR-0206) with the canonical-row
digest form (№828/№868) and the append-only store discipline.

The honest gaps this record closes:

- **There is no jurisdiction axis.** The same binary, the same package
  and the same policy answer identically in every jurisdiction. The
  strategy plan's C10 names the missing dimension: a compliance
  artifact must be able to express «in jurisdiction X, carrier Y is
  limited to Z» — as a CONFIGURATION of limits, deterministically
  compiled into the existing policy decisions, before anything runs.
- **The limit configuration has no typed form.** Nothing in the type
  system says what a jurisdiction rule IS; a limit that lives in prose
  cannot be compiled, cannot be pinned by a digest, and cannot refuse
  loudly when it is incomplete.
- **The KPI is not machine-checked.** The plan replaced the C10 KPI
  with a measurable one: 100% of the package rules carry a norm
  reference AND a verification date. Without a fail-closed compile-site
  check that number is a declaration, not a fact.
- **No machine fact pins the coverage breadth.** Nothing measures how
  many jurisdictions the matrix actually covers — a future regression
  that silently drops a jurisdiction would drift unnoticed.

## §2. Decision — the jurisdiction matrix is typed data; the compilation is fail-closed; the frame is limits, not legal advice

The C10 record adopts four decisions:

1. **The rule is a typed artifact.** `JurisdictionRule` is a struct:
   the jurisdiction (a validated code), the carrier/subject (the
   existing carrier name space — the same names `ToolPolicy` and
   `builtins_classification` speak), the effect (`allow` / `deny` /
   `limit`), the `norm_ref` (a REFERENCE to a norm — the rule carries
   the pointer, never the legal text), the `checked_at` (the
   verification date, ISO date), and the source. Every field is part
   of the canonical row; the canonical row is digest-pinned (the
   №828/№868 form — SHA-256 over the `\x1f`-joined fields).
2. **The matrix is append-only.** `JurisdictionMatrix` follows the
   AttestationMap discipline (ADR-0206 §2.2): issuing adds, nothing
   deletes; the rows stay in issue order; the export sort is the
   compiler's contract. A changed rule is a NEW row that supersedes —
   never a silent overwrite.
3. **The compilation is fail-closed at the existing compile-site.**
   `compile_jurisdiction_policy(matrix, jurisdiction)` produces the
   typed policy view; the MCP layer narrows its `ToolPolicy` through it
   (the ADR-0196 surface — one composition, not a parallel gate). A
   rule without `norm_ref` or `checked_at` refuses the compilation
   BEFORE anything runs (the KPI C10 as a machine check); an unknown
   jurisdiction refuses; a jurisdiction-deny over an irreversible
   carrier refuses through the EXISTING refusal classes (№385 — the
   vocabulary does not grow).
4. **The frame is honest about its limits.** This record compiles
   LIMITS — it never interprets the content of a norm, never renders
   legal advice, never claims a jurisdiction is «compliant». The legal
   texts stay outside the repository; the `norm_ref` is a pointer the
   auditor follows, the `checked_at` is the date the pointer was last
   verified. The jurisdiction shield itself (the bus-factor contour)
   is the 8H-1 subphase (0.46), NOT this one.

## §3. The parents are consumed by reference — what this record does NOT redefine

- `docs/feature-gates.md` (№676) stays the SSOT of the feature-flag
  surface; the jurisdiction layer NARROWS it per jurisdiction and never
  flips a flag itself.
- `mcp_policy.rs` (ADR-0196/ADR-0168) stays the SSOT of the per-method
  policy compilation; the narrowing is a pure function OVER its output
  (`ToolPolicy` in → `ToolPolicy` out or a loud refusal), and the
  existing `compile_policy` results are byte-identical when no
  jurisdiction narrowing is applied (the 1:1 migration rule).
- `capability.rs` (ADR-0203) stays the rights SSOT; a jurisdiction
  effect NEVER widens a right — it can only refuse or limit.
- `model_passport.rs` (ADR-0202) and `compliance.rs` (ADR-0206) keep
  their domains; the jurisdiction matrix scopes them, does not
  re-describe them.
- The egress provenance gate (ADR-0207, wave 77) stays the network
  policy; the jurisdiction layer adds no second network gate.
- The refusal vocabulary stays frozen (№385/ADR-0169, 14 words); the
  refusals ride the existing `IRREVERSIBLE_NO_GRANT` /
  `SANDBOX_VIOLATION` classes.

## §4. The consumption contracts — what №886–№890 land

- **№886 (the kernel):** `src/jurisdiction.rs` — `JurisdictionId`
  (a validated code), `JurisdictionRule`, `PolicyEffect`, the
  append-only `JurisdictionMatrix` (issue/query/digest — the
  canonical-row form), `compile_jurisdiction_policy` producing the
  typed per-jurisdiction view; the seed matrix ≥ 3 jurisdictions
  (EU/RU/US) as code data.
- **№887 (the coverage rule):** the fail-closed checks (a rule without
  `norm_ref`/`checked_at` refuses; an unknown jurisdiction refuses;
  the valid path passes byte-identically double-run) + the
  `ToolPolicy` narrowing over the ADR-0196 surface + the e2e trace
  jurisdiction → rule → compiled policy → carrier verdict on both
  backends (TW + VM).
- **№888 (the machine fact):** `JURISDICTION_MATRIX` in
  `scripts/gen_metrics.py` + the `gate_040` floor (≥ 3, only-up, the
  floor-twin + the tamper traps — the gate_038/039 лекало); the count
  is CHECKED, not declared — a registered row counts only when the
  jurisdiction lives in the compiled matrix source.
- **№889 (the re-pin ride):** the five surface-gate baselines re-verify
  under the jurisdiction types — the floors follow the fact (the
  №863/№882 form; an honest no-drift is a valid verdict).
- **№890 (docs, strictly last):** the weekly wave record (the №789
  generator) + the sanitized 0.42 phase line.

## §5. Honest limits — what the jurisdiction layer is NOT

- **Not legal advice, not a compliance verdict.** The layer carries
  pointers and dates; it cannot say «this is legal in X». The frame is
  the plan's own: «конфигурация ограничений, не юридический совет».
- **Not a new network policy.** The egress gate (ADR-0207) stays the
  only network gate; a jurisdiction `deny` over a network carrier
  refuses through the capability/clearance path, not through a second
  firewall.
- **Not a domain-freeze breach.** The matrix is a continuation of the
  executed subsystems (feature-gates + MCP policy + the typed carriers)
  — the ADR-0177 quota is not touched (the wave class is
  `[core]/[process]`, not `[domain]`).
- **No new refusal vocabulary, no new crates.** The dependencies of
  the kernel are the crate's own (`serde`, `sha2` — both already in
  the tree).

## §6. The migration discipline — 1:1, byte-identical surfaces

The grammar, the builtin registry, the golden files and the existing
policy decisions do not change: a program that compiled and ran before
the jurisdiction layer compiles and runs identically after it — unless
the operator explicitly attaches a jurisdiction narrowing, in which
case the refusals are loud, pre-run and named. The only intentional
new surface is the typed matrix + the compile entry point; every
existing surface stays byte-identical (the double-run serialization
byte-identity is the acceptance form of №886/№887).

## §7. Acceptance

- ADR-0209 lands as Proposed and is fixed Accepted by the wave-79
  acceptance (the dispatch gh#1764 acceptance comment).
- №886–№890 land green on the merge commits (the blocking CI on every
  merge, the gh#1599 rule).
- `JURISDICTION_MATRIX` in `gen_metrics --check` ≥ 3 (only-up) with
  the tamper traps alive (the gate_040 record).
- The KPI C10 is machine-true: 100% of the seed-matrix rules carry
  `norm_ref` + `checked_at`, enforced fail-closed by №887's checks.
- The surfaces stay 1:1: no grammar, registry, or golden change; the
  refusal vocabulary stays at 14 words (№385).
