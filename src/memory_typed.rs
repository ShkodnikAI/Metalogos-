// ── Naryad #350 (P1, feature/memory): the typed Memory<K> layer ──────
//
// The Phase-4 memory lane: a TYPED container over the existing memory
// subsystems (an ADDITIVE layer — the mem_/memorize/recall surfaces,
// the FTS5 recall (ADR-0093/0094) and the persistence store (ADR-0041)
// stay untouched; the consent_ledger store is not migrated — this
// module only ADDS a reader to it).
//
// Model (the module doc IS the design record; the ADR of the derived
// graph is №351's):
//   - a container is `Memory<K>` where K is the Phase-1 confidentiality
//     label of the contents: "public" or "private" (the conf words of
//     the ADR-0154 lattice; "secret"/"network" are NOT storage classes
//     — secrets live in the secret()/vault contour, network is a
//     boundary label, and both words are loud errors here);
//   - the handle is an opaque `Value::Memory` (the ADR-0114 pattern,
//     the Session precedent): the map carries `id`/`subject`/`label`
//     as the printable projection; the registry in this file is the
//     state;
//   - at-rest encryption: a PRIVATE container's entries are encrypted
//     with a PER-SUBJECT key derived from the process master via the
//     EXISTING crypto contour (HMAC-SHA-256 over the master, then the
//     in-tree AES-256-GCM — the `encrypt()` machinery; NO new crypto).
//     The master comes from `METALOGOS_MEMORY_MASTER` (64 hex chars)
//     or is freshly generated per process — the loud honest boundary:
//     without the env anchor a restart cannot decrypt (№351's
//     persistent store will pin the same discipline);
//   - per-subject isolation: each subject's private container encrypts
//     under its OWN derived key — a subject's entries are unreadable
//     outside their container by construction;
//   - cross-subject access goes through CONSENT (№335): opening a
//     private container requires an ACTIVE consent grant for the scope
//     `memory:<subject>` (the additive `consent::active_grant_for`
//     reader; fail-closed — no grant, no container);
//   - reads/exports are AUDITED SINKS: every read/export/keys/
//     provenance call appends an Action-Ledger record (`memory.*`
//     family, ADR-0167 §3.4, best-effort §2 driver 5) and emits the
//     loud audit line. The private READ returns `Value::Secret` — the
//     existing lattice does the rest: `print(Secret)` refuses, and
//     `redact()` (№326/ADR-0136) is the legal and ONLY egress path;
//     the explicit file export of a private entry refuses with
//     MEMORY_REDACT_REQUIRED (export the redact() output instead);
//   - derived-from: every put records the ORIGIN of the value (the
//     parent keys inside the same container, validated to exist —
//     fail-closed). This is the raw material of the derived-graph +
//     cascade naryad №351 (the graph itself is NOT built here).

use crate::interpreter::values::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// The container label K (Phase-1 conf words only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemLabel {
    Public,
    Private,
}

impl MemLabel {
    pub fn parse(word: &str) -> Result<MemLabel, String> {
        match word.trim().to_ascii_lowercase().as_str() {
            "public" => Ok(MemLabel::Public),
            "private" => Ok(MemLabel::Private),
            other => Err(format!(
                "memory_open: unknown container label '{}' (available: public, private) — secret/network are not storage classes",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MemLabel::Public => "public",
            MemLabel::Private => "private",
        }
    }
}

/// One stored entry. Private entries keep ONLY the AES-GCM blob
/// (nonce-prefixed, the `encrypt()` format); public entries keep the
/// plaintext. The plaintext of a private entry never rests in memory
/// outside the decrypted read.
#[derive(Debug, Clone)]
pub enum Stored {
    Plain(String),
    Enc(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct TypedEntry {
    pub stored: Stored,
    pub derived_from: Vec<String>,
    pub created_unix: u64,
}

#[derive(Debug)]
pub struct TypedContainer {
    pub id: String,
    pub subject: String,
    pub label: MemLabel,
    pub created_unix: u64,
    pub entries: HashMap<String, TypedEntry>,
}

type Registry = HashMap<String, TypedContainer>;

fn registry() -> &'static Mutex<Registry> {
    static REG: OnceLock<Mutex<Registry>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

static SEQ: AtomicU64 = AtomicU64::new(1);

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The process master key material: `METALOGOS_MEMORY_MASTER` (64 hex
/// chars = 32 bytes) or a fresh random key per process. The env anchor
/// is the restart-stability contract (loudly absent by default — the
/// in-process store is the only reader of its own blobs).
fn master() -> [u8; 32] {
    static MASTER: OnceLock<[u8; 32]> = OnceLock::new();
    *MASTER.get_or_init(|| {
        if let Ok(hex_key) = std::env::var("METALOGOS_MEMORY_MASTER") {
            if let Ok(bytes) = hex::decode(hex_key.trim()) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    return arr;
                }
            }
            eprintln!(
                "[MEMORY] WARNING: METALOGOS_MEMORY_MASTER is set but is not 64 hex chars — using a fresh per-process key"
            );
        }
        use rand::Rng;
        let mut arr = [0u8; 32];
        rand::rng().fill_bytes(&mut arr);
        arr
    })
}

/// Per-subject derived at-rest key (hex, 64 chars) — HMAC-SHA-256 over
/// the master with the subject as the message (the existing crypto
/// contour: `hmac_sha256`'s primitive, no new crypto).
fn subject_key_hex(subject: &str) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let Ok(mut mac) = HmacSha256::new_from_slice(&master()) else {
        // Unreachable with a 32-byte master, but never panic (the crate
        // denies expect_used): degrade to the sha256 contour (in-tree).
        use sha2::Digest;
        return hex::encode(sha2::Sha256::digest(
            format!("memory-typed-fallback:{}", subject).as_bytes(),
        ));
    };
    mac.update(format!("memory-typed:{}", subject).as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// AES-256-GCM encrypt with a random nonce (nonce-prefixed blob — the
/// same wire shape the `encrypt()` builtin produces).
fn encrypt_at_rest(plain: &str, key_hex: &str) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    let key_bytes = hex::decode(key_hex).map_err(|e| format!("memory key decode: {}", e))?;
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| "memory at-rest key conversion failed".to_string())?;
    let cipher = Aes256Gcm::new(&key);
    use rand::Rng;
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| "memory at-rest nonce conversion failed".to_string())?;
    let ct = cipher
        .encrypt(&nonce, plain.as_bytes())
        .map_err(|e| format!("memory at-rest encryption failed: {}", e))?;
    let mut blob = nonce_bytes.to_vec();
    blob.extend_from_slice(&ct);
    Ok(blob)
}

fn decrypt_at_rest(blob: &[u8], key_hex: &str) -> Result<String, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    if blob.len() < 13 {
        return Err("MEMORY_DECRYPT_FAILED: at-rest blob too short".to_string());
    }
    let key_bytes = hex::decode(key_hex).map_err(|e| format!("memory key decode: {}", e))?;
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| "memory at-rest key conversion failed".to_string())?;
    let cipher = Aes256Gcm::new(&key);
    let (nonce, ct) = blob.split_at(12);
    let nonce = Nonce::try_from(nonce)
        .map_err(|_| "MEMORY_DECRYPT_FAILED: invalid nonce length".to_string())?;
    let plain = cipher.decrypt(&nonce, ct).map_err(|_| {
        "MEMORY_DECRYPT_FAILED: at-rest blob does not decrypt under this subject key".to_string()
    })?;
    String::from_utf8(plain).map_err(|_| "MEMORY_DECRYPT_FAILED: blob is not UTF-8".to_string())
}

/// Best-effort Action-Ledger side effect (the grants.rs template).
fn ledger_memory_event(kind: &str, container_id: &str, detail: &str) {
    eprintln!(
        "[MEMORY_{}] {} {}",
        kind.to_uppercase(),
        container_id,
        detail
    );
    crate::ledger::record(&format!("memory.{}", kind), container_id, "memory", detail);
}

fn fresh_container_id(subject: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let preimage = format!("{}|{}|{}", subject, millis, seq);
    format!(
        "mem-{}",
        &crate::ledger::sha256_hex(preimage.as_bytes())[..16]
    )
}

fn container_value(c: &TypedContainer) -> Value {
    Value::Memory(HashMap::from([
        ("id".to_string(), c.id.clone()),
        ("subject".to_string(), c.subject.clone()),
        ("label".to_string(), c.label.as_str().to_string()),
    ]))
}

/// Extract the container id from a `Value::Memory` handle argument.
pub fn container_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::Memory(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: Memory handle has no id field — open containers with memory_open",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected Memory as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing Memory argument {}", fn_name, idx + 1)),
    }
}

/// Open (or return the existing) container for (subject, label).
/// Private containers are consent-gated: an ACTIVE consent grant for
/// `memory:<subject>` must exist (fail-closed).
pub fn open(subject: &str, label: MemLabel) -> Result<Value, String> {
    if subject.trim().is_empty() {
        return Err("memory_open: subject must be a non-empty string".to_string());
    }
    if label == MemLabel::Private {
        let scope = format!("memory:{}", subject);
        if !crate::consent::active_grant_for(&scope) {
            return Err(format!(
                "memory_open: no active consent for scope '{}' — private containers are consent-gated (MEMORY_CONSENT_REQUIRED); grant it via consent_grant(<value>, \"memory:<subject>\", \"<subject>\")",
                scope
            ));
        }
    }
    let mut reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    // Re-open returns the SAME container (deterministic identity per
    // (subject, label) — the subject IS the address).
    if let Some(existing) = reg
        .values()
        .find(|c| c.subject == subject && c.label == label)
    {
        let value = container_value(existing);
        return Ok(value);
    }
    let c = TypedContainer {
        id: fresh_container_id(subject),
        subject: subject.to_string(),
        label,
        created_unix: unix_now(),
        entries: HashMap::new(),
    };
    ledger_memory_event(
        "open",
        &c.id,
        &format!("subject={}|label={}", subject, label.as_str()),
    );
    let value = container_value(&c);
    reg.insert(c.id.clone(), c);
    Ok(value)
}

/// Normalize a put/read value to its TEXT form (String|Secret|Float|Bool
/// accepted; opaque types refuse — memory stores text, not handles).
pub fn value_to_text(fn_name: &str, v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Secret(zs) => Ok(zs.as_str().to_string()),
        Value::Float(f) => Ok(format!("{}", f)),
        Value::Bool(b) => Ok(format!("{}", b)),
        other => Err(format!(
            "{}: expected String|Secret|Float|Bool as the value, got {} (opaque handles are not memory content)",
            fn_name,
            other.type_name()
        )),
    }
}

/// Put an entry (validated derived-from parents) + `memory.put`.
pub fn put(
    handle_id: &str,
    key: &str,
    text: &str,
    derived_from: Vec<String>,
) -> Result<(), String> {
    let mut reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    let c = reg.get_mut(handle_id).ok_or_else(|| {
        format!(
            "memory_put: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    if key.trim().is_empty() {
        return Err("memory_put: key must be a non-empty string".to_string());
    }
    // Fail-closed provenance: every parent key must already exist.
    for parent in &derived_from {
        if !c.entries.contains_key(parent) {
            return Err(format!(
                "memory_put: derived-from parent '{}' does not exist in container '{}' (MEMORY_UNKNOWN_PARENT)",
                parent, c.id
            ));
        }
    }
    let stored = match c.label {
        MemLabel::Public => Stored::Plain(text.to_string()),
        MemLabel::Private => {
            let key_hex = subject_key_hex(&c.subject);
            Stored::Enc(encrypt_at_rest(text, &key_hex)?)
        }
    };
    let overwrite = c.entries.contains_key(key);
    c.entries.insert(
        key.to_string(),
        TypedEntry {
            stored,
            derived_from,
            created_unix: unix_now(),
        },
    );
    ledger_memory_event(
        "put",
        &c.id,
        &format!(
            "key={}|label={}|overwrite={}|parents={}",
            key,
            c.label.as_str(),
            overwrite,
            c.entries
                .get(key)
                .map(|e| e.derived_from.len())
                .unwrap_or(0)
        ),
    );
    Ok(())
}

/// The audited read: ledger + loud audit line; private returns the
/// plaintext as `Value::Secret` (the lattice/redact contract).
pub fn read(handle_id: &str, key: &str) -> Result<Value, String> {
    let reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_read: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let entry = c.entries.get(key).ok_or_else(|| {
        format!(
            "memory_read: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
            key, c.id
        )
    })?;
    let value = match &entry.stored {
        Stored::Plain(s) => Value::String(s.clone()),
        Stored::Enc(blob) => {
            let key_hex = subject_key_hex(&c.subject);
            Value::Secret(crate::interpreter::SecretString::new(decrypt_at_rest(
                blob, &key_hex,
            )?))
        }
    };
    ledger_memory_event(
        "read",
        &c.id,
        &format!("key={}|label={}", key, c.label.as_str()),
    );
    Ok(value)
}

/// The keys of a container (metadata; audited).
pub fn keys(handle_id: &str) -> Result<Vec<String>, String> {
    let reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_keys: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let mut ks: Vec<String> = c.entries.keys().cloned().collect();
    ks.sort();
    ledger_memory_event("keys", &c.id, &format!("count={}", ks.len()));
    Ok(ks)
}

/// The provenance (derived-from parents) of one entry (audited).
pub fn provenance(handle_id: &str, key: &str) -> Result<Vec<String>, String> {
    let reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_provenance: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let entry = c.entries.get(key).ok_or_else(|| {
        format!(
            "memory_provenance: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
            key, c.id
        )
    })?;
    ledger_memory_event(
        "provenance",
        &c.id,
        &format!("key={}|parents={}", key, entry.derived_from.len()),
    );
    Ok(entry.derived_from.clone())
}

/// The explicit file-export sink. Public entries export as-is; PRIVATE
/// entries REFUSE — redact() is the only legal egress for private
/// content (MEMORY_REDACT_REQUIRED).
pub fn export(handle_id: &str, key: &str) -> Result<(), String> {
    let reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_export: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    if !c.entries.contains_key(key) {
        return Err(format!(
            "memory_export: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
            key, c.id
        ));
    }
    if c.label == MemLabel::Private {
        return Err(format!(
            "memory_export: entry '{}' is private — export the redact() output instead (MEMORY_REDACT_REQUIRED; №326/ADR-0136 is the only egress path for private content)",
            key
        ));
    }
    ledger_memory_event("export", &c.id, &format!("key={}", key));
    Ok(())
}

/// Container label lookup (the export builtin needs the label to decide
/// the redact gate BEFORE reading the plaintext).
pub fn label_of(handle_id: &str) -> Result<MemLabel, String> {
    let reg = registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))?;
    reg.get(handle_id).map(|c| c.label).ok_or_else(|| {
        format!(
            "memory_export: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })
}

/// Introspection for contract tests: is the entry stored ENCRYPTED at
/// rest (private) — without exposing any plaintext.
pub fn entry_is_encrypted(handle_id: &str, key: &str) -> bool {
    lock_registry()
        .map(|reg| {
            reg.get(handle_id)
                .and_then(|c| c.entries.get(key))
                .map(|e| matches!(e.stored, Stored::Enc(_)))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// The subject key material NEVER leaves this module; tests verify the
/// isolation by attempting a cross-decrypt through this gate.
pub fn cross_decrypt_fails(subject: &str, blob_owner_handle: &str, key: &str) -> bool {
    let Ok(reg) = lock_registry() else {
        return true;
    };
    let Some(c) = reg.get(blob_owner_handle) else {
        return true;
    };
    let Some(entry) = c.entries.get(key) else {
        return true;
    };
    match &entry.stored {
        Stored::Plain(_) => false, // public: nothing to isolate
        Stored::Enc(blob) => {
            let wrong_hex = subject_key_hex(subject);
            decrypt_at_rest(blob, &wrong_hex).is_err()
        }
    }
}

/// The entry count introspection used by the tests.
pub fn entry_count(handle_id: &str) -> usize {
    lock_registry()
        .map(|reg| reg.get(handle_id).map(|c| c.entries.len()).unwrap_or(0))
        .unwrap_or(0)
}

fn lock_registry() -> Result<std::sync::MutexGuard<'static, Registry>, String> {
    registry()
        .lock()
        .map_err(|e| format!("memory registry lock: {}", e))
}
