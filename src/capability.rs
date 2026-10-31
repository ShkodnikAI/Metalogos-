// ── Naryad №844 (wave 72 — W-B, P1, [core][security]): Capability-Token ──
//
//! The typed capability carrier (ADR-0203 §2.1): the language-visible,
//! read-only VIEW over a grant — the single consumption contract for an
//! irreversible action. The grant ledger (src/grants.rs) stays the SSOT
//! behind every field; the token is an UNTRUSTED CACHE of the ledger
//! record (the ADR-0155 handle posture kept verbatim), so a stale token
//! cannot revive power: every authority check reads the ledger at use
//! time, and consume happens ONLY through the single points
//! (`grant_use` — the explicit consume, and the carrier gates' own
//! post-success consumption, e.g. `db_execute_with_grant`).
//!
//! The language surface is a `Value::Struct` with `type_name
//! "Capability"` (the №838 ModelPassport typed-artifact pattern; the
//! DenyEvent/ForecastState runtime-Struct precedent):
//!
//!   Capability { scope, class, ttl, quota, revoked, grant }
//!     scope   String  — the grant scope (`db:delete:sessions`, …)
//!     class   String  — the canonical ledger form "once"|"n:<k>"|"unlimited"
//!     ttl     Float   — the expiry instant (unix seconds)
//!     quota   Float   — the remaining uses at MINT (-1 = unlimited); the
//!                       LIVE state reads through `capability_state`
//!     revoked Bool    — mint-time state (always false at mint; the live
//!                       view is `capability_state`)
//!     grant   Value::Grant — the opaque authority handle (non-printable,
//!                       serde dead marker; a program cannot forge one —
//!                       only `grant_issue`/`grant_subgrant` mint it)
//!
//! Migration 1:1 (the №838 pattern): `grant_issue`/`grant_subgrant` now
//! return the typed Capability, and EVERY grant-argument extractor in the
//! tree accepts BOTH forms (the opaque `Value::Grant` and the Capability
//! view) through `grant_from_value` — existing programs keep working
//! byte-for-byte.

use crate::grants::{GrantClass, GrantHandle};
use crate::interpreter::values::Value;

/// The Struct type_name of the typed capability carrier.
pub const CAPABILITY_TYPE: &str = "Capability";

/// The Struct type_name of the LIVE capability-state view.
pub const CAPABILITY_STATE_TYPE: &str = "CapabilityState";

/// Build the mint-time Capability view over a freshly minted handle.
/// `remaining` is the ledger row's remaining uses at mint (-1 = unlimited).
pub fn mint_view(handle: &GrantHandle, remaining: i64) -> Value {
    let mut fields = std::collections::HashMap::new();
    fields.insert("scope".to_string(), Value::String(handle.scope.clone()));
    fields.insert(
        "class".to_string(),
        Value::String(handle.class.as_ledger_str()),
    );
    fields.insert("ttl".to_string(), Value::Float(handle.expires_at as f64));
    fields.insert("quota".to_string(), Value::Float(remaining as f64));
    fields.insert("revoked".to_string(), Value::Bool(false));
    fields.insert("grant".to_string(), handle.to_value());
    Value::Struct {
        type_name: CAPABILITY_TYPE.to_string(),
        fields,
    }
}

/// True when the value is the typed Capability carrier.
pub fn is_capability(v: &Value) -> bool {
    matches!(v, Value::Struct { type_name, .. } if type_name == CAPABILITY_TYPE)
}

/// The shared grant-argument extractor (the migration 1:1 seam): accepts
/// the opaque `Value::Grant` (the pre-№844 form) AND the typed Capability
/// view (its `grant` field). A forged Struct without a real Grant handle
/// inside refuses — programs cannot construct `Value::Grant` (only the
/// mint builtins produce it), and a deserialized Capability carries the
/// dead `"[GRANT]"` String marker, which is not a handle (fail-closed).
pub fn grant_from_value(v: &Value) -> Option<GrantHandle> {
    match v {
        Value::Grant(h) => Some(h.clone()),
        Value::Struct { type_name, fields } if type_name == CAPABILITY_TYPE => {
            match fields.get("grant") {
                Some(g @ Value::Grant(_)) => grant_from_value(g),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The LIVE typed view over the ledger record (`capability_state(c)`):
/// every field is read from the grant ledger AT CALL TIME — the SSOT
/// posture of ADR-0203 §2.1. Refuses `GRANT_MISSING` for an unknown id
/// (fail-closed honesty: a dead token does not fabricate a state).
pub fn state_view(handle: &GrantHandle) -> Result<Value, String> {
    let rec = crate::grants::record_of(&handle.grant_id)?;
    let mut fields = std::collections::HashMap::new();
    fields.insert("scope".to_string(), Value::String(rec.scope.clone()));
    fields.insert(
        "class".to_string(),
        Value::String(rec.class.as_ledger_str()),
    );
    fields.insert("ttl".to_string(), Value::Float(rec.expires_at as f64));
    fields.insert("quota".to_string(), Value::Float(rec.remaining as f64));
    fields.insert("revoked".to_string(), Value::Bool(rec.state == "revoked"));
    fields.insert("state".to_string(), Value::String(rec.state.clone()));
    Ok(Value::Struct {
        type_name: CAPABILITY_STATE_TYPE.to_string(),
        fields,
    })
}

/// The mint-time remaining uses for a class (the ledger row's initial
/// `remaining`; -1 = unlimited). Mirrors `GrantClass::initial_remaining`
/// (kept in sync by the grants.rs unit test).
pub fn initial_remaining(class: &GrantClass) -> i64 {
    match class {
        GrantClass::Once => 1,
        GrantClass::N(k) => *k as i64,
        GrantClass::Unlimited => -1,
    }
}

// ── Naryad №845 (wave 72 — W-B, P1, [security][core]): the coverage rule
// ── the capability decision BEFORE every irreversible action ────────────
//
// ADR-0203 §2.4: every carrier's call passes the capability decision
// before run by one of two routes. THE REQUIREMENT PROFILE (finalized by
// the executor per the №843 boundary; the frame is the ADR):
//
//   **GrantAlways** — the destroy-effect classes that already carry the
//   typed grant-argument contract: `memory_forget_cascade` and `forget`
//   (the ADR-0155 linear actions — GRANT_MISSING without a grant, the
//   ledger checks state/TTL/quota at use time) and `db_execute_with_grant`
//   (the granted destructive-SQL action itself). A valid grant is required
//   ALWAYS; the profile cannot weaken this disposition. The enforcement
//   site is the carrier's own typed argument gate — the dispatch layer
//   records the disposition, it does not re-decide it.
//
//   **ProfilePermit** — the delivery / egress / host-effect classes: a
//   call is allowed only through (1) a covering capability grant (the
//   scope `builtin:<name>` or wider — the №801 lookup machinery), (2) an
//   explicit `profile capability { allow_carrier: "<name>" }` permit (the
//   №839 `allow_license_class` spelling precedent), or (3) the named host
//   permit for exec (`METALOGOS_ALLOW_EXEC=1`, the №97 opt-in — a NAMED
//   permit source, integrated, not a bypass).
//
//   The baseline resolution — a program with NO profile declared — is the
//   NAMED `standard` posture with an EXPLICIT machine-checked roster
//   (`BASELINE_STANDARD` below): the runtime-state and own-channel
//   classes (print, respond*, db_insert, the memory/session/kv/cron
//   state ops…). The roster is TOTALITY-CHECKED against the
//   classification SSOT (the №845 negative control (а)): every carrier
//   the classification marks Irreversible must sit in exactly one
//   disposition — a NEW irreversible carrier added without its
//   disposition fails the totality test and reddens the №846 coverage
//   gate loudly (GRANT_COVERAGE < 10000 bp).

/// Disposition of an irreversible carrier (ADR-0203 §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// A valid grant is required ALWAYS; the profile cannot weaken.
    GrantAlways,
    /// Allowed only through a covering grant, an explicit profile permit,
    /// or (for exec) the named host permit.
    ProfilePermit,
}

/// The GrantAlways roster: the destroy-effect classes whose typed
/// grant-argument contract IS the enforcement site (ADR-0203 §2.4).
pub const GRANT_ALWAYS: &[&str] = &["memory_forget_cascade", "forget"];

/// The ProfilePermit sharp roster: the delivery / egress / host-effect
/// classes — no baseline permit; every call needs a covering grant, an
/// explicit `profile capability { allow_carrier: … }`, or (exec only) the
/// №97 named host permit.
pub const PROFILE_PERMIT_SHARP: &[&str] = &[
    "exec",
    "exec_argv",
    "delete_file",
    "git_push",
    "http_post",
    "http_post_multipart",
    "send_message",
    "send_photo",
    "send_document",
    "smtp_send",
    "smtp_send_html",
    "tts_send",
];

/// The named `standard` baseline roster — the ProfilePermit carriers the
/// default posture permits (the runtime-state and own-channel classes).
/// NOT silence: the decision exists for every name here too, and the
/// totality test (negative control (а)) keeps this roster total over the
/// classification machine. A carrier in the classification that sits in
/// NONE of the three rosters refuses fail-closed and drops the №846
/// coverage below the floor — that is the gate working.
pub const BASELINE_STANDARD: &[&str] = &[
    "print",
    "respond",
    "respond_html",
    "respond_html_status",
    "respond_html_doc",
    "human_respond",
    "ask_approval",
    "answer_callback_query",
    "sleep",
    "db_insert",
    "db_execute",
    "kv_delete",
    "mem_delete",
    "session_clear",
    "memory_export",
    "memory_prune",
    "memory_forget",
    "mtree_forget",
    "voice_delete",
    "human_delete",
    "human_forget",
    "cron_remove",
    "cron_run",
    "cal_create",
    "cal_delete",
    "card_create",
    "card_delete",
    "ledger_rotate",
    "ledger_snapshot",
];

/// The disposition of a registered builtin, by the classification SSOT
/// (the machine source — never a hand list, the №845 negative control (а)).
/// `None` = not an irreversible carrier (the decision resolves trivially).
pub fn disposition(name: &str) -> Option<Disposition> {
    let class = crate::builtins_classification::classify(name)?;
    if class.reversibility != crate::builtins_classification::Reversibility::Irreversible {
        return None;
    }
    if GRANT_ALWAYS.contains(&name) {
        Some(Disposition::GrantAlways)
    } else if PROFILE_PERMIT_SHARP.contains(&name) {
        Some(Disposition::ProfilePermit)
    } else {
        // Baseline OR undisposed — resolved by the roster check in
        // `authorize_call` (the undisposed fallthrough refuses).
        Some(Disposition::ProfilePermit)
    }
}

/// True when `word` names a carrier the capability layer disposes over
/// (the machine check the `allow_carrier` validation uses — a typo in a
/// profile is a loud error, never a silent no-op permit).
pub fn is_carrier_name(word: &str) -> bool {
    GRANT_ALWAYS.contains(&word) || disposition(word).is_some()
}

/// The explicit program-level permit roster: the
/// `profile capability { allow_carrier: "<name>" }` declarations
/// (repeatable; the LAST declaration wins per option — the documented
/// profile re-declaration override). Resolved by BOTH lanes: the TW
/// interpreter (`run()` pre-pass) and the compiler (the
/// `Program::capability_permits` wire field, loaded by `Vm::load_program`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityPermits {
    carriers: std::collections::HashSet<String>,
}

impl CapabilityPermits {
    pub fn allow_carrier(&self, name: &str) -> bool {
        self.carriers.contains(name)
    }

    /// The roster as a Vec (the Program wire-field form — the compiler's
    /// emit and the VM's load go through this).
    pub fn to_vec(&self) -> Vec<String> {
        self.carriers.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.carriers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.carriers.is_empty()
    }
}

/// The trait form (the clippy `should_implement_trait` posture): the
/// permits roster is collected from the declaration options through the
/// standard `FromIterator` — the call sites keep reading naturally.
impl std::iter::FromIterator<String> for CapabilityPermits {
    fn from_iter<I: IntoIterator<Item = String>>(names: I) -> Self {
        CapabilityPermits {
            carriers: names.into_iter().collect(),
        }
    }
}

/// Collect the capability permits from the program declarations
/// (`profile capability { allow_carrier: … }`). Unknown profile names or
/// option shapes were already refused loudly by `profile::validate` in
/// the semantic lane — here a non-capability profile simply contributes
/// nothing.
pub fn permits_from_declarations(declarations: &[crate::ast::Declaration]) -> CapabilityPermits {
    let mut carriers = Vec::new();
    for decl in declarations {
        if let crate::ast::Declaration::Profile(p) = decl {
            if p.name == "capability" {
                for (k, v) in &p.options {
                    if k == "allow_carrier" {
                        carriers.push(v.clone());
                    }
                }
            }
        }
    }
    CapabilityPermits::from_iter(carriers)
}

/// The named HOST permit (№97): `METALOGOS_ALLOW_EXEC=1` authorizes the
/// exec class — a named, documented permit source (the env opt-in the
/// exec handler already required), NOT a bypass: without it the refusal
/// below fires the same way.
fn host_permitted(name: &str) -> bool {
    if name != "exec" && name != "exec_argv" {
        return false;
    }
    // The №253 SSOT semantics — the host flag is CONTEXT-DEPENDENT
    // ("замена", not AND): a serve-route body opens ONLY through the
    // serve flag (the process-level flag does NOT reach route bodies),
    // and the process context opens ONLY through the process flag. The
    // capability host permit rides the same context resolution —
    // otherwise the gate would either bypass Variant A (the process
    // flag opening a route) or lock the serve flag out.
    match crate::builtins::io::current_exec_context() {
        crate::builtins::io::ExecContext::ServeRoute => {
            std::env::var("METALOGOS_SERVE_ALLOW_EXEC").as_deref() == Ok("1")
        }
        crate::builtins::io::ExecContext::Process => {
            std::env::var("METALOGOS_ALLOW_EXEC").as_deref() == Ok("1")
        }
    }
}

/// The refusal text (the typed deny surface: the reason class is
/// IRREVERSIBLE_NO_GRANT — the vocabulary does not grow, ADR-0203 §2.4 —
/// and this human sentence carries the remedy verbatim).
fn refusal(name: &str) -> String {
    // The remedy names the CONTEXT-EXACT flag (№253): a serve-route body
    // is opened by the serve flag, the process context — by the process
    // flag; naming the wrong one would send the operator chasing a flag
    // that cannot help.
    let host_note = if name == "exec" || name == "exec_argv" {
        match crate::builtins::io::current_exec_context() {
            crate::builtins::io::ExecContext::ServeRoute => {
                ", or the named host permit (METALOGOS_SERVE_ALLOW_EXEC=1 — the \
                 process-level METALOGOS_ALLOW_EXEC does NOT apply to route bodies, №253)"
            }
            crate::builtins::io::ExecContext::Process => {
                ", or the named host permit (METALOGOS_ALLOW_EXEC=1)"
            }
        }
    } else {
        ""
    };
    format!(
        "IRREVERSIBLE_NO_GRANT: capability gate: '{name}' is an irreversible carrier — no covering capability grant (issue one with scope \"builtin:{name}\" or wider), no 'profile capability {{ allow_carrier: \"{name}\" }}' permit{host_note}; the refusal is BEFORE the action (ADR-0203 §2.4, №845 — fail-closed, the profile cannot weaken the GrantAlways classes)"
    )
}

/// THE capability decision (ADR-0203 §2.4, №845) — ONE shared surface,
/// called by BOTH executors at their dispatch choke points (TW invoke +
/// the TW expression dispatch sites; VM call_builtin). TW/VM parity by
/// construction — the №829 lesson. The decision is BEFORE the action:
/// the carrier's handler never runs on a refusal.
///
/// №861 (ADR-0205 §2.6): the decision is COMPOSED — the capability layer
/// first, then the approval gate (`approval::authorize_overseen_call`):
/// a human-oversight carrier additionally demands a LIVE mandate. One
/// composed surface for both backends — no dispatch site changes, the
/// parity stays by construction.
pub fn authorize_call(name: &str, permits: &CapabilityPermits) -> Result<(), String> {
    authorize_capability_call(name, permits)?;
    // №861 (ADR-0205 §2.6): the approval gate — the human-oversight class
    // additionally demands a live mandate; carriers outside the class
    // resolve trivially (Ok). The composed surface keeps the TW/VM
    // parity and the deny-event wiring of the №845 refusal path.
    crate::approval::authorize_overseen_call(name)
}

fn authorize_capability_call(name: &str, permits: &CapabilityPermits) -> Result<(), String> {
    let Some(Disposition::ProfilePermit) = disposition(name) else {
        // Not a carrier, or GrantAlways: the GrantAlways carriers carry
        // their typed grant-argument contract at the call itself
        // (GRANT_MISSING without a grant) — the profile cannot weaken
        // them and the dispatch layer does not re-decide what the
        // carrier's own gate already refuses.
        return Ok(());
    };
    if permits.allow_carrier(name) {
        return Ok(());
    }
    if host_permitted(name) {
        return Ok(());
    }
    if BASELINE_STANDARD.contains(&name) {
        // The named `standard` posture permits this carrier (machine
        // roster, totality-checked — honest limits, not silence). The
        // baseline route sits BEFORE the grant route: a covering grant
        // must not burn a use on a carrier that never needed it.
        return Ok(());
    }
    // The covering capability grant (ADR-0203 §2.2: "a valid token
    // CONSUMES"): the scope `builtin:<name>` or wider — the strongest
    // active covering grant (the №801 lookup), and the authorization
    // consumes ONE use through the single consume point (`grant_use` —
    // the ledger is the SSOT; the Once/N quota is metered HERE, the same
    // way the №801 gateway consumes for `mcp:<tool>`). A failed consume
    // (raced/missing) falls through to the refusal — fail-closed.
    let action_scope = format!("builtin:{name}");
    if let Some(handle) = crate::grants::lookup_active_covering(&action_scope) {
        if crate::grants::grant_use(&handle, &format!("capability gate consume for {name}")).is_ok()
        {
            return Ok(());
        }
    }
    Err(refusal(name))
}

#[cfg(test)]
mod coverage_tests {
    //! The №845 negative control (а): the roster totality over the
    //! classification machine. A NEW irreversible carrier without its
    //! disposition fails HERE first (and the №846 coverage gate second).

    use super::*;

    #[test]
    fn every_irreversible_carrier_has_exactly_one_disposition() {
        let mut missing = Vec::new();
        let mut double = Vec::new();
        for entry in crate::builtins_classification::BUILTIN_CLASSES {
            if entry.class.reversibility
                != crate::builtins_classification::Reversibility::Irreversible
            {
                continue;
            }
            let in_ga = GRANT_ALWAYS.contains(&entry.name);
            let in_sharp = PROFILE_PERMIT_SHARP.contains(&entry.name);
            let in_base = BASELINE_STANDARD.contains(&entry.name);
            if in_ga && (in_sharp || in_base) {
                double.push(entry.name);
            }
            if in_sharp && in_base {
                double.push(entry.name);
            }
            if !in_ga && !in_sharp && !in_base {
                missing.push(entry.name);
            }
        }
        assert!(
            missing.is_empty(),
            "irreversible carriers WITHOUT a capability disposition (add them to GRANT_ALWAYS / PROFILE_PERMIT_SHARP / BASELINE_STANDARD — the silent baseline is forbidden, ADR-0203 §2.4): {missing:?}"
        );
        assert!(
            double.is_empty(),
            "carriers in MORE THAN ONE disposition roster (the disposition is total and disjoint): {double:?}"
        );
    }

    #[test]
    fn baseline_and_sharp_rosters_are_carriers_by_the_machine() {
        // Every roster entry must name a REGISTERED builtin classified
        // Irreversible — a stale roster name (renamed/removed carrier) is
        // a loud failure, not a silent no-op row.
        for roster in [PROFILE_PERMIT_SHARP, BASELINE_STANDARD, GRANT_ALWAYS] {
            for name in roster {
                let class = crate::builtins_classification::classify(name)
                    .unwrap_or_else(|| panic!("roster name '{name}' is not a registered builtin"));
                assert_eq!(
                    class.reversibility,
                    crate::builtins_classification::Reversibility::Irreversible,
                    "roster name '{name}' is not Irreversible by the classification SSOT"
                );
            }
        }
    }

    #[test]
    fn authorize_refuses_sharp_without_permit_and_allows_with_it() {
        // exec without any permit (and without the env) refuses.
        let e = authorize_call("exec", &CapabilityPermits::default()).unwrap_err();
        assert!(e.starts_with("IRREVERSIBLE_NO_GRANT"), "got: {e}");
        // With the explicit profile permit it passes.
        let permits = CapabilityPermits::from_iter(["exec".to_string()]);
        authorize_call("exec", &permits).unwrap();
        // A baseline carrier passes with NO permits (the named standard
        // posture) and stays a machine-resolved decision.
        authorize_call("print", &CapabilityPermits::default()).unwrap();
        // A non-carrier passes trivially.
        authorize_call("upper", &CapabilityPermits::default()).unwrap();
        // An UNKNOWN name passes (the dispatch never reaches it — the
        // unknown-function error fires before the gate).
        authorize_call("definitely_not_a_builtin", &CapabilityPermits::default()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grants;

    #[test]
    fn mint_view_carries_the_adr_fields() {
        let h = grants::issue("db:delete:users", 3600, &GrantClass::N(3), "t").unwrap();
        let v = mint_view(&h, 3);
        match &v {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "Capability");
                for key in ["scope", "class", "ttl", "quota", "revoked", "grant"] {
                    assert!(fields.contains_key(key), "field {key} missing");
                }
                assert_eq!(fields.get("scope").unwrap().type_name(), "String");
                assert_eq!(fields.get("class").unwrap().type_name(), "String");
                assert_eq!(fields.get("quota").unwrap().type_name(), "Float");
                assert_eq!(fields.get("revoked").unwrap().type_name(), "Bool");
                assert_eq!(fields.get("grant").unwrap().type_name(), "Grant");
            }
            other => panic!("expected a Capability struct, got {other:?}"),
        }
    }

    #[test]
    fn grant_from_value_accepts_both_forms_and_refuses_forgery() {
        let h = grants::issue("db:delete:users", 60, &GrantClass::Once, "t").unwrap();
        // The opaque form (pre-№844) still extracts.
        assert!(grant_from_value(&h.to_value()).is_some());
        // The typed Capability view extracts the same handle.
        let cap = mint_view(&h, 1);
        let extracted = grant_from_value(&cap).unwrap();
        assert_eq!(extracted.grant_id, h.grant_id);
        // A forged Struct without a real handle refuses.
        let mut fields = std::collections::HashMap::new();
        fields.insert("grant".to_string(), Value::String("[GRANT]".to_string()));
        let forged = Value::Struct {
            type_name: CAPABILITY_TYPE.to_string(),
            fields,
        };
        assert!(grant_from_value(&forged).is_none());
        // A deserialized Capability carries the tombstone handle (the
        // serde dead marker reconstructs the expired-on-arrival Grant —
        // the ADR-0155 posture): the SEAM extracts it, but every USE
        // refuses at the ledger — power never revives.
        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("\"[GRANT]\""), "serde dead marker: {json}");
        let back: Value = serde_json::from_str(&json).unwrap();
        let tomb =
            grant_from_value(&back).expect("the tombstone extracts — the ADR-0155 serde posture");
        let err = grants::grant_use(&tomb, "revived").unwrap_err();
        assert!(
            err.starts_with("GRANT_"),
            "a deserialized capability must refuse every use, got: {err}"
        );
    }

    #[test]
    fn state_view_is_live_and_fail_closed() {
        let h = grants::issue("db:delete:users", 60, &GrantClass::N(2), "t").unwrap();
        let v = state_view(&h).unwrap();
        match &v {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "CapabilityState");
                assert_eq!(
                    fields.get("quota").unwrap().type_name(),
                    "Float",
                    "quota is a live ledger read"
                );
                assert_eq!(fields.get("state").unwrap().type_name(), "String");
            }
            other => panic!("expected CapabilityState, got {other:?}"),
        }
        // Consume through the SINGLE point → the view reflects the ledger.
        grants::grant_use(&h, "t").unwrap();
        let v2 = state_view(&h).unwrap();
        match &v2 {
            Value::Struct { fields, .. } => match fields.get("quota") {
                Some(Value::Float(q)) => assert_eq!(*q, 1.0, "live quota decremented"),
                other => panic!("quota must be Float, got {other:?}"),
            },
            _ => unreachable!(),
        }
        // Revoke → the live view flips.
        grants::revoke(&h, "t").unwrap();
        let v3 = state_view(&h).unwrap();
        match &v3 {
            Value::Struct { fields, .. } => {
                assert!(matches!(fields.get("revoked"), Some(Value::Bool(true))));
                assert!(matches!(
                    fields.get("state"),
                    Some(Value::String(s)) if s == "revoked"
                ));
            }
            _ => unreachable!(),
        }
        // An unknown id refuses (fail-closed, no fabricated state).
        let ghost = GrantHandle {
            grant_id: "g-does-not-exist".to_string(),
            scope: "db:*:*".to_string(),
            class: GrantClass::Once,
            expires_at: grants::now_secs() + 60,
            issuer: "t".to_string(),
        };
        let err = state_view(&ghost).unwrap_err();
        assert!(err.starts_with("GRANT_MISSING"), "got: {err}");
    }

    #[test]
    fn initial_remaining_matches_the_ledger_rule() {
        assert_eq!(initial_remaining(&GrantClass::Once), 1);
        assert_eq!(initial_remaining(&GrantClass::N(7)), 7);
        assert_eq!(initial_remaining(&GrantClass::Unlimited), -1);
    }
}

// ── №861 (wave 75 — W-D, ADR-0205 §2.6): the approval-gate composition
// tests. They live HERE (not in approval.rs) so the test-only reference
// to this module does not create a capability→approval→capability
// module cycle — the C4 acyclicity ratchet (the architecture contract)
// counts code edges, and the composed gate must stay a DAG edge:
// capability → approval, never back.
#[cfg(test)]
mod gate_tests {
    use super::*;
    use crate::approval::{
        authorize_overseen_call, mandate_issue, mandate_revoke, mandate_store_reset,
        HUMAN_OVERSIGHT,
    };

    #[test]
    fn the_roster_is_total_over_reachable_profile_permit_carriers() {
        // The №861 composition totality: every human-oversight name is an
        // irreversible ProfilePermit carrier — the ONLY shape the gate
        // composition in authorize_call actually reaches (a GrantAlways
        // or non-carrier name in the roster would bypass the approval
        // check and this test reddens loudly — no silent shelves).
        for name in HUMAN_OVERSIGHT {
            let d = disposition(name)
                .unwrap_or_else(|| panic!("{name}: the roster name must be a disposed carrier"));
            assert_eq!(
                d,
                Disposition::ProfilePermit,
                "{name}: the class composition must stay reachable by the authorize_call tail"
            );
            let class = crate::builtins_classification::classify(name)
                .unwrap_or_else(|| panic!("{name}: must be classified"));
            assert_eq!(
                class.reversibility,
                crate::builtins_classification::Reversibility::Irreversible,
                "{name}: the oversight class rides on irreversible carriers"
            );
        }
        assert!(!HUMAN_OVERSIGHT.is_empty(), "the class must not be vacuous");
    }

    #[test]
    #[serial_test::serial]
    fn the_gate_refuses_without_a_mandate_and_passes_with_one() {
        mandate_store_reset();
        // fail-closed: no mandate → the loud refusal with the remedy.
        let err = authorize_overseen_call("cron_run").unwrap_err();
        assert!(err.starts_with("IRREVERSIBLE_NO_GRANT"), "got: {err}");
        assert!(err.contains("approval_issue"), "got: {err}");
        assert!(err.contains("carrier:cron_run"), "got: {err}");
        // a mandate over a foreign target does NOT cover the carrier.
        mandate_issue("owner", "carrier:other", 600).unwrap();
        assert!(authorize_overseen_call("cron_run").is_err());
        // the mandate over the carrier target passes.
        mandate_issue("owner", "carrier:cron_run", 600).unwrap();
        assert!(authorize_overseen_call("cron_run").is_ok());
        // the mandate store resets clean (the harness seam).
        mandate_store_reset();
        assert!(authorize_overseen_call("cron_run").is_err());
    }

    #[test]
    #[serial_test::serial]
    fn a_revoked_mandate_stops_covering_the_carrier() {
        mandate_store_reset();
        mandate_issue("owner", "carrier:cron_run", 600).unwrap();
        assert!(authorize_overseen_call("cron_run").is_ok());
        assert_eq!(mandate_revoke("carrier:cron_run").unwrap(), 1);
        assert!(authorize_overseen_call("cron_run").is_err());
        assert_eq!(mandate_revoke("carrier:cron_run").unwrap(), 0, "idempotent");
        mandate_store_reset();
    }
}
