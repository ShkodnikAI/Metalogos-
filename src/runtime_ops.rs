//! №466 (gh#687) — group 7 (server/runtime) of the TW/VM duplicate-name
//! transfer: the shared LIVE home of the runtime utility surface (gate
//! gh#680, decision 4-A, step 3; threshold 28 → 20 — the LAST group).
//!
//! The eight names — `exec`, `find`, `fit_to_budget`, `inspect`,
//! `json_body`, `require`, `resolve_skill_index`, `server_path_param` —
//! keep their per-backend STATE divergences where the state shapes
//! differ (the №462 duplicates were the dispatch literals + the
//! byte-identical body fragments). What moves here:
//!
//! - the single spelling of the eight names outside `BUILTIN_REGISTRY`
//!   (constants + `handles()`);
//! - `fit_to_budget` — the FULL body (byte-identical identity-on-List
//!   on both backends, the №72 documented stub);
//! - `require` — the FULL body over the identical `Vec<String>` roles
//!   state (both backends hold `server_user_roles: Vec<String>`);
//! - `json_body` — the FULL body over the identical
//!   `Option<Value>` state (the empty-`JsonBody` fallback included);
//! - `server_path_param` — the FULL body over the identical
//!   `Option<HashMap<String, String>>` state (the empty-string
//!   fallback included);
//! - `find` — the shared argument validation (the four exact error
//!   texts) + the shared operator predicate (the unknown-operator
//!   error); the store iteration stays per-backend (the TW walks its
//!   `variables`, the VM its `globals` — the documented twin posture);
//! - `inspect` — the shared validation prologue (the two exact error
//!   texts); the stats lookup stays per-backend;
//! - `resolve_skill_index` — the shared validation prologue (the exact
//!   error text); the index lookup stays per-backend (the TW keeps a
//!   `HashMap`, the VM a `Vec` — different shapes, same contract);
//! - `exec` — the name constant referenced by the THREE TW sandbox
//!   guards (№17 Г.2) and the VM untrusted-sink rule (№325/№327);
//!   `exec_argv` stays a VM-only literal (never duplicated — the
//!   №462 counter never counted it).
//!
//! The №465 diff fuzzer runs before/after the transfer (the divergence
//! classes must not move); the shared fragments are pinned by the unit
//! tests below.

use std::collections::HashMap;

use crate::interpreter::Value;

// ── The single spelling of the group outside the registry ───────────

pub const NAME_EXEC: &str = "exec";
pub const NAME_FIND: &str = "find";
pub const NAME_FIT_TO_BUDGET: &str = "fit_to_budget";
pub const NAME_INSPECT: &str = "inspect";
pub const NAME_JSON_BODY: &str = "json_body";
pub const NAME_REQUIRE: &str = "require";
pub const NAME_RESOLVE_SKILL_INDEX: &str = "resolve_skill_index";
pub const NAME_SERVER_PATH_PARAM: &str = "server_path_param";

/// Whether `name` belongs to the server/runtime group (the dispatch
/// hook; the exec_argv twin is NOT part of the group — it never was a
/// №462 duplicate).
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        NAME_EXEC
            | NAME_FIND
            | NAME_FIT_TO_BUDGET
            | NAME_INSPECT
            | NAME_JSON_BODY
            | NAME_REQUIRE
            | NAME_RESOLVE_SKILL_INDEX
            | NAME_SERVER_PATH_PARAM
    )
}

// ── fit_to_budget (№72, the documented identity stub) ───────────────

/// The full shared body: return the first List argument unchanged;
/// a loud error on anything else (the exact text both backends had).
pub fn fit_to_budget(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("fit_to_budget() expects first argument to be a List".to_string()),
    };
    Ok(Value::List(list))
}

// ── require (RBAC, №14 P2-6) ────────────────────────────────────────

/// The full shared body over the identical roles state: `Ok(true)`
/// when the role is held, the loud access-denied error otherwise.
pub fn require(roles: &[String], args: &[Value]) -> Result<Value, String> {
    let role = args
        .first()
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();
    if roles.contains(&role) {
        return Ok(Value::Bool(true));
    }
    Err(format!(
        "require('{}'): access denied — user has roles {:?}",
        role, roles
    ))
}

// ── json_body (№283 parity surface) ─────────────────────────────────

/// The full shared body over the identical body state: the stored JSON
/// body clone, or the empty `JsonBody` struct in a non-server context.
pub fn json_body(server_json_body: Option<&Value>) -> Value {
    if let Some(body) = server_json_body {
        return body.clone();
    }
    Value::Struct {
        type_name: "JsonBody".to_string(),
        fields: HashMap::new(),
    }
}

// ── server_path_param (№283) ────────────────────────────────────────

/// The full shared body over the identical path-params state: the
/// parameter value, or the empty string when no templated route
/// matched / no server context (the query_param parity contract).
pub fn server_path_param(params: Option<&HashMap<String, String>>, args: &[Value]) -> Value {
    let param_name = args
        .first()
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();
    if let Some(params) = params {
        if let Some(val) = params.get(&param_name) {
            return Value::String(val.clone());
        }
    }
    Value::String(String::new())
}

// ── find (the entity-store query) ───────────────────────────────────

/// The validated find query: (type_name, field_name, op, threshold).
#[derive(Debug)]
pub struct FindQuery {
    pub type_name: String,
    pub field_name: String,
    pub op: String,
    pub threshold: f64,
}

/// The shared argument validation — the four exact error texts both
/// backends carried verbatim.
pub fn find_args(args: &[Value]) -> Result<FindQuery, String> {
    let type_name = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("find() requires type name as first argument (String)".to_string()),
    };
    let field_name = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("find() requires field name as second argument (String)".to_string()),
    };
    let op = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "find() requires operator as third argument (String: gt/lt/ge/le/eq)".to_string(),
            )
        }
    };
    let threshold = match args.get(3) {
        Some(Value::Float(f)) => *f,
        _ => return Err("find() requires threshold as fourth argument (Float)".to_string()),
    };
    Ok(FindQuery {
        type_name,
        field_name,
        op,
        threshold,
    })
}

/// The shared operator predicate: does the struct value match the
/// query? `Err` on an unknown operator (the exact shared text).
/// The float-equality tolerance (1e-9) is the shared `eq` semantics.
pub fn find_matches(value: &Value, q: &FindQuery) -> Result<bool, String> {
    if let Value::Struct {
        type_name: tn,
        fields,
    } = value
    {
        if tn == &q.type_name {
            if let Some(field_val) = fields.get(&q.field_name) {
                if let Ok(fv) = field_val.as_float() {
                    let matches = match q.op.as_str() {
                        "gt" => fv > q.threshold,
                        "lt" => fv < q.threshold,
                        "ge" => fv >= q.threshold,
                        "le" => fv <= q.threshold,
                        "eq" => (fv - q.threshold).abs() < 1e-9,
                        _ => return Err(format!("find(): unknown operator '{}'", q.op)),
                    };
                    return Ok(matches);
                }
            }
        }
    }
    Ok(false)
}

// ── inspect (ADR-0051, the pattern stats surface) ───────────────────

/// The shared validation prologue: the pattern-name argument with the
/// two exact error texts both backends carried. The stats lookup stays
/// per-backend (each reads its own learnables/patterns/pattern_stats).
pub fn inspect_pattern_name(args: &[Value]) -> Result<String, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!(
            "inspect() expected String pattern name, got {}",
            other.type_name()
        )),
        None => Err("inspect() requires 1 argument (pattern name)".to_string()),
    }
}

// ── resolve_skill_index (Problem A) ─────────────────────────────────

/// The shared validation prologue: the department argument with the
/// exact error text. The index lookup stays per-backend (the TW keeps
/// the indices in a `HashMap`, the VM in a `Vec` — different shapes,
/// the same not-found contract).
pub fn skill_dept(args: &[Value]) -> Result<String, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err("resolve_skill_index() expects a department name (String)".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }

    #[test]
    fn handles_covers_exactly_the_eight_names() {
        for name in [
            NAME_EXEC,
            NAME_FIND,
            NAME_FIT_TO_BUDGET,
            NAME_INSPECT,
            NAME_JSON_BODY,
            NAME_REQUIRE,
            NAME_RESOLVE_SKILL_INDEX,
            NAME_SERVER_PATH_PARAM,
        ] {
            assert!(handles(name), "{name} must be handled");
        }
        assert!(
            !handles("exec_argv"),
            "the VM-only twin is not in the group"
        );
        assert!(!handles("print"));
    }

    #[test]
    fn fit_to_budget_is_the_identity_on_lists() {
        let list = Value::List(vec![Value::Float(1.0), Value::Float(2.0)]);
        let out = fit_to_budget(std::slice::from_ref(&list)).expect("a list passes");
        match (out, list) {
            (Value::List(a), Value::List(b)) => assert_eq!(a.len(), b.len()),
            _ => panic!("expected lists"),
        }
        let err = fit_to_budget(&[s("nope")]).unwrap_err();
        assert_eq!(
            err,
            "fit_to_budget() expects first argument to be a List".to_string()
        );
        let err0 = fit_to_budget(&[]).unwrap_err();
        assert_eq!(
            err0,
            "fit_to_budget() expects first argument to be a List".to_string()
        );
    }

    #[test]
    fn require_grants_held_roles_and_denies_loudly() {
        let roles = vec!["admin".to_string(), "ops".to_string()];
        let ok = require(&roles, &[s("admin")]).expect("a held role passes");
        assert!(matches!(ok, Value::Bool(true)));
        let err = require(&roles, &[s("guest")]).unwrap_err();
        assert!(
            err.contains("require('guest'): access denied"),
            "the exact denial shape: {}",
            err
        );
        assert!(
            err.contains(r#"["admin", "ops"]"#),
            "the roles are listed: {}",
            err
        );
        // A missing/role-less argument defaults to "" and is denied.
        let err0 = require(&roles, &[]).unwrap_err();
        assert!(err0.contains("require(''): access denied"), "{}", err0);
    }

    #[test]
    fn json_body_clones_the_body_or_falls_back_to_empty_struct() {
        let body = Value::Struct {
            type_name: "Payload".to_string(),
            fields: HashMap::new(),
        };
        match json_body(Some(&body)) {
            Value::Struct { type_name, .. } => assert_eq!(type_name, "Payload"),
            other => panic!("expected the cloned body, got {:?}", other),
        }
        match json_body(None) {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "JsonBody");
                assert!(fields.is_empty());
            }
            other => panic!("expected the fallback struct, got {:?}", other),
        }
    }

    #[test]
    fn server_path_param_returns_the_value_or_the_empty_string() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), "alice".to_string());
        let params = Some(&map);
        match server_path_param(params, &[s("name")]) {
            Value::String(v) => assert_eq!(v, "alice"),
            other => panic!("expected the param value, got {:?}", other),
        }
        // No such param → empty string (the №283 parity contract).
        match server_path_param(params, &[s("absent")]) {
            Value::String(v) => assert_eq!(v, ""),
            other => panic!("expected an empty string, got {:?}", other),
        }
        // No server context at all → empty string.
        match server_path_param(None, &[s("name")]) {
            Value::String(v) => assert_eq!(v, ""),
            other => panic!("expected an empty string, got {:?}", other),
        }
        // A non-string argument defaults to "" → the empty lookup.
        match server_path_param(params, &[Value::Float(1.0)]) {
            Value::String(v) => assert_eq!(v, ""),
            other => panic!("expected an empty string, got {:?}", other),
        }
    }

    #[test]
    fn find_args_validates_all_four_with_the_exact_texts() {
        let good = find_args(&[s("Task"), s("done"), s("eq"), Value::Float(1.0)]);
        assert!(good.is_ok());
        let e0 = find_args(&[]).unwrap_err();
        assert_eq!(
            e0,
            "find() requires type name as first argument (String)".to_string()
        );
        let e1 = find_args(&[s("Task")]).unwrap_err();
        assert_eq!(
            e1,
            "find() requires field name as second argument (String)".to_string()
        );
        let e2 = find_args(&[s("Task"), s("done")]).unwrap_err();
        assert_eq!(
            e2,
            "find() requires operator as third argument (String: gt/lt/ge/le/eq)".to_string()
        );
        let e3 = find_args(&[s("Task"), s("done"), s("eq")]).unwrap_err();
        assert_eq!(
            e3,
            "find() requires threshold as fourth argument (Float)".to_string()
        );
    }

    fn task(done: f64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("done".to_string(), Value::Float(done));
        Value::Struct {
            type_name: "Task".to_string(),
            fields,
        }
    }

    #[test]
    fn find_matches_covers_every_operator_and_the_unknown_one() {
        let q_eq = find_args(&[s("Task"), s("done"), s("eq"), Value::Float(1.0)]).unwrap();
        assert!(find_matches(&task(1.0), &q_eq).unwrap());
        assert!(!find_matches(&task(0.5), &q_eq).unwrap());
        let q_lt = find_args(&[s("Task"), s("done"), s("lt"), Value::Float(1.0)]).unwrap();
        assert!(find_matches(&task(0.5), &q_lt).unwrap());
        let q_gt = find_args(&[s("Task"), s("done"), s("gt"), Value::Float(0.4)]).unwrap();
        assert!(find_matches(&task(0.5), &q_gt).unwrap());
        let q_ge = find_args(&[s("Task"), s("done"), s("ge"), Value::Float(0.5)]).unwrap();
        assert!(find_matches(&task(0.5), &q_ge).unwrap());
        let q_le = find_args(&[s("Task"), s("done"), s("le"), Value::Float(0.5)]).unwrap();
        assert!(find_matches(&task(0.5), &q_le).unwrap());
        // The eq tolerance: 1e-9.
        let q_tol = find_args(&[s("Task"), s("done"), s("eq"), Value::Float(1.0)]).unwrap();
        assert!(find_matches(&task(1.0 + 5e-10), &q_tol).unwrap());
        // A non-matching type never matches.
        assert!(!find_matches(&Value::Float(1.0), &q_eq).unwrap());
        // A missing field never matches.
        let empty = Value::Struct {
            type_name: "Task".to_string(),
            fields: HashMap::new(),
        };
        assert!(!find_matches(&empty, &q_eq).unwrap());
        // The unknown operator is a loud shared error.
        let bad = find_args(&[s("Task"), s("done"), s("like"), Value::Float(1.0)]).unwrap();
        let err = find_matches(&task(1.0), &bad).unwrap_err();
        assert_eq!(err, "find(): unknown operator 'like'".to_string());
    }

    #[test]
    fn inspect_validation_carries_the_exact_texts() {
        assert_eq!(inspect_pattern_name(&[s("Main")]).unwrap(), "Main");
        let e_non_string = inspect_pattern_name(&[Value::Float(1.0)]).unwrap_err();
        assert_eq!(
            e_non_string,
            "inspect() expected String pattern name, got Float".to_string()
        );
        let e_missing = inspect_pattern_name(&[]).unwrap_err();
        assert_eq!(
            e_missing,
            "inspect() requires 1 argument (pattern name)".to_string()
        );
    }

    #[test]
    fn skill_dept_validation_carries_the_exact_text() {
        assert_eq!(skill_dept(&[s("osp")]).unwrap(), "osp");
        let err = skill_dept(&[]).unwrap_err();
        assert_eq!(
            err,
            "resolve_skill_index() expects a department name (String)".to_string()
        );
        let err2 = skill_dept(&[Value::Float(1.0)]).unwrap_err();
        assert_eq!(
            err2,
            "resolve_skill_index() expects a department name (String)".to_string()
        );
    }
}
