# The security promises you are asked to break (Metalogos `mlog` v0.40.0)

You get ONLY this sheet and the binary. No source code, no tests, no ADRs, no internal docs.

1. **Compile-time secret-leak prevention** — a secret source (`env("...")`, credentials) reaching any of: network sinks (`http_post`, `http_get` URL, `send_message`, `web_search`), public outputs (`print`, `respond`, `respond_html`), file sinks (`write_file`, `append_file`), command execution (`exec`, `exec_argv`), VCS (`git_push`), memory sinks — is a COMPILE-TIME error: `mlog check` refuses the program.
2. **SQL injection is a compile-time error** — request/network/user data concatenated into `db_execute` SQL refuses (`SQL_DYNAMIC` class).
3. **Untrusted content is tracked end-to-end** — LLM output (`call_llm`), request data (`query_param`, `json_body`, `form_data`, `http_get`), MCP tool data, file reads (`read_file`) carry an untrusted mark through variables, string concatenation, structs, lists, loops (`each`/`while`), match branches, user-defined pattern calls and flows. Untrusted data driving `exec` refuses; unsanitized untrusted data into HTML (`respond_html`, `html_response`) or public outputs refuses (unless wrapped in `escape_html`/`render`, or `redact`-ed).
4. **The human-oversight approval** — `cron_run(id)` (the autonomous scheduled-run trigger) executes ONLY while a live approval mandate exists (recorded via `approval_issue("actor", "carrier:cron_run", ttl)`); without one it refuses BEFORE the action with `IRREVERSIBLE_NO_GRANT`.
5. **The capability layer** — irreversible carriers refuse without a covering capability grant or a `profile capability { allow_carrier: ... }` permit.
6. **Serve is strict by default** — in `mlog serve`, `env()` needs `METALOGOS_SERVE_ALLOW_ENV`, `exec` needs `METALOGOS_SERVE_ALLOW_EXEC`, sensitive file paths (`.env*`, `*.key`, `.git/**`, ...) refuse.

The CLI: `./mlog982 check <file.mlog>` (static analysis), `./mlog982 run <file.mlog>` (execute). A BYPASS = a program where a promise above says "refuses" but check PASSES (and/or run executes with the secret/untrusted data flowing to the promised-forbidden place). Everything else is CAUGHT — also a result, report it honestly.

## How programs look (the only syntax guidance you get — learn more by probing)

```
pattern P(d: String) -> String {
  let r = env("SP")
  return r
}
flow Main { input: String = "x" -> P -> output }
```

`check` prints refusal classes in `[CLASS_NAME]` brackets and exits non-zero on findings; `run` executes (print goes to stdout; runtime refusals print `[CLASS_NAME]` markers and fail). If a probe does not parse, simplify the shape and re-check — PARSE_ERROR is not a security verdict. Notes from the field: `exec` takes exactly ONE string argument; `session_set(sid, key, value)` is THREE-argument (the session id, the key, the value) while `mem_set(key, value)` is two; environment permits that exist: `METALOGOS_ALLOW_EXEC=1` (a named host permit for exec), `METALOGOS_STATIC_LABEL_GATE_OFF=1` (a TEST-ONLY silencer of the static label gates — using it in `run` probes is legitimate for probing whether the RUNTIME line itself catches things the static line was silenced for), and the file sandbox may require the probe file to live in the current directory (use bare filenames, not subdirectories).

## Your task

1. Design and run AT LEAST 15 distinct probes (aim for 18-25). Spread across all six promises, with SPECIAL ATTENTION to STORAGE AS A LAUNDERING MACHINE — this pass's focus: values stored and read back through `mem_set/mem_get`, `session_set/session_get`, `kv_set/kv_get`, and `write_file`→`read_file` file round-trips. Attack every dimension you can think of: store untrusted → read back → display; store into MULTIPLE keys and read one (key confusion); store a secret and read it under a DIFFERENT key's context; the session id as a namespace confused with the key; store trusted, overwrite with untrusted (and the reverse); store untrusted into file A, read from file B (path confusion); write untrusted, READ IT IN A SECOND RUN (the process boundary — write in probe A, run a separate program in probe B that reads the same file); append_file accumulating untrusted across writes; store trusted → read back → use as an `exec` decision or an `if` decision; the sensitive-path vocabulary (.env, api.key) written and read back; escape_html BEFORE storing vs AFTER reading (does the sanitizer's effect survive storage?); concat of stored+fresh. Some probes will be CAUGHT — that is fine and expected; the point is to find what is NOT.
2. For every probe: save the program as `qNN_<short_shape>.mlog` in your scratch `pocs982/`, capture the raw `check` (and, where relevant, `run`) output into the same directory as `qNN_<short_shape>.out.txt`, and record the verdict (BYPASS / CAUGHT / INCONCLUSIVE) plus one line of evidence.
3. Write `/home/z/my-project/metalogos-work/repo/docs/audits/red-team-2026-11-19-982/report.md` with: an executive summary (the counts, the binary SHA256, the date, the adversary posture statement, and one line on what differs from the earlier same-day passes if you can tell), the full probe table (probe → promise attacked → check verdict → run verdict → BYPASS/CAUGHT/INCONCLUSIVE), and for every BYPASS the full evidence (probe source + observed output verbatim). Copy your final `pocs982/` directory (programs + transcripts) into the report directory's `pocs/`, and copy the PROMISES.md verbatim into that `pocs/` too (write it out from this sheet above).
4. In the report, end with a numbered list of every BYPASS (or «none»), each with its promise and the observed behavior.

Report your final conclusion back to me in your last message: the probe count, the verdict counts, and the list of BYPASSes (probe names + one line each) — or «no bypass found».
