//! Backend registry builtins (Наряд №333, ADR-0163).
//!
//! `backend_list()` — read-only metadata over the static backend SSOT
//! (`src/backends.rs`): name, class, weights id, pin state, license
//! class + note. No weights bytes exist behind these entries (PARKED
//! №294); the listing is for tooling, docs, and in-program governance
//! checks. Stateless — no interception needed.

use crate::interpreter::Value;

/// `backend_list()` — the static backend registry as
/// `List[Struct { name, class, weights_id, pin, license, license_note }]`.
pub fn builtin_backend_list(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "backend_list: expects 0 arguments, got {}",
            args.len()
        ));
    }
    let items = crate::backends::BACKEND_REGISTRY
        .iter()
        .map(|e| {
            let mut fields = std::collections::HashMap::new();
            fields.insert("name".to_string(), Value::String(e.name.to_string()));
            fields.insert(
                "class".to_string(),
                Value::String(e.class.as_str().to_string()),
            );
            fields.insert(
                "weights_id".to_string(),
                Value::String(e.weights_id.to_string()),
            );
            fields.insert("pin".to_string(), Value::String(e.pin.as_str().to_string()));
            fields.insert(
                "license".to_string(),
                Value::String(e.license.as_str().to_string()),
            );
            fields.insert(
                "license_note".to_string(),
                Value::String(e.license_note.to_string()),
            );
            Value::Struct {
                type_name: "BackendEntry".to_string(),
                fields,
            }
        })
        .collect();
    Ok(Value::List(items))
}
