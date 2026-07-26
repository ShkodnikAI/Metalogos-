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

    #[error("sandbox violation")]
    Sandbox,
}

impl RuntimeError {
    pub fn parse_msg(rule: &str, detail: &str) -> Self {
        Self::Parse(format!("{}: {}", rule, detail))
    }
}
