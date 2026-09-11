# Threat Model — Metalogos Static Audit

Concrete threat checks implemented in `src/audit.rs` (`mlog audit`).
Each row maps to a real `check_id` emitted by the analyzer.

## Category A — Blocking (Severity::Error)

These checks prevent compilation/execution. The program is rejected
until the data-flow shape is fixed.

| check_id | Threat | Source → Sink | OWASP | Mitigation |
|---|---|---|---|---|
| `SQL_DYNAMIC` | SQL injection | Non-literal SQL string → `query()` / `db_execute()` | A03 Injection | Reject `query()` and `db_execute()` unless the SQL argument is a string literal — `db_execute` has no safe non-literal path (same rule as `query`: parameterized calls require a literal SQL template with `?`/`$N` placeholders). Only parameterized queries compile. |
| `SECRET_LEAK` | Plaintext secret leakage | `env()` / `secret()` → `respond()`, `write_file()`, `http_post()` body, `reflex_train()` data/labels | A02 Cryptographic Failures | Static taint-tracking: both `env()` and `secret()` results are flagged as secret-derived (Category A). Audit rejects if the tainted value reaches a network, response, or model-training sink. **Наряд №201**: `reflex_train` data/labels (args 1, 2) are now checked — secrets baked into model weights persist via `reflex_save` (ADR-0116), bypassing file-level sinks. Runtime opacity (`Value::Secret`) is enforced in two ways: (1) `secret("KEY")` returns `Value::Secret` directly (hard-failure if env var missing); (2) `env("KEY")` returns `Value::String`, and `Value::Secret` appears only when the value is bound to a `Secret`-typed entity (`entity k: Secret = env("KEY")`). |
| `HTML_INJECTION` | XSS via LLM output | `call_llm()` / `call_claude()` / `reflex_generate()` / learnable pattern → `respond()` without sanitization | A03 Injection | Requires `render()` or `escape_html()` between LLM source and `respond()`. **Наряд №201**: `reflex_generate` output is untrusted (model trained on data that may include LLM-tainted content per ADR-0117). Learnable patterns (declared with `learnable pattern`) are also flagged — their output is the result of an LLM call, so it is untrusted text. Single-level inline nesting (e.g. `respond(upper(call_llm(...)))`) is detected; deeper nesting is a known boundary. |
| `UNTRUSTED_TRAINING_DATA` | Model poisoning / PII in weights | `json_body()` / `query_param()` / `form_data()` / `mcp_call()` → `reflex_train()` data/labels | A09 Security Logging and Monitoring Failures | **Наряд №201**: Untrusted user input must not be used as training data for Reflex models. User-provided data can poison the model (adversarial examples) or bake PII into weights that persist via `reflex_save` (ADR-0116). The check flags `reflex_train` args 1 (data) and 2 (labels) if they carry `UserInput` taint. Intraprocedural level — same as all Category-A checks. **Наряд №268**: `mcp_call()` output joins the `UserInput` sources (owner decision — reuse of the existing kind, ADR-0132 D3): the output of a third-party MCP tool is untrusted, so poisoning a Reflex model through an MCP tool is statically rejected from day one. Policy is pinned equal to `json_body` by a parity test. |

## Category B — Advisory (Severity::Warning)

These are heuristic checks. They may false-positive in safe code and
miss complex indirection. They produce warnings, not errors.

| check_id | Threat | Detection Method | OWASP | Why Warning, Not Error |
|---|---|---|---|---|
| `SECRETS` | Hardcoded secrets in source | Regex scan for secret-like patterns in string literals | A02 | Cannot distinguish real secrets from test fixtures or error messages. |
| `OPEN_REDIRECT` | Open redirect via user-controlled URL | `query_param()` / `json_body()` → `respond_html()` without validation | A01 | Custom URL validation (allowlist) is not recognized by the static check. |
| `TAINT_PERSISTENCE` | XSS via memorize/recall persistence | File-level: `memorize(call_llm(...))` + same-scope `recall()` + `respond()` | A03 | Data flow through persistence is not tracked. Same-scope heuristic only. |
| `TAINT_PASSTHROUGH` | XSS via trivial passthrough pattern | `respond(Passthrough(call_llm(...)))` where `Passthrough` is a 1-param `return param` pattern | A03 | Interprocedural taint is not tracked. Only exact trivial passthroughs (1 param, single return) are flagged. |
| `SANDBOX_COVERAGE` | Unsandboxed self-modification | `adapt` / `mutate` without enclosing `sandbox` block | A05 | Sandbox is opt-in. External review or infra-level isolation may handle the risk. |
| `RATE_LIMIT` | Missing rate limiting | No `rate_limit` middleware in `mlogserver` block | A05 | External infra (reverse proxy, CDN) may enforce rate limits. |
| `CSRF` | Missing CSRF protection | No `csrf` middleware in `mlogserver` block | A01 | Token-authenticated APIs do not need CSRF (cookies not used for auth). Cookie-based sessions do. |

## Vision / generative media (0.19+)

Vision (ADR-0122) is the feature-gated generative pillar: `vision` (which implies
`candle`) is **not** in `default`/`full` in `Cargo.toml`. Its security surface is
enforced by five audit checks in `src/audit.rs`. The two Category-A checks become
compile errors through the `audit_category_a` wiring (semantic №98 promotion). The
compile path deliberately runs an errors-only variant of the export-gate check
(`check_vision_export_gates_errors_only`): semantic №98 promotes every
`audit_category_a` Warning to a compile error, which would contradict ADR-0125's
explicit Warning severities — the three Vision warnings below stay advisory
(`mlog audit`), never compile errors.

| check_id | Severity | Threat | Vector (source → sink) | OWASP | Mitigation |
|---|---|---|---|---|---|
| `VISION_UNSIGNED_EXPORT` | Error (Category A) | Silent export of unsigned media — the provenance chain is broken by construction | `vision_export` call site in a file with **no** `vision { }` declaration — the provenance-manifest source is impossible there, so the artifact cannot be signed. Runtime backstop with the same check-id: exporting a manifest-less artifact via `vision_export_dispatch` is a loud `Err` (`src/builtins/vision.rs`) | A08 Data Integrity | Compile error by construction. Declare `vision { }` in the file, or use `vision_export_raw` explicitly (which stays a loud warning). |
| `VISION_UNSIGNED_EXPORT_RAW` | Warning (advisory) | Deliberate unsigned export — no watermark, no manifest sidecar | Every `vision_export_raw` call site — the explicit opt-out chosen in source is made loud | A08 Data Integrity | Advisory warning on every raw-export call site (ADR-0125's explicit opt-out; deliberately NOT promoted to a compile error). |
| `MODEL_WEIGHTS_UNSAFE` | Error (Category A) | Poisoned / unsafe model weights (supply-chain attack surface) | Literal URL at a `vision_fetch_weights(url, ...)` call site, three statically visible classes: (1) host in the SSRF-blocked class (loopback / private / link-local / metadata IPs, `localhost`; naryad #261 widened the class: IPv4-mapped IPv6, unspecified 0.0.0.0/::, CGNAT 100.64.0.0/10, benchmark 198.18.0.0/15); (2) bare `.safetensors` file — no manifest, no pinned SHA-256 source; (3) pickle-RCE-class extension (`.pkl`, `.pickle`, `.pt`, `.pth`, `.ckpt`, `.bin`, `.py`, `.so`, `.dll`, `.exe`, `.zip`, `.tar`, `.gz`, `.7z`) | A08 Data Integrity / A10 SSRF | Compile error by construction. Runtime layers in `vision_fetch_weights`: allowlist default-deny via `MLOG_VISION_WEIGHTS_ALLOWLIST` + SSRF guard with DNS resolve-pinning + per-entry SHA-256 pinning (reuses `WeightsManifest`). |
| `VISION_POLICY_MISSING` | Warning (advisory) | Honest use not explicit — a `vision { }` block declared without `policy:` | Every `vision { }` declaration whose `policy:` field is omitted; the manifest records `"policy": "unspecified"` | A05 Security Misconfiguration | Advisory warning — the audit IS the static policy validation (ADR-0125). The parser relax is policy-only; the other six declaration fields stay required. |
| `VISION_PROMPT_USER_INPUT` | Warning (advisory) | User-tainted input used as a generation/edit prompt | `form_data()` / `json_body()` / `query_param()` taint → argument 1 (the prompt) of `vision_generate`, `vision_edit`, `vision_lora_generate`. Argument 0 (declaration name / Vision handle / adapter name) is **not** flagged. `Value::Vision` itself carries no taint (opaque handle) | A03 Injection | Advisory warning: submitting a user-typed prompt is a legitimate use case; the prompt is recorded in the artifact's provenance manifest (`prompt_sha256`). |

### Honest boundary (ADR-0125 "Provenance MVP")

- **What ships:** every default export writes an LSB watermark (the bytes `MLGV`
  plus a 32-bit model hash, embedded in the RGB least-significant bits) and a
  `.manifest.json` sidecar (model id + weights SHA-256 or the literal `unpinned`,
  seed, prompt hash, policy or `unspecified`, timestamp, SHA-256 of the final PNG).
- **What this is NOT:** the LSB watermark + sidecar are provenance for our own
  pipeline — **not a cryptographic signature of the file and not proof for a third
  party**. The MVP watermark is detectable by us, not adversarially robust (it does
  not survive resize/JPEG in general).
- **Research backlog, not promised** (ADR-0125 phase 2): robust watermarking
  surviving resize/JPEG; C2PA-compatible manifests.
- **Taint boundaries for Vision are the same as everywhere in this model:**
  intra-procedural and positional — the prompt is argument 1; interprocedural
  flows are not tracked (see Known Boundaries above). No stronger claim is made.

## Known Boundaries

Patterns the audit does **not** detect (see README for full table):

- **Interprocedural taint**: LLM output passed through a non-trivial pattern call chain.
- **Persistence taint**: LLM output stored via `memorize()` then read back via `recall()` in a different scope.
- **`format()` in SQL**: `query(format("...", x))` — `format()` output is not a compile-time constant.
- **Inline nesting in open redirect**: `respond_html(query_param("url"))` — the check only tracks via variable, not inline call.
- **Raw template output**: `{{{ var }}}` in `template_render` with `raw=true` skips escaping by design.

## Runtime Protections (Outside Audit)

These are not audit checks but contribute to the overall threat posture:

- **Opaque `Secret` type**: opacity is enforced via two paths: (1) `secret("KEY")` returns `Value::Secret` directly — runtime opacity enforced immediately, hard-failure if env var missing (unlike `env()` which returns empty string); (2) `entity k: Secret = env("KEY")` → `coerce_to_declared_type` produces `Value::Secret` at binding time. In both cases, `Value::Secret` is zeroized on drop, and `print`/concat are blocked at runtime. A bare `env()` call (not bound to a `Secret` entity) returns a plain `String` — protection is then static-only (Category A taint).
- **SSRF prevention**: `http_get` / `http_post` reject private/internal IP ranges unless `METALOGOS_HTTP_ALLOW_PRIVATE=1` is set.
- **exec gates (Наряд №253, Variant A)**: `exec()` / `exec_argv()` are denied by default with the stable diagnostic code `EXEC_NOT_PERMITTED` (ADR-0131 naming convention). Two independent context gates — replacement semantics, not AND: process contexts (`mlog run`, `mlog check`, serve top-level route registration) require `METALOGOS_ALLOW_EXEC=1`; serve route bodies (the per-request HTTP handlers, TW and VM backends alike) require `METALOGOS_SERVE_ALLOW_EXEC=1` — the process-level flag deliberately does NOT reach route bodies, because route code is frequently authored or generated by someone else and must not inherit the operator's shell pass. Every allowed invocation is recorded in the subprocess audit log (`METALOGOS_AUDIT_LOG_PATH`). `exec_restricted` (html_render, pdf pipeline) has no flag: the binary is fixed by code and arguments are built from code, never from request bodies.
- **env gate (Наряд №259)**: `env()` in serve route bodies is denied by default with the stable diagnostic code `ENV_NOT_PERMITTED` — before №259 one `env("...")` call in a route/template handed the process's secrets (LLM API keys, DB credentials, deploy tokens) to code that receives untrusted input. The gate reuses the №253-А serve-route context SSOT (the same thread-local guard in both route paths, TW and VM) and runs BEFORE the read, so a denial is identical for existing and non-existing names (no existence oracle). Escape hatches — alternatives, not AND: `METALOGOS_SERVE_ALLOW_ENV=1` allows all env reads in route bodies, or `METALOGOS_ENV_ALLOWLIST="NAME1,NAME2"` allows exactly the listed names (unset/empty = deny all). Process contexts (`mlog run`, `mlog check`, serve top level) stay ungated — a local script reading its own environment is the contract. The serve banner prints the route-env state and lists both escape hatches among the danger flags.
- **MCP stdio client (Наряд №268, ADR-0132)**: `mcp_call` / `mcp_list_tools` spawn a third-party MCP server as a subprocess — the same exec class as `exec()`, so the spawn inherits the №253-А exec-gate verbatim (process flag in process contexts, `METALOGOS_SERVE_ALLOW_EXEC` in route bodies, refusal `EXEC_NOT_PERMITTED`) and every permitted spawn lands in the subprocess audit log. On top of the gate, `METALOGOS_MCP_ALLOWLIST` (comma-separated, trim/empty-element convention of №259, exact argv[0] match) narrows which server commands may run at all: unset does not narrow, an empty value denies all MCP, a non-empty list refuses anything else with `MCP_NOT_ALLOWLISTED`. The lifecycle is stateless per call (spawn → handshake → call → shutdown with a Drop kill-guarantee — no orphan MCP processes), so gate and audit attribution are one-to-one with calls. The tool OUTPUT carries `UserInput` taint (see `UNTRUSTED_TRAINING_DATA` above); tool METADATA (names/descriptions/inputSchema) is untainted by design — descriptions are third-party text, and including them in LLM context is the program's explicit decision (prompt-injection via tool descriptions is a documented boundary of the taint system, common to every MCP host). Children inherit the interpreter environment — the same contract as `exec()`/`exec_argv()`; the №259 env-gate governs `env()` reads in route bodies, not what spawned children see. Deployments that need child isolation should run the interpreter with a reduced environment.
- **Explicit request body limit (Наряд №255)**: every route body is capped at 2 MiB (2 097 152 bytes, `REQUEST_BODY_LIMIT_BYTES` in `src/server.rs`, applied via `DefaultBodyLimit` in `build_router`). Larger bodies get HTTP 413. Before №255 the limit was the implicit axum 0.8 default (~2 MB) — the source of truth lived in a foreign crate and would silently change on an upgrade; now the number is ours, documented here and in REFERENCE.md, pinned by the n255 test at N±1.
- **Sandbox timeout**: `sandbox { timeout: N }` cancels the *wait* at the deadline (preemptive via `mpsc::recv_timeout`). The in-flight HTTP request to the LLM provider may still complete.
- **File access sandbox**: `sandbox_path()` resolves symlinks via `canonicalize()` and verifies the path stays within the allowed base directory.
