// ── src/jurisdiction.rs ──────────────────────────────────────────────
// №886 (issue #1759, wave 79 — W-H, ADR-0209 §2.1): the jurisdiction-
// aware policy compilation (the 8D-2/C10 kernel). A jurisdiction rule
// is a TYPED CONFIGURATION OF LIMITS — never legal advice (ADR-0209
// §2/§5): the rule carries a POINTER to a norm (`norm_ref`) and the
// date the pointer was last verified (`checked_at`), never the text of
// the norm and never a compliance verdict. The matrix is append-only
// (the AttestationMap discipline, ADR-0206 §2.2); the compilation is
// fail-closed at the compile-site (ADR-0209 §2.3): a rule without its
// norm reference or its verification date refuses BEFORE anything
// runs, an unknown jurisdiction refuses, and the KPI C10 — 100% of
// the package rules carry a norm reference and a verification date —
// is a machine check, not a declaration.
//
// The dedup guard (№886): the type names are `JurisdictionRule` /
// `JurisdictionMatrix` — apart from every existing surface. The
// parents are consumed by reference (ADR-0209 §3): the carrier names
// are the EXISTING name space the `ToolPolicy` compiler and
// `builtins_classification` speak; a jurisdiction effect never WIDENS
// a right (ADR-0203 stays the rights SSOT) and never adds a second
// network gate (ADR-0207 stays the egress SSOT). The refusal
// vocabulary stays frozen (№385/ADR-0169): the deny-composition
// refusal rides the EXISTING `IRREVERSIBLE_NO_GRANT` class.

use sha2::{Digest, Sha256};

/// The canonical separator (the compliance/canonical-row form of
/// №828/№868 — the same `\x1f` byte, the same determinism contract).
const SEP: char = '\u{1f}';

/// The typed jurisdiction identifier (ADR-0209 §4): an uppercase
/// country code, optionally narrowed by an uppercase region
/// (`EU`, `RU`, `US-CA`). The validation is the contract — a
/// lowercase, longer or empty code refuses at construction.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct JurisdictionId(String);

impl JurisdictionId {
    pub fn new(code: &str) -> Result<Self, String> {
        let ok_len = code.len() == 2 || (code.len() == 5 && code.as_bytes()[2] == b'-');
        let ok_chars = code
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_uppercase() || (i == 2 && b == b'-'));
        if !ok_len || !ok_chars {
            return Err(format!(
                "jurisdiction: '{code}' is not a valid jurisdiction code (the form is AB or AB-CD, uppercase)"
            ));
        }
        Ok(JurisdictionId(code.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The jurisdiction effect over a carrier (ADR-0209 §2.1): allow — the
/// carrier stays available; deny — the carrier refuses (the
/// composition site turns it into the pre-run refusal); limit — the
/// carrier stays available under the jurisdiction's limits (the view
/// MARKS it; the consumer honors the mark, the kernel does not
/// interpret the norm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
    Limit,
}

impl PolicyEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            PolicyEffect::Allow => "allow",
            PolicyEffect::Deny => "deny",
            PolicyEffect::Limit => "limit",
        }
    }
}

/// The typed jurisdiction rule (ADR-0209 §2.1): a configuration of
/// limits over one carrier in one jurisdiction. `norm_ref` is a
/// REFERENCE — the pointer the auditor follows, never the legal text;
/// `checked_at` is the ISO date the pointer was last verified. A rule
/// without either is not a rule (the KPI C10, fail-closed).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JurisdictionRule {
    pub jurisdiction: JurisdictionId,
    /// The carrier/subject name — the EXISTING name space (the
    /// builtin/tool-carrier names, e.g. `forget`, `http_post`).
    pub carrier: String,
    pub effect: PolicyEffect,
    /// The pointer to the norm (e.g. "GDPR (EU) 2016/679 Art. 17").
    pub norm_ref: String,
    /// The verification date of the pointer, ISO `YYYY-MM-DD`.
    pub checked_at: String,
    /// Who supplied the rule (the provenance of the configuration).
    pub source: String,
    /// The row digest (the G5 №828 form): SHA-256 over the canonical
    /// serialization of every field above. A row without a digest is
    /// not a row of the matrix.
    pub row_digest: String,
}

/// `YYYY-MM-DD` with a real calendar day (the honest date check —
/// `2026-02-30` refuses at construction, not at the audit).
fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let (y, m, d) = (
        s[0..4].parse::<u32>(),
        s[5..7].parse::<u32>(),
        s[8..10].parse::<u32>(),
    );
    let (Ok(y), Ok(m), Ok(d)) = (y, m, d) else {
        return false;
    };
    if !(1..=9999).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return false;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let feb = if leap { 29 } else { 28 };
    let max = match m {
        2 => feb,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    d <= max
}

impl JurisdictionRule {
    /// Construct + digest-pin one rule (the fail-closed constructor:
    /// an empty carrier, an empty norm reference, an invalid date or a
    /// bad effect refuses HERE — the KPI C10 is enforced at the
    /// construction site, not at the audit).
    pub fn new(
        jurisdiction: JurisdictionId,
        carrier: &str,
        effect: PolicyEffect,
        norm_ref: &str,
        checked_at: &str,
        source: &str,
    ) -> Result<Self, String> {
        if carrier.trim().is_empty() {
            return Err(
                "jurisdiction: the rule carrier is empty — a rule names a carrier".to_string(),
            );
        }
        if norm_ref.trim().is_empty() {
            return Err(format!(
                "jurisdiction: the rule over '{carrier}' in {} has no norm reference — the KPI C10 refuses an unnamed norm (fail-closed)",
                jurisdiction.as_str()
            ));
        }
        if !is_iso_date(checked_at) {
            return Err(format!(
                "jurisdiction: the rule over '{carrier}' in {} has no valid verification date '{checked_at}' (YYYY-MM-DD, a real calendar day) — the KPI C10 refuses (fail-closed)",
                jurisdiction.as_str()
            ));
        }
        if source.trim().is_empty() {
            return Err(
                "jurisdiction: the rule source is empty — a configuration names its supplier"
                    .to_string(),
            );
        }
        let rule = JurisdictionRule {
            jurisdiction,
            carrier: carrier.to_string(),
            effect,
            norm_ref: norm_ref.to_string(),
            checked_at: checked_at.to_string(),
            source: source.to_string(),
            row_digest: String::new(),
        };
        let digest = canonical_rule_digest(&rule);
        Ok(JurisdictionRule {
            row_digest: digest,
            ..rule
        })
    }
}

/// The canonical row serialization (the №828/№868 form: the fixed
/// field order, the `\x1f` separator, pure and deterministic).
pub fn canonical_rule(rule: &JurisdictionRule) -> String {
    [
        "v1",
        rule.jurisdiction.as_str(),
        &rule.carrier,
        rule.effect.as_str(),
        &rule.norm_ref,
        &rule.checked_at,
        &rule.source,
    ]
    .join(&SEP.to_string())
}

/// The row digest: SHA-256 over `canonical_rule`, hex-encoded (the G5
/// form of №828).
pub fn canonical_rule_digest(rule: &JurisdictionRule) -> String {
    let digest = Sha256::digest(canonical_rule(rule).as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

/// The append-only jurisdiction matrix (ADR-0209 §2.2, the
/// AttestationMap discipline): issuing adds, nothing deletes; the rows
/// stay in issue order; a changed rule is a NEW row that supersedes —
/// never a silent overwrite.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JurisdictionMatrix {
    rows: Vec<JurisdictionRule>,
}

impl JurisdictionMatrix {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append one rule. An EXACT duplicate (the same row digest)
    /// refuses: the same configuration twice is a construction error,
    /// not a row. A re-configuration of the same carrier is a NEW row
    /// — the effect resolution applies the strictest-wins precedence.
    pub fn issue(&mut self, rule: JurisdictionRule) -> Result<usize, String> {
        if self.rows.iter().any(|r| r.row_digest == rule.row_digest) {
            return Err(format!(
                "jurisdiction matrix: the exact row {} is already in the matrix (append-only, no duplicates)",
                rule.row_digest
            ));
        }
        self.rows.push(rule);
        Ok(self.rows.len() - 1)
    }

    /// The read-only row view (the compiler's input).
    pub fn rows(&self) -> &[JurisdictionRule] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The distinct jurisdiction codes in the matrix, sorted (the
    /// JURISDICTION_MATRIX fact counts THESE — №888 counts from the
    /// machine source, never a declared number).
    pub fn jurisdictions(&self) -> Vec<&str> {
        let mut codes: Vec<&str> = self.rows.iter().map(|r| r.jurisdiction.as_str()).collect();
        codes.sort_unstable();
        codes.dedup();
        codes
    }

    /// The rules over one jurisdiction (issue order).
    pub fn rules_for(&self, jurisdiction: &str) -> Vec<&JurisdictionRule> {
        self.rows
            .iter()
            .filter(|r| r.jurisdiction.as_str() == jurisdiction)
            .collect()
    }

    /// The strictest effect over a carrier in a jurisdiction — the
    /// precedence is deny > limit > allow (ADR-0209 §2.3: the
    /// jurisdiction layer never widens; the strictest configuration
    /// wins when rows disagree).
    pub fn effect_for(&self, jurisdiction: &str, carrier: &str) -> Option<PolicyEffect> {
        let mut strictest: Option<PolicyEffect> = None;
        for r in self
            .rows
            .iter()
            .filter(|r| r.jurisdiction.as_str() == jurisdiction && r.carrier == carrier)
        {
            let rank = match r.effect {
                PolicyEffect::Deny => 3,
                PolicyEffect::Limit => 2,
                PolicyEffect::Allow => 1,
            };
            let cur = strictest
                .map(|e| match e {
                    PolicyEffect::Deny => 3,
                    PolicyEffect::Limit => 2,
                    PolicyEffect::Allow => 1,
                })
                .unwrap_or(0);
            if rank > cur {
                strictest = Some(r.effect);
            }
        }
        strictest
    }

    /// The matrix digest: SHA-256 over the row digests in issue order
    /// (append-only stability — the digest changes only when the
    /// configuration changes).
    pub fn digest(&self) -> String {
        let joined = self
            .rows
            .iter()
            .map(|r| r.row_digest.as_str())
            .collect::<Vec<_>>()
            .join(&SEP.to_string());
        let digest = Sha256::digest(joined.as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push_str(&format!("{:02x}", byte));
        }
        hex
    }
}

/// The compiled per-jurisdiction policy view (ADR-0209 §2.3): the
/// carriers grouped by the strictest effect, plus the digest of the
/// matrix the view was compiled from (the compiled artifact is
/// reproducible from its source).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JurisdictionPolicy {
    pub jurisdiction: JurisdictionId,
    pub deny: Vec<String>,
    pub limit: Vec<String>,
    pub allow: Vec<String>,
    /// The matrix digest this view was compiled from.
    pub matrix_digest: String,
}

impl JurisdictionPolicy {
    /// The strictest effect over a carrier in this compiled view.
    pub fn effect_of(&self, carrier: &str) -> Option<PolicyEffect> {
        if self.deny.iter().any(|c| c == carrier) {
            Some(PolicyEffect::Deny)
        } else if self.limit.iter().any(|c| c == carrier) {
            Some(PolicyEffect::Limit)
        } else if self.allow.iter().any(|c| c == carrier) {
            Some(PolicyEffect::Allow)
        } else {
            None
        }
    }
}

/// The gate verdict over ONE carrier (№887 — the coverage rule, the
/// №845 form): the fail-closed resolution at the CONSUMPTION point.
/// An uncovered carrier CANNOT resolve to an implicit allow — «молча
/// пропустить» is forbidden: the verdict refuses, and the consumer
/// (the MCP composition, ADR-0196) refuses the run with it. The KPI
/// C10 stays a machine check on the consumed path, not a declaration.
pub fn jurisdiction_verdict(
    policy: &JurisdictionPolicy,
    carrier: &str,
) -> Result<PolicyEffect, String> {
    match policy.effect_of(carrier) {
        Some(effect) => Ok(effect),
        None => Err(format!(
            "jurisdiction: no rule covers '{carrier}' in {} — the gate verdict refuses an uncovered carrier (fail-closed, KPI C10; the №845 form)",
            policy.jurisdiction.as_str()
        )),
    }
}

/// Compile the per-jurisdiction policy view — FAIL-CLOSED (ADR-0209
/// §2.3): an unknown jurisdiction refuses; a rule without its norm
/// reference or its verification date refuses the compilation BEFORE
/// anything runs. The re-check is deliberate defence-in-depth: the
/// constructor refuses at `JurisdictionRule::new`, but a rule
/// deserialized from a foreign payload bypasses the constructor — the
/// compile-site refuses it here (the KPI C10 is a machine check on
/// the compiled path, not a constructor courtesy).
pub fn compile_jurisdiction_policy(
    matrix: &JurisdictionMatrix,
    jurisdiction: &str,
) -> Result<JurisdictionPolicy, String> {
    let known = matrix.jurisdictions().contains(&jurisdiction);
    if !known {
        return Err(format!(
            "jurisdiction: '{jurisdiction}' is not in the matrix — an unknown jurisdiction refuses (fail-closed)"
        ));
    }
    let mut deny = Vec::new();
    let mut limit = Vec::new();
    let mut allow = Vec::new();
    for rule in matrix.rules_for(jurisdiction) {
        if rule.norm_ref.trim().is_empty() || !is_iso_date(&rule.checked_at) {
            return Err(format!(
                "jurisdiction: the rule over '{}' in {} is incomplete (norm_ref/checked_at) — the compilation refuses before any run (fail-closed, KPI C10)",
                rule.carrier,
                rule.jurisdiction.as_str()
            ));
        }
        let bucket = match rule.effect {
            PolicyEffect::Deny => &mut deny,
            PolicyEffect::Limit => &mut limit,
            PolicyEffect::Allow => &mut allow,
        };
        if !bucket.iter().any(|c| c == &rule.carrier) {
            bucket.push(rule.carrier.clone());
        }
    }
    Ok(JurisdictionPolicy {
        jurisdiction: JurisdictionId::new(jurisdiction)?,
        deny,
        limit,
        allow,
        matrix_digest: matrix.digest(),
    })
}

/// The seed matrix (ADR-0209 §4, №886): the initial jurisdiction
/// configuration as CODE DATA — append-only, superseded by new rows,
/// never edited in place. The seed rules are conservative and honest:
/// each names a real norm pointer and a verification date; the frame
/// is the ADR's own — a configuration of limits, NOT legal advice
/// (ADR-0209 §5). The seed proves the machine: ≥3 jurisdictions for
/// the JURISDICTION_MATRIX fact (gate_040, №888) and the e2e trace of
/// №887.
#[allow(clippy::expect_used)] // the seed constants are valid by construction: fixed jurisdiction codes ("EU"/"RU"/"US-CA"), fixed ISO dates, fixed norm pointers — the expects fire only on a programming error, never on data
pub fn seed_matrix() -> JurisdictionMatrix {
    let mut m = JurisdictionMatrix::new();
    let mut issue = |jur: &str, carrier: &str, effect: PolicyEffect, norm: &str, src: &str| {
        let rule = JurisdictionRule::new(
            JurisdictionId::new(jur).expect("seed code"),
            carrier,
            effect,
            norm,
            "2026-11-03",
            src,
        )
        .expect("seed rule");
        m.issue(rule).expect("seed append");
    };
    // The erasure path stays AVAILABLE where the norm grants the right
    // to erasure — an `allow` that keeps the deletion carrier
    // reachable (the erasure lines of the wave-22 GDPR posture, №521).
    issue(
        "EU",
        "forget",
        PolicyEffect::Allow,
        "GDPR (EU) 2016/679 Art. 17",
        "seed/wave-79 (№886)",
    );
    issue(
        "RU",
        "forget",
        PolicyEffect::Allow,
        "152-ФЗ «О персональных данных» ст. 21",
        "seed/wave-79 (№886)",
    );
    issue(
        "US-CA",
        "forget",
        PolicyEffect::Allow,
        "Cal. Civ. Code §1798.105 (CPRA)",
        "seed/wave-79 (№886)",
    );
    // The data-minimisation limit over the network egress carrier — a
    // `limit` MARK (the consumer honors it; the kernel does not
    // interpret the norm — ADR-0209 §5).
    issue(
        "EU",
        "http_post",
        PolicyEffect::Limit,
        "GDPR (EU) 2016/679 Art. 5(1)(c)",
        "seed/wave-79 (№886)",
    );
    m
}

#[cfg(test)]
mod kernel_tests {
    //! The negative controls are part of the CONTRACT (ADR-0209 §4,
    //! the №867 form): the refusals run at construction/compile time,
    //! not only in the integration suite.

    use super::*;

    fn rule(jur: &str, carrier: &str, effect: PolicyEffect) -> JurisdictionRule {
        JurisdictionRule::new(
            JurisdictionId::new(jur).unwrap(),
            carrier,
            effect,
            "GDPR (EU) 2016/679 Art. 17",
            "2026-11-03",
            "test",
        )
        .unwrap()
    }

    #[test]
    fn jurisdiction_id_validation() {
        for good in ["EU", "RU", "US", "US-CA", "DE"] {
            assert!(JurisdictionId::new(good).is_ok(), "{good}");
        }
        for bad in ["", "e", "eu", "EU-", "usa", "US-CAL", "US_CA", "E1", "EU--"] {
            assert!(JurisdictionId::new(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn rule_refuses_an_empty_norm_reference() {
        let err = JurisdictionRule::new(
            JurisdictionId::new("EU").unwrap(),
            "forget",
            PolicyEffect::Allow,
            "   ",
            "2026-11-03",
            "test",
        )
        .unwrap_err();
        assert!(err.contains("no norm reference"), "{err}");
        assert!(err.contains("KPI C10"), "{err}");
    }

    #[test]
    fn rule_refuses_an_invalid_verification_date() {
        for bad in [
            "",
            "2026-11-3",
            "26-11-03",
            "2026-13-01",
            "2026-02-30",
            "2026-11-32",
        ] {
            let err = JurisdictionRule::new(
                JurisdictionId::new("EU").unwrap(),
                "forget",
                PolicyEffect::Allow,
                "GDPR (EU) 2016/679 Art. 17",
                bad,
                "test",
            )
            .unwrap_err();
            assert!(err.contains("verification date"), "{bad}: {err}");
        }
        for good in ["2026-11-03", "2024-02-29", "2026-12-31"] {
            assert!(
                JurisdictionRule::new(
                    JurisdictionId::new("EU").unwrap(),
                    "forget",
                    PolicyEffect::Allow,
                    "n",
                    good,
                    "s",
                )
                .is_ok(),
                "{good}"
            );
        }
    }

    #[test]
    fn rule_refuses_an_empty_carrier_and_source() {
        assert!(JurisdictionRule::new(
            JurisdictionId::new("EU").unwrap(),
            " ",
            PolicyEffect::Deny,
            "n",
            "2026-11-03",
            "s"
        )
        .is_err());
        assert!(JurisdictionRule::new(
            JurisdictionId::new("EU").unwrap(),
            "forget",
            PolicyEffect::Deny,
            "n",
            "2026-11-03",
            ""
        )
        .is_err());
    }

    #[test]
    fn canonical_row_is_deterministic_and_digest_pinned() {
        let a = rule("EU", "forget", PolicyEffect::Allow);
        let b = rule("EU", "forget", PolicyEffect::Allow);
        assert_eq!(canonical_rule(&a), canonical_rule(&b));
        assert_eq!(a.row_digest, b.row_digest);
        assert_eq!(a.row_digest.len(), 64);
        assert!(a
            .row_digest
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
        // a changed field changes the digest
        let c = rule("RU", "forget", PolicyEffect::Allow);
        assert_ne!(a.row_digest, c.row_digest);
    }

    #[test]
    fn matrix_is_append_only_and_refuses_exact_duplicates() {
        let mut m = JurisdictionMatrix::new();
        assert!(m.issue(rule("EU", "forget", PolicyEffect::Allow)).is_ok());
        let err = m
            .issue(rule("EU", "forget", PolicyEffect::Allow))
            .unwrap_err();
        assert!(err.contains("append-only"), "{err}");
        // a NEW row with a different effect is a supersede, not a dup
        assert!(m.issue(rule("EU", "forget", PolicyEffect::Deny)).is_ok());
        assert_eq!(m.len(), 2);
        // no removal API exists — the append-only discipline is the type
    }

    #[test]
    fn effect_resolution_is_strictest_wins() {
        let mut m = JurisdictionMatrix::new();
        m.issue(rule("EU", "forget", PolicyEffect::Allow)).unwrap();
        m.issue(rule("EU", "forget", PolicyEffect::Limit)).unwrap();
        m.issue(rule("EU", "forget", PolicyEffect::Deny)).unwrap();
        assert_eq!(m.effect_for("EU", "forget"), Some(PolicyEffect::Deny));
        assert_eq!(m.effect_for("EU", "http_post"), None);
        assert_eq!(m.effect_for("RU", "forget"), None);
    }

    #[test]
    fn jurisdictions_are_distinct_and_sorted() {
        let m = seed_matrix();
        assert_eq!(m.jurisdictions(), vec!["EU", "RU", "US-CA"]);
    }

    #[test]
    fn compile_refuses_an_unknown_jurisdiction() {
        let m = seed_matrix();
        let err = compile_jurisdiction_policy(&m, "XX").unwrap_err();
        assert!(err.contains("not in the matrix"), "{err}");
        assert!(err.contains("fail-closed"), "{err}");
    }

    #[test]
    fn compile_refuses_a_deserialized_incomplete_rule() {
        // A rule that bypasses the constructor (the foreign-payload
        // form): the compile-site re-check refuses it — the KPI C10
        // holds on the compiled path, not just at construction.
        let mut m = seed_matrix();
        let mut bad = rule("EU", "forget", PolicyEffect::Allow);
        bad.norm_ref = String::new();
        bad.row_digest = canonical_rule_digest(&bad);
        m.rows.push(bad);
        let err = compile_jurisdiction_policy(&m, "EU").unwrap_err();
        assert!(err.contains("incomplete"), "{err}");
        assert!(err.contains("fail-closed"), "{err}");
    }

    #[test]
    fn compile_groups_by_strictest_effect_and_pins_the_matrix_digest() {
        let m = seed_matrix();
        let p = compile_jurisdiction_policy(&m, "EU").unwrap();
        assert_eq!(p.effect_of("forget"), Some(PolicyEffect::Allow));
        assert_eq!(p.effect_of("http_post"), Some(PolicyEffect::Limit));
        assert_eq!(p.effect_of("exec"), None);
        assert_eq!(p.matrix_digest, m.digest());
        let p2 = compile_jurisdiction_policy(&m, "RU").unwrap();
        assert_eq!(p2.effect_of("forget"), Some(PolicyEffect::Allow));
        assert_eq!(p2.effect_of("http_post"), None);
    }

    #[test]
    fn matrix_digest_is_append_order_stable() {
        let a = seed_matrix();
        let b = seed_matrix();
        assert_eq!(a.digest(), b.digest());
        let mut c = seed_matrix();
        c.issue(rule("US-CA", "http_post", PolicyEffect::Limit))
            .unwrap();
        assert_ne!(a.digest(), c.digest());
    }

    #[test]
    fn verdict_resolves_the_covered_carriers() {
        let p = compile_jurisdiction_policy(&seed_matrix(), "EU").unwrap();
        assert_eq!(
            jurisdiction_verdict(&p, "http_post").unwrap(),
            PolicyEffect::Limit
        );
        assert_eq!(
            jurisdiction_verdict(&p, "forget").unwrap(),
            PolicyEffect::Allow
        );
    }

    #[test]
    fn verdict_refuses_an_uncovered_carrier_fail_closed() {
        // «Молчаливый пропуск невозможен»: an uncovered carrier cannot
        // pass as an implicit allow — the verdict refuses (№887, the
        // №845 form; the KPI C10 on the consumed path).
        let p = compile_jurisdiction_policy(&seed_matrix(), "EU").unwrap();
        let err = jurisdiction_verdict(&p, "exec").unwrap_err();
        assert!(err.contains("no rule covers"), "{err}");
        assert!(err.contains("fail-closed"), "{err}");
        assert!(err.contains("KPI C10"), "{err}");
    }

    #[test]
    fn verdict_refusal_names_the_jurisdiction_and_tracks_the_supersede() {
        // US-CA covers forget, NOT http_post — the refusal names the
        // jurisdiction the consumer compiled under.
        let p = compile_jurisdiction_policy(&seed_matrix(), "US-CA").unwrap();
        assert_eq!(
            jurisdiction_verdict(&p, "forget").unwrap(),
            PolicyEffect::Allow
        );
        let err = jurisdiction_verdict(&p, "http_post").unwrap_err();
        assert!(err.contains("US-CA"), "{err}");
        // a supersede row re-shapes the verdict (strictest wins)
        let mut m = seed_matrix();
        m.issue(rule("EU", "http_post", PolicyEffect::Deny))
            .unwrap();
        let p2 = compile_jurisdiction_policy(&m, "EU").unwrap();
        assert_eq!(
            jurisdiction_verdict(&p2, "http_post").unwrap(),
            PolicyEffect::Deny
        );
    }
}
