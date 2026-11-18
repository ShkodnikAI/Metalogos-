# Metalogos v0.40.0 — Black-Box Adversarial Security Pass (Report of Record)

## 1. Header

- **Candidate binary**: `/home/z/my-project/metalogos-work/repo/target/release/mlog`
- **SHA256**: `f4a665bdfa6fecd2542c07f68fd7c9e6bd12cec688d2a96432d49c6108d30859` (copied from `/home/z/my-project/metalogos-work/redteam-973/candidate.sha256`; independently re-hashed at run time — match confirmed)
- **Date**: 2026-12-01
- **Pass scope**: Black-box adversarial pass against the promise sheet (`README-candidate.md`, verbatim copy in `pocs/PROMISES.md`) — the Three Core Ideas (strict security contexts; end-to-end labels/provenance with compile-time AND VM sink checks; loud typed degradation) and the "What it stops" refusal classes (SECRET_LEAK, SECRET_EGRESS_NETWORK, SECRET_TO_EXEC, SQL_DYNAMIC, HTML_INJECTION, IRREVERSIBLE_NO_GRANT, UNTRUSTED_EXEC_DECISION). The adversary authored 24 probe programs from the promise wording alone, executed them through the candidate binary only (`check`, `check --reference`, `audit`, `run`, `test`, `serve`), and read nothing under the repo tree. Probes and per-probe transcripts live in `pocs/` (`pNN_*.mlog`, `pNN_transcript.txt`; transcripts carry the real run timestamps, binary hash, full raw output and exit codes).

## 2. Executive summary

**24 probes: 18 CAUGHT, 2 BYPASS, 4 INCONCLUSIVE.** The label lattice is real and strong: every secret/injection source-to-sink shape I could author from the promise wording was refused at compile time on BOTH execution backends, echoed by `audit`, and re-enforced by the run/test compile gates — including through struct fields, joined lists, mutable branch joins, LLM output (mock), cross-pattern returns and cross-pattern arguments (the №950 boundary closure holds, with refusals naming the full call chain). The two bypasses are both in one promise: **IRREVERSIBLE_NO_GRANT never fired in any shape I could construct** — a literal `DROP TABLE users` through the `query()` SQL sink passes `mlog check` on both backends, is certified by `mlog audit` ("all 1 SQL-carrier calls use literal SQL ✓"), and `mlog run` exits 0 silently; the same holds for a destructive payload carried by `cron_run`, the carrier the binary's own `approve` help names as the human-oversight roster target. Nothing reached any sink in those probes (no database is wired at runtime), so the practical impact is a missing static gate, not an observed destructive execution. The degradation promise (typed `Degraded(t)` on ladder falls) could not be confirmed or denied: the advertised in-program local-mock rung never engages, and every ladder scenario terminates in a loud typed `[LLM_PROVIDER_UNAVAILABLE]` refusal instead of a typed degradation event — recorded as INCONCLUSIVE with the anomaly documented. Overall verdict: **the compile-time security core does what the sheet says it does, with one promise line (irreversible actions) not upheld as written, plus a loudness gap in the degradation story that I could not resolve black-box.**

## 3. The probe table

| Probe | Promise attacked | `mlog check` verdict | `mlog run` / `test` verdict | Classification |
|---|---|---|---|---|
| p01_secret_leak | SECRET_LEAK — env() into respond() | REFUSED: `[SECRET_LEAK]` + `[PII_EGRESS_OUTPUT]` (both backends; audit: 2 errors) | run gate: `Category A security invariant violated`, exit 1 | CAUGHT |
| p02_secret_egress_network | SECRET_EGRESS_NETWORK — env() into http_post | REFUSED: `[SECRET_LEAK] "env() value passed as http_post body"` + `[PII_EGRESS_NETWORK]` (both backends) | run gate refuses, exit 1 | CAUGHT (class token is `PII_EGRESS_NETWORK`, not the sheet's `SECRET_EGRESS_NETWORK` — refusal fires; name mismatch noted) |
| p03_secret_to_exec | SECRET_TO_EXEC — env() into exec() | REFUSED: `[SECRET_TO_EXEC]` exact class, argument 0 (both backends) | run gate refuses, exit 1 | CAUGHT |
| p04_sql_dynamic | SQL_DYNAMIC — request param AND network data concatenated into SQL | REFUSED: 2× `[SQL_DYNAMIC]` (q1 param-concat, q2 http_get-concat) | run gate refuses, exit 1 | CAUGHT |
| p05_html_injection | HTML_INJECTION — network output wrapped into HTML → respond() | REFUSED: `[HTML_INJECTION]`, threshold `untrusted-egress` | run gate refuses, exit 1 | CAUGHT |
| p06_irreversible_no_grant | IRREVERSIBLE_NO_GRANT — `query("DROP TABLE users")` without grant | **PASSES** ("OK: no issues found") on both backends; audit certifies "all 1 SQL-carrier calls use literal SQL ✓" | run: silent, exit 0 — no gate anywhere | **BYPASS** |
| p07_untrusted_exec_decision | UNTRUSTED_EXEC_DECISION — http_get result steering exec() | REFUSED: `[UNTRUSTED_EXEC_DECISION]` exact class (both backends) | run gate refuses, exit 1 | CAUGHT |
| p08_struct_label_flow | Core Idea 2 — secret label through a struct field into sink | REFUSED: `[PII_EGRESS_OUTPUT]`, label `'private, trusted'` carried through `payload.token` | run gate refuses | CAUGHT |
| p09_list_label_flow | Core Idea 2 — secret label through a list + join into sink | REFUSED: `[SECRET_LEAK]` + `[PII_EGRESS_OUTPUT]` — label propagated through `join()` | run gate refuses | CAUGHT |
| p10_llm_taint_sink | Core Idea 2 — LLM output (mock) into HTML respond() | REFUSED: 3 findings — `[HTML_INJECTION]` (LLM-specific), `[TAINT_INTERP]`, sink clearance `untrusted-egress` | run gate refuses; test lane refuses identically (mock on/off) | CAUGHT |
| p11_mock_unset_loud_refusal | Core Idea 3 — AI call with no backend must refuse loudly | clean (compile-time sees nothing — refusal is runtime) | `mlog test` without env: loud typed №898 refusal listing every config option, exit 1; with `METALOGOS_MOCK_LLM=1`: passes, deterministic `[mock-llm:<hash>]` answer | CAUGHT |
| p12_degradation_ladder | Core Idea 3 — backend ladder falls with typed `Degraded(t)` event | clean | ladder fall terminates in typed `[LLM_PROVIDER_UNAVAILABLE]` (loud) — **no `Degraded(t)` observed**; advertised in-program local rung never engages (see §5) | INCONCLUSIVE |
| p13_cross_pattern_return | №950 — pattern's return label carries sources inside its body, any depth | REFUSED: helper's env() label reaches caller's respond (`'private, trusted'`) | run gate refuses | CAUGHT |
| p14_cross_pattern_arg | №950 — argument checked against callee parameter clearance | REFUSED, naming the whole chain: `Caller → SinkParam(…) → respond (parameter 0 reaches the sink)` | run gate refuses | CAUGHT |
| p15_vm_runtime_sink_gate | Core Idea 2 — sinks check clearance in the VM (runtime), not only compile | REFUSED already at check (sink inside `test` block is in lattice scope) | run AND `mlog test` refuse (`Category A security invariant violated`) | CAUGHT (note: the compile gate always precedes execution, so a VM-only verdict is unreachable by design — recorded in §6) |
| p16_secret_in_status_arg | CREATIVE — Core Idea 2 sink-argument coverage: secret in respond()'s FIRST (status) argument | REFUSED: `[SECRET_LEAK]` + `[PII_EGRESS_OUTPUT]` on **argument 0** | run gate refuses | CAUGHT |
| p17_retaint_after_redact | CREATIVE — Core Idea 2 label union: redact() then concat with env() | REFUSED: `[SECRET_LEAK]` + `[PII_EGRESS_OUTPUT]` — re-taint via `+` caught; `[REDACT][audit-event]` emitted even during check | run gate refuses | CAUGHT |
| p18_branch_join_secret | CREATIVE — Core Idea 2: taint through a mutable if/else branch join (`let mut` + assignment) | REFUSED: `[SECRET_LEAK]` + `[PII_EGRESS_OUTPUT]` — flow-sensitive join caught | run gate refuses | CAUGHT |
| p19_self_approve_drop | CREATIVE — approval gate: `approval_issue("self",…)` then DROP; mandate "NEVER counts" | PASSES (identical to p06 — the gate never fires, so the "never counts" side is untestable) | run: silent, exit 0 | INCONCLUSIVE (nothing to bypass — see §5) |
| p20_exec_redacted_secret | CREATIVE — SECRET_TO_EXEC boundary: redact() clearing the exec sink | PASSES as the README documents ("the secret is REDACTED before the sink — passes"); `[REDACT][audit-event]` fired | run: exit 0, audit-events fired at runtime; no unexpected leak path | CAUGHT (documented positive path behaves exactly as promised) |
| p21_serve_env_allowlist | Core Idea 1 — serve-route context: env() denied without allowlist | parse error — `route_decl` grammar not constructible black-box (~50 shapes tried) | `mlog serve` started, printed the strict-context banner LOUDLY (`route exec: denied … METALOGOS_SERVE_ALLOW_EXEC=1`; `route env: denied … METALOGOS_ENV_ALLOWLIST`; LLM №898), then died on my parse error; no server left running | INCONCLUSIVE (route-level denial not exercisable; startup banner is strong partial evidence) |
| p22_cron_run_drop | IRREVERSIBLE_NO_GRANT — destructive payload through `cron_run` (the carrier `mlog approve` names as the oversight target) | **PASSES** ("OK: no issues found") on both backends; audit "clean" | run: silent, exit 0 | **BYPASS** |
| p23_db_drop_tool_method | IRREVERSIBLE_NO_GRANT — DROP via `db.drop()` tool method (pinned-negative name is `db_drop_no_grant`) | **PASSES** on both backends (checker emits no undefined-error for unknown qualified calls) | run refuses — but on call resolution (`compile: qualified call 'db.drop()' — not found`), not on any security gate | INCONCLUSIVE (cannot establish `db.drop` is the destructive-op construct; checker gap recorded in §6) |
| p24_private_url_post | SECRET_EGRESS_NETWORK — env() posted to private loopback URL (pinned negative named `private_url_http_post`) | REFUSED: `[SECRET_LEAK]` + `[PII_EGRESS_NETWORK]` (same classes as p02) | run gate refuses | CAUGHT (class-token mismatch vs sheet, as p02) |

**Totals: 24 probes — 18 CAUGHT, 2 BYPASS, 4 INCONCLUSIVE.**

## 4. Evidence for every BYPASS

There are two bypasses (p06, p22). Both are the same broken promise line — IRREVERSIBLE_NO_GRANT — attacked through two independent carriers authored from the promise wording.

### BYPASS 1 — p06_irreversible_no_grant.mlog

Probe source (verbatim):

```mlog
// p06 — Promise attacked: "What it stops" / IRREVERSIBLE_NO_GRANT — DROP destructive op without the owner grant
pattern Cleanup() -> String {
  let _ = query("DROP TABLE users")
  return "done"
}
```

Observed output (verbatim, from `pocs/p06_transcript.txt`, exit codes cross-verified by direct re-run):

```text
--- CMD: mlog check p06_irreversible_no_grant.mlog
OK: no issues found.
EXIT=0

--- CMD: mlog check --reference p06_irreversible_no_grant.mlog
OK: no issues found.
EXIT=0

--- CMD: mlog audit p06_irreversible_no_grant.mlog
[INFO] line 1: all 1 SQL-carrier calls use literal SQL ✓
Summary: 1 passed
EXIT=0

--- CMD: mlog run p06_irreversible_no_grant.mlog

EXIT=0
```

Why this breaks the promise text: the sheet says "**Irreversible actions without the owner grant** — `DROP`/destructive ops: [`IRREVERSIBLE_NO_GRANT`]… a program in the BLOCKING leak suite that fails `mlog check` on both execution backends", and the header paragraph calls it "a static gate pinned by the BLOCKING leak suite". A literal SQL `DROP TABLE users` — the canonical destructive op, inside the SQL sink the sheet itself names in the SQL_DYNAMIC bullet — fails nothing: check passes on both backends, `audit` actively certifies it, and the run gate waves it through (silently, exit 0 — no runtime grant check fired either). The class token `IRREVERSIBLE_NO_GRANT` never appeared in any output of this pass.

### BYPASS 2 — p22_cron_run_drop.mlog

Probe source (verbatim):

```mlog
// p22 — Promise attacked: "What it stops" / IRREVERSIBLE_NO_GRANT — destructive op carried by the cron_run carrier (the human-oversight roster target per `mlog approve`) without any owner grant
pattern Cleanup() -> String {
  let _ = cron_run("DROP TABLE users")
  return "done"
}
```

Observed output (verbatim, from `pocs/p22_transcript.txt`):

```text
--- CMD: mlog check p22_cron_run_drop.mlog
OK: no issues found.
EXIT=0

--- CMD: mlog check --reference p22_cron_run_drop.mlog
OK: no issues found.
EXIT=0

--- CMD: mlog audit p22_cron_run_drop.mlog
Summary: clean
EXIT=0

--- CMD: mlog run p22_cron_run_drop.mlog

EXIT=0
```

Why this breaks the promise text: the binary's own `mlog approve` help names `carrier:cron_run` as "the human-oversight roster target" — the counted class of the approval gate. A destructive `DROP TABLE users` payload scheduled through that carrier passes check on both backends, passes audit clean, and runs silently with no mandate demanded and no refusal anywhere.

Caveat recorded honestly: in both bypasses nothing destructive actually executed (no database/scheduler backend is wired in the probe runtime; `run` produced no output), and the pinned negative `examples/leak/n18_db_drop_no_grant.error` may use a spelling I could not discover black-box. What is observed, and what the promise text as written does not survive, is: **three independently authored destructive-op shapes (query-DROP, cron_run-DROP, db.drop) produce no IRREVERSIBLE_NO_GRANT static refusal anywhere**, while every other promised refusal class fired exactly as written.

## 5. The inconclusive list

- **p12_degradation_ladder** — blocked by the degradation machinery itself. The №898 refusal text advertises "the in-program local rung `llm { providers: [{ alias: "mock", provider: "local" }] }`" as a configuration path; I configured exactly that (bogus first rung + local rung). Observed: the local rung is never tried — the ladder terminates after rung 1 with typed `[LLM_PROVIDER_UNAVAILABLE]` ("All LLM providers failed. Last error: nonexistent_provider_xyz…"), even with `METALOGOS_MOCK_LLM=1` and even when `model:` is bound to the local rung's alias. A successful fall to a lower rung — the only state in which a `Degraded(t)` event would be observable — was not constructible black-box (the `fallback_if:` line grammar resisted ~15 shapes). What IS observed: the failure is loud and typed (never silent), which honors half of Core Idea 3; the typed `Degraded(t)` event itself is unconfirmed.
- **p19_self_approve_drop** — untestable because the underlying gate does not exist (see p06/p22): `approval_issue("self","db_drop",…)` followed by `query("DROP TABLE users")` passes check identically to plain p06. The promise "a mandate recorded by approval_issue from .mlog is actor_verified=false and NEVER counts" is trivially true here — nothing counts at all. A genuine bypass of that sentence would require the IRREVERSIBLE gate to fire ungranted but pass self-granted, which cannot be arranged when the gate never fires. (`mlog approve` with a host-owned mandate store was not exercised — it would write outside the pass workspace and there is no gate to feed.)
- **p21_serve_env_allowlist** — blocked by grammar, not infrastructure: the `mlogserver` block parses (`port: 8080` accepted; `host`/`middleware`/`rate_limit`/`redact_mode` line kinds exist) but `route_decl` rejected ~50 systematically varied shapes, so no route body calling `env()` could be authored, and the request-time denial could not be exercised. Strong partial evidence was observed: `mlog serve` (timeout 8 s, loopback, died on the parse error, nothing left running) prints the strict-by-default context loudly before any request: `[serve] route exec: denied (set METALOGOS_SERVE_ALLOW_EXEC=1 …)`, `[serve] route env: denied (set METALOGOS_SERVE_ALLOW_ENV=1 …, or METALOGOS_ENV_ALLOWLIST="NAME1,NAME2" …)`, `[serve] LLM UNCONFIGURED — every LLM call will refuse at the first call (№898)`. The banner substantiates Core Idea 1's declarations; the end-to-end route denial remains unverified.
- **p23_db_drop_tool_method** — blocked by construct ambiguity: `db.drop("users")` passes check on both backends (and audit "clean"), but `run` refuses with `compile: qualified call 'db.drop()' — not found` — a resolution failure, not a security refusal. I cannot establish black-box that `db.drop` is the destructive-op construct the pinned negative (`n18_db_drop_no_grant`) uses. The checker's silence on unknown qualified calls (no undefined-error, unlike bare calls) is recorded as a checker gap in §6; classifying it a BYPASS would overstate what I can prove.

## 6. The adversary's notes

**Promises I could not reach from the CLI:**
- `mlog run` execution of ordinary patterns: neither `main` nor `Main` is ever invoked (`run` exits 0 silently); the only execution paths I found are `test "…" { }` blocks (via `mlog test`) and bytecode (`.mbc`). The `flow` construct exists but its `flow_pipeline` grammar rejected ~50 shapes — so `mlog run`'s runtime behavior on normal programs is unreachable black-box. All runtime observations in this pass therefore come from the `test` lane; the VM-vs-TW parity claims are only verified at the check level (`check --reference` produced byte-identical refusals on every refused probe).
- The serve-route context end-to-end (p21 above) and the `Degraded(t)` event (p12 above) are the two promise surfaces blocked by locked grammar (route_decl, flow_pipeline, fallback_if, reflex bodies).
- `mlog approve` (mandate store), `mcp-serve`, `ledger`, `attest`, `resume`, `repl`, `eval` were scoped out: they either need infrastructure the pass cannot provide or are not promise-bearing for the sheet's refusal classes.

**Surprises observed (even where within the promise text):**
1. **The checker has a silent hole for qualified calls.** A bare unknown function is refused loudly (`undefined: function 'X' is not a builtin, pattern, or learnable`), but `db.drop("users")` — an unknown qualified call — passes check on both backends with no undefined-error and no security review (p23). Whatever the intended destructive-op construct is, unknown method calls should not sail through a "strict by default" checker.
2. **Class tokens differ from the sheet in two places.** The SECRET_EGRESS_NETWORK bullet emits `[PII_EGRESS_NETWORK]` (both public and private URL variants); and on composite/cross-pattern paths the env-specific `[SECRET_LEAK]` does not always fire — the generic `[PII_EGRESS_OUTPUT]` clearance refusal does. Sinks are refused either way (no bypass), but a SOC matching on the sheet's exact class names would miss these.
3. **`check` is not side-effect free on stdout**: `[REDACT][audit-event] redact applied in pattern … — policy '<dynamic>'/'secrets', target conf 'public'` lines print during static analysis (p17, p20). Benign, but unexpected from a "semantic analysis without execution" subcommand; `redact(x, "<arbitrary policy string>")` appears to set conf to public for any policy name — a wrong-policy redaction would still clear the sink (untested; flagged as follow-up).
4. **The №898 refusal's own advice does not work.** It advertises the in-program local rung as a config path; configured exactly as written, the local rung never engages (p12). Error-message claim vs. observed behavior mismatch.
5. **Runtime provider calls do real network egress**: during discovery, an `llm { providers: [{ alias: main, provider: openai }] }` ladder made a live HTTPS attempt (403 region-block response quoted verbatim in the refusal). The p12 probe used a bogus provider name that failed at DNS with no egress. An agent program's first LLM call attempts the network by default.
6. **Test blocks run under a typed file-I/O sandbox**: `write_file("/tmp/…")` → `[SANDBOX_VIOLATION] file I/O sandbox: absolute paths not allowed` (relative paths allowed). Not in the promise sheet, but a loud typed denial consistent with Core Idea 1's spirit.
7. **The refusal lattices are flow- and container-sensitive**: labels survive struct fields (`payload.token`), `join()` over lists, `let mut` branch joins, and inter-procedural returns/arguments (p08/p09/p17/p18/p13/p14). The p14 refusal names the entire inter-procedural chain — the №950 closure is genuinely implemented, not just claimed. Sink checks also cover every argument position (secret in respond()'s status argument refused, p16) and cover `test` blocks (p15).
8. **`return env(...)` passes check** — env is a source, and a return value is not a sink; consistent with the promise text, but it means a helper can hand a secret upward and the caller decides (the caller's sink is checked — p13 confirms the lattice catches the composed flow).
9. **`mlog run` on a refusal-free program with no flow block exits 0 having visibly done nothing** — no error, no "no entry point" message. Silent success on a program that cannot do anything is a minor loudness defect adjacent to Core Idea 3's philosophy.
10. **Parity held everywhere I could measure it**: `check --reference` (TW) produced byte-identical refusal output to the VM lane on all 15 refused probes, and identical "OK" on all passing ones — the "fails on both execution backends" wording of the sheet held for every class except the one that never fires (IRREVERSIBLE_NO_GRANT passes identically on both, which is parity of the gap, not of the gate).

**Method note (honesty):** all probes were authored from the promise wording plus shapes suggested by the binary's own help/error text (no repo files read, no network fetched by me; the binary's own provider call in note 5 was its behavior, not my fetch). ~90 additional scratch programs were burned on grammar discovery (flow_pipeline, route_decl, fallback_if, reflex); the final probe set contains only programs that parse. Where the binary's output disagreed with the sheet's vocabulary, both are quoted verbatim in the transcripts.
