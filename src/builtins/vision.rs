//! Vision builtins — loud stubs (Наряд №210, ADR-0124).
//!
//! These stub handlers are registered in `BUILTIN_REGISTRY` for bytecode
//! index stability and arity checks — same pattern as the Reflex stubs
//! (`src/builtins/reflex.rs:64-124`). The real dispatch will be added
//! when the vision inference stack lands (R3, naryad 212, per ADR-0122/0123).
//!
//! Each handler is a **loud refusal**: it returns `Err` with a message
//! naming the naryad and ADR. It does NOT return a placeholder value,
//! does NOT silently succeed, and does NOT `panic!`.

use crate::interpreter::Value;

/// `vision_generate(model_name, prompt, seed) -> Vision`
///
/// Generates a vision artifact from a text prompt. **Not implemented** in
/// this build — real implementation lands in R3 (naryad 212, ADR-0122/0123).
pub(crate) fn builtin_vision_generate_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_generate: Vision generation is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R3, naryad 212, \
         per ADR-0122/0123). This is a loud refusal, not a placeholder result."
            .to_string(),
    )
}

/// `vision_edit(handle, prompt) -> Vision`
///
/// Edits an existing vision artifact. **Not implemented** — R5/R6 (naryad 214/215).
pub(crate) fn builtin_vision_edit_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_edit: Vision editing is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R5/R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
            .to_string(),
    )
}

/// `vision_export(handle, path) -> String`
///
/// Exports a vision artifact to a file. **Not implemented** — R5/R6 (naryad 214/215).
pub(crate) fn builtin_vision_export_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_export: Vision export is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R5/R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
            .to_string(),
    )
}

/// `vision_list() -> List<String>`
///
/// Returns the names of all saved vision artifacts. Currently returns an
/// **empty list** — the registry exists but has no named artifacts in R1.
/// This is an honest answer from an empty registry, not a stub value.
///
/// **Note:** unlike the other vision_* stubs, this one does NOT return Err.
/// An empty list is a valid, truthful response when no artifacts exist.
/// When artifacts are added (R3+), this will return their names.
pub(crate) fn builtin_vision_list_stub(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::List(vec![]))
}

/// `vision_save(handle, name) -> String`
///
/// Saves a vision artifact under a name. **Not implemented** — R5/R6 (naryad 214/215).
pub(crate) fn builtin_vision_save_stub(_args: &[Value]) -> Result<Value, String> {
    Err("vision_save: Vision save is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R5/R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
        .to_string())
}

/// `vision_load(name) -> Vision`
///
/// Loads a previously saved vision artifact. **Not implemented** — R5/R6 (naryad 214/215).
pub(crate) fn builtin_vision_load_stub(_args: &[Value]) -> Result<Value, String> {
    Err("vision_load: Vision load is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R5/R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
        .to_string())
}
