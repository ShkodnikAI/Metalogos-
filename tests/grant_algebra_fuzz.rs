// ── Naryad #390: grant_algebra_fuzz — no amplification by construction ──
//
// DoD (б) of issue #484: fuzzing finds NO amplification — no combination
// of issue/subgrant/use/revoke may ever yield more usable power than was
// issued (ADR-0155 §3.3 rules 1/4/5).
//
// Method: differential fuzzing against an independent model of the algebra
// (re-derived here from the ADR rules, NOT shared with src/grants.rs). A
// deterministic xorshift PRNG drives op sequences over one shared ledger;
// every op's REAL outcome must match the model's prediction. The model
// tracks scope, class, remaining, lifecycle state AND the expiry horizon —
// the real ledger refuses a subgrant whose TTL would outlive its parent,
// so the model must too. Model keys are the REAL ledger ids, so every
// model op mirrors exactly the op the real ledger just performed.
//
// All fuzz TTLs are long enough that wall-clock expiry never fires mid-run
// (the expiry contract has dedicated unit tests in naryad_390_grants.rs).

use metalogos::grants::{self, GrantClass, GrantHandle};

// ── Deterministic PRNG (xorshift64*) ───────────────────────────────────

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

// ── The independent model ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum MState {
    Active,
    Consumed,
    Revoked,
}

#[derive(Debug, Clone, PartialEq)]
enum MClass {
    Once,
    N(u64),
    Unlimited,
}

impl MClass {
    fn power(&self) -> u8 {
        match self {
            MClass::Once => 0,
            MClass::N(_) => 1,
            MClass::Unlimited => 2,
        }
    }
    fn initial_remaining(&self) -> i64 {
        match self {
            MClass::Once => 1,
            MClass::N(k) => *k as i64,
            MClass::Unlimited => -1,
        }
    }
}

#[derive(Debug, Clone)]
struct MGrant {
    parent: Option<String>,
    scope: Vec<String>,
    class: MClass,
    remaining: i64, // -1 = unlimited
    expires: u64,
    state: MState,
}

#[derive(Debug, Clone, PartialEq)]
enum MFam {
    Missing,
    Reused,
    Revoked,
    Expired,
    Exhausted,
    Escalation,
    /// At zero remaining the ledger's lifecycle check fires before the
    /// debit math — both refusals are fail-closed-correct there.
    EscalationOrExhausted,
}

#[derive(Debug)]
enum MOutcome {
    Ok(()),
    Err(MFam),
}

fn m_scope_attenuates(parent: &[String], child: &[String]) -> bool {
    if child.len() > parent.len() {
        return false;
    }
    parent
        .iter()
        .zip(child.iter())
        .all(|(p, c)| p == "*" || p.eq_ignore_ascii_case(c))
}

#[derive(Default)]
struct Model {
    grants: std::collections::HashMap<String, MGrant>,
}

impl Model {
    fn dead_class(&self, id: &str) -> Option<MFam> {
        match self.grants.get(id) {
            None => Some(MFam::Missing),
            Some(g) => match g.state {
                MState::Consumed => Some(MFam::Reused),
                MState::Revoked => Some(MFam::Revoked),
                MState::Active => None,
            },
        }
    }

    fn issue(&mut self, id: &str, scope: &[String], class: &MClass) -> MOutcome {
        self.grants.insert(
            id.to_string(),
            MGrant {
                parent: None,
                scope: scope.to_vec(),
                class: class.clone(),
                remaining: class.initial_remaining(),
                expires: grants::now_secs() + 3600,
                state: MState::Active,
            },
        );
        MOutcome::Ok(())
    }

    /// Predict + apply a subgrant. The caller passes the REAL outcome so
    /// the child is inserted only when the real ledger minted it.
    fn subgrant(
        &mut self,
        parent_id: &str,
        child_real_id: Option<&str>,
        scope: &[String],
        class: &MClass,
    ) -> MOutcome {
        let (p_scope, p_class, p_remaining, p_expires) = match self.grants.get(parent_id) {
            None => return MOutcome::Err(MFam::Missing),
            Some(p) => (p.scope.clone(), p.class.clone(), p.remaining, p.expires),
        };
        match self.dead_class(parent_id) {
            Some(MFam::Missing) => return MOutcome::Err(MFam::Missing),
            Some(fam) => return MOutcome::Err(fam),
            None => {}
        }
        // Mirror the ledger's check ORDER: the lifecycle exhaustion check
        // (remaining == 0) fires before the attenuation math.
        if p_class != MClass::Unlimited && p_remaining == 0 {
            return MOutcome::Err(MFam::Exhausted);
        }
        if !m_scope_attenuates(&p_scope, scope) {
            return MOutcome::Err(MFam::Escalation);
        }
        let child_expires = grants::now_secs() + 600;
        if child_expires > p_expires {
            return MOutcome::Err(MFam::Escalation);
        }
        if class.power() > p_class.power() {
            return MOutcome::Err(MFam::Escalation);
        }
        let debit: i64 = match class {
            MClass::Unlimited => {
                if p_class != MClass::Unlimited {
                    return MOutcome::Err(MFam::Escalation);
                }
                0
            }
            MClass::Once => 1,
            MClass::N(k) => *k as i64,
        };
        if p_class != MClass::Unlimited && debit > p_remaining {
            return MOutcome::Err(MFam::EscalationOrExhausted);
        }
        // The real ledger accepted → apply the model transition.
        if let Some(p) = self.grants.get_mut(parent_id) {
            if p.class == MClass::Once {
                p.state = MState::Consumed;
                p.remaining = 0;
            } else if p.class != MClass::Unlimited {
                p.remaining -= debit;
            }
        }
        if let Some(child_id) = child_real_id {
            self.grants.insert(
                child_id.to_string(),
                MGrant {
                    parent: Some(parent_id.to_string()),
                    scope: scope.to_vec(),
                    class: class.clone(),
                    remaining: class.initial_remaining(),
                    expires: child_expires,
                    state: MState::Active,
                },
            );
        }
        MOutcome::Ok(())
    }

    fn use_grant(&mut self, id: &str) -> MOutcome {
        let (state, class, remaining, expires) = match self.grants.get(id) {
            None => return MOutcome::Err(MFam::Missing),
            Some(g) => (g.state.clone(), g.class.clone(), g.remaining, g.expires),
        };
        if state == MState::Revoked {
            return MOutcome::Err(MFam::Revoked);
        }
        if state == MState::Consumed {
            return MOutcome::Err(MFam::Reused);
        }
        if grants::now_secs() >= expires {
            return MOutcome::Err(MFam::Expired);
        }
        if remaining == 0 {
            return MOutcome::Err(MFam::Exhausted);
        }
        let (new_state, after) = match class {
            MClass::Once => (MState::Consumed, 0),
            MClass::N(_) => (MState::Active, remaining - 1),
            MClass::Unlimited => (MState::Active, -1),
        };
        if let Some(g) = self.grants.get_mut(id) {
            g.state = new_state;
            g.remaining = after;
        }
        MOutcome::Ok(())
    }

    fn revoke(&mut self, id: &str) -> MOutcome {
        if !self.grants.contains_key(id) {
            return MOutcome::Err(MFam::Missing);
        }
        let mut frontier = vec![id.to_string()];
        while let Some(cur) = frontier.pop() {
            let children: Vec<String> = self
                .grants
                .iter()
                .filter(|(_, g)| g.parent.as_deref() == Some(cur.as_str()))
                .map(|(k, _)| k.clone())
                .collect();
            if let Some(g) = self.grants.get_mut(&cur) {
                if g.state != MState::Revoked {
                    g.state = MState::Revoked;
                }
            }
            frontier.extend(children);
        }
        MOutcome::Ok(())
    }
}

// ── The fuzz loop ──────────────────────────────────────────────────────

const SCOPES: &[&[&str]] = &[
    &["db", "*", "*"],
    &["db", "delete", "*"],
    &["db", "delete", "users"],
    &["db", "drop", "users"],
    &["db", "alter", "users"],
];

fn scope_str(i: usize) -> String {
    SCOPES[i].join(":")
}

fn real_class(c: &MClass) -> GrantClass {
    match c {
        MClass::Once => GrantClass::Once,
        MClass::N(k) => GrantClass::N(*k),
        MClass::Unlimited => GrantClass::Unlimited,
    }
}

fn handle_for(id: &str) -> GrantHandle {
    // The ledger is the SSOT: the algebra reads the record; the handle's
    // cache fields are irrelevant to the algebra operations.
    GrantHandle {
        grant_id: id.to_string(),
        scope: String::new(),
        class: GrantClass::Once,
        expires_at: u64::MAX,
        issuer: "fuzz".to_string(),
    }
}

thread_local! {
    static MODEL: std::cell::RefCell<Model> = std::cell::RefCell::new(Model::default());
}

#[test]
fn fuzz_finds_no_amplification() {
    let mut rng = Rng(0x4D4554414C4F474F); // deterministic seed
    let mut live: Vec<String> = Vec::new(); // real ledger ids
    let mut ops_ok = 0usize;

    for round in 0..4000u64 {
        let op = rng.below(4);
        match op {
            0 => {
                // ── issue ──
                let class = match rng.below(3) {
                    0 => MClass::Once,
                    1 => MClass::N(1 + rng.below(4)),
                    _ => MClass::Unlimited,
                };
                let scope = scope_str(rng.below(SCOPES.len() as u64) as usize);
                let real = grants::issue(&scope, 3600, &real_class(&class), "fuzz");
                let key = match &real {
                    Ok(h) => h.grant_id.clone(),
                    Err(_) => format!("root-{}", round), // never happens for issue
                };
                let m = MODEL.with(|m| {
                    m.borrow_mut().issue(
                        &key,
                        &scope.split(':').map(|s| s.to_string()).collect::<Vec<_>>(),
                        &class,
                    )
                });
                if let Ok(h) = &real {
                    live.push(h.grant_id.clone());
                }
                let matched = match (&real, &m) {
                    (Ok(_), MOutcome::Ok(())) => true,
                    (Err(e), MOutcome::Err(fam)) => fam.real_matches_err(e),
                    _ => false,
                };
                assert!(
                    matched,
                    "round {}: issue real {:?} vs model {:?}",
                    round, real, m
                );
                ops_ok += 1;
            }
            1 => {
                // ── use ──
                if live.is_empty() {
                    continue;
                }
                let target = live[rng.below(live.len() as u64) as usize].clone();
                let real = grants::grant_use(&handle_for(&target), "fuzz use");
                let m = MODEL.with(|m| m.borrow_mut().use_grant(&target));
                let matched = match (&real, &m) {
                    (Ok(_), MOutcome::Ok(())) => true,
                    (Err(e), MOutcome::Err(fam)) => fam.real_matches_err(e),
                    _ => false,
                };
                assert!(
                    matched,
                    "round {}: use real {:?} vs model {:?}",
                    round, real, m
                );
                ops_ok += 1;
            }
            2 => {
                // ── subgrant ──
                if live.is_empty() {
                    continue;
                }
                let target = live[rng.below(live.len() as u64) as usize].clone();
                let class = match rng.below(3) {
                    0 => MClass::Once,
                    1 => MClass::N(1 + rng.below(4)),
                    _ => MClass::Unlimited,
                };
                let scope = scope_str(rng.below(SCOPES.len() as u64) as usize);
                let real = grants::subgrant(&handle_for(&target), &scope, 600, &real_class(&class));
                let child_id = match &real {
                    Ok(h) => Some(h.grant_id.clone()),
                    Err(_) => None,
                };
                if let Some(cid) = &child_id {
                    live.push(cid.clone());
                }
                let m = MODEL.with(|m| {
                    m.borrow_mut().subgrant(
                        &target,
                        child_id.as_deref(),
                        &scope.split(':').map(|s| s.to_string()).collect::<Vec<_>>(),
                        &class,
                    )
                });
                // The real outcome shape: Ok(handle) vs Err(refusal).
                let matched = match (&real, &m) {
                    (Ok(_), MOutcome::Ok(())) => true,
                    (Err(e), MOutcome::Err(fam)) => fam.real_matches_err(e),
                    _ => false,
                };
                assert!(
                    matched,
                    "round {}: subgrant real {:?} vs model {:?}",
                    round, real, m
                );
                ops_ok += 1;
            }
            _ => {
                // ── revoke (cascading) ──
                if live.is_empty() {
                    continue;
                }
                let target = live[rng.below(live.len() as u64) as usize].clone();
                let real = grants::revoke(&handle_for(&target), "fuzz revoke");
                let m = MODEL.with(|m| m.borrow_mut().revoke(&target));
                let matched = match (&real, &m) {
                    (Ok(_), MOutcome::Ok(())) => true,
                    (Err(e), MOutcome::Err(fam)) => fam.real_matches_err(e),
                    _ => false,
                };
                assert!(
                    matched,
                    "round {}: revoke real {:?} vs model {:?}",
                    round, real, m
                );
                ops_ok += 1;
            }
        }
    }

    // The fuzz must have exercised a large state space.
    let modeled = MODEL.with(|m| m.borrow().grants.len());
    assert!(
        modeled >= 800 && ops_ok >= 3000,
        "state space too small: modeled {}, ops {}",
        modeled,
        ops_ok
    );
}

impl MFam {
    fn real_matches_err(&self, e: &str) -> bool {
        match self {
            MFam::Missing => e.starts_with("GRANT_MISSING"),
            MFam::Reused => e.starts_with("GRANT_REUSED"),
            MFam::Revoked => e.starts_with("GRANT_REVOKED"),
            MFam::Expired => e.starts_with("GRANT_EXPIRED"),
            MFam::Exhausted => e.starts_with("GRANT_EXHAUSTED"),
            MFam::Escalation => e.starts_with("GRANT_ESCALATION"),
            MFam::EscalationOrExhausted => {
                e.starts_with("GRANT_ESCALATION") || e.starts_with("GRANT_EXHAUSTED")
            }
        }
    }
}
