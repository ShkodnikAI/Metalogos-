# ADR-0170: Persistence taint layer 2 — locally-bound key prefixes; points-to deferred

**Status:** Accepted
**Date:** 2026-09-20
**Naryad:** #405 (issue #528; wave: dispatch #406, gh#529)
**Pillar:** language / static taint analysis; extends the №386 cross-module persistence-taint MVP (ADR-0151 lattice; №376 interprocedural summaries) along the audit P1-4 axis
**Consumers:** agent-office dogfood (№395 — the source of the real key shapes), memory_store/memorize-recall flows, the leak corpus (TAINT_PERSISTENCE vocabulary)

## 1. Context

№386 (ADR MVP, `check_taint_persistence_cross_module`) closed the
cross-module gap for LITERAL / inline-prefix keys: module A writing
`memorize("draft386:", call_llm(x))` registers the prefix `"draft386:"`
in the fingerprint-keyed registry; module B's
`recall("draft386:…") → respond()` matches it and fails the audit with
the Category-A `TAINT_PERSISTENCE`.

The MVP resolves the key's prefix from the argument expression AT the
call site only (`memory_key_prefix`: a string literal, or the leading
literal of an inline concatenation). The office code — the №395 dogfood
is the source of real shapes — builds its keys in locals first:

```text
app.mlog:579          let key = "rate_limit:" + provider
app.mlog:707          let key = "model_cost:" + model + ":" + direction
dept/legal.mlog:325   let key = "legal_jx_" + jurisdiction
```

(those exact sites feed the office's kv store access — the office memory
surface is `kv_get`/`kv_set`; the office writes NO `memorize`/`recall`
calls today — but the KEY-CONSTRUCTION SHAPE is the norm the dogfood
surfaces, and the audit's motivating memory-persistence form is exactly
its memorize/recall twin: `let key = "user:" + user_id + ":summary";
memorize(key, call_llm("summarize", text))`). A `memorize(key, …)` /
`recall(key)` call whose argument is an `Ident` resolves to nothing
under the MVP: writer and reader in different modules do not match, the
taint does not propagate, and the honest boundary text had to say
"dynamically constructed keys are not covered".

## 2. Decision Drivers

1. **The office pattern is the common case.** Every real dynamic key the
   dogfood surfaces is a `let`-bound concatenation with a leading string
   literal. The layer must see it; anything less leaves the motivating
   audit finding open.
2. **Fail-closed posture.** A taint detector may over-approximate (flag
   a flow that might not exist — noisy) but must not silently lose one
   it has already seen (a hole). Every design fork below resolves to the
   over-approximation.
3. **The №386 pin tests freeze the literal behavior.** The MVP's
   semantics for inline literal/prefix keys must not move (§6).
4. **No new error codes (№385 frozen set).** The finding stays
   `TAINT_PERSISTENCE`; the message names the matched prefix, as before.
5. **Points-to is a Phase-7 problem.** The audit's P1-4 names three
   variants (prefix taint / strict default / points-to). This ADR picks
   the first, keeps the second opt-in, and explicitly defers the third.

## 3. Decision

**(1) Locally-bound key prefixes are resolved.** The persistence-taint
walkers (writer side: `collect_tainted_memory_writes_stmts`/`_expr`
feeding `PatternSummary::tainted_memory_keys` and the direct
tool/route/hook walk; reader side: `recall_taint_walk_stmts` and the
expression reachability walk) thread a per-scope binding map
`var → prefix`. A `let`/`assign` whose value resolves to a prefix
(string literal, leading literal of a concatenation, or an `Ident`
already in the map) records the binding; a later `memorize(key, …)` /
`recall(key)` whose key argument is that `Ident` resolves through the
map. Chained bindings compose
(`let k1 = "user:" + id; let k2 = k1 + ":summary"` → prefix `"user:"`).

**(2) Evaluation order is honored.** The RHS of a binding is scanned in
the PRE-statement environment; the binding is recorded after
(`let y = recall(key)` uses the old `key`, then binds `y` — the runtime
semantics).

**(3) Fail-closed forks.**
- An unresolvable re-assignment (`key = dynamic_fn()`) KEEPS the
  previous prefix — the detector never forgets a seen flow.
- Branch bodies share the enclosing scope's map (monotone, the same
  convention the recall walker's taint vars use): a binding made inside
  an `if`/`match`/loop body stays visible after it.
- Each SCOPE gets a fresh map (patterns, tool methods, server routes,
  hooks): bindings do not leak across scopes — crossing scopes with a
  shared map would manufacture flows that do not exist.

**(4) Strict mode stays opt-in.** `METALOGOS_TAINT_STRICT=1` (since
№386) flags any recall-to-sink flow when any other module writes LLM
output to memory at all. The DEFAULT for `mlog serve` is an owner
decision OUTSIDE this naryad — the flag ships unchanged, opt-in.

**(5) Points-to is explicitly deferred.** Field-sensitivity, heap
shapes, and full fixpoint interprocedural key resolution stay Phase 7
(the v1 plan's D8 family). The honest boundary moves from "dynamic keys
are not covered" to the precise statement: keys WITH a leading literal
(inline or through local bindings) are covered; keys with NO leading
literal anywhere are not.

## 4. Consequences

- The office shapes above now produce cross-module
  `TAINT_PERSISTENCE` findings end-to-end: `let key = "rate_limit:" +
  provider; memorize(key, call_llm(...))` in module A and
  `let k = "rate_limit:" + p; respond(recall(k))` in module B match on
  the prefix `"rate_limit:"`.
- The audit remains a heuristic layer: same-process, same-call-shape
  flows only; no new finding kinds; no new codes; the message text names
  the layer (№386 literal/prefix; №405 let-bound prefixes).
- Overhead: the binding map is a per-scope `HashMap<String, String>`
  threaded through walks that already materialize comparable state (the
  taint-var set); measured raw numbers on a production-like fixture are
  published in the naryad issue (gh#528). Measured (release build,
  `mlog audit` on the office `app.mlog` — 5 038 lines, 64 commands,
  14 departments, 7 runs each, sandbox container): main `8b3ed0e`
  median **0.230 s** (min 0.228), this layer median **0.222 s**
  (min 0.220) — the difference is inside the run-to-run noise band
  (±3 %); no measurable overhead on this fixture.

## 5. Alternatives considered

- **Strict mode by default for serve** — rejected for this naryad: it
  flags unrelated modules (any write + any recall), the ergonomic cost
  on a 64-command office surface is unknown until the №395 report
  lands, and the dispatch scopes the default flip to an owner decision.
- **Full points-to / fixpoint now** — rejected: the office corpus shows
  no key shape that requires it; the analysis cost (and the
  false-positive surface) is unbounded without a real workload study;
  Phase 7 owns it.
- **Registering dynamic keys under a `"<dynamic>"` bucket** — rejected:
  it degenerates to strict mode (every recall matches) while pretending
  to be key-precise; the honest version of that idea IS strict mode.

## 6. Verification

- `tests/naryad_405_taint_prefix_bindings.rs` — the red/green matrix for
  the `user:`+id pattern across modules, chained bindings, the
  fail-closed re-assignment, the per-scope isolation, and the №386
  literal pin replay.
- Leak corpus: `examples/leak/n405_a_writer_binding.mlog` (+`.error`) /
  `n405_b_reader_binding.mlog` (+`.error`) — the write/read halves of
  the let-bound pair; `examples/leak/ok_405_binding_sanitized.mlog` —
  the sanitized green path.
- №386 pin tests (`tests/naryad_386_taint_cross_module.rs`) stay green —
  the literal semantics did not move.
