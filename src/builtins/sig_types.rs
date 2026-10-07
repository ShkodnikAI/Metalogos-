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
//!
//! №627 (gh#1110) — the FIELD-METADATA form (stage-2 preparation, ADR-0178):
//! a `Struct<Name>` row may carry the per-field label table IN THE SPEC
//! STRING — `Struct<Name>{field:label,field2:label2}` — modeled on the
//! `X<private>` label-suffix vocabulary (№577). The grammar is the
//! machine-checkable minimalism: NO spaces, the field name is
//! `[A-Za-z0-9_]+` (non-empty), the label is one of the stage-0
//! `Label::as_str` tokens (`internal` / `private` / `untrusted`), the
//! entries are comma-separated, at least one entry. The metadata lives in
//! the REGISTRY side-table (`BuiltinSpec.field_meta`, the raw section
//! inner) — the `Type` enum is NOT extended (stage-2 minimality: the
//! enum stays erased, `Struct<...>{...}` erases to `Struct` exactly like
//! №623's `Struct<...>`). A MALFORMED section is honestly `Unknown`
//! (fail-closed — a typo must not launder into a typed row), and the
//! fourth metric (the field-label share among the parameterized Struct
//! rows) gates on the checked-in
//! `scripts/ci/type_signature_fieldmeta_baseline.txt` with the in-tree
//! twin in `tests/type_signature_metric.rs`.

/// The confidentiality-label vocabulary of stage 0 (the audit §4.4:
/// "метки Internal/Private" — the two labels the typed-signature lane
/// needs; the full №322 label algebra stays in `src/labels.rs`).
/// №544 step 3 (gh#882): the stage-2 HTML lane adds the third label —
/// `Untrusted` marks model-generated text (ADR-0117: the LLM output is
/// untrusted content; egress through respond() is gated by the
/// HTML_INJECTION check, sanitizers strip it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The value may move inside the trust boundary only.
    Internal,
    /// The value is user data / a secret — egress is gated (№325 lane).
    Private,
    /// №544 step 3: the value is model/user-generated content — untrusted
    /// text; sanitize (render/escape_html) before the HTML egress.
    Untrusted,
}

impl Label {
    /// The label spelling used in the type paths (`X<private>`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Label::Internal => "internal",
            Label::Private => "private",
            Label::Untrusted => "untrusted",
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
    /// - `Struct<...>` (any inner) erases to `Struct` — the SAME stage-0
    ///   erasure, symmetric to the List one (№623: the parameter lives in
    ///   the spec! string and in the third metric — the parameterized
    ///   share among the List/Struct rows — not in the enum; the enum
    ///   stays minimal, stage-0);
    /// - `Struct<Name>{field:label,...}` (the №627 field-metadata form)
    ///   erases to `Struct` TOO — the parameter AND the per-field label
    ///   table live in the spec! string, the metadata lands in the
    ///   registry side-table (`BuiltinSpec.field_meta`), the enum stays
    ///   erased. A MALFORMED section (bad grammar, unknown label token,
    ///   empty table, a brace tail on a non-`Struct<` head) is honestly
    ///   `Unknown` — fail-closed, a typo must not launder into a typed
    ///   row;
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
        if bytes_starts_with(b, b"Struct<") && bytes_ends_with(b, b">") {
            return Type::Struct;
        }
        // №627: the field-metadata form `Struct<Name>{field:label,...}`.
        // The head must be the parameterized Struct spelling and the
        // section must parse under the №627 grammar; everything else
        // with a brace tail is honest Unknown (including `List<...>{...}`
        // — the field table belongs to structs; a List row with a brace
        // section is a typo, not a type).
        if bytes_ends_with(b, b"}") {
            let (head, meta) = split_field_meta(s);
            let hb = head.as_bytes();
            if bytes_starts_with(hb, b"Struct<")
                && bytes_ends_with(hb, b">")
                && !meta.is_empty()
                && field_meta_well_formed(meta)
            {
                return Type::Struct;
            }
            return Type::Unknown;
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

/// №623: is this spec! type string the PARAMETERIZED spelling
/// (`List<...>` / `Struct<...>` whose inner is NOT the stage-0 label
/// suffix — `X<private>`/`<internal>`/`<untrusted>` are the №577 label
/// vocabulary, not a parameter)? Const-evaluable, same style as
/// `from_path`; the enum stays erased (stage-0 minimality) — this bool
/// is the in-tree fact the third metric's lock reads.
///
/// №627: the field-metadata form `Struct<Name>{field:label,...}` is
/// parameterized TOO — the head (before the brace section) is the
/// parameterized Struct spelling. The ORIGINAL logic runs on the head
/// verbatim (the §№623 label-suffix exclusion included); a malformed
/// brace tail (no `>` before the `{`) falls through to `false`
/// (fail-closed).
pub const fn path_is_parameterized(s: &str) -> bool {
    let (head, _meta) = split_field_meta(s);
    let b = head.as_bytes();
    let starts_list = bytes_starts_with(b, b"List<") && bytes_ends_with(b, b">");
    let starts_struct = bytes_starts_with(b, b"Struct<") && bytes_ends_with(b, b">");
    if !(starts_list || starts_struct) {
        return false;
    }
    !(bytes_ends_with(b, b"<private>")
        || bytes_ends_with(b, b"<internal>")
        || bytes_ends_with(b, b"<untrusted>"))
}

/// №627: split a spec! type string into (head, field-meta inner).
/// The section exists when the string ends with `}` and there is a `{`
/// whose preceding byte is `>` (the head closes the `Struct<Name>`
/// parameter before the table opens). Returns `(s, "")` when there is
/// no well-formed section shape — the meta inner being empty is the
/// ABSENT marker everywhere (the fill site, the metric, the tests).
/// Const-evaluable; the python twin in
/// `scripts/ci/type_signature_share.py::split_field_meta` mirrors this
/// byte-for-byte (the two-locks discipline).
pub const fn split_field_meta(s: &str) -> (&str, &str) {
    let b = s.as_bytes();
    if b.len() < 2 || b[b.len() - 1] != b'}' {
        return (s, "");
    }
    let mut i = b.len() - 1;
    let mut open = None;
    while i > 0 {
        if b[i] == b'{' {
            open = Some(i);
            break;
        }
        i -= 1;
    }
    match open {
        Some(o) if o >= 1 && b[o - 1] == b'>' => {
            // range-slicing is not const-stable on the toolchain floor —
            // split_at is (and `o` is the ASCII `{` byte, a char boundary)
            let head = s.split_at(o).0;
            let rest = s.split_at(o).1; // `{...}`
            let meta = rest.split_at(1).1; // drop the `{`
            let meta = meta.split_at(meta.len() - 1).0; // drop the `}`
            (head, meta)
        }
        _ => (s, ""),
    }
}

/// №627: is this field-meta section inner WELL-FORMED under the №627
/// grammar? `entry (',' entry)*` where entry = `name:label`, the name is
/// `[A-Za-z0-9_]+` (non-empty, ASCII — the byte-exact twin of the python
/// checker; unicode field names are NOT in the vocabulary), the label is
/// one of the stage-0 `Label::as_str` tokens, NO spaces anywhere
/// (the machine-checkable minimalism — the regex and the const parser
/// stay exact), at least one entry, no empty entries (a trailing comma
/// is malformed). Const-evaluable; consumed by `from_path` (fail-closed
/// erasure), by the fill-site extractor and by the in-tree fourth-metric
/// lock.
pub const fn field_meta_well_formed(meta: &str) -> bool {
    if meta.is_empty() {
        return false;
    }
    let b = meta.as_bytes();
    // walk the entries: name chars, ':', the label token, ','
    let mut i = 0;
    loop {
        // entry name: [A-Za-z0-9_]+
        let name_start = i;
        while i < b.len() && (bytes_is_alnum_ascii(b[i]) || b[i] == b'_') {
            i += 1;
        }
        if i == name_start || i >= b.len() || b[i] != b':' {
            return false;
        }
        i += 1; // the ':'
                // the label token: one of the stage-0 vocabulary spellings
                // (split_at, not range-slicing — const-stability; `i` is on an
                // ASCII boundary by construction of the walk above)
        let rest = meta.split_at(i).1;
        let token_len = if bytes_starts_with(rest.as_bytes(), b"internal") {
            8
        } else if bytes_starts_with(rest.as_bytes(), b"private") {
            7
        } else if bytes_starts_with(rest.as_bytes(), b"untrusted") {
            9
        } else {
            return false;
        };
        i += token_len;
        if i == b.len() {
            return true; // the last entry, no trailing comma
        }
        if b[i] != b',' {
            return false; // a space or any other byte between entries is malformed
        }
        i += 1;
        if i == b.len() {
            return false; // a trailing comma is malformed
        }
    }
}

const fn bytes_is_alnum_ascii(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || (c >= b'0' && c <= b'9')
}

/// №627: the fill-site extractor — the field-meta section inner of a
/// spec! type string, or `""` when the string does not carry the
/// `Struct<Name>{...}` shape. The head must be the `Struct<` spelling
/// (a brace tail on a `List<...>` head is NOT metadata — the section
/// belongs to structs); well-formedness is NOT checked here on purpose:
/// a malformed section that still extracts fails `from_path` (the row
/// erases to Unknown and the general typed floor catches it) AND the
/// in-tree `every_field_meta_is_the_well_formed_grammar` armor — the
/// two locks, not zero.
pub const fn field_meta_of(s: &str) -> &str {
    let (head, meta) = split_field_meta(s);
    let hb = head.as_bytes();
    if bytes_starts_with(hb, b"Struct<") && bytes_ends_with(hb, b">") {
        meta
    } else {
        ""
    }
}

/// №627: the FULL (non-const) field-metadata parser — the consumer API
/// over `BuiltinSpec.field_meta`. Returns `None` on a malformed section
/// (fail-closed) and the (field, label) vector otherwise; pinned to
/// agree with the const checker `field_meta_well_formed` (the tests).
/// The duplicates are NOT rejected by the grammar (stage-2 minimalism) —
/// a consumer building a map from this vector takes the LAST entry.
pub fn parse_field_meta(meta: &str) -> Option<Vec<(&str, Label)>> {
    if !field_meta_well_formed(meta) {
        return None;
    }
    let mut out = Vec::new();
    for entry in meta.split(',') {
        let colon = entry.find(':')?;
        let name = &entry[..colon];
        let label = match &entry[colon + 1..] {
            "internal" => Label::Internal,
            "private" => Label::Private,
            "untrusted" => Label::Untrusted,
            _ => return None,
        };
        out.push((name, label));
    }
    Some(out)
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
                "untrusted" => Some(Label::Untrusted),
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

    // ── №627: the field-metadata form ───────────────────────────────────

    #[test]
    fn field_meta_form_erases_to_struct() {
        // The №627 form: Struct<Name>{field:label,...} — the enum stays
        // erased (stage-2 minimality), the metadata lives in the registry
        // side-table (field_meta_of / BuiltinSpec.field_meta).
        assert_eq!(
            Type::from_path("Struct<Weather>{city:untrusted}"),
            Type::Struct
        );
        assert_eq!(
            Type::from_path("Struct<Weather>{temp:untrusted,description:internal}"),
            Type::Struct
        );
        assert_eq!(
            Type::from_path("Struct<LlmUsage>{total_calls:internal,providers:internal}"),
            Type::Struct
        );
    }

    #[test]
    fn field_meta_form_is_parameterized() {
        // The №623 third metric must SEE the meta rows (the head is the
        // parameterized Struct spelling).
        assert!(path_is_parameterized("Struct<Weather>{city:untrusted}"));
        assert!(path_is_parameterized(
            "Struct<Weather>{temp:untrusted,description:internal}"
        ));
        // The bare №623 forms keep their verdicts.
        assert!(path_is_parameterized("Struct<Weather>"));
        assert!(path_is_parameterized("List<Tool>"));
        assert!(!path_is_parameterized("Struct"));
        assert!(!path_is_parameterized("String<private>"));
    }

    #[test]
    fn field_meta_malformed_is_honest_unknown() {
        // Fail-closed: a malformed section never launders into a typed row.
        // unknown label token
        assert_eq!(
            Type::from_path("Struct<Weather>{city:secret}"),
            Type::Unknown
        );
        // a space between the entries (the grammar has none)
        assert_eq!(
            Type::from_path("Struct<Weather>{city:untrusted, temp:untrusted}"),
            Type::Unknown
        );
        // empty table
        assert_eq!(Type::from_path("Struct<Weather>{}"), Type::Unknown);
        // trailing comma
        assert_eq!(
            Type::from_path("Struct<Weather>{city:untrusted,}"),
            Type::Unknown
        );
        // missing label
        assert_eq!(Type::from_path("Struct<Weather>{city:}"), Type::Unknown);
        // missing colon
        assert_eq!(
            Type::from_path("Struct<Weather>{city_untrusted}"),
            Type::Unknown
        );
        // empty field name
        assert_eq!(
            Type::from_path("Struct<Weather>{:untrusted}"),
            Type::Unknown
        );
        // a brace tail on a List head — the section belongs to structs
        assert_eq!(Type::from_path("List<Tool>{name:untrusted}"), Type::Unknown);
        // a stray brace without the `>` anchor
        assert_eq!(
            Type::from_path("Struct<Weather>{city:untrusted}{x:internal}"),
            Type::Unknown
        );
    }

    #[test]
    fn split_field_meta_shapes() {
        assert_eq!(
            split_field_meta("Struct<Weather>{city:untrusted}"),
            ("Struct<Weather>", "city:untrusted")
        );
        assert_eq!(split_field_meta("Struct<Weather>"), ("Struct<Weather>", ""));
        assert_eq!(split_field_meta("String"), ("String", ""));
        // no `>` anchor before the `{` — no section
        assert_eq!(
            split_field_meta("Struct{a:internal}"),
            ("Struct{a:internal}", "")
        );
        // the anchor requires the byte right before `{` to be `>`
        assert_eq!(
            split_field_meta("Struct<Weather>x{a:internal}"),
            ("Struct<Weather>x{a:internal}", "")
        );
    }

    #[test]
    fn field_meta_grammar_pins() {
        assert!(field_meta_well_formed("city:untrusted"));
        assert!(field_meta_well_formed(
            "temp:untrusted,description:internal"
        ));
        assert!(field_meta_well_formed("a1_b2:private"));
        assert!(!field_meta_well_formed(""));
        assert!(!field_meta_well_formed("city:secret"));
        assert!(!field_meta_well_formed("city:untrusted,"));
        assert!(!field_meta_well_formed("city: untrusted")); // no spaces
        assert!(!field_meta_well_formed(":untrusted"));
        assert!(!field_meta_well_formed("city"));
        // the ASCII-only name class (the byte-exact python twin)
        assert!(!field_meta_well_formed("город:internal"));
    }

    #[test]
    fn field_meta_of_extracts_the_side_table_fact() {
        assert_eq!(
            field_meta_of("Struct<Weather>{city:untrusted,temp:untrusted}"),
            "city:untrusted,temp:untrusted"
        );
        // non-Struct heads carry no metadata
        assert_eq!(field_meta_of("List<Tool>{name:untrusted}"), "");
        assert_eq!(field_meta_of("Struct<Weather>"), "");
        assert_eq!(field_meta_of("String"), "");
        // extraction is shape-only: a malformed inner still extracts —
        // the armor is from_path (Unknown) + the in-tree grammar test
        assert_eq!(field_meta_of("Struct<X>{bad}"), "bad");
    }

    #[test]
    fn parse_field_meta_agrees_with_the_const_checker() {
        let meta = "temp:untrusted,description:internal,city:untrusted";
        let parsed = parse_field_meta(meta).expect("well-formed");
        assert_eq!(
            parsed,
            vec![
                ("temp", Label::Untrusted),
                ("description", Label::Internal),
                ("city", Label::Untrusted),
            ]
        );
        assert!(parse_field_meta("bad").is_none());
        assert!(parse_field_meta("a:secret,b:internal").is_none());
        assert!(parse_field_meta("").is_none());
    }

    #[test]
    fn parse_type_agrees_with_from_path_on_the_meta_form() {
        for s in [
            "Struct<Weather>{city:untrusted}",
            "Struct<Weather>{temp:untrusted,description:internal}",
            "Struct<Weather>{}",
            "List<Tool>{name:untrusted}",
        ] {
            assert_eq!(parse_type(s), Type::from_path(s), "divergence on {:?}", s);
        }
    }
}
