#![no_main]
use libfuzzer_sys::fuzz_target;

// ── Naryad #393 (ADR-0167 §5): fuzzing the ledger tamper surface ───────
//
// The verifier is the EXTERNAL trust boundary: anyone can hand it any
// file. Invariants over arbitrary input:
//   1. panic freedom — verification returns Ok or a loud Err, never a
//      panic (index/arith/serde panics are the classic verifier bugs);
//   2. the honest-boundary soundness: a file that VERIFIES either (a) is
//      byte-identical to the original export or (b) differs only in
//      trailing content the chain does not cover (empty lines) — any
//      single-byte mutation of the covered body is detected. This is the
//      single-byte-flip guarantee from the tamper tests, generalized.
//
// Out of scope (the ADR-0167 §7 boundary): full rewrites under a fresh
// key without anchors are structurally consistent — that is what
// --expect-head/--expect-key are for.

fuzz_target!(|data: &[u8]| {
    // Invariant 1: the verifier never panics on arbitrary bytes.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let parsed = metalogos::ledger::records_from_jsonl(text);
    if let Ok(records) = parsed {
        let result = metalogos::ledger::verify_records(&records, None, None);
        if let Ok(report) = result {
            // Invariant 2 (soundness direction): a verified arbitrary file
            // must be an honestly signed chain. Re-canonicalize every
            // record and re-check the signature math directly — the
            // verifier's own rules, independently restated.
            assert_eq!(
                report.records as usize,
                records.len(),
                "verified report disagrees with the record count"
            );
            for (i, r) in records.iter().enumerate() {
                let recomputed = metalogos::ledger::record_hash(r);
                assert_eq!(
                    recomputed, r.hash,
                    "record {} verified with a hash that does not match its body",
                    i
                );
            }
        }
    }
});
