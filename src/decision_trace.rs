// ── Naryad №892 (wave 80 — W-I, 8G/C8, ADR-0210): DecisionTrace ────────
//
// The deterministic decision trace: the typed, append-only, verifiable
// artifact of ONE irreversible decision. ADR-0210 §2: the trace carries
// WHAT the decision consumed and produced — the input digests, the
// model/provider version line (the ProviderContract №879 form), the
// grant references (ADR-0155), the refusal classes (the frozen №385
// vocabulary), the degradation markers (Degraded(t), backends.rs), the
// output digests — never a raw prompt (the privacy boundary: digests,
// not payloads; ADR-0210 §6).
//
// REPLAY_MATCH semantics live HERE on the type (ADR-0210 §3, the frame
// R38(5)): `verify_replay` is a RECONCILIATION of a claimed trace
// against the recorded rows — a comparison of inputs/versions/grants/
// refusals/outputs, NOT a bit-for-bit reproduction of the execution
// (the bit-for-bit replay is struck as unfulfillable — honest-limits
// №588). A trace that reconciles MATCHES; any divergence refuses with
// an EXISTING frozen class (№385 does not grow).
//
// The trace rides the EXISTING Ledger ACTION contour (ADR-0167 §3.4):
// `record_trace` writes through the live `ledger::record` API — the
// journal of the trace is a Ledger action row, not a parallel store;
// every existing Ledger call is untouched (the 1:1 migration, the
// zero-surface-drift rule).
//
// The structural form is the wave-79 kernel (№886): the fail-closed
// constructor, the canonical serialization + the G5 row digest (№828),
// the append-only log with the exact-duplicate refusal, supersede as a
// NEW row — never a silent overwrite.

use crate::refusal_algebra::Reason;
use sha2::{Digest, Sha256};

/// The field separator of the canonical serialization (the №828/№868
/// unit-separator form — the byte cannot appear inside the fields).
const SEP: char = '\u{1f}';

/// The schema version of the trace canonicalization: a field-order or
/// field-set change bumps this and breaks NOTHING silently — the
/// digests of old rows simply stop re-deriving (tamper-evident).
pub const DECISION_TRACE_SCHEMA_VERSION: u32 = 1;

/// The carrier a decision ran on (ADR-0210 §2.1). `Language` marks the
/// LLM carrier — the №879 version line is MANDATORY for it (a decision
/// that consumed a model without naming the model/provider versions is
/// not a trace); `Local` marks the pure runtime — the version line is
/// optional there (no model consumed, nothing to name honestly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DecisionCarrier {
    Language,
    Local,
}

impl DecisionCarrier {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionCarrier::Language => "language",
            DecisionCarrier::Local => "local",
        }
    }
}

/// `true` when `s` is a 64-char lowercase SHA-256 hex digest — the ONLY
/// payload shape the trace accepts for inputs/outputs (the privacy
/// boundary is enforced at the construction site, not at the audit).
fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The typed decision trace (ADR-0210 §2). Field order below IS the
/// canonical serialization order — do not reorder without a
/// schema-version bump.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecisionTrace {
    /// The irreversible decision the trace covers — a name from the
    /// EXISTING action space (the Ledger `action` names, e.g.
    /// `invoke_db_execute_with_grant`).
    pub decision: String,
    pub carrier: DecisionCarrier,
    /// The input digests (SHA-256 hex) — WHAT the decision consumed as
    /// digests, never the raw payloads.
    pub inputs: Vec<String>,
    /// The №879 contract line: the roster name + the model/provider
    /// versions the decision consumed. Mandatory on the Language
    /// carrier, empty on the Local one.
    pub contract_name: String,
    pub model_version: String,
    pub provider_version: String,
    /// The grant references the decision ran under (ADR-0155 ids).
    pub grants: Vec<String>,
    /// The refusal classes recorded DURING the decision — every word
    /// MUST parse in the frozen №385 vocabulary (the set does not grow).
    pub refusals: Vec<String>,
    /// The degradation markers recorded during the decision (the
    /// Degraded(t) subject markers of backends.rs — typed, loud,
    /// inspectable).
    pub degradations: Vec<String>,
    /// The output digests (SHA-256 hex) — WHAT the decision produced,
    /// digests again.
    pub outputs: Vec<String>,
    /// The trace timestamp (epoch seconds — the Ledger `ts` form).
    pub ts: u64,
    /// The row digest (the G5 №828 form): SHA-256 over the canonical
    /// serialization of every field above. A row without a digest is
    /// not a row of the log.
    pub trace_digest: String,
}

impl DecisionTrace {
    /// Construct + digest-pin one trace (the fail-closed constructor:
    /// an empty decision name, a trace without inputs, a Language
    /// trace without the №879 version line, a raw payload where a
    /// digest belongs, or an out-of-vocabulary refusal word refuses
    /// HERE — the KPI C8 is enforced at the construction site, not at
    /// the audit).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decision: &str,
        carrier: DecisionCarrier,
        inputs: &[String],
        contract_name: &str,
        model_version: &str,
        provider_version: &str,
        grants: &[String],
        refusals: &[String],
        degradations: &[String],
        outputs: &[String],
        ts: u64,
    ) -> Result<Self, String> {
        if decision.trim().is_empty() {
            return Err(
                "decision trace: the decision name is empty — a trace names its decision"
                    .to_string(),
            );
        }
        if inputs.is_empty() {
            return Err(format!(
                "decision trace: '{decision}' has no input digests — a trace without consumed inputs is not a trace (fail-closed, ADR-0210)"
            ));
        }
        for d in inputs {
            if !is_sha256_hex(d) {
                return Err(format!(
                    "decision trace: '{decision}' carries an input that is not a SHA-256 digest — raw payloads never enter the trace (the privacy boundary, ADR-0210 §6)"
                ));
            }
        }
        for d in outputs {
            if !is_sha256_hex(d) {
                return Err(format!(
                    "decision trace: '{decision}' carries an output that is not a SHA-256 digest — raw payloads never enter the trace (the privacy boundary, ADR-0210 §6)"
                ));
            }
        }
        if carrier == DecisionCarrier::Language {
            if contract_name.trim().is_empty() {
                return Err(format!(
                    "decision trace: '{decision}' runs on the language carrier without a contract name — the №879 version line is mandatory on the LLM carrier (fail-closed)"
                ));
            }
            if model_version.trim().is_empty() || provider_version.trim().is_empty() {
                return Err(format!(
                    "decision trace: '{decision}' runs on the language carrier without the model/provider versions — a decision that consumed a model names the versions it consumed (№879, fail-closed)"
                ));
            }
        }
        for g in grants {
            if g.trim().is_empty() {
                return Err(format!(
                    "decision trace: '{decision}' carries an empty grant reference — a grant line names the grant (ADR-0155)"
                ));
            }
        }
        let mut refusals_sorted: Vec<String> = refusals.to_vec();
        refusals_sorted.sort();
        refusals_sorted.dedup();
        for w in &refusals_sorted {
            if Reason::parse(w).is_none() {
                return Err(format!(
                    "decision trace: '{decision}' records refusal '{w}' outside the frozen vocabulary — the №385 set does not grow (fail-closed)"
                ));
            }
        }
        for m in degradations {
            if m.trim().is_empty() {
                return Err(format!(
                    "decision trace: '{decision}' carries an empty degradation marker — a Degraded(t) record names its subject"
                ));
            }
        }
        let mut trace = DecisionTrace {
            decision: decision.to_string(),
            carrier,
            inputs: inputs.to_vec(),
            contract_name: contract_name.to_string(),
            model_version: model_version.to_string(),
            provider_version: provider_version.to_string(),
            grants: grants.to_vec(),
            refusals: refusals_sorted,
            degradations: degradations.to_vec(),
            outputs: outputs.to_vec(),
            ts,
            trace_digest: String::new(),
        };
        trace.trace_digest = canonical_trace_digest(&trace);
        Ok(trace)
    }
}

/// The canonical trace serialization (the №828/№868 form: the fixed
/// field order, the `\x1f` separator, pure and deterministic). Refusal
/// words serialize in their stored (sorted, deduped) order.
pub fn canonical_trace(t: &DecisionTrace) -> String {
    [
        "v1",
        &t.decision,
        t.carrier.as_str(),
        &t.inputs.join(","),
        &t.contract_name,
        &t.model_version,
        &t.provider_version,
        &t.grants.join(","),
        &t.refusals.join(","),
        &t.degradations.join(","),
        &t.outputs.join(","),
        &t.ts.to_string(),
    ]
    .join(&SEP.to_string())
}

/// The row digest: SHA-256 over `canonical_trace`, hex-encoded (the G5
/// form of №828).
pub fn canonical_trace_digest(t: &DecisionTrace) -> String {
    let digest = Sha256::digest(canonical_trace(t).as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

/// The append-only decision-trace log (ADR-0210 §2.2, the
/// AttestationMap/JurisdictionMatrix discipline): appending adds,
/// nothing deletes; the rows stay in append order; a re-traced decision
/// is a NEW row that supersedes — never a silent overwrite.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecisionTraceLog {
    rows: Vec<DecisionTrace>,
}

impl DecisionTraceLog {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append one trace. An EXACT duplicate (the same trace digest)
    /// refuses: writing the same decision twice unchanged is a
    /// construction error, not a row. A re-traced decision is a NEW row
    /// — the verify reconciles against the latest row of the decision.
    pub fn append(&mut self, trace: DecisionTrace) -> Result<usize, String> {
        if self
            .rows
            .iter()
            .any(|r| r.trace_digest == trace.trace_digest)
        {
            return Err(format!(
                "decision trace log: the exact trace {} is already in the log (append-only, no duplicates)",
                trace.trace_digest
            ));
        }
        self.rows.push(trace);
        Ok(self.rows.len() - 1)
    }

    /// The read-only row view (the verifier's input).
    pub fn rows(&self) -> &[DecisionTrace] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The rows covering ONE decision name (append order).
    pub fn rows_for(&self, decision: &str) -> Vec<&DecisionTrace> {
        self.rows
            .iter()
            .filter(|r| r.decision == decision)
            .collect()
    }

    /// The log digest: SHA-256 over the row digests in append order
    /// (append-only stability — the digest changes only when the log
    /// changes).
    pub fn digest(&self) -> String {
        let joined = self
            .rows
            .iter()
            .map(|r| r.trace_digest.as_str())
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

/// The one field a reconciliation diverged on (the REPLAY_MATCH
/// report — the human-readable field name rides the refusal string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayField {
    Decision,
    Carrier,
    Inputs,
    ContractLine,
    Grants,
    Refusals,
    Degradations,
    Outputs,
}

impl ReplayField {
    pub fn as_str(self) -> &'static str {
        match self {
            ReplayField::Decision => "decision",
            ReplayField::Carrier => "carrier",
            ReplayField::Inputs => "inputs",
            ReplayField::ContractLine => "contract-line",
            ReplayField::Grants => "grants",
            ReplayField::Refusals => "refusals",
            ReplayField::Degradations => "degradations",
            ReplayField::Outputs => "outputs",
        }
    }
}

/// The REPLAY_MATCH reconciliation of a claimed trace against the log
/// (ADR-0210 §3 — СВЕРКА, not reproduction):
///
/// 0. the CLAIM must re-derive its own digest from its own fields — a
///    claim whose digest names a state its fields do not carry is a
///    forgery (UNTRUSTED_EXEC_DECISION, №385 does not grow);
/// 1. the exact trace digest exists in the log AND the recorded row
///    re-derives its own digest from its fields — the replay MATCHES;
/// 2. no digest match but the decision name is recorded — the FIRST
///    diverging field refuses with the frozen
///    `UNTRUSTED_EXEC_DECISION` class (a claimed replay that disagrees
///    with the record is an untrusted execution decision);
/// 3. the decision name is not recorded at all — the fail-closed
///    refusal (no recorded trace covers the decision; the coverage
///    composition №893 turns THIS refusal into the pre-effect deny —
///    the №845 form).
///
/// On match the recorded row is returned (the verifier's evidence).
pub fn verify_replay<'a>(
    trace: &DecisionTrace,
    log: &'a DecisionTraceLog,
) -> Result<&'a DecisionTrace, String> {
    // Step 0 — the claim is internally consistent: its stored digest
    // re-derives from its carried fields. A claim edited after the
    // write (the swapped output keeping the old digest) refuses HERE,
    // before any lookup.
    if canonical_trace_digest(trace) != trace.trace_digest {
        return Err(format!(
            "REPLAY_MATCH: the claimed trace of '{0}' does not re-derive its digest — the claim was altered after the write (UNTRUSTED_EXEC_DECISION, fail-closed)",
            trace.decision
        ));
    }
    // Defense in depth: a recorded row must re-derive its own digest —
    // an internally tampered row (fields edited without the digest
    // recompute) refuses before any reconciliation.
    for row in log.rows() {
        if canonical_trace_digest(row) != row.trace_digest {
            return Err(format!(
                "REPLAY_MATCH: the recorded trace {} does not re-derive its digest — the record was altered after the write (UNTRUSTED_EXEC_DECISION, fail-closed)",
                row.trace_digest
            ));
        }
    }
    if let Some(row) = log
        .rows()
        .iter()
        .find(|r| r.trace_digest == trace.trace_digest)
    {
        return Ok(row);
    }
    let same_decision: Vec<&DecisionTrace> = log.rows_for(&trace.decision);
    if same_decision.is_empty() {
        return Err(format!(
            "REPLAY_MATCH: no recorded trace covers decision '{0}' — the fail-closed coverage refuses a decision without a trace (SINK_CLEARANCE; the №845 form; ADR-0210)",
            trace.decision
        ));
    }
    // The claimed replay names a decision the log knows but does not
    // match — reconcile field-by-field against the LATEST row of the
    // decision and name the first diverging field.
    let recorded = same_decision[same_decision.len() - 1];
    let field = if recorded.carrier != trace.carrier {
        Some(ReplayField::Carrier)
    } else if recorded.inputs != trace.inputs {
        Some(ReplayField::Inputs)
    } else if recorded.contract_name != trace.contract_name
        || recorded.model_version != trace.model_version
        || recorded.provider_version != trace.provider_version
    {
        Some(ReplayField::ContractLine)
    } else if recorded.grants != trace.grants {
        Some(ReplayField::Grants)
    } else if recorded.refusals != trace.refusals {
        Some(ReplayField::Refusals)
    } else if recorded.degradations != trace.degradations {
        Some(ReplayField::Degradations)
    } else if recorded.outputs != trace.outputs {
        Some(ReplayField::Outputs)
    } else {
        None
    };
    match field {
        Some(f) => Err(format!(
            "REPLAY_MATCH: the claimed replay of '{0}' diverges from the recorded trace on {1} — the record and the claim disagree (UNTRUSTED_EXEC_DECISION, fail-closed)",
            trace.decision,
            f.as_str()
        )),
        None => Err(format!(
            "REPLAY_MATCH: the claimed replay of '{0}' carries an unrecorded trace digest — the record was superseded or the claim is foreign (UNTRUSTED_EXEC_DECISION, fail-closed)",
            trace.decision
        )),
    }
}

/// The Ledger consumption (ADR-0210 §2.3): the trace append writes ONE
/// action row through the live `ledger::record` API — the journal of
/// the decision is a Ledger row keyed by the trace digest, never a
/// parallel store. Best-effort by the Ledger posture (ADR-0167 §2
/// driver 5): a write failure is loud on stderr and counted, it never
/// flips the decision's outcome.
pub fn record_trace(actor: &str, scope: &str, trace: &DecisionTrace) {
    crate::ledger::record("decision_trace", actor, scope, &trace.trace_digest);
}

#[cfg(test)]
mod kernel_tests {
    //! The negative controls are part of the CONTRACT (ADR-0210 §4,
    //! the №867/№886 form): the refusals run at construction/verify
    //! time, not only in the integration suite.

    use super::*;

    const D: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const E: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn trace(decision: &str, outputs: &[String]) -> DecisionTrace {
        DecisionTrace::new(
            decision,
            DecisionCarrier::Language,
            &[D.to_string()],
            "local",
            "mock-1",
            "provider-1",
            &[],
            &[],
            &[],
            outputs,
            1_700_000_000,
        )
        .expect("valid trace")
    }

    #[test]
    fn constructor_refuses_a_trace_without_inputs() {
        let err = DecisionTrace::new(
            "invoke_db_execute_with_grant",
            DecisionCarrier::Local,
            &[],
            "",
            "",
            "",
            &[],
            &[],
            &[],
            &[E.to_string()],
            1,
        )
        .expect_err("no inputs — no trace");
        assert!(err.contains("no input digests"), "{err}");
        assert!(err.contains("fail-closed"), "{err}");
    }

    #[test]
    fn constructor_refuses_the_language_carrier_without_the_version_line() {
        let err = DecisionTrace::new(
            "llm_call",
            DecisionCarrier::Language,
            &[D.to_string()],
            "local",
            "",
            "",
            &[],
            &[],
            &[],
            &[E.to_string()],
            1,
        )
        .expect_err("no versions on the LLM carrier");
        assert!(err.contains("model/provider versions"), "{err}");
        assert!(err.contains("№879"), "{err}");
    }

    #[test]
    fn constructor_refuses_raw_payloads_and_foreign_refusal_words() {
        let err = DecisionTrace::new(
            "x",
            DecisionCarrier::Local,
            &["raw prompt text".to_string()],
            "",
            "",
            "",
            &[],
            &[],
            &[],
            &[],
            1,
        )
        .expect_err("a raw payload is not a digest");
        assert!(err.contains("never enter the trace"), "{err}");
        let err = DecisionTrace::new(
            "x",
            DecisionCarrier::Local,
            &[D.to_string()],
            "",
            "",
            "",
            &[],
            &["BRAND_NEW_REFUSAL_CLASS".to_string()],
            &[],
            &[],
            1,
        )
        .expect_err("the frozen vocabulary does not grow");
        assert!(err.contains("does not grow"), "{err}");
    }

    #[test]
    fn digest_is_stable_over_fields_and_sensitive_to_every_group() {
        let a = trace("decide", &[E.to_string()]);
        let b = trace("decide", &[E.to_string()]);
        assert_eq!(a.trace_digest, b.trace_digest, "same fields — same digest");
        let c = trace("decide", &[D.to_string()]);
        assert_ne!(a.trace_digest, c.trace_digest, "outputs participate");
        let mut d = trace("decide", &[E.to_string()]);
        d.grants = vec!["grant-1".to_string()];
        d.trace_digest = canonical_trace_digest(&d);
        assert_ne!(a.trace_digest, d.trace_digest, "grants participate");
        assert_eq!(
            canonical_trace(&a),
            canonical_trace(&b),
            "canonical determinism"
        );
    }

    #[test]
    fn log_is_append_only_and_refuses_the_exact_duplicate() {
        let mut log = DecisionTraceLog::new();
        let t1 = trace("decide", &[E.to_string()]);
        assert_eq!(log.append(t1.clone()).expect("first append"), 0);
        let err = log
            .append(t1.clone())
            .expect_err("the exact duplicate refuses");
        assert!(err.contains("append-only, no duplicates"), "{err}");
        // A re-traced decision (a changed output) is a NEW row — the
        // supersede form, never a silent overwrite.
        let t2 = trace("decide", &[D.to_string()]);
        assert_eq!(log.append(t2).expect("supersede append"), 1);
        assert_eq!(log.len(), 2);
        assert_eq!(log.rows_for("decide").len(), 2);
    }

    #[test]
    fn verify_matches_the_recorded_trace_and_returns_the_row() {
        let mut log = DecisionTraceLog::new();
        let t = trace("decide", &[E.to_string()]);
        log.append(t.clone()).expect("append");
        let row = verify_replay(&t, &log).expect("the exact trace matches");
        assert_eq!(row.trace_digest, t.trace_digest);
        // Verify is a pure reconciliation — the second run is byte-identical.
        let again = verify_replay(&t, &log).expect("the second run matches");
        assert_eq!(row.trace_digest, again.trace_digest);
    }

    #[test]
    fn verify_refuses_a_decision_without_any_recorded_trace_fail_closed() {
        let log = DecisionTraceLog::new();
        let t = trace("untraced_decision", &[E.to_string()]);
        let err = verify_replay(&t, &log).expect_err("no recorded trace");
        assert!(err.contains("no recorded trace covers"), "{err}");
        assert!(err.contains("SINK_CLEARANCE"), "{err}");
        assert!(err.contains("№845"), "{err}");
    }

    #[test]
    fn verify_catches_the_forged_claim_that_keeps_the_old_digest() {
        let mut log = DecisionTraceLog::new();
        let t = trace("decide", &[E.to_string()]);
        log.append(t.clone()).expect("append");
        let mut tampered = trace("decide", &[D.to_string()]);
        // The tamper keeps the ORIGINAL stored digest so the claim
        // presents itself as the recorded row — the claim-side digest
        // re-derivation must catch the forgery before any lookup.
        tampered.trace_digest = t.trace_digest.clone();
        let err = verify_replay(&tampered, &log).expect_err("the forged claim refuses");
        assert!(err.contains("does not re-derive its digest"), "{err}");
        assert!(err.contains("UNTRUSTED_EXEC_DECISION"), "{err}");
        // The honest re-trace of the same decision (the recomputed
        // digest) takes the OTHER negative path — the field
        // reconciliation names the diverging field.
        let honest_retrace = trace("decide", &[D.to_string()]);
        let err = verify_replay(&honest_retrace, &log).expect_err("the diverging re-trace");
        assert!(
            err.contains("diverges from the recorded trace on outputs"),
            "{err}"
        );
    }

    #[test]
    fn verify_catches_a_tampered_recorded_row_via_digest_rederivation() {
        let mut log = DecisionTraceLog::new();
        let t = trace("decide", &[E.to_string()]);
        log.append(t).expect("append");
        // Edit the RECORDED row's fields without recomputing its
        // digest — the log itself is now internally inconsistent and
        // the verifier must refuse BEFORE any reconciliation.
        log.rows[0].outputs = vec![D.to_string()];
        let claim = trace("decide", &[E.to_string()]);
        let err = verify_replay(&claim, &log).expect_err("the altered record refuses");
        assert!(err.contains("does not re-derive its digest"), "{err}");
        assert!(err.contains("UNTRUSTED_EXEC_DECISION"), "{err}");
    }

    #[test]
    fn serde_double_run_is_byte_in_byte() {
        let t = trace("decide", &[E.to_string()]);
        let s1 = serde_json::to_string(&t).expect("serialize 1");
        let back: DecisionTrace = serde_json::from_str(&s1).expect("deserialize");
        let s2 = serde_json::to_string(&back).expect("serialize 2");
        assert_eq!(s1, s2, "the serde double run is byte-in-byte");
        let mut log = DecisionTraceLog::new();
        log.append(t).expect("append");
        let l1 = serde_json::to_string(&log).expect("log serialize 1");
        let lback: DecisionTraceLog = serde_json::from_str(&l1).expect("log deserialize");
        let l2 = serde_json::to_string(&lback).expect("log serialize 2");
        assert_eq!(l1, l2, "the log serde double run is byte-in-byte");
        assert_eq!(
            log.digest(),
            lback.digest(),
            "the digest survives the round trip"
        );
    }
}
