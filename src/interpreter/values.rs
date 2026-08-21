use std::collections::HashMap;
use zeroize::Zeroizing;

/// A single variant inside a Fluid value (runtime). Contains a concrete
/// value, its declared type name, and a confidence score (0.0..1.0).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FluidValueVariant {
    pub type_name: String,
    pub value: Value,
    pub confidence: f64,
}

/// Opaque secret string with automatic memory zeroing on drop (Phase 7.3).
/// Implements serde by serializing as "[SECRET]" marker — actual value is NEVER persisted.
#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);

impl serde::Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Never serialize the actual secret — emit a safe marker
        s.serialize_str("[SECRET]")
    }
}

impl<'de> serde::Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let inner = String::deserialize(d)?;
        Ok(SecretString(Zeroizing::new(inner)))
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString([REDACTED])")
    }
}

impl SecretString {
    pub fn new(s: String) -> Self {
        SecretString(Zeroizing::new(s))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Runtime value.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
    String(String),
    Float(f64),
    Bool(bool),
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    /// List value: ordered collection of items.
    List(Vec<Value>),
    /// Fluid value: superposition of typed variants with confidence scores.
    /// Collapses lazily at point of use (see `maybe_collapse`).
    Fluid(Vec<FluidValueVariant>),
    Unit,
    /// Opaque HTML content (Phase 6.2) — cannot be concatenated, printed, or converted to String
    Html(String),
    /// Opaque SQL query (Phase 6.3) — only created via query() builtin
    Query(String),
    /// Opaque secret value (Phase 6.4) — cannot be printed or converted to String.
    /// Phase 7.3: Internally uses SecretString (Zeroizing<String>) — memory is zeroed on drop.
    Secret(SecretString),
    /// Opaque encrypted data (Phase 6.4)
    Encrypted(Vec<u8>),
    /// Opaque password hash (Phase 6.4)
    Hash(String),
    /// Opaque session data (Phase 6.5)
    Session(std::collections::HashMap<String, String>),
    /// HTTP response value (Phase 6.1)
    HttpResponse {
        status: u16,
        body: String,
    },
    /// Graph subgraph — opaque first-class graph value (V3).
    /// Contains a serializable GraphSnapshot that can be passed between functions.
    Subgraph(crate::memory_graph::GraphSnapshot),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Struct { type_name, fields } => {
                write!(f, "{} {{", type_name)?;
                let pairs: Vec<_> = fields.iter().collect();
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Fluid(variants) => {
                // Display as the highest-confidence variant
                let best = variants.iter().max_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                match best {
                    Some(v) => write!(f, "{}", v.value),
                    None => write!(f, "()"),
                }
            }
            Value::Unit => write!(f, "()"),
            Value::Html(_) => write!(f, "[Html]"),
            Value::Query(_) => write!(f, "[Query]"),
            Value::Secret(_) => write!(f, "[Secret]"),
            Value::Encrypted(_) => write!(f, "[Encrypted]"),
            Value::Hash(_) => write!(f, "[Hash]"),
            Value::Session(_) => write!(f, "[Session]"),
            Value::HttpResponse { status, .. } => write!(f, "[HttpResponse {}]", status),
            Value::Subgraph(snap) => write!(
                f,
                "[Subgraph {} nodes, {} edges]",
                snap.nodes.len(),
                snap.edges.len()
            ),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "String",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::List(_) => "List",
            Value::Struct { .. } => "Struct",
            Value::Fluid(_) => "Fluid",
            Value::Unit => "Unit",
            Value::Html(_) => "Html",
            Value::Query(_) => "Query",
            Value::Secret(_) => "Secret",
            Value::Encrypted(_) => "Encrypted",
            Value::Hash(_) => "Hash",
            Value::Session(_) => "Session",
            Value::HttpResponse { .. } => "HttpResponse",
            Value::Subgraph(_) => "Subgraph",
        }
    }

    /// Get a field value from a struct. Returns Err if not a struct or field missing.
    pub fn get_field(&self, field: &str) -> Result<&Value, String> {
        match self {
            Value::Struct { fields, .. } => fields
                .get(field)
                .ok_or_else(|| format!("field '{}' not found on struct", field)),
            _ => Err(format!(
                "cannot access field '{}' on non-struct value ({})",
                field,
                self.type_name()
            )),
        }
    }

    /// Set a field value on a mutable struct.
    pub fn set_field(&mut self, field: &str, value: Value) -> Result<(), String> {
        match self {
            Value::Struct { fields, .. } => {
                if fields.contains_key(field) {
                    fields.insert(field.to_string(), value);
                    Ok(())
                } else {
                    Err(format!("field '{}' not found on struct", field))
                }
            }
            _ => Err(format!("cannot set field '{}' on non-struct value", field)),
        }
    }

    /// Convert to f64 for numeric comparisons.
    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::String(s) => s
                .parse::<f64>()
                .map_err(|_| format!("cannot convert '{}' to Float", s)),
            _ => Err(format!("cannot convert {} to Float", self.type_name())),
        }
    }

    /// Convert to bool for condition checking.
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Float(f) => Ok(*f != 0.0),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Unit => Ok(false),
            _ => Err(format!("cannot convert {} to Bool", self.type_name())),
        }
    }

    /// Check if this value is a Fluid superposition.
    pub fn is_fluid(&self) -> bool {
        matches!(self, Value::Fluid(_))
    }
}


/// Opaque / sensitive values that must not be rendered by print.
/// Наряд №114.
pub fn is_nonprintable(v: &Value) -> bool {
    matches!(
        v,
        Value::Html(_)
            | Value::Query(_)
            | Value::Secret(_)
            | Value::Encrypted(_)
            | Value::Hash(_)
            | Value::Subgraph(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_never_leaks() {
        let secret = SecretString::new("super-secret-value-12345".to_string());
        let debug_output = format!("{:?}", secret);
        assert!(
            !debug_output.contains("super-secret-value-12345"),
            "Debug output must not contain the actual secret value"
        );
        assert_eq!(debug_output, "SecretString([REDACTED])");
    }

    #[test]
    fn value_secret_debug_never_leaks() {
        let value = Value::Secret(SecretString::new("another-secret".to_string()));
        let debug_output = format!("{:?}", value);
        assert!(
            !debug_output.contains("another-secret"),
            "Value Debug output must not contain the actual secret value"
        );
    }
}
