// ── Phase 6.4/6.5 — Crypto & Auth builtins ───────────────────────────

use crate::interpreter::{SecretString, Value};

use super::core::expect_string_arg;

pub(crate) fn builtin_hash_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("hash_password", args, 0)?;
    // Argon2id with random salt — real password hashing (Phase 7.3)
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // Argon2id
    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => Ok(Value::Hash(hash.to_string())),
        Err(e) => Err(format!("hash_password() failed: {}", e)),
    }
}

pub(crate) fn builtin_verify_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("verify_password", args, 0)?;
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
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};

    let argon2 = Argon2::default();
    match PasswordHash::new(hash_str) {
        Ok(parsed_hash) => {
            // Constant-time comparison inside argon2
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(Value::Bool(true)),
                Err(argon2::password_hash::Error::Password) => Ok(Value::Bool(false)),
                Err(e) => Err(format!("verify_password() failed: {}", e)),
            }
        }
        Err(e) => Err(format!("verify_password() invalid hash format: {}", e)),
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
    use aes_gcm::aead::{Aead, KeyInit, OsRng};
    use aes_gcm::{AeadCore, Aes256Gcm, Key};

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("encrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "encrypt() key must be 256-bit (64 hex chars), got {} bytes",
            key_bytes.len()
        ));
    }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bit random nonce

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
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    match cipher.decrypt(nonce, ciphertext) {
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
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
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
