// ── Naryad #390 (P0, security/action): Grant value — ADR-0155 §3 ──────
//
// The language-level Grant capability: opaque `Value::Grant` handles over
// an in-process SQLite ledger (the `consent_ledger` template,
// `src/consent.rs:47-137`). The ledger is the SSOT for quota/revocation
// state; handles are untrusted caches of it.
//
// Algebra (ADR-0155):
//   - classes: Once (statically linear, one use), N(n) (runtime quota),
//     Unlimited (copyable, every use audited);
//   - subgrant is attenuation-only: scope narrows, TTL shortens, class
//     power only decreases (Unlimited > N > Once);
//   - revoke is cascading (the №335 consent-cascade precedent);
//   - fail-closed: a grant that is missing/revoked/consumed/expired/
//     exhausted refuses with a typed error — never a panic.
//
// Typed error vocabulary (ADR-0155 §3.5): GRANT_MISSING, GRANT_REUSED,
// GRANT_EXHAUSTED, GRANT_EXPIRED, GRANT_REVOKED, GRANT_ESCALATION,
// GRANT_SCOPE_MISMATCH. The audit-side deny for ungranted destructive
// SQL keeps its historical class name IRREVERSIBLE_NO_GRANT (№325).
//
// Signing (prev-hash + Ed25519, in-toto/PROV profile) is naryad #393:
// it upgrades the INTEGRITY of this ledger, not its semantics.

use crate::interpreter::values::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// Grant class (ADR-0155 §3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantClass {
    /// Statically linear: exactly one use, then consumed.
    Once,
    /// Runtime quota: n uses, metered in the ledger.
    N(u64),
    /// Copyable; every use is audited.
    Unlimited,
}

impl GrantClass {
    /// Parse the DSL-level class word: "once" | "n" | "unlimited".
    /// The N(n) count comes as a separate `uses` argument.
    pub fn parse(word: &str) -> Result<GrantClass, String> {
        match word.trim().to_ascii_lowercase().as_str() {
            "once" => Ok(GrantClass::Once),
            "n" => Ok(GrantClass::N(0)), // count patched by the caller
            "unlimited" => Ok(GrantClass::Unlimited),
            other => Err(format!(
                "grant_issue: unknown class '{}' (expected \"once\", \"n\" or \"unlimited\")",
                other
            )),
        }
    }

    /// Monotone power rank for the attenuation law (§3.3 rule 4):
    /// child class power must be <= parent class power.
    pub fn power(&self) -> u8 {
        match self {
            GrantClass::Once => 0,
            GrantClass::N(_) => 1,
            GrantClass::Unlimited => 2,
        }
    }

    /// Canonical ledger form: "once" | "n:<k>" | "unlimited".
    pub fn as_ledger_str(&self) -> String {
        match self {
            GrantClass::Once => "once".to_string(),
            GrantClass::N(k) => format!("n:{}", k),
            GrantClass::Unlimited => "unlimited".to_string(),
        }
    }

    /// Parse back the canonical ledger form.
    pub fn from_ledger_str(s: &str) -> GrantClass {
        if let Some(k) = s.strip_prefix("n:") {
            GrantClass::N(k.parse().unwrap_or(0))
        } else {
            match s {
                "unlimited" => GrantClass::Unlimited,
                _ => GrantClass::Once,
            }
        }
    }

    /// Initial `remaining` value for the ledger row (-1 = unlimited).
    fn initial_remaining(&self) -> i64 {
        match self {
            GrantClass::Once => 1,
            GrantClass::N(k) => *k as i64,
            GrantClass::Unlimited => -1,
        }
    }
}

impl std::fmt::Display for GrantClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrantClass::Once => write!(f, "once"),
            GrantClass::N(k) => write!(f, "n({})", k),
            GrantClass::Unlimited => write!(f, "unlimited"),
        }
    }
}

/// Opaque Grant handle carried in `Value::Grant` (ADR-0155 §3.1).
///
/// The handle is an untrusted CACHE of the ledger record (identity +
/// descriptors). All authority checks read the ledger at use time.
#[derive(Debug, Clone)]
pub struct GrantHandle {
    pub grant_id: String,
    pub scope: String,
    pub class: GrantClass,
    /// Unix seconds; a use past this instant fails with GRANT_EXPIRED.
    pub expires_at: u64,
    pub issuer: String,
}

// serde mirrors the SecretString posture (values.rs): the ACTUAL capability
// never crosses a serialization boundary — only a safe dead marker.
impl serde::Serialize for GrantHandle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("[GRANT]")
    }
}

impl<'de> serde::Deserialize<'de> for GrantHandle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?;
        // A deserialized grant is a tombstone: expired on arrival, so every
        // use refuses with GRANT_EXPIRED (serialization cannot revive power).
        Ok(GrantHandle {
            grant_id: "deserialized".to_string(),
            scope: String::new(),
            class: GrantClass::Once,
            expires_at: 0,
            issuer: String::new(),
        })
    }
}

impl GrantHandle {
    /// Build the `Value::Grant` variant.
    pub fn to_value(&self) -> Value {
        Value::Grant(self.clone())
    }

    /// Safe one-line descriptor for logs/errors — the SCOPE IS NOT SECRET
    /// metadata in the deny-message sense, but the handle itself is
    /// non-printable (rule 2); only these typed error paths show it.
    pub fn describe(&self) -> String {
        format!(
            "grant {} ({}, scope '{}')",
            self.grant_id, self.class, self.scope
        )
    }
}

static GRANT_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_grant_id() -> String {
    let seq = GRANT_SEQ.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("g-{:x}-{:x}", nanos, seq)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Ledger (the consent_ledger template, src/consent.rs) ───────────────

fn ledger() -> &'static Mutex<Option<rusqlite::Connection>> {
    static LEDGER: OnceLock<Mutex<Option<rusqlite::Connection>>> = OnceLock::new();
    LEDGER.get_or_init(|| {
        let ok = rusqlite::Connection::open_in_memory()
            .and_then(|conn| {
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS grant_state (
                        id TEXT PRIMARY KEY,
                        parent_id TEXT,
                        scope TEXT NOT NULL,
                        class TEXT NOT NULL,
                        remaining INTEGER NOT NULL DEFAULT -1,
                        issued_at INTEGER NOT NULL,
                        expires_at INTEGER NOT NULL,
                        issuer TEXT NOT NULL DEFAULT '',
                        state TEXT NOT NULL DEFAULT 'active'
                    );
                    CREATE TABLE IF NOT EXISTS grant_events (
                        seq INTEGER PRIMARY KEY AUTOINCREMENT,
                        kind TEXT NOT NULL,
                        grant_id TEXT NOT NULL,
                        scope TEXT NOT NULL DEFAULT '',
                        class TEXT NOT NULL DEFAULT '',
                        remaining_after INTEGER NOT NULL DEFAULT -1,
                        note TEXT NOT NULL DEFAULT '',
                        ts INTEGER NOT NULL
                    );",
                )
                .map(|_| conn)
            })
            .ok();
        Mutex::new(ok)
    })
}

fn with_conn<T>(f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>) -> Result<T, String> {
    let guard = ledger()
        .lock()
        .map_err(|e| format!("grant ledger lock: {}", e))?;
    match guard.as_ref() {
        Some(conn) => f(conn),
        None => Err("grant ledger unavailable (in-memory sqlite failed to initialize)".to_string()),
    }
}

fn record_event(
    conn: &rusqlite::Connection,
    kind: &str,
    grant_id: &str,
    scope: &str,
    class: &str,
    remaining_after: i64,
    note: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO grant_events (kind, grant_id, scope, class, remaining_after, note, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            kind,
            grant_id,
            scope,
            class,
            remaining_after,
            note,
            now_secs() as i64
        ],
    )
    .map_err(|e| format!("grant ledger event write: {}", e))?;
    // ── Naryad #393 (ADR-0167 §3.4): every grant lifecycle event lands in
    // the Action Ledger as a SIDE EFFECT of the grant operation itself —
    // this call site IS the action's own bookkeeping path, there is no
    // separate "also log it" step to forget. Best-effort (ADR-0167 §2
    // driver 5): a ledger failure is loud on stderr, never flips the
    // grant operation's outcome. The args preimage never enters the
    // journal — only its SHA-256 (the confidentiality rule).
    crate::ledger::record(
        &format!("grant.{}", kind),
        note,
        scope,
        &format!(
            "{}|{}|{}|{}|{}",
            grant_id, kind, scope, class, remaining_after
        ),
    );
    Ok(())
}

/// A ledger row snapshot (the authoritative state).
#[derive(Debug, Clone)]
pub struct GrantRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub scope: String,
    pub class: GrantClass,
    pub remaining: i64,
    pub expires_at: u64,
    pub state: String, // active | consumed | revoked
}

fn load_record(conn: &rusqlite::Connection, id: &str) -> Result<GrantRecord, String> {
    conn.query_row(
        "SELECT id, parent_id, scope, class, remaining, expires_at, state
         FROM grant_state WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(GrantRecord {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                scope: row.get(2)?,
                class: GrantClass::from_ledger_str(&row.get::<_, String>(3)?),
                remaining: row.get(4)?,
                expires_at: row.get::<_, i64>(5)? as u64,
                state: row.get(6)?,
            })
        },
    )
    .map_err(|_| format!("GRANT_MISSING: no grant '{}' in the ledger", id))
}

/// The lifecycle check every use starts with (ADR-0155 §3.3 rules 1/3/5).
fn check_record(rec: &GrantRecord) -> Result<(), String> {
    match rec.state.as_str() {
        "revoked" => {
            return Err(format!("GRANT_REVOKED: {} was revoked", rec.id));
        }
        "consumed" => {
            return Err(format!(
                "GRANT_REUSED: {} (class once) was already used — a Once grant is linear",
                rec.id
            ));
        }
        _ => {}
    }
    if now_secs() >= rec.expires_at {
        return Err(format!(
            "GRANT_EXPIRED: {} expired at unix {}",
            rec.id, rec.expires_at
        ));
    }
    if rec.remaining == 0 {
        return Err(format!(
            "GRANT_EXHAUSTED: {} (class {}) has no uses left",
            rec.id, rec.class
        ));
    }
    Ok(())
}

// ── Scope algebra ──────────────────────────────────────────────────────

/// Scope vocabulary: colon-hierarchies with `*` wildcards, e.g.
/// `db:delete:users`, `db:delete:*`, `db:*:*`. A child scope attenuates a
/// parent scope iff every segment is equal to the parent's segment or the
/// parent's segment is `*` (more child segments than parent segments are a
/// widening — refused).
pub fn scope_attenuates(parent: &str, child: &str) -> bool {
    let p: Vec<&str> = parent.split(':').collect();
    let c: Vec<&str> = child.split(':').collect();
    if c.len() > p.len() {
        return false;
    }
    p.iter()
        .zip(c.iter())
        .all(|(ps, cs)| *ps == "*" || ps.eq_ignore_ascii_case(cs))
}

/// Does `grant_scope` authorize one destructive op (op, table)?
/// The canonical action scope is `db:<op>:<table>` (ADR-0155 §3.1);
/// table case is normalized (SQLite identifiers are case-insensitive).
pub fn scope_covers(grant_scope: &str, op: &str, table: &str) -> bool {
    scope_attenuates(grant_scope, &format!("db:{}:{}", op, table.to_lowercase()))
}

/// Extract destructive operations from a SQL literal — the same population
/// the `IRREVERSIBLE_NO_GRANT` class denies (DROP/DELETE/TRUNCATE/ALTER).
/// Returns (op, table) pairs; best-effort lexical scan, uppercase-insensitive.
pub fn extract_destructive_ops(sql: &str) -> Vec<(String, String)> {
    let mut ops = Vec::new();
    let up = sql.to_ascii_uppercase();
    let scan = |prefix: &str, op: &str, ops: &mut Vec<(String, String)>| {
        let mut rest: &str = &up;
        while let Some(pos) = rest.find(prefix) {
            let tail = &rest[pos + prefix.len()..];
            let word: String = tail
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '"')
                .collect();
            let table = word
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
            if !table.is_empty() {
                ops.push((op.to_string(), table));
            }
            rest = tail;
        }
    };
    scan("DROP TABLE ", "drop", &mut ops);
    scan("DELETE FROM ", "delete", &mut ops);
    scan("TRUNCATE ", "truncate", &mut ops);
    scan("ALTER TABLE ", "alter", &mut ops);
    ops
}

// ── Lifecycle operations ───────────────────────────────────────────────

/// `grant_issue` core: mint a fresh grant (ADR-0155 §3.1). `ttl_secs == 0`
/// produces an already-expired grant (deterministic expiry tests; the DSL
/// surface warns in its docs to use ttl >= 1).
pub fn issue(
    scope: &str,
    ttl_secs: u64,
    class: &GrantClass,
    issuer: &str,
) -> Result<GrantHandle, String> {
    if scope.trim().is_empty() {
        return Err("grant_issue: scope must be a non-empty string".to_string());
    }
    let id = next_grant_id();
    let expires_at = now_secs() + ttl_secs;
    with_conn(|conn| {
        conn.execute(
            "INSERT INTO grant_state (id, parent_id, scope, class, remaining, issued_at, expires_at, issuer, state)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, 'active')",
            rusqlite::params![
                id,
                scope,
                class.as_ledger_str(),
                class.initial_remaining(),
                now_secs() as i64,
                expires_at as i64,
                issuer
            ],
        )
        .map_err(|e| format!("grant ledger write: {}", e))?;
        record_event(
            conn,
            "issued",
            &id,
            scope,
            &class.as_ledger_str(),
            class.initial_remaining(),
            issuer,
        )?;
        Ok(())
    })?;
    Ok(GrantHandle {
        grant_id: id,
        scope: scope.to_string(),
        class: class.clone(),
        expires_at,
        issuer: issuer.to_string(),
    })
}

/// `grant_subgrant` core: attenuation-only derivation (§3.3 rule 4).
/// Subgranting a Once parent CONSUMES the parent (linear transfer) — an
/// unbounded chain of Once children from one parent would be amplification.
/// An N(n) parent's remaining is debited by the child's quota at split time
/// (conservation: parent remaining + carved-out child quotas <= issued n).
pub fn subgrant(
    parent: &GrantHandle,
    scope: &str,
    ttl_secs: u64,
    class: &GrantClass,
) -> Result<GrantHandle, String> {
    if scope.trim().is_empty() {
        return Err("grant_subgrant: scope must be a non-empty string".to_string());
    }
    let parent_id = parent.grant_id.clone();
    let child_id = next_grant_id();
    with_conn(|conn| {
        let prec = load_record(conn, &parent_id)?;
        check_record(&prec)?;
        // Attenuation law — every widening is GRANT_ESCALATION.
        if !scope_attenuates(&prec.scope, scope) {
            return Err(format!(
                "GRANT_ESCALATION: scope '{}' does not attenuate parent scope '{}' of {}",
                scope, prec.scope, parent_id
            ));
        }
        let child_expires = now_secs() + ttl_secs;
        if child_expires > prec.expires_at {
            return Err(format!(
                "GRANT_ESCALATION: child TTL ({}s) would outlive parent {} (expires at unix {})",
                ttl_secs, parent_id, prec.expires_at
            ));
        }
        if class.power() > prec.class.power() {
            return Err(format!(
                "GRANT_ESCALATION: child class {} outranks parent class {} of {}",
                class, prec.class, parent_id
            ));
        }
        // Quota conservation (the amplification oracle): every child use
        // must come out of the parent's budget. Unlimited children are only
        // derivable from Unlimited parents; Once children debit 1; N(k)
        // children debit k. An Unlimited parent is never debited (-1).
        let debit: i64 = match class {
            GrantClass::Unlimited => {
                if prec.class != GrantClass::Unlimited {
                    return Err(format!(
                        "GRANT_ESCALATION: child class Unlimited outranks parent class {} of {}",
                        prec.class, parent_id
                    ));
                }
                0
            }
            GrantClass::Once => 1,
            GrantClass::N(k) => *k as i64,
        };
        let parent_remaining_after = if prec.class == GrantClass::Unlimited {
            prec.remaining
        } else {
            if debit > prec.remaining {
                return Err(format!(
                    "GRANT_ESCALATION: child debit {} exceeds parent {} remaining {}",
                    debit, parent_id, prec.remaining
                ));
            }
            prec.remaining - debit
        };
        // Once parent: subgranting consumes it (linear transfer).
        if prec.class == GrantClass::Once {
            conn.execute(
                "UPDATE grant_state SET state='consumed', remaining=0 WHERE id=?1",
                rusqlite::params![parent_id],
            )
            .map_err(|e| format!("grant ledger write: {}", e))?;
            record_event(
                conn,
                "consumed",
                &parent_id,
                &prec.scope,
                "once",
                0,
                "subgranted away (linear transfer)",
            )?;
        } else if prec.class != GrantClass::Unlimited {
            conn.execute(
                "UPDATE grant_state SET remaining=?2 WHERE id=?1",
                rusqlite::params![parent_id, parent_remaining_after],
            )
            .map_err(|e| format!("grant ledger write: {}", e))?;
            record_event(
                conn,
                "subgranted",
                &parent_id,
                &prec.scope,
                &prec.class.as_ledger_str(),
                parent_remaining_after,
                &format!("debited {} for child", debit),
            )?;
        }
        conn.execute(
            "INSERT INTO grant_state (id, parent_id, scope, class, remaining, issued_at, expires_at, issuer, state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active')",
            rusqlite::params![
                child_id,
                parent_id,
                scope,
                class.as_ledger_str(),
                class.initial_remaining(),
                now_secs() as i64,
                child_expires as i64,
                parent.issuer.clone(),
            ],
        )
        .map_err(|e| format!("grant ledger write: {}", e))?;
        record_event(
            conn,
            "subgranted",
            &child_id,
            scope,
            &class.as_ledger_str(),
            class.initial_remaining(),
            &format!("child of {}", parent_id),
        )?;
        Ok(())
    })?;
    Ok(GrantHandle {
        grant_id: child_id,
        scope: scope.to_string(),
        class: class.clone(),
        expires_at: now_secs() + ttl_secs,
        issuer: parent.issuer.clone(),
    })
}

/// `grant_revoke` core: cascading revocation (§3.3 rule 5). Returns the
/// number of grants transitioned to `revoked` (target + all descendants).
/// Revoking an already-revoked grant is legal and revokes 0 additional rows.
pub fn revoke(handle: &GrantHandle, note: &str) -> Result<usize, String> {
    let root = handle.grant_id.clone();
    with_conn(|conn| {
        let _ = load_record(conn, &root)?; // GRANT_MISSING if unknown
                                           // BFS over the parent_id tree.
        let mut frontier = vec![root.clone()];
        let mut revoked: Vec<String> = Vec::new();
        while let Some(id) = frontier.pop() {
            let mut stmt = conn
                .prepare("SELECT id FROM grant_state WHERE parent_id = ?1 AND state != 'revoked'")
                .map_err(|e| format!("grant ledger read: {}", e))?;
            let children: Vec<String> = stmt
                .query_map(rusqlite::params![id], |row| row.get::<_, String>(0))
                .map_err(|e| format!("grant ledger read: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);
            let changed = conn
                .execute(
                    "UPDATE grant_state SET state='revoked' WHERE id=?1 AND state != 'revoked'",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("grant ledger write: {}", e))?;
            if changed > 0 {
                let (scope, class): (String, String) = conn
                    .query_row(
                        "SELECT scope, class FROM grant_state WHERE id=?1",
                        rusqlite::params![id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .unwrap_or_default();
                record_event(conn, "revoked", &id, &scope, &class, -1, note)?;
                revoked.push(id.clone());
            }
            frontier.extend(children);
        }
        Ok(revoked.len())
    })
}

/// Consume one use of the grant (ADR-0155 §3.2). Returns the remaining
/// uses (-1 = unlimited). Typed errors, never panics.
pub fn grant_use(handle: &GrantHandle, note: &str) -> Result<i64, String> {
    let id = handle.grant_id.clone();
    with_conn(|conn| {
        let rec = load_record(conn, &id)?;
        check_record(&rec)?;
        let (new_state, remaining_after) = match rec.class {
            GrantClass::Once => ("consumed", 0i64),
            GrantClass::N(_) => ("active", rec.remaining - 1),
            GrantClass::Unlimited => ("active", -1i64),
        };
        conn.execute(
            "UPDATE grant_state SET state=?2, remaining=?3 WHERE id=?1",
            rusqlite::params![id, new_state, remaining_after],
        )
        .map_err(|e| format!("grant ledger write: {}", e))?;
        record_event(
            conn,
            "used",
            &id,
            &rec.scope,
            &rec.class.as_ledger_str(),
            remaining_after,
            note,
        )?;
        Ok(remaining_after)
    })
}

/// Non-consuming validity check (the `db_execute_with_grant` pre-check:
/// the consumption happens only after the SQL statement succeeded).
pub fn check_active(handle: &GrantHandle) -> Result<GrantRecord, String> {
    let id = handle.grant_id.clone();
    with_conn(|conn| {
        let rec = load_record(conn, &id)?;
        check_record(&rec)?;
        Ok(rec)
    })
}

// ── Introspection (tests / ledger export surfaces) ─────────────────────

/// Current (state, remaining) of a grant, if it exists.
pub fn state_of(grant_id: &str) -> Option<(String, i64)> {
    with_conn(|conn| {
        let rec = load_record(conn, grant_id)?;
        Ok((rec.state, rec.remaining))
    })
    .ok()
}

/// Number of ledger events recorded for one grant id.
pub fn events_for(grant_id: &str) -> usize {
    with_conn(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM grant_events WHERE grant_id = ?1",
            rusqlite::params![grant_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n as usize)
        .map(Ok)
        .unwrap_or(Ok(0))
    })
    .unwrap_or(0)
}

/// Total grants currently in `state` ('active' | 'consumed' | 'revoked').
pub fn count_in_state(state: &str) -> usize {
    with_conn(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM grant_state WHERE state = ?1",
            rusqlite::params![state],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n as usize)
        .map(Ok)
        .unwrap_or(Ok(0))
    })
    .unwrap_or(0)
}
