// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

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
//
// ── №479 (gh#727): the fuzzer v2 — codes, AST view, stateful generation ──
//
// The audit 26.09 §3.4 finding: the fuzzer is right in approach but
// NARROW and with an UNREVIEWABLE corpus. Three repairs, all in this
// file:
//   1. The class signature = the STABLE ERROR CODE (the `[CODE] ` origin
//      stamp, ADR-0131/ADR-0169 loud format; errors without a code were
//      assigned stable codes FIRST — UNDEFINED_VARIABLE /
//      UNDEFINED_FUNCTION / TYPE_MISMATCH at the origin sites of BOTH
//      backends) + the AST NODE-KIND VIEW of the minimized program (the
//      sorted set of node kinds). The old normalization collapsed every
//      error to the same `iii: iii iiiiiiii` shape — a human could not
//      read what is allowed, and a new divergence of the same shape was
//      absorbed. A coded signature is readable and wording-proof.
//   2. The corpus is HUMAN-READABLE and machine-checked so: every
//      signature line in `known_divergences.txt` must carry a comment
//      block with `# class:`, `# example:` (a checked-in minimal program
//      that REPRODUCES the class every run) and `# status:` (open/closed
//      by <naryad>). The example is load-bearing: if it stops
//      reproducing the class, the gap is CLOSED and the run fails
//      demanding the line + the example be removed in the fix PR — the
//      "class disappeared = gap closed" ratchet now self-enforces.
//   3. The generator is STATEFUL: a db declaration (in-memory SQLite),
//      query/db_execute, call_llm (the deterministic mock — no network),
//      and try smtp_send (the deterministic config-refusal — the honest
//      MockSmtp posture: no SMTP env in tests, so no real connection is
//      ever attempted). Deterministic by seed; minimization preserved.
//      The duplication zones (DB, memory, mail, LLM) are now exercised
//      by the fuzzer itself — a backend divergence there is caught HERE,
//      not only by unit tests (the audit: the query_row divergence was
//      found by a unit test, not the fuzzer).

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
                    "each item in [\"a\", \"b\", \"c\"] {{\n  let acc = len(item)\n}}".to_string(),
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
///
/// №479: the program is STATEFUL — an in-memory-SQLite db declaration +
/// a third pattern (pc) that exercises query/db_execute (the №474 ONE
/// db_execute contract), call_llm (the deterministic mock — default
/// METALOGOS_MOCK_LLM=1 set explicitly, no network) and try smtp_send (the
/// deterministic SMTP config-refusal — no SMTP env in tests, so the
/// loud refusal IS the MockSmtp, no connection is ever attempted).
/// Everything is deterministic per seed; minimization is preserved.
fn gen_program(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut budget = 26u32;
    let mut src = String::new();

    // №479: the db declaration — in-memory SQLite, deterministic, no disk.
    src.push_str("db { url: \"sqlite::memory:\" }\n");

    // Entities (top-level initializers).
    for i in 0..2 {
        src.push_str(&format!(
            "entity e{}: String = \"{}\"\n",
            i,
            rng.alnum(5 + i)
        ));
    }

    // №510: the seeded rule — a float entity vs a float threshold joined by
    // one of the six comparison operators (incl. `!=`, the C-01 operator
    // the compiler wildcard used to mis-compile to Eq). Previously the
    // generator produced ZERO rules, so the whole rule path was invisible
    // to the diff harness. The rule outcome (fzr.flag) rides the program
    // output through pfzf at the flow head, so any TW/VM disagreement on
    // the condition evaluation is a divergence. Deterministic per seed.
    const RULE_OPS: &[&str] = &["!=", "==", ">", "<", ">=", "<="];
    let rule_op = rng.pick(RULE_OPS);
    let rule_value = 1.0 + (seed % 7) as f64;
    let rule_threshold = (seed % 10) as f64;
    src.push_str("entity FzFlag { flag: Float }\n");
    src.push_str("entity fzr: FzFlag = { flag: 0.0 }\n");
    src.push_str(&format!("entity fzn: Float = {}\n", rule_value));
    src.push_str(&format!(
        "rule If(fzn {} {}) then fzr.flag = 1.0\n",
        rule_op, rule_threshold
    ));

    // Pattern A: pure transformation (calls the string/math ops).
    // №510: pfzf heads the pipeline — it renders the rule-written flag
    // (flow-input binding reads fzr.flag AFTER the rules execute) so the
    // rule outcome is part of the diffed output on both backends.
    src.push_str("\npattern pfzf(x: Float) -> String { return \"flag:\" + to_string(x) }\n");
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

    // Pattern C: the №479 stateful group — db + LLM mock + SMTP refusal.
    // Table name is seed-derived (kept short so minimization can keep it).
    let table = format!("fz{}", seed % 100_000);
    src.push_str("\npattern pc(z: String) -> String {\n");
    src.push_str(&format!(
        "  let created = db_execute(\"CREATE TABLE IF NOT EXISTS {table} (k TEXT, v TEXT)\", [])\n"
    ));
    src.push_str(&format!(
        "  let ins = db_execute(\"INSERT INTO {table} VALUES ($1, $2)\", [z, \"{}\"])\n",
        rng.alnum(4)
    ));
    src.push_str(&format!(
        "  let sel = query(\"SELECT v FROM {table} WHERE k = $1\", [z])\n"
    ));
    src.push_str("  let n_rows = len(sel)\n");
    // call_llm: no SmartRouter in the test process + the explicit
    // METALOGOS_MOCK_LLM=1 (env hygiene above) → the deterministic mock
    // answer, no network.
    src.push_str("  let answer = call_llm(\"summarize\", z)\n");
    // try smtp_send: without SMTP_HOST/SMTP_USER/SMTP_PASS the builtin
    // refuses LOUDLY and identically on both backends — the deterministic
    // MockSmtp. try (ADR-0142) keeps the program running; the refusal
    // classifies to the honest RUNTIME_ERROR fallback (ADR-0169).
    src.push_str("  let mail = try smtp_send(\"ops@example.test\", \"fuzz report\", z)\n");
    src.push_str("  let mail_state = if mail.ok then \"sent\" else \"refused\"\n");
    src.push_str(
        "  return to_string(n_rows) + \":\" + answer + \":\" + mail_state + \":\" + to_string(len(mail.error.code))\n}\n",
    );

    // The flow: fzr.flag → pfzf → pa → pb → pc → output. The input binds
    // the RULE-WRITTEN flag (№510): rules execute before the input binding
    // (the p42 contracts), so pfzf's rendering reflects the rule outcome.
    // Both backends run the same wiring.
    src.push_str(
        "\nflow Main {\n  input: Float = fzr.flag -> pfzf -> pa -> pb -> pc -> output\n}\n",
    );
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

/// №479: the STABLE CODE of an error — the `[CODE]` origin stamp at
/// position 0 (the ADR-0131/ADR-0169 loud format, generalizing №254).
/// Position 0 ONLY: a `[CODE]`-looking substring mid-message is program
/// content, not a stamp — a program cannot forge its class by echoing a
/// marker into its own strings (the split_origin_stamp protocol).
fn err_code(err: &str) -> Option<String> {
    let rest = err.strip_prefix('[')?;
    let close = rest.find(']')?;
    let code = &rest[..close];
    if code.is_empty()
        || !code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
    {
        return None;
    }
    Some(code.to_string())
}

/// №479: the normalized SHAPE of a value — digits → N, quoted strings → S,
/// lowercase runs → i. The shape is seed-stable: the same class keeps the
/// same shape across programs.
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

/// №479: the AST NODE KIND the error was born at — the deterministic
/// construct map of the origin sites (the stable-code sites above are
/// pinned to these kinds by the n479 pins). The class signature =
/// code + node kind: the same code on a different construct is a
/// different class, and the reviewer sees the SHAPE, not a normalized
/// blob. This is the fuzzer-side view of the divergence point; the full
/// program's node-kind set stays in the report (ast_view) for reviewers.
fn err_node(err: &str) -> &'static str {
    if err.contains("undefined function")
        || err.contains("unknown function")
        || err.contains("() expected")
        || err.contains("() requires")
    {
        "fn_call"
    } else if err.contains("undefined variable") {
        "ident"
    } else if err.contains("each:") {
        "each"
    } else if err.contains("concatenation") || err.contains("binary operation") {
        "binary_op"
    } else {
        "other"
    }
}

/// One backend's outcome, carrying the RAW error text when the backend
/// refused (№479: the class signature needs the origin stamp, the report
/// needs the raw text a human can read).
#[derive(Debug, Clone)]
enum Side {
    Ok(String),
    Err(String),
}

impl Side {
    /// The readable, wording-proof side repr: `ok(<shape>)` for success,
    /// `err:[CODE]@<node>` for a stamped refusal, `err:uncoded(<shape>)@<node>`
    /// for an unstamped one — the honest gap that fails loudly until the
    /// origin site carries its stable code.
    fn repr(&self) -> String {
        match self {
            Side::Ok(v) => format!("ok({})", norm(v)),
            Side::Err(e) => {
                let node = err_node(e);
                match err_code(e) {
                    Some(c) => format!("err:[{}]@{}", c, node),
                    None => format!(
                        "err:uncoded({})@{}",
                        norm(&e.chars().take(48).collect::<String>()),
                        node
                    ),
                }
            }
        }
    }

    /// The raw text for the report (a human reads the RIGHT side here).
    fn raw(&self) -> String {
        match self {
            Side::Ok(v) => format!("ok: {}", v),
            Side::Err(e) => format!("err: {}", e),
        }
        .chars()
        .take(220)
        .collect()
    }
}

#[derive(Debug)]
enum Divergence {
    /// One backend ok, the other refused.
    Outcome { tw: Side, vm: Side },
    /// Both ok, different stdout.
    Output { tw: String, vm: String },
    /// Both refused, different error classes.
    ErrorClass { tw: String, vm: String },
}

impl Divergence {
    /// The RAW signature (divergence identity for minimization): the
    /// un-normalized sides. Statement-level delta debugging preserves it.
    fn signature(&self) -> String {
        match self {
            Divergence::Outcome { tw, vm } => {
                format!("outcome|tw={:?}|vm={:?}", tw.raw(), vm.raw())
            }
            Divergence::Output { tw, vm } => format!("output|tw={}|vm={}", tw, vm),
            Divergence::ErrorClass { tw, vm } => format!("errorclass|tw={}|vm={}", tw, vm),
        }
    }

    /// №479: the RATCHET signature — the class a human reads and CI pins.
    ///
    /// `errorclass|tw=err:[CODE_A]@node_a|vm=err:[CODE_B]@node_b` — the
    /// stable error codes (ADR-0131) PLUS the AST node kind each error
    /// was born at: the same code pair on a DIFFERENT construct is a
    /// DIFFERENT class. The old normalization collapsed every error to
    /// one unreadable `iii: iii iiiiiiii` shape; a coded, node-tagged
    /// signature is readable and wording-proof.
    fn class_signature(&self) -> String {
        match self {
            Divergence::Outcome { tw, vm } => {
                format!("outcome|tw={}|vm={}", tw.repr(), vm.repr())
            }
            Divergence::Output { tw, vm } => {
                format!("output|tw=ok({})|vm=ok({})", norm(tw), norm(vm))
            }
            Divergence::ErrorClass { tw, vm } => format!(
                "errorclass|tw={}|vm={}",
                // ErrorClass carries the raw error texts.
                Side::Err(tw.clone()).repr(),
                Side::Err(vm.clone()).repr(),
            ),
        }
    }

    /// The first raw side pair for the report (the human-readable right
    /// side the audit demanded — what is actually allowed and what broke).
    fn raw_sides(&self) -> (String, String) {
        match self {
            Divergence::Outcome { tw, vm } => (tw.raw(), vm.raw()),
            Divergence::Output { tw, vm } => (format!("ok: {}", tw), format!("ok: {}", vm)),
            Divergence::ErrorClass { tw, vm } => (format!("err: {}", tw), format!("err: {}", vm)),
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
                Some(Divergence::ErrorClass { tw: a, vm: b })
            } else {
                None
            }
        }
        (Ok(a), Err(b)) => Some(Divergence::Outcome {
            tw: Side::Ok(norm_ok(&a)),
            vm: Side::Err(b),
        }),
        (Err(a), Ok(b)) => Some(Divergence::Outcome {
            tw: Side::Err(a),
            vm: Side::Ok(norm_ok(&b)),
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

// ── №476 (gh#724): the BLOCKED-DOMAIN rule ─────────────────────────
// Classes touching SQL, filesystem, network, exec, labels, or secrets
// are BLOCKING correctness errors — pinning them as known is
// FORBIDDEN. Three enforcement points, all in this file:
//   1. corpus parse — a `blocked-`-prefixed line fails IMMEDIATELY with
//      the line named;
//   2. corpus parse — an unprefixed line is read as `known-` (back-compat);
//   3. run time — a divergence whose RAW text carries a blocked-domain
//      marker NEVER matches as known: the run fails naming the class,
//      even if someone pinned it in the corpus file.

/// The blocked domains (the audit 26.09 §3.2/§3.4 shelter class) and the
/// raw-text markers that detect them. The markers are deliberately
/// distinctive — the corpus holds language-core classes, and the fuzzer
/// generator never produces state calls, so a marker hit means the
/// divergence really is about state.
const BLOCKED_DOMAINS: &[(&str, &[&str])] = &[
    (
        "SQL",
        &[
            "sql",
            "database",
            "sqlite",
            "db_execute",
            "db_insert",
            "query_row",
            "query_scalar",
        ],
    ),
    (
        "FS",
        &[
            "read_file",
            "write_file",
            "fs_gate",
            "filesystem",
            "file_path",
        ],
    ),
    ("NET", &["http", "socket", "tcp", "reqwest", "network"]),
    ("EXEC", &["exec(", "subprocess", "shell", "command::new"]),
    ("LABEL", &["taint", "label_env", "label:"]),
    ("SECRET", &["secret", "api_key", "credential"]),
];

impl Divergence {
    /// The blocked domain of this divergence (from the RAW sides — the
    /// coded class signature masks the keywords by design).
    fn blocked_domain(&self) -> Option<&'static str> {
        let hay = match self {
            Divergence::Outcome { tw, vm } => format!("{}\n{}", tw.raw(), vm.raw()),
            Divergence::Output { tw, vm } => format!("{}\n{}", tw, vm),
            Divergence::ErrorClass { tw, vm } => format!("{}\n{}", tw, vm),
        }
        .to_lowercase();
        for (domain, markers) in BLOCKED_DOMAINS {
            if markers.iter().any(|m| hay.contains(m)) {
                return Some(domain);
            }
        }
        None
    }
}

/// The env-var mutex (the repo posture): the runs are in-process.
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ── №479: the AST NODE-KIND VIEW ───────────────────────────────────────
//
// The class signature carries the sorted, deduplicated set of AST node
// kinds (declarations, statements, expressions) the MINIMIZED program
// contains. Deterministic and human-readable: `ast=binary_op,db,entity_simple,flow,fn_call,let_binding,pattern,return`
// tells the reviewer the SHAPE of the program the class lives on — the
// same code pair on a different shape is a different class.

type Kinds = std::collections::BTreeSet<String>;

fn walk_stmts(stmts: &[metalogos::ast::Statement], kinds: &mut Kinds) {
    use metalogos::ast::Statement as S;
    for s in stmts {
        match s {
            S::LetBinding { value, .. } => {
                kinds.insert("let_binding".to_string());
                walk_expr(value, kinds);
            }
            S::Assign { value, .. } => {
                kinds.insert("assign".to_string());
                walk_expr(value, kinds);
            }
            S::Each { iterable, body, .. } | S::EachWithIndex { iterable, body, .. } => {
                kinds.insert("each".to_string());
                walk_expr(iterable, kinds);
                walk_stmts(body, kinds);
            }
            S::While {
                condition, body, ..
            } => {
                kinds.insert("while".to_string());
                walk_expr(condition, kinds);
                walk_stmts(body, kinds);
            }
            S::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                kinds.insert("if_else_block".to_string());
                walk_expr(condition, kinds);
                walk_stmts(then_body, kinds);
                for (cond, body) in else_ifs {
                    walk_expr(cond, kinds);
                    walk_stmts(body, kinds);
                }
                if let Some(b) = else_body {
                    walk_stmts(b, kinds);
                }
            }
            S::IfThen {
                condition, body, ..
            } => {
                kinds.insert("if_then".to_string());
                walk_expr(condition, kinds);
                walk_stmts(body, kinds);
            }
            S::Return { value, .. } => {
                kinds.insert("return".to_string());
                walk_expr(value, kinds);
            }
            S::ExprStmt { expr, .. } => {
                kinds.insert("expr_stmt".to_string());
                walk_expr(expr, kinds);
            }
            S::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                kinds.insert("match".to_string());
                walk_expr(scrutinee, kinds);
                for arm in arms {
                    walk_stmts(arm.body(), kinds);
                }
                if let Some(b) = else_body {
                    walk_stmts(b, kinds);
                }
            }
            S::Break | S::Continue => {
                kinds.insert("loop_control".to_string());
            }
            // Statement-shaped declarations (meta-blocks; the fuzzer's
            // generated programs never produce them) contribute their kind.
            S::Memorize(_) => {
                kinds.insert("memorize".to_string());
            }
            S::Forget(_) => {
                kinds.insert("forget".to_string());
            }
            S::Relate(_) => {
                kinds.insert("relate".to_string());
            }
        }
    }
}

fn walk_expr(e: &metalogos::ast::Expr, kinds: &mut Kinds) {
    use metalogos::ast::Expr as E;
    match e {
        E::StringLit { .. } => {
            kinds.insert("string_lit".to_string());
        }
        E::FloatLit { .. } => {
            kinds.insert("float_lit".to_string());
        }
        E::BoolLit { .. } => {
            kinds.insert("bool_lit".to_string());
        }
        E::Ident { .. } => {
            kinds.insert("ident".to_string());
        }
        E::FieldAccess { object, .. } => {
            kinds.insert("field_access".to_string());
            walk_expr(object, kinds);
        }
        E::FnCall { args, .. } => {
            kinds.insert("fn_call".to_string());
            for a in args {
                walk_expr(a, kinds);
            }
        }
        E::QualifiedCall { args, .. } => {
            kinds.insert("fn_call".to_string());
            for a in args {
                walk_expr(a, kinds);
            }
        }
        E::BinaryOp { left, right, .. } => {
            kinds.insert("binary_op".to_string());
            walk_expr(left, kinds);
            walk_expr(right, kinds);
        }
        E::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            kinds.insert("if_else".to_string());
            walk_expr(condition, kinds);
            walk_expr(then_branch, kinds);
            walk_expr(else_branch, kinds);
        }
        E::List { items, .. } => {
            kinds.insert("list".to_string());
            for i in items {
                walk_expr(i, kinds);
            }
        }
        E::IndexAccess { object, index, .. } => {
            kinds.insert("index_access".to_string());
            walk_expr(object, kinds);
            walk_expr(index, kinds);
        }
        E::StructLit { fields, .. } => {
            kinds.insert("struct_lit".to_string());
            for v in fields.values() {
                walk_expr(v, kinds);
            }
        }
        E::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            kinds.insert("if_else_block".to_string());
            walk_expr(condition, kinds);
            walk_stmts(then_body, kinds);
            for (cond, body) in else_ifs {
                walk_expr(cond, kinds);
                walk_stmts(body, kinds);
            }
            if let Some(b) = else_body {
                walk_stmts(b, kinds);
            }
        }
        E::MatchExpr {
            scrutinee, arms, ..
        } => {
            kinds.insert("match".to_string());
            walk_expr(scrutinee, kinds);
            for arm in arms {
                walk_stmts(arm.body(), kinds);
            }
        }
        E::Try { expr, .. } => {
            kinds.insert("try".to_string());
            walk_expr(expr, kinds);
        }
        E::HandleSource { .. } => {
            kinds.insert("handle_source".to_string());
        }
        E::ProvBind { inner, .. } => {
            kinds.insert("prov_bind".to_string());
            walk_expr(inner, kinds);
        }
    }
}

/// The AST node-kind view of a program (sorted, `+`-joined). Unparsable
/// sources view as `unparsed` — an honest class on its own.
fn ast_view(source: &str) -> String {
    let mut kinds: Kinds = Kinds::new();
    match metalogos::parser::parse(source) {
        Ok(decls) => {
            for d in &decls {
                walk_decl(d, &mut kinds);
            }
        }
        Err(_) => {
            kinds.insert("unparsed".to_string());
        }
    }
    if kinds.is_empty() {
        kinds.insert("empty".to_string());
    }
    kinds.into_iter().collect::<Vec<_>>().join("+")
}

fn walk_decl(d: &metalogos::ast::Declaration, kinds: &mut Kinds) {
    use metalogos::ast::Declaration as D;
    // Declarations with executable bodies get walked; the rest contribute
    // their kind (the fuzzer's programs only produce the walked shapes).
    match d {
        D::Pattern(p) => {
            kinds.insert("pattern".to_string());
            walk_stmts(&p.body, kinds);
        }
        D::LearnablePattern(_) => {
            kinds.insert("learnable_pattern".to_string());
        }
        D::Test(t) => {
            kinds.insert("test".to_string());
            walk_stmts(&t.body, kinds);
        }
        D::Hook(h) => {
            kinds.insert("hook".to_string());
            walk_stmts(&h.body, kinds);
        }
        D::OnDeny(o) => {
            kinds.insert("on_deny".to_string());
            walk_stmts(&o.body, kinds);
        }
        D::Flow(f) => {
            kinds.insert("flow".to_string());
            walk_expr(&f.source, kinds);
        }
        D::EntitySimple(e) => {
            kinds.insert("entity_simple".to_string());
            walk_expr(&e.value, kinds);
        }
        D::Db(_) => {
            kinds.insert("db".to_string());
        }
        D::MlogServer(_) => {
            kinds.insert("mlog_server".to_string());
        }
        D::Template(_) => {
            kinds.insert("template".to_string());
        }
        D::Schema(_) => {
            kinds.insert("schema".to_string());
        }
        D::SkillIndex(_) => {
            kinds.insert("skill_index".to_string());
        }
        D::Memory(_) => {
            kinds.insert("memory".to_string());
        }
        D::Import(_) => {
            kinds.insert("import".to_string());
        }
        D::EntityType(_) => {
            kinds.insert("entity_type".to_string());
        }
        D::EntityRecord(_) => {
            kinds.insert("entity_record".to_string());
        }
        D::Rule(_) => {
            kinds.insert("rule".to_string());
        }
        D::Memorize(_) => {
            kinds.insert("memorize".to_string());
        }
        D::Forget(_) => {
            kinds.insert("forget".to_string());
        }
        D::Fluid(_) => {
            kinds.insert("fluid".to_string());
        }
        D::Adapt(_) => {
            kinds.insert("adapt".to_string());
        }
        D::Relate(_) => {
            kinds.insert("relate".to_string());
        }
        D::Sandbox(_) => {
            kinds.insert("sandbox".to_string());
        }
        D::Mutate(_) => {
            kinds.insert("mutate".to_string());
        }
        D::Eval(_) => {
            kinds.insert("eval".to_string());
        }
        D::Conversation(_) => {
            kinds.insert("conversation".to_string());
        }
        D::Tool(_) => {
            kinds.insert("tool".to_string());
        }
        D::LlmConfig(_) => {
            kinds.insert("llm_config".to_string());
        }
        D::ContextBudget(_) => {
            kinds.insert("context_budget".to_string());
        }
        D::TypeAlias(_) => {
            kinds.insert("type_alias".to_string());
        }
        D::Reflex(_) => {
            kinds.insert("reflex".to_string());
        }
        D::ReflexSeq(_) => {
            kinds.insert("reflex_seq".to_string());
        }
        D::ReflexGen(_) => {
            kinds.insert("reflex_gen".to_string());
        }
        D::Vision(_) => {
            kinds.insert("vision".to_string());
        }
        D::Profile(_) => {
            kinds.insert("profile".to_string());
        }
        D::Origin(_) => {
            kinds.insert("origin".to_string());
        }
    }
}

#[test]
fn n465_diff_fuzzer_tw_vm() {
    let _env = lock_env();
    // №479 env hygiene: the stateful generation relies on the DETERMINISTIC
    // postures — no SMTP config (the loud refusal IS the MockSmtp; a real
    // connection must never fire in tests), the LLM mock contour EXPLICIT
    // (№478: METALOGOS_MOCK_LLM=1 — the SSOT predicate; the default is the
    // loud real path), no fault injection, no trace file. Set/remove under
    // ENV_LOCK, restore nothing (test process).
    std::env::remove_var("SMTP_HOST");
    std::env::remove_var("SMTP_USER");
    std::env::remove_var("SMTP_PASS");
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    std::env::remove_var("METALOGOS_MOCK_LLM_FAULT");
    std::env::remove_var("METALOGOS_LLM_TRACE");

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let corpus_dir = repo.join("tests").join("fuzz_corpus");
    let known_path = corpus_dir.join("known_divergences.txt");
    // №476: the corpus lines carry the known-/blocked- prefix. A
    // `blocked-` line fails IMMEDIATELY with the line named — there is
    // nothing to pin in a blocked domain (SQL/FS/NET/EXEC/LABEL/SECRET);
    // an unprefixed line is read as known- (back-compat with the pre-
    // №476 corpus).
    // №479: every signature line must ALSO carry a readable comment
    // block — `# class:`, `# example:` (a checked-in minimal program),
    // `# status:` (open / closed by <naryad>) — enforced BELOW by
    // require_readable_block; the corpus a human cannot review is the
    // defect the audit named.
    let corpus_text = std::fs::read_to_string(&known_path).unwrap_or_default();
    let corpus_lines: Vec<String> = corpus_text.lines().map(|l| l.trim().to_string()).collect();
    let mut known: Vec<String> = Vec::new();
    // (class signature → example file name) — the load-bearing examples.
    let mut example_of: Vec<(String, String)> = Vec::new();
    for (idx, line) in corpus_lines.iter().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("blocked-") {
            panic!(
                "BLOCKED-DOMAIN class pinned in known_divergences.txt: \"blocked-{}\" — \
                 classes touching SQL/FS/NET/EXEC/LABEL/SECRET are blocking errors \
                 (№476): fix them in a repair naryad, never pin them",
                rest
            );
        }
        let sig = match line.strip_prefix("known-") {
            Some(rest) => rest.to_string(),
            None => line.clone(),
        };
        let (class_desc, example, status) = require_readable_block(&corpus_lines, idx, &corpus_dir);
        println!(
            "n465 corpus line {}: {} | class: {} | status: {} | example: {}",
            idx + 1,
            sig,
            class_desc,
            status,
            example
        );
        known.push(sig.clone());
        example_of.push((sig, example));
    }

    let base_dir = repo.clone();
    let iters = iterations();
    // (seed, source, divergence, from_file) — from_file names the checked-in
    // seed/example that produced the divergence (None for generated programs);
    // the №479 example-reproduces-class ratchet reads it.
    let mut found: Vec<(u64, String, Divergence, Option<String>)> = Vec::new();
    let mut generated = 0u32;
    let mut seeds_run = 0u32;

    // 1. The checked-in seed corpus runs through the same diff.
    if let Ok(entries) = std::fs::read_dir(&corpus_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "mlog").unwrap_or(false) {
                seeds_run += 1;
                let src = std::fs::read_to_string(&path).expect("read seed");
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                if let Some(div) = diff_one(&src, &base_dir) {
                    found.push((0, src, div, name));
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
            found.push((seed, src, div, None));
        }
    }

    // 3. Minimize each divergence and ratchet against the known CLASSES.
    //    №479: the class signature = the stable codes + the AST node-kind
    //    view of the MINIMIZED program; the minimized catch is kept in
    //    target/fuzz_min/ for NEW classes (the curation source for the
    //    corpus example files).
    let min_dir = repo.join("target").join("fuzz_min");
    let _ = std::fs::create_dir_all(&min_dir);
    let mut new_classes: Vec<(String, usize)> = Vec::new();
    let mut class_counts: Vec<(String, String, usize, String, String, String)> = Vec::new();
    // №479: class → its checked-in example reproduced it in THIS run.
    let mut example_reproduced: std::collections::BTreeMap<String, bool> =
        std::collections::BTreeMap::new();
    for (_seed, src, div, from_file) in &found {
        let sig = div.signature();
        let minimized = minimize(src, &|s| match diff_one(s, &base_dir) {
            Some(d) if d.signature() == sig => d.signature(),
            _ => "changed".to_string(),
        });
        // №479: the class = stable codes + node kinds (from the raw sides);
        // the full program's AST view stays in the report for reviewers.
        let class = div.class_signature();
        let fp = fingerprint(&class);
        // №476: a blocked-domain divergence NEVER matches as known —
        // the run fails naming the class, even if the corpus carries it.
        // The blocked domains are the audit 26.09 shelter class: a
        // correctness defect in SQL/FS/NET/EXEC/LABEL/SECRET is repaired
        // by a naryad, never pinned.
        if let Some(domain) = div.blocked_domain() {
            panic!(
                "BLOCKED-DOMAIN TW/VM divergence ({} domain) — blocking, not pinnable (№476): \
                 repair it in a naryad, never add it to known_divergences.txt. \
                 Class: {} | raw: {}",
                domain,
                class,
                div.signature()
            );
        }
        let is_known = known.iter().any(|k| *k == *class);
        if let Some(fname) = from_file {
            // The class's OWN example fired — the load-bearing guarantee.
            if example_of.iter().any(|(c, e)| *c == *class && *e == *fname) {
                example_reproduced.insert(class.clone(), true);
            }
        }
        if is_known {
            if let Some(entry) = class_counts.iter_mut().find(|e| e.0 == class) {
                entry.2 += 1;
            } else {
                let (rtw, rvm) = div.raw_sides();
                class_counts.push((class.clone(), fp.clone(), 1, rtw, rvm, ast_view(&minimized)));
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
                let (rtw, rvm) = div.raw_sides();
                class_counts.push((class.clone(), fp.clone(), 1, rtw, rvm, ast_view(&minimized)));
                // Keep the minimized catch for curation (the future
                // `# example:` file of the class).
                let catch = min_dir.join(format!("min_{}.mlog", &fp[..8.min(fp.len())]));
                let _ = std::fs::write(&catch, &minimized);
            }
        }
    }

    let mut report = String::new();
    println!(
        "n465 fuzzer: {} generated + {} seeds (№479: stateful db/llm/mail group live)",
        generated, seeds_run
    );
    println!(
        "n465 fuzzer: {} divergences across {} classes",
        found.len(),
        class_counts.len()
    );
    for (class, fp, count, rtw, rvm, ast) in &class_counts {
        let status = if known.contains(class) {
            "KNOWN"
        } else {
            "NEW"
        };
        let line = format!(
            "{} x{} fp={}\n    class: {}\n    ast: {}\n    raw tw: {}\n    raw vm: {}",
            status, count, fp, class, ast, rtw, rvm
        );
        report.push_str(&line);
        report.push('\n');
        println!("{}", line);
    }
    std::fs::write(corpus_dir.join("last_report.txt"), &report).expect("write report");

    // №479: the EXAMPLE-REPRODUCES-CLASS ratchet — each known class's
    // checked-in example must have reproduced ITS class in THIS run (the
    // seed loop runs every corpus .mlog, examples included; the class
    // firing from generated programs or other seeds does NOT count — the
    // example is the deterministic, load-bearing reproducer). A class the
    // example no longer reproduces is a CLOSED gap: the run fails
    // demanding the line and the example be removed in the fix PR — the
    // corpus never outlives the divergence it pins.
    for (class, example) in &example_of {
        if !example_reproduced.get(class).copied().unwrap_or(false) {
            panic!(
                "KNOWN class no longer reproduces via its example — the gap is CLOSED \
                 (№479 ratchet): remove the known- line AND its example {} from the \
                 corpus in the fix PR. Class: {}",
                example, class
            );
        }
    }

    assert!(
        new_classes.is_empty(),
        "NEW TW/VM divergence CLASSES found ({}): review, then add the class \
         signature to tests/fuzz_corpus/known_divergences.txt ONLY with the \
         naryad report that explains them, a readable comment block \
         (# class:/# example:/# status:) and a checked-in minimal example \
         program — silent landing is the bug class this fuzzer exists for. \
         The minimized catch is in target/fuzz_min/. NOTE (№476): classes \
         touching SQL/FS/NET/EXEC/LABEL/SECRET are BLOCKING — never pin \
         them; repair them in a naryad. NOTE (№479): `err:uncoded(...)` \
         sides mean the origin site lacks its stable ADR-0131 code — \
         assign the code first, then re-pin",
        new_classes.len(),
    );
}

/// №479: the corpus READABILITY self-check — every signature line must be
/// preceded by a contiguous comment block carrying `# class:` (the human
/// description), `# example:` (a checked-in minimal program in the corpus
/// dir) and `# status:` (open/closed by <naryad-or-issue>). Returns the
/// parsed triple; panics loudly naming the line otherwise.
fn require_readable_block(
    lines: &[String],
    sig_idx: usize,
    corpus_dir: &Path,
) -> (String, String, String) {
    let mut block: Vec<&str> = Vec::new();
    let mut i = sig_idx;
    while i > 0 {
        i -= 1;
        let l = lines[i].trim();
        if l.starts_with('#') {
            block.push(l);
        } else {
            break;
        }
    }
    let fail = |why: String| -> ! {
        panic!(
            "CORPUS NOT HUMAN-READABLE (№479) — line {}: {}. Every signature \
             line needs a comment block with `# class:`, `# example:` (a \
             checked-in minimal .mlog that reproduces the class) and \
             `# status:` (open/closed by <naryad>).",
            sig_idx + 1,
            why
        )
    };
    let class_desc = block
        .iter()
        .find_map(|l| l.strip_prefix("# class:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| fail("missing `# class:`".to_string()));
    let example_raw = block
        .iter()
        .find_map(|l| l.strip_prefix("# example:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| fail("missing `# example:`".to_string()));
    let status = block
        .iter()
        .find_map(|l| l.strip_prefix("# status:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| fail("missing `# status:`".to_string()));
    let example_name = example_raw
        .split_whitespace()
        .next()
        .unwrap_or_else(|| fail("empty `# example:`".to_string()));
    let example_path = corpus_dir.join(example_name);
    if !example_path.exists() {
        fail(format!(
            "example file `{}` does not exist in the corpus dir",
            example_name
        ));
    }
    (class_desc, example_name.to_string(), status)
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ── №476: the blocked-domain rule pins ─────────────────────────────

#[test]
fn n476_blocked_domain_is_detected_from_the_raw_text() {
    // SQL markers on either side.
    let div = Divergence::ErrorClass {
        tw: "query_row() error: type mismatch binding $1".to_string(),
        vm: "compile error: undefined function query_row".to_string(),
    };
    assert_eq!(div.blocked_domain(), Some("SQL"));
    // FS.
    let div = Divergence::ErrorClass {
        tw: "read_file() error: permission denied".to_string(),
        vm: "compile error: undefined function read_file".to_string(),
    };
    assert_eq!(div.blocked_domain(), Some("FS"));
    // SECRET.
    let div = Divergence::ErrorClass {
        tw: "api_key is required for anthropic".to_string(),
        vm: "compile error: undefined variable".to_string(),
    };
    assert_eq!(div.blocked_domain(), Some("SECRET"));
    // Language-core errors carry NO blocked domain — the corpus class
    // shapes stay pinnable.
    let div = Divergence::ErrorClass {
        tw: "type mismatch in string concatenation: List + String".to_string(),
        vm: "compile error: undefined function memory_forget".to_string(),
    };
    assert_eq!(div.blocked_domain(), None);
    let div = Divergence::Outcome {
        tw: Side::Ok("flow completed".to_string()),
        vm: Side::Err("run failed: undefined variable e0".to_string()),
    };
    assert_eq!(div.blocked_domain(), None);
}

#[test]
fn n476_corpus_rejects_a_blocked_prefix_line() {
    // The rule the corpus parse enforces: a `blocked-` line is a CI
    // failure naming the line. Pinned here as the executable form of
    // the README rule (the parse itself panics inside the main test —
    // this test pins the same predicate directly).
    let line = "blocked-errorclass|tw=iiii|vm=iiii";
    assert!(line.starts_with("blocked-"));
    let stripped = line.strip_prefix("blocked-").unwrap();
    assert!(stripped.starts_with("errorclass|"));
    // The known- form strips cleanly.
    assert_eq!(
        "known-errorclass|tw=x|vm=y".strip_prefix("known-"),
        Some("errorclass|tw=x|vm=y")
    );
}

// ── №479: the fuzzer-v2 pins ────────────────────────────────────────────

#[test]
fn n479_err_code_reads_position_zero_stamps_only() {
    // A position-0 stamp is the code.
    assert_eq!(
        err_code("[SQL_ERROR] query() failed: syntax"),
        Some("SQL_ERROR".to_string())
    );
    assert_eq!(
        err_code("[UNDEFINED_VARIABLE] undefined variable: x"),
        Some("UNDEFINED_VARIABLE".to_string())
    );
    // A stamp mid-message is CONTENT, not a class (no forging).
    assert_eq!(
        err_code("call failed: the marker [SQL_ERROR] appeared in the text"),
        None
    );
    // Lowercase or empty brackets are not codes.
    assert_eq!(err_code("[sql_error] no"), None);
    assert_eq!(err_code("[] no"), None);
    // Prose errors carry no code.
    assert_eq!(err_code("compile: undefined function: foo"), None);
}

#[test]
fn n479_ast_view_is_deterministic_and_shape_sensitive() {
    let p1 = "pattern pa(x: String) -> String {\n  let y = upper(x)\n  return y\n}\nflow Main {\n  input: String = \"a\" -> pa -> output\n}\n";
    // Deterministic: the same program → the same view.
    assert_eq!(ast_view(p1), ast_view(p1));
    // The view names the node kinds actually present.
    for kind in [
        "pattern",
        "flow",
        "let_binding",
        "return",
        "fn_call",
        "ident",
    ] {
        assert!(
            ast_view(p1).contains(kind),
            "the ast view must contain `{}`: {}",
            kind,
            ast_view(p1)
        );
    }
    // Shape-sensitive: a db declaration changes the view.
    let p2 = format!("db {{ url: \"sqlite::memory:\" }}\n{}", p1);
    assert_ne!(ast_view(p1), ast_view(&p2));
    assert!(ast_view(&p2).contains("db"));
    // An unparsable source is its own honest view.
    assert_eq!(ast_view("this is not ( mlog"), "unparsed");
}

#[test]
fn n479_stateful_generation_is_alive() {
    // The №479 stateful group: every generated program carries the db
    // declaration, the db calls, the deterministic LLM mock call and the
    // try-wrapped SMTP call. The duplication zones (DB, memory, mail,
    // LLM) are exercised by the fuzzer itself — not by unit tests alone.
    for seed in 0..40u64 {
        let src = gen_program(0x4e34_6546_0000_0000u64 + seed);
        assert!(
            src.contains("db { url: \"sqlite::memory:\" }"),
            "seed {}: no db declaration",
            seed
        );
        assert!(src.contains("db_execute("), "seed {}: no db_execute", seed);
        assert!(src.contains("query("), "seed {}: no query", seed);
        assert!(src.contains("call_llm("), "seed {}: no call_llm", seed);
        assert!(
            src.contains("try smtp_send("),
            "seed {}: no try-wrapped smtp_send",
            seed
        );
        assert!(
            src.contains("memory_open("),
            "seed {}: no memory group",
            seed
        );
        // №510: the seeded rule with one of the six comparison operators
        // (incl. the C-01 `!=`) must be present, and the rule-written flag
        // must ride the flow output through pfzf.
        assert!(
            src.contains("rule If(fzn "),
            "seed {}: no seeded comparison rule",
            seed
        );
        assert!(
            src.contains("then fzr.flag = 1.0"),
            "seed {}: the rule must write fzr.flag",
            seed
        );
        assert!(
            src.contains("input: Float = fzr.flag -> pfzf"),
            "seed {}: the rule outcome must surface at the flow head",
            seed
        );
    }
}

#[test]
fn n510_rule_generation_sweeps_all_six_operators() {
    // №510: across a seed sweep the generator must produce EVERY comparison
    // operator (the C-01 class stays visible to the diff harness forever).
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..200u64 {
        let src = gen_program(0x4e34_6546_0000_0000u64 + seed);
        let line = src
            .lines()
            .find(|l| l.starts_with("rule If(fzn "))
            .unwrap_or_else(|| panic!("seed {}: no rule line", seed));
        for op in ["!=", "==", ">=", "<=", ">", "<"] {
            if line.contains(&format!("fzn {} ", op)) {
                seen.insert(op);
            }
        }
    }
    let missing: Vec<&str> = ["!=", "==", ">=", "<=", ">", "<"]
        .iter()
        .filter(|op| !seen.contains(**op))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "operators never generated across the sweep: {:?}",
        missing
    );
}

#[test]
fn n479_stable_codes_carry_across_backends() {
    // №479: the origin sites the class signatures read carry their
    // ADR-0131 codes on BOTH backends — the fuzzer's errorclass sides
    // compare codes, so a wording change can never move a class again.
    let _env = lock_env();
    std::env::remove_var("SMTP_HOST");
    std::env::remove_var("SMTP_USER");
    std::env::remove_var("SMTP_PASS");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // TW: undefined variable at runtime → [UNDEFINED_VARIABLE].
    let src = "pattern p(x: String) -> String {\n  return missing_name\n}\nflow Main {\n  input: String = \"a\" -> p -> output\n}\n";
    let err = run_tw(src, &repo).expect_err("TW must refuse the undefined name");
    assert!(
        err.starts_with("[UNDEFINED_VARIABLE] "),
        "the TW undefined-variable site must stamp the stable code, got: {}",
        err
    );
    assert_eq!(err_node(&err), "ident");

    // VM: undefined function at compile → [UNDEFINED_FUNCTION].
    let src = "pattern p(x: String) -> String {\n  return no_such_fn(x)\n}\nflow Main {\n  input: String = \"a\" -> p -> output\n}\n";
    let err = run_vm(src, &repo).expect_err("the VM must refuse the undefined call at compile");
    assert!(
        err.starts_with("[UNDEFINED_FUNCTION] "),
        "the VM compile site must stamp the stable code, got: {}",
        err
    );
    assert_eq!(err_node(&err), "fn_call");

    // BOTH backends: the heterogeneous '+' refusal → [TYPE_MISMATCH],
    // the SAME code on both sides (the class pair collapses when both
    // lanes agree — the №474 lesson, generalized to codes).
    let src = "pattern p(x: String) -> String {\n  return x + 5\n}\nflow Main {\n  input: String = \"a\" -> p -> output\n}\n";
    let tw_err = run_tw(src, &repo).expect_err("TW must refuse the heterogeneous concat");
    let vm_err = run_vm(src, &repo).expect_err("the VM must refuse the heterogeneous concat");
    assert!(
        tw_err.starts_with("[TYPE_MISMATCH] ") && vm_err.starts_with("[TYPE_MISMATCH] "),
        "both backends must stamp TYPE_MISMATCH on the heterogeneous '+', got tw: {} / vm: {}",
        tw_err,
        vm_err
    );
    assert_eq!(err_node(&tw_err), "binary_op");
    assert_eq!(err_node(&vm_err), "binary_op");

    // BOTH backends: a real SQL-layer failure → [SQL_ERROR] (the ADR-0169
    // stamped family; the stateful group fuzzes this domain).
    let src = "db { url: \"sqlite::memory:\" }\npattern p(x: String) -> String {\n  let bad = db_execute(\"THIS IS NOT SQL\", [])\n  return x\n}\nflow Main {\n  input: String = \"a\" -> p -> output\n}\n";
    let tw_err = run_tw(src, &repo).expect_err("TW must refuse the invalid SQL");
    let vm_err = run_vm(src, &repo).expect_err("the VM must refuse the invalid SQL");
    assert!(
        tw_err.starts_with("[SQL_ERROR] ") && vm_err.starts_with("[SQL_ERROR] "),
        "both backends must stamp SQL_ERROR on a SQL-layer failure, got tw: {} / vm: {}",
        tw_err,
        vm_err
    );

    // TW: each over a non-List → [TYPE_MISMATCH]@each (the VM silently
    // skips — the divergence is an OUTCOME class if ever generated; the
    // site's code is pinned here so the node map stays honest).
    let src = "pattern p(x: String) -> String {\n  each item in x {\n    let q = item\n  }\n  return x\n}\nflow Main {\n  input: String = \"a\" -> p -> output\n}\n";
    let err = run_tw(src, &repo).expect_err("TW must refuse each over a non-List");
    assert!(
        err.starts_with("[TYPE_MISMATCH] "),
        "the TW each site must stamp the stable code, got: {}",
        err
    );
    assert_eq!(err_node(&err), "each");
}

#[test]
fn n479_node_map_is_pinned() {
    // The node kinds of the origin sites — the class-signature construct
    // view must not drift silently.
    assert_eq!(
        err_node("[UNDEFINED_FUNCTION] compile: undefined function: f"),
        "fn_call"
    );
    assert_eq!(
        err_node("some wrapper: [X] unknown function 'g'"),
        "fn_call"
    );
    assert_eq!(
        err_node("[UNDEFINED_VARIABLE] undefined variable: x"),
        "ident"
    );
    assert_eq!(
        err_node("[TYPE_MISMATCH] each: expected List, got String"),
        "each"
    );
    assert_eq!(
        err_node("[TYPE_MISMATCH] type mismatch in string concatenation: String + Float"),
        "binary_op"
    );
    assert_eq!(
        err_node("[TYPE_MISMATCH] type mismatch in binary operation: Float Sub String"),
        "binary_op"
    );
    assert_eq!(err_node("smtp_send: SMTP_HOST env not set"), "other");
}
