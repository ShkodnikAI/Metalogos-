# ADR-0204: Portable Refusal Algebra — the Refusal Algebra as a Portable Artifact (the 8F-1 / C6 Subphase Record)

- Status: **Accepted** (the owner decision of 2026-10-28 — the plan
  revision R38 fixed 0.37 = 8F-1, the C6 "Portable Refusal Algebra"
  subphase with the SEMA_EXTERNAL ≥ 1 gate; the wave-74 dispatch gh#1684
  issued the subphase's naryads the same week; the wave acceptance fixes
  the ADR status as Accepted)
- Date: 2026-10-31
- Deciders: the OWNER (the R38 revision — the train slot 0.37 and the
  SEMA_EXTERNAL ≥ 1 floor are the plan §23 numbers, accepted in full;
  the wave-74 dispatch gh#1684)
- Naryad: №853 (issue #1678, Wave 74 — W-C; ADR before the first code of
  the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R49 GET-only
  fact-check over live main `58e300f8` (2026-10-30)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.37 is this record's train slot),
  [ADR-0169](0169-try-stable-error-codes.md) and №385 (the frozen stable
  refusal codes — this record REFERENCES them, never redefines),
  [ADR-0167](0167-action-ledger-v1.md) (the Action Ledger v1 — the event
  binding this record must not perturb), [ADR-0197](0197-vm-single-executor.md)
  (the TW/VM parity posture — the №829 lesson), №392 (the DenyEvent
  surface — one carrier of the algebra, not its definition), №567 (the
  crate-split stage 2 — the pure-library root this record builds on),
  №854–№858 (the code naryads that land on top of this record)

## §1. Context — the refusal surface lives, but it is not portable

The refusal surface of the language is real and audited: `DenyEvent` is
alive at 40 sites in `src/` (`deny.rs`×9, `ast.rs`×2, `audit.rs`,
`audit_ops.rs`×3, `builtins/registry.rs`×3, `builtins_classification.rs`
and the interpreter/VM construction paths), the reason vocabulary is the
fixed 14-word `DENY_REASONS` table (№392; R47 — `IRREVERSIBLE_NO_GRANT`
is the core class), and the stable refusal codes are frozen contracts
(№385, ADR-0169; the spec README:34–45 — «The stable refusal codes are
frozen», the `CODE: <stable refusal code>` form).

What does not exist is the algebra AS A VALUE. `RefusalKind`,
`RefusalCode`, `refusal_algebra`, `RefusalAlgebra` — 0 hits on
`58e300f8`. A refusal today is a constructor call deep inside the
language runtime (`deny.rs::make_event` behind the interpreter hooks and
the VM twin): an external consumer — another process, another language,
a CI platform — cannot COMPUTE a refusal verdict without lifting the
whole language toolchain. The refusal is an event stream to observe, not
an algebra to evaluate.

R38 raised C6 to the 0.37 train slot with the verdict: «спека + WASM +
Action открывают воронку принятия (Мастер-план §5) и страхуют провал
языка — единственная линия, платящая даже при неудаче языка». The
parent preconditions are done: the crate split stage 2 (№567) left the
root a pure library, and the frozen-code surface is stable since №385.

## §2. Decision — the refusal is a value of a finite lattice, not a log line

The algebra is defined OVER the existing frozen vocabulary — this record
aggregates, it does not extend:

- **The reason set** is the fixed 14-word `DENY_REASONS` table (№392) —
  the same words the audit check_ids use. No new reasons, no new codes
  (§6); the lattice is CLOSED over this set.
- **The severity order** is the canonical №392 array order: the seven
  core classes first, then the extending pair, the №455 file-path class,
  the generic class, the legacy corpus classes last. Lower index = more
  severe (the core classes dominate the legacy ones). This is a
  DEFINITION of this record (the array order was presentation, it is
  now the rank), fixed by the table itself so that every consumer ranks
  identically.
- **The lattice**: the reasons form a chain (a total order). `join(a, b)`
  is the MORE SEVERE cause (the lower rank) — when two refusals
  combine, the dominant class carries the verdict. `meet(a, b)` is the
  LESS SEVERE cause. Chain ⇒ commutative, associative, idempotent, with
  absorption — all machine-testable over the full 14×14 table.
- **The verdict** is `Allow | Deny { code, cause }`. `Allow` is the
  bottom element (`join(Allow, x) = x`, `meet(Allow, x) = Allow`); a
  `Deny` carries the frozen stable refusal code — for gate refusals the
  code IS the reason word (the №392 check_id = the code), for grant
  refusals `cause` is `IRREVERSIBLE_NO_GRANT` and the typed `GRANT_*`
  detail stays the producer's `human` payload exactly as №390 folded it
  (the algebra does not re-parse it — §6).
- **The refusal ≠ a log line**: the verdict is a VALUE — computable,
  comparable, serializable — and the event stream (`DenyEvent`, the
  Ledger) is one of its CARRIERS, not its definition.

## §3. The portable-kernel boundary

**Enters the portable kernel** (the `wasm32`-compilable surface, №855):

- the reason vocabulary (the 14-word table) and its rank;
- the lattice operations `join` / `meet`;
- the verdict type `Allow | Deny { code, cause }` and its constructors;
- the JSON rendering of a verdict for the consumption contracts (§4).

**Stays outside the kernel** (and outside any `wasm32` facade):

- IO of every kind — files, network, clock, environment;
- the Action Ledger (the Ledger is a host-side carrier; the kernel
  computes verdicts, it does not record them);
- the language compiler, the parser, the VM, the interpreter — the
  kernel must never pull the language dependency tree (the №567
  pure-library discipline, applied to the facade);
- the taint machinery and the label lattice — the kernel consumes
  VERDICT CAUSES, it does not re-derive taint.

## §4. The consumption contracts — the three legs of the external artifact

The algebra becomes externally consumable through exactly three
contracts (the R38 «воронка принятия»); each is a separate naryad and
each is machine-verifiable:

1. **The spec page** (№858) — the normative text an external consumer
   reads FIRST: the lattice, the verdict, the frozen codes BY REFERENCE
   to №385/ADR-0169 and spec topic 3 (S-ERR-001…006 — the spec refers,
   it does not redefine), the WASM/Action consumption contracts.
2. **The WASM contour** (№855) — the kernel compiled to
   `wasm32-unknown-unknown` behind a C-ABI/JSON boundary: any host (a
   Node script, a Python tool, a CI runner) evaluates `join`/`meet`/
   verdicts without a Rust toolchain. The smoke consumer runs in the
   main CI — the leg is alive, not a build log.
3. **The reusable Action** (№856) — `policy-check`, a composite GitHub
   Action: any repository evaluates a refusal policy (a JSON of
   lattice requests) in its own CI through the algebra. The dogfood
   job in THIS repository's CI is the living external consumer.

`DenyEvent` is ONE carrier of the algebra — the runtime surfaces keep
their byte-identical event shape and Ledger records (the §5 migration
discipline of №854); the algebra is the definition layer OVER them.

## §5. SEMA_EXTERNAL — the machine fact of external adoption

The 0.37 gate is a counted fact, not a claim. `SEMA_EXTERNAL` (the
implementation — №857, the shape of №846/№735: the fact rides with its
checker) is the number of externally consumable algebra artifacts that
EXIST AND ARE LIVE on main, each counted ONLY on a real existence check
plus a live wiring in the blocking CI workflow:

- the WASM facade: the crate exists AND the `ci.yml` build job is
  active AND the smoke consumer step is present;
- the reusable Action: `action.yml` exists AND the dogfood job is
  active and green on main;
- the spec page: the topic exists in `docs/spec/` with its conformance
  pairs (№858).

No self-declaration: a file without a live job is NOT counted (the
tamper traps of №857 prove the counter catches placeholders). The floor
is `goal_sema_external_owner >= 1` in `scripts/ci/gate_035_goals.txt` —
only-up (the №846 discipline: the floor moves by evidence, never down;
a lowered floor is a loud refusal). The 0.38 subphase does not open
until this gate is GREEN (the №630 pattern).

## §6. Honest limits — what the algebra is NOT

- **No new codes.** The vocabulary is the frozen 14; the codes are the
  frozen №385/ADR-0169 set. Extending the reason set is a
  semantics-change naryad with the owner's gate — never a silent
  algebra edit.
- **Not IO, not network, not a policy engine.** The kernel computes
  verdicts over the causes it is GIVEN; it does not observe the world.
- **Not a formal verification of the compiler.** The lattice laws are
  tested over the finite table — a proof of the table's consistency,
  not of the language's safety.
- **Not a behavioral change.** №854 migrates the `DenyEvent`
  construction 1:1: the same inputs produce the same events and the
  same Ledger records byte-for-byte (the ledger-golden and conformance
  suites must pass with ZERO golden edits). The legacy passthrough for
  a non-vocabulary reason word stays (unreachable from the gate
  dictionary — pinned by test) so the migration can never invent a new
  failure mode on the runtime path.
- **The GRANT_* detail is not re-derived.** The algebra carries
  `IRREVERSIBLE_NO_GRANT` as the cause; the typed `GRANT_EXHAUSTED` /
  `GRANT_EXPIRED` / `GRANT_REVOKED` / `GRANT_ESCALATION` wording remains
  the producer's `human` payload (№390), outside the portable kernel.
- **Publication is not adoption.** Publishing the facade as an npm/crate
  package or a Marketplace action is the owner's act; until then
  SEMA_EXTERNAL counts live artifacts on main, not downloads.

## §7. Acceptance

- Merges №853–№858 in Metalogos- with the green CI of the repository on
  every merge commit (the gh#1599 rule); auto-merge on every PR at
  creation (the wave-71 lesson).
- The DenyEvent surface and the Ledger records unchanged: the
  ledger-golden and conformance suites green WITHOUT golden edits.
- The portability gate: the wasm facade builds in CI, the smoke test
  from the host is green on main; the Action dogfood run is green on
  main.
- The 0.37 gate: SEMA_EXTERNAL in `gen_metrics --check`, floor ≥ 1,
  only-up; the tamper traps live.
- The spec page carries the conformance pairs on the real refusal
  surface; the docs-metrics contour stays in sync.
