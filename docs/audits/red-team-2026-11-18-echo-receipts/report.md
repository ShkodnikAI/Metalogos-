# Red-team pass report — STORE/MEMORY/SESSION builtins and sink-shaped calls

- Pass: independent black-box adversary pass, attack surface "the program's STORE/MEMORY/SESSION builtins and sink-shaped calls" (promise sheet line 9).
- Candidate: `/home/z/my-project/metalogos-work/rt-977/mlog-candidate` — `mlog` CLI (`check`, `run`, `compile`, `audit`, ...).
- Candidate sha256 (sha256sum mlog-candidate): `b4bd6942b0e74ffc798de6ac8b971674b8c375a79b2a43aea94f5992fad7df79`
- Promise sheet: `pocs/PROMISES.md` (verbatim self-description). Promises attacked this pass: **P2** ("sinks check clearance at compile time and in the VM. A secret reaching respond() is a compile error"; sources are tainted: env, user input) and **P4** ("plaintext secrets reaching sinks ... are compile-time refusals").
- Method: observation only. 37 probe programs (p00–p36) written to this directory; each was checked with `mlog-candidate check` on BOTH execution backends (default VM and `--reference`, per P5 "both execution backends") and executed with `SP=mocksecret mlog-candidate run` (mock value; no real credential; no network — the only URL used is the non-routable loopback discard port http://127.0.0.1:9/). Full verbatim output of every command is in `pocs/pNN_*_transcript.txt`; one-line summaries in `pocs/check_results.txt`.

## Executive summary

**37 probes** (30 attack probes + 7 controls). **BYPASS: 0. CAUGHT: 30. INCONCLUSIVE: 0.**

No bypass was found. Every attack shape the store/memory/session surface offered was refused at compile time on both execution backends, and every `run` of a refused program also refused (exit 1, "Category A security invariant violated") — check and run never disagreed in the dangerous direction (a refusal that passes at runtime was not observed anywhere).

Two honest negative observations were recorded (neither is a bypass; neither shipped a leak):

1. **Read-back asymmetry (p08, p09, p11, p12, p15, p34, p35):** when a secret is round-tripped through a store builtin and the *read-back* (or in two cases the store call's own return value under `kv_set`/`memorize`) is passed to `respond()` inside a pattern, the `respond` site itself is often NOT flagged — only the store call is. The program still fails `check` (exit 1) because storing a secret is itself a compile-time refusal (`TAINT_PERSISTENCE` / `PII_EGRESS_NETWORK`), so the promise holds; but the observed evidence shows the guarantee on this surface rests on refusing the store call, not on tracking read-back taint at the respond site. By contrast, read-back taint IS tracked when it flows through a pattern return to the flow terminator (p13, p17) — the tracking is not uniformly absent.
2. **Check/run verdict divergence on quarantine_write (p23):** `check` accepts `quarantine_write(env("SP"), reason)` with exit 0 as "legal egress with audit event (№335)"; `run` then DENIES the same call at runtime (`SINK_CLEARANCE_RUNTIME`, deny class `PII_EGRESS_NETWORK`, exit 1). The runtime refusal is the safe direction (no leak ships), but compile-time and runtime reached different verdicts on the same call — recorded verbatim in `pocs/p23_quarantine_secret_transcript.txt`.

## Probe table

| Probe | Promise attacked | check (VM / --reference) | run (SP=mocksecret) | Verdict |
|---|---|---|---|---|
| p00 sanity (no sources) | — (baseline) | OK / OK | exit 0 | CONTROL |
| p01 direct `respond(env("SP"))` | P2: secret→respond() must be compile error | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A refusal, exit 1 | CAUGHT |
| p02 direct `respond(query_param("q"))` | P2: untrusted→public sink refused | HTML_INJECTION ×2 (untrusted-egress), exit 1 / identical | Category A refusal, exit 1 | CAUGHT |
| p03 `mem_set` literal, respond return | control: store-return path not blanket-denied | OK / OK | exit 0 | CONTROL |
| p04 bare assignment | — (syntax control) | immutability error, exit 1 | UNDEFINED_VARIABLE, exit 1 | CONTROL |
| p05 `env()` used, no sink | control: source use alone is legal | OK / OK | exit 0 | CONTROL |
| p06 `query_param()` used, no sink | control: source use alone is legal | OK / OK | exit 0 | CONTROL |
| p07 `mem_set` return → respond | P2/P4: store roundtrip | TAINT_PERSISTENCE (mem_set arg1) + SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p08 `mem_set` → `kv_get` read-back → respond | P2/P4: write-then-read-back | TAINT_PERSISTENCE + flow-print PII_EGRESS_OUTPUT, exit 1; respond site NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p09 `kv_set` → `kv_get` read-back → respond | P2/P4: write-then-read-back | TAINT_PERSISTENCE only, exit 1; respond + terminator NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p10 `session_set` return → respond | P2/P4: session store roundtrip | SECRET_LEAK + PII_EGRESS_NETWORK (session_set arg2) + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p11 `memorize` → `recall` read-back → respond | P2/P4: memorize roundtrip | TAINT_PERSISTENCE only, exit 1; respond site NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p12 `mem_set` → `mem_delete` return → respond | P2/P4: mem_delete return echo | TAINT_PERSISTENCE + flow-print PII_EGRESS_OUTPUT, exit 1; respond site NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p13 store in helper, read-back across flow edge | P2/P4: helper + flow edge | TAINT_PERSISTENCE (Stash) + terminator PII_EGRESS_OUTPUT (read-back tracked), exit 1 / identical | Category A, exit 1 | CAUGHT |
| p14 `query_param` → `mem_set` → respond | P2: untrusted store roundtrip | TAINT_PERSISTENCE (untrusted-egress) + HTML_INJECTION ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p15 `mem_set` → `recall` cross-store read-back | P2/P4: cross-store read-back | TAINT_PERSISTENCE + terminator, exit 1; respond site NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p16 `mem_set(42, secret)` numeric key | P2: typed/numeric key variant | SECRET_LEAK + TAINT_PERSISTENCE + PII_EGRESS_OUTPUT ×2, exit 1 (numeric key accepted, same refusal) / identical | Category A, exit 1 | CAUGHT |
| p17 store return through flow edge (`-> Store -> output`) | P2/P4: flow-edge variant | TAINT_PERSISTENCE + terminator PII_EGRESS_OUTPUT, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p18 `let t = env("SP"); respond(t)` | P2: variable passthrough laundering | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p19 `let mut x = "safe"; x = env("SP"); mem_set("k", x)` | P2/P4: reassignment laundering | SECRET_LEAK + TAINT_PERSISTENCE + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p20 `"prefix-" + env("SP")` → `mem_set` → respond | P2/P4: concat laundering | SECRET_LEAK + TAINT_PERSISTENCE + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p21 flow input literal `env("SP")` → P → output | P2: flow-input taint | terminator PII_EGRESS_OUTPUT, exit 1 (no SECRET_LEAK class) / identical | Category A, exit 1 | CAUGHT |
| p22 `let v = Helper("x")` (helper returns env) → respond | P2: inter-pattern call laundering | PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p23 `quarantine_write(env("SP"), reason)` | P2: quarantine sink exception | exit 0, "[CONSENT][audit-event] poisoned value reaches the quarantine sink ... legal egress with audit event (№335)" / identical | SINK_CLEARANCE_RUNTIME deny (PII_EGRESS_NETWORK), exit 1 | CAUGHT (note 2) |
| p24 `write_file(path, env("SP"))` | P4: secret to file sink | SECRET_LEAK ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p25 `http_post("http://127.0.0.1:9/", env("SP"))` | P4: secret to network sink | SECRET_LEAK + PII_EGRESS_NETWORK + terminator, exit 1 / identical | Category A, exit 1 (no network attempted — refused before execution) | CAUGHT |
| p26 `print(env("SP"))` | P2: print sink | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p27 `redact(env("SP"),"all")` → respond | control: sanctioned downward move | OK with [REDACT][audit-event], exit 0 / identical | exit 0 | CONTROL |
| p28 `env("DEFINITELY_NOT_SET_RT_977")` → respond | P2: blanket env taint | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p29 orphan (unwired) pattern with `respond(env)` | P2: dead-code escape | SECRET_LEAK + PII_EGRESS_OUTPUT on the orphan pattern, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p30 `mem_set("k", Helper("x"))` (helper returns env) | P2/P4: call-result laundering into store | TAINT_PERSISTENCE + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p31 `if` branch containing `respond(env)` | P2: conditional laundering | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p32 `query_param` → `output<public, untrusted>` | attack-surface framing: untrusted→public channel | OK, exit 0 / identical | exit 0 | CONTROL (documented escape hatch, see "Controls" below) |
| p33 `respond(type_of(env("SP")))` | P2: metadata laundering | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |
| p34 `kv_set` return → respond | P2/P4: kv_set return echo | TAINT_PERSISTENCE only, exit 1; respond site NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p35 `memorize` return → respond | P2/P4: memorize return echo | TAINT_PERSISTENCE only, exit 1; respond site NOT flagged / identical | Category A, exit 1 | CAUGHT (note 1) |
| p36 `while` branch containing `respond(env)` | P2: loop laundering | SECRET_LEAK + PII_EGRESS_OUTPUT ×2, exit 1 / identical | Category A, exit 1 | CAUGHT |

## BYPASS evidence

**There are no BYPASS probes.** No program that the promise sheet requires to be refused passed `check` on either backend. The statement is recorded plainly per the pass contract: zero bypasses on this attack surface in this pass.

## Recorded observations (verbatim classes, no bypass claimed)

### Note 1 — read-back asymmetry

In p08/p09/p11/p12/p15/p34/p35 the `respond()` call that consumes a store read-back (`kv_get("k")`, `recall("k")`, `mem_delete` return, or the `kv_set`/`memorize` call's own return value) is NOT flagged, while in the mirror-image probes p07/p10/p14 (and p16/p18–p20/p30 via variables/concat/call results) the `respond()` call IS flagged. In all seven note-1 probes the program still fails `check` with exit 1 because the store call itself is refused:

- verbatim (p09, both backends): `[TAINT_PERSISTENCE] sink clearance violated: argument 1 of kv_set in pattern P carries label 'private, trusted'; sinks require public — kv_set is classified sink/internal/reversible (the builtins classification, №316-class); the bridge (№391) table covers the six ACTION sinks only; failed threshold: private-egress` — and nothing else; `respond(kv_get("k"))` passed silently.
- verbatim (p13, for contrast): the read-back value flowing through a pattern return to the flow terminator IS flagged: `... argument 0 of print in flow Main carries label 'private, trusted' ...`.

Consequently: the observed enforcement of the store roundtrip rests entirely on refusing the store call; the read-back→respond edge is not uniformly guarded. No promise text is broken by this (the compile fails either way, exit 1, both backends), so it is recorded as an observation, not a bypass. The asymmetry's cause was not investigated (source tree off-limits per contract).

### Note 2 — check/run divergence on quarantine_write (p23)

Verbatim check output (exit 0, both backends): `[CONSENT][audit-event] poisoned value reaches the quarantine sink in pattern 'P' — legal egress with audit event (№335)` followed by `OK: no issues found.`
Verbatim run output (exit 1): `[SINK_CLEARANCE_RUNTIME] sink clearance violated at runtime: quarantine_write argument '@env' carries label 'private, trusted' (line 1) — the static gate and the runtime agree on the verdict; deny reason class: PII_EGRESS_NETWORK`.
The compile-time layer classifies quarantine as legal egress; the runtime layer denies the same call. Direction is safe (a program clean under `check` cannot execute its quarantine call), but the two layers issued different verdicts on the same program, and the runtime message's own claim ("the static gate and the runtime agree on the verdict") does not match the observed exit codes. Recorded as CAUGHT-with-note; not a bypass (no output channel received the value).

### Controls (7 probes — designed behavior or sanity, excluded from the attack counts)

- p00/p05/p06: clean programs and bare source use pass; the checker is not blanket-hostile.
- p03: storing a literal and responding the store call's return passes — the store-return path is label-driven, not denied per se.
- p04: syntax-only refusal (immutability), consistent between check and run.
- p27: `redact(env("SP"), "all")` → `respond` passes both layers with a REDACT audit event — the tool's sanctioned downward move works as its own refusal messages advertise (№967/№326 pointers).
- p32: the untrusted→terminator flow with the explicit `-> output<public, untrusted>` annotation passes both layers. This annotation form is prescribed by the tool's own refusal text in p02 ("declare `-> output<public, untrusted>` if the display is intended (ADR-0154 §13.4 form (а))"). The undeclared form (p02) is refused as promised. Because the promise sheet is silent on whether the declared form should be legal and the tool explicitly sanctions it in its own enforcement output, this is classified CONTROL (designed, visible-in-source declaration), not a bypass.

## Comparison demanded by the task: direct value vs store-roundtrip verdict

- Direct secret → respond (p01): refusal classes `SECRET_LEAK` + `PII_EGRESS_OUTPUT` (respond) + `PII_EGRESS_OUTPUT` (flow print), exit 1.
- Store roundtrip → respond (p07): the SAME classes plus `TAINT_PERSISTENCE` at the store call, exit 1. The roundtrip adds a refusal; it never removes one.
- `kv_set`/`memorize` roundtrips (p09, p11, p34, p35): refusal class reduces to `TAINT_PERSISTENCE` at the store call alone (the respond site is not flagged — note 1). The difference in flagged classes between direct and roundtrip forms is real and verbatim-recorded, but the compile-time refusal itself never disappears: exit 1 on both backends in every store-roundtrip probe.

## Scope and limits

- Black-box only: no repository sources were read or listed; knowledge came from PROMISES.md, `--help` output, and observed refusals. No speculation about internals is included.
- Runtime probes used `SP=mocksecret` only; the sole URL probed is the non-routable `http://127.0.0.1:9/` (p25 was refused at check time and never attempted a connection).
- Not covered by this pass: REPL persistent-state vectors, bytecode (.mbc) tampering, the `serve`/MCP surfaces, LLM-output and SQL-injection promise classes, authorization-grant promises (irreversible-action gates). These are separate surfaces.
- All 37 transcripts with full verbatim command output (check VM, check --reference, run) are in `pocs/pNN_*_transcript.txt`; per-probe one-liners in `pocs/check_results.txt`.
- Supplementary corroboration (not part of the promise surface): the separate `mlog audit` subcommand (ADR-0057) independently flagged p01 with the same 3 findings and p07 with the same 4 findings as `check` (classes `SECRET_LEAK`, `PII_EGRESS_OUTPUT`, `TAINT_PERSISTENCE`), exit 1. Recorded for completeness; the promises under attack cite `mlog check`.

candidate: mlog-candidate sha256 b4bd6942b0e74ffc798de6ac8b971674b8c375a79b2a43aea94f5992fad7df79
