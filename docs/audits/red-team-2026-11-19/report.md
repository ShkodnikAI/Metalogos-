# Red-Team Pass Report — Metalogos `mlog` v0.40.0 (black-box adversary)

- **Date:** 2026-11-19
- **Binary:** `/tmp/adversary-980/mlog`, version string `mlog mlog 0.40.0`
- **SHA256:** `34f85430e533efa88fcc9946fdc97901b5bca530a56f5aaa2202e37b597a176f` (verified against the promise sheet before probing)
- **Posture:** independent black-box adversary. The only inputs used were the binary and the promise sheet (`PROMISES.md`, copied verbatim in `pocs/`). No source code, tests, ADRs, examples, issue trackers or change history were opened; no `git` commands were run. Probes were written from the syntax sample on the sheet plus a syntax-reconnaissance phase (parse-error driven, ~100 throwaway programs). All network targets are non-routable placeholders (`127.0.0.1:9`, `example.invalid`).
- **Probe count:** 53 programs (`p01`–`p53`), each with a raw `check`/`run` transcript in `pocs/pNN_*.out.txt`.
- **Verdicts:** **9 BYPASS** (all one root cause) · **44 CAUGHT** · **0 INCONCLUSIVE**.

## Executive summary

The static taint machinery is genuinely strong: across the 44 CAUGHT probes — attacks on tracking through concatenation, structs, lists, `each`/`while` loops, `if/else` merges, user-pattern call chains, struct-field round trips and flow edges, plus the gate and control probes — every tracking attack is refused at `check` time with the correct refusal class (`SECRET_LEAK`, `PII_EGRESS_*`, `SECRET_TO_EXEC`, `UNTRUSTED_EXEC_DECISION`, `SQL_DYNAMIC`, `HTML_INJECTION`, `TAINT_INTERP`, `TAINT_PERSISTENCE`, `UNTRUSTED_EGRESS_NETWORK`). The approval gate (promise 4), capability layer (promise 5) and serve/MCP strict-by-default gates (promise 6) all held, including the "in-program `approval_issue` never counts" rule and the host-owned mandate store.

The one thing that breaks promise 1 completely is `redact()`: called on a **secret** (an `env()` value), it re-labels the value `private → public` while **not changing the content** whenever the value does not match the policy's recognition patterns. `redact(s, "secrets")` and even `redact(s, "all")` leave the raw secret byte-for-byte intact, clear the sink-clearance label, and every downstream sink — `print`, `write_file`, `exec`, `http_post` — passes `mlog check`. Run then writes the raw secret to disk (`p49`), runs it through a shell (`p50`), prints it (`p10`, `p43`) and pushes it into the HTTP-request pipeline (`p52`, `p53`, where only the address-based SSRF guard / DNS failure stop the request, not any label gate). Notably, the same `redact()` does **not** clear the *untrusted* half of a label (p29), so the sanitizer downgrade is mis-scoped: it declassifies confidentiality on arbitrary content but the promise-sheet sanction for `redact` was only ever meant as a masking step.

Secondary observations (not bypasses, reported for completeness): a static/runtime skew exists around `render()`-wrapped untrusted data (check accepts, runtime label check refuses — safe direction, p44); the flow terminator accepts a self-declared `-> output<public, untrusted>` annotation that silences the terminator finding (p31; `print` inside the pattern is still caught); the `match`-as-`let` construct parses in `check` but the VM cannot execute it (`[UNDEFINED_VARIABLE] undefined variable: match` — p09); `mlog check` emits its `redact` audit events to stderr even in static mode; the `mlogserver` route-body grammar could not be reverse-engineered black-box in this session (1300+ shape attempts), so the route-handler gates were exercised through the equivalent MCP tool-method context, which the refusal texts name as the same enforcement context (`Naryad #259`, `Naryad #457`).

**Bottom line:** one root-cause vulnerability (`redact` = declassify-without-mask) defeats the compile-time half of promise 1 for every sink class. Fix direction: `redact` must be a masking function (return content that provably does not contain the input) before any label downgrade, or label downgrades must be limited to policies whose mask provably covers the input (e.g. `hash_only`).

## Probe table

| Probe | Promise attacked | check verdict | run verdict | Final |
|---|---|---|---|---|
| p01_env_print | 1 secret→print | refuse (SECRET_LEAK, PII_EGRESS_OUTPUT) | refuse | CAUGHT |
| p02_env_http_post_body | 1 secret→http_post body | refuse (SECRET_LEAK, PII_EGRESS_NETWORK) | refuse | CAUGHT |
| p03_env_concat_websearch | 1 secret concat→web_search | refuse (PII_EGRESS_NETWORK) | refuse | CAUGHT |
| p04_env_pattern_indirect_sendmsg | 1 secret via pattern call→send_message | refuse (PII_EGRESS_NETWORK) | refuse | CAUGHT |
| p05_env_struct_field_writefile | 1 secret via struct field→write_file | refuse (SECRET_LEAK) | refuse | CAUGHT |
| p06_env_list_each_print | 1 secret via list+each→print | refuse (PII_EGRESS_OUTPUT) | refuse | CAUGHT |
| p07_env_while_mut_print | 1 secret via while loop→print | refuse (SECRET_LEAK, PII_EGRESS_OUTPUT) | refuse | CAUGHT |
| p08_env_ifelse_branch_print | 1 secret via if/else merge→print | refuse (PII_EGRESS_OUTPUT) | refuse | CAUGHT |
| p09_env_match_let | 1 secret via match-as-value→print | refuse (tracked through match arms) | refuse at label; VM cannot execute match (`UNDEFINED_VARIABLE`) | CAUGHT (skew noted) |
| p10_env_redact_print | 1 secret→redact(secrets)→print | **PASS** | **prints raw secret** | **BYPASS** |
| p11_env_escape_html_print | 1 secret→escape_html→print | refuse (SECRET_LEAK, PII_EGRESS_OUTPUT) | refuse | CAUGHT |
| p12_env_exec | 1 secret→exec | refuse (SECRET_TO_EXEC) | refuse | CAUGHT |
| p13_env_kv_roundtrip_print | 1 secret→kv_set (memory sink) roundtrip | refuse (TAINT_PERSISTENCE at kv_set) | refuse | CAUGHT |
| p14_env_writefile_content | 1 secret→write_file content arg | refuse (SECRET_LEAK) | refuse | CAUGHT |
| p15_env_writefile_path | 1 secret→write_file path arg | refuse (SECRET_LEAK) | refuse | CAUGHT |
| p16_sql_qparam_concat | 2 query_param concat→db_execute | refuse (SQL_DYNAMIC) | refuse | CAUGHT |
| p17_sql_env_concat | 2 secret concat→db_execute | refuse (SQL_DYNAMIC + SINK_CLEARANCE) | refuse | CAUGHT |
| p18_sql_struct_field | 2 untrusted via struct→db_execute | refuse (SQL_DYNAMIC) | refuse | CAUGHT |
| p19_sql_literal_control | 2 control: literal SQL | PASS | runs (errs only on missing db block) | CAUGHT (control) |
| p20_qparam_exec | 3 untrusted→exec | refuse (UNTRUSTED_EXEC_DECISION) | refuse | CAUGHT |
| p21_llm_respond_html | 3 call_llm→respond_html | refuse (HTML_INJECTION + TAINT_INTERP) | refuse | CAUGHT |
| p22_readfile_respond_html | 3 read_file→respond_html | refuse (HTML_INJECTION) | refuse | CAUGHT |
| p23_qparam_escape_respond_html | 3 control: escape_html sanctioned path | PASS | ok | CAUGHT (control) |
| p24_qparam_render_template | 3 control: render() sanctioned path | PASS | ok | CAUGHT (control) |
| p25_formdata_send_message | 3 form_data→send_message | refuse (UNTRUSTED_EGRESS_NETWORK) | refuse | CAUGHT |
| p26_struct_exec_argv | 3 untrusted via struct→exec_argv | refuse (UNTRUSTED_EXEC_DECISION) | refuse | CAUGHT |
| p27_jsonbody_exec_argv | 3 json_body→exec_argv | refuse (UNTRUSTED_EXEC_DECISION) | refuse | CAUGHT |
| p28_qparam_ifelse_exec | 3 untrusted via if/else merge→exec | refuse (UNTRUSTED_EXEC_DECISION) | refuse | CAUGHT |
| p29_qparam_redact_print | 3 untrusted→redact(pii)→print | refuse (untrusted half survives redact) | refuse | CAUGHT |
| p30_untrusted_two_patterns_exec | 3 untrusted via 2-level pattern chain→exec | refuse (UNTRUSTED_EXEC_DECISION) | refuse | CAUGHT |
| p31_flow_output_label_declare | 3 declare `output<public, untrusted>` attack | refuse at print (terminator finding silenced only) | refuse | CAUGHT |
| p32_httpget_respond_html | 3 http_get body→respond_html | refuse (HTML_INJECTION) | refuse | CAUGHT |
| p33_cron_no_approval | 4 cron_run, no mandate | PASS | IRREVERSIBLE_NO_GRANT before action | CAUGHT |
| p34_cron_self_approval | 4 cron_run + in-program approval_issue | PASS | IRREVERSIBLE_NO_GRANT (actor_verified=false never counts) | CAUGHT |
| p35_cron_external_approval | 4 cron_run + host-CLI mandate | PASS | executes (exit 0) | CAUGHT (gate admits verified mandate) |
| p36_gitpush_nogrant | 5 git_push, no grant | PASS | IRREVERSIBLE_NO_GRANT before action | CAUGHT |
| p37_gitpush_profile_permit | 5 git_push + profile capability permit | PASS | executes | CAUGHT (permit works) |
| p38_exec_run_gate | 5 exec without/with METALOGOS_ALLOW_EXEC | PASS | refuse / executes | CAUGHT |
| p39_serve_noblock | 6 serve without mlogserver | PASS | `error: no mlogserver block found in source` | CAUGHT |
| p40_mcp_env_tool | 6 MCP tool method env() | PASS | ENV_NOT_PERMITTED at tools/call | CAUGHT |
| p41_mcp_exec_tool | 6 MCP tool method exec | PASS | IRREVERSIBLE_NO_GRANT at tools/call | CAUGHT |
| p42_sensitive_paths_run | 6 sensitive path (`.env`) read | PASS | SANDBOX_SENSITIVE_PATH | CAUGHT |
| p43_env_redact_all_print | 1 secret→redact(**all**)→print | **PASS** | **prints raw secret** | **BYPASS** |
| p44_render_untrusted_passthrough | 3 render-wrapped untrusted→print | PASS | runtime SINK_CLEARANCE refuses | CAUGHT (static/runtime skew, safe direction) |
| p45_escape_html_runtime_control | 3 control: escape_html really escapes | PASS | prints escaped HTML | CAUGHT (control) |
| p46_env_redact_exec | 1 secret→redact→exec | **PASS** (promise demands compile-time refusal) | capability gate stops run | **BYPASS** (compile-time) |
| p47_env_redact_httppost | 1 secret→redact→http_post | **PASS** (promise demands compile-time refusal) | capability gate stops run | **BYPASS** (compile-time) |
| p48_env_redact_writefile | 1 secret→redact→write_file (abs path) | **PASS** (promise demands compile-time refusal) | sandbox path rule stops run | **BYPASS** (compile-time) |
| p49_env_redact_writefile_relative | 1 secret→redact→write_file (relative path) | **PASS** | **executes; raw secret on disk** | **BYPASS** (end-to-end) |
| p50_env_redact_exec_allowed | 1 secret→redact→exec with permit | **PASS** | **executes; secret in command line** | **BYPASS** (end-to-end) |
| p51_serve_minimal | 6 control: serve starts, /health | PASS (1 warning) | serve listens; `mlog health` exit 0 | CAUGHT (control) |
| p52_env_redact_httppost_permit | 1 secret→redact→http_post + permit, loopback URL | **PASS** | SSRF guard refuses (address-based, not label-based) | **BYPASS** (compile-time) |
| p53_env_redact_httppost_public | 1 secret→redact→http_post + permit, `example.invalid` | **PASS** | reached DNS with secret in body; DNS fail | **BYPASS** (compile-time; no label refusal remains) |

## BYPASS evidence (verbatim)

### Root cause

`redact(<secret>, <policy>)` downgrades the confidentiality label (`private → public`, see the audit event `redact applied in pattern P — policy 'secrets', target conf 'public'`) **without masking content that the policy's patterns do not recognize**. After that call the compiler sees a `public, trusted` value, so promise 1's compile-time gate has nothing left to refuse, for every sink class.

### BYPASS 1 — p10_env_redact_print (secret → stdout)

```mlog
pattern P(d: String) -> String {
  let s = env("SP")
  let r = redact(s, "secrets")
  print(r)
  return d
}
flow Main { input: String = "x" -> P -> output }
```

check (exit 0):

```
[REDACT][audit-event] redact applied in pattern P — policy 'secrets', target conf 'public'
OK: no issues found.
```

run (`SP=s3cr3t-t0ken-980`), exit 0 — the secret reaches stdout verbatim:

```
[REDACT][audit-event] redact applied in pattern P — policy 'secrets', target conf 'public'
[print] s3cr3t-t0ken-980
x
```

### BYPASS 2 — p43_env_redact_all_print (policy `all`, the strongest, same result)

Same shape with `redact(s, "all")`. check: `OK: no issues found.` run: `[print] s3cr3t-t0ken-980` (raw). Full transcript: `pocs/p43_env_redact_all_print.out.txt`.

### BYPASS 3 — p49_env_redact_writefile_relative (secret written to disk, end-to-end)

```mlog
pattern P(d: String) -> String {
  let s = env("SP")
  let r = redact(s, "secrets")
  let w = write_file("leaked-out.txt", r)
  return d
}
flow Main { input: String = "x" -> P -> output }
```

check (exit 0): `OK: no issues found.` — run (exit 0) leaves the raw secret on disk:

```
##### EVIDENCE: leaked file content after RUN from cwd /tmp/adversary-980/work
--- cat leaked-out.txt:
s3cr3t-t0ken-980
```

### BYPASS 4 — p50_env_redact_exec_allowed (secret into a shell command, end-to-end)

Same as p46 plus the operator-side permit (`METALOGOS_ALLOW_EXEC=1`). check: `OK: no issues found.` run (exit 0): the process executes with the secret embedded in the command string. Full transcript: `pocs/p50_env_redact_exec_allowed.out.txt`.

### BYPASS 5–7 — p46/p47/p48 (check-level refusal absent for exec / http_post / write_file)

`env → redact(s,"secrets") → {exec, http_post, write_file}` all print `OK: no issues found.` at check time, while promise 1 demands a compile-time refusal for each of these sink classes. At run time they are stopped by *different, non-label* layers (capability gate / capability gate / absolute-path sandbox rule) — i.e. with the ordinary operator permits (`profile capability { allow_carrier: ... }`, relative paths) nothing stops them. Transcripts: `pocs/p46_env_redact_exec.out.txt`, `pocs/p47_env_redact_httppost.out.txt`, `pocs/p48_env_redact_writefile.out.txt`.

### BYPASS 8–9 — p52/p53 (secret enters the HTTP request pipeline)

With `profile capability { allow_carrier: "http_post" }` and the body carrying `redact(env("SP"), "secrets")`, check passes and run reaches the network layer with the secret in the POST body:

- p52 (loopback URL): stopped by the **address-based SSRF guard** (`URL 'http://127.0.0.1:9/hook' resolves to 127.0.0.1 ...`), not by any label gate.
- p53 (`http://example.invalid/hook`): the request pipeline proceeds to **DNS resolution carrying the secret**; only the DNS failure of the placeholder stops it (`DNS resolution failed for 'example.invalid'`). No label refusal exists at any point.

## What held (selected CAUGHT evidence)

- **Tracking machinery:** every structural carrier is statically tracked — concatenation through intermediates (p03), user-pattern returns (p04), struct fields (p05, p18, p26), lists + `each` (p06), `while` loops with mutation (p07), `if/else` merges (p08, p28), match arms (p09), multi-level pattern chains (p30), `kv_set` storage (p13, TAINT_PERSISTENCE), both argument positions of `write_file` (p14/p15). Refusals come with the correct class names and label algebra (`private, trusted` / `public, untrusted`).
- **SQL:** `SQL_DYNAMIC` fires at compile time on any non-literal `db_execute` SQL — including struct-field concatenation (p16–p18); the literal control passes (p19).
- **Approval gate:** `cron_run` without a live mandate refuses with `IRREVERSIBLE_NO_GRANT` before the action (p33); an in-program `approval_issue` does not satisfy it (p34); only the host-owned `mlog approve` CLI mandate (actor_verified=true, persisted in the store JSON) lets it execute (p35).
- **Capability layer:** `git_push` refuses without a grant (p36); `profile capability { allow_carrier: "git_push" }` authorizes it (p37); `exec` obeys `METALOGOS_ALLOW_EXEC` (p38).
- **Serve/MCP:** serve announces strict-by-default context and denies route `exec`/`env` by default (p39, p51 banners); MCP tool methods refuse `env()` with `ENV_NOT_PERMITTED` (p40) and `exec` with `IRREVERSIBLE_NO_GRANT` (p41) at `tools/call`; sensitive paths (`.env`) refuse even in plain `run` (p42); `redact` does **not** clear the untrusted half of a label (p29) and the runtime label layer catches what the static layer misses in the `render` case (p44).

## Session limitations (honest disclosure)

1. **`mlogserver` route-body grammar not cracked.** Over 1200 candidate shapes for `route_decl` (combinatorial search over keyword/string/separator/tail forms, guided by the pest expectation lists) all failed at the same parser position; no route body could therefore be written in this session. The route-handler gates of promise 6 were instead exercised through **MCP tool methods** (p40/p41), which the binary's own refusal texts place in the same enforcement context ("denied in serve route handlers and MCP tool methods", Naryad #259). The route-body path remains untested by this pass.
2. **`match` statement/executable skew.** `check` parses and statically tracks `let x = match s { a : v, _ : v2 }` (p09 shows the label flowing through match arms), but the VM refuses to execute the construct (`[UNDEFINED_VARIABLE] undefined variable: match`). Runtime behavior of match-carried taint could not be observed; the static half is CAUGHT.
3. **No network:** all network probes used the sheet-mandated placeholders; the farthest any secret-bearing request traveled was to DNS resolution (p53).
4. No repo material outside this report directory was read; the only information source beyond the sheet was `strings`/error-message output of the binary itself (permitted: the binary is a legitimate black-box input).

## Final verdict list

1. **p10_env_redact_print** (promise 1) — `env()` secret wrapped in `redact(s, "secrets")`: check passes, run prints the raw secret to stdout.
2. **p43_env_redact_all_print** (promise 1) — same with policy `all`: check passes, run prints the raw secret.
3. **p49_env_redact_writefile_relative** (promise 1) — secret + `redact` into a relative-path `write_file`: check passes, run writes the raw secret to `leaked-out.txt` on disk.
4. **p50_env_redact_exec_allowed** (promise 1) — secret + `redact` into `exec` with the standard operator permit: check passes, run executes the command containing the secret.
5. **p46_env_redact_exec** (promise 1) — `env` + `redact` + `exec`: check passes where the promise demands compile-time refusal (run stopped only by the independent capability gate).
6. **p47_env_redact_httppost** (promise 1) — `env` + `redact` + `http_post` body: check passes where the promise demands compile-time refusal (run stopped only by the capability gate).
7. **p48_env_redact_writefile** (promise 1) — `env` + `redact` + `write_file`: check passes where the promise demands compile-time refusal (run stopped only by the absolute-path sandbox rule).
8. **p52_env_redact_httppost_permit** (promise 1) — secret + `redact` + `http_post` with a capability permit: check passes; the request pipeline runs and is stopped only by the address-based SSRF guard, not any label gate.
9. **p53_env_redact_httppost_public** (promise 1) — same against the `example.invalid` placeholder: check passes; the secret-bearing request proceeds to DNS resolution; no label refusal remains anywhere.

All nine share one root cause: `redact()` declassifies (label `private → public`) without masking content that its policy patterns do not recognize, so any secret source is laundered into a `public, trusted` value with one call. No bypass was found against promises 2–6; all 44 non-bypass probes are CAUGHT (details and raw transcripts in `pocs/`).
