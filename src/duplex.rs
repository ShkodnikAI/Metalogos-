// ── Naryad #352 (ADR-0174 §3.3): the duplex channel — the barge-in
//    state machine over the №348 session priority ladder ──────────────
//
// A duplex channel binds ONE live №348 session to the two DIRECTED
// audio flows (listen in / speak out). The registry is the state; the
// `Value::Duplex` map is only the printable projection (the ADR-0114
// opaque pattern, the Session/Memory precedents).
//
// Contract (ADR-0174 §3.3):
//   - open requires a LIVE session (typed SESSION_UNKNOWN otherwise —
//     fail-closed, no implicit recreate);
//   - each direction holds AT MOST ONE active stream (a same-direction
//     start refuses with DUPLEX_BUSY — one flow per direction);
//   - BARGE-IN: a start on the OPPOSITE direction with priority rank
//     >= the active opposite stream's rank PREEMPTS it — the preempted
//     stream's outcome becomes the typed `InterruptedBy { by, priority }`
//     (never a panic, never a silent drop); a LOWER-priority start
//     refuses with DUPLEX_PREEMPT_DENIED (fail-closed; equal wins —
//     barge-in is the point of duplex);
//   - streams end typed: Active / Completed / InterruptedBy — there is
//     no way to observe a half-torn stream;
//   - EVERY transition is a ledger record (duplex.* family, the
//     №393/№415 surfaces, best-effort per ADR-0167 §2 driver 5);
//     preemption emits TWO records — the preempt decision AND the
//     preempted stream's terminal outcome — so interruption loses no
//     audit event BY CONSTRUCTION (the differential test pins it).

use crate::interpreter::values::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// The closed priority vocabulary — the SESSION ladder (№348), not a
/// new one (ADR-0174 §2 driver 2).
pub const DUPLEX_PRIORITIES: [&str; 4] = crate::session::INTERRUPT_PRIORITIES;

fn priority_rank(word: &str) -> Option<u8> {
    DUPLEX_PRIORITIES
        .iter()
        .position(|p| *p == word)
        .map(|i| i as u8)
}

/// The closed direction vocabulary (the two directed effects).
pub const DUPLEX_DIRECTIONS: [&str; 2] = ["listen", "speak"];

/// The typed outcome of a stream (ADR-0174 §3.3): there is no way to
/// observe a half-torn stream — every outcome is a constructor.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamOutcome {
    Active,
    Completed,
    InterruptedBy { by: String, priority: u8 },
}

impl StreamOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            StreamOutcome::Active => "active",
            StreamOutcome::Completed => "completed",
            StreamOutcome::InterruptedBy { .. } => "interrupted",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stream {
    pub id: String,
    pub direction: &'static str,
    pub rank: u8,
    pub priority_word: String,
    pub outcome: StreamOutcome,
}

#[derive(Debug)]
pub struct DuplexChannel {
    pub id: String,
    pub session: String,
    pub default_rank: u8,
    /// The ACTIVE streams only (the slot invariant: a slot is Some iff
    /// a stream is running on that direction).
    pub listen: Option<Stream>,
    pub speak: Option<Stream>,
    /// The TERMINAL outcomes (the last ended stream per direction) —
    /// the typed interruption/completion is observable here after the
    /// slot is vacated (the "прерванный поток завершается типизированно"
    /// contract; ADR-0174 §3.3).
    pub listen_last: Option<Stream>,
    pub speak_last: Option<Stream>,
}

type Registry = HashMap<String, DuplexChannel>;

fn registry() -> &'static Mutex<Registry> {
    static REG: OnceLock<Mutex<Registry>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

static SEQ: AtomicU64 = AtomicU64::new(1);

fn fresh_id(prefix: &str, seed: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let preimage = format!("{}|{}|{}|{}", seed, millis, seq, prefix);
    format!(
        "{}-{}",
        prefix,
        &crate::ledger::sha256_hex(preimage.as_bytes())[..16]
    )
}

/// The best-effort ledger record (the session.rs template).
fn ledger_duplex_event(kind: &str, channel_id: &str, detail: &str) {
    eprintln!("[DUPLEX_{}] {} {}", kind.to_uppercase(), channel_id, detail);
    crate::ledger::record(&format!("duplex.{}", kind), channel_id, "duplex", detail);
}

/// №428: the audio consent gate — speak/listen are directed audio
/// egress/ingress, so they require an ACTIVE consent grant for the
/// direction (`audio.speak` / `audio.listen`), recorded through the №335
/// consent contour (`consent_grant`). Fail-closed: on any store error the
/// answer is NO (`consent::active_grant_for` contract). The refusal is
/// itself a ledger record (`duplex.speak_denied` / `duplex.listen_denied`)
/// — no silent egress OR silent refusal. The error is origin-stamped
/// `AUDIO_CONSENT_REQUIRED` (№413 convention, never double-stamped) so
/// the `try` classifier can branch the office's ask-for-consent policy.
fn require_audio_consent(direction_flow: &str, channel_id: &str) -> Result<(), String> {
    let scope = format!("audio.{}", direction_flow);
    if crate::consent::active_grant_for(&scope) {
        return Ok(());
    }
    let detail = format!(
        "no active consent grant '{}' — audio {} refused fail-closed (№428/ADR-0174 §4)",
        scope, direction_flow
    );
    ledger_duplex_event(&format!("{}_denied", direction_flow), channel_id, &detail);
    Err(crate::interpreter::values::coded_error(
        crate::interpreter::values::CODE_AUDIO_CONSENT_REQUIRED,
        format!(
            "{}: channel '{}' has {}",
            direction_flow, channel_id, detail
        ),
    ))
}

/// Extract the channel id from a `Value::Duplex` handle argument (the
/// session_id_arg template).
pub fn channel_id_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::Duplex(map)) => match map.get("id") {
            Some(id) if !id.is_empty() => Ok(id.clone()),
            _ => Err(format!(
                "{}: Duplex handle has no id field — open channels with duplex_open",
                fn_name
            )),
        },
        Some(other) => Err(format!(
            "{}: expected Duplex as argument {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing Duplex argument {}", fn_name, idx + 1)),
    }
}

/// The channel projection (the printable handle value).
fn channel_value(c: &DuplexChannel) -> Value {
    Value::Duplex(HashMap::from([
        ("id".to_string(), c.id.clone()),
        ("session".to_string(), c.session.clone()),
    ]))
}

/// Open a duplex channel bound to a LIVE session (№348). The default
/// priority word is validated against the closed ladder; unknown
/// sessions refuse with typed SESSION_UNKNOWN (fail-closed).
pub fn open(session_id: &str, default_priority: &str) -> Result<Value, String> {
    let rank = priority_rank(default_priority).ok_or_else(|| {
        format!(
            "duplex_open: unknown priority '{}' (available: {}) — the ladder is the session's (№348)",
            default_priority,
            DUPLEX_PRIORITIES.join("/")
        )
    })?;
    if !crate::session::is_live(session_id) {
        return Err(format!(
            "duplex_open: unknown session '{}' (SESSION_UNKNOWN) — login first (session_login); the channel binds a LIVE session, fail-closed",
            session_id
        ));
    }
    let mut reg = registry()
        .lock()
        .map_err(|e| format!("duplex registry lock: {}", e))?;
    let c = DuplexChannel {
        id: fresh_id("dup", session_id),
        session: session_id.to_string(),
        default_rank: rank,
        listen: None,
        speak: None,
        listen_last: None,
        speak_last: None,
    };
    ledger_duplex_event(
        "open",
        &c.id,
        &format!(
            "session={}|default_priority={}",
            session_id, default_priority
        ),
    );
    let value = channel_value(&c);
    reg.insert(c.id.clone(), c);
    Ok(value)
}

/// Run `f` against the live channel; typed DUPLEX_UNKNOWN on an
/// unknown channel (fail-closed, no panic). The closure carries its
/// own typed errors — they pass through unchanged.
fn with_channel<T>(
    fn_name: &str,
    id: &str,
    f: impl FnOnce(&mut DuplexChannel) -> Result<T, String>,
) -> Result<T, String> {
    let mut reg = registry()
        .lock()
        .map_err(|e| format!("duplex registry lock: {}", e))?;
    match reg.get_mut(id) {
        Some(c) => f(c),
        None => Err(format!(
            "{}: unknown duplex channel '{}' (DUPLEX_UNKNOWN)",
            fn_name, id
        )),
    }
}

/// Resolve the effective rank for a start: the explicit word wins, the
/// channel default fills the gap.
fn resolve_rank(c: &DuplexChannel, explicit: Option<&str>) -> Result<(u8, String), String> {
    match explicit {
        Some(word) => {
            let rank = priority_rank(word).ok_or_else(|| {
                format!(
                    "unknown priority '{}' (available: {})",
                    word,
                    DUPLEX_PRIORITIES.join("/")
                )
            })?;
            Ok((rank, word.to_string()))
        }
        None => Ok((
            c.default_rank,
            DUPLEX_PRIORITIES[c.default_rank as usize].to_string(),
        )),
    }
}

/// The typed start of one directed flow (the shared engine of
/// speak_start / listen_start). Same-direction busy refuses; the
/// opposite-direction active stream is preempted when the incoming
/// rank >= its rank (barge-in, equal wins), refused otherwise.
fn start_stream(
    fn_name: &str,
    channel_id: &str,
    direction: &'static str,
    opposite: &'static str,
    explicit_priority: Option<&str>,
) -> Result<(Stream, Vec<Stream>), String> {
    let (rank, word) = {
        let reg = registry()
            .lock()
            .map_err(|e| format!("duplex registry lock: {}", e))?;
        let c = reg.get(channel_id).ok_or_else(|| {
            format!(
                "{}: unknown duplex channel '{}' (DUPLEX_UNKNOWN)",
                fn_name, channel_id
            )
        })?;
        resolve_rank(c, explicit_priority)?
    };
    with_channel(fn_name, channel_id, |c| {
        // Same direction: at most one active stream (DUPLEX_BUSY).
        let same = if direction == "speak" {
            &mut c.speak
        } else {
            &mut c.listen
        };
        if same.is_some() {
            return Err(format!(
                "{}: direction '{}' already has an active stream on channel '{}' (DUPLEX_BUSY) — stop it first",
                fn_name, direction, c.id
            ));
        }
        // Opposite direction: the barge-in decision (the session ladder).
        let (opposite_id, opposite_rank) = match if opposite == "speak" {
            c.speak.as_ref()
        } else {
            c.listen.as_ref()
        } {
            Some(active) => (active.id.clone(), active.rank),
            None => (String::new(), 0),
        };
        if !opposite_id.is_empty() {
            if rank < opposite_rank {
                return Err(format!(
                    "{}: {} stream rank {} cannot preempt the active {} stream '{}' (rank {}) — DUPLEX_PREEMPT_DENIED; use a higher priority",
                    fn_name, direction, rank, opposite, opposite_id, opposite_rank
                ));
            }
            // BARGE-IN (equal wins): the incoming stream id is minted
            // first, the opposite stream ends TYPED, then the incoming
            // takes its (empty) slot. The preempted stream's terminal
            // outcome is ITS OWN record — interruption loses no audit
            // event by construction.
            let incoming = Stream {
                id: fresh_id(direction, channel_id),
                direction,
                rank,
                priority_word: word.clone(),
                outcome: StreamOutcome::Active,
            };
            let opposite_slot = if opposite == "speak" {
                c.speak.as_mut()
            } else {
                c.listen.as_mut()
            };
            if let Some(slot) = opposite_slot {
                slot.outcome = StreamOutcome::InterruptedBy {
                    by: incoming.id.clone(),
                    priority: rank,
                };
                ledger_duplex_event(
                    "preempt",
                    &c.id,
                    &format!(
                        "incoming={}|direction={}|rank={}|preempted={}|preempted_rank={}",
                        incoming.id, direction, rank, slot.id, slot.rank
                    ),
                );
                ledger_duplex_event(
                    "interrupted",
                    &c.id,
                    &format!(
                        "stream={}|direction={}|by={}|priority={}",
                        slot.id, slot.direction, incoming.id, rank
                    ),
                );
                // Vacate the slot: the ended stream moves to the
                // terminal-history slot; the direction is free again.
                let preempted = vec![slot.clone()];
                if opposite == "speak" {
                    c.speak_last = c.speak.take();
                } else {
                    c.listen_last = c.listen.take();
                }
                let incoming_slot = if direction == "speak" {
                    c.speak = Some(incoming.clone());
                    c.speak.as_ref()
                } else {
                    c.listen = Some(incoming.clone());
                    c.listen.as_ref()
                };
                if let Some(slot) = incoming_slot {
                    ledger_duplex_event(
                        &format!("{}_start", direction),
                        &c.id,
                        &format!(
                            "stream={}|priority={}|barge_in=true",
                            slot.id, slot.priority_word
                        ),
                    );
                }
                return Ok((incoming, preempted));
            }
            return Err(format!(
                "{}: opposite slot vanished mid-barge-in (registry invariant broken)",
                fn_name
            ));
        }
        // No opposite stream: plain start.
        let stream = Stream {
            id: fresh_id(direction, channel_id),
            direction,
            rank,
            priority_word: word.clone(),
            outcome: StreamOutcome::Active,
        };
        ledger_duplex_event(
            &format!("{}_start", direction),
            &c.id,
            &format!(
                "stream={}|priority={}|barge_in=false",
                stream.id, stream.priority_word
            ),
        );
        let incoming_slot = if direction == "speak" {
            c.speak = Some(stream.clone());
            c.speak.as_ref()
        } else {
            c.listen = Some(stream.clone());
            c.listen.as_ref()
        };
        if incoming_slot.is_none() {
            return Err(format!(
                "{}: slot vanished mid-start (registry invariant broken)",
                fn_name
            ));
        }
        Ok((stream, Vec::new()))
    })
}

/// The typed stop of one directed flow (the shared engine of
/// speak_stop / listen_stop). An idle direction refuses with
/// DUPLEX_IDLE (fail-closed, loud — not a silent no-op).
fn stop_stream(fn_name: &str, channel_id: &str, direction: &'static str) -> Result<Stream, String> {
    with_channel(fn_name, channel_id, |c| {
        let slot = if direction == "speak" {
            c.speak.as_mut()
        } else {
            c.listen.as_mut()
        };
        let Some(stream) = slot else {
            return Err(format!(
                "{}: direction '{}' has no active stream on channel '{}' (DUPLEX_IDLE)",
                fn_name, direction, c.id
            ));
        };
        stream.outcome = StreamOutcome::Completed;
        ledger_duplex_event(
            "stop",
            &c.id,
            &format!(
                "stream={}|direction={}|outcome=completed",
                stream.id, stream.direction
            ),
        );
        let ended = stream.clone();
        if direction == "speak" {
            c.speak_last = c.speak.take();
        } else {
            c.listen_last = c.listen.take();
        }
        Ok(ended)
    })
}

/// `speak_start(duplex, text, priority?)` — the SPEAK flow engine.
/// The text never enters the ledger (a digest only — the №415 posture).
/// №428: requires an active `audio.speak` consent grant (fail-closed).
pub fn speak_start(
    channel_id: &str,
    _text: &str,
    priority: Option<&str>,
) -> Result<(Stream, Vec<Stream>), String> {
    require_audio_consent("speak", channel_id)?;
    start_stream("speak_start", channel_id, "speak", "listen", priority)
}

/// `listen_start(duplex, priority?)` — the LISTEN flow engine.
/// №428: requires an active `audio.listen` consent grant (fail-closed).
pub fn listen_start(
    channel_id: &str,
    priority: Option<&str>,
) -> Result<(Stream, Vec<Stream>), String> {
    require_audio_consent("listen", channel_id)?;
    start_stream("listen_start", channel_id, "listen", "speak", priority)
}

/// `speak_stop(duplex)` — ends the active speak stream.
pub fn speak_stop(channel_id: &str) -> Result<Stream, String> {
    stop_stream("speak_stop", channel_id, "speak")
}

/// `listen_stop(duplex)` — ends the active listen stream.
pub fn listen_stop(channel_id: &str) -> Result<Stream, String> {
    stop_stream("listen_stop", channel_id, "listen")
}

/// The stream projection for `duplex_state` (no content — metadata only).
fn stream_value(s: &Stream) -> Value {
    let mut fields: Vec<(&str, Value)> = vec![
        ("stream_id", Value::String(s.id.clone())),
        ("direction", Value::String(s.direction.to_string())),
        ("priority", Value::String(s.priority_word.clone())),
        ("outcome", Value::String(s.outcome.as_str().to_string())),
    ];
    if let StreamOutcome::InterruptedBy { by, priority } = &s.outcome {
        fields.push(("interrupted_by", Value::String(by.clone())));
        fields.push((
            "interrupted_at_priority",
            Value::String(DUPLEX_PRIORITIES[*priority as usize].to_string()),
        ));
    }
    struct_of("DuplexStream", fields)
}

/// The local Struct constructor (the fields map — the memory_forget.rs
/// helper shape; NOT the global core helper, no glob-namespace leak).
fn struct_of(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
    let map: std::collections::HashMap<String, Value> = fields
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    Value::Struct {
        type_name: type_name.to_string(),
        fields: map,
    }
}

/// `duplex_state(duplex)` — the introspection projection of both
/// directions (metadata only; audited).
pub fn state(channel_id: &str) -> Result<Value, String> {
    let reg = registry()
        .lock()
        .map_err(|e| format!("duplex registry lock: {}", e))?;
    let c = reg.get(channel_id).ok_or_else(|| {
        format!(
            "duplex_state: unknown duplex channel '{}' (DUPLEX_UNKNOWN)",
            channel_id
        )
    })?;
    let fields: Vec<(&str, Value)> = vec![
        ("id", Value::String(c.id.clone())),
        ("session", Value::String(c.session.clone())),
        (
            "speak",
            c.speak.as_ref().map(stream_value).unwrap_or(Value::Unit),
        ),
        (
            "listen",
            c.listen.as_ref().map(stream_value).unwrap_or(Value::Unit),
        ),
        (
            "speak_last",
            c.speak_last
                .as_ref()
                .map(stream_value)
                .unwrap_or(Value::Unit),
        ),
        (
            "listen_last",
            c.listen_last
                .as_ref()
                .map(stream_value)
                .unwrap_or(Value::Unit),
        ),
    ];
    ledger_duplex_event("state", &c.id, "introspection");
    Ok(struct_of("DuplexState", fields))
}

/// Introspection for tests: the raw stream outcome of one direction.
pub fn stream_outcome(channel_id: &str, direction: &str) -> Option<StreamOutcome> {
    let reg = registry().lock().ok()?;
    let c = reg.get(channel_id)?;
    match direction {
        "speak" => c.speak.as_ref().map(|s| s.outcome.clone()),
        "listen" => c.listen.as_ref().map(|s| s.outcome.clone()),
        _ => None,
    }
}

/// Introspection for tests: the stream id of one direction (active).
pub fn stream_id(channel_id: &str, direction: &str) -> Option<String> {
    let reg = registry().lock().ok()?;
    let c = reg.get(channel_id)?;
    match direction {
        "speak" => c.speak.as_ref().map(|s| s.id.clone()),
        "listen" => c.listen.as_ref().map(|s| s.id.clone()),
        _ => None,
    }
}

/// Introspection for tests: the TERMINAL outcome of the last ended
/// stream on one direction (the typed interruption observer).
pub fn stream_last_outcome(channel_id: &str, direction: &str) -> Option<StreamOutcome> {
    let reg = registry().lock().ok()?;
    let c = reg.get(channel_id)?;
    match direction {
        "speak" => c.speak_last.as_ref().map(|s| s.outcome.clone()),
        "listen" => c.listen_last.as_ref().map(|s| s.outcome.clone()),
        _ => None,
    }
}
