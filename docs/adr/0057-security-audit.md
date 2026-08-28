# ADR-0057: Static Security Audit (`mlog audit`)

## Status: Accepted

## Context

Metalogos programs (.mlog) can define HTTP servers, handle user input, manage secrets, execute SQL queries, and generate HTML responses via LLM calls. Without automated security scanning, developers must manually verify that their programs follow OWASP best practices: no hardcoded secrets, parameterized SQL, CSRF protection, rate limiting, proper sanitization of LLM output, and no open redirects.

Existing approaches like Semgrep, Bandit (Python), and cargo-audit provide language-specific static analysis. Metalogos needs its own audit tool tailored to the .mlog AST, capable of detecting security anti-patterns specific to the Metalogos type system (opaque types `Secret`, `Html`, `Encrypted`) and runtime conventions (`call_llm`, `env()`, `respond()`, `query()`, `sandbox`).

## Decision

Implement `mlog audit <file.mlog>` as a static analysis subcommand that parses the source without executing it, walks the AST, and reports security findings with severity levels (Error, Warning, Info) and exit codes (0=clean, 1=errors, 2=warnings-only).

### Architecture

1. **Module**: `src/audit.rs` — standalone module, no external security analysis dependencies
2. **Public API**: `metalogos::audit_program(source) -> Result<AuditResult, String>`
3. **CLI**: `mlog audit <file>` in `src/main.rs` via clap derive
4. **Taint tracking**: `TaintTracker` per scope, tracking data provenance through variable bindings (`env()` -> Secret, `call_llm()` -> LlmOutput, `query_param()` -> UserInput, `render()` -> Sanitized)

### Security Checks (10 total)

| Check ID | Severity | What it detects |
|---|---|---|
| `SECRETS` | Warning | Hardcoded secret strings (30+ chars matching patterns like `sk-`, `api_key`, `token=`). Skips `env()` arguments. |
| `SQL_DYNAMIC` | Error | `query()` calls with non-literal SQL arguments. Reports `Info` if all query calls use string literals. |
| `SANDBOX_COVERAGE` | Warning | `adapt` or `mutate` declarations without a corresponding `sandbox` declaration in the same file. |
| `RATE_LIMIT` | Warning | `mlogserver` blocks missing `rate_limit` in their middleware list. Reports `Info` if present. |
| `CSRF` | Warning | `mlogserver` with POST/PUT/DELETE routes but no `csrf` middleware. Reports `Info` if present. |
| `HTML_INJECTION` | Warning | LLM output (`call_llm`/`call_claude` result) passed directly to `respond()`/`respond_html()` without `render()` or `escape_html()` sanitization. |
| `SECRET_LEAK` | Error | `env()` result passed to sink functions (`respond`, `respond_html`, `http_post`, `write_file`, `send_message`). |
| `OPEN_REDIRECT` | Warning | User-controlled input (`query_param`, `form_data`, `json_body`) passed to `respond()`/`respond_html()`. |
| `TAINT_PERSISTENCE` | Warning | File has both `memorize(call_llm(...))` and a scope using `recall()` + `respond()` — potential taint through memory persistence (heuristic, not data-flow). |
| `TAINT_PASSTHROUGH` | Warning | `respond()`/`respond_html()` arg is a trivial passthrough pattern call (single param, `return param`) wrapping an LLM source — potential taint through indirection (heuristic). |

### Output Format

```
[ERROR] line 5: SQL injection risk — query() with non-literal SQL (sql)
[WARN]  line 3: adapt Classify without sandbox declaration
[INFO]  line 1: server has csrf middleware ✓

Summary: 1 error, 1 warning, 1 passed
```

### Exit Codes

| Code | Meaning |
|---|---|
| 0 | Clean — no errors, no warnings |
| 1 | Has errors (SECRET_LEAK, SQL_DYNAMIC) |
| 2 | Has warnings only (SECRETS, SANDBOX_COVERAGE, RATE_LIMIT, CSRF, HTML_INJECTION, OPEN_REDIRECT, TAINT_PERSISTENCE, TAINT_PASSTHROUGH) |

### Prior Art

- **Semgrep**: Pattern-based static analysis with taint tracking. Inspired our check architecture.
- **Bandit**: Python security linter. Similar severity/output model.
- **cargo-audit**: Rust dependency vulnerability scanner. Inspired the CLI subcommand pattern.

## Consequences

### Positive

- Developers get immediate feedback on security issues before deployment
- CI/CD pipelines can run `mlog audit` as a gate check using exit codes
- Taint tracking catches cross-statement data flows (e.g., `env()` -> variable -> `respond()`)
- No external dependencies required — pure AST walking
- 18 unit tests + 1 integration test ensure correctness

### Limitations

- **Taint does not propagate through assignments**: Only initial `let` bindings are tracked; reassignment does not update taint. This is a known false-negative path.
- **Line numbers are approximate**: `find_line()` does substring search in source text rather than using Pest span information. Accurate enough for audit output but not exact.
- **No inter-file analysis**: Each file is audited independently. Cross-module taint flows (via `import`) are not tracked.
- **No `--format json` output**: Only console text format is supported. Machine-readable JSON output can be added later.

### Future Work

- Add `--format json` for CI integration
- Track taint through assignment chains
- Add `--ignore` flag to suppress specific check IDs
- Inter-file analysis via import graph
- Configuration file (`.mlog-audit.toml`) for threshold tuning
