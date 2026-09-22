//! Consent ledger (Наряд №335, spec §7.2 v2) — the storage half of the
//! consent component. Every grant and every revocation is recorded:
//! (subject, scope, TTL, issued_at, expires_at) for grants; revocations
//! in the same table with `kind = 'revoke'` (the LOUD decision of this
//! naryad: one table, two record kinds — a revocation is the same fact
//! class as a grant, and one table keeps export/audit total).
//!
//! Precedent: `consent_ledger` in `src/voice/store.rs` (voiceprint
//! consent) — generalized here to subject/scope/TTL consent over ANY
//! value. The ledger is PROCESS-LOCAL (in-memory SQLite): it is the
//! bookkeeping of what was granted/revoked during the run. Exporting it
//! is FILE EGRESS — `consent_ledger_export` is a classified Sink with an
//! audit event (the №326 "policy as value" audit posture); reading it
//! in-process (`consent_ledger_count`) is not egress.

use std::sync::{Mutex, OnceLock};

/// One ledger row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsentRecord {
    pub id: i64,
    /// "grant" | "revoke".
    pub kind: String,
    /// The consenting subject ("patient-1", …). Empty for revocations.
    pub subject: String,
    /// The consent scope ("gdpr", …) or "<all>" for a total revocation.
    pub scope: String,
    /// TTL of the GRANT in seconds (0 = no expiry). Revocations: 0.
    pub ttl_seconds: u64,
    /// Unix epoch seconds when the record was written.
    pub issued_at: u64,
    /// `issued_at + ttl` for grants with a TTL; `None` = no expiry.
    pub expires_at: Option<u64>,
    pub note: String,
}

fn ledger() -> &'static Mutex<Option<rusqlite::Connection>> {
    static LEDGER: OnceLock<Mutex<Option<rusqlite::Connection>>> = OnceLock::new();
    LEDGER.get_or_init(|| {
        // Non-panicking init: an unavailable ledger degrades to Err on
        // every API call (a loud runtime refusal, never a panic) — the
        // in-memory open can only fail on resource exhaustion.
        match rusqlite::Connection::open_in_memory() {
            Ok(conn) => {
                let ok = conn
                    .execute_batch(
                        "CREATE TABLE IF NOT EXISTS consent_ledger (
                            id INTEGER PRIMARY KEY AUTOINCREMENT,
                            kind TEXT NOT NULL,
                            subject TEXT NOT NULL DEFAULT '',
                            scope TEXT NOT NULL,
                            ttl_seconds INTEGER NOT NULL DEFAULT 0,
                            issued_at INTEGER NOT NULL,
                            expires_at INTEGER,
                            note TEXT NOT NULL DEFAULT ''
                        );",
                    )
                    .is_ok();
                Mutex::new(if ok { Some(conn) } else { None })
            }
            Err(_) => Mutex::new(None),
        }
    })
}

fn with_conn<T>(f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>) -> Result<T, String> {
    let guard = ledger()
        .lock()
        .map_err(|e| format!("consent ledger lock: {}", e))?;
    match guard.as_ref() {
        Some(conn) => f(conn),
        None => {
            Err("consent ledger unavailable (in-memory sqlite failed to initialize)".to_string())
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Record a grant: (subject, scope, ttl_seconds). TTL 0 = no expiry.
pub fn record_grant(subject: &str, scope: &str, ttl_seconds: u64) -> Result<(), String> {
    if scope.trim().is_empty() {
        return Err("consent ledger: scope must be a non-empty consent scope (§7.2)".to_string());
    }
    let now = now_secs() as i64;
    let expires = if ttl_seconds > 0 {
        Some(now + ttl_seconds as i64)
    } else {
        None
    };
    with_conn(|conn| {
        conn.execute(
            "INSERT INTO consent_ledger (kind, subject, scope, ttl_seconds, issued_at, expires_at, note) \
             VALUES ('grant', ?1, ?2, ?3, ?4, ?5, '')",
            rusqlite::params![subject, scope, ttl_seconds as i64, now, expires],
        )
        .map_err(|e| format!("consent ledger insert: {}", e))?;
        Ok(())
    })
}

/// Record a revocation: `Some(scope)` revokes one scope, `None` revokes
/// ALL scopes of the subject chain (the flat-cascade entry point).
pub fn record_revoke(scope: Option<&str>) -> Result<(), String> {
    let now = now_secs() as i64;
    with_conn(|conn| {
        conn.execute(
            "INSERT INTO consent_ledger (kind, subject, scope, ttl_seconds, issued_at, expires_at, note) \
             VALUES ('revoke', '', ?1, 0, ?2, NULL, '')",
            rusqlite::params![scope.unwrap_or("<all>"), now],
        )
        .map_err(|e| format!("consent ledger insert: {}", e))?;
        Ok(())
    })
}

/// Is there an ACTIVE consent grant for `scope`? (Naryad #350 — the
/// typed-memory cross-subject gate reads it; additive reader, the store
/// semantics are untouched.)
///
/// Active = a 'grant' row for the scope that is NEWER than the last
/// 'revoke' row for the same scope and whose TTL has not expired
/// (expires_at NULL = no expiry). "Newer" is ROW ORDER (rowid), not the
/// wall clock: `issued_at` has second granularity, so a grant recorded
/// the same second as the revoke it supersedes must still win (the
/// operation order is the ledger truth — №428 surfaced the flaw through
/// the audio consent gate; the rowid comparison keeps the same rule
/// without the clock-granularity hole). Fail-closed: on any store
/// error the answer is NO.
pub fn active_grant_for(scope: &str) -> bool {
    let now = now_secs() as i64;
    with_conn(|conn| {
        let found: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM consent_ledger \
                 WHERE kind = 'grant' AND scope = ?1 \
                   AND (expires_at IS NULL OR expires_at > ?2) \
                   AND rowid > COALESCE((SELECT MAX(rowid) FROM consent_ledger \
                                         WHERE kind = 'revoke' AND scope = ?1), 0) \
                 LIMIT 1",
                rusqlite::params![scope, now],
                |_row| Ok(1),
            )
            .map(|_| Some(1))
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .unwrap_or(None);
        Ok(found.is_some())
    })
    .unwrap_or(false)
}

/// Number of records (in-process read — not egress).
pub fn entry_count() -> Result<i64, String> {
    with_conn(|conn| {
        conn.query_row("SELECT COUNT(*) FROM consent_ledger", [], |r| r.get(0))
            .map_err(|e| format!("consent ledger count: {}", e))
    })
}

/// Full JSON export (the caller — `consent_ledger_export` — performs the
/// sandboxed write and the audit event; export is FILE EGRESS).
pub fn export_json() -> Result<String, String> {
    with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, subject, scope, ttl_seconds, issued_at, expires_at, note \
                 FROM consent_ledger ORDER BY id",
            )
            .map_err(|e| format!("consent ledger export: {}", e))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ConsentRecord {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    subject: r.get(2)?,
                    scope: r.get(3)?,
                    ttl_seconds: r.get::<_, i64>(4)?.max(0) as u64,
                    issued_at: r.get::<_, i64>(5)?.max(0) as u64,
                    expires_at: r.get::<_, Option<i64>>(6)?.map(|v| v.max(0) as u64),
                    note: r.get(7)?,
                })
            })
            .map_err(|e| format!("consent ledger export: {}", e))?;
        let mut records = Vec::new();
        for r in rows {
            records.push(r.map_err(|e| format!("consent ledger export: {}", e))?);
        }
        serde_json::to_string_pretty(&records).map_err(|e| format!("consent ledger json: {}", e))
    })
}
