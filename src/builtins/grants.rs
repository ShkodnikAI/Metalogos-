//! Capability grants (authorization), not funding — the DSL surface.
//!
//! This module is the algebra of ACCESS: the opaque `Value::Grant` handles
//! that authorize irreversible operations inside a scope/TTL/quota
//! (ADR-0155). Nothing financial lives here — no donations, no funding
//! flows, no money movement (the project's funding surfaces are
//! Open Collective / FUNDING.yml and are unrelated to this code).
// ── Naryad #390 (P0, security/action): Grant algebra builtins ──────────
//
// The DSL surface of ADR-0155 (лecало: src/builtins/consent.rs — builtins,
// NOT new AST nodes; the ledger is the SSOT, the handles are untrusted
// caches). Five builtins, appended to BUILTIN_REGISTRY (bytecode indices
// must not shift):
//
//   grant_issue(scope, ttl, class?, uses?) -> Capability
//       class: "once" (default) | "n" (requires `uses`) | "unlimited";
//       ttl in seconds (>= 1 recommended; 0 = born expired, tests only).
//       №844 (ADR-0203 §2.1): the mint returns the TYPED carrier — the
//       read-only Capability view over the ledger row; the opaque Grant
//       handle rides in the `grant` field (extractors accept both forms —
//       the migration 1:1).
//   grant_subgrant(parent, scope, ttl, class?, uses?) -> Capability
//       attenuation-only (ADR-0155 §3.3 rule 4): narrower scope, shorter
//       TTL, class power only down; widening attempts fail with
//       GRANT_ESCALATION. Once parents are consumed by the split (linear
//       transfer); N(n) parents are debited by the child quota.
//   grant_revoke(parent) -> Number
//       cascading (rule 5): the target and every descendant transition
//       to 'revoked'; returns the count.
//   grant_use(g) -> Number
//       consume one use: Once -> consumed (second use: GRANT_REUSED),
//       N(n) -> decrement (0 left: GRANT_EXHAUSTED), Unlimited -> -1.
//   db_execute_with_grant(g, sql, params?) -> String
//       the granted destructive-SQL action (§3.3 rule 6 keeps the
//       ungranted `db_execute` deny unchanged). Runtime gates: ledger
//       state, TTL, scope coverage of the SQL's destructive ops
//       (GRANT_SCOPE_MISMATCH), quota. Consumption happens only after
//       the statement succeeded; every use is a ledger event.
//   capability_state(c) -> Struct<CapabilityState> (№844, ADR-0203 §2.1)
//       the LIVE typed view over the ledger record — scope, class, ttl,
//       quota (the remaining uses NOW), revoked, state. Read-only.
//
// Display/serialization refusal (rule 2) lives in the Value layer
// (is_nonprintable + the "[GRANT]" serde marker), not here.

use crate::capability;
use crate::grants::{self, GrantClass, GrantHandle};
use crate::interpreter::values::Value;

/// №844 (ADR-0203 §2.1, the migration 1:1 seam): a grant argument accepts
/// BOTH the opaque `Value::Grant` (the pre-№844 form) and the typed
/// Capability view. The refusal text keeps the historical "must be a
/// Grant" wording (the №390 contract tests pin it).
fn expect_grant_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<GrantHandle, String> {
    match args.get(idx).and_then(capability::grant_from_value) {
        Some(h) => Ok(h),
        None => match args.get(idx) {
            Some(other) => Err(format!(
                "{}: argument {} must be a Grant, got {}",
                fn_name,
                idx + 1,
                other.type_name()
            )),
            None => Err(format!("{}: missing argument {} (Grant)", fn_name, idx + 1)),
        },
    }
}

fn expect_scope_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!(
            "{}: argument {} (scope) must be String, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!("{}: missing argument {} (scope)", fn_name, idx + 1)),
    }
}

fn expect_ttl_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<u64, String> {
    match args.get(idx) {
        Some(Value::Float(n)) if *n >= 0.0 && n.fract() == 0.0 => Ok(*n as u64),
        Some(other) => Err(format!(
            "{}: argument {} (ttl_seconds) must be a non-negative whole Number, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing argument {} (ttl_seconds)",
            fn_name,
            idx + 1
        )),
    }
}

/// Parse the optional (class, uses) tail shared by issue/subgrant:
/// absent -> (Once, 1); ("once") -> (Once, 1); ("unlimited") -> (Unlimited);
/// ("n", uses) -> (N(uses)); ("n") without uses is an error; a uses count
/// without class "n" is an error.
fn parse_class_tail(fn_name: &str, args: &[Value], class_idx: usize) -> Result<GrantClass, String> {
    let class_word = match args.get(class_idx) {
        None => return Ok(GrantClass::Once), // the safest default (ADR-0155 §3.2)
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "{}: argument {} (class) must be String, got {}",
                fn_name,
                class_idx + 1,
                other.type_name()
            ))
        }
    };
    let has_uses = class_idx + 1 < args.len();
    match GrantClass::parse(&class_word)? {
        GrantClass::N(_) => match args.get(class_idx + 1) {
            Some(Value::Float(n)) if *n >= 1.0 && n.fract() == 0.0 => {
                Ok(GrantClass::N(*n as u64))
            }
            Some(other) => Err(format!(
                "{}: class \"n\" requires a whole uses count >= 1 after the class word, got {}",
                fn_name,
                other.type_name()
            )),
            None => Err(format!(
                "{}: class \"n\" requires a uses count argument (e.g. grant_issue(scope, ttl, \"n\", 5))",
                fn_name
            )),
        },
        GrantClass::Once => {
            if has_uses {
                Err(format!(
                    "{}: a uses count is only valid with class \"n\"",
                    fn_name
                ))
            } else {
                Ok(GrantClass::Once)
            }
        }
        GrantClass::Unlimited => {
            if has_uses {
                Err(format!(
                    "{}: a uses count is only valid with class \"n\"",
                    fn_name
                ))
            } else {
                Ok(GrantClass::Unlimited)
            }
        }
    }
}

/// `grant_issue(scope, ttl, class?, uses?) -> Grant`
pub(crate) fn builtin_grant_issue(args: &[Value]) -> Result<Value, String> {
    let fn_name = "grant_issue";
    if args.len() < 2 || args.len() > 4 {
        return Err(format!(
            "{}: expects 2..4 arguments (scope, ttl_seconds, class?, uses?), got {}",
            fn_name,
            args.len()
        ));
    }
    let scope = expect_scope_arg(fn_name, args, 0)?;
    let ttl = expect_ttl_arg(fn_name, args, 1)?;
    let class = parse_class_tail(fn_name, args, 2)?;
    let handle = grants::issue(&scope, ttl, &class, "program")?;
    // №844 (ADR-0203 §2.1): the mint returns the TYPED carrier — the
    // read-only Capability view over the ledger row (the handle rides in
    // the `grant` field; extractors accept both forms — 1:1 migration).
    Ok(capability::mint_view(
        &handle,
        capability::initial_remaining(&class),
    ))
}

/// `grant_subgrant(parent, scope, ttl, class?, uses?) -> Grant`
pub(crate) fn builtin_grant_subgrant(args: &[Value]) -> Result<Value, String> {
    let fn_name = "grant_subgrant";
    if args.len() < 3 || args.len() > 5 {
        return Err(format!(
            "{}: expects 3..5 arguments (parent, scope, ttl_seconds, class?, uses?), got {}",
            fn_name,
            args.len()
        ));
    }
    let parent = expect_grant_arg(fn_name, args, 0)?;
    let scope = expect_scope_arg(fn_name, args, 1)?;
    let ttl = expect_ttl_arg(fn_name, args, 2)?;
    let class = parse_class_tail(fn_name, args, 3)?;
    let handle = grants::subgrant(&parent, &scope, ttl, &class)?;
    // №844: the typed carrier, same posture as grant_issue.
    Ok(capability::mint_view(
        &handle,
        capability::initial_remaining(&class),
    ))
}

/// `grant_revoke(g) -> Number` — cascading; returns the revoked count.
pub(crate) fn builtin_grant_revoke(args: &[Value]) -> Result<Value, String> {
    let fn_name = "grant_revoke";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (grant), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle = expect_grant_arg(fn_name, args, 0)?;
    let n = grants::revoke(&handle, "revoked by program")? as f64;
    Ok(Value::Float(n))
}

/// `grant_use(g) -> Number` — consume one use; returns remaining
/// (-1 for Unlimited).
pub(crate) fn builtin_grant_use(args: &[Value]) -> Result<Value, String> {
    let fn_name = "grant_use";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (grant), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle = expect_grant_arg(fn_name, args, 0)?;
    let remaining = grants::grant_use(&handle, "grant_use()")?;
    Ok(Value::Float(remaining as f64))
}

/// `db_execute_with_grant(g, sql, params?) -> String` — granted destructive
/// SQL action; intercepted by BOTH backends (db_conn required) — reaching
/// this generic fallback means no database connection exists.
pub(crate) fn builtin_db_execute_with_grant(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err(
        "db_execute_with_grant() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
            .to_string(),
    )
}

/// `capability_state(c) -> Struct<CapabilityState>` — №844 (ADR-0203
/// §2.1): the LIVE typed view over the grant ledger — scope, class, ttl
/// (the expiry instant), quota (the remaining uses NOW), revoked, state
/// (active|consumed|revoked). Read-only: the view consumes NOTHING — the
/// single consume points stay `grant_use` and the carrier gates' own
/// post-success consumption. Accepts the Capability view and the opaque
/// Grant handle (the migration 1:1 seam); an unknown id refuses
/// GRANT_MISSING (fail-closed — a dead token does not fabricate a state).
pub(crate) fn builtin_capability_state(args: &[Value]) -> Result<Value, String> {
    let fn_name = "capability_state";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (capability), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle = expect_grant_arg(fn_name, args, 0)?;
    capability::state_view(&handle)
}
