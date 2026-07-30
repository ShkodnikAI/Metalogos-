/// Unified runtime error type for Metalogos.
/// ADR: RuntimeError is for Rust-level failures (IO, lock poisoning, parse errors in production paths).
/// Soft language-level failures (type mismatches, undefined variables) are reported via
/// interpreter::Value::Unit / Result<Value, String> and are NOT represented here.
#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("llm error: {0}")]
    Llm(String),

    #[error("lock poisoned: {0}")]
    Lock(String),

    #[error("sqlite error: {0}")]
    Sqlite(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("sandbox violation")]
    Sandbox,
}

impl RuntimeError {
    pub fn parse_msg(rule: &str, detail: &str) -> Self {
        Self::Parse(format!("{}: {}", rule, detail))
    }
}

/// Helper macro for acquiring mutex/rwlock guards in production code.
/// Replaces `.lock().unwrap()` with proper error propagation.
/// Наряд №29 §3.3
#[macro_export]
macro_rules! lock_or_err {
    ($lock_expr:expr) => {
        $lock_expr
            .lock()
            .map_err(|e| $crate::error::RuntimeError::Lock(e.to_string()))
    };
}
