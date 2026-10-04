// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL network
// stack (run_test_server + reqwest) by design — the allow is scoped here.
#![allow(clippy::disallowed_methods)]

// ── Naryad №585 (gh#999, P1, testing/core; the audit d63cc1d §7 п.6) ──
//
// The route-body differential fuzzer — the lane the №465 fuzzer did not
// have. The audit's "why was X-1/X-2 not caught": the №465 generator
// produced FLOW programs (no routes), the crosscheck and the realistic
// serve tests used only TAIL responds — the mid-route respond and the
// let-tail classes were invisible to the whole apparatus.
//
// This file closes the lane EXACTLY per the audit plan (no general growth):
//   1. GENERATES deterministic route bodies over the audit-named shapes:
//      (a) respond*/respond_html* in non-tail positions and non-tail
//          branches (bare AND `return` forms);
//      (b) let/assignment as the FINAL statement of the body/branch;
//      (c) nested if/match with respond inside;
//      (d) guard shapes (if … { respond } … more code after).
//      No new builtins enter the tree (literals, let, if, match, respond,
//      redact only); query_param is deliberately NOT generated — the №327
//      UNTRUSTED_DECISION gate would refuse the program for a reason
//      outside this lane's shapes.
//   2. RUNS each program through BOTH serve backends (real HTTP):
//      - both refuse at startup with the SAME class (e.g.
//        [RESPOND_NOT_TERMINAL] — the №581 form-error oracle) → not a
//        divergence;
//      - both refuse with DIFFERENT classes, or only one side starts →
//        divergence;
//      - both start → every generated route is requested on both lanes
//        and the (status, body) pair MUST be identical.
//   3. ANY divergence fails the run LOUDLY: the seed is printed, the full
//      program is the repro (the generation is deterministic by seed —
//      the same repro the minimized №465 cases provide), plus the oracle
//      detail naming the route and the two answers.
//   4. RATCHETS: the checked-in seed corpus (the audit §4 shapes,
//      SEEDS below) runs through the same oracle; ANY divergence fails —
//      the corpus starts EMPTY of known classes by design (both
//      X-classes are CLOSED; a new one must not land silently).
//
// The budget: 30 programs per run (FUZZ_ROUTES_ITERS overrides for a deep
// manual run: `FUZZ_ROUTES_ITERS=500 cargo test ... -- --nocapture`).

use metalogos_server::server::ServeBackend;

fn iterations() -> u32 {
    std::env::var("FUZZ_ROUTES_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}

// ── The deterministic RNG (the №465 xorshift posture) ────────────────

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
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let i = self.below(items.len() as u64) as usize;
        &items[i]
    }
    fn alnum(&mut self, n: usize) -> String {
        (0..n)
            .map(|_| {
                let alphabet = b"abcdefghjkmnpqrstuvwxyz23456789";
                alphabet[(self.next_u64() % alphabet.len() as u64) as usize] as char
            })
            .collect()
    }
}

// ── The route-body generator ──────────────────────────────────────────

const STATUSES: &[&str] = &["200", "403", "404"];
const WORDS: &[&str] = &["alpha", "beta", "gamma", "delta"];

enum Stmt {
    Let(String),
    MutLet(String),
    Assign(String),
    Respond { args: String, returned: bool },
    If {
        cond: String,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    Match {
        scrutinee: String,
        arms: Vec<(String, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
}

fn render(stmts: &[Stmt], indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut out = String::new();
    for s in stmts {
        match s {
            Stmt::Let(e) => out.push_str(&format!("{pad}let {e}\n")),
            Stmt::MutLet(e) => out.push_str(&format!("{pad}let mut {e}\n")),
            Stmt::Assign(e) => out.push_str(&format!("{pad}{e}\n")),
            Stmt::Respond { args, returned } => {
                let form = if *returned {
                    format!("return respond({args})")
                } else {
                    format!("respond({args})")
                };
                out.push_str(&format!("{pad}{form}\n"));
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                out.push_str(&format!("{pad}if {cond} {{\n"));
                out.push_str(&render(then_body, indent + 1));
                out.push_str(&format!("{pad}}}\n"));
                if let Some(eb) = else_body {
                    out.push_str(&format!("{pad}else {{\n"));
                    out.push_str(&render(eb, indent + 1));
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            Stmt::Match {
                scrutinee,
                arms,
                else_body,
            } => {
                out.push_str(&format!("{pad}match {scrutinee} {{\n"));
                for (lit, body) in arms {
                    out.push_str(&format!("{pad}  \"{lit}\" then {{\n"));
                    out.push_str(&render(body, indent + 2));
                    out.push_str(&format!("{pad}  }}\n"));
                }
                if let Some(eb) = else_body {
                    out.push_str(&format!("{pad}  else {{\n"));
                    out.push_str(&render(eb, indent + 2));
                    out.push_str(&format!("{pad}  }}\n"));
                }
                out.push_str(&format!("{pad}}}\n"));
            }
        }
    }
    out
}

/// Generate one route body over the audit §4 shapes. The forms are chosen
/// so the program is either SERVED identically on both backends or REFUSED
/// identically by the №581 gate — anything else is a divergence.
fn gen_body(rng: &mut Rng, budget: &mut u32) -> Vec<Stmt> {
    let mut out: Vec<Stmt> = Vec::new();
    let n = 1 + rng.below(3);
    for i in 0..n {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let is_last = i + 1 == n;
        // The tail statement is a value or a let-tail (the X-2 shape);
        // interior statements may be the X-1 shapes (bare/respond guards).
        let kind = if is_last { rng.below(4) } else { rng.below(8) };
        match kind {
            0 => {
                let status = rng.pick(STATUSES).to_string();
                let body_word = rng.pick(WORDS).to_string();
                out.push(Stmt::Respond {
                    args: format!("\"{status}\", \"{body_word}\""),
                    returned: false,
                });
            }
            1 => {
                let status = rng.pick(STATUSES).to_string();
                let body_word = rng.pick(WORDS).to_string();
                out.push(Stmt::Respond {
                    args: format!("\"{status}\", \"{body_word}\""),
                    returned: true,
                });
            }
            2 => {
                let v = format!("v{}", rng.alnum(3));
                let val = if rng.below(2) == 0 {
                    format!("\"{}\"", rng.alnum(4))
                } else {
                    (rng.below(90) + 10).to_string()
                };
                out.push(Stmt::Let(format!("{v} = {val}")));
            }
            3 => {
                // The X-2 shape: the let TAIL (the fall-through used to
                // leave the slot on the stack; the epilogue answers Unit).
                let v = format!("t{}", rng.alnum(3));
                out.push(Stmt::Let(format!("{v} = \"{}\"", rng.alnum(5))));
            }
            4 => {
                // The assignment tail (the same leftover-local class):
                // a declared MUT binding, assigned as the final statement.
                let name = format!("m{}", rng.alnum(2));
                out.push(Stmt::MutLet(format!(
                    "{name} = \"{}\"",
                    rng.alnum(4)
                )));
                out.push(Stmt::Assign(format!(
                    "{name} = \"{}\"",
                    rng.alnum(5)
                )));
            }
            5 => {
                // (d): the guard with respond inside, MORE CODE AFTER —
                // the bare form here is the №581 gate's exact class (both
                // backends must refuse); the return form must serve.
                let status = rng.pick(STATUSES).to_string();
                let returned = rng.below(2) == 0;
                let cond = format!("\"{}\" == \"{}\"", rng.alnum(3), rng.alnum(3));
                let body = vec![Stmt::Respond {
                    args: format!("\"{status}\", \"guard\""),
                    returned,
                }];
                out.push(Stmt::If {
                    cond,
                    then_body: body,
                    else_body: None,
                });
            }
            6 => {
                // (c): a nested if with respond inside — the branch ends
                // with a respond (a value tail) or with a let (№582).
                let cond = format!("{} == {}", rng.below(5), rng.below(5));
                let inner = if rng.below(2) == 0 {
                    vec![Stmt::Respond {
                        args: "\"200\", \"nested\"".to_string(),
                        returned: false,
                    }]
                } else {
                    vec![Stmt::Let(format!("n{} = \"{}\"", rng.alnum(2), rng.alnum(4)))]
                };
                out.push(Stmt::If {
                    cond,
                    then_body: inner,
                    else_body: None,
                });
            }
            _ => {
                // A let-tail one level deeper (the same №582 class).
                out.push(Stmt::Let(format!(
                    "d{} = \"{}\"",
                    rng.alnum(2),
                    rng.alnum(5)
                )));
            }
        }
    }
    if rng.below(4) == 0 {
        // One level of nesting: a match with respond in an arm and a
        // let-else or no else at all (the fall-through path).
        let scrutinee = (rng.below(3)).to_string();
        let lit = (rng.below(3)).to_string();
        let arm_body = vec![Stmt::Respond {
            args: "\"200\", \"match-arm\"".to_string(),
            returned: rng.below(2) == 0,
        }];
        let else_body = if rng.below(2) == 0 {
            Some(vec![Stmt::Let(format!("e{} = 1", rng.alnum(2)))])
        } else {
            None
        };
        out.push(Stmt::Match {
            scrutinee,
            arms: vec![(lit, arm_body)],
            else_body,
        });
    }
    out
}

/// Generate one program: a server with 1–2 GET routes over the shapes.
fn gen_program(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut budget = 12u32;
    let mut src = String::from("mlogserver {\n  port: 0\n");
    let routes = 1 + rng.below(2);
    for r in 0..routes {
        let path = format!("/r{}/{}/q", seed % 1000, r);
        src.push_str(&format!("  route \"{path}\" method=GET {{\n"));
        src.push_str(&render(&gen_body(&mut rng, &mut budget), 2));
        src.push_str("  }\n");
    }
    src.push_str("}\n");
    src
}

// ── The oracle ────────────────────────────────────────────────────────

fn startup_class(err: &str) -> String {
    let head = err.trim();
    match (head.find('['), head.find(']')) {
        (Some(a), Some(b)) if b > a => head[a..=b].to_string(),
        _ => head.chars().take(40).collect(),
    }
}

enum Outcome {
    /// Both backends refuse with the SAME class — the form-error oracle
    /// (e.g. [RESPOND_NOT_TERMINAL]). Not a divergence.
    BothRefuseSame { class: String },
    /// Both started — the HTTP diff runs over every route.
    BothStart {
        tw: u16,
        vm: u16,
        routes: Vec<String>,
    },
    /// The classes differ, or only one side started — a divergence BY SHAPE.
    Asymmetric { detail: String },
}

fn extract_routes(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("route \"") {
            if let Some(path) = rest.split('"').next() {
                out.push(path.to_string());
            }
        }
    }
    out
}

async fn probe(source: &str) -> Outcome {
    let tw = metalogos_server::server::run_test_server_with_backend(
        source,
        ServeBackend::Interpreter,
    )
    .await;
    let vm =
        metalogos_server::server::run_test_server_with_backend(source, ServeBackend::Vm).await;
    match (tw, vm) {
        (Err(et), Err(ev)) => {
            let ct = startup_class(&et.to_string());
            let cv = startup_class(&ev.to_string());
            if ct != cv {
                Outcome::Asymmetric {
                    detail: format!(
                        "both refused, different classes: TW {ct} vs VM {cv}\nTW: {et}\nVM: {ev}"
                    ),
                }
            } else {
                Outcome::BothRefuseSame { class: ct }
            }
        }
        (Ok(_), Err(e)) => Outcome::Asymmetric {
            detail: format!("TW started, VM refused: {e}"),
        },
        (Err(e), Ok(_)) => Outcome::Asymmetric {
            detail: format!("VM started, TW refused: {e}"),
        },
        (Ok((tp, _)), Ok((vp, _))) => Outcome::BothStart {
            tw: tp,
            vm: vp,
            routes: extract_routes(source),
        },
    }
}

async fn http_pair(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    match reqwest::get(&url).await {
        Ok(r) => {
            let status = r.status().as_u16();
            let body = r.text().await.unwrap_or_default();
            (status, body)
        }
        Err(e) => (0, format!("reqwest-error: {e}")),
    }
}

async fn diff_one(source: &str) -> Option<String> {
    match probe(source).await {
        Outcome::BothRefuseSame { class } => {
            // The form-error oracle: the №581 contract — both backends
            // refuse with the same class. Not a divergence.
            println!("n585: both refuse identically: {class}");
            None
        }
        Outcome::Asymmetric { detail } => Some(detail),
        Outcome::BothStart { tw, vm, routes } => {
            for path in &routes {
                let t = http_pair(tw, path).await;
                let v = http_pair(vm, path).await;
                if t != v {
                    return Some(format!("route {path}: TW {t:?} != VM {v:?}"));
                }
            }
            None
        }
    }
}

// ── The seed corpus (the audit §4 shapes, checked-in) ─────────────────

const SEED_GUARD_BARE: &str = r#"
mlogserver {
  port: 0
  route "/guard" method=GET {
    let key = redact("k", "hash_only")
    let expected = redact("admin", "hash_only")
    if key == expected {
      respond("403", "forbidden")
    }
    respond("200", "purged")
  }
}
"#;

const SEED_LET_TAIL: &str = r#"
mlogserver {
  port: 0
  route "/leak" method=GET {
    let user = "alice"
    let row = "email:phone:balance of " + user
  }
}
"#;

const SEED_GUARD_RETURN: &str = r#"
mlogserver {
  port: 0
  route "/guard" method=GET {
    let key = redact("k", "hash_only")
    let expected = redact("admin", "hash_only")
    if key != expected {
      return respond("403", "forbidden")
    }
    respond("200", "purged")
  }
}
"#;

const SEEDS: &[(&str, &str)] = &[
    ("seed_route_guard_bare", SEED_GUARD_BARE),
    ("seed_route_let_tail", SEED_LET_TAIL),
    ("seed_route_guard_return", SEED_GUARD_RETURN),
];

#[tokio::test]
async fn n585_route_body_fuzzer_both_backends() {
    // The seed corpus first: the audit §4 shapes MUST pass the oracle
    // (both X-classes are closed — a divergence here is a regression).
    for (name, src) in SEEDS {
        if let Some(div) = diff_one(src).await {
            panic!("n585 SEED REGRESSION ({name}) — the closed class reopened:\n{div}\n--- repro ---\n{src}");
        }
    }

    let iters = iterations();
    let mut refused = 0u32;
    let mut served = 0u32;
    let mut asymmetric: Vec<(u64, String)> = Vec::new();
    for i in 0..iters {
        let seed = 0x0058_5835_0000_0000u64 + i as u64;
        let src = gen_program(seed);
        match probe(&src).await {
            Outcome::BothRefuseSame { .. } => refused += 1,
            Outcome::BothStart { .. } => served += 1,
            Outcome::Asymmetric { detail } => asymmetric.push((seed, detail)),
        }
    }
    println!(
        "n585: {iters} programs — {served} served (HTTP-diffed), {refused} refused identically"
    );

    assert!(
        served > 0,
        "n585: the generator produced no servable program — the lane is blind"
    );
    if !asymmetric.is_empty() {
        let (seed, detail) = &asymmetric[0];
        let repro = gen_program(*seed);
        panic!(
            "n585 ASYMMETRIC TW/VM divergence (seed {seed:#x}, +{} more) — the route-body lane \
             caught a backend difference. Repro (deterministic by seed):\n{repro}\n--- oracle ---\n{detail}",
            asymmetric.len() - 1
        );
    }
}
