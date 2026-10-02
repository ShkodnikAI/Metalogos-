//! The sequence-layer SPEC surface — the check-time registry and the
//! Value-argument builders (naryad №545, sub-step б).
//!
//! WHY THIS FILE EXISTS (naryad №545, issue #883): the
//! `SEQUENCE_LAYER_REGISTRY` and the `build_*` functions parse the
//! grammar's `layer_spec` arguments (`&[Value]` — наряд №178's shape),
//! which makes them LANGUAGE SURFACE, not domain machinery. The №545
//! crate split (metalogos + metalogos-reflex) moves the generative
//! machinery (the layer/model implementations) into the reflex crate;
//! the Value-speaking spec surface STAYS in the language crate — the
//! DbAccess №484 principle applied at the crate seam: the consumer
//! side owns the marshaling, the domain side owns the machinery.
//!
//! The builders construct the layer objects through the machinery's
//! Value-free constructors; the physical move (№545 (в)) turns those
//! constructors into cross-crate calls without changing this file's
//! contract.
//!
//! ## Feature gating
//!
//! The whole module is gated behind the `candle` feature (off by
//! default) — same posture as the machinery it builds (the mirror of
//! the №183/№184 gating, relocated verbatim from the machinery files).

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::sequence_layer::SequenceLayer;

/// Type alias for the build function signature.
/// Takes `&[Value]` (the same shape as `LayerSpec::build` from наряд №178)
/// so the grammar's `layer_spec = { IDENT ~ "(" ~ args ")" }` works
/// uniformly for both registries.
pub type SequenceLayerBuildFn =
    fn(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String>;

/// Specification for a sequence-layer type — analogous to `LayerSpec`
/// (наряд №178). The registry is the single source of truth for
/// available sequence-layer types.
pub struct SequenceLayerSpec {
    /// Layer type name (e.g. "attention", "rms_norm" in future naryads).
    pub name: &'static str,
    /// Parameter names in order (e.g. `&["heads", "dim"]`).
    /// Used for error messages and documentation.
    pub param_names: &'static [&'static str],
    /// Build function: takes Value args + seed, returns a boxed SequenceLayer.
    /// The seed is for deterministic weight init (xorshift64 from наряд №177,
    /// reused so the same declaration with the same seed always produces
    /// the same forward-pass result — Наряд №183 Contract 5).
    pub build: SequenceLayerBuildFn,
}

/// The sequence-layer registry — extensible without grammar changes
/// (ADR-0114 addendum principle, applied to the new category).
///
/// Наряд №183 shipped `attention`. Наряд №184 adds `rms_norm`,
/// `swiglu`, and `transformer_block`. Future naryads may add GQA if/when
/// authorized.
pub static SEQUENCE_LAYER_REGISTRY: &[SequenceLayerSpec] = &[
    SequenceLayerSpec {
        name: "attention",
        param_names: &["heads", "dim", "kv_heads?"],
        build: build_attention,
    },
    // Наряд №184 (Block 1): RmsNorm.
    SequenceLayerSpec {
        name: "rms_norm",
        param_names: &["dim", "eps?"],
        build: build_rmsnorm,
    },
    // Наряд №184 (Block 2): SwiGLU feedforward.
    SequenceLayerSpec {
        name: "swiglu",
        param_names: &["dim", "ff_dim"],
        build: build_swiglu,
    },
    // Наряд №184 (Block 3): full transformer block.
    SequenceLayerSpec {
        name: "transformer_block",
        param_names: &["heads", "dim", "ff_dim"],
        build: build_transformer_block,
    },
];

/// Look up a sequence-layer spec by name. Returns None if not found.
pub fn find_sequence_layer_spec(name: &str) -> Option<&'static SequenceLayerSpec> {
    SEQUENCE_LAYER_REGISTRY.iter().find(|s| s.name == name)
}

/// List all registered sequence-layer names (for error messages).
pub fn sequence_layer_names() -> Vec<&'static str> {
    SEQUENCE_LAYER_REGISTRY.iter().map(|s| s.name).collect()
}

// ── Forward-only builders (the №183/№188 registry entries) ───────────

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// `args` is the parsed `layer_arg` list from the grammar
/// (наряд №178's `layer_spec = { IDENT ~ "(" ~ layer_arg_list? ~ ")" }`).
///
/// Args:
///   - `attention(heads, dim)` → standard MHA (Наряд №183 backward compat)
///   - `attention(heads, dim, kv_heads)` → GQA (Наряд №188)
///
/// The 3rd arg is optional — when omitted, `n_kv_heads = n_heads`.
pub fn build_attention(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 && args.len() != 3 {
        return Err(format!(
            "attention: expected 2 args (heads, dim) or 3 args (heads, dim, kv_heads), got {}",
            args.len()
        ));
    }
    let heads = match &args[0] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("attention: heads must be a positive integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "attention: heads must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let dim = match &args[1] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("attention: dim must be a positive integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "attention: dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    // Наряд №188: optional 3rd arg — n_kv_heads for GQA.
    let n_kv_heads = if args.len() == 3 {
        match &args[2] {
            Value::Float(n) => *n as usize,
            Value::String(s) => s.parse::<usize>().map_err(|_| {
                format!(
                    "attention: kv_heads must be a positive integer, got '{}'",
                    s
                )
            })?,
            other => {
                return Err(format!(
                    "attention: kv_heads must be a number, got {}",
                    other.type_name()
                ))
            }
        }
    } else {
        heads // default: standard MHA
    };

    let attn = crate::nn::attention::Attention::new_with_kv_heads(heads, n_kv_heads, dim, seed)?;
    Ok(Box::new(attn))
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// Args: `(dim, [eps])`. `eps` is optional (default 1e-6).
pub fn build_rmsnorm(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "rms_norm: expected 1 or 2 args (dim, [eps]), got {}",
            args.len()
        ));
    }
    let dim = parse_usize_arg(&args[0], "rms_norm", "dim")?;
    let eps = if args.len() == 2 {
        parse_f64_arg(&args[1], "rms_norm", "eps")?
    } else {
        1e-6
    };
    let layer = crate::nn::rmsnorm::RmsNorm::new(dim, seed, eps)?;
    Ok(Box::new(layer))
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// Args: `(dim, ff_dim)`. Both are required — no defaults, to keep
/// the declaration explicit (consistent with Attention's `(heads, dim)`
/// API from Наряд №183).
pub fn build_swiglu(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 {
        return Err(format!(
            "swiglu: expected 2 args (dim, ff_dim), got {}",
            args.len()
        ));
    }
    let dim = parse_usize_arg(&args[0], "swiglu", "dim")?;
    let ff_dim = parse_usize_arg(&args[1], "swiglu", "ff_dim")?;
    let layer = crate::nn::swiglu::SwiGlu::new(dim, ff_dim, seed)?;
    Ok(Box::new(layer))
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// Args: `(heads, dim, ff_dim)`. All three are required — no defaults.
///
/// Matches the naryad spec's example:
/// ```text
/// reflex_seq TinyTransformer {
///   input: embedding(64)
///   seq_len: 16
///   layers: [transformer_block(4, 64, 256)]
///   seed: 42
/// }
/// ```
pub fn build_transformer_block(
    args: &[Value],
    seed: u64,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 3 {
        return Err(format!(
            "transformer_block: expected 3 args (heads, dim, ff_dim), got {}",
            args.len()
        ));
    }
    let heads = parse_usize_arg(&args[0], "transformer_block", "heads")?;
    let dim = parse_usize_arg(&args[1], "transformer_block", "dim")?;
    let ff_dim = parse_usize_arg(&args[2], "transformer_block", "ff_dim")?;
    let layer = crate::nn::transformer_block::TransformerBlock::new(heads, dim, ff_dim, seed)?;
    Ok(Box::new(layer))
}

// ── Trainable builders (the №185/№190 autograd path — used by
//    `reflex_seq` construction, builtins/reflex.rs) ────────────────────

/// Build function — accepts same args as `build_attention` (Наряд №188:
/// heads, dim, [kv_heads]). Takes the `VarMap` (not VarBuilder) so it can
/// register the Vars manually with deterministic init values.
///
/// Наряд №190: `prefix` parameter makes each layer in a stack register
/// its weights under unique VarMap names (avoids collision).
pub fn build_trainable_attention(
    args: &[Value],
    seed: u64,
    var_map: &candle_nn::VarMap,
    prefix: &str,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 && args.len() != 3 {
        return Err(format!(
            "trainable_attention: expected 2 args (heads, dim) or 3 args (heads, dim, kv_heads), got {}",
            args.len()
        ));
    }
    let heads = match &args[0] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_attention: heads must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_attention: heads must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let dim = match &args[1] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_attention: dim must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_attention: dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    // Наряд №188: optional 3rd arg — n_kv_heads for GQA.
    let n_kv_heads = if args.len() == 3 {
        match &args[2] {
            Value::Float(n) => *n as usize,
            Value::String(s) => s.parse::<usize>().map_err(|_| {
                format!("trainable_attention: kv_heads must be integer, got '{}'", s)
            })?,
            other => {
                return Err(format!(
                    "trainable_attention: kv_heads must be a number, got {}",
                    other.type_name()
                ))
            }
        }
    } else {
        heads
    };
    let attn = crate::nn::trainable_attention::TrainableAttention::new_with_kv_heads(
        heads, n_kv_heads, dim, seed, var_map, prefix,
    )?;
    Ok(Box::new(attn))
}

/// Build function — accepts same args as `build_transformer_block`
/// (heads, dim, ff_dim) from Наряд №184.
///
/// Наряд №190: `prefix` parameter makes each block in a stack register
/// its weights under unique VarMap names.
pub fn build_trainable_transformer_block(
    args: &[Value],
    seed: u64,
    var_map: &candle_nn::VarMap,
    prefix: &str,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 3 {
        return Err(format!(
            "trainable_transformer_block: expected 3 args (heads, dim, ff_dim), got {}",
            args.len()
        ));
    }
    let heads = match &args[0] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_tb: heads must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_tb: heads must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let dim = match &args[1] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_tb: dim must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_tb: dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let ff_dim = match &args[2] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_tb: ff_dim must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_tb: ff_dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let block = crate::nn::trainable_transformer_block::TrainableTransformerBlock::new(
        heads, dim, ff_dim, seed, var_map, prefix,
    )?;
    Ok(Box::new(block))
}

// ── helpers (the rmsnorm.rs local copies, unified — the build surface
//    is ONE file now, so the "kept local to avoid cross-module
//    coupling" copies collapse into a single definition) ───────────────

fn parse_usize_arg(v: &Value, layer: &str, name: &str) -> Result<usize, String> {
    match v {
        Value::Float(n) => Ok(*n as usize),
        Value::String(s) => s.parse::<usize>().map_err(|_| {
            format!(
                "{}: {} must be a positive integer, got '{}'",
                layer, name, s
            )
        }),
        other => Err(format!(
            "{}: {} must be a number, got {}",
            layer,
            name,
            other.type_name()
        )),
    }
}

fn parse_f64_arg(v: &Value, layer: &str, name: &str) -> Result<f64, String> {
    match v {
        Value::Float(n) => Ok(*n),
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| format!("{}: {} must be a number, got '{}'", layer, name, s)),
        other => Err(format!(
            "{}: {} must be a number, got {}",
            layer,
            name,
            other.type_name()
        )),
    }
}
