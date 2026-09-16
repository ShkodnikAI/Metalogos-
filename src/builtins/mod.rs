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
/// Наряд №182: shared f64 math primitives (sigmoid_raw, softmax_raw) used by
/// both `math.rs` (builtin_sigmoid/builtin_softmax) and `nn/activation.rs`.
pub(crate) mod math_core;
use math::*;
pub(crate) mod collections;
use collections::*;
pub mod string;
use string::*;
// Наряд №274 (ADR-0136): core-функция маскирования публична для fuzz-цели
// (конвенция №256) — metalogos::builtins::redact_string.
pub use string::redact_string;
// Наряд №284 (P1, M1): canary-токены недоверенного текста — чистые ядра
// публичны для тестов и fuzz (конвенция №256); модуль pub(crate),
// контрактная поверхность — два билтина в реестре.
pub(crate) mod canary;
use canary::*;
pub use canary::{
    canary_check_core, canary_insert_core, is_canary_id, CanaryCheck, CanaryMark, CANARY_PREFIX,
};
pub(crate) mod crypto;
use crypto::*;
pub(crate) mod json;
use json::*;
pub(crate) mod llm;
use llm::*;
// Наряд №269: structured LLM output — pure validator/loop exported for tests
// and embedders (ADR-0133); the module itself stays pub(crate).
pub(crate) mod llm_schema;
use llm_schema::*;
pub use llm_schema::{
    call_llm_schema_core, check_schema_supported, mock_instance_from_schema,
    schema_retries_from_env, validate_json_against_schema,
};
// Наряд №286 (P2, M1): json_validate — валидатор ADR-0133 как standalone
// builtin («shape-before-use»); чистое ядро публично для тестов (лекало
// canary №284). Модуль pub(crate), контрактная поверхность — один билтин.
pub(crate) mod json_validate;
use json_validate::*;
pub use json_validate::{json_validate_core, JsonValidateResult};
pub(crate) mod http;
use http::*;
pub use http::{check_url_ssrf, is_blocked_address};
// Наряд №268: MCP stdio-клиент (ADR-0132, Accepted) — модуль pub(crate),
// контрактная поверхность — два билтина в реестре.
pub(crate) mod mcp;
use mcp::*;
pub(crate) mod memory;
pub use memory::init_kv_persist;
use memory::*;
// Наряд №272 (ADR-0134): векторный контур — embed / vec_store / vec_search.
// Feature-gate `vec` off-by-default (паттерн candle/vision, ADR-0104);
// в `portable` включён (ADR-0134 D3).
#[cfg(feature = "vec")]
pub(crate) mod vector;
pub use memory::{reset_session_store, session_key_count, session_store_count};
#[cfg(feature = "vec")]
use vector::*;
// Наряд №281 (P2, M2): user_profile — детерминированная выжимка
// контейнера (static/dynamic/buckets) из KV-записей container:<c>:...
// с ин-процессным кэшем (поколение KV-записей + mtime файла).
// БЕЗ feature-гейта: kv-контур ядровой, vec не нужен.
pub(crate) mod profile;
use profile::*;
pub use profile::{user_profile_core, CONTAINER_PREFIX};
// Наряд №280 (P2, M2): memory_forget — управляемое забывание с границами
// (dry_run-превью → apply по явным ids; soft-delete ledger с batch_id;
// vec_search получает include_forgotten). Тот же feature-gate `vec`.
#[cfg(feature = "vec")]
pub(crate) mod memory_forget;
#[cfg(feature = "vec")]
use memory_forget::*;
#[cfg(feature = "vec")]
pub use memory_forget::{memory_forget_core, FORGET_BATCH_PREFIX};
// Наряд №285 (P2, feature/memory): text_chunk — структура-осознанное
// чанкование для RAG-пайплайна (каскад разделителей + overlap + слияние,
// markdown-секции с header_path). Без feature-гейта: чистая строковая
// функция; token-бюджет — реюз token_count (memory.rs SSOT-estimate).
pub(crate) mod text_chunk;
use text_chunk::*;
pub(crate) mod cron;
pub use cron::init_reminder_persist;
use cron::*;
// Наряд №253: exec-гейт serve-контекста — публичный контракт для тестов
// и эмбеддеров (сам модуль io остаётся pub(crate)).
pub use io::{current_exec_context, env_gate, exec_gate, ExecContext, ServeRouteExecGuard};
pub mod pdf;
pub use pdf::*;
pub(crate) mod regex;
use regex::*;

// Наряд №179b: Reflex training/prediction builtins.
// Наряд №180:   Reflex persistence (save/load to SQLite ADR-0116).
// Handlers are stubs — real dispatch is in interpreter::execution::invoke()
// and interpreter::reflex_builtin::invoke_reflex_* (for FnCall
// expressions inside pattern bodies). The dispatch functions are pub so
// the interpreter can call them; the stubs are pub(crate) so the spec!
// macro in registry.rs can reference them.
pub mod reflex;
pub mod vision;
// Наряд №331 (ADR-0162): unified media layer — media_store_* / media_save /
// media_retain / media_release / media_meta. NOT feature-gated: the store,
// handles, and at-rest sealing have no inference-stack dependencies (mirrors
// the vision-store reasoning: the contract is testable in the default build).
pub mod media;
// Наряд №333 (ADR-0163): backend registry builtins — backend_list().
pub mod backends;
// Наряд №335 (spec §7.2 v2): consent grant/revoke + quarantine sink +
// ledger export — the consent component's language surface.
pub mod consent;
// Наряд №275 (ADR-0137): LLM streaming builtins — llm_stream_open/next/close.
// Module is NOT feature-gated: the opaque handle + registry + SSE parser
// live in `crate::llm` (always available); HTTP streaming requires
// `reqwest::blocking` (always available, no extra feature flag).
pub mod llm_stream;
#[cfg(feature = "candle")]
pub use reflex::{build_reflex_gen_model, build_reflex_seq_model};
pub use reflex::{
    build_reflex_model, reflex_generate_dispatch, reflex_list_dispatch, reflex_load_dispatch,
    reflex_metrics_dispatch, reflex_predict_dispatch, reflex_save_dispatch, reflex_train_dispatch,
};
pub use reflex::{
    builtin_reflex_bpe_decode, builtin_reflex_bpe_encode, builtin_reflex_bpe_save,
    builtin_reflex_bpe_train, builtin_reflex_detokenize, builtin_reflex_tokenize,
};
// Наряд №240 (Vision R4.2): real-path dispatch functions (shared by the
// interpreter and the VM) + last-resort registry stubs.
// Наряд №242 (R6.1): vision_save/vision_load dispatches joined the family
// (state-carrying + the program's db connection).
// Наряд №243 (R6.2): vision_edit dispatch joined the family
// (state-carrying; signed-source contract).
// Наряд №244 (R6.3): LoRA dispatches joined the family (SQLite BLOB store,
// composite provenance). The gated sign/insert contract function
// (`vision_generate_sign_and_insert`) is NOT re-exported here — like its
// №243 лекало, tests import it via the module path.
pub use vision::{
    vision_edit_check_dims_r41, vision_edit_check_dims_vae_factor, vision_edit_dispatch,
    vision_export_dispatch, vision_export_raw_dispatch, vision_generate_dispatch,
    vision_list_dispatch, vision_load_dispatch, vision_lora_check_adapter_path,
    vision_lora_composite_model_sha256, vision_lora_generate_dispatch, vision_lora_load_dispatch,
    vision_save_dispatch,
};
// Наряд №331 (ADR-0162): media dispatch functions (shared TW + VM).
// The last-resort registry stubs are pub(crate) — registry.rs imports
// them directly from the module (лекало vision).
pub use media::{
    media_bind_origin_dispatch, media_manifest_dispatch, media_meta_dispatch,
    media_release_dispatch, media_retain_dispatch, media_save_dispatch,
    media_source_capture_dispatch, media_store_dispatch,
};
// Наряд №333 (ADR-0163): backend registry listing.
pub use backends::builtin_backend_list;

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

    /// Наряд №287: заменить хендлер билтина на пользовательский
    /// (doc-тесты: read-only профиль — сетевые/exec-заглушки).
    /// Реестр НЕ меняется (SSOT нетронут) — подмена только в этом
    /// экземпляре Builtins данного Interpreter.
    pub fn override_handler(&mut self, name: &str, f: BuiltinFn) {
        self.funcs.insert(name.to_string(), f);
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
