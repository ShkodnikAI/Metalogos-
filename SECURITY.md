# Security Policy

## Supported Versions

The following versions of Metalogos are currently supported with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.20.x  | :white_check_mark: |
| 0.19.x  | :white_check_mark: |
| < 0.19  | :x:                |

## Reporting a Vulnerability

**Please do not open a public issue for security vulnerabilities.**

### Contact

Email: **security@metalogos.dev**

If you do not receive a response within 48 hours, please follow up.

### What to Include

When reporting a vulnerability, please include:

- **Description** — Clear explanation of the vulnerability
- **Impact** — What could an attacker achieve?
- **Steps to Reproduce** — Detailed instructions to trigger the vulnerability
- **Proof of Concept** — Minimal code or scenario demonstrating the issue
- **Affected Versions** — Which versions are vulnerable?
- **Suggested Fix** — If you have one (optional but appreciated)
- **Your Contact** — How we can reach you for follow-up

### Response Timeline

| Phase | Timeline |
|-------|----------|
| Acknowledgment | Within 48 hours |
| Initial Assessment | Within 7 days |
| Fix Development | Within 30 days (critical), 90 days (non-critical) |
| Coordinated Disclosure | After fix is released |

### Disclosure Policy

We follow **responsible disclosure**:

1. We acknowledge receipt of your report
2. We investigate and develop a fix
3. We release the fix and publish a security advisory
4. We publicly disclose the vulnerability with full details and credit to the reporter

### Security Credits

We publicly credit security researchers who report valid vulnerabilities (unless they prefer to remain anonymous).

### Scope

The following are in scope for security reports:

- Compiler vulnerabilities (code injection, buffer overflows, etc.)
- Runtime security issues
- Dependency vulnerabilities
- Build system security
- Documentation of security features

The following are **out of scope**:

- Social engineering attacks
- Physical security
- Third-party services not under our control
- Issues in unsupported versions

## Security Best Practices for Users

### When Using Metalogos

1. **Keep your compiler updated** — Always use the latest supported version
2. **Review dependencies** — Audit third-party packages before use
3. **Follow the principle of least privilege** — Run compiled code with minimal permissions
4. **Use sandboxes** — Declare `sandbox` blocks to restrict filesystem access and cap iteration limits at runtime
5. **Report suspicious behavior** — If you notice unexpected behavior, report it

### For Developers Building on Metalogos

1. **Validate all inputs** — Never trust external data
2. **Use memory-safe patterns** — Leverage Metalogos's security-by-design features
3. **Keep dependencies minimal** — Reduce attack surface
4. **Run `mlog audit`** — Static analysis for secrets, SQL injection, HTML injection, and more (`src/audit.rs`). Three critical checks (dynamic SQL, secret leaks via env sinks, unsanitized HTML in responses) are also enforced automatically at compile time by `mlog run`/`check`/`serve` — they cannot be bypassed even without an explicit `mlog audit` run (Наряд №98).
5. **Regular audits** — Periodically review your code for security issues

## Security Features of Metalogos

Metalogos is designed with security as a first-class concern:

- **Memory safety by default** — Built on Rust's ownership model
- **Type safety** — Prevents entire classes of bugs at compile time
- **Sandboxed execution** — `sandbox` declarations restrict filesystem access to an allowlist, block `exec()` calls, and cap loop iterations (10 000 per loop) at runtime
- **exec() behind explicit flags (Наряд №253, Variant A)** — `exec()`/`exec_argv()` are denied by default (`EXEC_NOT_PERMITTED`): process contexts (`mlog run`, `mlog check`, serve top level) require `METALOGOS_ALLOW_EXEC=1`; serve route bodies require `METALOGOS_SERVE_ALLOW_EXEC=1` — a route handler never inherits the process flag (replacement semantics, not AND). Every allowed invocation lands in the subprocess audit log. Internal subprocess calls (`exec_restricted` — html_render, pdf) take no flag: fixed binary, arguments from code, never from request bodies.
- **Formal verification support** — Integration with proof assistants (planned; formulation last reviewed 2026-09-10 — unchanged, still planned: no verification has been performed, and the wording is kept because it is true, not because it is pretty)
- **Static security analysis** — Core security invariants (SQL injection, secret leaks, HTML injection) are enforced at compile time; additional advisory checks (hardcoded secrets, sandbox coverage, rate limiting, CSRF, open redirects) are available via `mlog audit`

### Generative pillars (Reflex, Vision)

The generative pillars extend the security surface. Reflex (local neural models, ADR-0114) is part of the default build; its sequence layer and the Vision pillar (images, ADR-0122) are feature-gated — `vision` (which implies `candle`) and `candle` are **not** part of `default`/`full` in `Cargo.toml [features]`. Supply-chain gates apply before any weights are used: `MODEL_WEIGHTS_UNSAFE` (a Category-A compile error) statically refuses SSRF-class hosts, bare un-pinned `.safetensors` URLs, and pickle-RCE-class weight formats at `vision_fetch_weights`, backed at runtime by a default-deny allowlist (`MLOG_VISION_WEIGHTS_ALLOWLIST`), SSRF resolve-pinning, and SHA-256 pinning; provenance is enforced by `VISION_UNSIGNED_EXPORT` (LSB watermark + `.manifest.json` sidecar on every default export). Honest status: **Vision weights run PARKED — no production PNG yet** (go-no-go №212 + runbook №237: the real-weights run has not been executed, so no production image claim is made).

## Label lattice controls (Wave 1 — ADR-0154/0156/0161, landed 2026-09-15)

Since Wave 1 (наряды №322–№328) every value in the language carries a three-component security label **(conf, integrity, consent-scope)** — ADR-0154. Confidentiality is ordered `public < consented < private < poisoned`, where `poisoned` is an absorbing **quarantine**, not an ordinary level: neither `join` nor `meet` cures it (a confirmed-compromised channel — today `CanaryLeak` — has no legal sinks, and lattice arithmetic cannot launder it). Integrity is ordered `untrusted < trusted` and dual to confidentiality: mixing data can only lower integrity, combining requirements takes the stronger one. Consent scopes travel with the value and intersect on data combination — a value used in two contexts keeps only the consent both contexts carry.

### Compile-time gates (Category A — compile errors)

- **`SINK_CLEARANCE` family (№325, ADR-0161)** — at every call site of a classified sink builtin (the list is read from the №316 SSOT classification, never hand-written), each argument's inferred label must clear the sink. Default clearance is `public`; `poisoned` clears no sink at all. Specialized scenarios keep their own class names: `VOICE_EGRESS_UNCONSENTED` (voice egress without a consent scope), `UNTRUSTED_EXEC_DECISION` / `SECRET_TO_EXEC` (untrusted or private data driving `exec`/`exec_argv`), `SECRET_EGRESS_VCS` (private labels into `git_push`), `SECRET_EGRESS_NETWORK` (private-infrastructure destinations), `PII_EGRESS_NETWORK` / `PII_EGRESS_OUTPUT` (personal data into network/public outputs), `UNTRUSTED_EGRESS_NETWORK`, `IRREVERSIBLE_NO_GRANT` (destructive SQL literals — `DROP`/`DELETE`/`TRUNCATE`/`ALTER` — in `db_execute`), plus the inherited `SECRET_LEAK`, `HTML_INJECTION`, `TAINT_PERSISTENCE` classes.
- **`UNTRUSTED_DECISION` (№327)** — anti-injection on the integrity axis: data that decides control flow (`if`/`else if` conditions, `while` conditions, `match` scrutinees) must be `trusted`. Untrusted data as DATA (carrying it, transforming it, returning it) is legal. The sanctioned paths to a trusted decision: validate before deciding, or one-way redact (`hash_only` destroys the data and restores trust).
- **`redact(value, "<policy>")` (№326) — the only sanctioned downward move.** The policy is a VALUE that names both the transformation and the target confidentiality: `hash_only` → `public` (one-way SHA-256 fingerprint — the sanctioned path down), `all` → `public` (full masking, legacy ADR-0136), `secrets` → `public` (legacy), `pii` → `private`, `pii_strip` → `private`, `truncate` → `private` (conservative — pattern strips can miss data, so they do NOT declassify; the gate keeps blocking their output). Unknown policy words are loud runtime errors; dynamic (non-literal) policies pass the label through — no silent downward moves. Every application is an unconditional `REDACT_APPLIED` audit event (`[REDACT][audit-event]` on stderr, a `Severity::Info` finding in `mlog audit`) — no profile or env toggle can switch them off.
- **Literal markers (ADR-0161 §3)** — string literals carrying personal-data markers (passport/SNILS shapes, confidential wording) or private-infrastructure URL markers are seeded `private, trusted`. Sound by conservatism: literals without markers stay bottom, so plain programs keep a zero behavioral delta.

### Migration bridge — `profile legacy { egress: permissive_with_audit }` (ADR-0161)

Pre-lattice programs relay user-provided data into outputs as their whole point; cutting them off with no migration path would make the gate a cliff.

- **What the profile weakens**: ONLY the №325 sink-clearance verdicts become advisory — compilation and execution stay green, and every gate hit is recorded as an audit event (`[SINK_CLEARANCE][audit-event]` on stderr + a `Severity::Info` finding in `mlog audit`).
- **What it does NOT weaken**: every other Category-A gate stays at full strength — `SECRET_LEAK`, `HTML_INJECTION`, `UNTRUSTED_DECISION`, `SQL_DYNAMIC`, `UNTRUSTED_FRAME`, `MEDIA_SYNTHETIC_UNMARKED`, `MODEL_WEIGHTS_UNSAFE` and the rest are unchanged, and `REDACT_APPLIED` events remain unconditional.
- **How long it may stay**: it is a **bridge, not a residence**, and there is no fixed expiry date — the exit criterion is per-program. The profile is removed when every flow that hit the gate is either redacted/annotated to pass the strict gate or demonstrably dead. The audit-event count is the burn-down metric; a program whose event count is stuck at a non-zero value across releases carries standing debt, visible in every audit report (ADR-0161 §3).

### Runtime second line (№328, ADR-0156)

The compiler lowers static label knowledge into the bytecode: source-backed bindings carry `LabelJoin` (the VM seeds runtime labels from the same №316 mapping), and every sink call site carries `SinkCheck` — the runtime twin of the clearance gate. A runtime violation is a distinct `[SINK_CLEARANCE_RUNTIME]` error plus an audit-event line. The static gates remain the SSOT of every verdict; the runtime twin exists so any divergence between the two backends is loud, not silent. Label instructions are explicitly outside the JIT-eligible class (`bytecode::is_jit_eligible`) — when a JIT dispatcher appears it must reject label-bearing functions with a distinct error naming ADR-0156, never skip them silently.

### Evidence — the leak suite is BLOCKING

`tests/run_leak_suite.rs` + `examples/leak/` hold **28 negative programs that must not compile** (each pinned to its expected failure class via an `.error` file) and 16 legal positives that must keep compiling and running with pinned output. Since №325 the suite runs in BLOCKING mode: a negative that compiles (or a class mismatch) fails CI. This is the regression fence for every gate above.

### Honest status and parked objects

This section describes what has landed — no new controls are promised here. The **Vision real-weights run remains PARKED** (no production PNG; №294 formal No-Go) — the lattice does not change that status. Consent *sources* (voice egress consent) are Phase 2 (№335) — until then every voice egress is unconsented by default, loud by design. The grant algebra for irreversible operations is Phase 3 (№339, ADR-0155) — until then destructive SQL literals are gated loudly without grants.

References: ADR-0154 (the lattice), ADR-0156 (TW/VM/JIT parity), ADR-0158 (declassify-boundaries booking; the implemented declassify contract lives in ADR-0154 §10), ADR-0161 (sink clearance + the legacy profile). The annotated language contract is REFERENCE.md §2; the full trust-boundary analysis is [docs/threat-model.md](docs/threat-model.md).

## Acknowledgments

We thank the security researchers and community members who help keep Metalogos secure.

---

*Last updated: 2026-09-16*
