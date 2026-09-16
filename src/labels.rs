// ── Label lattice (Наряд №322, ADR-0154) ─────────────────────────────
//
//! Three-component security label: `(conf, integrity, consent-scope)`.
//!
//! Prior art (mandatory references per naryad №322 / plan v2 §13.3):
//! - **DLM / Jif** (Myers): decentralized labels — confidentiality +
//!   integrity policies as first-class values, componentwise joins.
//! - **FlowCaml** (Simonet): ML with static information-flow, lattice
//!   of security levels with pointwise ordering.
//! - **LIO** (Stefan et al.): label arithmetic in a dynamic IFC kernel —
//!   join on data combination, meet on requirement combination, and the
//!   observation that `label.join` in confidentiality space corresponds
//!   to `label.meet` in integrity space.
//!
//! ## The lattice (ADR-0154 §2)
//!
//! **Conf axis** — `public < consented < private`, plus the quarantine
//! element `poisoned`. `poisoned` is NOT an ordinary top: it is
//! absorbing for BOTH join and meet (NaN-like quarantine semantics).
//! Any value derived from a poisoned value stays poisoned and has no
//! legal sinks. Absorbing meet is the safety-critical choice: if
//! `meet(poisoned, private) == private`, a malicious `meet` would be a
//! one-step declassifier; quarantine must not be curable by lattice
//! arithmetic.
//!
//! ```text
//!         poisoned            (quarantine — absorbing, no legal sinks)
//!             |
//!          private
//!             |
//!         consented           (released under a consent scope)
//!             |
//!          public
//! ```
//!
//! **Integrity axis** — `untrusted < trusted`. Dual to confidentiality
//! (DLM/LIO): combining data can only LOWER integrity (join = min),
//! while combining requirements keeps the STRONGEST (meet = max).
//!
//! **Consent-scope axis** — the set of consent scopes a value is
//! covered by (ties to `consent_ledger`, `src/voice/store.rs`; ledger
//! itself is the Phase-2 consent-source work, naryad №335). Empty set
//! = no consent backing. join = intersection (a value usable in two
//! contexts retains only the consent both contexts carry), meet =
//! union (requirements combine permissively). The consent axis only
//! matters below `private` on the conf axis; `poisoned` dominates
//! everything.
//!
//! join/meet are componentwise (ADR-0154 D2). Rejected alternatives
//! are recorded in the ADR (single numeric lattice; independent
//! dimensions without meet).
//!
//! ## Annotation syntax (parsed by the grammar, validated here)
//!
//! ```text
//! String<private>
//! String<private, untrusted>
//! String<consented, trusted, consent(gdpr, analytics)>
//! ```
//!
//! The AST stores the raw annotation text (`ast::LabelAnn`); semantic
//! analysis parses it via [`Label::parse`] and reports unknown words
//! with the annotation's span.

use std::collections::BTreeSet;
use std::fmt;

// ── Conf axis ────────────────────────────────────────────────────────

/// Confidentiality component. Display/parse order:
/// `public < consented < private < poisoned` (ADR-0154 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Conf {
    /// World-visible.
    #[default]
    Public,
    /// Released under a consent scope (the scope itself lives in
    /// [`Label::consent`]).
    Consented,
    /// Private (e.g. user data, secrets).
    Private,
    /// Quarantine — absorbing for join AND meet; no legal sinks.
    Poisoned,
}

impl Conf {
    /// Combination of data: `poisoned` absorbs everything, otherwise
    /// the stronger (max) level wins.
    pub fn join(self, other: Conf) -> Conf {
        match (self, other) {
            (Conf::Poisoned, _) | (_, Conf::Poisoned) => Conf::Poisoned,
            (a, b) => a.max(b),
        }
    }

    /// Combination of requirements: `poisoned` absorbs everything
    /// (quarantine is not curable by meet — ADR-0154 D1), otherwise the
    /// weaker (min) level wins.
    pub fn meet(self, other: Conf) -> Conf {
        match (self, other) {
            (Conf::Poisoned, _) | (_, Conf::Poisoned) => Conf::Poisoned,
            (a, b) => a.min(b),
        }
    }

    fn parse_word(word: &str) -> Option<Conf> {
        match word {
            "public" => Some(Conf::Public),
            "consented" => Some(Conf::Consented),
            "private" => Some(Conf::Private),
            "poisoned" => Some(Conf::Poisoned),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Conf::Public => "public",
            Conf::Consented => "consented",
            Conf::Private => "private",
            Conf::Poisoned => "poisoned",
        }
    }

    /// Public word form (№331: `media_meta` reports the declared conf).
    pub fn as_str(self) -> &'static str {
        self.word()
    }
}

// ── Integrity axis ───────────────────────────────────────────────────

/// Integrity component: `untrusted < trusted`. Dual to conf (DLM/LIO):
/// data combination takes the weaker integrity (join = min),
/// requirement combination keeps the strongest (meet = max).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Integrity {
    /// Derived from (or mixed with) untrusted sources — user input,
    /// LLM output, remote payloads.
    Untrusted,
    /// Trusted: literals, sanitized data, compiler-derived values.
    /// Default so that [`Label::bottom()`] / [`Label::default()`] is
    /// `public, trusted` — a missing integrity word never silently
    /// degrades a value (ADR-0154 §3).
    #[default]
    Trusted,
}

impl Integrity {
    /// Data combination: the weaker integrity wins.
    pub fn join(self, other: Integrity) -> Integrity {
        self.min(other)
    }

    /// Requirement combination: the stronger integrity wins.
    pub fn meet(self, other: Integrity) -> Integrity {
        self.max(other)
    }

    fn parse_word(word: &str) -> Option<Integrity> {
        match word {
            "untrusted" => Some(Integrity::Untrusted),
            "trusted" => Some(Integrity::Trusted),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Integrity::Untrusted => "untrusted",
            Integrity::Trusted => "trusted",
        }
    }
}

// ── Consent-scope axis ───────────────────────────────────────────────

/// Set of consent scopes a value is covered by. Empty = no consent
/// backing. join = intersection, meet = union (ADR-0154 §2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConsentScope(BTreeSet<String>);

impl ConsentScope {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn from_scopes<I, S>(scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self(scopes.into_iter().map(Into::into).collect())
    }

    pub fn scopes(&self) -> &BTreeSet<String> {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Intersection — a value flowing through two contexts keeps only
    /// the consent both contexts carry.
    pub fn join(self, other: ConsentScope) -> ConsentScope {
        ConsentScope(self.0.intersection(&other.0).cloned().collect())
    }

    /// Union — requirements combine permissively.
    pub fn meet(self, other: ConsentScope) -> ConsentScope {
        ConsentScope(self.0.union(&other.0).cloned().collect())
    }
}

impl fmt::Display for ConsentScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        write!(
            f,
            "consent({})",
            self.0.clone().into_iter().collect::<Vec<_>>().join(", ")
        )
    }
}

// ── Label ────────────────────────────────────────────────────────────

/// Three-component security label `(conf, integrity, consent-scope)`.
/// join/meet are componentwise (ADR-0154 D2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Label {
    pub conf: Conf,
    pub integrity: Integrity,
    pub consent: ConsentScope,
}

impl Label {
    /// The bottom label: fully open — `public, trusted`, no consent.
    pub fn bottom() -> Label {
        Label::default()
    }

    /// Componentwise join (data combination).
    pub fn join(&self, other: &Label) -> Label {
        Label {
            conf: self.conf.join(other.conf),
            integrity: self.integrity.join(other.integrity),
            consent: self.consent.clone().join(other.consent.clone()),
        }
    }

    /// Componentwise meet (requirement combination).
    pub fn meet(&self, other: &Label) -> Label {
        Label {
            conf: self.conf.meet(other.conf),
            integrity: self.integrity.meet(other.integrity),
            consent: self.consent.clone().meet(other.consent.clone()),
        }
    }
}

impl fmt::Display for Label {
    /// Canonical rendering — always all static components, consent only
    /// when non-empty:
    /// `public, trusted`, `private, untrusted, consent(gdpr, analytics)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.conf.word(), self.integrity.word())?;
        if !self.consent.is_empty() {
            write!(f, ", {}", self.consent)?;
        }
        Ok(())
    }
}

// ── Annotation parsing ───────────────────────────────────────────────

/// Error produced by [`Label::parse`] — carries enough context for the
/// semantic layer to point at the offending word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelParseError {
    pub kind: LabelParseErrorKind,
    /// The offending word (or the offending part) as written.
    pub word: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelParseErrorKind {
    /// Annotation is empty / whitespace.
    Empty,
    /// Conf word repeated.
    DuplicateConf,
    /// Integrity word repeated.
    DuplicateIntegrity,
    /// More than one consent(...) part.
    DuplicateConsent,
    /// Word is neither a conf word, an integrity word, nor consent(...).
    UnknownWord,
    /// consent(...) with an empty scope list.
    EmptyConsentList,
    /// No conf word at all — conf is REQUIRED (ADR-0154 §3): a bare
    /// `<untrusted>` must not silently mean "public".
    MissingConf,
}

impl fmt::Display for LabelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            LabelParseErrorKind::Empty => write!(f, "empty label annotation"),
            LabelParseErrorKind::DuplicateConf => {
                write!(f, "duplicate confidentiality word '{}'", self.word)
            }
            LabelParseErrorKind::DuplicateIntegrity => {
                write!(f, "duplicate integrity word '{}'", self.word)
            }
            LabelParseErrorKind::DuplicateConsent => {
                write!(f, "duplicate consent part '{}'", self.word)
            }
            LabelParseErrorKind::UnknownWord => write!(f, "unknown label word '{}'", self.word),
            LabelParseErrorKind::EmptyConsentList => {
                write!(f, "consent() with empty scope list")
            }
            LabelParseErrorKind::MissingConf => {
                write!(f, "label annotation has no confidentiality word (public|consented|private|poisoned)")
            }
        }
    }
}

impl Label {
    /// Parse an annotation body (the text between `<` and `>`),
    /// e.g. `"private, untrusted, consent(gdpr, analytics)"`.
    ///
    /// Component rules (ADR-0154 §3):
    /// - exactly one conf word (`public|consented|private|poisoned`);
    /// - at most one integrity word (`trusted|untrusted`);
    /// - at most one `consent(scope, ...)` part;
    /// - conf is REQUIRED (an annotation without a conf word is
    ///   ambiguous — refusing it keeps `String<>` from silently meaning
    ///   "public", the loudest default would be the wrong one);
    /// - missing integrity defaults to `trusted`; missing consent
    ///   defaults to the empty set. Defaults are only safe on the
    ///   permissive side: a value without an integrity word is trusted
    ///   unless proven otherwise, and every downgrade remains an
    ///   explicit annotation act.
    pub fn parse(raw: &str) -> Result<Label, LabelParseError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(LabelParseError {
                kind: LabelParseErrorKind::Empty,
                word: String::new(),
            });
        }

        let mut conf: Option<Conf> = None;
        let mut integrity: Option<Integrity> = None;
        let mut consent: Option<ConsentScope> = None;

        // Split on commas OUTSIDE consent(...) parens — scope lists
        // themselves are comma-separated (`consent(gdpr, analytics)`).
        let mut parts: Vec<String> = Vec::new();
        let mut depth = 0usize;
        let mut current = String::new();
        for ch in raw.chars() {
            match ch {
                '(' => {
                    depth += 1;
                    current.push(ch);
                }
                ')' => {
                    if depth == 0 {
                        return Err(LabelParseError {
                            kind: LabelParseErrorKind::UnknownWord,
                            word: current.clone(),
                        });
                    }
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    parts.push(current.clone());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }
        parts.push(current);
        if depth != 0 {
            return Err(LabelParseError {
                kind: LabelParseErrorKind::UnknownWord,
                word: raw.to_string(),
            });
        }

        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                return Err(LabelParseError {
                    kind: LabelParseErrorKind::UnknownWord,
                    word: part.to_string(),
                });
            }

            // consent(scopes) part? — exact `consent(` prefix check:
            // `consented` is a conf word and must NOT be caught here.
            if let Some(rest) = part.strip_prefix("consent(") {
                if consent.is_some() {
                    return Err(LabelParseError {
                        kind: LabelParseErrorKind::DuplicateConsent,
                        word: part.to_string(),
                    });
                }
                let inner = rest.strip_suffix(')').ok_or_else(|| LabelParseError {
                    kind: LabelParseErrorKind::UnknownWord,
                    word: part.to_string(),
                })?;
                let scopes: Vec<String> = inner
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if scopes.is_empty() {
                    return Err(LabelParseError {
                        kind: LabelParseErrorKind::EmptyConsentList,
                        word: part.to_string(),
                    });
                }
                consent = Some(ConsentScope::from_scopes(scopes));
                continue;
            }

            // Conf word?
            if let Some(c) = Conf::parse_word(part) {
                if conf.is_some() {
                    return Err(LabelParseError {
                        kind: LabelParseErrorKind::DuplicateConf,
                        word: part.to_string(),
                    });
                }
                conf = Some(c);
                continue;
            }

            // Integrity word?
            if let Some(i) = Integrity::parse_word(part) {
                if integrity.is_some() {
                    return Err(LabelParseError {
                        kind: LabelParseErrorKind::DuplicateIntegrity,
                        word: part.to_string(),
                    });
                }
                integrity = Some(i);
                continue;
            }

            return Err(LabelParseError {
                kind: LabelParseErrorKind::UnknownWord,
                word: part.to_string(),
            });
        }

        let conf = conf.ok_or_else(|| LabelParseError {
            kind: LabelParseErrorKind::MissingConf,
            word: raw.to_string(),
        })?;

        Ok(Label {
            conf,
            integrity: integrity.unwrap_or(Integrity::Trusted),
            consent: consent.unwrap_or_default(),
        })
    }
}

// ── Legacy taint-kind projection (Наряд №322 task 3) ─────────────────

/// Projection of the legacy `TaintKind` kinds (`src/audit.rs:119`) onto
/// the label lattice — additive mapping, no removal of kinds, no
/// change to Category-A check behavior (ADR-0154 §5):
///
/// | TaintKind    | conf      | integrity | consent |
/// |--------------|-----------|-----------|---------|
/// | `LlmOutput`  | public    | untrusted | —       |
/// | `Secret`     | private   | trusted   | —       |
/// | `UserInput`  | public    | untrusted | —       |
/// | `Sanitized`  | public    | trusted   | —       |
/// | `CanaryLeak` | poisoned  | untrusted | —       |
///
/// Rationale:
/// - `LlmOutput` is meant for display (no conf concern) but is the
///   HTML_INJECTION vector — its problem is integrity, not secrecy.
/// - `Secret` is the confidentiality concern: private, integrity
///   trusted (the secret itself is intact; the LEAK is the sink's
///   problem, checked by SECRET_LEAK).
/// - `UserInput` is not secret but untrusted (SQL_DYNAMIC's vector).
/// - `Sanitized` passed through render()/escape_html(): trusted again.
/// - `CanaryLeak` is a confirmed-compromised channel — the quarantine
///   element: no legal sinks (advisory CANARY_LEAK today, sink-gate
///   №325 will make it structural).
///
/// Keyed by the `TaintKind` variant name; `src/audit.rs` carries an
/// exhaustiveness unit test asserting every variant has an entry here.
pub fn legacy_taint_label(kind: &str) -> Option<Label> {
    let (conf, integrity) = match kind {
        "LlmOutput" => (Conf::Public, Integrity::Untrusted),
        "Secret" => (Conf::Private, Integrity::Trusted),
        "UserInput" => (Conf::Public, Integrity::Untrusted),
        "Sanitized" => (Conf::Public, Integrity::Trusted),
        "CanaryLeak" => (Conf::Poisoned, Integrity::Untrusted),
        _ => return None,
    };
    Some(Label {
        conf,
        integrity,
        consent: ConsentScope::new(),
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn label(conf: Conf, integrity: Integrity, scopes: &[&str]) -> Label {
        Label {
            conf,
            integrity,
            consent: ConsentScope::from_scopes(scopes.iter().copied()),
        }
    }

    // ── Conf axis: order + poisoned absorbing ────────────────────────

    #[test]
    fn conf_order_is_public_lt_consented_lt_private_lt_poisoned() {
        assert!(Conf::Public < Conf::Consented);
        assert!(Conf::Consented < Conf::Private);
        assert!(Conf::Private < Conf::Poisoned);
    }

    #[test]
    fn conf_join_is_max() {
        assert_eq!(Conf::Public.join(Conf::Private), Conf::Private);
        assert_eq!(Conf::Consented.join(Conf::Public), Conf::Consented);
        assert_eq!(Conf::Private.join(Conf::Private), Conf::Private);
    }

    #[test]
    fn conf_meet_is_min() {
        assert_eq!(Conf::Public.meet(Conf::Private), Conf::Public);
        assert_eq!(Conf::Private.meet(Conf::Private), Conf::Private);
        assert_eq!(Conf::Consented.meet(Conf::Private), Conf::Consented);
    }

    #[test]
    fn poisoned_absorbs_join_and_meet() {
        // ADR-0154 D1: quarantine is absorbing for BOTH — meet must not
        // be a one-step declassifier.
        for c in [Conf::Public, Conf::Consented, Conf::Private] {
            assert_eq!(c.join(Conf::Poisoned), Conf::Poisoned);
            assert_eq!(Conf::Poisoned.join(c), Conf::Poisoned);
            assert_eq!(c.meet(Conf::Poisoned), Conf::Poisoned);
            assert_eq!(Conf::Poisoned.meet(c), Conf::Poisoned);
        }
        assert_eq!(Conf::Poisoned.join(Conf::Poisoned), Conf::Poisoned);
        assert_eq!(Conf::Poisoned.meet(Conf::Poisoned), Conf::Poisoned);
    }

    // ── Integrity axis ───────────────────────────────────────────────

    #[test]
    fn integrity_join_takes_weaker_meet_takes_stronger() {
        assert_eq!(
            Integrity::Trusted.join(Integrity::Untrusted),
            Integrity::Untrusted
        );
        assert_eq!(
            Integrity::Trusted.meet(Integrity::Untrusted),
            Integrity::Trusted
        );
        assert_eq!(
            Integrity::Untrusted.meet(Integrity::Untrusted),
            Integrity::Untrusted
        );
    }

    // ── Consent-scope axis ───────────────────────────────────────────

    #[test]
    fn consent_join_intersects_meet_unions() {
        let a = ConsentScope::from_scopes(["gdpr", "analytics"]);
        let b = ConsentScope::from_scopes(["gdpr", "marketing"]);
        assert_eq!(a.clone().join(b.clone()).scopes().len(), 1);
        assert!(a.clone().join(b.clone()).scopes().contains("gdpr"));
        assert_eq!(a.clone().meet(b.clone()).scopes().len(), 3);
        // Empty set = no consent backing: join with empty keeps nothing.
        assert!(a.clone().join(ConsentScope::new()).is_empty());
        assert_eq!(a.clone().meet(ConsentScope::new()).scopes().len(), 2);
    }

    // ── Label: componentwise join/meet ───────────────────────────────

    #[test]
    fn label_join_meet_are_componentwise() {
        let a = label(Conf::Public, Integrity::Trusted, &["gdpr"]);
        let b = label(Conf::Private, Integrity::Untrusted, &["analytics"]);
        let j = a.join(&b);
        assert_eq!(j.conf, Conf::Private); // max conf
        assert_eq!(j.integrity, Integrity::Untrusted); // min integrity
        assert!(j.consent.is_empty()); // intersection of disjoint sets
        let m = a.meet(&b);
        assert_eq!(m.conf, Conf::Public); // min conf
        assert_eq!(m.integrity, Integrity::Trusted); // max integrity
        assert_eq!(m.consent.scopes().len(), 2); // union
    }

    #[test]
    fn label_join_with_poisoned_is_poisoned_no_legal_sinks() {
        let clean = label(Conf::Public, Integrity::Trusted, &[]);
        let quarantine = label(Conf::Poisoned, Integrity::Untrusted, &[]);
        let j = clean.join(&quarantine);
        assert_eq!(j.conf, Conf::Poisoned);
        // meet also cannot cure quarantine (ADR-0154 D1).
        let m = clean.meet(&quarantine);
        assert_eq!(m.conf, Conf::Poisoned);
    }

    // ── Annotation parsing ───────────────────────────────────────────

    #[test]
    fn parse_conf_only() {
        let l = Label::parse("private").expect("parses");
        assert_eq!(l, label(Conf::Private, Integrity::Trusted, &[]));
    }

    #[test]
    fn parse_conf_and_integrity() {
        let l = Label::parse("public, untrusted").expect("parses");
        assert_eq!(l, label(Conf::Public, Integrity::Untrusted, &[]));
    }

    #[test]
    fn parse_full_three_components() {
        let l = Label::parse("consented, trusted, consent(gdpr, analytics)").expect("parses");
        assert_eq!(l.conf, Conf::Consented);
        assert_eq!(l.integrity, Integrity::Trusted);
        assert_eq!(l.consent.scopes().len(), 2);
    }

    #[test]
    fn parse_display_roundtrip_canonical() {
        let l = Label::parse("private, untrusted, consent(gdpr)").expect("parses");
        assert_eq!(l.to_string(), "private, untrusted, consent(gdpr)");
        // Canonical rendering of bottom.
        assert_eq!(Label::bottom().to_string(), "public, trusted");
        assert_eq!(
            Label::parse("poisoned").expect("parses").to_string(),
            "poisoned, trusted"
        );
    }

    #[test]
    fn parse_errors_are_loud() {
        assert_eq!(
            Label::parse("").unwrap_err().kind,
            LabelParseErrorKind::Empty
        );
        assert_eq!(
            Label::parse("   ").unwrap_err().kind,
            LabelParseErrorKind::Empty
        );
        assert_eq!(
            Label::parse("secretive").unwrap_err().kind,
            LabelParseErrorKind::UnknownWord
        );
        assert_eq!(
            Label::parse("private, private").unwrap_err().kind,
            LabelParseErrorKind::DuplicateConf
        );
        assert_eq!(
            Label::parse("private, trusted, untrusted")
                .unwrap_err()
                .kind,
            LabelParseErrorKind::DuplicateIntegrity
        );
        assert_eq!(
            Label::parse("private, consent(a), consent(b)")
                .unwrap_err()
                .kind,
            LabelParseErrorKind::DuplicateConsent
        );
        assert_eq!(
            Label::parse("private, consent()").unwrap_err().kind,
            LabelParseErrorKind::EmptyConsentList
        );
        // conf word is REQUIRED — `<>` must not silently mean "public".
        assert_eq!(
            Label::parse("untrusted").unwrap_err().kind,
            LabelParseErrorKind::MissingConf
        );
        assert_eq!(
            Label::parse("trusted, consent(a)").unwrap_err().kind,
            LabelParseErrorKind::MissingConf
        );
    }

    // ── Legacy taint-kind projection ─────────────────────────────────

    #[test]
    fn legacy_taint_projection_table() {
        let l = |k: &str| legacy_taint_label(k).expect("known kind");
        // LlmOutput: shown to users (public conf) but untrusted (HTML_INJECTION vector).
        assert_eq!(
            l("LlmOutput"),
            label(Conf::Public, Integrity::Untrusted, &[])
        );
        // Secret: the confidentiality concern.
        assert_eq!(l("Secret"), label(Conf::Private, Integrity::Trusted, &[]));
        // UserInput: not secret, untrusted (SQL_DYNAMIC vector).
        assert_eq!(
            l("UserInput"),
            label(Conf::Public, Integrity::Untrusted, &[])
        );
        // Sanitized: render()/escape_html() restore trust.
        assert_eq!(l("Sanitized"), label(Conf::Public, Integrity::Trusted, &[]));
        // CanaryLeak: confirmed-compromised channel → quarantine.
        assert_eq!(
            l("CanaryLeak"),
            label(Conf::Poisoned, Integrity::Untrusted, &[])
        );
        assert!(legacy_taint_label("NoSuchKind").is_none());
    }
}
