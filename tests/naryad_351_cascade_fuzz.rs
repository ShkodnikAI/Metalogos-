// ── Naryad #351: the cascade differential fuzz (BLOCKING tier) ────────
//
// The ADR-0173 §3.7 Tier-1 fuzzer, the grant_algebra_fuzz pattern: a
// deterministic xorshift PRNG drives random DAGs, random pins and
// random forget roots; the REAL `plan_cascade` output must equal an
// INDEPENDENTLY derived model (re-derived here from the ADR invariants,
// NOT shared with src/memory_typed.rs) on:
//   P1 provenance integrity — after the modeled forget, every
//      surviving entry's derived_from parents all survive;
//   P2 completeness — the delete set IS the full descendant closure;
//   P3 isolation — nothing outside the closure changes;
//   Veto — a retained node inside the closure blocks the whole forget.
// The O(производных) claim is pinned by the EXACT visited counters:
// one pop per closure node, one edge scan per closure-internal edge.
//
// No wall-clock anywhere — determinism is the point (the cargo-fuzz
// Tier-2 target runs the same pure function in fuzz-smoke CI).

use metalogos::memory_typed::plan_cascade;
use std::collections::{HashMap, HashSet};

// ── Deterministic PRNG (xorshift64*, the grant_algebra_fuzz shape) ────

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

// ── The independent model (re-derived from the ADR, NOT shared code) ──

/// The descendant closure of `root` by transitive parent→child expansion.
/// Written as a FIXPOINT loop (not the implementation's BFS) so a bug in
/// the queue discipline cannot hide in both.
fn model_closure(children: &HashMap<&str, Vec<&str>>, root: &str) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();
    set.insert(root.to_string());
    loop {
        let mut grew = false;
        for (parent, kids) in children {
            if set.contains(*parent) {
                for k in kids {
                    if set.insert((*k).to_string()) {
                        grew = true;
                    }
                }
            }
        }
        if !grew {
            break;
        }
    }
    set
}

fn model_plan(
    children: &HashMap<&str, Vec<&str>>,
    retained: &HashSet<&str>,
    root: &str,
) -> (Vec<String>, Vec<String>) {
    let closure = model_closure(children, root);
    let blocked: Vec<String> = retained
        .iter()
        .filter(|r| closure.contains(**r))
        .map(|r| r.to_string())
        .collect();
    let deleted: Vec<String> = if blocked.is_empty() {
        let mut v: Vec<String> = closure.into_iter().collect();
        v.sort();
        v
    } else {
        Vec::new()
    };
    (deleted, blocked)
}

// ── The fuzz driver ────────────────────────────────────────────────────

#[test]
fn cascade_plan_matches_the_independent_model_over_random_dags() {
    // DAG by construction: node i may only derive from nodes j < i.
    // Every reachable edge therefore points "backwards" — cycles are
    // impossible, exactly like the language surface (put validates
    // parents BEFORE the child exists).
    for seed in 1u64..=512 {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let n = 1 + rng.below(48) as usize;
        let names: Vec<String> = (0..n).map(|i| format!("n{i}")).collect();
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut edge_count = 0usize;
        for i in 1..n {
            // Each node derives from 0..=3 earlier nodes.
            let k = rng.below(4) as usize;
            let mut parents: Vec<usize> = Vec::new();
            for _ in 0..k {
                let p = rng.below(i as u64) as usize;
                if !parents.contains(&p) {
                    parents.push(p);
                }
            }
            for p in parents {
                children
                    .entry(names[p].as_str())
                    .or_default()
                    .push(names[i].as_str());
                edge_count += 1;
            }
        }
        // Random pins: ~1/4 of the nodes retained.
        let retained: HashSet<&str> = names
            .iter()
            .map(|nm| nm.as_str())
            .filter(|_| rng.below(4) == 0)
            .collect();
        let retained_owned: HashSet<String> = retained.iter().map(|s| s.to_string()).collect();

        // The REAL plan on a random root; the model's verdict must match.
        let root = names[rng.below(n as u64) as usize].as_str();
        let real = plan_cascade(
            root,
            &children
                .iter()
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        v.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                    )
                })
                .collect::<HashMap<String, Vec<String>>>(),
            &retained_owned,
        );
        let (model_deleted, model_blocked_unsorted) = model_plan(&children, &retained, root);
        let mut model_blocked = model_blocked_unsorted;
        model_blocked.sort();

        let mut real_blocked = real.blocked_by.clone();
        real_blocked.sort();
        assert_eq!(
            real_blocked, model_blocked,
            "seed {seed}: the veto set diverged from the model"
        );
        if model_blocked.is_empty() {
            assert_eq!(
                real.closure, model_deleted,
                "seed {seed}: the delete set diverged from the model closure"
            );
        } else {
            assert!(
                real.closure.is_empty() || !real.blocked_by.is_empty(),
                "seed {seed}: a blocked plan must never be applied as-is"
            );
        }

        // The model invariants applied to the REAL delete set:
        if real.blocked_by.is_empty() {
            // P1: every survivor's derived-from parents all survive.
            // Edge direction: children[p] contains k ⇔ k derives from p,
            // so the survivor is the KID and the required-alive node is
            // the parent.
            let deleted: HashSet<&str> = real.closure.iter().map(|s| s.as_str()).collect();
            for (parent, kids) in &children {
                for k in kids {
                    if !deleted.contains(k) {
                        assert!(
                            !deleted.contains(parent),
                            "seed {seed}: P1 dangled {k} (parent {parent} deleted)"
                        );
                    }
                }
            }
            // P3: nothing outside the closure was named.
            for d in &real.closure {
                assert!(
                    model_closure(&children, root).contains(d.as_str()),
                    "seed {seed}: P3 violated — {d} is outside the closure"
                );
            }
        }

        // O() counters: EXACTLY one pop per closure node and one scan
        // per closure-internal edge (the ADR §3.2 complexity, pinned).
        assert_eq!(
            real.visited_nodes,
            real.closure.len(),
            "seed {seed}: node pops must equal the closure size"
        );
        let closure_set: HashSet<&str> = real.closure.iter().map(|s| s.as_str()).collect();
        let internal_edges: usize = children
            .iter()
            .filter(|(p, _)| closure_set.contains(**p))
            .map(|(_, kids)| kids.iter().filter(|k| closure_set.contains(**k)).count())
            .sum();
        assert_eq!(
            real.visited_edges, internal_edges,
            "seed {seed}: edge scans must equal the closure-internal edges (total {edge_count})"
        );
    }
}

// ── The O(производных) scaling proof (deterministic, no wall-clock) ────

#[test]
fn cascade_cost_is_linear_in_the_closure_by_exact_counters() {
    // A chain of N nodes: the worst-case BFS depth. The cost counters
    // must be EXACTLY N pops + (N-1) scans — i.e., cost = O(closure),
    // independent of the container's total capacity the index could
    // have grown to.
    for n in [1000usize, 4000, 16000] {
        let names: Vec<String> = (0..n).map(|i| format!("c{i}")).collect();
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        for i in 0..n - 1 {
            children.insert(names[i].clone(), vec![names[i + 1].clone()]);
        }
        let plan = plan_cascade("c0", &children, &HashSet::new());
        assert_eq!(plan.closure.len(), n, "the whole chain is the closure");
        assert_eq!(plan.visited_nodes, n, "one pop per node");
        assert_eq!(plan.visited_edges, n - 1, "one scan per edge");
        assert!(plan.blocked_by.is_empty());
    }
    // Doubling the closure exactly doubles the node pops; the edges
    // follow the chain law (a chain of n has n-1 edges, so doubling n
    // gives 2·(n-1)+1). The per-size EXACT counters above already pin
    // the linear law — these relations are the cross-size sanity.
    let build = |n: usize| {
        let names: Vec<String> = (0..n).map(|i| format!("s{i}")).collect();
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        for i in 0..n - 1 {
            children.insert(names[i].clone(), vec![names[i + 1].clone()]);
        }
        children
    };
    let half = plan_cascade("s0", &build(5000), &HashSet::new());
    let full = plan_cascade("s0", &build(10000), &HashSet::new());
    assert_eq!(full.visited_nodes, half.visited_nodes * 2);
    assert_eq!(full.visited_edges, half.visited_edges * 2 + 1);
}
