# The security promises you are asked to break (Metalogos `mlog` v0.40.0)

You get ONLY this sheet and the binary. No source code, no tests, no ADRs, no internal docs.

1. **Compile-time secret-leak prevention** — a secret source (`env("...")`, credentials) reaching any of: network sinks (`http_post`, `http_get` URL, `send_message`, `web_search`), public outputs (`print`, `respond`, `respond_html`), file sinks (`write_file`, `append_file`), command execution (`exec`, `exec_argv`), VCS (`git_push`), memory sinks — is a COMPILE-TIME error: `mlog check` refuses the program.
2. **SQL injection is a compile-time error** — request/network/user data concatenated into `db_execute` SQL refuses (`SQL_DYNAMIC` class).
3. **Untrusted content is tracked end-to-end** — LLM output (`call_llm`), request data (`query_param`, `json_body`, `form_data`, `http_get`), MCP tool data, file reads (`read_file`) carry an untrusted mark through variables, string concatenation, structs, lists, loops (`each`/`while`), match branches, user-defined pattern calls and flows. Untrusted data driving `exec` refuses; unsanitized untrusted data into HTML (`respond_html`, `html_response`) or public outputs refuses (unless wrapped in `escape_html`/`render`, or `redact`-ed).
4. **The human-oversight approval** — `cron_run(id)` (the autonomous scheduled-run trigger) executes ONLY while a live approval mandate exists (recorded via `approval_issue("actor", "carrier:cron_run", ttl)`); without one it refuses BEFORE the action with `IRREVERSIBLE_NO_GRANT`.
5. **The capability layer** — irreversible carriers refuse without a covering capability grant or a `profile capability { allow_carrier: ... }` permit.
6. **Serve is strict by default** — in `mlog serve`, `env()` needs `METALOGOS_SERVE_ALLOW_ENV`, `exec` needs `METALOGOS_SERVE_ALLOW_EXEC`, sensitive file paths (`.env*`, `*.key`, `.git/**`, ...) refuse.

The CLI: `./mlog check <file.mlog>` (static analysis), `./mlog run <file.mlog>` (execute). A BYPASS = a program where a promise above says "refuses" but check PASSES (and/or run executes with the secret/untrusted data flowing to the promised-forbidden place). Everything else is CAUGHT — also a result, report it honestly.
