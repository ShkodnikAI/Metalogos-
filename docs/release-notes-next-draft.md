### Migration

Changes in [Unreleased] that can break existing programs, each with the
repair:

- **№800 (gh#1547, V3):** `run` / `eval` / the REPL execute on the VM —
  the single user-command executor; the TW stays the reference oracle
  behind the transitional `--reference` flag. A program that diverged
  between the backends now surfaces the VM's verdict; pin the expected
  outputs (the examples carry `.expected` pairs) and report a divergence
  as an issue, do not rely on the TW parity by default.
- **№799 (gh#1546, C3-a T2):** a parked builtin now refuses at `check`
  with `[BUILTIN_PARKED] <name> requires --features <f>` instead of
  failing at runtime — enable the feature in the build (or the run
  profile) before shipping a program that names a parked builtin.
- **№781 (gh#1492, A2):** `escape_html` no longer declassifies —
  escaping a Secret now produces the masked form instead of the secret
  bytes. Templates that escaped a secret to embed it must switch to the
  explicit declassification points (the redaction pipeline or the
  sanctioned fingerprint), not the HTML escaper.
- **№787 (gh#1498, I-c):** the integer precision loss beyond 2⁵³ at the
  SQLite/JSON borders is a loud refusal — retype the affected fields to
  the text lane (or the hashed comparison) if a program relied on the
  silent truncation.

### Added

- **№763 (gh#1416, Wave 58 — the agent-index honesty, the R-01
  finding):** the llms.txt drift (uncovered by any generator gate since
  its creation) is closed by a BLOCKING gate. (1) llms.txt refreshed
  FROM the generators — the builtins count 409 → 526 (the registry
  scan), the examples 214 → 253, the ADRs 132 → 187 (the gen_metrics
  machines), the naryad range №1-№289+ → №1-№763 (the merged-history
  №505-grammar scan — the naryad sequence machine), and the three
  missing canonical rows added (docs/spec/tools.md, docs/limitations.md,
  docs/PLAN-SUMMARY.md). (2) CLAUDE.md — the declared "bridge copy of
  AGENTS.md" — restored byte-identical to AGENTS.md (was 8958 vs 10703
  bytes; the №676 A5 content-source form: AGENTS.md is the source, the
  copy is one). (3) NEW `scripts/ci/agent_index_gate.py` (the
  gen_metrics --check form): the llms.txt counters == the generator
  machines, the bridge byte-identity, every llms.txt link target
  exists; `--self-test` carries the tamper traps (the stale counter,
  the broken bridge, the dangling link — all spring). Wired as the
  `agent-index (blocking)` CI job (fetch-depth: 0 — the git-log
  machine) + the blocking_checks.tsv cell (33 cells) pinned by
  `tests/naryad_763_agent_index_gate.rs`. Stale agent indexes are
  false facts on the first screen of IDE agents and RAG pipelines —
  the same honesty family as the stale-draft rule (№713); the drift
  now refuses the merge.

- **№788 (gh#1511, Wave 63 — audit-23.10 C4-a T3, the owner decision
  23.10):** the honest-boundary CHANGELOG verdict — `security: no —
  <reason>` on the row itself is the THIRD legal path (next to the
  `### Security` section and the `release-block` evidence); the gate
  accepts it and prints it for the review. The rephrase-circumvention
  detector rides the same blocking job: a dictionary-matching line
  removed from CHANGELOG/limitations whose replacement matches neither
  the dictionary nor the verdict exits 1 («rephrase without verdict»)
  — the №762.1/№771 synonym lane is closed, the dictionary unweakened,
  the hunk-scoped pairing keeps multi-line entries together. Self-test:
  six new cases (the verdict accepted, the bare `security: no.`
  rejection, the №771 shape caught, the verdict-carrying rewrite
  lawful, the dict-vocabulary lane, the limitations shape). The
  AGENTS.md rule: a naryad whose only content is a rephrase under the
  gate is not issued.

- **№789 (gh#1512, Wave 63 — audit-23.10 C4-a T1, the owner decision
  23.10):** `scripts/gen_wave_record.py` — the wave-record generator:
  by the `wave-N` label it finds the dispatch, parses the naryad
  composition table, and assembles the REALITY §-record (per naryad:
  the issue, the source issue, the merge commit from the git-history
  scan, the PR, the CI verdict ON the merge commit) + the PLAN-SUMMARY
  line. The machine counters ride VERBATIM from the scripts/ci
  machines — nothing paraphrased; a broken machine prints
  `MACHINE UNAVAILABLE` into the record (never silently dropped, never
  invented). The prose is ONLY the `HONEST-FINDING:` marked lines from
  the naryad audit comments (no markers → an empty section — the
  generator writes no prose, the C4 form). The weekly flow (one PR for
  all the waves of the week, manual until the owner decides on the
  schedule) replaces the per-wave «strictly last» docs naryad. The
  verification: the wave-62 and wave-61 records generated — the
  structured fields reproduce the git/API facts 1:1 (the comparison
  with the manual §6.46 rides the naryad report); the wave-61 sweep
  surfaced №778 as closed-without-a-merge-commit (the honest
  no-surface rendering).

- **№790 (gh#1513, Wave 63 — audit-23.10 C1 (b), the owner decision
  23.10):** ADR-0196 **Accepted** — the MCP policy gateway is the
  FLAGSHIP PILOT: the language stays a product until the fork; the
  pilot scope G0 (#1458 landed as №780, #1460) → G1–G4 (#1480–#1483,
  waves 64–66); the MEASURABLE exit criteria (the published G4 demo
  with the reproducible script; the Fosved Office dogfood; ≥ 3
  external installation attempts) — conjunctive; the fork by the
  results is a NEW owner act (the expansion (a) or the course
  unchanged (c)); the boundaries: no builtin expansion beyond the
  gateway's need, the domain quota and the stop-list in force, the
  language roadmap untouched. The full owner verbatim lives in #1470;
  the ADR quotes the posture line (the docs-language budget — the
  lint is the honest machine). The ADR index 187 → 188.

- **№791 (gh#1514, Wave 63 — audit-23.10 C5-b V1, the owner decision
  23.10):** ADR-0197 **Accepted** — the VM is the ONLY executor of the
  user commands (`run`, `serve`, `mcp-serve`, `eval`, REPL); TW is the
  reference oracle in CI (the crosscheck, the diff-fuzzer, the
  doc-tests) and behind the transitional `--reference` flag (two
  release windows by default, an extension is a new owner act). The
  same-PR lineage edits: ADR-0105 — the remainder SUPERSEDED (the
  TW-default posture; ADR-0141 had already superseded the readiness
  reservations), ADR-0171 — EXTENDED from `serve` to ALL the
  user-command lanes. The oracle's contract stays: the core-semantics
  criterion, the shared-RuntimeContext dedup, the CI oracle surfaces.
  The landing order (the owner's verbatim): the precondition #1458
  landed (№780), V2 rides with the gateway pilot, then V3, then the
  V4 gate closes the lane. The ADR index 188 → 189.

- **№792 (gh#1515, Wave 63 — audit-23.10 C6 R-b 1, the owner decision
  23.10):** ADR-0198 **Accepted** (supersedes ADR-0106) — the error
  model is the LOUD failure with a code + the explicit `_or`
  quietness + `try` for the handling; in the type layer `Result<T>`
  (the try result) and `Option<T>` (= `T|Unit`) are the ALIASES of the
  existing shapes — **no new `Value` variants, the grammar of values
  and the 526 signatures unchanged, no full sum types** (`Ok/Err/
  Some/None`, `?` are not introduced). The same-PR lineage edits:
  ADR-0106 → Superseded (the soft-failure premise expired; the
  "not without demonstrated need" principle survives), AGENTS.md §3/§6
  re-pointed (the CLAUDE.md bridge byte-identical). The I-c Int piece
  (the loud `[PRECISION_LOSS]`) already landed as №787; the static
  use-check rides #1505. The ADR index 189 → 190.

- **№793 (gh#1531, Wave 64 — the gateway G1, the C1 (b) owner
  decision, ADR-0196):** the MCP contour gains the SESSION layer —
  long-lived upstream sessions for the gateway. The new
  `src/mcp_session.rs` module carries the connection primitives
  (moved verbatim in behavior from the builtins module — the
  stateless `mcp_call`/`mcp_list_tools` contract is unchanged, the
  back-compat form the naryad requires) plus `McpSession`: ONE live
  process per declared upstream for the gateway's lifetime, the
  loud reconnect on an upstream death (the deterministic stream-EOF
  death signal + the reaped-exit check; a failed call is never
  auto-retried — the side-effect decision stays with the caller).
  The gateway reads `METALOGOS_MCP_UPSTREAMS` once at startup (the
  №263 discipline; entries `name=argv0,arg1,...` separated by `;`),
  aggregates the upstream `tools/list` under the `name.` prefix and
  proxies `tools/call name.tool` to the session — the response
  shape, the size cap and the error envelope are byte-identical to
  the local path; the LOCAL tools win a name collision (pinned
  loudly at startup); the allowlist gate rides every declaration
  fail-closed. The protocol: the client declares the CURRENT
  revision `2025-11-25` (the legacy `2025-03-26` declaration is
  superseded), the server negotiates the client's known revision
  (the hardcoded `2024-11-05` answer is gone), a mismatch is a loud
  refusal naming both versions. The 6-pin gateway battery +
  the 4-pin session battery verified RED-before on the pre-№793
  revision: 4/6 gateway pins failed (no upstream aggregation, the
  hardcoded version, no declaration gate), 2 regression pins green.
  AFFECTS: gateway deployments can now serve stateful upstreams
  (a browser, a db session); the stateless builtins unchanged.

- **№796 (gh#1534, Wave 64 — audit-23.10 C3-a T1, the C3 (a) owner
  decision, ADR-0196-adjacent):** the maturity tier SSOT — every
  `spec!` registration carries an EXPLICIT `tier`
  (`"stable" | "experimental" | "parked"`, the owner's rule table
  23.10). The gate IS the grammar: the spec! macro requires the
  trailing tier literal on every arm — a registration without one is
  a compile error (the №796 gate form, stronger than a CI script).
  The classification (the owner's rule): `stable` = the working,
  test-covered core (the labels, the grants, the ledger, MCP, the
  memory, the media handles, the working perception backends, the
  provider voice); `experimental` = the embodied family
  (device_*/pose_make/proof_*), the forecast family (series_*/forecast_*),
  the not-implemented stub seams (provisional — the dispute list);
  `parked` = the ADR-0178 generative contour (reflex-generate,
  vision-generate/edit/lora, voice_design/tts_speak/voice_enroll,
  video-render/extend/interp + the weights-fetch infrastructure).
  The compiled default build: 476 stable / 22 experimental / 11
  parked (509 = 526 source rows minus the 69 feature-gated); the
  source-level 526/526 ride gen_reference --check. REFERENCE's
  generated index gains the Tier column (the generator re-validates
  the vocabulary loudly — a tier outside the three-word set is
  exit 2). The BEHAVIOR IS UNTOUCHED (the naryad boundary: the
  marking only). The 3-pin battery (the vocabulary coverage, the
  owner-rule spot checks, the census) — new-surface pins. The
  disputable rows ride the naryad report for the owner's review.

- **№802 (gh#1549, Wave 65 — the C6 R-b 2 static check, the ADR-0198
  queue):** the try result and the `T|Unit` union are verified BEFORE use —
  the flow-sensitive semantic pass refuses `[UNCHECKED_RESULT]` (a
  `.value` read on a try-bound variable outside a proven `.ok` guard: the
  `if r.ok { … }` then-branch, the early-exit `r.ok == false { return … }`,
  the short-circuit `r.ok and …`, the `match r.ok { "true" then { … } }`
  arm) and `[UNCHECKED_OPTION]` (a union-with-Unit value — `query_scalar`'s
  `Float|String|Unit`, a declared `-> T|Unit` — used where `T` is required:
  the arithmetic/ordering operands, the `each` iterable, the string-key
  index, a declared precise parameter; `to_string` is the sanctioned total
  projection). Conservative posture: an unprovable guard refuses; the
  sanctioned escape is the explicit suppression comment with a non-empty
  reason (`// allow(unchecked_result): <reason>` — a bare allow is NOT a
  suppression; one comment sanctions one access line, several reads on the
  same line are a unit). The refusal fires on `mlog check` and blocks the
  №523 run gate and the serve startup on BOTH backends (the shared
  semantic pass; the mcp-serve startup keeps the unfiltered refusal — the
  fail-closed asymmetry is recorded). The registry SSOT carries the raw
  return-type spelling (`return_type_raw`) so the union arms are readable
  without a shadow table. Examples migrated: p30 projects the union cell
  through `to_string` (the honest fix); w385_office_branch and the №670
  conformance specimens carry the suppression comments with reasons (the
  redact-obfuscated guards and the verbatim shape pins are deliberate).
  RED-before 4/12 (the №772 form). security: no — the check widens the
  refusal surface only (fail-closed), no gate, allowlist or taint floor
  moves; the honest-boundary dictionary is untouched. AFFECTS: programs
  reading `r.value` without an `.ok` guard or consuming `T|Unit` values
  raw now refuse at check/run time — guard them, project through
  `to_string`, or suppress with a reason.

- **№805 (gh#1561, Wave 66 — the gateway G4, the pilot's exit criterion):**
  the demo + e2e — `examples/gateway/policy.mlog` (the 30-line gateway
  policy over two upstream MCP servers: file + fetch) and
  `examples/gateway/README.md` (the 5-minute guide for Claude Desktop /
  any MCP client) ship as CI-verified artifacts: the e2e
  (`naryad_805_gateway_demo`, 4 scenarios) reads the SHIPPED policy file,
  pools BOTH upstreams in ONE gateway process (the №793 pool's first
  two-upstream exercise), and runs the canonical event — the agent reads
  `.env` (Source private joins the session label), the POST through the
  fetch upstream is refused with `[SINK_CLEARANCE]` BEFORE the upstream
  sees the call, the refusal is a signed `mcp.call.denied` ledger record
  and the export verifies externally (`ledger_verify` ok); plus the
  allowed path (a fresh session posts), the once-grant unblock (№801) and
  the audit tap (`ledger_dump.dump` → the export round-trips through MCP
  and verifies). The fixture upstreams (`tests/fixtures/mcp_file_server.py`,
  `tests/fixtures/mcp_fetch_server.py`) speak the newline-delimited
  JSON-RPC the fixtures already use. `examples/gateway/screencast.sh`
  drives the REAL gateway with live JSON-RPC for the 60–90 s take (the
  video cut itself — verified-wait за вербатимом владельца, форма №755).
  RED-before: the naryad lands no product change (additive examples +
  tests + fixtures only) — the pins are the new two-upstream coverage and
  ran on the pre-№805 revision; the result is recorded in the PR body.
  security: no — the demo adds no executable product surface; the demo
  policy is conservative (Source private on the file upstream), the
  ledger/allowlist/label contracts are untouched. AFFECTS: none (the
  gateway surface is unchanged; the example is the new entry point).

### Changed

- **№767 (gh#1420, Wave 59 — the audit 21.10 M-1, the exemption's paper
  trail):** the read-back pair exemption (№729 — `cron_list` →
  `cron_remove`; `mtree_retrieve` → `mtree_forget`, the table
  `semantic::READBACK_PAIRS`) leaves the documentation trace the audit
  methodology demands: a limitations row (the Static Analysis section —
  the live anchor, the conservative boundary, the revisit condition) +
  the risk-review verdict recorded in ADR-0154 §11 (accepted,
  position-only, domain-matched, quarantine-proof; a third pair or a
  recognition-shape change re-opens the review). The implementation is
  NOT changed — the docs-only honesty pass; every future Category-A
  exemption follows the same protocol (the row + the verdict in the
  same PR).

- **№777 (gh#1452, Wave 61 — the №772 honest finding closed, the №744
  class):** the classification-SSOT generator is now drift-proof and
  gate-covered. A bare `gen_classification.py` run on a clean HEAD
  produced a non-default diff (builtins_classification.rs +8/−26 — the
  template's static fragments were not in the fmt-gate style;
  REFERENCE.md +1 — the block carried a trailing newline the tail after
  the END marker already provides, one empty line per run). (1) The
  template fragments now carry the fmt-gate style and the emitted Rust
  is normalized through rustfmt when it is on PATH — the generator
  cannot drift from the fmt gate by a formatter-default change. (2) The
  REFERENCE block no longer grows an empty line per run (the replace
  path emits END without the doubled newline). (3) The argv is loudly
  validated via argparse: unknown flags are a loud exit 2 — the
  `--goal032 --list` spelling from the №772 report was never a mode
  (the flags were dropped and the run REGENERATED the tree). (4) NEW
  `--check` mode (the gen_metrics --check form): both artifacts are
  rebuilt in memory and compared against the tree — exit 1 on drift,
  nothing written; wired BLOCKING into the fmt job right after
  `cargo fmt --all --check` (the fact key WITH its checker, №735).
  Verification: a one-token classification tamper springs `--check`
  (exit 1); `--goal032 --list` exits 2 with a named refusal; the bare
  run on a clean HEAD is diff-empty (idempotent). AFFECTS: the
  regeneration workflow and the CI surface only — no product behavior
  change; the classification data (526 entries, 289 non-Pure) is
  byte-identical; the floors stand (sensitive 7586 bp, goal032 35/67).

- **№794 (gh#1532, Wave 64 — the gateway G3, the C1 (b) owner
  decision, ADR-0196):** the ledger entry on EVERY gateway
  `tools/call` — `mcp.call` (allowed: the local ok, the local body
  error, the upstream proxy ok/proxy error) and `mcp.call.denied`
  (the gateway refusals: the unknown tool, the not-in-allowlist
  miss), the tool in the record scope, the reason/outcome in the
  detail. The detail rides the ledger schema's `args_hash` BY
  CONSTRUCTION — the arguments and the reasons never appear in the
  journal in the open form (the ledger must not become a leak
  channel; the pin replays a `LEAKME` argument and reads the stored
  record back). The actor is the gateway process; the floating
  session label extends the record with G2 (#1481) — the
  `label=pre-G2` marker is the explicit extension point, not
  silence. The 2-pin battery verified RED-before via the
  hooks-stash run (форма №772): 2/2 failed on the pre-№794
  hooks (no mcp.* records); the 5-call session (3 allowed / 2
  denied) leaves 5 signed records, `mlog ledger verify --expect-head`
  is GREEN on the export and RED after a one-letter tamper.
  AFFECTS: gateway deployments gain the audit trail the G4 demo
  needs; the stateless builtins' audit-log contract unchanged
  (№268).

- **№799 (gh#1546, Wave 65 — the C3 (a) T2 owner decision, the maturity
  tiers):** the parked builtins refuse at CHECK time — the tier-`parked`
  rows (the №796 census) compile ONLY under the owning feature
  (`vision`/`voice`/`video`/`candle`; 11 rows ungated before — the vision
  family, `reflex_generate`, the №526 erasure rows), and a build without
  the feature answers `mlog check` with
  `[BUILTIN_PARKED] <name> requires --features <f> (parked: <basis>)`
  instead of the old split (the ungated rows passed check and refused
  only at the call; the gated rows read the lie UNDEFINED_FUNCTION).
  The always-compiled parked table (`src/builtins/parked.rs`) is the
  name/basis SSOT — the parity lock pins table ↔ rows set-equality over
  the registry SOURCE (the №467 two-locks discipline; the gated rows
  ride the source metric only, the REFERENCE 526-row surface and the
  typed floors unchanged). The run/serve gates block on the same
  finding (№523, the position-0 stamp), the VM compile path stamps the
  coded `[BUILTIN_PARKED]` origin (№479 shape), the TW eval/REPL lane
  reads it branchably through `try r.error.code` (№413 —
  `split_origin_stamp`); the environment-parked exception
  (`reflex_bpe_load`, no feature to gate with — the never-Ok handler,
  №717/№195) stays registered. The tier-`experimental` calls now WARN
  on check (`[BUILTIN_EXPERIMENTAL]`, exit-0 — never an error; the
  №523 blocking gates never see it). The 8-pin battery (форма №772)
  ran against the pre-№799 main: 6 behavioral REDs (the ungated parked
  calls PASSED check, the gated name read UNDEFINED_FUNCTION, no
  warning, the run/compile lanes didn't name the contract) + 2 guards
  (stable calls clean, truly-unknown stays UNDEFINED_FUNCTION); the
  parity lock caught the paren-in-comment scan hazard and
  the environment-parked exception by construction. AFFECTS: programs
  calling parked builtins stop passing check in the default build
  (the fix: the feature build, or the documented alternative); the
  №483 media-handle pin now reads registry ∪ parked (the names stay
  in the language); the №526 erasure pair rides the `voice` build with
  its rows — a build without `voice` collects no prints, so the erasure
  right holds by construction (the voice CI job carries the pair's
  tests; the dead-code debt floor is untouched). The office e2e proxies
  (probe021cron/probe021qp/probe318qp) check clean; the leak-suite
  classes unchanged (cat_a precedes the parked consult).

- **№800 (gh#1547, Wave 65 — the C5 (b) V3 owner decision, ADR-0197):**
  `run`/`eval`/REPL execute on the VM — the single user-command executor
  (the same executor `serve` and `mcp-serve` already use; TW becomes the
  REFERENCE ORACLE: it stays in CI — the crosscheck, the diff-fuzzer, the
  doc-tests — and behind the transitional `--reference` flag, which
  announces itself with the loud banner and answers `[backend:
  reference-tw]` under `METALOGOS_BACKEND_ECHO=1`; two release windows
  per ADR-0197 §4.3). The VM run lane carries the SAME gate set as the
  TW run path — the shared `enforce_run_gates` body (Category A + the
  №523 semantic gate over the statically-import-merged declarations):
  the №535 pins caught the VM lane refusing a missing import with a raw
  io error and no №523 stamp — a gate divergence, fixed by the shared
  body, not a wording nit. The REPL session is a persistent VM: the
  session chunk compiler (`Compiler::with_session` + `Vm::run_chunk`)
  compiles each line against the live session tables — globals keep
  their slots, patterns/learnables/rules join them, memory persists
  (the phase72 carry-over pinned on the VM lane; the cross-line pattern
  call, the redefinition shadowing, and the loud `[UNDEFINED_VARIABLE]`
  on unknown names — the silent Unit read in `LoadGlobalByName` is
  retired on BOTH loops, the №780 one-contract-both-loops form).
  `mlog eval` scores through the VM harness (`Vm::run_eval_blocks` — the
  same `call_llm` path a user call takes; the aggregation lives in the
  shared `EvalResult::from_examples`, one body for both backends).
  Programs where TW and VM diverged now behave in `run` as they do in
  `serve`; the examples corpus reproduces its `.expected` outputs
  through the VM lane unchanged (191/191 runnable). AFFECTS: consumers
  running `mlog run` on TW-specific behavior — the `--reference` lane
  keeps it during the transition; a doubly-invalid program (an unknown
  function AND an unknown variable) surfaces the compile-time
  `[UNDEFINED_FUNCTION]` first on the VM lane, the reference lane keeps
  the TW runtime order (the №535 pins state both lanes; the
  single-defect classes — undefined variable, undefined function —
  refuse with the same codes on both). The flip SURFACED one
  pre-existing VM defect: the self-host parser self-parse
  (ParseWhile→ParseBlock→ParseStmt→ParseIfStmt, deep) replaces a
  pattern-call result with an AST string — the №197 bootstrap rides the
  `--reference` oracle lane until the repair (tracked gh#1555; the
  corpus unaffected; the №762 (a1) name-miss soft fallback and the №287
  doc-test broken-example code follow the contract — loud
  `[UNDEFINED_VARIABLE]` on both).

- **№801 (gh#1548, Wave 65 — the C1 (b) G2 owner decision, the gateway
  pilot, ADR-0196):** the MCP gateway carries a FLOATING session label
  (LIO current-label form) — one current label per agent session that
  the upstream tool classes move. The policy is a first-class `.mlog`
  declaration: `mcp_policy { tool "upstream.name" source "private";
  tool "upstream.mail" sink "public"; }` — a Source entry JOINs its
  label into the session on a successful call; a Sink entry demands the
  session label FLOW TO it before the call (`Label::flows_to` — the
  conf ≤ required, the integrity ≥ required, the consent scopes cover;
  Poisoned never flows). A sink refusal is a TYPED error to the agent
  (`[SINK_CLEARANCE]` naming the tool, the requirement and the current
  label) + the №794 ledger record; the ONE downgrade path is the owner
  grant — `grant_issue(scope "mcp:<tool>", class "once")` unblocks the
  call EXACTLY ONCE (the grant table consumes it; the second call
  refuses again). The session label rides every `tools/call` result
  `_meta` (`metalogos.dev/session-label`). `mlog check` validates the
  policy (an unknown Label-annotation word or a bare local-namespace
  tool name refuse with `[MCP_POLICY]`). Conservative posture: an
  upstream tool WITHOUT a policy entry passes UNCLASSIFIED (the №793
  posture — the pilot is opt-in; the startup prints the loaded-entry
  count, the first call of an unpoliced tool prints a one-time loud
  note). AFFECTS: gateway operators — the policy-less deployments
  behave exactly as №793; the classified ones gain the inter-call data
  flow the LIO model demands. The per-connection label split for the
  network transports rides with the http session work (the stdio
  transport IS one session; the pilot documents this).

- **№803 (gh#1550, Wave 65 — the C2 showcase, the gh#1486 registry task):**
  the leak-suite showcase reads in ENGLISH — the `// Threat:/Attack:`
  headers of `examples/leak/*.mlog` and the explanatory parts of the
  `EXPECTED: <CLASS> — <scenario>` lines are translated (66 .mlog + 47
  .error files); the failure CLASS before the em-dash is byte-identical
  (the runner compares ONLY the class — `run_leak_suite` green after the
  translation, the corpus BLOCKING gate intact). The program payload
  strings (the realistic PII/secret fixtures the ok_*.expected outputs
  pin) stay verbatim — they are behavior, not comments. The showcase is
  the first-screen surface for the C2 positioning (№804 rides this). The
  documented acceptance: no Cyrillic in the comments/explanations of
  `examples/leak/` (the payload strings are exempt by the naryad's own
  wording). security: no — the refusal classes, the runner comparison
  and every gate input are unchanged; a text-only translation. AFFECTS:
  none — the suite output, the classes and the .expected pins are
  identical; the English text makes the showcase legible to the
  non-Russian-speaking AI-agent audience (the C2 decision).

- **№804 (gh#1551, Wave 65 — the C2 positioning, the gh#1485 registry
  task, the owner decision 23.10):** the first screen carries ONE
  formulation across the three entry points — **README**, the GitHub
  description (prepared for the owner's verbatim — the repo-settings
  act) and **llms.txt**: *"A language for AI agents where data cannot
  reach actions it is not cleared for: secret leaks, injection and
  unauthorized irreversible actions are rejected before the program
  runs."* The irreversible-actions claim is PINNED — the static
  `[IRREVERSIBLE_NO_GRANT]` refusal with the leak-suite test link
  (n18). «A Taste» is now the n02 showcase: the leaking program, the
  REAL `mlog check` output (shot from the release binary v0.33.2,
  sha256 48b98167…) and the `redact(…, "secrets")` fix that passes;
  Quick Start's first command is `mlog check
  examples/leak/n02_secret_respond.mlog`; the new «What it stops»
  section lists the five refusal classes, each linked to its pinned
  negative. **Metrics moved to [docs/metrics.md](docs/metrics.md)** —
  the full generated table (the one honest place, + the new Leak Suite
  row: 48 negatives / 27 positives); the README front page carries
  three rows max (version, leak suite, the pointer); `gen_metrics.py`
  and `tests/readme_consistency.rs` re-pointed (the size/count rows are
  checked in the new place). Honest status preserved verbatim. security:
  no — the surface text and the metrics placement; every promise on the
  first screen is pinned by a test or a leak-suite negative, nothing
  new promised. AFFECTS: the README-facing docs only; the GitHub
  description/topics flip is the owner's repo-settings act (the
  prepared string sits in gh#1551).

- **№806 (gh#1562, Wave 66 — the audit-23.10 B1, the secret-comparison
  recipe):** the unkeyed `hash_only` redact policy no longer DECLASSIFIES —
  the fingerprint keeps the source confidentiality and restores only
  integrity (the №327 decision lift survives on non-secret data: the
  error-code branching form stays legal). The hole being closed: the old
  contract shipped a 64-bit UNKEYED SHA-256 prefix labelled `public` — a
  low-entropy secret (a PIN, a short code) is brute-forced offline in
  seconds, so the sanctioned recipe was a leak channel dressed as hygiene.
  The changes: (1) `hash_only` now emits the FULL 256-bit hex digest
  (`[HASH:<64hex>] hex`, was `[HASH:<16hex>]`) and its registry target is
  `private`; (2) the NEW `hash_keyed` policy — one-way HMAC-SHA256 under
  the process-scoped `METALOGOS_REDACT_KEY` (`[HMAC:<64hex>] hex`,
  process-scoped comparability, declassifies to public; the missing/empty
  key is a LOUD fail-closed refusal, never a silent unkeyed fallback — a
  one-word env-keyed design was rejected because the check-time and
  run-time environments could diverge and the static label would lie,
  №525); (3) the static label mapping (semantic.rs) reads the amended
  registry — `hash_only` keeps the input conf and restores integrity,
  `hash_keyed` lands at bottom; (4) the secret-comparison recipe in the
  `UNTRUSTED_DECISION` refusal, limitations.md and threat-model.md is now
  `ct_eq(provided, expected)` (constant-time) or the keyed fingerprint on
  BOTH sides; (5) ADR-0154 §10 amended in place (the two-word form keeps
  the static label truthful with zero env dependence). Consumers: the
  dogfood contour migrates to `hash_keyed` (l1_dogfood, l1_redact), the
  leak suite gains the `n806_unkeyed_fingerprint_respond` refusal and the
  `ok_806_secret_keyed_fingerprint_respond` fence (the pre-№806
  `ok_781_secret_hash_only_respond` is retired — the unkeyed digest
  respond is a leak now), the office's `hash_only`-BOTH-sides comparison
  keeps WORKING (deterministic comparability is preserved) but its
  digests stop declassifying — migrate the egressing fingerprints to
  `hash_keyed` with a configured process key. RED-before (форма №772):
  the n806 leak-suite refusal was RED on the pre-№806 revision (the
  program compiled and the digest flowed to respond) — the fix turns it
  into the pinned refusal; the keyed refusal/comparability pins went
  RED (arity/shape) before the implementation landed. security: YES —
  this is the B1 security fix; the refusal surface widens (fail-closed),
  no gate, allowlist or taint floor moves; the honest-boundary dictionary
  is untouched. AFFECTS: programs egressing unkeyed `hash_only`
  fingerprints of secret-derived values now refuse at check/run — use
  `ct_eq` for comparisons or `hash_keyed` with a configured
  `METALOGOS_REDACT_KEY` for public fingerprints; digest outputs are
  longer (64 hex) and `[HMAC:…]`-shaped for the keyed policy.

- **№807 (gh#1563, Wave 66 — the audit-23.10 B2, the Secret zeroize
  discipline):** the Secret material no longer lands in plain unsecured
  String copies on the crypto/masking paths — `hash_password` and
  `verify_password` BORROW the Secret (argon2 reads the Zeroizing buffer
  directly), `redact` reads it in place, and `hmac_sha256` now accepts
  the opaque Secret as the KEY (the recommended path — the material never
  leaves the Zeroizing buffer; the String key stays accepted for backward
  compat, both borrowed). The same-class find: the CalDAV session
  credential (`cal_connect` pass) lives in a `Zeroizing<String>` field —
  the copy is unavoidable there (the session stores it for the connection
  lifetime), so it round-trips inside the zeroizing buffer and is wiped on
  drop. The classification rationale (builtins_classification.rs) records
  the arg0-Private recommendation for `hmac_sha256` (the REFERENCE row
  follows the registry regen). The zero-copy invariant is pinned
  STRUCTURALLY (the №604 source-scan form — a heap copy is not observable
  at runtime, the source IS the observable): the named sites must not
  carry `as_str().to_string()` on the Secret arms. RED-before (форма
  №772): `hmac_sha256(secret("K"), msg)` was run against the pre-№807
  revision — the static check passed (the opaque posture) and the run
  refused LOUD (`hmac_sha256() key must be String`); the pins flip that
  refusal into the working contract on BOTH backends (the digest is
  carrier-independent: Secret and String keys produce the identical tag).
  security: YES — the B2 fix; the memory posture only tightens (fewer
  unsecured copies), no gate, allowlist or taint floor moves; the
  honest-boundary dictionary is untouched. AFFECTS: none functionally —
  every previously-working call keeps working; programs passing a Secret
  key to `hmac_sha256` (previously refused at runtime) now work.

- **№808 (gh#1564, Wave 66 — the audit-23.10 C5-b V4, the TW-oracle
  boundary gate):** the C5 (b) contract is now ENFORCED by a blocking
  CI gate — the VM executes everything a user runs; the tree-walking
  interpreter (TW) is the reference oracle. `scripts/ci/
  tw_user_path_gate.py` scans the user-path sources (`src/lib.rs`,
  `metalogos-server/src/server.rs`, `metalogos-server/src/bin/mlog.rs`)
  for the TW-entry calls (`run_program*`, `eval_program*`,
  `execute_route_body`) and demands an explicit `[TW-ORACLE]` mark in
  the source or a checked-in baseline row (`tw_user_path_baseline.txt`,
  `path :: anchor :: reason`); a new unmarked TW entry is RED
  (fail-closed: an unreadable file or a missing baseline is RED too —
  №525). The inventory at the landing: 9 entries, ALL marked in-source
  (the oracle runners `run_program`/`eval_program` + the `--reference`
  lanes in the CLI + the serve `interpreter` opt-out dispatch + the
  in-file test harnesses) — the baseline is EMPTY BY DESIGN (the marks
  ARE the documentation). The gate self-test (the №535 form) pins the
  shipped GREEN and the synthetic unmarked RED, wired as the new
  blocking job `tw-oracle-gate`. The crosscheck report wording moves to
  «VM vs the TW oracle» (the step name and the crosscheck header; the
  JOB names are untouched — the branch-protection contract, №471).
  `METALOGOS_SERVE_BACKEND=interpreter` stays transitional with the
  ADR-0197 banner (unchanged — the same term as `--reference`).
  security: no — the gate widens the refusal surface only (a CI
  boundary), no runtime gate, allowlist or taint floor moves. AFFECTS:
  none at runtime; a PR adding an unmarked TW entry now fails CI with
  the named remedy.

- **№809 (gh#1565, Wave 66 — the C3-a T3 levels, ADR-0199):** the
  typed-signature metric reads by MATURITY TIER — the PRIMARY
  definition is the stable-tier share (typed∩stable / stable:
  363/483 = 75.15% at the definition date); the full-registry figure
  (372/526 = 70.72%) stays reported beside it with its own untouched
  floor (ADR-0179), and the new stable floor
  (`type_signature_stable_baseline.txt`, 7515 bp) moves only up (the
  №757 procedure). The REFERENCE §6 index reads by the same levels —
  stable is the primary reference, experimental the opt-in tier,
  parked the appendix (the counts pinned by the new
  `reference_consistency` test: the headline triple, the per-tier
  headers, every row in its own tier section, the name sets equal to
  the registry per tier). The four feature-gated pillar jobs
  (candle/vision/voice/video) move out of the every-push CI into the
  `parked-features` lane (nightly + PR path filter on the pillar
  catalogs + dispatch) as the NIGHTLY health signal — the jobs stay in
  the every-push CI until the owner's protection toggle (the branch
  protection requires their four contexts; removing the jobs made
  every PR unmergeable — the episode caught it on the first head, the
  revision is in ADR-0199 §4), and the soft release rule lands in
  docs/maintainers.md: a red nightly does NOT block a tag but is
  recorded (the auto-filed fix issue + the CHANGELOG/limitations
  line). security: no — a docs/metrics/CI-lane re-aim; no gate loses
  its verdict power (the general floor is untouched, the new floor
  only adds). AFFECTS: the primary metric figure changes definition —
  both figures are quoted side by side from now on; the parked CI
  latency leaves the per-PR path.

- **№810 (gh#1566, Wave 66 — the C4-a T2 owner decision):** docs/REALITY.md
  is the CURRENT TRUTH only (≤30 KB, machine-pinned) — the §6.x
  wave-record history moved VERBATIM to the monthly archive
  docs/reality/2026-09.md + 2026-10.md (50 sections, the
  reconciliation figure is machine-pinned by
  tests/naryad_810_reality_split.rs — «закрытое не удаляется молча» is
  preserved by the transfer). REALITY.md carries the generated stub:
  the verbatim machine-truth block (the same collect_machines SSOT the
  records use) + the archive map (the map is checked against the
  archive files). The wave-record generator gains `--archive`: the
  records APPEND to the record-run month archive (idempotent) and the
  stub refreshes (scripts/gen_reality_current.py, --check is the CI
  gate). The language-lint allowlist carries the two archive files
  (the Russian prose is the RECORD content — the owner decision's
  sanctioned growth). security: no — a docs-layout change; the gates
  only strengthen (the size ceiling, the reconciliation counter, the
  map check are new only-up pins). AFFECTS: the wave records append to
  docs/reality/ from №810 on; docs/REALITY.md regenerates via
  gen_reality_current (≤30 KB enforced); the README links are
  unchanged (the current-truth entry stays the entry).

- **№811 (gh#1567, Wave 66 — the audit-23.10 B3, SSRF):** the V6 arm of
  the SSRF guard covers the two remaining IPv4-smuggling shapes — NAT64
  `64:ff9b::/96` (RFC 6052) and the legacy IPv4-compatible `::a.b.c.d`
  both UNFOLD and re-check through the V4 branch (the parity with the
  №261 mapped-form fix: `64:ff9b::a00:1` is 10.0.0.1 behind a NAT64
  gateway and used to pass as a "public" V6), and the documentation
  range `2001:db8::/32` (RFC 3849) is blocked outright (nowhere-
  routable; an egress there is a configuration against the operator).
  The special forms keep their standard lanes: `::` (unspecified) and
  `::1` (loopback) stay on the standard checks — the unfold requires a
  nonzero sixth segment (unfolding `::1` would produce `0.0.0.1`, which
  the V4 branch does not block). The №261 regression pin moves
  `2001:db8::1` out of the public set (the honest contract change, the
  replacement pin rides the neighboring routable prefix). Documented in
  threat-model.md and REFERENCE.md. RED-before (форма №772): the new
  unit pins ran against the pre-№811 revision — the NAT64/compatible
  private forms passed the guard (RED), the fix turns them into the
  pinned blocks. security: YES — the B3 fix; the block-list only
  widens, allowlist-конвенции №259 не тронуты. AFFECTS: programs
  egressing to `64:ff9b::…`/`::a.b.c.d`/`2001:db8::…` addresses now
  refuse with the loud `SSRF guard` error (they were silent passes
  before).

- **№812 (gh#1568, Wave 66 — the audit-23.10 B4, the assert crash):**
  `assert_contains` never PANICS on non-ASCII — the failure preview
  slices by CHARS (the byte slice `&haystack[..min(len, 80)]` crashed
  with `byte index is not a char boundary` on a Cyrillic haystack past
  80 bytes: the assert refusal was a program crash), and the ARGUMENTS
  in the error text go through the masking pipeline (`redact(…,
  "secrets")` — a token-like secret in the needle/haystack lands in the
  error MASKED: the error text is data-controlled). The char-safe
  preview helper carries an explicit ellipsis on truncation. RED-before
  (форма №772): the Cyrillic pin was run against the pre-№812 revision
  and PANICKED (the byte-boundary crash) instead of returning the
  refusal — the fix turns the crash into the pinned refusal. security:
  no — a crash-to-refusal fix plus redaction of error text; no gate,
  allowlist or taint floor moves. AFFECTS: programs relying on the raw
  (unredacted) error preview of assert_contains now see the masked
  form; the ASCII refusal shape is pinned unchanged.

- **№814 (gh#1579, hygiene — the wave-record PR-citation class, issue
  #1577):** the wave-record generator VERIFIES the PR citation — the
  squash-merge tail `(#N)` is a candidate, not a fact. A 404 (an issue
  number in the tail — the #1577 message-convention class), an unmerged
  record, or a missing tail resolves through the commit-association
  lane (exactly one merged PR) and the record carries a VISIBLE
  tail-ref-correction marker; unresolvable stays a loud `PR unresolved`
  marker — the citation is never an unverified number riding the audit
  trail, never a guess. The merged §6.65 record is corrected in place
  (the №762.1 form): №802 cites PR #1557 and №803 cites PR #1558 (the
  record run had swallowed the issue-number tails #1549/#1550). The
  acceptance ran live: the generator on wave 65 cites #1557/#1558 with
  the correction markers, no hand-editing. security: no — a
  reporting-path fix in a read-only generator; the weekly form
  (C4-a T1) is unchanged. AFFECTS: the wave records cite verified PRs;
  the convention breaks stay visible in the audit trail.

### Removed

- **№786 (gh#1484, Wave 63 — the grant-application sweep, the owner's
  directive 23.10, repeated 24.10):** the funding-programme mentions
  (the milestone M1 is closed by the owner — #1135) are swept from
  REALITY (17 lines) and PLAN-SUMMARY (6 lines); the sweep itself is
  documented in the current REALITY section (closed milestones are not
  silently deleted — the record keeps the fact, drops the programme
  names). The recurrence root — every wave dispatch re-planting the
  stale funding-programme owner gate — is closed by the NEW blocking
  docs-metrics step `scripts/ci/no_stale_grant_refs.py` (git grep over
  the doc surfaces; red on an artificially planted mention —
  self-tested). The language capability grants (`grant_issue`,
  `src/grants.rs`, ADR-0167) and the Open Collective sponsorship are
  untouched. AFFECTS: documentation only — no runtime, grammar, or
  registry surfaces.

### Fixed

- **№756 (gh#1408, Wave 57 — the two lane divergences from the №751
  (α)-walk):** the TW/VM lanes answer ONE contract on both discoveries.
  (1) `resolve_skill_index`: the VM lane answers the dept-qualified
  type_name and carries the truncation field now — `CompiledSkillIndex`
  carries `truncation` through the compile (bytecode.rs; the compiler
  maps the declaration's `TruncationMode`), the VM arm inserts the field
  in the TW spelling (`whole_skill_only` / `truncate_at_boundary`) and
  answers `format!("SkillIndex_{dept}")` (was: the fixed `"SkillIndex"`,
  the truncation TW-only — the ADR-0075 divergence the №751 walk pinned).
  (2) The absent-field READ is LOUD on both lanes (the ADR-0194 package
  A = (c) loud-fail contract): the VM `GetField` `Unit` fallback is
  removed on BOTH execution loops (`run()` and `execute_code_inner()`
  — was `unwrap_or(Value::Unit)`, the error masked as a value — the
  №604 class-4 shape); the `get_field` Err propagates — the same
  string the TW
  `FieldAccess` always propagated (`field 'x' not found on struct` /
  `cannot access field 'x' on non-struct value (...)`). The registry
  comment carries the resolution anchors; the №751 divergence pin
  re-pinned to the parity fact (форма №740, same-PR). Pins:
  `tests/naryad_756_lane_divergences.rs` (8: the dept-qualified
  type_name probe + the negative fixed-form control, the truncation
  both-modes + the absent-stays-absent, the absent-field and non-struct
  loud refusals on both lanes, the present-field happy path). AFFECTS:
  the VM lane semantics — absent-field reads now refuse loudly (the
  programs that relied on the silent Unit read the loud refusal; the
  TW lane unchanged); the `resolve_skill_index` answer gains the
  dept-suffixed type_name and the truncation field on the VM lane (the
  shape parity).

- **№762 (gh#1415, Wave 58 — the census/classification of the 129 quiet
  `unwrap_or(Value::Unit)` fallbacks in `vm.rs`, the №756 scope tail):**
  the loud operand contract (the ADR-0194 package A = (c) RATIFIED) is
  pinned for the reachable class. The census split every site by the
  live handler/opcode facts: **(a) 47 pinned, semantics unchanged** —
  (a1) 39 loud-through-result (the fallback feeds an evaluator that
  rejects `Unit` loudly on every path: the binop family through
  `eval_binop`'s coded mismatch, `Contains` on the run loop through
  `eval_contains`, `CmpGt/Lt/Ge/Le` through the `eval_cmp` catch-all —
  only `CmpEq`/`CmpNe` answer `Unit` silently, `GetField` through
  `get_field` (the №756 contract), the rule-value `FieldAccess` read,
  and `LoadGlobalByName` — every valid-program emission is a
  name-reference position whose miss lands the builtin's own loud
  `model not declared` text, which stays UNCHANGED); (a2) 6
  documented-soft language semantics (the `IndexAccess` in-value
  lookups — out-of-range index / missing key are valid-program events
  with the documented `Unit` answer); (a3) 2 internal register-stack
  pops (`EndValueExpr` over `value_registers`). **(b) 82
  reachable-masking — fixed loud:** the `.mbc` delivery path
  (`deserialize` — no version/integrity gate) makes stack underflow and
  slot-bounds misses reachable via a desynchronized or corrupt program,
  and the silent `Unit` fallback flowed into results, stores, branches
  and memory as the №604 class-4 shape (the error masked as a value) —
  every class-(b) site now refuses loudly naming the instruction and the fact
  (`VM: stack underflow at …`, the slot/range/name spelled out). Zero
  observable change on valid bytecode: the compiler guarantees every
  operand (`expr; Return` in every context, the №582 epilogue, the
  constructor-paired slots), all fixes are line-count-neutral (the
  registry `vm.rs:` anchors hold). Pins:
  `tests/naryad_762_unit_fallback_census.rs` (7: the run-loop and
  inner-loop underflow pins with the named refusals, the slot-bounds
  pins, the inner `Return` + the body-epilogue tail, the (a1)
  loud-through-result behavioral pins (Add/CmpGt/GetField refuse with
  the evaluator's text, no underflow text), the (a2) documented-soft
  pins, and the happy-path negative controls). AFFECTS: the VM lane
  semantics — a desynchronized or corrupt `.mbc` program now fails
  loudly at the first operand-stack violation instead of silently
  producing `Unit`-poisoned results/branches/memory (the programs that
  relied on the silent fallback read the loud refusal); valid programs
  are byte-for-byte unchanged; the TW lane unchanged (structurally
  incapable of stack underflow).

- **№765 (gh#1418, Wave 59 — the S-1b XSS closure, the external audit
  §3 H-1):** the let-form of render() is no longer a gap. The audit
  scenario — `let page = render("Comment", j.author, j.text)` →
  `respond_html(page)` — produced no finding and the raw slot carried
  the request data straight into the page. Three layers move together:
  (1) `builtin_render` computes the RAW-slot param names from the
  template body (the SAME `template_scan::raw_param_positions` the
  audit taint layer and the html_label walk read — ONE scan, four
  consumers) and requires `Value::Html` for them exactly like the safe
  slots since №695 — the fail-closed refusal `RAW_SLOT_NOT_HTML` names
  the slot and the fact (the ungated dynamics: the VM lane refuses in
  the real pipeline; the TW lane's checked programs hit the static
  implicit-coercion gate first). (2) `html_label::walk` — the BINDING
  walk resolves the raw positions through the shared resolver (a
  StringLit name or a bare Ident that names a DECLARED template —
  render is a name-reference builtin; a variable-carried name stays
  the conservative unknown, the №685-E pin) and answers
  `Labeled(String, Untrusted)` when a labeled argument sits at a raw
  position; the sink walk keeps its №685 contract. (3) render is OUT of
  the blanket sanitizer lists (the binary predicates: the memory-taint
  layer, the interp state walker) and POSITION-AWARE in the label
  engines (the raw map decides per slot: the all-escaped template keeps
  the sanitizer semantics — the №685-D/n98 pins; any raw slot conducts
  the join of its raw args; the dynamic name stays the conservative
  conductor). The check path seeds the raw map now (the same
  re-seeding discipline the audit entries follow). Pins: NEW
  `tests/naryad_765_s1b_let_form.rs` (7: the audit scenario flagged on
  BOTH surfaces — red before the fix, green after; query_param into a
  raw slot; render→memory→respond_html keeps the taint fact; the TW
  static coercion refusal + the VM runtime backstop refusal; the Html
  producer composition runs; the render(render()) negative; the
  all-escaped interlock) + 2 inline units in http.rs (the raw-slot
  backstop at the pub(crate) boundary). The interlocks hold: №685
  24/24, №695 4/4, №686 3/3, №376 4/4, №448 11/11, №201 8/8,
  template_integration 8/8, n98 8/8. AFFECTS: programs that relied on
  render-as-sanitizer for RAW slots now get findings (the static layers)
  and runtime refusals (a plain String in a `{{{ }}}` slot); the
  all-escaped templates keep the previous behavior byte-for-byte; the
  escape_html status untouched; the raw semantics of №685/№695/№696 not
  regressed.

- **№766 (gh#1419, Wave 59 — the diff-fuzzer H-2, the external audit
  §3):** the fuzzer's statement generator is RECURSIVE now — the old
  `gen_stmts` carried a `_depth` parameter it never used, and the
  «form A inside form B» class (№369: the nested match; gh#1336: the
  nested each — both found by CONSUMERS, not by the fuzzer) was
  unmeasurable by construction. (1) `gen_stmts(depth)` at every level
  picks among the SIX nesting forms (if/else, bounded while, each,
  each-with-index, match, the flat pool) and recurses into the body at
  depth-1 (the audit's п.1). (2) Every generated block appends a
  deterministic TAG to the trace accumulator the program prints in its
  return — any silently dropped block is a TW/VM divergence by
  construction, not by luck (п.2). (3) The each-in-match-arm pins land
  in `tests/n1336_vm_each_in_if.rs` (the parity + the liveness — the
  accumulator is read AFTER the match; the each-with-index form too) —
  the last row of the audit's §6 п.3 list (п.3). (4) The nesting-pairs
  metric (форма-родитель × форма-ребёнок): {top, if, while, each,
  each_idx, match} × {flat, if, while, each, each_idx, match} = 36
  cells, all reachable — the achieved coverage is 36/36 over the seed
  sweep; the floor lives in
  `scripts/ci/fuzz_nesting_pairs_baseline.txt` (the only-up ratchet:
  a lower count is a regression, a higher count DEMANDS the bump in
  the same PR) and the CI log carries the 6×6 table (п.4). (5) The
  mutation probe: the Each-arm sensitivity is pinned BY CONSTRUCTION —
  the sweep programs whose output carries an each-trace tag diverge the
  moment the Each arm stops emitting code; the probe asserts the
  sensitive share (≥40% of the executed sweep) — disabling the Each arm
  in `compile_stmt_with_locals` sends the run red (п.5; the manual
  procedure — the №503/№618 form — in the naryad report). The
  generator's liveness holds: the undefined-ident error-class lane
  scaled 1/8 → 1/48 per leaf pick (the recursion multiplies the leaf
  picks; the PER-PROGRAM refusal rate returns under the №621 sweep
  floor — the lane keeps its coverage). The audit's «Не делать» held:
  no new fixed fragment — the coverage is systemic. Pins: the fuzzer
  suite 13/13 (the legacy n1336 shape-production probe re-pinned to the
  pairs (if, each)/(while, each)), n1336 9/9. AFFECTS: none for the
  production code — the change is the test generator + the metric (the
  compiler/VM semantics untouched).

- **№768 (gh#1421, Wave 59 — the audit 21.10 §3 M-2 + L-1):** the 0.31
  narrative lands (the third carry — the audit's «это пять минут
  работы») and the mirror counter stops lying about its CWD. (1) The
  version-line narrative — «0.31 was never released; its goal folded
  into 0.32.0» — a Note subsection in the historical [0.32.0] section
  (the gh#1350 Note form; the tags confirm: `v0.30.1` → `v0.32.0`, no
  `v0.31.*`) + the same note in the REALITY §6.29 record (the
  historical sections gain a line, not a rewrite). (2)
  `scripts/ci/mirror_counter.py` builds the scan root from
  `Path(__file__).parents[2]` — the old CWD-relative glob printed a
  bare `0` (exit 0) when run from `scripts/ci/`, a false green for a
  metric that can only move DOWN (the audit L-1, reproduced 1-в-1);
  now any CWD prints the honest 6 (verified from the root,
  `scripts/ci/` and `scripts/`), and a missing `src/` tree fails LOUD
  (exit 2) — never a quiet zero. The CI invocation from the root is
  unchanged; the metric value (6, threshold 6) is untouched.

- **№772 (gh#1435, Wave 60 — the prod bug gh#1430, the fosved-acc
  АСС-1.9 finding on v0.33.2):** catching a pattern-call error with
  `try` no longer wipes the caller's frame locals on the VM. The
  unwound CallPattern left its CallFrame installed (the `?` propagation
  skipped the cleanup the success path performs), so every later
  LoadLocal/StoreLocal resolved against the stale base_bp — the pre-try
  locals read back as Unit and the reported `Unit Sub Float` /
  `Float Sub Unit` killed the route (the fosved-acc site monitor went
  dark during an outage on the serve VM lane; TW kept the values — the
  divergence the consumer found before the machine, the gh#1336 class
  again). (1) The fix: BOTH TryEval arms (the run loop and the inner
  loop — форма №762) snapshot the stack and call_stack depths before
  the guarded code runs and restore both on catch — the same discipline
  `run_deny_handler` (№392) applies to its own body on every path;
  nothing is invented, the loud contract (ADR-0194 (c)) is unchanged —
  the error still prints once and still lands in the result struct; TW
  (the reference) untouched. (2) The pins:
  `tests/n1430_try_pattern_unwind.rs` — 6 tests, 4 RED before the fix
  on unpatched main (the issue's p3 shape 1-в-1 with TW, the consumer
  ms shape, the nested-frames shape, the route-body lane through
  `execute_route_code`) + 2 negative controls that were green before
  and must stay (the inline-builtin boundary, the try success path).
  (3) The differential fuzzer (the bug's ask 3) carries the class shape
  now — `try boom()` → catch → a trace append reading a PRE-try local;
  verified honest: with the fix stashed the fuzzer diverged loudly
  (`Unit + String` on the VM), with the fix it is green 13/13 — the
  class cannot return quietly. AFFECTS: VM programs that relied on
  reading pre-try locals as Unit after a caught pattern-call error now
  see the TW values (the divergence was the defect; TW behavior
  unchanged byte-for-byte; the inline-builtin try boundary unchanged).

- **№776 (gh#1445, Wave 60 hygiene tail — the №771 form):** the
  gate-facts-sync verdict read survives a GitHub API filter flap. The
  `status=completed` runs-list filter answered `total_count=0` on the
  0-side of a server-side flap while the same endpoint without the
  filter showed all five completed GREEN runs (the evidence log in
  gh#1445: 0,0,0,5,5,5,0,5 on identical queries ~20s apart); two main
  runs failed fail-closed INFRA (№525 — the honest behavior, the gate
  did not pass an unreadable state). The empty-list answer is retried
  now within a bounded window (6 attempts × 20s ≈ 2 min; the observed
  0-side of the flap is ~20-80s); the verdict still comes ONLY from
  completed runs, an API exception stays the loud immediate exit 2,
  and the exhausted retries still fail closed — the №525 contract is
  unchanged (the retry never turns an unreadable state into a pass).

- **№783 (gh#1494, Wave 62 — the audit-23.10 A4/S-3+N-2, P1):** the
  query* family no longer loses parameters silently, and `each` over a
  non-List refuses identically on both backends. (1) The params SSOT —
  `bind_params(who, args, idx)` in db_ops.rs now serves ALL SIX db
  functions (query, query_vm, query_row, query_scalar, db_execute,
  db_execute_with_grant): the pre-№783 query* family answered a
  non-List second argument with `Vec::new()` — SILENTLY
  (`query("… WHERE id = ?1", user_id)` bound ZERO parameters and,
  without a mandatory placeholder, passed quietly with the caller's
  data never reaching the engine); the refusal NAMES the `[x, y]`
  form, the №553 loudness contract moves into the SSOT (its pins ride
  the landed wording). (2) The VM `each` over a NON-List answered 0
  iterations silently (ListLen → 0.0) while the TW refused — a live
  TW/VM divergence of the gh#1336 class (a client-string
  `json_body()` made the VM lane skip the loop); both ListLen arms
  now carry the byte-identical TW coded refusal
  (`[TYPE_MISMATCH] each: expected List, got <type>`). (3) The №766
  diff-fuzzer carries the class shape — one in 31 generated iterables
  is a String literal (the refusal-parity shape; the executed-counter
  floors of the №621/№766 probes hold — the form stays rare at the
  statement level). The 7-pin battery (4 functions × both backends +
  the TW/VM each-parity pair + 2 happy-path controls) verified
  RED-before on the pre-№783 engines: 5/7 failed (the quiet Ok
  instead of the refusal, the old №553 wording, the VM divergence);
  the №553 wording pins re-spelled to the landed form. AFFECTS:
  programs that passed a non-List params argument now catch the loud
  refusal (the redact-equivalent `[x, y]` List form stays the legal
  path); the each-over-non-List programs now refuse on the VM lane
  exactly as the TW lane always did.

- **№785 (gh#1507, Wave 62 hygiene tail — the №776 form):** the
  cargo-audit/cargo-deny RED on main (`e77a93c`) — `RUSTSEC-2026-0385`
  (ctutils 0.4.2: the non-constant-time mask generation / `u128`
  comparisons) landed in the RustSec advisory database BETWEEN the CI
  rounds (the same commit graph was GREEN an hour earlier on the
  tokio bump — the base flap, not a diff regression; the №780 diff
  does not touch Cargo.lock). The bump: ctutils 0.4.2 → 0.4.4
  (`cargo update --precise`, the single Cargo.lock hunk) — the
  constant-time comparison stack (the №1002/gh#1211 `ct_eq` contour)
  rides the fixed release. 0 vulnerabilities after (5 allowed
  unmaintained/yanked warnings, the advisory class); the
  0367/0368/0369 zip ignores are untouched (UPSTREAM-BLOCKED, the
  2026-11-17 expiry review stands).

- **№787 (gh#1510, Wave 63 — audit-23.10 C6 I-c):** the integer
  precision-loss boundary — a SQLite/JSON integer f64 cannot represent
  exactly (|n| > 2^53: 64-bit ids, snowflakes) is a LOUD
  `[PRECISION_LOSS]` coded refusal naming the column (the four
  SQLite row-conversion sites — query/query_scalar/query_row on both
  lanes through the ONE shared `value_ref_to_value`) or the document
  path (`parse_json`/`config_load`/MCP tool schemas/the VM LlmResponse
  — the JSON `unwrap_or(0.0)` arm is gone). The exactly-representable
  integers (|n| ≤ 2^53 and the exact powers of two beyond it, e.g.
  i64::MIN) ride the UNCHANGED Float behavior — the boundary battery
  pins 2^53 and i64::MIN as value-comparisons (RED-before: the reads
  returned the distorted 9007199254740992). AFFECTS: programs that
  relied on the quiet round-trip now catch `[PRECISION_LOSS]`
  (try-classified on both backends); re-read lossy columns as TEXT or
  CAST in SQL.

- **№797 (gh#1535, Wave 64 — audit-23.10 B5/D-1, P2):** the AGENTS.md
  stale facts (the file every coding agent reads first) — three
  corrections: (1) the §5 coverage claim ("~59%, not 100%") is
  re-spelled to the current machine truth (the GENERATED index covers
  100% of the registry; the curated prose may lag — the registry grep
  stays the SSOT); (2) the §6 hardcoded "all 110 in a row" ADR count
  is REMOVED per the docs-metrics form (the number is regenerated by
  the machines, never hand-maintained — the header says so); (3) the
  §6 "VM parity with TW … deliberately not implemented (ADR-0105)"
  is re-pointed to the current architecture: the parity is a BLOCKING
  contract, the VM is the single executor of the user commands and TW
  is the reference oracle in CI (`ADR-0197` supersedes the `ADR-0105`
  remainder — the C5 owner decision). The §8.5 site-files audit item
  (secret_label.rs / sql_label.rs missing from the sanitizer-layer
  list) had ALREADY landed with №781's A2 additions — verified, no
  edit needed. CLAUDE.md re-synced byte-for-byte (the №763 identity;
  the agent-index gate green). AFFECTS: documentation only.

- **№798 (gh#1536, Wave 64 — audit-23.10 B6/D-2, P3):** `llms.txt` is
  ENGLISH — the owner's 2026-09-17 "documentation in English only"
  directive is now machine-enforced for the file too. The pre-№798
  state: the file carried Russian prose wholesale (1298 Cyrillic
  letters / 39.4% — the verified scan) while
  `tests/docs_language_lint.rs` walked `.md` files only, so the
  directive was declared but not enforced for the agent-facing index.
  Three halves: (1) the full translation (the structure, the links
  and the machine-readable counters preserved — the agent-index gate
  counters identical); (2) the lint's scanned set gains the explicit
  NON-`.md` agent inputs (`llms.txt`) — the coverage pin + the
  scanner's negative control (a synthetic Russian document is
  flagged, an English one passes; no repo file is mutated); the RED
  form verified honestly: the extended lint against the pre-№798
  Russian `llms.txt` FAILS with the 39.4% verdict (stash-run, форма
  №772); (3) the agent-index gate's two Russian-coupled llms patterns
  (`рабочих .mlog`, `наряды №1-№N`) move to the English spellings —
  a regression back to Russian now trips the gate twice (the missing
  statement + the lint). The `.rs` Cyrillic comments stay OUT of
  scope (the owner's gray-zone decision, the audit's own note).
  AFFECTS: documentation + the lint surface only.

- **№813 (gh#1578, hygiene — the weekly merge-ci-audit exit-2 class, issue
  #1577):** the detective treats a 404 on the squash-merge tail ref as the
  DETERMINATE message-convention finding it is — the association lane
  (`GET /commits/{sha}/pulls`, exactly one merged PR) resolves the real PR
  and the break stays a VISIBLE divergence line (a convention break never
  heals silently); the transient lane (5xx, throttling, network) stays
  fail-closed exit 2 (№525: an unreadable state is not a pass). The first
  trigger: №802/№803 carried the ISSUE numbers (#1549/#1550) in the tails —
  the scheduled run 40309903631 died exit 2 without a single verdict; the
  manual verdict (issue #1577) also surfaced six tail-less merges
  (№762/№762.1/№763/№765/№766/№729-tail) — all eight resolved via
  associations, all heads 13/13 success, zero M-1 findings. The self-test
  carries the resolver cases (exactly-one-merged resolves;
  zero/several/none-merged stay unresolvable). security: no — a
  reporting-path fix in a read-only detective; the required set, the
  fail-closed posture and the report-only boundary are pinned unchanged.
  AFFECTS: the weekly audit reports convention breaks as exit-1 divergence
  lines instead of dying unreadable; the merge-convention tail `(#PR)` is
  enforced by visibility, not by silence.

- **№815 (gh#1580, hygiene — the conflict-marker class):** `llms.txt`
  on main carried a RAW git conflict block (landed with №806/0e4ce59,
  carried through four merges and every gate — the agent-index gate
  covers the machine-generated index lines, not the hand-maintained
  region; the language lint checks the language, not the structure).
  The block is resolved to its HEAD side (the CHANGELOG reference
  line); the tripwire lands in `docs_language_lint` — a stateful
  scanner (the seven-equals separator counts only inside an open
  marker block, so the setext underlines stay content) pins that no
  tracked doc carries conflict markers, and the synthetic pin proves
  the shape is caught. security: no — a docs-hygiene fix + a structural
  tripwire; no gate loses coverage. AFFECTS: the conflict-marker class
  is grounded — the silent pass-through is impossible from now on.

### Security

- **№780 (gh#1491, Wave 62 — the audit-23.10 A1, P0):** the runtime
  label gates (`LabelJoin`/`SinkCheck` — the №325/ADR-0156 runtime
  twin, the `[SINK_CLEARANCE_RUNTIME]` verdict) execute in the
  pattern/route bodies too. Two halves of one hole: the compiler did
  NOT arm the body's sink sites (`compile_pattern_body_with_locals`
  had no `emit_armed_sink_checks` on `ExprStmt`/`Return`), and the VM
  inner loop (the pattern/route executor) dropped both label
  instructions into the wildcard tail — the serve route bodies (the
  server code) never ran the sink-clearance check, the label env
  stayed unseeded there. The fix: the compiler arms the body sinks
  symmetrically to `compile_stmt_with_locals` (the deny path №392
  included), the VM inner loop executes the label instructions from
  the SAME executors as the main loop, and the main-only instructions
  answer LOUD in a body
  (`VM: instruction <X> is main-only — compiler/VM skew`) instead of
  the quiet skip. The split (inner vs main-only, 46+12) is pinned by
  the exhaustive enumeration test — a new `Instruction` variant
  without a class is a compile error; ADR-0156 §1 carries the
  №780 note (the contract sentence is now the fact);
  docs/wildcard-tail-ledger.md updated. Red-before: 5/6 of the new
  pins fail on the unpatched main (verified by a stash-run). The
  verification sweep also caught the COMPILER half of the hole (the
  body sink sites compiled WITHOUT the armed check at all — the
  debug byte dump) — both halves are closed by this naryad. The
  variable sink arguments carry their local slot (serde-default,
  old .mbc files deserialize cleanly) so the VM reads the value
  behind the name and keeps the typed materialization canon
  (№355/№691: a WorldState handle refuses `WORLD_STATE_PRIVATE`,
  the sst_001 pair, on BOTH backends) even when the twin fires
  before the shared builtin leg.
  AFFECTS (форма №756): programs relying on the missing body-level
  runtime gate (a private label reaching a sink inside a
  pattern/route body) are now refused loudly on the VM — the TW
  static gate always refused; the serve lane now agrees with the
  static verdict. The main-only skew error is a new loud class for
  hand-built bytecode only (the compiler emits none).

- **№781 (gh#1492, Wave 62 — the audit-23.10 A2, P0):** escaping a
  secret is still a secret. The `escape_html` builtin and the ESCAPED
  slots of `render` restored the INTEGRITY of the data but carried an
  unconditional declassification — the result was treated as
  `Public`, so `respond("200 OK", escape_html(env("API_TOKEN")))`
  passed every gate on all three label lanes. The fix (the N2/
  gh#1211 ct_eq form, one shape in every lane): the INTEGRITY is
  restored (the escape still cleans untrusted markup — the №685/
  №695/№765 XSS contract is NOT weakened), the CONFIDENTIALITY is
  the join of the arguments. The lanes: `secret_label.rs` — the
  render/escape_html overrides are removed from `label_of_expr` and
  `binding_label` (the argument join decides); the label lattice
  (`semantic.rs` `label_source` + the `render` position-aware arm,
  `audit.rs` `interp_expr_label` — both engines symmetric) — an
  escaped slot restores the integrity only, a raw slot conducts
  everything (№765 unchanged), an unknown/dynamic template name
  stays the conservative conductor. The audit A5-class stakes:
  `secret_label.rs`/`sql_label.rs` join the risk-review site set
  (№686, the item-10 reminder fires on their diffs; AGENTS.md §8.5
  updated, the self-test set-pin moves 4 → 6). The threat-model
  Boundary 3 carries the corollary verbatim (escape restores
  integrity only, never lowers confidentiality; the redact-policy
  registry stays the only path down). The leak-suite quartet pins
  the hole RED-before (verified on the unpatched main: 4/4 compiled
  — NOT CAUGHT; a stash-run of the mutation probe shows every pin
  losing the SECRET_LEAK class to the PII_EGRESS_* mismatch —
  the pins are hard-wired to the lane); the A2 fences
  (`ok_781_escape_html_untrusted_respond`, `ok_781_secret_hash_only_
  respond`) keep the false-positive side green. AFFECTS: programs
  that sent an escaped env-secret to a sink now catch `SECRET_LEAK`
  at compile time (the redact one-way policies — `hash_only`/
  `secrets`/`all` — stay the legal path); everything else compiles
  unchanged. security: yes — the compile-time gate grew teeth on the
  escaped-secret egress class.

- **№782 (gh#1493, Wave 62 — the audit-23.10 A3, P0):** the executor
  resource limits — `serve` can no longer be killed by a runaway
  program. Three halves: (1) the string capacity — ONE SSOT
  (`src/limits.rs::MAX_STRING_LENGTH`, the old two drift-prone local
  copies in vm.rs/execution.rs are gone); the multiplicative builtins
  (`repeat`, `pad_left`, `pad_right`) check `unit_len * count` BEFORE
  the allocation — the old `s.repeat(1e12)` died in the allocator
  (an abort, not a panic — `spawn_blocking` cannot catch it, the whole
  serve process went down); `join` computes the exact need up front,
  `format`/`word_wrap` carry the loud input/result checks (their
  growth is input-proportional). (2) the native-recursion depth cap —
  the CallPattern recursion (`pattern R(n) { return R(n+1) }`) used to
  overflow the thread stack (in serve the route bodies run inside
  `spawn_blocking` with the 2 MiB stack — the overflow ABORTED the
  serve process); BOTH executors guard the entry now (`[CALL_DEPTH_EXCEEDED]`,
  default 512, `METALOGOS_VM_MAX_DEPTH` is the deployment knob, the
  VM run path executes on a 256 MB thread (the №197 form), the serve
  blocking pool runs 8 MiB stacks). (3) the main VM loop
  (`execute_main_code`) carries the iteration counter mirrored from
  `execute_code_inner` — an eternal top-level loop hung `mlog run`
  forever. The subprocess pin battery (6 tests, both backends) verified
  the RED-before on the unpatched main: the alloc abort + the stack
  overflow, byte-for-byte the audit's symptoms; the `while`-shape pin
  patches the compiled Halt into a self-jump (the top-level grammar
  cannot express the eternal main loop — the patch is the honest
  construction). The new section in docs/limitations.md: the defaults,
  the knob, what the caps do NOT cover (the opaque subsystems'
  allocations). AFFECTS: programs that relied on the unlimited
  recursion/string growth (below the limits the semantics is
  unchanged — crosscheck green).

- **№784 (gh#1495, Wave 62 — the audit-23.10 A5/S-8, P1):** the
  authenticate mock answered `Ok(Unit)` while the REFERENCE row
  promised "In a server context, checks the credentials" — a promise
  with no code behind it (no server user-store exists on either
  backend). Every call now refuses with the coded `[NOT_IMPLEMENTED]`
  refusal naming the real credential path —
  `verify_password(password, hash)` against the stored hash (the
  ADR-0198 loud-failure line); the code joins `ORIGIN_STAMPED_CODES`
  (the №413 mechanics) so `try authenticate(...)` can branch on it.
  The 5-pin battery (the two loud-refusal pins, the two
  validation-refusal regression pins, the try-code pin) verified
  RED-before on the pre-№784 engines: 3/5 failed (the printable Unit
  instead of the refusal; no try code). The REFERENCE row (curated +
  the generated index) is re-spelled honestly, and the regeneration
  surfaced a generator defect fixed in the same PR: the curated-row
  harvest split table cells on EVERY pipe, so the Markdown-escaped
  union signatures (`String\|String`) were cut mid-cell and 17
  generated rows (authenticate, len, to_float, to_int, parse_json,
  call_llm, http paths, send_message, send_photo, render, inspect,
  reflex_metrics, vec_store, vec_search) carried truncated signatures
  with the return-type column riding in as the description; the
  harvest now splits on unescaped pipes and the cell escaping is
  idempotent — all 526 rows render intact (gen_reference --check
  526/526, TODO(doc) 0, curated sections untouched). AFFECTS:
  programs calling authenticate now catch the loud refusal (the
  migration is the named verify_password path); hash_password /
  verify_password untouched.

- **№795 (gh#1533, Wave 64 — audit-23.10 C5-b V2, the C5 (b) owner
  decision, ADR-0197; fix date 2026-10-24):** `mcp-serve` executes tool methods on the VM —
  the SAME compile-path as serve (the strict VM compile at startup; a
  non-compilable file refuses the gateway loudly, where the pre-№795
  TW path silently ignored the non-tool declarations' run errors).
  The capability the switch buys: the RUNTIME label gates
  (`[SINK_CLEARANCE_RUNTIME]`, the №780/ADR-0156 machinery) fire IN
  TOOL BODIES — a private value routed into a sink at runtime is
  refused loudly through the tools/call envelope, where the pre-№795
  TW path answered green (the leak went through; the TW lane has no
  runtime label twin). The invocation rides the new
  `Vm::invoke_pattern` (the CallPattern convention: the JSON args
  bind positionally per the declaration order; the per-request depth
  budget and the backend-tag contract match `execute_route_code`).
  The gates are unchanged: `ServeRouteExecGuard` (№253-А/№259),
  the Category-A startup gate, the allowlist, the exec/env posture.
  The 3-pin battery (the runtime-gate pin + the honest parity pins)
  verified RED-before via the changes-stash run (форма №772): the
  runtime pin FAILED on the pre-№795 TW path (the green answer —
  the leak), the parity pins green. The existing mcp suites
  (№394/№477/№536/№793/№794) stay green on the VM lane. AFFECTS:
  gateway tool files must compile on the VM lane (the strict form);
  the tool bodies gain the runtime label enforcement.
