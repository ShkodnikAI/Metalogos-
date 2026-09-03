//! Pattern-name collision detection for `mlog run` / `mlog serve`.
//!
//! Origins and the strict flag are stored in `module_namespaces` under
//! reserved `__n163_*` keys so this module does not need new Interpreter
//! fields. Diagnostics reuse the `duplicate pattern:` prefix that
//! `mlog check` already emits. ADR-0113.

use super::*;

const STRICT_KEY: &str = "__n163_strict";
const ORIGIN_PREFIX: &str = "__n163_origin::";
const LEARNABLE_PREFIX: &str = "__n163_learnable::";
const WARN_PREFIX: &str = "__n163_warn::";

/// Format a duplicate-pattern diagnostic.
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

impl Interpreter {
    /// Module path currently being loaded, or `"<program>"` for the entry file.
    pub(super) fn current_origin(&self) -> &str {
        self.loading_stack
            .last()
            .map(|s| s.as_str())
            .unwrap_or("<program>")
    }

    /// Opt into failing the load when two modules register the same pattern
    /// name. Default is warning-only so existing deployments keep starting.
    pub fn set_strict_pattern_names(&mut self, strict: bool) {
        if strict {
            self.module_namespaces
                .insert(STRICT_KEY.to_string(), "1".to_string());
        } else {
            self.module_namespaces.remove(STRICT_KEY);
        }
    }

    fn is_strict_pattern_names(&self) -> bool {
        if self.module_namespaces.get(STRICT_KEY).map(|s| s.as_str()) == Some("1") {
            return true;
        }
        match std::env::var("METALOGOS_STRICT") {
            Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
            Err(_) => false,
        }
    }

    /// Warnings produced by silent HashMap overwrites of pattern names.
    pub fn name_collision_warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (k, v) in &self.module_namespaces {
            if k.starts_with(WARN_PREFIX) {
                out.push(v.clone());
            }
        }
        out.sort();
        out
    }

    fn record_warning(&mut self, msg: String) {
        let idx = self
            .module_namespaces
            .keys()
            .filter(|k| k.starts_with(WARN_PREFIX))
            .count();
        self.module_namespaces
            .insert(format!("{WARN_PREFIX}{idx}"), msg);
    }

    /// Register a compiled pattern, warning (or erroring, if strict) when
    /// the name is already bound to a *different* origin.
    pub(super) fn register_pattern(
        &mut self,
        name: String,
        pat: CompiledPattern,
        origin: &str,
    ) -> Result<(), String> {
        let key = format!("{ORIGIN_PREFIX}{name}");
        if let Some(prev) = self.module_namespaces.get(&key) {
            if prev != origin {
                let msg = format_duplicate_pattern(&name, Some(prev.as_str()), Some(origin));
                if self.is_strict_pattern_names() {
                    return Err(msg);
                }
                eprintln!("warning: {msg}");
                self.record_warning(msg);
            }
        }
        self.module_namespaces.insert(key, origin.to_string());
        self.patterns.insert(name, pat);
        Ok(())
    }

    pub(super) fn register_learnable(
        &mut self,
        name: String,
        pat: CompiledLearnable,
        origin: &str,
    ) -> Result<(), String> {
        let key = format!("{LEARNABLE_PREFIX}{name}");
        if let Some(prev) = self.module_namespaces.get(&key) {
            if prev != origin {
                let msg = format!(
                    "{} (already defined in {prev}, redefined in {origin})",
                    format_duplicate_learnable(&name)
                );
                if self.is_strict_pattern_names() {
                    return Err(msg);
                }
                eprintln!("warning: {msg}");
                self.record_warning(msg);
            }
        }
        self.module_namespaces.insert(key, origin.to_string());
        self.learnable_patterns.insert(name, pat);
        Ok(())
    }
}
