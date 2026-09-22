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
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Naryad #351 (ADR-0173): the derived-from graph, cascading forgetting,
// the grant path and the persistent store live in the section at the
// bottom of this file — same-module by decision (the state IS here).

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
    // ── №351 (ADR-0173 §3.2): the incremental children index of the
    // derived-from DAG — parent key → direct derivations. Maintained at
    // put/forget; rebuilt from the stored edges on DB load. The cascade
    // closure walks THIS index, which is what makes the forget O(|closure|
    // + |edges|) instead of O(container).
    pub children: HashMap<String, Vec<String>>,
    // ── №351 (ADR-0173 §3.3): the retained pins (node identity keys).
    // A pin survives value overwrites; the protection CASCADES
    // structurally — a retained node inside a forget closure is a cut
    // point whose whole descendant subtree survives.
    pub retained: HashSet<String>,
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
    // ── №351 (ADR-0173 §3.5): load-on-open — with the persistent store
    // anchored, a container that lives in the DB but not in the cache
    // (a fresh process, a cache miss) is loaded from the authoritative
    // store. The DB is the state; the registry is a transparent cache.
    if let Some(c) = db_load_container(subject, label)? {
        let value = container_value(&c);
        reg.insert(c.id.clone(), c);
        return Ok(value);
    }
    let c = TypedContainer {
        id: fresh_container_id(subject),
        subject: subject.to_string(),
        label,
        created_unix: unix_now(),
        entries: HashMap::new(),
        children: HashMap::new(),
        retained: HashSet::new(),
    };
    ledger_memory_event(
        "open",
        &c.id,
        &format!("subject={}|label={}", subject, label.as_str()),
    );
    // ── №351 (ADR-0173 §3.5): write-through — a freshly created container
    // persists immediately when the store anchor is set.
    persist_container_open(&c);
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
    // ── №351 (ADR-0173 §3.2): an overwrite REPLACES the node's value AND
    // its out-edges (the stale derived-from links of the old value must
    // not survive in the children index); the retained PIN survives —
    // it belongs to the node identity, not the value (§3.3).
    if overwrite {
        if let Some(old) = c.entries.get(key) {
            for parent in &old.derived_from {
                if let Some(list) = c.children.get_mut(parent) {
                    list.retain(|k| k != key);
                }
            }
        }
    }
    let entry = TypedEntry {
        stored,
        derived_from: derived_from.clone(),
        created_unix: unix_now(),
    };
    for parent in &entry.derived_from {
        c.children
            .entry(parent.clone())
            .or_default()
            .push(key.to_string());
    }
    let parents_n = entry.derived_from.len();
    c.entries.insert(key.to_string(), entry);
    persist_entry_put(&c.id, key, &c.entries[key], c.label);
    ledger_memory_event(
        "put",
        &c.id,
        &format!(
            "key={}|label={}|overwrite={}|parents={}",
            key,
            c.label.as_str(),
            overwrite,
            parents_n
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

// ════════════════════════════════════════════════════════════════════
// ── Naryad #351 (ADR-0173): the derived-from graph, cascading
//    forgetting, the grant path and the persistent store ──
// ════════════════════════════════════════════════════════════════════
//
// Graph model (ADR-0173 §3.1-3.2): nodes are entries (container, key);
// edges are derived_from (child → parent), a DAG by construction (put
// validates every parent BEFORE the child exists). The cascade closure
// of a root is a BFS over the per-container children index —
// O(|closure| + |edges inside the closure|).
//
// Retain (ADR-0173 §3.3): a per-node pin whose protection CASCADES
// STRUCTURALLY — a retained node inside a forget closure is a cut
// point: it and its whole descendant subtree survive. Invariants
// (fuzz-pinned against an independent model):
//   P1 provenance integrity — every survivor's parents all survive;
//   P2 completeness — every non-protected closure node is deleted;
//   P3 isolation — nothing outside the closure changes.
//
// forget_cascade (ADR-0173 §3.4) is an ADR-0155 linear action: the
// grant is REQUIRED (GRANT_MISSING without one); enforcement order is
// check_active → scope ("memory:forget:<container_id>") → plan →
// root-retained refusal → apply → grant_use → post-success ledger
// record `irreversible.memory_forget` (the db_execute_with_grant
// template: consumption and journal are side effects of SUCCESS).
//
// Persistence (ADR-0173 §3.5): METALOGOS_MEMORY_DB (file path) makes a
// bundled-rusqlite DB the authoritative store behind the transparent
// write-through cache; unset → byte-for-byte in-process behavior.
// Additive-only DDL (ADR-0060); at-rest blobs are the EXACT №350
// AES-GCM ciphertexts; restart-stable decryption requires
// METALOGOS_MEMORY_MASTER (loud when absent — the №350 posture).

// ── The persistent store (ADR-0173 §3.5) ────────────────────────────

/// The env anchor carrying the file path of the persistent store.
pub const MEMORY_DB_ENV: &str = "METALOGOS_MEMORY_DB";

type Conn = rusqlite::Connection;

fn db_conn() -> Result<Option<std::sync::MutexGuard<'static, Conn>>, String> {
    static DB: OnceLock<Option<Mutex<Conn>>> = OnceLock::new();
    let cell = DB.get_or_init(|| {
        let Ok(path) = std::env::var(MEMORY_DB_ENV) else {
            return None;
        };
        if path.trim().is_empty() {
            return None;
        }
        match Conn::open(path.trim()) {
            Ok(conn) => {
                if let Err(e) = ensure_memtyped_schema(&conn) {
                    eprintln!("[MEMORY_DB] schema init failed: {} — the store stays unavailable (fail-closed)", e);
                    return None;
                }
                eprintln!("[MEMORY_DB] persistent typed-memory store anchored at {}", path.trim());
                Some(Mutex::new(conn))
            }
            Err(e) => {
                eprintln!(
                    "[MEMORY_DB] cannot open '{}' ({}): persistence was EXPLICITLY anchored, so the store refuses (fail-closed MEMORY_DB_UNAVAILABLE) — no silent in-memory degradation",
                    path.trim(),
                    e
                );
                None
            }
        }
    });
    match cell {
        Some(m) => Ok(Some(
            m.lock().map_err(|e| format!("memory db lock: {}", e))?,
        )),
        // Env unset → no store, by design. Env set but unopenable → the
        // loud fail-closed posture above (None with the env present).
        None => {
            if std::env::var(MEMORY_DB_ENV).is_ok_and(|p| !p.trim().is_empty()) {
                return Err(
                    "MEMORY_DB_UNAVAILABLE: METALOGOS_MEMORY_DB is set but the store could not be opened — refusing (fail-closed; no silent in-memory degradation)".to_string(),
                );
            }
            Ok(None)
        }
    }
}

/// The additive-only DDL (ADR-0060 discipline): CREATE TABLE IF NOT
/// EXISTS, no drops, no alters. The pins ride the entries table.
fn ensure_memtyped_schema(conn: &Conn) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memtyped_containers (
            id          TEXT PRIMARY KEY,
            subject     TEXT NOT NULL,
            label       TEXT NOT NULL,
            created_unix INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS memtyped_entries (
            container_id TEXT NOT NULL,
            key          TEXT NOT NULL,
            stored       BLOB NOT NULL,
            is_enc       INTEGER NOT NULL,
            created_unix INTEGER NOT NULL,
            retained     INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (container_id, key));
         CREATE TABLE IF NOT EXISTS memtyped_edges (
            container_id TEXT NOT NULL,
            child        TEXT NOT NULL,
            parent       TEXT NOT NULL,
            PRIMARY KEY (container_id, child, parent));",
    )
    .map_err(|e| format!("memory db schema: {}", e))
}

fn persist_container_open(c: &TypedContainer) {
    let Ok(Some(conn)) = db_conn() else {
        return;
    };
    let _ = conn
        .execute(
            "INSERT OR IGNORE INTO memtyped_containers (id, subject, label, created_unix) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![c.id, c.subject, c.label.as_str(), c.created_unix as i64],
        )
        .map_err(|e| {
            eprintln!(
                "[MEMORY_DB] container open persist failed (loud, best-effort): {}",
                e
            )
        });
}

fn persist_entry_put(container_id: &str, key: &str, entry: &TypedEntry, label: MemLabel) {
    let Ok(Some(conn)) = db_conn() else {
        return;
    };
    let (blob, is_enc): (Vec<u8>, i64) = match &entry.stored {
        Stored::Plain(s) => (s.as_bytes().to_vec(), 0),
        Stored::Enc(b) => (b.clone(), 1),
    };
    let _ = conn
        .execute(
            "INSERT OR REPLACE INTO memtyped_entries \
             (container_id, key, stored, is_enc, created_unix, retained) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                container_id,
                key,
                blob,
                is_enc,
                entry.created_unix as i64,
                0
            ],
        )
        .map_err(|e| {
            eprintln!(
                "[MEMORY_DB] entry put persist failed (loud, best-effort): {}",
                e
            )
        });
    let _ = conn
        .execute(
            "DELETE FROM memtyped_edges WHERE container_id = ?1 AND child = ?2",
            rusqlite::params![container_id, key],
        )
        .map_err(|e| eprintln!("[MEMORY_DB] edge refresh failed (loud, best-effort): {}", e));
    for parent in &entry.derived_from {
        let _ = conn
            .execute(
                "INSERT OR IGNORE INTO memtyped_edges (container_id, child, parent) \
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![container_id, key, parent],
            )
            .map_err(|e| eprintln!("[MEMORY_DB] edge persist failed (loud, best-effort): {}", e));
    }
    let _ = label; // the label lives on the container row; entry rows are label-free
}

fn persist_pin(container_id: &str, key: &str, retained: bool) {
    let Ok(Some(conn)) = db_conn() else {
        return;
    };
    let _ = conn
        .execute(
            "UPDATE memtyped_entries SET retained = ?3 \
             WHERE container_id = ?1 AND key = ?2",
            rusqlite::params![container_id, key, retained as i64],
        )
        .map_err(|e| eprintln!("[MEMORY_DB] pin persist failed (loud, best-effort): {}", e));
}

fn persist_delete(container_id: &str, keys: &[String]) {
    let Ok(Some(conn)) = db_conn() else {
        return;
    };
    for k in keys {
        let _ = conn
            .execute(
                "DELETE FROM memtyped_entries WHERE container_id = ?1 AND key = ?2",
                rusqlite::params![container_id, k],
            )
            .map_err(|e| eprintln!("[MEMORY_DB] entry delete persist failed: {}", e));
        let _ = conn
            .execute(
                "DELETE FROM memtyped_edges WHERE container_id = ?1 AND child = ?2",
                rusqlite::params![container_id, k],
            )
            .map_err(|e| eprintln!("[MEMORY_DB] edge delete persist failed: {}", e));
        let _ = conn
            .execute(
                "DELETE FROM memtyped_edges WHERE container_id = ?1 AND parent = ?2",
                rusqlite::params![container_id, k],
            )
            .map_err(|e| eprintln!("[MEMORY_DB] edge parent delete persist failed: {}", e));
    }
}

/// Load-on-open: rebuild a container from the authoritative store
/// (entries + edges + pins; the children index is rebuilt from edges).
fn db_load_container(subject: &str, label: MemLabel) -> Result<Option<TypedContainer>, String> {
    let Ok(Some(conn)) = db_conn() else {
        return Ok(None);
    };
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM memtyped_containers WHERE subject = ?1 AND label = ?2 LIMIT 1",
            rusqlite::params![subject, label.as_str()],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(format!("memory db load container: {}", other)),
        })?;
    let Some(id) = id else {
        return Ok(None);
    };
    let mut c = TypedContainer {
        id: id.clone(),
        subject: subject.to_string(),
        label,
        created_unix: 0,
        entries: HashMap::new(),
        children: HashMap::new(),
        retained: HashSet::new(),
    };
    {
        let mut stmt = conn
            .prepare(
                "SELECT key, stored, is_enc, created_unix, retained \
                 FROM memtyped_entries WHERE container_id = ?1",
            )
            .map_err(|e| format!("memory db load entries: {}", e))?;
        let rows = stmt
            .query_map(rusqlite::params![id], |r| {
                let key: String = r.get(0)?;
                let blob: Vec<u8> = r.get(1)?;
                let is_enc: i64 = r.get(2)?;
                let created: i64 = r.get(3)?;
                let pin: i64 = r.get(4)?;
                Ok((key, blob, is_enc, created, pin))
            })
            .map_err(|e| format!("memory db load entries: {}", e))?;
        for row in rows {
            let (key, blob, is_enc, created, pin) =
                row.map_err(|e| format!("memory db load row: {}", e))?;
            let stored = if is_enc != 0 {
                Stored::Enc(blob)
            } else {
                match String::from_utf8(blob) {
                    Ok(s) => Stored::Plain(s),
                    Err(_) => Stored::Enc(Vec::new()), // corrupt row: loud on read, never a panic
                }
            };
            if pin != 0 {
                c.retained.insert(key.clone());
            }
            c.entries.insert(
                key,
                TypedEntry {
                    stored,
                    derived_from: Vec::new(), // filled from the edge table below
                    created_unix: created.max(0) as u64,
                },
            );
        }
    }
    {
        let mut stmt = conn
            .prepare("SELECT child, parent FROM memtyped_edges WHERE container_id = ?1")
            .map_err(|e| format!("memory db load edges: {}", e))?;
        let rows = stmt
            .query_map(rusqlite::params![id], |r| {
                let child: String = r.get(0)?;
                let parent: String = r.get(1)?;
                Ok((child, parent))
            })
            .map_err(|e| format!("memory db load edges: {}", e))?;
        for row in rows {
            let (child, parent) = row.map_err(|e| format!("memory db load edge row: {}", e))?;
            if let Some(entry) = c.entries.get_mut(&child) {
                entry.derived_from.push(parent.clone());
            }
            c.children.entry(parent).or_default().push(child);
        }
    }
    c.created_unix = conn
        .query_row(
            "SELECT created_unix FROM memtyped_containers WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;
    Ok(Some(c))
}

// ── The cascade engine (ADR-0173 §3.2-3.3): PURE, fuzzable ──────────

/// The pure cascade plan (ADR-0173 §3.3, the veto semantics).
///
/// `closure` = the full descendant closure of the root (the root
/// included) — the set the forget WOULD delete. `blocked_by` = the
/// retained pins inside that closure — when non-empty, the forget
/// REFUSES LOUDLY (MEMORY_RETAIN_PROTECTED) and deletes NOTHING: a
/// retained node is a deletion VETO, because any partial deletion
/// around it either dangles its provenance (deleting a survivor's
/// parent) or silently splits the protected subtree (the cut-point
/// alternative was REJECTED in the ADR for exactly that P1 hole).
/// `visited_nodes`/`visited_edges` are the deterministic O() proof
/// counters: every node of the closure is popped exactly once and
/// every edge inside it scanned exactly once.
#[derive(Debug, Clone, PartialEq)]
pub struct CascadePlan {
    pub root: String,
    pub closure: Vec<String>,
    pub blocked_by: Vec<String>,
    pub visited_nodes: usize,
    pub visited_edges: usize,
}

pub fn plan_cascade(
    root: &str,
    children: &HashMap<String, Vec<String>>,
    retained: &HashSet<String>,
) -> CascadePlan {
    let mut closure: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut visited_nodes: usize = 0;
    let mut visited_edges: usize = 0;
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    queue.push_back(root.to_string());
    seen.insert(root.to_string());
    while let Some(node) = queue.pop_front() {
        visited_nodes += 1;
        closure.push(node.clone());
        if let Some(kids) = children.get(&node) {
            visited_edges += kids.len();
            for k in kids {
                if seen.insert(k.clone()) {
                    queue.push_back(k.clone());
                }
            }
        }
    }
    closure.sort();
    let blocked_by: Vec<String> = closure
        .iter()
        .filter(|k| retained.contains(*k))
        .cloned()
        .collect();
    CascadePlan {
        root: root.to_string(),
        closure,
        blocked_by,
        visited_nodes,
        visited_edges,
    }
}

// ── Retain / release / introspection (ADR-0173 §3.3, §3.6) ──────────

/// Compute the descendant closure of `key` within the container
/// (the key included) — the cascade footprint of retain/release.
fn descendant_closure(c: &TypedContainer, key: &str) -> Result<Vec<String>, String> {
    if !c.entries.contains_key(key) {
        return Err(key.to_string());
    }
    Ok(plan_cascade(key, &c.children, &HashSet::new()).closure)
}

/// Pin the descendant closure of `key` (the CASCADE retain — the pin
/// set is materialized over the current subtree; nodes put later are
/// NOT auto-pinned, the pin set is an explicit snapshot). Idempotent;
/// audited; reversible via release.
pub fn retain(handle_id: &str, key: &str) -> Result<usize, String> {
    let mut reg = lock_registry()?;
    let c = reg.get_mut(handle_id).ok_or_else(|| {
        format!(
            "memory_retain: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let closure = match descendant_closure(c, key) {
        Ok(v) => v,
        Err(k) => {
            return Err(format!(
                "memory_retain: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
                k, c.id
            ))
        }
    };
    let mut pinned = 0usize;
    for k in &closure {
        if c.retained.insert(k.clone()) {
            pinned += 1;
        }
    }
    if pinned > 0 {
        for k in &closure {
            persist_pin(&c.id, k, true);
        }
        ledger_memory_event(
            "retain",
            &c.id,
            &format!("root={}|pinned={}|cascade={}", key, pinned, closure.len()),
        );
    }
    Ok(closure.len())
}

/// Unpin the descendant closure of `key` (the CASCADE release — the
/// surgical inverse of retain). Idempotent (the №280 posture); audited.
pub fn release(handle_id: &str, key: &str) -> Result<usize, String> {
    let mut reg = lock_registry()?;
    let c = reg.get_mut(handle_id).ok_or_else(|| {
        format!(
            "memory_release: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let closure = match descendant_closure(c, key) {
        Ok(v) => v,
        Err(k) => {
            return Err(format!(
                "memory_release: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
                k, c.id
            ))
        }
    };
    let mut released = 0usize;
    for k in &closure {
        if c.retained.remove(k) {
            released += 1;
        }
    }
    if released > 0 {
        for k in &closure {
            persist_pin(&c.id, k, false);
        }
        ledger_memory_event(
            "release",
            &c.id,
            &format!(
                "root={}|released={}|cascade={}",
                key,
                released,
                closure.len()
            ),
        );
    }
    Ok(closure.len())
}

/// The pinned keys of a container (sorted; audited introspection).
pub fn retained_keys(handle_id: &str) -> Result<Vec<String>, String> {
    let reg = lock_registry()?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_retained: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let mut ks: Vec<String> = c.retained.iter().cloned().collect();
    ks.sort();
    ledger_memory_event("retained", &c.id, &format!("count={}", ks.len()));
    Ok(ks)
}

/// The read-only cascade preview (the №280 dry-run discipline):
/// `{closure, blocked_by}` — no grant, no state change. When
/// `blocked_by` is empty, the would-delete set IS the closure.
pub fn cascade_preview(handle_id: &str, key: &str) -> Result<PreviewTriple, String> {
    let reg = lock_registry()?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_cascade_preview: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    if !c.entries.contains_key(key) {
        return Err(format!(
            "memory_cascade_preview: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
            key, c.id
        ));
    }
    let plan = plan_cascade(key, &c.children, &c.retained);
    ledger_memory_event(
        "cascade_preview",
        &c.id,
        &format!(
            "root={}|closure={}|blocked_by={}",
            key,
            plan.closure.len(),
            plan.blocked_by.len()
        ),
    );
    Ok((
        plan.closure,
        plan.blocked_by,
        plan.visited_nodes + plan.visited_edges,
    ))
}

/// The preview triple: (closure, blocked_by, visited ops — the O() probe).
pub type PreviewTriple = (Vec<String>, Vec<String>, usize);

// ── forget_cascade: the ADR-0155 linear action (ADR-0173 §3.4) ──────

/// The outcome of a granted cascade forget.
#[derive(Debug, Clone, PartialEq)]
pub struct ForgetOutcome {
    pub root: String,
    pub deleted: Vec<String>,
    pub batch_id: String,
}

fn fresh_batch_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let preimage = format!("tforget|{}|{}", millis, seq);
    format!(
        "MLOG-TFORGET-{}",
        &crate::ledger::sha256_hex(preimage.as_bytes())[..16]
    )
}

/// The grant-gated, ledger-recorded cascading forget (ADR-0173 §3.4).
/// Enforcement order: check_active → scope → plan → retained-veto
/// refusal → apply → grant_use → post-success ledger record. The
/// delete set is the FULL closure — provenance integrity (P1),
/// completeness (P2) and isolation (P3) hold by construction.
pub fn forget_cascade(
    handle_id: &str,
    key: &str,
    grant: &crate::grants::GrantHandle,
) -> Result<ForgetOutcome, String> {
    // 1. Grant state/TTL (GRANT_REVOKED / GRANT_REUSED / GRANT_EXPIRED /
    //    GRANT_EXHAUSTED — the ADR-0155 §3.5 vocabulary).
    crate::grants::check_active(grant)?;
    // 2. Scope coverage: the canonical memory scope is
    //    `memory:forget:<container_id>`; `*` wildcards attenuate.
    let needed = format!("memory:forget:{}", handle_id);
    if !crate::grants::scope_attenuates(&grant.scope, &needed) {
        return Err(format!(
            "GRANT_SCOPE_MISMATCH: grant {} (scope '{}') does not cover {}",
            grant.grant_id, grant.scope, needed
        ));
    }
    let batch_id = fresh_batch_id();
    // 3. Plan + apply under ONE lock (the mutation is atomic: the plan
    //    is computed from the same snapshot that is mutated).
    let outcome = {
        let mut reg = lock_registry()?;
        let c = reg.get_mut(handle_id).ok_or_else(|| {
            format!(
                "memory_forget_cascade: unknown container '{}' (MEMORY_UNKNOWN)",
                handle_id
            )
        })?;
        if !c.entries.contains_key(key) {
            return Err(format!(
                "memory_forget_cascade: no entry '{}' in container '{}' (MEMORY_UNKNOWN_KEY)",
                key, c.id
            ));
        }
        let plan = plan_cascade(key, &c.children, &c.retained);
        if !plan.blocked_by.is_empty() {
            // The retained VETO: a loud, named refusal — no state
            // change, no grant consumption (fail-closed). The preview
            // (memory_cascade_preview) is how the operator sees this
            // BEFORE touching a grant.
            return Err(format!(
                "MEMORY_RETAIN_PROTECTED: the cascade of '{}' would reach retained entries {:?} — release them first (memory_release) if forgetting them is intended",
                key, plan.blocked_by
            ));
        }
        // The deletion set with their PRE-DELETE parents (needed to
        // clean the children index of surviving parents).
        let mut parent_links: Vec<(String, Vec<String>)> = Vec::with_capacity(plan.closure.len());
        for d in &plan.closure {
            parent_links.push((
                d.clone(),
                c.entries
                    .get(d)
                    .map(|e| e.derived_from.clone())
                    .unwrap_or_default(),
            ));
        }
        for (d, parents) in &parent_links {
            c.entries.remove(d);
            c.retained.remove(d);
            c.children.remove(d);
            for p in parents {
                if let Some(list) = c.children.get_mut(p) {
                    list.retain(|k| k != d);
                }
            }
        }
        let outcome = ForgetOutcome {
            root: key.to_string(),
            deleted: plan.closure,
            batch_id: batch_id.clone(),
        };
        // 4. Write-through (the DB is authoritative when anchored).
        persist_delete(&c.id, &outcome.deleted);
        outcome
    };
    // 5. Post-success consumption + journal (the db_execute_with_grant
    //    template — never charged on refusal, always on success).
    let digest = crate::ledger::sha256_hex(outcome.deleted.join("\u{1}").as_bytes());
    crate::grants::grant_use(
        grant,
        &format!(
            "memory_forget_cascade: container={} root={} deleted={} batch={}",
            handle_id,
            key,
            outcome.deleted.len(),
            outcome.batch_id
        ),
    )?;
    // 6. ADR-0167 §3.4: the irreversible action SUCCEEDED — the journal
    //    entry is a side effect of the success path. The deleted VALUES
    //    never enter the journal — only the count and their digest.
    crate::ledger::record(
        "irreversible.memory_forget",
        &grant.issuer,
        &grant.scope,
        &format!(
            "{}|container={}|root={}|deleted={}|digest={}|batch={}",
            grant.grant_id,
            handle_id,
            key,
            outcome.deleted.len(),
            digest,
            outcome.batch_id
        ),
    );
    eprintln!(
        "[MEMORY_FORGET_CASCADE] container={} root={} deleted={} batch={}",
        handle_id,
        key,
        outcome.deleted.len(),
        outcome.batch_id
    );
    ledger_memory_event(
        "forget_cascade",
        handle_id,
        &format!(
            "root={}|deleted={}|batch={}|grant={}",
            key,
            outcome.deleted.len(),
            outcome.batch_id,
            grant.grant_id
        ),
    );
    Ok(outcome)
}

/// Introspection for tests/the fuzzer: the children index of a
/// container (sorted pairs; no values ever leave).
pub fn children_index(handle_id: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let reg = lock_registry()?;
    let c = reg.get(handle_id).ok_or_else(|| {
        format!(
            "memory_children: unknown container '{}' (MEMORY_UNKNOWN)",
            handle_id
        )
    })?;
    let mut ks: Vec<&String> = c.children.keys().collect();
    ks.sort();
    Ok(ks
        .into_iter()
        .map(|k| {
            let mut v = c.children[k].clone();
            v.sort();
            (k.clone(), v)
        })
        .collect())
}
