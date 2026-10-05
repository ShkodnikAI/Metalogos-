// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL network
// stack (run_test_server + reqwest) by design — the allow is scoped here.
#![allow(clippy::disallowed_methods)]

// ── Naryad №594 (issue #1019; Волна 30, Камертон Н1-05; P1 [testing]/[core]) ──
//
// The tick↔route parity of a PURE function — the third diff axis of the
// differential family, not duplicating №465 (flow programs, TW↔VM) or
// №585 (route bodies, TW↔VM). The audit's consumer posture: the ENGINE
// (the selection, the decay arithmetic, the draw) is computed in the
// cron TICK, the REPORT is served by the ROUTE — and since ADR-0171 the
// route default backend is the VM while the tick context is the TW
// interpreter construction (ADR-0175 §4's honest boundary). A divergence
// means the system COMPUTES one answer and SERVES another — the worst
// visible class for the Камертон consumer, whose one program executes
// `normal_sample` and the decay arithmetic on two different backends in
// two different contexts.
//
// The contract under test (the issue's Задача 1): a pure function with
// zero effects produces the IDENTICAL result in the cron-tick body and
// in the route body at the same seed — byte-for-byte, on BOTH serve
// backends (route TW and route VM), against the tick's TW.
//
// The lanes:
//   1. THE SAMPLE PROGRAM (checked-in, the DoD shape): `normal_sample`
//      + an exponent (`pow`) + a comparison, executed 100 times — the
//      tick sequence (ONE serve boot, 100 scheduler dispatches) against
//      100 HTTP requests on each route lane (the call lane: the route
//      CALLS the pattern; the inline lane: the route body carries the
//      same statements — both compilation paths on both backends). The
//      honest expected value is computed by the INDEPENDENT replication
//      below (the №590 GoldenReplication contract — seed_to_state →
//      xorshift64 → Box–Muller), so the test pins not just "the three
//      lanes agree" but "they agree on the RIGHT value".
//   2. THE FUZZER ARM (the issue's Задача 3 — the class joins the
//      differential corpus): a deterministic generator over the
//      pure-computation grammar (the seed literal, the sampler
//      parameters, the transform, the comparison, the branch labels —
//      xorshift64 by seed, the №465 posture) produces programs run
//      through the tick↔route oracle. ANY divergence fails LOUDLY with
//      the seed and the full program as the reproducing input (the
//      generation is deterministic by seed — the same repro shape the
//      minimized №465 cases provide).
//
// PLACEMENT (the honest note on the issue's «корпус №465» wording): the
// №465 corpus file lives in the language crate's tests/, which cannot
// depend on the server crate (the №567 package graph forbids the cycle),
// and the tick executor + the route server live in metalogos-server —
// the №585 precedent placed its lane here for exactly that reason. The
// class is therefore machine-watched HERE, with the same ratchet
// discipline: the seed corpus checked in, the generator deterministic,
// a divergence = a loud fail with the repro.
//
// THE BOUNDARY (the issue's Границы): tests only — ZERO production
// fixes. A found divergence becomes a separate repair naryad with the
// reproducing input this file prints.

use metalogos::interpreter::Value;
use metalogos_server::server::{run_test_server_with_backend, test_tick_sequence, ServeBackend};

// ── The independent replication (the №590 mutation-pin contract) ─────

fn seed_to_state(seed: f64) -> u64 {
    let bits = seed.to_bits();
    let state = bits ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        0x9E3779B97F4A7C15
    } else {
        state
    }
}

fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// One normal draw per the №590 contract: u1 = 1−u ∈ (0,1], u2 = u ∈ [0,1),
/// z = sqrt(−2·ln u1)·cos(2π·u2), result = mean + stddev·z.
struct GoldenReplication {
    state: u64,
}

impl GoldenReplication {
    fn seeded(seed: f64) -> Self {
        Self {
            state: seed_to_state(seed),
        }
    }

    fn next_uniform(&mut self) -> f64 {
        let mantissa = xorshift64(&mut self.state) >> 11;
        (mantissa as f64) / ((1u64 << 53) as f64)
    }

    fn normal(&mut self, mean: f64, stddev: f64) -> f64 {
        let u1 = 1.0 - self.next_uniform();
        let u2 = self.next_uniform();
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        mean + stddev * z
    }
}

// ── The deterministic generator RNG (the №465 xorshift posture) ──────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
    }
}

fn iterations() -> u32 {
    std::env::var("FUZZ_TICK_ROUTE_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}

// ── The pure-computation program assembly ────────────────────────────

/// One generated computation: every field lands as a LITERAL in the
/// program text (the seed fully determines both the program and the
/// expected value).
struct Shape {
    seed: f64,
    mean: f64,
    stddev: f64,
    /// The transform template with `{x}` as the draw placeholder.
    transform: &'static str,
    cmp: &'static str,
    threshold: f64,
    hi: &'static str,
    lo: &'static str,
}

impl Shape {
    fn body(&self) -> String {
        format!(
            "  random_seed({})\n  let x = normal_sample({}, {})\n  let y = {}\n  let mut label = \"{}\"\n  if y {} {} {{\n    label = \"{}\"\n  }}\n  return label + \":\" + to_string(y)",
            fmt_f(self.seed),
            fmt_f(self.mean),
            fmt_f(self.stddev),
            self.transform.replace("{x}", "x"),
            self.lo,
            self.cmp,
            fmt_f(self.threshold),
            self.hi,
        )
    }

    /// The independent expected value (the replication, NOT the src).
    fn expected(&self) -> String {
        let mut g = GoldenReplication::seeded(self.seed);
        let x = g.normal(self.mean, self.stddev);
        let y = match self.transform {
            "pow(2.0, {x})" => 2.0f64.powf(x),
            "pow(10.0, {x})" => 10.0f64.powf(x),
            "exp({x})" => x.exp(),
            "sqrt({x} * {x} + 1.0)" => (x * x + 1.0).sqrt(),
            t => panic!("unmapped transform in the replication: {t}"),
        };
        let fired = match self.cmp {
            ">" => y > self.threshold,
            "<" => y < self.threshold,
            ">=" => y >= self.threshold,
            "<=" => y <= self.threshold,
            t => panic!("unmapped comparison in the replication: {t}"),
        };
        let label = if fired { self.hi } else { self.lo };
        format!("{}:{}", label, y)
    }

    fn program(&self) -> String {
        format!(
            "pattern Compute(_payload: String) -> String {{\n{}\n}}\nmlogserver {{\n  port: 0\n  route \"/parity/call\" method=GET {{\n    let r = Compute(\"p\")\n    respond(\"200\", r)\n  }}\n  route \"/parity/inline\" method=GET {{\n{}\n    respond(\"200\", label + \":\" + to_string(y))\n  }}\n}}\n",
            self.body(),
            self.body(),
        )
    }
}

/// Float literal rendering: the mlog float literal is the same value the
/// replication reads (Rust's `{}` on f64 — round-trip exact).
fn fmt_f(v: f64) -> String {
    format!("{}", v)
}

// ── The oracle ───────────────────────────────────────────────────────

async fn boot(
    source: &str,
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    run_test_server_with_backend(source, backend)
        .await
        .expect("the route server must boot for the parity lane")
}

async fn http_body(port: u16, path: &str) -> String {
    let url = format!("http://127.0.0.1:{port}{path}");
    match reqwest::get(&url).await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(e) => format!("reqwest-error: {e}"),
    }
}

/// The ONE-tick lane: a fresh serve boot per program (the test_tick_call
/// posture), the pattern target, one String argument.
async fn tick_once(source: &str) -> Result<Value, String> {
    // test_tick_call is a ONE-tick surface; a sequence surface would hold
    // the boot across programs — for the fuzzer the per-program boot is
    // the honest isolation (a dirty process-global state must not mask a
    // divergence by leaking INTO the next program).
    metalogos_server::server::test_tick_call(
        source,
        "Compute",
        vec![Value::String("p".to_string())],
    )
    .await
}

fn value_string(v: &Value, what: &str) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => panic!(
            "{what}: the pure computation must return String, got {}",
            other.type_name()
        ),
    }
}

// ── The checked-in sample (the DoD program) ──────────────────────────

const SAMPLE: Shape = Shape {
    seed: 42.0,
    mean: 0.0,
    stddev: 1.0,
    transform: "pow(2.0, {x})",
    cmp: ">",
    threshold: 1.0,
    hi: "hi",
    lo: "lo",
};

const RUNS: usize = 100;

/// The DoD row: the sample program gives an IDENTICAL result in the tick
/// and in the route on 100 runs, both backends — byte-for-byte, and
/// equal to the INDEPENDENT replication's value every single run (the
/// state-bleed axis: a leaked PRNG stream, a dirty pool slot or a
/// per-request drift breaks the per-run equality immediately).
#[tokio::test]
async fn n594_sample_parity_100_runs_both_backends() {
    let src = SAMPLE.program();
    let expected = SAMPLE.expected();

    // Both route lanes boot ONCE; the 100 runs are 100 real HTTP
    // requests (the route recomputes per request — the production
    // posture), not one answer copied 100 times.
    let (tw_port, _tw) = boot(&src, ServeBackend::Interpreter).await;
    let (vm_port, _vm) = boot(&src, ServeBackend::Vm).await;

    // The tick side: ONE serve boot, 100 scheduler dispatches (the
    // №426 test_tick_sequence surface — the executor the scheduler
    // actually uses).
    let calls = vec![("Compute".to_string(), vec![Value::String("p".to_string())]); RUNS];
    let ticks = test_tick_sequence(&src, calls).await;

    let mut seen: Option<String> = None;
    for (i, tick) in ticks.iter().enumerate() {
        let t = tick
            .as_ref()
            .unwrap_or_else(|e| panic!("run {i}: the tick must succeed: {e}"));
        let t = value_string(t, &format!("run {i}: tick"));
        let tw = http_body(tw_port, "/parity/call").await;
        let vm = http_body(vm_port, "/parity/inline").await;
        assert_eq!(
            t, tw,
            "run {i}: the tick and the TW route diverge (byte-for-byte contract)"
        );
        assert_eq!(
            t, vm,
            "run {i}: the tick and the VM route diverge (byte-for-byte contract)"
        );
        match &seen {
            Some(prev) => assert_eq!(
                prev, &t,
                "run {i}: the value drifted across runs — the state is not per-context"
            ),
            None => seen = Some(t.clone()),
        }
        assert_eq!(
            t, expected,
            "run {i}: the lanes agree on a WRONG value (the replication is the anchor)"
        );
    }

    // The second inline route lane rides the same boot — the inline
    // route-body compilation on both backends must equal the call lane
    // AND the tick too (the whole point: the same computation from any
    // context is one answer).
    let tw_inline = http_body(tw_port, "/parity/inline").await;
    let vm_inline = http_body(vm_port, "/parity/inline").await;
    assert_eq!(expected, tw_inline, "the TW inline lane diverges");
    assert_eq!(expected, vm_inline, "the VM inline lane diverges");
}

// ── The fuzzer arm ───────────────────────────────────────────────────

const TRANSFORMS: &[&str] = &[
    "pow(2.0, {x})",
    "pow(10.0, {x})",
    "exp({x})",
    "sqrt({x} * {x} + 1.0)",
];
const CMPS: &[&str] = &[">", "<", ">=", "<="];
const LABELS: &[(&str, &str)] = &[("hi", "lo"), ("above", "below"), ("pos", "neg")];

fn gen_shape(seed: u64) -> Shape {
    let mut rng = Rng::new(seed);
    let mean = (rng.below(200) as f64 - 100.0) / 10.0;
    let stddev = 0.5 + (rng.below(20) as f64) / 10.0;
    // NOTE: direct INDEXING, not a generic pick — the element comes out
    // as `&'static str` by value, so no reference-of-reference unification
    // happens at the field site (the 1.93 MSRV refuses that coercion;
    // found by the msrv job, fixed version-proof).
    let transform = TRANSFORMS[rng.below(TRANSFORMS.len() as u64) as usize];
    let cmp = CMPS[rng.below(CMPS.len() as u64) as usize];
    let (hi, lo) = LABELS[rng.below(LABELS.len() as u64) as usize];
    Shape {
        seed: (rng.below(1_000_000) as f64) + 1.0,
        mean,
        stddev,
        transform,
        cmp,
        threshold: (rng.below(20) as f64) / 10.0,
        hi,
        lo,
    }
}

/// The tick↔route differential oracle over the generated corpus: a
/// divergence = a LOUD fail with the seed, the program and the three
/// answers (the repro is deterministic by seed — the №465 discipline).
#[tokio::test]
async fn n594_tick_route_diff_fuzzer() {
    let iters = iterations();
    let mut served = 0u32;
    for i in 0..iters {
        let seed = 0x0059_5494_0000_0000u64 + i as u64;
        let shape = gen_shape(seed);
        let src = shape.program();
        let expected = shape.expected();

        let tick = tick_once(&src).await;
        let (tw_port, _tw) = boot(&src, ServeBackend::Interpreter).await;
        let (vm_port, _vm) = boot(&src, ServeBackend::Vm).await;
        let tw_call = http_body(tw_port, "/parity/call").await;
        let vm_call = http_body(vm_port, "/parity/call").await;
        let tw_inline = http_body(tw_port, "/parity/inline").await;
        let vm_inline = http_body(vm_port, "/parity/inline").await;

        match &tick {
            Ok(v) => {
                let t = value_string(v, &format!("seed {seed:#x}: tick"));
                let answers = [
                    ("tick", t.clone()),
                    ("route/call TW", tw_call),
                    ("route/call VM", vm_call),
                    ("route/inline TW", tw_inline),
                    ("route/inline VM", vm_inline),
                ];
                let diverged = answers.iter().any(|(_, s)| *s != t);
                if diverged {
                    let detail: String = answers
                        .iter()
                        .map(|(n, s)| format!("  {n}: {s:?}\n"))
                        .collect();
                    panic!(
                        "n594 TICK↔ROUTE DIVERGENCE (fuzz seed {seed:#x}, iteration {i}) — \
                         the pure computation answered differently across contexts. \
                         Repro (deterministic by seed):\n{src}\n--- the answers ---\n{detail}\n\
                         THE BOUNDARY: zero production fixes in this naryad — file the \
                         repair naryad with this repro."
                    );
                }
                if t != expected {
                    let detail: String = answers
                        .iter()
                        .map(|(n, s)| format!("  {n}: {s:?}\n"))
                        .collect();
                    panic!(
                        "n594 LANES AGREE ON A WRONG VALUE (fuzz seed {seed:#x}) — the \
                         independent replication anchors {expected:?}. The answers:\n{detail}\n{src}",
                    );
                }
                served += 1;
            }
            Err(e) => {
                // A refusal must be CONSISTENT: both route lanes refuse
                // too (the same-class posture), or the tick is the odd
                // one out — either way that IS a divergence for this
                // lane's shapes (the grammar is pure computation; there
                // is no legal per-backend refusal asymmetry here).
                let tw_refused = tw_call.starts_with("reqwest-error") || tw_call.is_empty();
                let vm_refused = vm_call.starts_with("reqwest-error") || vm_call.is_empty();
                panic!(
                    "n594: the tick REFUSED where the lanes serve (fuzz seed {seed:#x}, \
                     iteration {i}): {e}\nTW route answered: {tw_call:?} (refused={tw_refused})\n\
                     VM route answered: {vm_call:?} (refused={vm_refused})\n--- repro ---\n{src}",
                );
            }
        }
    }
    assert!(
        served > 0,
        "n594: the generator produced no served program — the lane is blind"
    );
    println!(
        "n594: {iters} generated programs — {served} served (tick + both route lanes, \
         byte-for-byte against the replication)"
    );
}
