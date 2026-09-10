# Security Policy

## Supported Versions

The following versions of Metalogos are currently supported with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.19.x  | :white_check_mark: |
| 0.18.x  | :white_check_mark: |
| < 0.18  | :x:                |

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
- **Formal verification support** — Integration with proof assistants (planned; formulation last reviewed 2026-09-10 — unchanged, still planned: no verification has been performed, and the wording is kept because it is true, not because it is pretty)
- **Static security analysis** — Core security invariants (SQL injection, secret leaks, HTML injection) are enforced at compile time; additional advisory checks (hardcoded secrets, sandbox coverage, rate limiting, CSRF, open redirects) are available via `mlog audit`

### Generative pillars (Reflex, Vision)

The generative pillars extend the security surface. Reflex (local neural models, ADR-0114) is part of the default build; its sequence layer and the Vision pillar (images, ADR-0122) are feature-gated — `vision` (which implies `candle`) and `candle` are **not** part of `default`/`full` in `Cargo.toml [features]`. Supply-chain gates apply before any weights are used: `MODEL_WEIGHTS_UNSAFE` (a Category-A compile error) statically refuses SSRF-class hosts, bare un-pinned `.safetensors` URLs, and pickle-RCE-class weight formats at `vision_fetch_weights`, backed at runtime by a default-deny allowlist (`MLOG_VISION_WEIGHTS_ALLOWLIST`), SSRF resolve-pinning, and SHA-256 pinning; provenance is enforced by `VISION_UNSIGNED_EXPORT` (LSB watermark + `.manifest.json` sidecar on every default export). Honest status: **Vision weights run PARKED — no production PNG yet** (go-no-go №212 + runbook №237: the real-weights run has not been executed, so no production image claim is made).

## Acknowledgments

We thank the security researchers and community members who help keep Metalogos secure.

---

*Last updated: 2026-09-10*
