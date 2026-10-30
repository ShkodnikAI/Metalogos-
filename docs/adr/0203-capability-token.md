# ADR-0203: Capability-Token — the Typed Carrier of a Grant and the Single Consumption Contract (the 8B-1 Subphase Record)

- Status: **Accepted** (the owner decision of 2026-10-28 — the plan
  revision R38 fixed 0.36 = 8B-1, the C2 "Capability/Grant Runtime"
  subphase with the GRANT_COVERAGE 10000 bp floor; the wave-72 dispatch
  gh#1666 issued the subphase's naryads the same week; the wave
  acceptance fixes the ADR status as Accepted)
- Date: 2026-10-30
- Deciders: the OWNER (the R38 revision — the train slot 0.36 and the
  10000 bp floor are the plan §23 numbers, accepted in full; the
  wave-72 dispatch gh#1666)
- Naryad: №843 (issue #1660, Wave 72 — W-B; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R47 GET-only
  fact-check over live main `1e3a1622e` (2026-10-30)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.36 is this record's train slot),
  [ADR-0155](0155-grant-algebra.md) (the grant algebra — the parent
  runtime this record types OVER, never instead),
  [ADR-0167](0167-action-ledger-v1.md) (the Action Ledger v1 — the
  event binding), [ADR-0202](0202-model-passport.md) (the sibling
  subphase record — the typed-artifact pattern this record follows),
  [ADR-0196](0196-mcp-policy-gateway.md) (the MCP policy gateway —
  read here, not rewritten), [ADR-0197](0197-vm-single-executor.md)
  (the TW/VM parity posture — the №829 lesson), №839 (the license
  class × profile permit rule — the profile-permit pattern), №392 (the
  DenyEvent surface), №316 (the builtin classification SSOT),
  №844–№848 (the code naryads that land on top of this record)

## §1. Context — the grant runtime lives, but the capability contract does not

ADR-0155 (№390) landed the grant algebra and it works: `src/grants.rs`
(767 lines on `1e3a1622e`) carries `grant_issue` / `grant_subgrant` /
`grant_revoke` / `grant_use` with the once / n / unlimited classes, the
TTL, the attenuation law (`GRANT_ESCALATION` on every widening), the
once-linearity checked at compile time (`GRANT_REUSED`,
`registry.rs:1115–1124`), and `db_execute_with_grant` as the allowing
path for destructive SQL. The ledger writes the grant/deny/irreversible
events in the ACTION paths themselves (ADR-0167 §3.4) — no "also log
it" API exists.

But the contract a consumer of an irreversible action actually holds is
invisible. What the language surfaces is only the opaque handle:
`values.rs:125` (`Variant::Grant(crate::grants::GrantHandle)`,
non-printable per `values.rs:933`) — a capability handle, not a
capability TYPE. The word "capability" has 0 code hits on main. The
ledger record (`GrantRecord`) carries scope, class, quota, expiry,
revocation — and none of it has a typed, language-readable carrier.

The coverage hole is the sharper half. The capability gate today covers
exactly two contours: the SQL class (the №325 destructive-SQL deny with
its `IRREVERSIBLE_NO_GRANT` event, plus the №390 allowing path) and the
consent classes. The rest of the irreversible surface executes with NO
rights decision at all: 43 builtins classified
`Reversibility::Irreversible` in `src/builtins_classification.rs`
(print, exec, exec_argv, delete_file, git_push, respond*, http_post*,
send_*, smtp_*, db_insert, db_execute, kv_delete, mem_delete,
memory_prune, mtree_forget, memory_forget, memory_export,
memory_forget_cascade, forget, human_delete, human_forget,
human_respond, voice_delete, session_clear, cron_remove, cron_run,
ask_approval, cal_create, cal_delete, card_create, card_delete,
tts_send, answer_callback_query, sleep, ledger_rotate,
ledger_snapshot) plus the MCP tool methods compiled with
`irreversible: true` (`metalogos-server/src/mcp_policy.rs:44–45`). Two
of those carriers (the ADR-0155 linear actions `memory_forget_cascade`
and `forget`) already demand a grant by their own contract; the
remaining majority runs on nothing but the call itself. This is the
same class-shape №839 closed for licenses: a ladder of decisions that
the gate never walks. And the phase's machine fact — GRANT_COVERAGE —
has 0 hits anywhere in the tree: nothing counts the coverage, so
nothing can hold a floor.

## §2. Decision — the token is a type, the decision is one layer, the roster is machine-built

1. **The capability token is the typed carrier of a grant and the
   single consumption contract for an irreversible action.** Mint stays
   `grant_issue` (the ADR-0155 §3.1 machinery unchanged — the migration
   is 1:1, the №838 pattern: existing programs keep working). The token
   carries the ledger-backed fields as a read-only typed view —
   `scope`, `class` (once / n / unlimited), `ttl` (the expiry instant),
   `quota` (the remaining uses), `revoked` — and the grant ledger stays
   the SSOT behind every field: the token is an untrusted cache of the
   ledger record (the handle posture of `values.rs:125` kept verbatim),
   so a stale token cannot revive power (the serialization rule of the
   ADR-0155 linearity rules holds).
2. **One consumption decision, fail-closed.** Every consumption of an
   irreversible carrier passes the capability layer before the action:
   a valid token consumes (the single consume point — `grant_use`
   semantics), and a revoked, expired, or exhausted token refuses the
   consumption LOUDLY — never a silent degradation. The once-linearity
   stays where it is today (compile-time `GRANT_REUSED`, №390); the
   runtime layer never re-decides what the compiler already refused.
3. **The carrier roster is machine-built, never hand-written.** The
   roster is derived from the two classification sources the executors
   themselves dispatch through: `builtins_classification.rs`
   (`Reversibility::Irreversible` — the №316 SSOT, GENERATED by
   `gen_classification.py`; the №777 freshness gate refuses hand
   edits) and `mcp_policy.rs` (`irreversible: true`). No roster copy
   lives in a hand-maintained list; the №845 gate and the №846 metric
   both read the machine sources.
4. **The requirement profile — two dispositions, no silence (the
   frame; №845 fixes the exact roster and names the choices).** Every
   carrier's call passes the capability decision before run by one of
   two routes:
   - **GrantAlways** — the destructive / destroy-effect classes: the
     ADR-0155 linear actions and the destructive-SQL arm keep their
     grant-or-refuse semantics; a valid grant is required ALWAYS, and
     the profile cannot weaken this disposition.
   - **ProfilePermit** — the delivery / egress / host-effect classes:
     allowed only when the program's profile carries the carrier in an
     explicit permit roster (the №839 pattern: `allow_license_class`
     is the spelling precedent). The baseline resolution — a program
     with no profile declared — is itself a NAMED profile with a named
     roster, machine data, not silence: the decision exists for every
     carrier, and the honest-limits documentation (№845, №848) states
     the roster openly.
   Absence of both routes = the refusal BEFORE the action, with the
   existing DenyEvent class `IRREVERSIBLE_NO_GRANT`. The deny
   vocabulary does not grow (`DENY_REASONS` stays 14): the typed
   `GRANT_*` text rides the event's `human` field verbatim (the №390
   fold-in posture).
5. **The Ledger binding rides the ACTION paths (ADR-0167 §3.4).** The
   consume, revoke, and deny events write where the actions execute —
   this record introduces no parallel logging surface.
6. **TW/VM parity by construction.** The decision function is ONE
   shared surface; both executors call it. The №829 lesson is binding:
   a gate that exists on only one lane is a divergence class of its
   own, and the wave's parity tests (№844, №847) pin it.
7. **The coverage is a machine fact with an owner floor.**
   GRANT_COVERAGE = the share of carriers whose rights decision passes
   the capability layer, counted by the machine from the
   classification/policy sources (№846 — `grant_coverage_bp()` beside
   the №841 counters, the gate_034 file with `goal_grant_coverage_bp
   10000`). The 10000 bp floor is the plan §23 number (the owner
   revision R38) — after №845 the gate sits on the single dispatch
   choke point every carrier passes, so the coverage is 100% by
   construction, and any future carrier added without its capability
   disposition reddens the CI loudly (that is the gate working).

## §3. What this record does NOT decide

- The gate-site spelling, the profile option syntax, the exact permit
  roster, the refusal texts: №845 fixes them and names the choices in
  its report.
- The metric mechanics, the gate_034 file, and the negative controls
  of the floor: №846.
- The e2e battery (issue → use → revoke → deny) and the Ledger golden:
  №847.
- The public phase digest (PLAN-SUMMARY 0.36), the honest-limits prose,
  and the CHANGELOG notes: №848, strictly last.
- The MCP policy gateway internals: read (the `irreversible` flag is
  roster input), not rewritten — ADR-0196 stays the contour owner; the
  schema-pinning (the G5, №828) is closed and untouched. The MCP audit
  side is outside this wave.
- The license contour boundary: capability is the right of ACTION
  (C2), the license is the right of WEIGHTS (C4, ModelPassport
  №837/№838). The overlap is the fail-closed style only (the №842/G5
  pattern); neither line rewrites the other, and no ModelPassport
  surface changes here.
- The token's on-disk serialization: out of the 8B-1 slice (the
  ledger's canonical form is the ADR-0155 record; the token is the
  typed view over it).
- The market sections of the plan (§16.0-7): not published — the
  ADR-0201 §5 sanitization boundary applies to this record and every
  wave-72 PR.
