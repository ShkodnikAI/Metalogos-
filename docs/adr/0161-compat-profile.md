# ADR-0161: Legacy compatibility profile (`profile legacy`)

**Status:** Accepted
**Date:** 2026-09-15
**Naryad:** №325 (issue #419, plan v2 §16.3 / Волна 1 · Фаза 1)
**Supersedes:** the reserved stub written by №319 (booking note in git history)
**Absorbs:** №314 v1 (the legacy-compat slice of the wave-1 issue #400, folded per dispatch #424)

## 1. Context

The label lattice (ADR-0154), the statement-level inference (№323), and the sink-gate vocabulary make it possible to enforce, statically and at compile time, that data entering a sink builtin carries a label the sink can accept. The №316 SSOT classification names every sink (`Role::Sink`); the №317 leak-suite corpus pins the negative scenarios with their expected failure classes.

A strict-by-default gate has a migration cost: legitimate programs written before the lattice exist that relay user-provided data into outputs — their whole point (e.g. the webhook demos `p7_json_body`, `p8_route_patterns`). Cutting them off at compile time with no migration path would make the lattice a cliff instead of a gate.

## 2. Decision

### 2.1 The gate: `SINK_CLEARANCE` (Category-A)

- At every call site of a **classified sink** (the list is read from `builtins_classification::classify` — `Role::Sink`; never hand-written), each argument's inferred label (№322 annotation / №323 flow inference / №325 literal markers) is checked against the sink's clearance.
- Default clearance: **`public`**. `poisoned` clears no sink at all (ADR-0154 §2.1).
- Specialized classes (the leak-suite vocabulary):
  - `VOICE_EGRESS_UNCONSENTED` — voice egress without a consent scope (consent *sources* are Phase 2, №335; until then every voice egress is unconsented by default — loud by design);
  - `IRREVERSIBLE_NO_GRANT` — a destructive SQL literal (`DROP`/`DELETE`/`TRUNCATE`/`ALTER`) in `db_execute` (the grant algebra is Phase 3, №339 — until then destructive literals are loud);
  - `UNTRUSTED_EXEC_DECISION` / `SECRET_TO_EXEC` — untrusted data driving / private labels entering `exec`/`exec_argv`;
  - `SECRET_EGRESS_VCS` — private labels into `git_push`;
  - `SECRET_EGRESS_NETWORK` — a private-infrastructure destination marker in the address position of a network sink;
  - `PII_EGRESS_NETWORK` / `PII_EGRESS_OUTPUT` — personal-data labels into network / public outputs;
  - `UNTRUSTED_EGRESS_NETWORK` — untrusted labels into network sinks;
  - `TAINT_PERSISTENCE` — untrusted data written to persistent memory (`memorize`/`mem_set`/`mtree_store`/`kv_set` — inherits the legacy class);
  - `HTML_INJECTION` — untrusted data into a public output (inherits the legacy class);
  - `SECRET_LEAK` — private labels into file sinks (inherits the legacy class);
  - `SINK_CLEARANCE` — every other confidentiality excess.

### 2.2 Literal confidentiality markers (ADR-0161 §3)

String literals carrying personal-data markers (passport / SNILS / diagnosis / confidential wording — a deliberately small bilingual vocabulary, plus structural digit shapes for RU passport `dddd dddddd` and SNILS `ddd-ddd-ddd`) or private-infrastructure URL markers (`internal`, `intranet`, `corp.`, `private`, `secret`) are seeded `private, trusted`. Sound by conservatism: a literal without markers stays bottom, so plain programs keep a zero behavioral delta.

### 2.3 The compatibility profile: `profile legacy { egress: permissive_with_audit }`

A program-level declaration. Under `legacy` the `SINK_CLEARANCE` gate runs **advisory**:

- compilation and execution stay green (Severity::Info is not promoted by №98);
- every gate hit is recorded as an **audit event**: a `Severity::Info` finding in the `mlog audit` report AND a `[SINK_CLEARANCE][audit-event]` line on the compile/run stderr;
- the event count is the burn-down metric for the migration.

## 3. Lifecycle and exit criterion

`legacy` is a **migration bridge, not a residence**. Exit criterion, per program: the `profile legacy` declaration is removed when every flow that hit the gate is either (а) redacted/annotated to pass the strict gate (redact is the one legitimate downward path, №326) or (б) demonstrably dead. The audit event count makes the burn-down measurable; a program whose event count is stuck at a non-zero value across releases is a standing debt, visible in the audit report.

The profile does not weaken ANY other gate: SECRET_LEAK, HTML_INJECTION, UNTRUSTED_FRAME, SQL_DYNAMIC, MEDIA_SYNTHETIC_UNMARKED and the rest of Category A stay at full strength; only the №325 clearance hits are converted to events.

## 4. Rejected alternatives

- **Opt-in strictness** (the gate active only under a hypothetical `profile strict`): would leave every pre-lattice program silently ungated — the corpus holes would stay open by default. Rejected: strict-by-default, opt-out via `legacy`.
- **Silent permissiveness under `legacy`** (no audit events): a bridge nobody can see is a residence. Rejected: every hit must be an event.
- **A global (CLI/env) profile switch**: profiles are per-PROGRAM declarations — the migration state belongs to the program, not the operator's shell. Rejected.
- **Hand-written sink lists**: №316's SSOT map is the single source of truth; a second list would drift. Rejected — the gate reads the classification.

## 5. Consequences

- The leak-suite corpus flips to BLOCKING: 100% of the negative scenarios are caught at compile time with their expected classes (28/28 on the merge commit); the positives keep compiling and running.
- Pre-lattice demos that intentionally relay request data into outputs declare `profile legacy` (the two corpus examples) — the honest migration path.
- №326 (redact/declassify) owns the only sanctioned downward move; №327 generalizes the integrity dimension beyond the exec/network special cases; №328 mirrors the gate into the VM.

## 6. Verification

- `cargo test naryad_325` — 16 tests: the red/green scenario (strict Error vs legacy compile+events), classification-backed sink list, the specialized classes, `poisoned` clears nothing, redact-before-sink passes, zero delta for plain programs, loud unknown profile words, the corpus closure recomputed independently.
- `cargo test --test run_leak_suite` — BLOCKING mode: 28 caught / 0 not caught / 0 mismatches.
- The two corpus demos compile under `profile legacy` and produce their expected outputs (golden suite green).
