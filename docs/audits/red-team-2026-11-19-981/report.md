# Red-team pass report — Metalogos `mlog` v0.40.0, candidate `mlog981`

- **Date:** 2026-11-19
- **Candidate binary:** `mlog981`, SHA256 `64f5a0d7d6ee662ab64ac291a9158cd9418b728442cd910ec6e523312d62a224`
- **Report type:** independent black-box adversarial pass (no source, no tests, no docs, no repo history consulted)
- **Scratch/transcripts:** this directory's `pocs/` (copy of `/tmp/adversary-980/pocs981/`)

## Executive summary

**45 distinct numbered probes were executed (q01–q45; q40 is a run-only variant of q01's program and has no separate source file — 44 `.mlog` sources total). Verdicts: 1 BYPASS, 41 CAUGHT, 3 INCONCLUSIVE.** All six promises were attacked. The compile-time secret-leak gates (Promise 1), the SQL_DYNAMIC gate (Promise 2), the untrusted-taint end-to-end tracking incl. every laundering channel I could construct — memory/kv/session stores, file write→read round trips, sink return values, pattern-call boundaries, loops (Promise 3), the cron approval gate (Promise 4), the capability layer (Promise 5) and the serve strict-mode gates and sensitive-path deny vocabulary (Promise 6) — all held under attack, several of them fail-closed in both directions. The single BYPASS is **q26**: the flow terminator accepts an explicit per-egress declaration `output<public, untrusted>` which lets **unsanitized untrusted data reach a public output with no `escape_html`/`render`/`redact` wrapper**, with `check` passing (exit 0) and `run` printing the raw untrusted content to stdout.

**Adversary posture.** I did not write, compile, or previously see this program; I had only the binary and the promise sheet. I did not read `src/`, `tests/`, `docs/`, `examples/`, git history, or the earlier pass's `pocs/` artifacts; the earlier binary `mlog` was present in the scratch directory and was deliberately not used. All probes used non-routable endpoints (127.0.0.1, example.invalid) and never depended on network access. Where the sheet sanctioned permits (`METALOGOS_ALLOW_EXEC`, `METALOGOS_STATIC_LABEL_GATE_OFF`, serve-mode allows), I used them as labeled attack levers, and I report each use.

**Difference from the earlier same-day pass:** the candidate I was given at `/tmp/adversary-980/mlog981` did not exist in the sandbox; the current-candidate binary matching this pass's naming and timestamp is `/tmp/mlog_981` (2026-11-19 15:10), which I staged at the promised path and used exclusively. Its SHA256 (`64f5a0d7…`) differs from the older binary present in the scratch dir (`/tmp/adversary-980/mlog` = `34f85430…`, identical to `/tmp/mlog_980`), confirming this pass ran against a different build than the earlier pass. I did not inspect the earlier pass's findings or transcripts, so no statement about behavioral differences is made beyond the hash.

## Method

- Recon phase (t01–t74 files in `pocs/`): ~25 additional syntax/signature probes used to learn the language (flows are pipelines `input: T = expr -> Stage -> output`; logic lives in `pattern` functions; label model is a pair `(confidentiality, trust)` — `env()` yields `private, trusted`; `query_param`/`read_file` of ordinary files/HTTP responses/LLM output yield `public, untrusted`; literals are `public, trusted`). Recon artifacts are in `pocs/` and are not counted in the verdict table.
- Every counted probe is a self-contained `.mlog` program checked with `mlog981 check` and, where the promise is a runtime gate or execution evidence matters, executed with `mlog981 run` (10 s timeout; mandate store redirected into the scratch dir via `METALOGOS_MANDATE_STORE`).
- `PARSE_ERROR` shapes (struct/`type` declarations, `match` arms, the `mlogserver` route entry) were recorded as INCONCLUSIVE per the ground rules, not as verdicts.

## Full probe table

| Probe | Promise attacked | check verdict | run verdict | Classification | Evidence (one line) |
|---|---|---|---|---|---|
| q01_secret_print | 1 | refused exit 1 | n/a (compile gate) | CAUGHT | `[SECRET_LEAK] env() value passed to print()` + `[PII_EGRESS_OUTPUT]` private-egress |
| q02_secret_httppost | 1 | refused | n/a | CAUGHT | `[SECRET_LEAK] …as http_post body` + `[PII_EGRESS_NETWORK]` |
| q03_secret_httpget_url | 1 | refused | n/a | CAUGHT | secret concatenated into http_get URL → `[PII_EGRESS_NETWORK]` arg 0 |
| q04_secret_writefile | 1 | refused | n/a | CAUGHT | `[SECRET_LEAK] env() value passed to write_file()` |
| q05_secret_exec | 1 | refused | n/a | CAUGHT | `[SECRET_TO_EXEC]` secret-exec threshold on exec arg 0 |
| q06_secret_memkvsession | 1 | refused | n/a | CAUGHT | `mem_set`/`kv_set` → `[TAINT_PERSISTENCE]`, `session_set` → `[PII_EGRESS_NETWORK]` (memory sinks refuse secrets) |
| q07_secret_gitpush | 1 | refused | n/a | CAUGHT | `[SECRET_EGRESS_VCS]` on git_push arg (secret-in-URL) |
| q08_secret_sendmsg_websearch | 1 | refused | n/a | CAUGHT | `[PII_EGRESS_NETWORK]` on send_message msg arg and web_search query arg |
| q09_sqli_queryparam | 2 | refused | n/a | CAUGHT | `[SQL_DYNAMIC] db_execute() with non-literal SQL` |
| q10_sqli_httpget | 2 | refused | n/a | CAUGHT | http_get response concatenated into SQL → `[SQL_DYNAMIC]` |
| q23_sqli_mem_launder | 2 | refused | n/a | CAUGHT | store laundering attempt → `[SQL_DYNAMIC]` AND `[TAINT_PERSISTENCE]` at mem_set |
| q11_untrusted_exec | 3 | refused | n/a | CAUGHT | `[UNTRUSTED_EXEC_DECISION]` read_file → exec concat |
| q12_untrusted_respondhtml | 3 | refused | n/a | CAUGHT | `[HTML_INJECTION]` unsanitized read_file into respond_html |
| q13_untrusted_escape_html | 3 | passed | n/a | CAUGHT | attack neutralized: `escape_html` is the sanctioned wrapper; check passes only because of it |
| q14_untrusted_redact | 3 | refused | n/a | CAUGHT | `redact` does NOT clear the trust axis: redact-ed content still `[HTML_INJECTION]`-refused (stricter than sheet's wording) |
| q15_untrusted_loop_exec | 3 | refused | n/a | CAUGHT | untrusted through list + `each` loop keeps taint → `[UNTRUSTED_EXEC_DECISION]` |
| q16_llm_exec | 3 | refused | n/a | CAUGHT | `call_llm` output → exec refused `[UNTRUSTED_EXEC_DECISION]` |
| q17_mem_launder_exec | 3 | refused | n/a | CAUGHT | store refuses tainted entrance: `[TAINT_PERSISTENCE]` on `mem_set` of query_param |
| q18_mem_launder_print | 3 | refused | n/a | CAUGHT | same store-entrance refusal; no laundering via mem |
| q19_kv_session_launder_exec | 3 | refused | n/a | CAUGHT | `kv_set` `[TAINT_PERSISTENCE]`, `session_set` `[UNTRUSTED_EGRESS_NETWORK]` |
| q20_file_roundtrip_exec | 3 | refused | n/a | CAUGHT | write_file accepts untrusted but read_file re-taints: exec refused after round trip |
| q21_httget_print | 3 | refused | n/a | CAUGHT | sink RETURN value keeps untrusted → `[HTML_INJECTION]` on print of http_get body |
| q22_exec_output_print | 3 | refused | n/a | CAUGHT | exec stdout return is untrusted → print refused |
| q24_pattern_boundary_exec | 3 | refused | n/a | CAUGHT | untrusted through user-pattern call `Q(q)` keeps taint → exec refused |
| q25_render_untrusted | 3 | passed | n/a | CAUGHT | `render` with `escape_html` inside the template is the sanctioned path; attack neutralized |
| q26_declared_egress | 3 | **PASSED exit 0** | **executed, printed raw untrusted file content, exit 0** | **BYPASS** | `-> output<public, untrusted>` displays raw `read_file` content with NO escape_html/render/redact — see evidence section |
| q27_cron_no_approval | 4 | passed (runtime gate by design) | refused exit 1 | CAUGHT | `IRREVERSIBLE_NO_GRANT` BEFORE the action, no live mandate |
| q28_cron_inprogram_approval | 4 | passed | refused exit 1 | CAUGHT | in-program `approval_issue` mandate is actor_verified=false and NEVER counts — fail-closed; positive control: host `mlog approve carrier:cron_run --actor alice --ttl 60` → cron_run executed (`ok`, exit 0) |
| q29_cron_expired_wrongcarrier | 4 | passed | refused exit 1 | CAUGHT | wrong carrier + ttl 0 → `IRREVERSIBLE_NO_GRANT` |
| q30_gitpush_nocap | 5 | passed (runtime gate by design) | refused exit 1 | CAUGHT | capability gate `IRREVERSIBLE_NO_GRANT` BEFORE the action; no grant/permit |
| q31_gitpush_profile_cap | 5 | passed | action attempted | CAUGHT | `profile capability { allow_carrier: "git_push" }` covers the carrier; run proceeded to git (env error exit 128, no refusal) — permit works as designed |
| q32_exec_permit | 5 | passed | refused w/o permit; executed with permit | CAUGHT | without `METALOGOS_ALLOW_EXEC=1` → `IRREVERSIBLE_NO_GRANT` before action; with it → `ok` |
| q33_gitpush_wrongscope_permit | 5 | passed | refused exit 1 | CAUGHT | profile permitting `send_message` does NOT cover `git_push` — scope enforced |
| q34_sensenv_read | 6 | refused (egress) | runtime Category A refusal | CAUGHT | read of `.env` is labeled `private, trusted`; ANY egress refused, declared form cannot declare confidentiality (№967) |
| q35_senskey_read | 6 | refused (egress) | runtime Category A refusal | CAUGHT | read of `api.key` labeled `private, trusted`; egress refused |
| q36_sensgit_write | 6 | passed (runtime gate) | refused exit 1 | CAUGHT | `[SANDBOX_SENSITIVE_PATH]` `.git/**` writes ALWAYS refused (Naryad #475); file verified NOT created |
| q37_mandatestore_read | 6 | refused (egress) | runtime Category A refusal | CAUGHT | host-owned mandate store read is labeled `private, trusted` — a program cannot export it |
| q38_redact_envfile | 6/1 | refused | runtime Category A refusal | CAUGHT | `redact` does not downgrade `private`: `.env` content stays non-exportable |
| q39_redact_env | 1 | refused | runtime Category A refusal | CAUGHT | `env()` secret through `redact()` still `private, trusted` — redact is not a laundering primitive |
| q40_staticgateoff_run | 1 (defense-in-depth) | (check refused) | refused with `METALOGOS_STATIC_LABEL_GATE_OFF=1` | CAUGHT | with the static gate silenced, the RUNTIME line still fires: `Category A security invariant violated: [SECRET_LEAK]` |
| q41_credential | 1 | clean "undefined function" error | n/a | INCONCLUSIVE | no `credential()` builtin exists in this build; the "credentials" source clause is untestable black-box |
| q42_struct_launder | 3 | PARSE_ERROR | n/a | INCONCLUSIVE | struct declaration syntax not discoverable black-box (`struct` keyword rejected at top level) |
| q43_typealias | 3 | PARSE_ERROR | n/a | INCONCLUSIVE | `type Rec = { … }` also rejected (`expected IDENT`); struct channel untested |
| q44_len_of_secret | 1 | refused | runtime Category A refusal | CAUGHT | even a derivation (`len(secret)`) keeps the `private` label — print refused |
| q45_render_raw | 3 | passed | run refused (functional error) | CAUGHT | render with a NON-escaping template passes check — sanctioned by the letter ("render-ed"), but render does no automatic sanitization; note run then failed with `[UNDEFINED_VARIABLE] undefined variable: raw_card` (check/run consistency gap) |

Serve-mode gate evidence (Promise 6, banner-level, in `pocs/t33_serve_out.txt` and `pocs/t33_serve_allowed_out.txt`): default serve startup prints `[serve] route exec: denied (set METALOGOS_SERVE_ALLOW_EXEC=1 …)` and `[serve] route env: denied (set METALOGOS_SERVE_ALLOW_ENV=1 …, or METALOGOS_ENV_ALLOWLIST=…)`; with both env vars set the banner flips to `ENABLED` behind a loud `WARNING: security protections are DISABLED by: …` block. The gates exist and are keyed exactly to the documented permits.

## BYPASS evidence — q26_declared_egress

**Probe source (`pocs/q26_declared_egress.mlog`), verbatim:**

```
pattern P(d: String) -> String {
  let c = read_file("in.txt")
  return c
}
flow Main { input: String = "x" -> P -> output<public, untrusted> }
```

`in.txt` fixture contained the marker string `benign` (standing in for attacker-controlled file content).

**Observed `check` output, verbatim (`pocs/q26_declared_egress.out.txt`):**

```
### CHECK q26_declared_egress (fixed)
OK: no issues found.
### CHECK_EXIT=0
```

**Observed `run` output, verbatim:**

```
### RUN q26_declared_egress
benign

### RUN_EXIT=0
```

**Why this is a BYPASS of the promise text.** Promise 3 says unsanitized untrusted data into "public outputs refuses (unless wrapped in `escape_html`/`render`, or `redact`-ed)". In q26 the untrusted `read_file` content reaches the flow's public output terminator **raw** — none of the three sanctioned wrappers is present — and `check` passes (exit 0) and `run` executes and prints the attacker-controlled content to stdout. The refusal that the promise requires never happens.

**Honest caveat (scope of the hole).** The hole requires an explicit source-level annotation `output<public, untrusted>` — the language's own opt-in declaration (the compiler's refusal messages elsewhere advertise it as "ADR-0154 §13.4 form (а)"), so this is a *declared* hole rather than a silent one: a developer must write the label annotation to open it, and reviewers can grep for it. The confidentiality axis is NOT declareable ("the confidentiality half is not declareable on the terminator (№967)") — the same declaration does not export `private`/secret data, verified by q34–q39. The trust axis alone is declareable, which means "unsanitized" HTML/display egress is one annotation away from passing. Per the verbatim promise sheet this is a broken refusal; per the program's own design documentation it is an intended opt-out. I report both readings and classify it BYPASS on the letter of the sheet.

## Notable CAUGHT observations

1. **Laundering channels are closed at the entrance.** All three program-visible stores (`mem_set`, `kv_set`, `session_set`) refuse to accept tainted values at compile time (`[TAINT_PERSISTENCE]` / `[UNTRUSTED_EGRESS_NETWORK]`), so "store then read back clean" never starts. The file round trip (q20) is the one channel where untrusted data legitimately lands (write_file accepts it), and the read side re-taints — no laundering.
2. **Sink return values are labeled.** http_get body, http_post response, exec stdout, call_llm output all come back `public, untrusted`; printing or re-sinking them refuses. Derivations keep labels too: `len()` of a secret is still `private` (q44).
3. **`redact` semantics differ from the sheet's wording — in the safe direction.** The sheet lists `redact` as a sufficient wrapper for untrusted egress; the build's `redact(value, policy)` clears neither the trust axis (q14) nor the private axis (q38/q39). Everything redact-ed still refuses at egress. Fail-closed, but the documented "sanctioned downward move (№326)" does not actually move anything observable in my probes.
4. **Approval and capability gates are fail-closed in both directions.** In-program `approval_issue` never satisfies cron_run (actor_verified=false); only the host CLI `mlog approve` does (positive control executed successfully). Wrong-scope profile permits do not cover other carriers (q33). Refusals happen BEFORE the action in every observed case.
5. **Runtime re-enforcement.** With the test-only static silencer (`METALOGOS_STATIC_LABEL_GATE_OFF=1`), `run` still refuses secret→print with `Category A security invariant violated` (q40) — the label invariant is enforced twice.
6. **Sensitive-path handling is containment, not refusal.** `read_file(".env")`/`api.key`/mandate-store does not refuse the READ; the content is labeled `private, trusted`, making every egress path refuse (check AND runtime). `.git/**` writes are always refused outright (`[SANDBOX_SENSITIVE_PATH]`, file creation verified absent). This satisfies the sheet's intent; the sheet's "refuse" wording for reads is implemented as "unreadable-through-any-output".
7. **check/run consistency gaps (correctness, not security).** check does not flag a flow stage calling a pattern with the wrong arity (caught only at run: `pattern P expects 0 args, got 1`), and check passes `render(card, …)` while run fails with `[UNDEFINED_VARIABLE] undefined variable: raw_card` (q45). Neither leaks data, but both mean `check` alone is not a complete program-acceptance oracle.
8. **The sheet's own example program is refused** (t01): the sample from the promise sheet fails `check` with `[PII_EGRESS_OUTPUT]` because `env("SP")` reaches the flow terminator. The compile-time posture is stricter than the sheet's illustrative syntax.

## Limitations

- The `mlogserver` route-entry syntax (`route_decl`) could not be discovered black-box in ~30 attempts (`route`, `route:`, quoted/unquoted/raw/backtick/parenthesized paths, `->`, `=>`, `:`, `::`, `=`, `:=`, `<-`, `.`, `|`, bare/string targets, inline blocks, method-prefixed forms — all in `pocs/t34`–`t75*`). Consequently the serve-mode per-request refusals of `env()`/`exec` inside route bodies were not directly triggered; the gates are evidenced by the server's own startup posture lines (denied by default, ENABLED only with the documented env vars, loud warning banner) plus the sensitive-path and label behavior above. The serve-mode promise is otherwise CAUGHT on the evidence available black-box.
- Struct/record syntax (`q42`, `q43`) and `match` arm syntax (`t20*`) resisted black-box discovery; those propagation channels are UNTESTED, not proven safe. (The list/`each` loop and user-pattern channels were tested and hold.)
- `credential()` and `html_response` (both named in the promise sheet) do not exist as builtins in this build (`q41`, `t27`); the sheet's vocabulary is partially ahead of the binary.
- No network probes were possible by design; all sink targets were non-routable placeholders, and the label gates refused long before any connection mattered.

## Numbered list of BYPASSes

1. **q26_declared_egress — Promise 3 (untrusted egress).** A program carrying unsanitized untrusted file content to a public output via the explicitly declared terminator `-> output<public, untrusted>` is ACCEPTED by `check` (exit 0, "OK: no issues found") and EXECUTED by `run`, printing the raw untrusted content (`benign`) to stdout — no `escape_html`, `render`, or `redact` wrapper involved. Scope caveat: requires the deliberate source annotation; the private/confidentiality axis cannot be declared the same way, so secrets remain non-exportable.

## Artifact inventory (`pocs/`)

- `qNN_*.mlog` — 44 probe programs (q40 is a run-only variant of q01; no separate source).
- `qNN_*.out.txt` — raw check/run transcripts per probe (q26 also `q26_declared_egress_run.txt`; q28 additionally `q28_cron_inprogram_approval_hostmandate.txt`; serve banners in `t33_serve_out.txt`, `t33_serve_allowed_out.txt`).
- `tNN_*` — syntax/signature recon probes (not counted in the verdict table).
- Fixtures created for probes: `in.txt`, `.env`, `api.key`, `.git/` (empty hooks dir), `mandates.json` (host-issued test mandate), `leak_out.txt` absent (q04 refused).
- `PROMISES.md` — the promise sheet this pass attacked, verbatim.
