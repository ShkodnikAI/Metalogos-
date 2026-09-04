// ── Built-in functions for METALOGOS M1+M2 ────────────────────────────

use crate::interpreter::Value;
pub type BuiltinFn = fn(&[Value]) -> Result<Value, String>;

/// Registry of built-in functions.
pub struct Builtins {
    funcs: std::collections::HashMap<String, BuiltinFn>,
}

/// Metadata for a single builtin function.
/// This is the SINGLE SOURCE OF TRUTH for all builtin metadata AND handler.
/// Every consumer (compiler, VM, semantic, runtime) reads from here.
///
/// - `name`: function name as exposed to the DSL
/// - `arity`: minimum arity; 0 = variadic (skip arity check)
/// - `max_arity`: None = exact match (arity is exact), Some(M) = accepts arity..=M
/// - `category`: logical group for documentation and error messages
/// - `layer`: architectural layer — "core", "platform", or "ext"
/// - `handler`: the Rust function that implements this builtin.
///   `None` = осознанная заглушка (stub — no runtime handler, e.g.
///   historical placeholders kept for bytecode index stability).
#[derive(Debug, Clone)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arity: usize,             // minimum arity; 0 = variadic (skip arity check)
    pub max_arity: Option<usize>, // None = exact match (arity is exact), Some(M) = accepts arity..=M
    pub category: &'static str,
    pub layer: &'static str, // "core" | "platform" | "ext"; default "core"
    pub handler: Option<BuiltinFn>, // None = stub (intentionally no handler)
}

/// Macro for concise BuiltinSpec construction.
///
/// Stub variants (no handler — `handler: None`):
///   spec!("name", N, "cat")                  → exact arity, core layer
///   spec!("name", N, M, "cat")               → range N..=M, core layer
///   spec!("name", N, "cat" => "ext")         → exact arity, explicit layer
///   spec!("name", N, M, "cat" => "ext")      → range, explicit layer
///
/// Handler variants (use `;` to separate handler from metadata):
///   spec!("name", N, "cat"; handler)         → exact arity, core layer, handler
///   spec!("name", N, M, "cat"; handler)       → range, core layer, handler
///   spec!("name", N, "cat" => "ext"; handler) → exact arity, explicit layer, handler
///   spec!("name", N, M, "cat" => "ext"; handler) → range, explicit layer, handler
#[macro_export]
macro_rules! spec {
    // ── Handler variants (most specific first) ──
    ($name:expr, $arity:expr, $max:expr, $cat:expr => $layer:expr; $handler:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: Some($max),
            category: $cat,
            layer: $layer,
            handler: Some($handler as $crate::builtins::BuiltinFn),
        }
    };
    ($name:expr, $arity:expr, $cat:expr => $layer:expr; $handler:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: None,
            category: $cat,
            layer: $layer,
            handler: Some($handler as $crate::builtins::BuiltinFn),
        }
    };
    ($name:expr, $arity:expr, $max:expr, $cat:expr; $handler:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: Some($max),
            category: $cat,
            layer: "core",
            handler: Some($handler as $crate::builtins::BuiltinFn),
        }
    };
    ($name:expr, $arity:expr, $cat:expr; $handler:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: None,
            category: $cat,
            layer: "core",
            handler: Some($handler as $crate::builtins::BuiltinFn),
        }
    };
    // ── Stub variants (handler: None) ──
    ($name:expr, $arity:expr, $max:expr, $cat:expr => $layer:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: Some($max),
            category: $cat,
            layer: $layer,
            handler: None,
        }
    };
    ($name:expr, $arity:expr, $cat:expr => $layer:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: None,
            category: $cat,
            layer: $layer,
            handler: None,
        }
    };
    ($name:expr, $arity:expr, $max:expr, $cat:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: Some($max),
            category: $cat,
            layer: "core",
            handler: None,
        }
    };
    ($name:expr, $arity:expr, $cat:expr) => {
        $crate::builtins::BuiltinSpec {
            name: $name,
            arity: $arity,
            max_arity: None,
            category: $cat,
            layer: "core",
            handler: None,
        }
    };
}

pub(crate) mod core;
use core::*;
pub(crate) mod io;
use io::*;
pub(crate) mod registry;
pub use registry::*;
pub(crate) mod math;
use math::*;
pub(crate) mod collections;
use collections::*;
pub(crate) mod string;
use string::*;
pub(crate) mod crypto;
use crypto::*;
pub(crate) mod json;
use json::*;
pub(crate) mod llm;
use llm::*;
pub(crate) mod http;
use http::*;
pub use http::{check_url_ssrf, is_blocked_address};
pub(crate) mod memory;
pub use memory::init_kv_persist;
use memory::*;
pub use memory::{reset_session_store, session_key_count, session_store_count};
pub(crate) mod cron;
pub use cron::init_reminder_persist;
use cron::*;
pub mod pdf;
pub use pdf::*;
pub(crate) mod regex;
use regex::*;

impl Default for Builtins {
    fn default() -> Self {
        Self::new()
    }
}

impl Builtins {
    pub fn new() -> Self {
        // Наряд №170: SSOT — all handlers are in BUILTIN_REGISTRY.
        // No manual funcs.insert calls needed. The registry is the
        // single source of truth for both metadata and handlers.
        let mut funcs = std::collections::HashMap::with_capacity(BUILTIN_REGISTRY.len());
        for spec in BUILTIN_REGISTRY {
            if let Some(h) = spec.handler {
                funcs.insert(spec.name.to_string(), h);
            }
        }

        Builtins { funcs }
    }

    /// Verify builtin registry consistency (debug builds).
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    fn check_registry_sync(&self) {
        for spec in BUILTIN_REGISTRY.iter() {
            if spec.category != "stateful"
                && spec.category != "stub"
                && spec.category != "graph"
                && spec.category != "mtree"
                && spec.category != "cron"
                && spec.category != "test"
            {
                assert!(
                    self.funcs.contains_key(spec.name),
                    "BUILTIN_REGISTRY '{}' has no handler in Builtins::new()",
                    spec.name
                );
            }
        }
    }

    /// Look up a built-in by name.
    pub fn get(&self, name: &str) -> Option<&BuiltinFn> {
        self.funcs.get(name)
    }

    /// Return the set of function names registered in the dispatcher.
    /// Used by registry_sync_check test to detect funcs.insert without paired spec!.
    pub fn dispatcher_names(&self) -> std::collections::HashSet<String> {
        self.funcs.keys().cloned().collect()
    }
}

pub(crate) mod server;
use server::*;
pub(crate) mod office;
use office::*;
pub(crate) mod email;
use email::*;
pub(crate) mod calendar;
use calendar::*;
pub(crate) mod contacts;
use contacts::*;

// Наряд №74 / №111: SVG Graphics & Diagrams (feature-gated)
#[cfg(any(feature = "svg", feature = "chart", feature = "diagram"))]
pub(crate) mod svg;
#[cfg(any(feature = "svg", feature = "chart", feature = "diagram"))]
use svg::*;
// Наряд №86 / №111: Mini template engine
#[cfg(feature = "template")]
pub(crate) mod template;
#[cfg(feature = "template")]
use template::*;
#[cfg(test)]
mod tests;
