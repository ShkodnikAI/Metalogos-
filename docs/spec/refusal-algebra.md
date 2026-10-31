# Topic 7 — The Portable Refusal Algebra

**Status: normative (the №645 protocol).** The topic records the
REFUSAL surface as the language exposes it — the same surface the
Portable Refusal Algebra (ADR-0204, №853) defines over and the
`refusal-wasm` facade (№855) and the `policy-check` Action (№856)
consume. Every norm below was probed on BOTH backends before it was
written; the conformance pairs prove the record on every CI run. The
topic-7 probe found NO cross-backend divergence: the deny-event
construction is the ONE shared choke point (`deny.rs::make_event`,
algebra-backed since №854), so TW and VM render the same refusals by
construction.

The stable refusal codes are the FROZEN contract (№385/ADR-0169): this
topic REFERENCES the codes, never redefines or renames them — the same
rule as topic 3. The algebra aggregates the existing codes; it never
extends them (ADR-0204 §6 — no new codes).

### S-RA-001 — a handled runtime refusal degrades; the refused action never executes

A runtime refusal covered by an `on_deny` handler does NOT abort the
program: the refused call degrades to `Unit`, the handler runs with the
live `DenyEvent` (its verdict is FINAL — the handler can log, notify or
degrade, it can never re-allow the refused action), and the statements
after the refusal run normally. The reason word the handler reads via
`deny_reason()` is one of the FROZEN 14-word vocabulary (`DENY_REASONS`,
№392) — the analyzer enforces an exhaustive match (every vocabulary
word covered, or an explicit `else` arm).
**Anchors:** TW the `fire_on_deny` hook
(src/interpreter/hooks.rs — the ledger record BEFORE the handler
selection, ADR-0167 §3.4); VM the `vm_fire_on_deny` twin
(src/vm.rs); the shared construction choke point `deny.rs::make_event`
— algebra-backed (№854, `src/refusal_algebra.rs`).
**Conformance:** `tests/conformance/sra_001_handled_refusal_degrades_unit.mlog`.

### S-RA-002 — the refusal codes are the frozen stable codes, stamped at position 0

The static refusal gate stamps the FIRST blocking finding's stable code
at position 0 of the refusal (`[<CODE>] ` — the №479 form); the machine
consumers read the class, not the prose. The code set is the frozen
№385/ADR-0169 contract — a refusal NEVER invents a new code (the
algebra's closed-set rule, ADR-0204 §6).
**Anchors:** TW `enforce_run_gates`
(src/lib.rs — the `[CODE] ` stamp before the blocking lines); VM the
compile-path refusal (the same stamp form in the `run_vm` production
path); the check-id SSOT `audit.rs` (the `GRANT_REUSED` check, №390).
**Conformance:** `tests/conformance/sra_002_static_refusal_frozen_code.mlog`.

## The portable surface — the consumption contract (ADR-0204 §2–§5)

The lattice and the verdict are DEFINED in
[ADR-0204](../adr/0204-portable-refusal-algebra.md) — this page records
the consumption seams an external consumer uses:

- **The lattice.** The reasons form a chain over the canonical №392
  array order (lower index = more severe); `join` = the more severe
  cause, `meet` = the less severe; `Allow` is the bottom element of the
  verdict lattice. The laws hold over the FULL 14×14 table — pinned by
  `tests/naryad_854_refusal_algebra.rs` (the №854 machine proof).
- **The WASM contour (№855).** The dependency-free kernel
  (`src/refusal_algebra.rs`, compiled verbatim into the facade via the
  `#[path]` seam) answers a JSON request
  (`{"op":"join"|"meet"|"verdict","reasons":[...]}`) with
  `{"verdict":"allow"}` / `{"verdict":"deny","code":...,"cause":...}` /
  `{"verdict":"error","detail":...}` behind the C ABI
  (`rw_pool_len`/`rw_pool_base`/`rw_alloc`/`rw_eval`/`rw_reset`).
  Machine proof: the `wasm-facade (blocking)` CI job — the build for
  `wasm32-unknown-unknown` + the host smoke consumer
  (`refusal-wasm/smoke.mjs`, node, no Rust in the loop).
- **The reusable Action (№856).** `policy-check` (the root
  `action.yml`) evaluates a refusal policy (a JSON of lattice
  requests) in ANY repository's CI; the outputs are `verdict` (deny if
  ANY request denies) and `code` (the join — the MOST severe cause —
  across the denying requests, computed by the algebra itself). The
  honest limit: the Action checks policy THROUGH the algebra — it does
  not compile or run the METALOGOS language. Machine proof: the
  `policy-check-dogfood (blocking)` CI job — this repository consumes
  its own Action on every push.
- **The machine fact (№857).** `SEMA_EXTERNAL` in `gen_metrics --check`
  counts the live external-consumption artifacts (the facade + the
  Action + this page — each ONLY with its real existence and its live
  blocking-CI wiring) against the gate_035 only-up floor ≥ 1 (the plan
  §23 number). The tamper traps catch placeholders, deletions and
  definition substitutions.

## Honest limits

- **The algebra is not IO.** It computes verdicts over the causes it is
  GIVEN — it does not observe the world, does not touch the network,
  the filesystem or the clock (ADR-0204 §3/§6).
- **The Ledger stays outside.** The kernel computes verdicts; the
  Ledger records what the HOST did with them (ADR-0167 §3.4 — the
  runtime deny paths unchanged by the №854 migration; the ledger-golden
  and conformance suites passed with ZERO golden edits).
- **Not a formal verification of the compiler.** The lattice laws are
  tested over the finite 14-word table — a proof of the table's
  consistency, not of the language's safety.
- **No new codes, ever.** The vocabulary is the frozen 14; the codes
  are the frozen №385/ADR-0169 set. Extending either is a
  semantics-change naryad with the owner's gate.
- **The `GRANT_*` detail is not re-derived.** The algebra carries
  `IRREVERSIBLE_NO_GRANT` as the cause; the typed
  `GRANT_EXHAUSTED`/`GRANT_EXPIRED`/`GRANT_REVOKED`/`GRANT_ESCALATION`
  wording remains the producer's `human` payload (№390) — outside the
  portable kernel.
- **Publication is not adoption.** Publishing the facade (npm/crate) or
  the Action (Marketplace) is the owner's act; until then
  `SEMA_EXTERNAL` counts live artifacts on main, not downloads.
