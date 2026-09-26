//! №467 (gh#688) — stage 0 of the type system: the signature `Type` enum
//! for the builtin registry (gate gh#680, decision 3-A).
//!
//! The audit 25.09 §4.4 finding: the language types are strings
//! (`ast.rs` `return_type: String` / `type_name: String`), there is no
//! inference, and the 500+ builtin signatures are untrustworthy — the
//! documentation says "String" while the handler returns anything. Stage
//! 0 (this module) does NOT touch the runtime: it adds the typed
//! signature vocabulary to `BUILTIN_REGISTRY` and converts the string
//! path into the enum at the registry fill site (the `spec!` macro arms
//! with a return-type argument call `Type::from_path` at expansion time).
//!
//! The taxonomy is the one fixed by the naryad — minimal, deliberately:
//! - the nine structural kinds (`Int`..`Fluid`) mirror `Value`'s shapes
//!   (the machine has no `Int` value yet — every number is `Float`;
//!   `Int` exists because the documented vocabulary uses it and stage 1
//!   will need it);
//! - `Opaque(OpaqueKind)` covers the opaque-handle surfaces with the
//!   audit's own grouping (Secret / Reflex / Grant / Series / Media /
//!   Embodied, `Other` for the unmodeled remainder);
//! - `Labeled(Box<Type>, Label)` carries the №322 confidentiality-label
//!   annotation shape (`String<private>`); `Box` is not const-constructible
//!   on stable, so the CONST fill-site parser (`from_path`) serves the flat
//!   vocabulary and maps a labeled string it cannot lift to `Unknown`
//!   honestly — the full parser (`parse_type`, non-const) handles the
//!   labeled/nested path and is pinned to agree with `from_path` on the
//!   flat vocabulary (the tests below);
//! - `Unknown` is the honest answer for anything the vocabulary does not
//!   cover: NEVER a fictitious "precise" type.
//!
//! The CI metric (the owner's strengthening: the typed-signature share
//! grows every release) is computed from the registry: a spec row is
//! typed when its `return_type` is not `Unknown`. The checked-in floor
//! lives in `scripts/ci/type_signature_baseline.txt` and the gate script
//! `scripts/ci/type_signature_share.py` runs in CI every push (the
//! `type-signatures` job); the in-tree test `tests/type_signature_metric.rs`
//! double-locks the floor from inside the binary.

/// The confidentiality-label vocabulary of stage 0 (the audit §4.4:
/// "метки Internal/Private" — the two labels the typed-signature lane
/// needs; the full №322 label algebra stays in `src/labels.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The value may move inside the trust boundary only.
    Internal,
    /// The value is user data / a secret — egress is gated (№325 lane).
    Private,
}

impl Label {
    /// The label spelling used in the type paths (`X<private>`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Label::Internal => "internal",
            Label::Private => "private",
        }
    }
}

/// The minimal opaque-handle grouping (the audit §4.4 wording: Secret /
/// Reflex / Grant / SeriesHandle / media / the seven embodied — `Other`
/// for the unmodeled remainder, e.g. the Html/Query/Session opaques).
/// Deliberately NOT finer: the naryad forbids taxonomy bloat at stage 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpaqueKind {
    /// Secrets, password hashes, encrypted blobs (the `Value::Secret`
    /// family — non-printable, zeroized on drop).
    Secret,
    /// Reflex model and BPE vocabulary handles (№178/№195).
    Reflex,
    /// Grant capability handles (№390, ADR-0155).
    Grant,
    /// Forecast series handles (№440, ADR-0175).
    Series,
    /// The unified media family + vision/voice/audio/video/LLM-stream
    /// handles (№331 ADR-0162, №302/№307/№240, №275).
    Media,
    /// The seven embodied-pillar handles (kitchen/robotics lane).
    Embodied,
    /// Any other opaque surface (Html, Query, Session, Subgraph,
    /// Likeness...): opaque is the fact, the family is unmodeled.
    Other,
}

/// The stage-0 signature type (the naryad's fixed taxonomy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Unit,
    List,
    Map,
    Struct,
    /// Superposition of typed variants with confidence scores
    /// (`Value::Fluid` — collapses lazily at point of use).
    Fluid,
    /// An opaque handle of the given family.
    Opaque(OpaqueKind),
    /// A type with a confidentiality-label annotation (`X<private>`).
    Labeled(Box<Type>, Label),
    /// The honest "the string path is outside the stage-0 vocabulary"
    /// answer — never a fictitious precise type.
    Unknown,
}

/// The flat fill-site vocabulary. The const table maps the documented
/// string spellings to small integer TAGS (a `Type` value itself cannot
/// be moved out of a const slice — the enum carries `Box` in `Labeled`);
/// `Type::from_tag` then constructs the value with a plain integer match
/// (stable in constant functions).
const TAG_INT: u8 = 0;
const TAG_FLOAT: u8 = 1;
const TAG_BOOL: u8 = 2;
const TAG_STRING: u8 = 3;
const TAG_UNIT: u8 = 4;
const TAG_LIST: u8 = 5;
const TAG_MAP: u8 = 6;
const TAG_STRUCT: u8 = 7;
const TAG_FLUID: u8 = 8;
const TAG_OPAQUE_SECRET: u8 = 9;
const TAG_OPAQUE_REFLEX: u8 = 10;
const TAG_OPAQUE_GRANT: u8 = 11;
const TAG_OPAQUE_SERIES: u8 = 12;
const TAG_OPAQUE_MEDIA: u8 = 13;
const TAG_OPAQUE_EMBODIED: u8 = 14;

const FLAT_TABLE: &[(&str, u8)] = &[
    ("Int", TAG_INT),
    ("Float", TAG_FLOAT),
    ("Bool", TAG_BOOL),
    ("String", TAG_STRING),
    ("Unit", TAG_UNIT),
    ("List", TAG_LIST),
    ("Map", TAG_MAP),
    ("Dict", TAG_MAP),
    ("Struct", TAG_STRUCT),
    ("Fluid", TAG_FLUID),
    ("Secret", TAG_OPAQUE_SECRET),
    ("Reflex", TAG_OPAQUE_REFLEX),
    ("Grant", TAG_OPAQUE_GRANT),
    ("Series", TAG_OPAQUE_SERIES),
    ("SeriesHandle", TAG_OPAQUE_SERIES),
    ("Media", TAG_OPAQUE_MEDIA),
    ("Embodied", TAG_OPAQUE_EMBODIED),
];

impl Type {
    /// The CONST fill-site parser: the flat stage-0 vocabulary.
    ///
    /// Runs inside `BUILTIN_REGISTRY` (a const item), so the parser is a
    /// `const fn` over byte matching — no allocation, no `Box`, no `str`
    /// pattern matching (not yet stable in constant functions). The
    /// vocabulary:
    /// - the nine structural spellings: `Int`, `Float`, `Bool`, `String`,
    ///   `Unit`, `List`, `Map`, `Struct`, `Fluid` (the documented alias
    ///   `Dict` maps to `Map`; numbers are `Float` in `Value` today —
    ///   `Int` is the documented vocabulary's spelling, stage 1 reconciles);
    /// - the opaque spellings: `Secret`, `Reflex`, `Grant`, `Series`,
    ///   `SeriesHandle`, `Media`, `Embodied`;
    /// - `List<...>` (any inner) erases to `List` — the stage-0 taxonomy
    ///   is parametric-free;
    /// - anything else — including labeled strings, which the const
    ///   parser cannot lift to `Labeled(Box)` on stable — is honestly
    ///   `Unknown`.
    pub const fn from_path(s: &str) -> Type {
        let b = s.as_bytes();
        let mut i = 0;
        while i < FLAT_TABLE.len() {
            if bytes_eq(b, FLAT_TABLE[i].0.as_bytes()) {
                return Type::from_tag(FLAT_TABLE[i].1);
            }
            i += 1;
        }
        if bytes_starts_with(b, b"List<") && bytes_ends_with(b, b">") {
            return Type::List;
        }
        Type::Unknown
    }

    const fn from_tag(tag: u8) -> Type {
        match tag {
            TAG_INT => Type::Int,
            TAG_FLOAT => Type::Float,
            TAG_BOOL => Type::Bool,
            TAG_STRING => Type::String,
            TAG_UNIT => Type::Unit,
            TAG_LIST => Type::List,
            TAG_MAP => Type::Map,
            TAG_STRUCT => Type::Struct,
            TAG_FLUID => Type::Fluid,
            TAG_OPAQUE_SECRET => Type::Opaque(OpaqueKind::Secret),
            TAG_OPAQUE_REFLEX => Type::Opaque(OpaqueKind::Reflex),
            TAG_OPAQUE_GRANT => Type::Opaque(OpaqueKind::Grant),
            TAG_OPAQUE_SERIES => Type::Opaque(OpaqueKind::Series),
            TAG_OPAQUE_MEDIA => Type::Opaque(OpaqueKind::Media),
            TAG_OPAQUE_EMBODIED => Type::Opaque(OpaqueKind::Embodied),
            _ => Type::Unknown,
        }
    }
}

const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn bytes_starts_with(b: &[u8], prefix: &[u8]) -> bool {
    if b.len() < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if b[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn bytes_ends_with(b: &[u8], suffix: &[u8]) -> bool {
    if b.len() < suffix.len() {
        return false;
    }
    let mut i = 0;
    while i < suffix.len() {
        if b[b.len() - suffix.len() + i] != suffix[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The FULL string-path parser (non-const): the flat vocabulary of
/// `from_path` PLUS the labeled/nested path the const parser cannot lift
/// (`X<private>` → `Labeled`, `List<Float>` → `List`). This is the parser
/// stage 1 builds on; the tests pin that it agrees with `from_path`
/// wherever `from_path` is defined.
pub fn parse_type(s: &str) -> Type {
    // The label suffix: `X<private>` / `X<internal>` (the LAST `<...>`
    // group when it spells a stage-0 label).
    if let Some(inner) = s.strip_suffix('>') {
        if let Some(lt) = inner.rfind('<') {
            let label_body = &inner[lt + 1..];
            let base = &inner[..lt];
            let label = match label_body {
                "private" => Some(Label::Private),
                "internal" => Some(Label::Internal),
                _ => None,
            };
            if let Some(label) = label {
                return Type::Labeled(Box::new(parse_type(base)), label);
            }
        }
    }
    Type::from_path(s)
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => f.write_str("Int"),
            Type::Float => f.write_str("Float"),
            Type::Bool => f.write_str("Bool"),
            Type::String => f.write_str("String"),
            Type::Unit => f.write_str("Unit"),
            Type::List => f.write_str("List"),
            Type::Map => f.write_str("Map"),
            Type::Struct => f.write_str("Struct"),
            Type::Fluid => f.write_str("Fluid"),
            Type::Opaque(k) => write!(f, "Opaque({:?})", k),
            Type::Labeled(t, l) => write!(f, "{}<{}>", t, l),
            Type::Unknown => f.write_str("Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── the flat vocabulary ─────────────────────────────────────────────

    #[test]
    fn from_path_maps_the_nine_structural_kinds() {
        assert_eq!(Type::from_path("Int"), Type::Int);
        assert_eq!(Type::from_path("Float"), Type::Float);
        assert_eq!(Type::from_path("Bool"), Type::Bool);
        assert_eq!(Type::from_path("String"), Type::String);
        assert_eq!(Type::from_path("Unit"), Type::Unit);
        assert_eq!(Type::from_path("List"), Type::List);
        assert_eq!(Type::from_path("Map"), Type::Map);
        assert_eq!(Type::from_path("Dict"), Type::Map);
        assert_eq!(Type::from_path("Struct"), Type::Struct);
        assert_eq!(Type::from_path("Fluid"), Type::Fluid);
    }

    #[test]
    fn from_path_maps_the_opaque_spellings() {
        assert_eq!(Type::from_path("Secret"), Type::Opaque(OpaqueKind::Secret));
        assert_eq!(Type::from_path("Reflex"), Type::Opaque(OpaqueKind::Reflex));
        assert_eq!(Type::from_path("Grant"), Type::Opaque(OpaqueKind::Grant));
        assert_eq!(Type::from_path("Series"), Type::Opaque(OpaqueKind::Series));
        assert_eq!(
            Type::from_path("SeriesHandle"),
            Type::Opaque(OpaqueKind::Series)
        );
        assert_eq!(Type::from_path("Media"), Type::Opaque(OpaqueKind::Media));
        assert_eq!(
            Type::from_path("Embodied"),
            Type::Opaque(OpaqueKind::Embodied)
        );
    }

    #[test]
    fn from_path_erases_list_inner_and_is_honest_about_the_rest() {
        assert_eq!(Type::from_path("List<Float>"), Type::List);
        assert_eq!(Type::from_path("List<Struct>"), Type::List);
        // Honest Unknown: never a fictitious precise type.
        assert_eq!(Type::from_path("string"), Type::Unknown);
        assert_eq!(Type::from_path("HashMap"), Type::Unknown);
        assert_eq!(Type::from_path("List"), Type::List);
        assert_eq!(Type::from_path(""), Type::Unknown);
    }

    // ── the full parser (labels) ────────────────────────────────────────

    #[test]
    fn parse_type_lifts_the_label_suffix() {
        assert_eq!(
            parse_type("String<private>"),
            Type::Labeled(Box::new(Type::String), Label::Private)
        );
        assert_eq!(
            parse_type("Float<internal>"),
            Type::Labeled(Box::new(Type::Float), Label::Internal)
        );
    }

    #[test]
    fn parse_type_agrees_with_from_path_on_the_flat_vocabulary() {
        for s in [
            "Int",
            "Float",
            "Bool",
            "String",
            "Unit",
            "List",
            "Map",
            "Dict",
            "Struct",
            "Fluid",
            "Secret",
            "Reflex",
            "Grant",
            "Series",
            "SeriesHandle",
            "Media",
            "Embodied",
            "List<Float>",
            "nonsense",
            "",
        ] {
            assert_eq!(parse_type(s), Type::from_path(s), "divergence on {:?}", s);
        }
    }

    #[test]
    fn display_is_stable() {
        assert_eq!(Type::Float.to_string(), "Float");
        assert_eq!(Type::Unknown.to_string(), "Unknown");
        assert_eq!(parse_type("String<private>").to_string(), "String<private>");
    }

    #[test]
    fn label_spellings_are_the_path_vocabulary() {
        assert_eq!(Label::Private.as_str(), "private");
        assert_eq!(Label::Internal.as_str(), "internal");
    }
}
