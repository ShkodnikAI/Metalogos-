# Security Policy

## Supported Versions

The following versions of Metalogos are currently supported with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |

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
4. **Enable security features** — Use built-in security flags and sandboxing when available
5. **Report suspicious behavior** — If you notice unexpected behavior, report it

### For Developers Building on Metalogos

1. **Validate all inputs** — Never trust external data
2. **Use memory-safe patterns** — Leverage Metalogos's security-by-design features
3. **Keep dependencies minimal** — Reduce attack surface
4. **Enable compiler security checks** — Use `--security-level` flags where available
5. **Regular audits** — Periodically review your code for security issues

## Security Features of Metalogos

Metalogos is designed with security as a first-class concern:

- **Memory safety by default** — Built on Rust's ownership model
- **Type safety** — Prevents entire classes of bugs at compile time
- **Sandboxed execution** — Optional runtime isolation
- **Formal verification support** — Integration with proof assistants (planned)
- **Secure standard library** — All standard library functions are audited for security
- **Dependency scanning** — Built-in `cargo audit` integration

## Acknowledgments

We thank the security researchers and community members who help keep Metalogos secure.

---

*Last updated: 2026-08-09*
