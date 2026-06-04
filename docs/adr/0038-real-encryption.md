# ADR-0038: Real Encryption (Phase 7.3)

## Status: Accepted

## Context

Phase 6.4 introduced opaque types (`Secret`, `Encrypted`, `Hash`) but the underlying
implementations were stubs:
- `hash_password()` used `std::collections::hash_map::DefaultHasher` — not a real hash function
- `verify_password()` always returned `false`
- `encrypt()`/`decrypt()` used XOR with a repeating pattern
- `generate_key()` produced a deterministic value from timestamp

These stubs are insufficient for production use. OWASP recommends Argon2id for password
hashing and AES-256-GCM for symmetric encryption. Memory containing secrets must be
zeroed when no longer in use.

## Decision

### 1. Password Hashing: Argon2id
- Replace `DefaultHasher` with the `argon2` crate (v0.5)
- `hash_password(secret)` generates a random salt via `SaltString::generate(&mut OsRng)`
  and hashes with `Argon2::default()` (Argon2id variant)
- Output format: PHC string (`$argon2id$v=19$m=...`) stored in `Value::Hash`
- `verify_password(secret, hash)` parses the PHC string via `PasswordHash::new()` and
  verifies with constant-time comparison inside argon2

### 2. Symmetric Encryption: AES-256-GCM
- Replace XOR stub with `aes-gcm` crate (already in dependencies from Phase 6.4)
- `generate_key()` generates 32 cryptographically random bytes via `rand::thread_rng().fill_bytes()`,
  hex-encodes to 64 chars, wraps in `Value::Secret`
- `encrypt(data, key)` decodes hex key to 32 bytes, creates `Aes256Gcm` cipher,
  generates random 96-bit nonce via `AeadCore::generate_nonce(&mut OsRng)`,
  encrypts, and stores as `nonce (12 bytes) || ciphertext_with_tag`
- `decrypt(encrypted, key)` splits first 12 bytes as nonce, rest as ciphertext+tag,
  decrypts with AES-256-GCM. Returns `Err` on wrong key (no panic)

### 3. Zeroize for Secrets
- Added `zeroize` crate (v1)
- Created `SecretString(Zeroizing<String>)` wrapper in `interpreter.rs`
  - Implements `serde::Serialize` (serializes as `"[SECRET]"` — value never persisted)
  - Implements `serde::Deserialize` (wraps deserialized string in Zeroizing)
  - Implements `Deref<Target=String>` for ergonomic access
  - Memory automatically zeroed on drop
- `Value::Secret` now wraps `SecretString` instead of bare `String`

### 4. Dependencies Added
| Crate    | Version | Purpose                          |
|----------|---------|----------------------------------|
| argon2   | 0.5     | Password hashing (Argon2id)      |
| zeroize  | 1       | Memory zeroing on drop           |

Existing: `aes-gcm = "0.10"`, `rand = "0.8"`, `hex = "0.4"` (already in Cargo.toml)

## Contract Tests (8 tests, all passing)

| Contract | Test |
|----------|------|
| verify(correct_password, hash) -> true | `test_73_verify_correct_password` |
| verify(wrong_password, hash) -> false | `test_73_verify_wrong_password_returns_false` |
| encrypt -> decrypt round-trip | `test_73_encrypt_decrypt_roundtrip` |
| decrypt with wrong key -> error | `test_73_decrypt_wrong_key_returns_error` |
| generate_key produces 256-bit | `test_73_generate_key_256bit` |
| print(Secret) -> error | `test_73_print_secret_errors` |
| hash format is PHC argon2id | `test_73_hash_password_format` |
| random salt per call | `test_73_hash_password_random_salt` |

## Security Properties
- Password hashes use Argon2id with random salt (immune to rainbow tables)
- AES-256-GCM provides authenticated encryption (tamper detection via GCM tag)
- Nonce is random per encryption (no nonce reuse)
- Secret values zeroed from memory on drop (no residual secrets in RAM)
- Serde serialization of secrets emits `[SECRET]` marker, never the actual value
- print(Secret) is blocked at the type system level (Secret != String)

## Consequences
- MockLlm untouched — no impact on AI testing
- All Phase 6.4 opaque type contracts still pass
- Breaking change: `Value::Secret` now wraps `SecretString` instead of `String`.
  Any code matching on `Value::Secret(s)` must use `s.as_str()` instead of `s.clone()`.
