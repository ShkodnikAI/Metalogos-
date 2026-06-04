# ADR-0031: Secret, Encrypted, Hash Opaque Types

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.4 — Cryptographic Opaque Types

## Context

Cryptographic failures (OWASP A02:2021) frequently stem from secrets leaking through logging, error messages, or response bodies. When passwords, API keys, and tokens are represented as plain strings, nothing prevents them from being accidentally printed, logged, or serialized.

Metalogos needs first-class types for sensitive data that the language itself protects from accidental exposure.

## Decision

Introduce three opaque value variants — `Secret`, `Encrypted`, and `Hash` — that cannot be converted to `String`, printed, logged, or included in response bodies.

```mlog
let db_password = env("DB_PASSWORD")           // Secret
let hashed = hash_password("argon2", password) // Hash
let encrypted = encrypt(plaintext, key)        // Encrypted

// All three are opaque:
// print(db_password)    → runtime error: "cannot display Secret"
// log(hashed)           → runtime error: "cannot display Hash"
// to_string(encrypted)  → runtime error: "cannot convert Encrypted to String"
```

### Type Semantics

| Type | Produced By | Consumed By | Can Display? |
|------|------------|-------------|-------------|
| `Secret` | `env()`, `read_secret()` | `decrypt()`, database connect | No |
| `Encrypted` | `encrypt()` | `decrypt(key)` | No |
| `Hash` | `hash_password()`, `hash()` | `verify_password()`, `verify_hash()` | No |

### Debugging Override

In debug mode (`METALOGOS_DEBUG=1`), `Debug` trait displays `Secret("****")`, `Encrypted(****)`, `Hash(****)` — contents redacted.

## Prior Art

- **Rust `secrecy` crate:** `Secret<String>` wraps sensitive data; `Debug` impl redacts contents.
- **Haskell newtype:** `newtype Secret = Secret Text` — no derived `Show` instance.
- **Go `crypto/subtle`:** Constant-time comparison functions for sensitive byte slices.

## Consequences

- **Positive:** Plaintext secrets cannot leak through `print()`, logging, or HTTP responses.
- **Positive:** The type system enforces a clear data flow: secrets enter through `env()` and exit only through authorized operations like `decrypt()`.
- **Neutral:** Developers must use `Secret` consistently; mixing `Secret` and `String` requires explicit conversion through designated safe endpoints.
- **Negative:** Debugging secret-related logic requires opt-in debug mode, adding friction during development.
