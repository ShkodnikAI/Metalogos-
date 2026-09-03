//! Shared pattern-name collision diagnostics.
//!
//! `mlog check` and the interpreter loader (`mlog run` / `mlog serve`)
//! must emit the same `duplicate pattern:` prefix so operators can grep
//! either path. ADR-0113.

/// Format a duplicate-pattern diagnostic.
///
/// When both origins are present the message names the two modules;
/// otherwise it matches the historical `mlog check` wording.
pub fn format_duplicate_pattern(
    name: &str,
    existing_origin: Option<&str>,
    incoming_origin: Option<&str>,
) -> String {
    match (existing_origin, incoming_origin) {
        (Some(prev), Some(new)) if !prev.is_empty() && !new.is_empty() => {
            format!("duplicate pattern: {name} (already defined in {prev}, redefined in {new})")
        }
        _ => format!("duplicate pattern: {name}"),
    }
}

/// Same wording family for learnable patterns.
pub fn format_duplicate_learnable(name: &str) -> String {
    format!("duplicate learnable pattern: {name}")
}
