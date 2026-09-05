// Type alias for the layer build function signature.
pub type LayerBuildFn =
    fn(args: &[crate::interpreter::Value], seed: u64) -> Result<Box<dyn Layer>, String>;

// Layer trait + LAYER_REGISTRY — registry-extensible (ADR-0114 addendum).
//
// New layer types are added by:
// 1. Implementing `Layer` trait in a new module
// 2. Adding a `LayerSpec` entry to `LAYER_REGISTRY`
// No grammar change needed — `layer_spec = { IDENT ~ "(" ~ layer_arg_list? ")" }`
// resolves the layer name at `mlog check` time via this registry.

/// A neural network layer — forward-only (training is Наряд №179).
pub trait Layer: Send + Sync {
    /// Forward pass: input → output.
    /// Input is `&[f64]` (not `Value`) — layers operate on raw floats,
    /// the Value↔float boundary is at the builtin level (reflex_predict).
    fn forward(&self, input: &[f64]) -> Vec<f64>;

    /// Input dimension (number of features the layer expects).
    fn input_size(&self) -> usize;

    /// Output dimension (number of features the layer produces).
    fn output_size(&self) -> usize;

    /// Layer type name (e.g. "dense") for debugging.
    fn name(&self) -> &str;

    /// Serialize weights to bytes (used in Наряд №180 for persistence).
    fn serialize_weights(&self) -> Vec<u8>;

    /// Deserialize weights from bytes (used in Наряд №180).
    fn deserialize_weights(&mut self, data: &[u8]) -> Result<(), String>;
}

/// Specification for a layer type — analogous to `BuiltinSpec`.
/// The registry is the single source of truth for available layer types.
pub struct LayerSpec {
    /// Layer type name (e.g. "dense", "conv1d" in future).
    pub name: &'static str,
    /// Parameter names in order (e.g. &["units", "activation"]).
    /// Used for error messages and documentation.
    pub param_names: &'static [&'static str],
    /// Build function: takes Value args, returns a boxed Layer.
    pub build: LayerBuildFn,
}

/// The layer registry — extensible without grammar changes (ADR-0114).
/// Order is NOT significant (unlike BUILTIN_REGISTRY which determines
/// bytecode indices). Layers are looked up by name.
pub static LAYER_REGISTRY: &[LayerSpec] = &[LayerSpec {
    name: "dense",
    param_names: &["units", "activation"],
    build: crate::nn::dense::build_dense,
}];

/// Look up a layer spec by name. Returns None if not found.
pub fn find_layer_spec(name: &str) -> Option<&'static LayerSpec> {
    LAYER_REGISTRY.iter().find(|s| s.name == name)
}

/// List all registered layer names (for error messages).
pub fn layer_names() -> Vec<&'static str> {
    LAYER_REGISTRY.iter().map(|s| s.name).collect()
}
