# Red-team pass report — delivery-side boundary of the mlog candidate

Candidate binary: `/home/z/my-project/metalogos-work/rt-978/mlog-candidate`
SHA256: `28d7cdaa08c2f47d8140d2105f3e1b613c4ea0b93c9f32b16008db6f0bb1d2b0`
Promise sheet: `/home/z/my-project/metalogos-work/rt-978/pocs/PROMISES.md` (verbatim self-description P1–P5).
Pass date: 2026-11-18. Black-box, observation only; no repository sources were read.

## Executive summary

26 probes were executed against the DELIVERY side of the taint boundary: the value that COMES BACK from a delivery/network/exec call (http response body, command stdout, a human's answer, messaging-call returns, git_push return) was fed into public/persistent output channels, including through laundering forms (concatenation, upper()/type_of() wrappers, let bindings, if/while branches, helper patterns, string interpolation).

- **BYPASS: 3** (p10, p15, p16) — programs carrying unsanitized delivery output to the `write_file` sink pass `check` cleanly on BOTH backends, and the sink verifiably received external content at runtime (canary strings recovered from the sink files).
- **CAUGHT: 18** — every laundering shape aimed at the public channel (respond/print/flow output) and at the messaging data arguments (send_message) was refused at check on both backends, and refused again at run.
- **INCONCLUSIVE: 2** (p17, p25) — compile-time laundering confirmed (check clean on both backends for unsanitized remote data aimed at the public output channel via string interpolation), but the strict BYPASS standard (sink receives external content) was not met: the VM performs NO interpolation at runtime (p25 printed the literal text `prefix: ${r}`), and p17's fetch died on the dead URL.
- **CONTROL: 3** (p00, p18, p19) — literal payloads and the promised-sanitized `redact(v,"all")` path pass check as promised; no literal-marker rule interferes with the control column.

**check vs `check --reference` disagreement: zero.** All 26 probes produced byte-identical verdicts on both backends.

The central finding: **the compile-time clearance table does not cover `write_file` at all.** Every refusal message observed names respond/print as "sink/public/irreversible" and send_message/tts_send via the "№391 bridge (six ACTION sinks)"; `write_file` never appears in any classification. Unsanitized remote/exec output therefore reaches the filesystem with no compile-time refusal, breaking P4 ("plaintext secrets reaching sinks ... are compile-time refusals") and P2 ("sinks check clearance at compile time") for this sink.

## Method and fairness notes

- All network targets were loopback (127.0.0.1) only: either the dead port `http://127.0.0.1:9/` or a host-local canary HTTP server on 127.0.0.1:8099 serving `CANARY-p16-remote-body` (started solely to observe whether the sink receives external content; no packet leaves the host).
- Two documented escape-hatch env vars were used for RUN observations only, as named by the binary's own refusals: `METALOGOS_ALLOW_EXEC=1` (named by the IRREVERSIBLE_NO_GRANT refusal) and `METALOGOS_HTTP_ALLOW_PRIVATE=1` (named by the SSRF-guard refusal). `check` is static and is unaffected by them.
- Runtime refusals occurring after a clean `check` are recorded verbatim; they are findings about check/run asymmetry, not successes of the compile-time promise.

## Probe table

| Probe | Promise attacked | Shape | check (VM) | check (--reference) | run | Verdict |
|---|---|---|---|---|---|---|
| p00_baseline | — | literal control | PASS | PASS | ok | CONTROL |
| p01_httpget_respond | P2/P4 | http_get → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p02_httpget_respond_call | P2/P4 | http_get → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p03_concat_launder | P2/P4 | http_get + literal concat → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p04_upper_launder | P2/P4 | upper(http_get) → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p05_typeof_launder | P2/P4 | type_of(http_get) → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p06_ifwhile_launder | P2/P4 | http_get via let mut + if/while → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p07_helper_pattern | P2/P4 | http_get → helper pattern → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p08_exec_respond | P2/P4 | exec stdout → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p09_execargv_flow | P2/P4 | exec_argv stdout → flow output | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p10_httpget_writefile | P2/P4 | http_get → write_file | **PASS** | **PASS** | SSRF guard / [HTTP_CONNECT] | **BYPASS** |
| p11_gitpush_respond | P2/P4 | git_push return → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p12_messaging_returns | P2/P4 | send_message/photo/document returns → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p13_callback_edit_tts | P2/P4 | callback/edit/tts returns → respond() | REFUSE [HTML_INJECTION] + [VOICE_EGRESS_UNCONSENTED] | same | Category A refusal | CAUGHT |
| p14_human_respond | P2/P4 | human's answer → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p15_exec_writefile | P2/P4 | exec stdout → write_file | **PASS** | **PASS** | wrote; sink file contained canary | **BYPASS** |
| p16_httpget_writefile_canary | P2/P4 | http_get body → write_file (loopback canary) | **PASS** | **PASS** | wrote; sink file contained remote body | **BYPASS** |
| p17_interpolation_launder | P2/P4 | `"prefix: ${r}"` → flow output | **PASS** | **PASS** | [HTTP_CONNECT] (dead URL) | INCONCLUSIVE |
| p18_redact_control | P4 (sanitized path) | redact(http_get,"all") → flow output | PASS [REDACT] | PASS [REDACT] | audit event printed | CONTROL |
| p19_respond_literal_control | control | literal → respond() | PASS | PASS | done | CONTROL |
| p20_env_respond | P2 (secret source control) | env("SP") → respond() | REFUSE [SECRET_LEAK],[PII_EGRESS_OUTPUT] | same | Category A refusal | CAUGHT |
| p21_queryparam_respond | P2 (user-input source) | query_param("q") → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p22_exec_sendmessage | P2/P4 | exec stdout → send_message body | REFUSE [UNTRUSTED_EGRESS_NETWORK],[HTML_INJECTION] | same | Category A refusal | CAUGHT |
| p23_httpget_sendmessage | P2/P4 | http_get body → send_message body | REFUSE [UNTRUSTED_EGRESS_NETWORK],[HTML_INJECTION] | same | Category A refusal | CAUGHT |
| p24_httppost_respond | P2/P4 | http_post response → respond() | REFUSE [HTML_INJECTION] | REFUSE [HTML_INJECTION] | Category A refusal | CAUGHT |
| p25_interpolation_canary | P2/P4 | `"prefix: ${r}"` → flow output (loopback canary) | **PASS** | **PASS** | printed `prefix: ${r}` (literal) | INCONCLUSIVE |

Full verbatim transcripts (source + both check backends + run) for every probe: `pocs/pNN_<shape>_transcript.txt`. Per-probe one-line table: `pocs/check_results.txt`.

## BYPASS evidence (full, verbatim)

### BYPASS 1 — p15_exec_writefile: exec stdout → write_file, compile-time silence on both backends

Source (`pocs/p15_exec_writefile.mlog`):

```
pattern P(d: String) -> String {
    let out = exec("echo CANARY-p15-exec-stdout")
    write_file("probe-tmp/p15_out.txt", out)
    return "wrote"
}

flow Main {
    input: String = "x" -> P -> output
}
```

Observed:

- `check` (VM): `OK: no issues found.` — exit 0
- `check --reference`: `OK: no issues found.` — exit 0
- `run` without knobs: `error: IRREVERSIBLE_NO_GRANT: capability gate: 'exec' is an irreversible carrier — no covering capability grant (issue one with scope "builtin:exec" or wider), no 'profile capability { allow_carrier: "exec" }' permit, or the named host permit (METALOGOS_ALLOW_EXEC=1); the refusal is BEFORE the action (ADR-0203 §2.4, №845 — fail-closed, the profile cannot weaken the GrantAlways classes)` — exit 1 (a runtime capability gate, NOT the promised compile-time sink clearance; note check said nothing)
- `METALOGOS_ALLOW_EXEC=1 run`: stdout `wrote`, exit 0. Sink file `probe-tmp/p15_out.txt` contained exactly:

```
CANARY-p15-exec-stdout
```

The command's stdout (external content) reached a persistent sink with no compile-time refusal on either backend. This breaks P4 ("plaintext secrets reaching sinks ... are compile-time refusals") and P2 ("sinks check clearance at compile time") for the write_file sink. Note the runtime exec capability gate is a separate mechanism and is fail-closed; the compile-time promise about SINKS is nonetheless silent.

### BYPASS 2 — p16_httpget_writefile_canary: http_get response body → write_file, remote body recovered from disk

Source (`pocs/p16_httpget_writefile_canary.mlog`):

```
pattern P(d: String) -> String {
    let r = http_get("http://127.0.0.1:8099/canary.txt")
    write_file("probe-tmp/p16_out.txt", r)
    return "wrote"
}

flow Main {
    input: String = "x" -> P -> output
}
```

Observed:

- `check` (VM): `OK: no issues found.` — exit 0
- `check --reference`: `OK: no issues found.` — exit 0
- `METALOGOS_HTTP_ALLOW_PRIVATE=1 run`: stdout `wrote`, exit 0. Sink file `probe-tmp/p16_out.txt` contained exactly:

```
CANARY-p16-remote-body
```

The remote response body was fetched over HTTP and persisted to disk through a program that both static backends accept without any refusal. Unsanitized network output reaching a sink is exactly the shape P4 promises is a compile-time refusal.

### BYPASS 3 — p10_httpget_writefile: same shape, absolute-path variant; compile-time silence confirmed independently

Source (`pocs/p10_httpget_writefile.mlog`): http_get to `http://127.0.0.1:9/` → `write_file("/tmp/mlog_p10_out.txt", r)` → return "wrote", wired through flow output.

Observed:

- `check` (VM): `OK: no issues found.` — exit 0
- `check --reference`: `OK: no issues found.` — exit 0
- `run` without knobs: `error: SSRF guard: URL 'http://127.0.0.1:9/' resolves to 127.0.0.1 which is a private/loopback/link-local or otherwise reserved address — outgoing requests to internal networks are blocked. Set METALOGOS_HTTP_ALLOW_PRIVATE=1 to disable this protection.` — exit 1 (runtime-only guard; check was silent)
- `run` with knobs: `error: [HTTP_CONNECT] http_get: connection failed: error sending request for url (http://127.0.0.1:9/)` — the fetch failed on the dead port, so this variant's own sink delivery was not observed; the sink-delivery step of the IDENTICAL shape was observed end-to-end in p16 (only URL and target path differ). Verdict BYPASS on the compile-time promise, with the runtime caveat recorded.

## INCONCLUSIVE evidence

- p17 / p25 (string interpolation laundering): `check` and `check --reference` both PASS a program that carries `http_get` output inside `"prefix: ${r}"` to the flow output (public channel). Compile-time laundering is thereby observed. However the strict BYPASS standard (sink receives external content) was not met: with the loopback canary, the VM printed `prefix: ${r}` literally — the VM does NOT perform `${...}` interpolation at runtime, so the remote body itself never reached the channel; p17's fetch failed on the dead URL before any sink was reached. Recorded verbatim in the transcripts. Risk note (observation, not speculation about internals): if interpolation is ever implemented, the same accepted program would deliver remote content to the public channel.

## Controls

- p00 (literal through flow): PASS on both backends — the analysis does not refuse innocuous literals.
- p19 (literal into respond): PASS on both backends — no literal-marker rule contaminates the control column.
- p18 (redact(http_get,"all") into flow output): PASS with [REDACT] marker — the promised sanitized path is open, so the refusals elsewhere are about taint, not about network values per se.

## check vs --reference

All 26 probes: byte-identical verdicts and refusal texts on the VM and reference backends. No disagreement found.

## Refusal classes observed

HTML_INJECTION, SECRET_LEAK, PII_EGRESS_OUTPUT, VOICE_EGRESS_UNCONSENTED, UNTRUSTED_EGRESS_NETWORK, REDACT (audit marker), plus runtime-only classes: IRREVERSIBLE_NO_GRANT, SSRF guard, SANDBOX_VIOLATION (absolute write paths), HTTP_CONNECT.

candidate: mlog-candidate sha256 28d7cdaa08c2f47d8140d2105f3e1b613c4ea0b93c9f32b16008db6f0bb1d2b0
