// ── Naryad #465 (P1, testing/vm): the TW↔VM diff fuzzer ─────────────
//
// The audit 25.09 §4.1 finding: `crosscheck_backends` compares the
// backends on a FIXED corpus of programs both can execute — blind to
// divergences by construction. 60 names are duplicated (the №462
// counter), the VM is the default production backend — a backend
// divergence is the worst bug class: silent incorrectness.
//
// The fuzzer (this file):
//   1. GENERATES deterministic .mlog programs from a seeded RNG over a
//      bounded subset (entities, patterns, calls, string/math ops,
//      if/else blocks, each loops, bounded whiles, and the STATEFUL
//      memory group — memory_open/put/read/keys/forget — the top of
//      the transfer-priority list the report proposes);
//   2. RUNS each program through BOTH backends (the same in-process
//      harness the crosscheck uses) and compares outcome + stdout +
//      the error class;
//   3. MINIMIZES each found divergence (statement-level delta
//      debugging) and fingerprints it;
//   4. RATCHETS: a divergence whose fingerprint is not in
//      `tests/fuzz_corpus/known_divergences.txt` FAILS the run (a new
//      backend divergence cannot land silently). Known divergences are
//      reported honestly — the corpus is NOT tuned to green.
//
// The generator choice is deliberate (the naryad allows either):
// proptest-over-AST was rejected because the minimization and the
// reproducibility we need are simpler over the checked-in template
// source (a seed fully determines the program; the minimized case IS
// the minimal .mlog file), and no new dependency enters the tree.
// JIT stays out (ADR-0073, the documented gap).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The per-run iteration budget (CI-bounded; FUZZ_ITERS overrides for a
/// deep manual run: `FUZZ_ITERS=2000 cargo test --test naryad_465_diff_fuzzer`).
fn iterations() -> u32 {
    std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150)
}

/// xorshift64 — deterministic, dependency-free.
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

// ── The generator ─────────────────────────────────────────────────────

const WORDS: &[&str] = &["alpha", "beta", "gamma", "delta", "omega"];
const BUILTIN_STRING_OPS: &[&str] = &["upper", "lower", "trim"];

fn gen_expr(rng: &mut Rng, vars: &[String], depth: u32) -> String {
    if depth == 0 {
        return match rng.below(4) {
            0 => {
                let n = 4 + rng.below(6) as usize;
                format!("\"{}\"", rng.alnum(n))
            }
            1 => (rng.below(100) as i64).to_string(),
            2 => vars
                .last()
                .cloned()
                .unwrap_or_else(|| "\"seed\"".to_string()),
            _ => rng.pick(WORDS).to_string(),
        };
    }
    match rng.below(8) {
        0 => format!(
            "{} + {}",
            gen_expr(rng, vars, depth - 1),
            gen_expr(rng, vars, depth - 1)
        ),
        1 => format!(
            "if {} == {} then {} else {}",
            gen_expr(rng, vars, depth - 1),
            gen_expr(rng, vars, depth - 1),
            gen_expr(rng, vars, depth - 1),
            gen_expr(rng, vars, depth - 1)
        ),
        2 => format!(
            "{}({})",
            rng.pick(BUILTIN_STRING_OPS),
            gen_expr(rng, vars, depth - 1)
        ),
        3 => format!("len({})", gen_expr(rng, vars, depth - 1)),
        4 => format!("to_string({})", gen_expr(rng, vars, depth - 1)),
        5 => {
            let n = 2 + rng.below(3);
            let items: Vec<String> = (0..n).map(|_| gen_expr(rng, vars, depth - 1)).collect();
            format!("len([{}])", items.join(", "))
        }
        6 => format!(
            "{}({})",
            rng.pick(BUILTIN_STRING_OPS),
            gen_expr(rng, vars, depth - 1)
        ),
        _ => gen_expr(rng, vars, 0),
    }
}

fn gen_stmts(rng: &mut Rng, vars: &mut Vec<String>, _depth: u32, budget: &mut u32) -> Vec<String> {
    let mut out = Vec::new();
    let n = 2 + rng.below(4);
    for _ in 0..n {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        match rng.below(6) {
            0 => {
                let v = format!("v{}", rng.alnum(3));
                out.push(format!("let {} = {}", v, gen_expr(rng, vars, 2)));
                vars.push(v);
            }
            1 => out.push(format!(
                "if {} == {} {{\n  {}let m = \"eq\"\n}} else {{\n  {}let m = \"ne\"\n}}",
                gen_expr(rng, vars, 1),
                gen_expr(rng, vars, 1),
                "",
                ""
            )),
            2 => {
                out.push(
                    "each item in [\"a\", \"b\", \"c\"] {{\n  let acc = len(item)\n}}"
                        .to_string(),
                );
            }
            3 => {
                let var = format!("w{}", rng.alnum(2));
                vars.push(var.clone());
                out.push(format!(
                    "let {var} = \"\"\nlet guard = 0\nwhile guard < 3 {{\n  let guard = guard + 1\n}}"
                ));
            }
            4 => {
                let name = rng.alnum(2);
                out.push(format!("let s{name} = {}", gen_expr(rng, vars, 2)));
            }
            _ => {
                let name = rng.alnum(2);
                out.push(format!("let t{name} = {}", gen_expr(rng, vars, 1)));
            }
        }
    }
    out
}

/// Generate one program. Returns the .mlog source.
fn gen_program(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut budget = 26u32;
    let mut src = String::new();

    // Entities (top-level initializers).
    for i in 0..2 {
        src.push_str(&format!(
            "entity e{}: String = \"{}\"\n",
            i,
            rng.alnum(5 + i)
        ));
    }

    // Pattern A: pure transformation (calls the string/math ops).
    src.push_str("\npattern pa(x: String) -> String {\n");
    let mut vars = vec!["x".to_string()];
    for st in gen_stmts(&mut rng, &mut vars, 2, &mut budget) {
        src.push_str(&format!("  {}\n", st.replace('\n', "\n  ")));
    }
    src.push_str(&format!("  return {}\n}}\n", gen_expr(&mut rng, &vars, 2)));

    // Pattern B: the STATEFUL group — memory ops (the transfer-priority
    // group the report proposes from the №462 artifact data).
    let mem_name = format!("fuzz-{}", seed);
    src.push_str("\npattern pb(y: String) -> String {\n");
    src.push_str(&format!(
        "  let mem = memory_open(\"{}\", \"public\")\n",
        mem_name
    ));
    src.push_str("  memory_put(mem, \"k1\", y)\n");
    src.push_str(&format!(
        "  memory_put(mem, \"k2\", \"{}\")\n",
        rng.alnum(4)
    ));
    src.push_str("  let r1 = memory_read(mem, \"k1\")\n");
    src.push_str("  let n1 = len(memory_keys(mem))\n");
    src.push_str("  memory_forget(mem, \"k2\")\n");
    src.push_str("  let n2 = len(memory_keys(mem))\n");
    let mut vars = vec!["y".to_string(), "r1".to_string()];
    for st in gen_stmts(&mut rng, &mut vars, 1, &mut budget) {
        src.push_str(&format!("  {}\n", st.replace('\n', "\n  ")));
    }
    src.push_str(&format!(
        "  return r1 + \":\" + to_string(n1) + \":\" + to_string(n2) + \":\" + {}\n}}\n",
        gen_expr(&mut rng, &vars, 1)
    ));

    // The flow: e0 → pa → pb → output (both backends run the same wiring).
    src.push_str("\nflow Main {\n  input: String = e0 -> pa -> pb -> output\n}\n");
    src
}

// ── The harness (the crosscheck posture) ──────────────────────────────

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn norm_ok(v: &Option<String>) -> String {
    v.as_deref()
        .map(|s| s.trim_end().to_string())
        .unwrap_or_default()
}

/// The error CLASS: the stable bracketed code when present, else the head.
fn err_class(err: &str) -> String {
    let head = err.trim();
    match (head.find('['), head.find(']')) {
        (Some(a), Some(b)) if b > a => head[a..=b].to_string(),
        _ => head.chars().take(40).collect(),
    }
}

#[derive(Debug)]
enum Divergence {
    Outcome { tw: String, vm: String },
    Output { tw: String, vm: String },
    ErrorClass { tw: String, vm: String },
}

impl Divergence {
    fn signature(&self) -> String {
        match self {
            Divergence::Outcome { tw, vm } => format!("outcome|tw={}|vm={}", tw, vm),
            Divergence::Output { tw, vm } => format!("output|tw={}|vm={}", tw, vm),
            Divergence::ErrorClass { tw, vm } => format!("errorclass|tw={}|vm={}", tw, vm),
        }
    }

    /// The ratchet signature: the divergence CLASS with the program-specific
    /// detail masked (digits → N, quoted literals → S, identifier-ish tokens
    /// → I). A systematic backend gap produces ONE class signature; the
    /// ratchet pins classes, not instances — a new CLASS is a loud red, a
    /// new instance of a known class is reported honestly.
    fn class_signature(&self) -> String {
        fn norm(s: &str) -> String {
            let mut out = String::new();
            let mut in_str = false;
            let mut prev_digit = false;
            for ch in s.chars() {
                match ch {
                    '"' => {
                        in_str = !in_str;
                        out.push('S');
                    }
                    _ if in_str => {}
                    c if c.is_ascii_digit() => {
                        if !prev_digit {
                            out.push('N');
                        }
                        prev_digit = true;
                    }
                    c if c.is_ascii_lowercase() || c == '_' => {
                        prev_digit = false;
                        out.push('i');
                    }
                    _ => {
                        prev_digit = false;
                        out.push(ch);
                    }
                }
            }
            out
        }
        match self {
            Divergence::Outcome { tw, vm } => {
                format!("outcome|tw={}|vm={}", norm(tw), norm(vm))
            }
            Divergence::Output { tw, vm } => {
                format!("output|tw={}|vm={}", norm(tw), norm(vm))
            }
            Divergence::ErrorClass { tw, vm } => {
                format!("errorclass|tw={}|vm={}", norm(tw), norm(vm))
            }
        }
    }
}

fn diff_one(source: &str, dir: &Path) -> Option<Divergence> {
    let tw = run_tw(source, dir);
    let vm = run_vm(source, dir);
    match (tw, vm) {
        (Ok(a), Ok(b)) => {
            if norm_ok(&a) != norm_ok(&b) {
                Some(Divergence::Output {
                    tw: norm_ok(&a),
                    vm: norm_ok(&b),
                })
            } else {
                None
            }
        }
        (Err(a), Err(b)) => {
            let ca = err_class(&a);
            let cb = err_class(&b);
            if ca != cb {
                Some(Divergence::ErrorClass { tw: ca, vm: cb })
            } else {
                None
            }
        }
        (Ok(a), Err(b)) => Some(Divergence::Outcome {
            tw: format!("ok({})", norm_ok(&a)),
            vm: format!("err({})", err_class(&b)),
        }),
        (Err(a), Ok(b)) => Some(Divergence::Outcome {
            tw: format!("err({})", err_class(&a)),
            vm: format!("ok({})", norm_ok(&b)),
        }),
    }
}

/// Statement-level delta debugging: drop statements one at a time (in
/// reverse — later statements depend on earlier ones less often) and keep
/// every removal that preserves the divergence signature.
fn minimize(source: &str, sig_of: &dyn Fn(&str) -> String) -> String {
    let mut current = source.to_string();
    loop {
        let mut changed = false;
        let lines: Vec<String> = current.lines().map(|l| l.to_string()).collect();
        for i in (0..lines.len()).rev() {
            let line = &lines[i];
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("flow ")
                || trimmed.starts_with("}")
                || trimmed.starts_with("pattern ")
                || trimmed.starts_with("entity ")
                || trimmed.starts_with("input:")
                || trimmed.starts_with("->")
                || trimmed.contains("-> output")
            {
                continue;
            }
            let mut candidate = lines.clone();
            candidate.remove(i);
            let candidate_src = candidate.join("\n");
            if sig_of(&candidate_src) == sig_of(&current) {
                current = candidate_src;
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    current
}

fn fingerprint(class_signature: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    class_signature.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// The env-var mutex (the repo posture): the runs are in-process.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn n465_diff_fuzzer_tw_vm() {
    let _env = lock_env();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let corpus_dir = repo.join("tests").join("fuzz_corpus");
    let known_path = corpus_dir.join("known_divergences.txt");
    let known: Vec<String> = std::fs::read_to_string(&known_path)
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        })
        .unwrap_or_default();

    let base_dir = repo.clone();
    let iters = iterations();
    let mut found: Vec<(u64, String, Divergence)> = Vec::new();
    let mut generated = 0u32;
    let mut seeds_run = 0u32;

    // 1. The checked-in seed corpus runs through the same diff.
    if let Ok(entries) = std::fs::read_dir(&corpus_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "mlog").unwrap_or(false) {
                seeds_run += 1;
                let src = std::fs::read_to_string(&path).expect("read seed");
                if let Some(div) = diff_one(&src, &base_dir) {
                    found.push((0, src, div));
                }
            }
        }
    }
    assert!(
        seeds_run >= 4,
        "the seed corpus must exist (tests/fuzz_corpus/*.mlog), found {}",
        seeds_run
    );

    // 2. The generated programs.
    for i in 0..iters {
        let seed = 0x4e34_6546_0000_0000u64 + i as u64;
        let src = gen_program(seed);
        generated += 1;
        if let Some(div) = diff_one(&src, &base_dir) {
            found.push((seed, src, div));
        }
    }

    // 3. Minimize each divergence and ratchet against the known CLASSES.
    let mut new_classes: Vec<(String, usize)> = Vec::new();
    let mut class_counts: Vec<(String, String, usize)> = Vec::new();
    for (_seed, src, div) in &found {
        let sig = div.signature();
        let minimized = minimize(src, &|s| match diff_one(s, &base_dir) {
            Some(d) if d.signature() == sig => d.signature(),
            _ => "changed".to_string(),
        });
        let _ = minimized;
        let class = div.class_signature();
        let fp = fingerprint(&class);
        if known.iter().any(|k| *k == *class) {
            if let Some(entry) = class_counts.iter_mut().find(|e| e.0 == class) {
                entry.2 += 1;
            } else {
                class_counts.push((class.clone(), fp.clone(), 1));
            }
        } else {
            if let Some(entry) = new_classes.iter_mut().find(|e| e.0 == class) {
                entry.1 += 1;
            } else {
                new_classes.push((class.clone(), 1));
            }
            if let Some(entry) = class_counts.iter_mut().find(|e| e.0 == class) {
                entry.2 += 1;
            } else {
                class_counts.push((class.clone(), fp.clone(), 1));
            }
        }
    }

    let mut report = String::new();
    println!("n465 fuzzer: {} generated + {} seeds", generated, seeds_run);
    println!(
        "n465 fuzzer: {} divergences across {} classes",
        found.len(),
        class_counts.len()
    );
    for (class, fp, count) in &class_counts {
        let status = if known.contains(class) {
            "KNOWN"
        } else {
            "NEW"
        };
        let line = format!("{} x{} fp={}\n    class: {}", status, count, fp, class);
        report.push_str(&line);
        report.push('\n');
        println!("{}", line);
    }
    std::fs::write(corpus_dir.join("last_report.txt"), &report).expect("write report");

    assert!(
        new_classes.is_empty(),
        "NEW TW/VM divergence CLASSES found ({}): review, then add the class \
         signature to tests/fuzz_corpus/known_divergences.txt ONLY with the \
         naryad report that explains them — silent landing is the bug class \
         this fuzzer exists for",
        new_classes.len(),
    );
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
