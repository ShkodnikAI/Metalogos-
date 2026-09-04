// ── Phase 6.4/6.5 — Crypto & Auth builtins ───────────────────────────

use crate::interpreter::{SecretString, Value};

use super::core::expect_string_arg;

pub(crate) fn builtin_hash_password(args: &[Value]) -> Result<Value, String> {
    // Accept both String (backward compat) and Secret (secure path)
    let password = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Secret(zs)) => zs.as_str().to_string(),
        Some(other) => {
            return Err(format!(
                "hash_password() expected String or Secret as first arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("hash_password() requires 1 argument".to_string()),
    };
    // Argon2id with random salt — real password hashing (Phase 7.3)
    // Наряд №173: argon2 0.6 dropped `SaltString`. Use `generate_salt()`
    // (returns `[u8; 16]`) and `hash_password_with_salt(password, &[u8])`.
    use argon2::{password_hash::generate_salt, Argon2, PasswordHasher};

    let salt = generate_salt();
    let argon2 = Argon2::default(); // Argon2id
    match argon2.hash_password_with_salt(password.as_bytes(), &salt) {
        Ok(hash) => Ok(Value::Hash(hash.to_string())),
        Err(e) => Err(format!("hash_password() failed: {}", e)),
    }
}

pub(crate) fn builtin_verify_password(args: &[Value]) -> Result<Value, String> {
    // Accept both String (backward compat) and Secret (secure path)
    let password = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Secret(zs)) => zs.as_str().to_string(),
        Some(other) => {
            return Err(format!(
                "verify_password() expected String or Secret as first arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("verify_password() requires 2 arguments".to_string()),
    };
    let hash_str = match args.get(1) {
        Some(Value::Hash(h)) => h.as_str(),
        Some(other) => {
            return Err(format!(
                "verify_password() expected Hash as second arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("verify_password() requires 2 arguments".to_string()),
    };
    // Real Argon2id verification with constant-time comparison (Phase 7.3)
    // Наряд №173: argon2 0.6 `PasswordVerifier<str>` accepts `&str` directly,
    // no need to construct `PasswordHash` first. `Error::Password` variant
    // renamed to `Error::PasswordInvalid` in password-hash 0.6.
    use argon2::password_hash::Error as PhError;
    use argon2::{Argon2, PasswordVerifier};

    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), hash_str) {
        Ok(_) => Ok(Value::Bool(true)),
        Err(PhError::PasswordInvalid) => Ok(Value::Bool(false)),
        Err(e) => Err(format!("verify_password() failed: {}", e)),
    }
}

pub(crate) fn builtin_encrypt(args: &[Value]) -> Result<Value, String> {
    let data = expect_string_arg("encrypt", args, 0)?;
    let key_str = match args.get(1) {
        Some(Value::Secret(zs)) => zs.as_str(),
        Some(other) => {
            return Err(format!(
                "encrypt() expected Secret as second arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("encrypt() requires 2 arguments".to_string()),
    };
    // Real AES-256-GCM with random 96-bit nonce (Phase 7.3)
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("encrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "encrypt() key must be 256-bit (64 hex chars), got {} bytes",
            key_bytes.len()
        ));
    }
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| "encrypt() key conversion failed".to_string())?;
    let cipher = Aes256Gcm::new(&key);
    let mut nonce_bytes = [0u8; 12]; // 96-bit nonce
                                     // Наряд №173: rand 0.10 — `thread_rng()` → `rng()`, `RngCore` trait
                                     // merged into `Rng`. `fill_bytes` is now a method on `Rng` itself.
    use rand::Rng;
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| "encrypt() nonce conversion failed".to_string())?;

    match cipher.encrypt(&nonce, data.as_ref()) {
        Ok(ciphertext) => {
            // Prepend nonce to ciphertext for self-contained Encrypted value
            let mut output = nonce.to_vec();
            output.extend_from_slice(&ciphertext);
            Ok(Value::Encrypted(output))
        }
        Err(e) => Err(format!("encrypt() AES-256-GCM encryption failed: {}", e)),
    }
}

pub(crate) fn builtin_decrypt(args: &[Value]) -> Result<Value, String> {
    let encrypted = match args.first() {
        Some(Value::Encrypted(data)) => data.clone(),
        Some(other) => {
            return Err(format!(
                "decrypt() expected Encrypted as first arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    let key_str = match args.get(1) {
        Some(Value::Secret(zs)) => zs.as_str(),
        Some(other) => {
            return Err(format!(
                "decrypt() expected Secret as second arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    // Real AES-256-GCM decryption (Phase 7.3)
    // Encrypted format: nonce (12 bytes) || ciphertext_with_tag
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    if encrypted.len() < 13 {
        // Need at least 12 (nonce) + 1 (tag minimum)
        return Err("decrypt() invalid encrypted data: too short".to_string());
    }

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("decrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "decrypt() key must be 256-bit (64 hex chars), got {} bytes",
            key_bytes.len()
        ));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let nonce = Nonce::try_from(nonce_bytes)
        .map_err(|_| "decrypt() nonce conversion failed".to_string())?;
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| "decrypt() key conversion failed".to_string())?;
    let cipher = Aes256Gcm::new(&key);

    match cipher.decrypt(&nonce, ciphertext) {
        Ok(plaintext) => match String::from_utf8(plaintext) {
            Ok(s) => Ok(Value::String(s)),
            Err(_) => Err("decrypt() decrypted data is not valid UTF-8".to_string()),
        },
        Err(_) => Err("decrypt() failed: incorrect key or corrupted data".to_string()),
    }
}

pub(crate) fn builtin_generate_key(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
                  // Generate a real 256-bit random key (Phase 7.3)
                  // Наряд №173: rand 0.10 API — `rng()` replaces `thread_rng()`,
                  // `RngCore` merged into `Rng`.
    use rand::Rng;

    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    let key_hex = hex::encode(key_bytes); // 64 hex chars
    Ok(Value::Secret(SecretString::new(key_hex)))
}

pub(crate) fn builtin_authenticate(args: &[Value]) -> Result<Value, String> {
    let _email = expect_string_arg("authenticate", args, 0)?;
    let _password = match args.get(1) {
        Some(Value::Secret(_)) => true,
        Some(Value::String(_)) => true,
        Some(other) => {
            return Err(format!(
                "authenticate() expected Secret or String as password, got {}",
                other.type_name()
            ))
        }
        None => return Err("authenticate() requires 2 arguments (email, password)".to_string()),
    };
    // In interpreter mode, always fail (mock)
    Ok(Value::Unit)
}

pub(crate) fn builtin_session_login(args: &[Value]) -> Result<Value, String> {
    let _user_id = expect_string_arg("session_login", args, 0)?;
    // In interpreter mode, return empty session
    Ok(Value::Session(std::collections::HashMap::new()))
}

pub(crate) fn builtin_session_logout(args: &[Value]) -> Result<Value, String> {
    let _session = match args.first() {
        Some(Value::Session(_)) => true,
        Some(other) => {
            return Err(format!(
                "session_logout() expected Session, got {}",
                other.type_name()
            ))
        }
        None => return Err("session_logout() requires 1 argument".to_string()),
    };
    Ok(Value::Unit)
}

// ── Наряд №50 Block 3: SHA-256 / HMAC-SHA-256 / hex builtins ──

pub(crate) fn builtin_sha256(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s,
        _ => return Err("sha256() requires 1 argument (String)".to_string()),
    };
    use sha2::Digest;
    let result = sha2::Sha256::digest(text.as_bytes());
    Ok(Value::String(hex::encode(result)))
}

pub(crate) fn builtin_hmac_sha256(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("hmac_sha256() requires 2 arguments (key, message)".to_string());
    }
    let key = match &args[0] {
        Value::String(s) => s.as_bytes(),
        _ => return Err("hmac_sha256() key must be String".to_string()),
    };
    let message = match &args[1] {
        Value::String(s) => s.as_bytes(),
        _ => return Err("hmac_sha256() message must be String".to_string()),
    };
    use hmac::{Hmac, KeyInit, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|e| format!("hmac_sha256() invalid key: {}", e))?;
    mac.update(message);
    let result = mac.finalize();
    Ok(Value::String(hex::encode(result.into_bytes())))
}

pub(crate) fn builtin_hex_encode(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s,
        _ => return Err("hex_encode() requires 1 argument (String)".to_string()),
    };
    Ok(Value::String(hex::encode(text.as_bytes())))
}

pub(crate) fn builtin_hex_decode(args: &[Value]) -> Result<Value, String> {
    let hex_str = match args.first() {
        Some(Value::String(s)) => s,
        _ => return Err("hex_decode() requires 1 argument (String)".to_string()),
    };
    hex::decode(hex_str)
        .map(|bytes| {
            String::from_utf8(bytes)
                .unwrap_or_else(|_| format!("<binary: {} bytes>", hex_str.len() / 2))
        })
        .map(Value::String)
        .map_err(|e| format!("hex_decode() invalid hex: {}", e))
}
