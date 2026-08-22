# ADR-0109: `imap` 3.0.0-alpha.15 — intentional pre-release dependency

> **Status:** Accepted  
> **Date:** 2026-08-21  
> **Naryad:** №116

## Context

Metalogos email/IMAP builtins (`src/builtins/email.rs`) use
`imap::ClientBuilder` with `ConnectionMode::AutoTls` / `TlsKind::Native`.
That API exists in the **3.0 development line** of the `imap` crate and is
**not** available in the last stable release **2.4.1**, which still exposes
the older `connect()`-style surface.

crates.io currently publishes **3.0.0-alpha.15** (2025-02-08) as the newest
3.0 artifact. The crate’s `main` branch continues development; the project
has also noted it is looking for maintainers.

An external audit correctly flagged a pre-release dependency in production
code. A naïve “just pin 2.4.1” would not be free: it forces a rewrite of the
connection path onto an older API the codebase deliberately left behind.

## Decision

1. **Keep** `imap = { version = "3.0.0-alpha.15", features = ["native-tls"] }`.
2. **Document** the choice in `Cargo.toml` (next to the dependency line) and
   in this ADR so future readers do not treat alpha as accidental drift.
3. **Mitigate** drift via `Cargo.lock` (locked resolution, no silent upgrades).
4. **Revisit** when a **3.0 stable** release appears on crates.io: bump the
   version string; prefer no logic change in `email.rs` if the ClientBuilder
   API remains stable as expected.

## Consequences

- Production still depends on a pre-release crate — accepted risk, not ignored.
- Live IMAP I/O is not exercised in CI without credentials; registry/arity
  coverage for IMAP builtins lives in `tests/phase_mlg4_email.rs` (not ignored).
- No automated “check crates.io every N days” job — too rare a task; human
  revisit on dependency hygiene passes is enough.

## Related

- `src/builtins/email.rs` — `ClientBuilder` connect path  
- `tests/phase_mlg4_email.rs` — registration tests (no live server in CI)
