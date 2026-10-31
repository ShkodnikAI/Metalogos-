// ── Naryad №854 (wave 74 — W-C, 8F-1/C6): the Portable Refusal Algebra ─
//
// ADR-0204 §2: the refusal is a VALUE of a finite lattice, not a log
// line. This module is the SSOT of that algebra — dependency-free by
// contract (ADR-0204 §3: no IO, no Ledger, no language tree) so the
// WASM facade (№855) can compile the very same source file for
// `wasm32-unknown-unknown` without pulling anything.
//
// The reason set is the frozen 14-word №392 vocabulary (moved here from
// deny.rs — the SSOT follows the algebra; `deny.rs` re-exports it, so
// every `crate::deny::DENY_REASONS` path is unchanged). The codes are
// the frozen №385/ADR-0169 set — the algebra AGGREGATES existing codes
// (the code of a gate refusal IS the reason word), it never extends
// them.
//
// The lattice: the reasons form a chain over the canonical №392 array
// order (lower index = more severe — the core classes dominate the
// legacy ones). `join` = the more severe cause (two refusals combine →
// the dominant class carries the verdict), `meet` = the less severe.
// Chain ⇒ commutative, associative, idempotent, with absorption — all
// pinned over the FULL 14×14 table by `tests/naryad_854_refusal_algebra.rs`.
// `Allow` is the bottom element: `join(Allow, x) = x`,
// `meet(Allow, x) = Allow`.

/// The canonical deny-reason vocabulary — exhaustive over every reason
/// the gates can emit. The seven core classes first, then the extending
/// pair, the №455 file-path class, the generic class, then the legacy
/// corpus classes. THE SEVERITY RANK of the Portable Refusal Algebra
/// (ADR-0204 §2) is this array order: lower index = more severe.
pub const DENY_REASONS: [&str; 14] = [
    // ── the seven core classes (№392) ─────────────────────────────
    "VOICE_EGRESS_UNCONSENTED",
    "IRREVERSIBLE_NO_GRANT",
    "UNTRUSTED_EXEC_DECISION",
    "SECRET_TO_EXEC",
    "SECRET_EGRESS_VCS",
    "SECRET_EGRESS_NETWORK",
    "PII_EGRESS_NETWORK",
    // ── extending classes (same gate dictionary) ──────────────────
    "PII_EGRESS_OUTPUT",
    "UNTRUSTED_EGRESS_NETWORK",
    // ── №455: the file-ingest gate ────────────────────────────────
    "UNTRUSTED_FILE_PATH",
    // ── the generic class ──────────────────────────────────────────
    "SINK_CLEARANCE",
    // ── legacy corpus classes (same gate, historical names) ───────
    "HTML_INJECTION",
    "TAINT_PERSISTENCE",
    "SECRET_LEAK",
];

/// One element of the finite refusal lattice: a frozen reason word with
/// its canonical №392 severity rank. Constructed ONLY by [`Reason::parse`]
/// — a `Reason` cannot exist outside the frozen vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reason {
    word: &'static str,
    rank: usize,
}

impl Reason {
    /// Parse a reason word into a lattice element. `None` for any word
    /// outside the frozen vocabulary — the algebra is CLOSED over the
    /// №392 set (ADR-0204 §6: no new codes).
    pub fn parse(word: &str) -> Option<Reason> {
        DENY_REASONS
            .iter()
            .position(|w| *w == word)
            .map(|rank| Reason {
                word: DENY_REASONS[rank],
                rank,
            })
    }

    /// The frozen reason word (the stable refusal code of the gate
    /// refusals — the №392 check_id = the code).
    pub fn word(self) -> &'static str {
        self.word
    }

    /// The canonical severity rank (lower = more severe).
    pub fn rank(self) -> usize {
        self.rank
    }
}

/// `join` — the MORE SEVERE cause wins (the lower №392 rank): when two
/// refusals combine, the dominant class carries the verdict.
///
/// ```
/// use metalogos::refusal_algebra::{join, Reason};
/// let core = Reason::parse("IRREVERSIBLE_NO_GRANT").unwrap();
/// let legacy = Reason::parse("SECRET_LEAK").unwrap();
/// // the core class dominates the legacy one
/// assert_eq!(join(core, legacy).word(), "IRREVERSIBLE_NO_GRANT");
/// ```
pub fn join(a: Reason, b: Reason) -> Reason {
    if a.rank <= b.rank {
        a
    } else {
        b
    }
}

/// `meet` — the LESS SEVERE cause survives (the higher №392 rank): the
/// intersection of two refusal claims carries only what BOTH assert.
pub fn meet(a: Reason, b: Reason) -> Reason {
    if a.rank >= b.rank {
        a
    } else {
        b
    }
}

/// The refusal verdict — a VALUE, computable and portable (ADR-0204 §2):
/// `Allow` is the bottom element; a `Deny` carries the frozen stable
/// refusal code and the lattice cause. The typed `GRANT_*` detail of
/// grant refusals stays the producer's `human` payload (№390) — the
/// algebra carries `IRREVERSIBLE_NO_GRANT` as the cause and never
/// re-parses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny { code: String, cause: Reason },
}

impl Verdict {
    /// Build the deny verdict for a reason word. `None` for a word
    /// outside the frozen vocabulary (the closed-set rule of
    /// [`Reason::parse`]).
    pub fn deny(word: &str) -> Option<Verdict> {
        Reason::parse(word).map(|cause| Verdict::Deny {
            code: cause.word().to_string(),
            cause,
        })
    }

    /// True when the verdict refuses.
    pub fn is_deny(&self) -> bool {
        matches!(self, Verdict::Deny { .. })
    }

    /// The frozen stable refusal code of a `Deny`.
    pub fn code(&self) -> Option<&str> {
        match self {
            Verdict::Deny { code, .. } => Some(code),
            Verdict::Allow => None,
        }
    }

    /// The verdict lattice join: `Allow` is the bottom element, two
    /// `Deny`s combine through [`join`] over their causes.
    pub fn join(self, other: Verdict) -> Verdict {
        match (self, other) {
            (Verdict::Allow, x) => x,
            (x, Verdict::Allow) => x,
            (Verdict::Deny { cause: a, .. }, Verdict::Deny { cause: b, .. }) => {
                let c = crate::refusal_algebra::join(a, b);
                Verdict::Deny {
                    code: c.word().to_string(),
                    cause: c,
                }
            }
        }
    }

    /// The verdict lattice meet: `Allow` absorbs to `Allow`, two `Deny`s
    /// combine through [`meet`] over their causes.
    pub fn meet(self, other: Verdict) -> Verdict {
        match (self, other) {
            (Verdict::Allow, _) => Verdict::Allow,
            (_, Verdict::Allow) => Verdict::Allow,
            (Verdict::Deny { cause: a, .. }, Verdict::Deny { cause: b, .. }) => {
                let c = crate::refusal_algebra::meet(a, b);
                Verdict::Deny {
                    code: c.word().to_string(),
                    cause: c,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frozen_vocabulary_is_the_lattice_domain() {
        let mut ranks = std::collections::HashSet::new();
        for w in DENY_REASONS {
            let r =
                Reason::parse(w).unwrap_or_else(|| panic!("the vocabulary word must parse: {w}"));
            assert_eq!(r.word(), w);
            assert!(ranks.insert(r.rank()), "duplicate rank for {w}");
        }
        assert_eq!(ranks.len(), 14);
        assert!(Reason::parse("NOT_A_REASON").is_none());
        assert!(Reason::parse("").is_none());
        assert!(Reason::parse("secret_leak").is_none()); // case-sensitive words
    }

    #[test]
    fn the_chain_lattice_laws_hold_over_the_full_table() {
        let rs: Vec<Reason> = DENY_REASONS
            .iter()
            .map(|w| Reason::parse(w).unwrap())
            .collect();
        for a in &rs {
            for b in &rs {
                let j = join(*a, *b);
                let m = meet(*a, *b);
                assert_eq!(j, join(*b, *a), "join commutative");
                assert_eq!(m, meet(*b, *a), "meet commutative");
                assert!(j.rank() <= m.rank(), "join dominates meet");
                assert_eq!(join(*a, m), *a, "absorption (join)");
                assert_eq!(meet(*a, j), *a, "absorption (meet)");
                assert_eq!(join(*a, *a), *a, "idempotent join");
                assert_eq!(meet(*a, *a), *a, "idempotent meet");
            }
        }
        for a in &rs {
            for b in &rs {
                for c in &rs {
                    assert_eq!(join(join(*a, *b), *c), join(*a, join(*b, *c)));
                    assert_eq!(meet(meet(*a, *b), *c), meet(*a, meet(*b, *c)));
                }
            }
        }
        // The severity definition: the core class dominates the legacy one.
        let core = Reason::parse("IRREVERSIBLE_NO_GRANT").unwrap();
        let legacy = Reason::parse("SECRET_LEAK").unwrap();
        assert_eq!(join(core, legacy), core);
        assert_eq!(meet(core, legacy), legacy);
    }

    #[test]
    fn verdicts_carry_the_frozen_code_and_the_cause() {
        for w in DENY_REASONS {
            let v = Verdict::deny(w)
                .unwrap_or_else(|| panic!("the vocabulary word builds a verdict: {w}"));
            assert!(v.is_deny());
            assert_eq!(v.code(), Some(w));
            match v {
                Verdict::Deny { code, cause } => {
                    assert_eq!(code, w);
                    assert_eq!(cause.word(), w);
                }
                Verdict::Allow => panic!("deny(w) cannot be Allow"),
            }
        }
        assert!(Verdict::deny("NOT_A_REASON").is_none());
    }

    #[test]
    fn allow_is_the_bottom_element_of_the_verdict_lattice() {
        for w in DENY_REASONS {
            let d = Verdict::deny(w).unwrap();
            assert_eq!(Verdict::Allow.join(d.clone()), d);
            assert_eq!(d.clone().join(Verdict::Allow), d);
            assert_eq!(Verdict::Allow.meet(d.clone()), Verdict::Allow);
            assert_eq!(d.clone().meet(Verdict::Allow), Verdict::Allow);
        }
        let a = Verdict::deny("SINK_CLEARANCE").unwrap();
        let b = Verdict::deny("SECRET_LEAK").unwrap();
        let j = a.clone().join(b.clone());
        match &j {
            Verdict::Deny { code, cause } => {
                assert_eq!(code, "SINK_CLEARANCE");
                assert_eq!(cause.word(), "SINK_CLEARANCE");
            }
            Verdict::Allow => panic!("two denies join to a deny"),
        }
        assert_eq!(a.clone().meet(b.clone()).code(), Some("SECRET_LEAK"));
        assert_eq!(b.clone().meet(a).code(), Some("SECRET_LEAK"));
    }
}
