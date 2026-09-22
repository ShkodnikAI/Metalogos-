#![no_main]
use libfuzzer_sys::fuzz_target;

// ── Naryad #351 (ADR-0173 §3.7): the cascade plan fuzz (Tier 2, the
//    fuzz-smoke non-blocking job; the BLOCKING Tier-1 differential model
//    lives in tests/naryad_351_cascade_fuzz.rs) ──────────────────────────
//
// The pure plan function (plan_cascade) is the trust boundary of the
// forget feature: whatever the graph and the pin set look like, the
// plan must stay total and honest. Invariants over arbitrary bytes:
//   1. panic freedom — building the graph/pins from the input and
//      planning a cascade never panics (index/arith/unwrap bugs are
//      the classic plan-function defects);
//   2. closure soundness — the delete set is a FIXPOINT: it contains
//      the root, and EVERY child of a deleted node is either deleted
//      or was pinned (the veto path); no deleted node may be missing
//      from the model closure;
//   3. determinism — the same input yields the byte-identical plan
//      (sorted outputs; the forget is ledger-recorded, so two plans
//      for one request would be an audit lie).

use metalogos::memory_typed::plan_cascade;
use std::collections::{HashMap, HashSet};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    // Derive the DAG shape from the bytes: node i may derive from
    // j < i (the put-validate-parents-first DAG, cycles impossible).
    let n = (data[0] as usize % 48) + 1;
    let names: Vec<String> = (0..n).map(|i| format!("n{i}")).collect();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut byte_idx = 1usize;
    for i in 1..n {
        let k = (data[byte_idx % data.len()] as usize) % 4;
        for _ in 0..k {
            let p = (data[(byte_idx + i) % data.len()] as usize) % i;
            children
                .entry(names[p].clone())
                .or_default()
                .push(names[i].clone());
        }
        byte_idx = byte_idx.wrapping_add(1);
    }
    // Pins from the tail bytes (~1/4 of nodes).
    let retained: HashSet<String> = names
        .iter()
        .enumerate()
        .filter(|(i, _)| data[(i + byte_idx) % data.len()] % 4 == 0)
        .map(|(_, nm)| nm.clone())
        .collect();

    // Plan from EVERY node as the root: each must be total.
    for root in &names {
        let plan = plan_cascade(root, &children, &retained);
        // Invariant 2 (soundness): the delete set contains the root and
        // is a fixpoint — every child of a deleted node is deleted too
        // (when the plan is unblocked, the closure is complete).
        assert!(
            plan.closure.iter().any(|k| k == root),
            "soundness: the closure must contain the root {root}"
        );
        let set: HashSet<&String> = plan.closure.iter().collect();
        assert_eq!(set.len(), plan.closure.len(), "no duplicates in the closure");
        for node in &plan.closure {
            if let Some(kids) = children.get(node) {
                for k in kids {
                    if plan.blocked_by.is_empty() {
                        assert!(
                            set.contains(k),
                            "soundness: {k} is a child of deleted {node} but not deleted"
                        );
                    }
                }
            }
        }
        // The blocked set must be the retained pins inside the closure.
        for b in &plan.blocked_by {
            assert!(retained.contains(b), "blocked_by {b} is not a pin");
            assert!(set.contains(b), "blocked_by {b} is outside the closure");
        }
        // Invariant 3 (determinism): same input → identical plan.
        let again = plan_cascade(root, &children, &retained);
        assert_eq!(plan, again, "the plan must be deterministic");
    }
});
