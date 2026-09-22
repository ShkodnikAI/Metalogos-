// ── Naryad #348 (P1, feature/session, ADR-0172): Session model ───────
//
// The real session surface replacing the №16.0-6(б) pillar stubs
// (`session_login` returned an empty Session map; `session_logout` was
// a no-op — both marked "mock" in the registry before this naryad).
//
// Model (ADR-0172):
//   - a session is an opaque `Value::Session` handle (the ADR-0114
//     pattern; the map carries `id`/`user`/`duty` — the printable
//     projection only) backed by a process-global registry;
//   - lifecycle: login (create) → wake(s) / interrupt(s) →
//     duty-enter / duty-exit → end (logout); EVERY transition lands in
//     the Action Ledger (ADR-0167 §3.4 — the record call site IS the
//     transition's own bookkeeping path, best-effort per §2 driver 5:
//     a ledger failure is loud on stderr, never flips the session
//     operation's outcome);
//   - wake sources are a closed set: "keyword" | "event" | "schedule".
//     Schedule wakes arrive through the №418 cron payload-dispatch
//     calling `session_wake(source: "schedule")` — no new cron
//     surface is introduced (SSOT №418, arity 2..5 untouched);
//   - interrupts carry a typed priority: low < normal < high <
//     critical. `take` returns the HIGHEST-priority pending interrupt
//     (FIFO within one priority) — the preemption contract №352
//     builds on. Every enqueue AND dequeue is ledger-recorded, so
//     preemption loses no audit trail (the №352 duplex invariant);
//   - the duty profile: session-side runtime state (duty-enter/exit,
//     ledger-recorded) plus the program-level `profile duty` carrier
//     (src/profile.rs) that the №349 compiler rule keys on. This
//     naryad lands the carrier and the runtime state; the static
//     enforcement (private-materialization / network-sink compile
//     errors) is №349 — the loud boundary is in ADR-0172 §6;
//   - honest auth boundary: there is NO server user-store in the
//     interpreter, so `session_login` does NOT verify credentials —
//     the password argument is accepted (String or Secret) but only
//     mixed into the session-id preimage. This is a documented
//     boundary, not a stub: the session state, ledger trail and
//     wake/interrupt machinery are real.

use crate::interpreter::values::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Closed wake-source vocabulary (ADR-0172 §4.1).
pub const WAKE_SOURCES: [&str; 3] = ["keyword", "event", "schedule"];

/// Typed interruption priorities (ADR-0172 §4.2), ascending rank.
pub const INTERRUPT_PRIORITIES: [&str; 4] = ["low", "normal", "high", "critical"];

fn priority_rank(word: &str) -> Option<u8> {
    INTERRUPT_PRIORITIES
        .iter()
        .position(|p| *p == word)
        .map(|i| i as u8)
}

/// A pending wake event (ADR-0172 §4.1).
#[derive(Debug, Clone)]
pub struct WakeEvent {
    pub source: String,
    pub payload: String,
    pub enqueued_unix: u64,
}

/// A pending interrupt event (ADR-0172 §4.2).
#[derive(Debug, Clone)]
pub struct InterruptEvent {
    pub priority: String,
    pub rank: u8,
    pub reason: String,
    pub enqueued_unix: u64,
}

/// Registry-side session state (ADR-0172 §3). The value-surface map is
/// only the printable projection; this struct is the real state.
/// Registry membership IS the liveness marker: a session removed by
/// `end` is unknown to every subsequent operation (fail-closed).
#[derive(Debug)]
pub struct SessionState {
    pub user: String,
    pub duty: bool,
    pub created_unix: u64,
    pub wakes: VecDeque<WakeEvent>,
    pub interrupts: VecDeque<InterruptEvent>,
}

impl SessionState {
    fn new(user: String) -> Self {
        SessionState {
            user,
            duty: false,
            created_unix: unix_now(),
            wakes: VecDeque::new(),
            interrupts: VecDeque::new(),
        }
    }
}

type Registry = HashMap<String, SessionState>;

type RegLockResult<T> = Result<T, String>;

fn lock_registry() -> RegLockResult<std::sync::MutexGuard<'static, Registry>> {
    registry()
        .lock()
        .map_err(|e| format!("session registry lock: {}", e))
}

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

/// Fresh opaque session id: `s-` + first 16 hex chars of a SHA-256 over
/// (user, unix-millis, process seq) — uniqueness is the requirement, the
/// id carries no secret (ADR-0172 §3.1).
fn fresh_session_id(user: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let preimage = format!("{}|{}|{}", user, millis, seq);
    format!(
        "s-{}",
        &crate::ledger::sha256_hex(preimage.as_bytes())[..16]
    )
}

/// Best-effort ledger side effect (the grants.rs template): a ledger
/// failure is loud on stderr, never flips the session operation.
fn ledger_session_event(kind: &str, session_id: &str, detail: &str) {
    crate::ledger::record(&format!("session.{}", kind), session_id, "session", detail);
}

/// Mint a new session: register the state, record `session.create`,
/// return the handle. (`session.create` already appears in the №393
/// golden corpus vocabulary — the name predates this naryad.)
pub fn create(user: &str) -> Value {
    let id = fresh_session_id(user);
    if let Ok(mut reg) = lock_registry() {
        reg.insert(id.clone(), SessionState::new(user.to_string()));
    }
    ledger_session_event("create", &id, &format!("user={}", user));
    session_value(&id, user, false)
}

/// The printable projection of a session (ADR-0114 opaque pattern: the
/// map is data, the registry is the state).
fn session_value(id: &str, user: &str, duty: bool) -> Value {
    Value::Session(HashMap::from([
        ("id".to_string(), id.to_string()),
        ("user".to_string(), user.to_string()),
        ("duty".to_string(), if duty { "1" } else { "0" }.to_string()),
    ]))
}

/// Extract the session id from a `Value::Session` handle argument.
pub fn session_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::Session(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: Session handle has no id field (stub-era value?) — create sessions with session_login",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected Session as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing Session argument {}", fn_name, idx + 1)),
    }
}

/// Run `f` against the live session state; typed SESSION_UNKNOWN on an
/// unknown/ended session (fail-closed, no panic, no implicit recreate).
fn with_state<T>(
    fn_name: &str,
    id: &str,
    f: impl FnOnce(&mut SessionState) -> T,
) -> Result<T, String> {
    let mut reg = lock_registry()?;
    match reg.get_mut(id) {
        Some(state) => Ok(f(state)),
        None => Err(format!(
            "{}: unknown session '{}' (SESSION_UNKNOWN)",
            fn_name, id
        )),
    }
}

/// End the session: remove the registry entry, record `session.end`.
/// Typed SESSION_UNKNOWN error when the id is unknown/already ended.
pub fn end(fn_name: &str, id: &str) -> Result<(), String> {
    let mut reg = lock_registry()?;
    match reg.remove(id) {
        Some(state) => {
            ledger_session_event("end", id, &format!("user={}", state.user));
            Ok(())
        }
        None => Err(format!(
            "{}: unknown session '{}' (SESSION_UNKNOWN)",
            fn_name, id
        )),
    }
}

/// Duty-enter: the session switches into the duty (background) mode.
/// Runtime half of the duty-profile carrier; the static half is the
/// `profile duty` declaration (№349 enforces).
pub fn duty_enter(id: &str) -> Result<bool, String> {
    let changed = with_state("session_duty_enter", id, |s| {
        let changed = !s.duty;
        s.duty = true;
        changed
    })?;
    ledger_session_event("duty_enter", id, &format!("changed={}", changed));
    Ok(changed)
}

/// Duty-exit: back to the foreground mode.
pub fn duty_exit(id: &str) -> Result<bool, String> {
    let changed = with_state("session_duty_exit", id, |s| {
        let changed = s.duty;
        s.duty = false;
        changed
    })?;
    ledger_session_event("duty_exit", id, &format!("changed={}", changed));
    Ok(changed)
}

/// Enqueue a wake event (closed source vocabulary) + `session.wake`.
pub fn wake(id: &str, source: &str, payload: &str) -> Result<(), String> {
    if !WAKE_SOURCES.contains(&source) {
        return Err(format!(
            "session_wake: unknown wake source '{}' (available: {})",
            source,
            WAKE_SOURCES.join(", ")
        ));
    }
    with_state("session_wake", id, |s| {
        s.wakes.push_back(WakeEvent {
            source: source.to_string(),
            payload: payload.to_string(),
            enqueued_unix: unix_now(),
        });
    })?;
    ledger_session_event("wake", id, &format!("source={}|{}", source, payload));
    Ok(())
}

/// Dequeue the OLDEST wake (FIFO; wakes are not prioritized — interrupts
/// are) + `session.wake_delivered`. Empty queue → Ok(None) → Unit.
pub fn poll_wake(id: &str) -> Result<Option<(String, String)>, String> {
    let popped = with_state("session_poll_wake", id, |s| s.wakes.pop_front())?;
    match popped {
        Some(w) => {
            ledger_session_event(
                "wake_delivered",
                id,
                &format!("source={}|{}", w.source, w.payload),
            );
            Ok(Some((w.source, w.payload)))
        }
        None => Ok(None),
    }
}

/// Enqueue an interrupt with a typed priority + `session.interrupt`.
pub fn interrupt(id: &str, priority: &str, reason: &str) -> Result<(), String> {
    let rank = match priority_rank(priority) {
        Some(r) => r,
        None => {
            return Err(format!(
                "session_interrupt: unknown priority '{}' (available: {})",
                priority,
                INTERRUPT_PRIORITIES.join(", ")
            ))
        }
    };
    with_state("session_interrupt", id, |s| {
        s.interrupts.push_back(InterruptEvent {
            priority: priority.to_string(),
            rank,
            reason: reason.to_string(),
            enqueued_unix: unix_now(),
        });
    })?;
    ledger_session_event(
        "interrupt",
        id,
        &format!("priority={}|{}", priority, reason),
    );
    Ok(())
}

/// Take the HIGHEST-priority pending interrupt (FIFO within one
/// priority) — the preemption contract — + `session.interrupt_taken`.
/// Empty queue → Ok(None) → Unit. Nothing is dropped silently: every
/// enqueue and every take is a ledger record (ADR-0172 §4.2).
pub fn take_interrupt(id: &str) -> Result<Option<(String, String)>, String> {
    let taken = with_state("session_take_interrupt", id, |s| {
        if s.interrupts.is_empty() {
            return None;
        }
        // Highest rank wins; ties resolve FIFO (the earliest enqueued
        // among the same rank — VecDeque front-scan keeps stability).
        let mut best = 0usize;
        for (i, ev) in s.interrupts.iter().enumerate() {
            if ev.rank > s.interrupts[best].rank {
                best = i;
            }
        }
        s.interrupts.remove(best)
    })?;
    match taken {
        Some(ev) => {
            ledger_session_event(
                "interrupt_taken",
                id,
                &format!("priority={}|{}", ev.priority, ev.reason),
            );
            Ok(Some((ev.priority, ev.reason)))
        }
        None => Ok(None),
    }
}

/// True when the id names a live (registered) session.
pub fn is_live(id: &str) -> bool {
    lock_registry()
        .map(|reg| reg.contains_key(id))
        .unwrap_or(false)
}
