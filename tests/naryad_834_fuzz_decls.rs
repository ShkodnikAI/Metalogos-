// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures by design — the allow is scoped to this file.

#![allow(clippy::disallowed_methods)]

// ── Naryad №834 (wave 70, P2, [testing]): the diff-fuzzer forms for the
//    top-level declarations between patterns (the H-1 CLASS, not the
//    instance) ─────────────────────────────────────────────────────────
//
// The audit (action 4, Medium): the H-1 finding (#1616 — `profile legacy`
// corrupting the pattern dispatch) was a CLASS — «a top-level declaration
// affects the dispatch/definition table», driven by the combination of
// declaration + unit size + definition order — while the №766 fuzzer met
// `profile legacy` once and without patterns declared around it. The
// generator here makes the class FIRST-CLASS:
//
//   1. FORMS: every generated unit carries 1-3 top-level declarations
//      (`profile legacy { egress: … }`, `origin …`, `skill_index …`,
//      `conversation { … }`) inserted BEFORE / BETWEEN / AFTER the
//      pattern definitions — the positions the audit named;
//   2. BACKWARD REFERENCES with MIXED ARITIES: the generated patterns
//      call patterns defined EARLIER with exact arities from {0,1,2},
//      and the mandatory «0-ary predecessor» branch (the silent H-1
//      mode — a 0-arg call whose callee slot may be hijacked) is
//      generated in a guaranteed share of programs;
//   3. THE ORACLE IS THE CLASS: every pattern returns a UNIQUE tag built
//      from its name + the arguments it received — a mis-dispatch that
//      executes ANOTHER pattern's body (or loses the arguments) changes
//      the output, so ANY TW↔VM divergence on the same file (outcome,
//      stdout, or the error class) fails the run. The oracle catches the
//      CLASS, not a specific message;
//   4. THE SEED: the №829 repro shape (the declaration + the same-arity
//      shadow pair + a 0-arg call) rides as the fixed fixture run through
//      the same oracle;
//   5. THE RATCHET: the sweep holds the №621 liveness discipline — a
//      program that refuses must refuse IDENTICALLY on both lanes (same
//      class), and a floor of the sweep must EXECUTE on both lanes (the
//      CI budget stays the №766-sized default; FUZZ_ITERS overrides).

use std::path::{Path, PathBuf};

fn iterations() -> u32 {
    std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150)
}

/// The liveness floor (the №621 posture): at least this share of the
/// generated sweep must EXECUTE on BOTH lanes — a fuzzer whose programs
/// all refuse compares refusals, not programs.
const LIVENESS_FLOOR_PCT: u32 = 70;

/// xorshift64 — deterministic, dependency-free (the №465 house RNG).
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
}

// ── The declaration pool (the class shapes; all inert for dispatch by
//    the №829 contract — which is exactly what makes a dispatch change a
//    divergence) ──────────────────────────────────────────────────────

const PROFILES: &[&str] = &["profile legacy { egress: permissive_with_audit }"];

fn gen_origin(rng: &mut Rng, tag: &str) -> String {
    let kind = rng.pick(&["generation", "camera"]);
    let label = rng.pick(&["public", "private"]);
    format!("origin {tag} {{ kind: {kind}, media: image, label: {label} }}")
}

fn gen_skill_index(tag: &str) -> String {
    format!("skill_index {tag} {{ tier 1 always {{ [\"skill_a\"] }} }}")
}

fn gen_conversation() -> String {
    let ttl = 600 + 300 * (faux_seed() % 5);
    format!("conversation {{ ttl: {ttl}, max_messages: 50, compress_after: 20 }}")
}

/// A deterministic side seed for the inert numeric shapes (keeps the
/// generator single-RNG without threading more state).
fn faux_seed() -> u64 {
    42
}

fn gen_declaration(rng: &mut Rng, i: usize) -> String {
    match rng.below(4) {
        0 => rng.pick(PROFILES).to_string(),
        1 => gen_origin(rng, &format!("fzo{i}")),
        2 => gen_skill_index(&format!("fzs{i}")),
        _ => gen_conversation(),
    }
}

// ── The program generator ────────────────────────────────────────────

const ARG_VALUES: &[&str] = &["\"a1\"", "\"b2\"", "\"c3\""];

fn gen_program(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let n_patterns = 2 + rng.below(3) as usize; // 2..=4
    let n_decls = 1 + rng.below(3) as usize; // 1..=3

    // The declarations and their positions: position k ∈ 0..=n_patterns —
    // 0 = before the first pattern, n = after the last (before the flow),
    // 1..n-1 = between the patterns. The audit's class: the DECLARATION
    // POSITION relative to the pattern definitions is part of the trigger
    // surface — the generator sweeps all of it.
    let mut decl_slots: Vec<(usize, String)> = Vec::new();
    for i in 0..n_decls {
        let pos = rng.below((n_patterns + 1) as u64) as usize;
        decl_slots.push((pos, gen_declaration(&mut rng, i)));
    }

    // The pattern arities: the MANDATORY 0-ary predecessor branch — when
    // the coin says so (about 1/2 of the programs), the FIRST pattern is
    // 0-ary (the silent H-1 mode hijacks exactly this slot shape); the
    // rest pick from {0,1,2}.
    let mut arities: Vec<u64> = Vec::new();
    for i in 0..n_patterns {
        let a = if i == 0 && rng.below(2) == 0 {
            0
        } else {
            rng.below(3)
        };
        arities.push(a);
    }

    let mut src = String::new();
    for (i, &arity) in arities.iter().enumerate() {
        let _ = arity;
        for (pos, decl) in &decl_slots {
            if *pos == i {
                src.push_str(decl);
                src.push_str("\n\n");
            }
        }
        let name = format!("fp{i}");
        let params: Vec<String> = (0..arities[i]).map(|k| format!("q{k}: String")).collect();
        src.push_str(&format!(
            "pattern {name}({}) -> String {{\n",
            params.join(", ")
        ));
        // The body: the UNIQUE tag (name + the echoed args) — a
        // mis-dispatch that executes another body, or loses the
        // arguments, changes the output. The backward-reference lane:
        // call ONE previously defined pattern with its EXACT arity (the
        // nested dispatch — the definition-order axis of the class).
        let echo: String = (0..arities[i])
            .map(|k| format!(" + \":\" + to_string(q{k})"))
            .collect();
        if i > 0 && rng.below(3) == 0 {
            let target = rng.below(i as u64) as usize;
            let tname = format!("fp{target}");
            let targs: Vec<String> = (0..arities[target])
                .map(|k| ARG_VALUES[k as usize].to_string())
                .collect();
            src.push_str(&format!("  let nested = {tname}({})\n", targs.join(", ")));
            src.push_str(&format!(
                "  return \"{name}:\"{echo} + \"<\" + nested + \"\\\"\"\n}}\n\n"
            ));
        } else {
            src.push_str(&format!("  return \"{name}:\"{echo}\n}}\n\n"));
        }
    }
    for (pos, decl) in &decl_slots {
        if *pos == n_patterns {
            src.push_str(decl);
            src.push_str("\n\n");
        }
    }

    // The Runner: BACKWARD references to every pattern with the EXACT
    // arity — the whole definition order rides the output. A dispatch
    // that loses the arguments («pattern X expects N args, got 0», the
    // #1618 form) or executes the wrong body (#1616) diverges HERE.
    src.push_str("pattern Runner(input: String) -> String {\n");
    let mut parts: Vec<String> = Vec::new();
    for (i, &arity) in arities.iter().enumerate() {
        let name = format!("fp{i}");
        let args: Vec<String> = (0..arity)
            .map(|k| ARG_VALUES[k as usize].to_string())
            .collect();
        src.push_str(&format!("  let r{i} = {name}({})\n", args.join(", ")));
        parts.push(format!("to_string(r{i})"));
    }
    parts.push("input".to_string());
    src.push_str(&format!("  return {} \n}}\n\n", parts.join(" + \"|\" + ")));
    src.push_str("flow Main {\n  input: String = \"seed\" -> Runner -> output\n}\n");
    src
}

// ── The №829 seed fixture: the repro shape as a fixed program ────────

/// The imported module whose `Shim` the entry SHADOWS (the #1616
/// trigger component — the import + the same-name local pattern).
const SHADOW_MODULE: &str = r#"
pattern Shim(a: String) -> String {
  return "imported"
}
"#;

/// The №829 silent-corruption form (#1616 verbatim shape): the import +
/// the shadowed local Shim + the same-arity pair + a 0-arg call +
/// `profile legacy` before the definitions. Pre-fix the VM executed the
/// WRONG body silently («DECOY» instead of «TARGET»); the class oracle
/// must keep it green forever.
const SEED_829: &str = r#"
import fixtures834/mod
profile legacy { egress: permissive_with_audit }
pattern Shim(a: String) -> String {
  return "local"
}
pattern Decoy() -> String {
  return "DECOY"
}
pattern Target() -> String {
  return "TARGET"
}
pattern Runner(input: String) -> String {
  let direct = Target()
  return direct + "|" + input
}
flow Main {
  input: String = "" -> Runner -> output
}
"#;

/// The args-lost form (#1618): the 2-arg/0-arg mix — pre-fix the args
/// routed to the WRONG callee («pattern X expects N args, got 0»).
const SEED_829_ARGS: &str = r#"
import fixtures834/mod
profile legacy { egress: permissive_with_audit }
pattern Shim(a: String) -> String {
  return "local"
}
pattern Mark(name: String, ok: String) -> String {
  return "M:" + name + ok
}
pattern Probe() -> String {
  return "probe-ok"
}
pattern Runner(input: String) -> String {
  let p = Probe()
  let out = p + Mark("b1", "t")
  return out + "|" + input
}
flow Main {
  input: String = "" -> Runner -> output
}
"#;

// ── The harness (the №465 posture: TW is the oracle lane) ────────────

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    // №465 posture: the VM lane mirrors the production `mlog run` gate
    // (the semantic check + the suppression filters) — a lane without the
    // gate compares behavior the production never reaches, and a
    // TW-refused unit would look like an outcome divergence (the seed-16
    // COMPAT_PROFILE_INVALID artifact this file's first sweep caught).
    let module_decls = metalogos::semantic::resolve_imports_statically(&declarations, base_dir)
        .map_err(|e| format!("Compilation error (Naryad #523): {}", e))?;
    let mut merged_decls = module_decls;
    merged_decls.extend(declarations.clone());
    let sem_result = metalogos::semantic::check_program(&merged_decls);
    let mut sem_errors = sem_result.errors;
    sem_errors = metalogos::semantic::filter_suppressed_unchecked(sem_errors, source);
    let blocking: Vec<&metalogos::semantic::SpannedError> = sem_errors
        .iter()
        .filter(|err| !metalogos::semantic::is_exempt_from_blocking(err.kind))
        .collect();
    if !blocking.is_empty() {
        let code = blocking.iter().find_map(|err| err.kind.stable_code());
        let stamp = code.map(|c| format!("[{}] ", c)).unwrap_or_default();
        let lines: Vec<String> = blocking
            .iter()
            .map(|err| metalogos::semantic::format_blocking_line(err))
            .collect();
        return Err(format!(
            "{}Compilation error (Naryad #523): semantic findings block execution:\n{}",
            stamp,
            lines.join("\n")
        ));
    }
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("n834_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The class oracle: ANY divergence between the lanes on the same file —
/// outcome, stdout, or the stable error class — is a failure. The class
/// is caught by the DIFFERENCE, never by a specific message.
fn diff_outcome(
    tw: &Result<Option<String>, String>,
    vm: &Result<Option<String>, String>,
) -> Option<String> {
    match (tw, vm) {
        (Ok(a), Ok(b)) => {
            let na = a.as_deref().map(str::trim_end).unwrap_or_default();
            let nb = b.as_deref().map(str::trim_end).unwrap_or_default();
            if na != nb {
                Some(format!("OUTCOME DIVERGENCE: tw={na:?} vm={nb:?}"))
            } else {
                None
            }
        }
        (Err(a), Err(b)) => {
            let ca = stable_class(a);
            let cb = stable_class(b);
            if ca != cb {
                Some(format!("ERROR-CLASS DIVERGENCE: tw=[{ca}] vm=[{cb}]"))
            } else {
                None
            }
        }
        (Ok(v), Err(e)) | (Err(e), Ok(v)) => Some(format!(
            "OUTCOME-CLASS DIVERGENCE: one lane ran ok={v:?}, the other refused: {}",
            e.lines().next().unwrap_or("")
        )),
    }
}

/// The stable error class: the first `[CODE]` stamp on ANY line (the
/// leak-suite extraction form — the Category-A refusal wraps its code
/// behind the invariant header, a position-0-only scan misses it), else
/// the head. The anti-forgery concern of №465's position-0 rule applies
/// to program OUTPUT; the refusal texts here are GATE productions.
fn stable_class(err: &str) -> String {
    for line in err.lines() {
        if let Some(start) = line.find('[') {
            if let Some(end_rel) = line[start + 1..].find(']') {
                let code = &line[start + 1..start + 1 + end_rel];
                if !code.is_empty()
                    && code
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                {
                    return code.to_string();
                }
            }
        }
    }
    let head = err.trim();
    head.chars().take(40).collect()
}

// ── The tests ────────────────────────────────────────────────────────

#[test]
fn seed_829_forms_stay_green_on_the_oracle() {
    for (tag, unit) in [("silent", SEED_829), ("args", SEED_829_ARGS)] {
        let dir = scratch_dir(tag);
        // The shadow-module fixture the import resolves (the #1616
        // trigger component).
        std::fs::create_dir_all(dir.join("fixtures834")).expect("fixture dir");
        std::fs::write(dir.join("fixtures834/mod.mlog"), SHADOW_MODULE).expect("write module");
        let tw = run_tw(unit, &dir);
        let vm = run_vm(unit, &dir);
        if let Some(d) = diff_outcome(&tw, &vm) {
            panic!("the №829 seed form {tag} diverged: {d}\nunit:\n{unit}");
        }
        // The seed must EXECUTE (the tags are the assertion surface):
        let out = tw.expect("the seed form must run").expect("output");
        assert!(
            out.contains("TARGET") || out.contains("probe-ok"),
            "the seed form {tag} produced an unexpected output: {out:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn sweep_declarations_between_patterns_diff_green() {
    let mut executed = 0u32;
    let mut refused_same_class = 0u32;
    let mut divergences: Vec<String> = Vec::new();

    for seed in 0..iterations() as u64 {
        let src = gen_program(seed);
        let dir = scratch_dir("sweep");
        let tw = run_tw(&src, &dir);
        let vm = run_vm(&src, &dir);
        let _ = std::fs::remove_dir_all(&dir);
        match (&tw, &vm) {
            (Ok(_), Ok(_)) => executed += 1,
            (Err(_), Err(_)) => refused_same_class += 1,
            _ => {}
        }
        if let Some(d) = diff_outcome(&tw, &vm) {
            if std::env::var("N834_DEBUG").is_ok() && divergences.is_empty() {
                eprintln!("N834 DEBUG seed {seed}: tw={tw:?}\nvm={vm:?}\nsrc:\n{src}");
            }
            divergences.push(format!("seed {seed}:\n{d}\n--- program ---\n{src}"));
        }
    }

    let total = executed + refused_same_class;
    let live_pct = (executed * 100).checked_div(total).unwrap_or(0);
    eprintln!(
        "n834 sweep: {} programs — {} executed on both lanes, {} refused same-class \
         (liveness {}%, floor {LIVENESS_FLOOR_PCT}%)",
        executed + refused_same_class,
        executed,
        refused_same_class,
        live_pct
    );
    assert!(
        divergences.is_empty(),
        "{} TW↔VM divergence(s) in the declarations-between-patterns class:\n{}",
        divergences.len(),
        divergences.join("\n=====\n")
    );
    assert!(
        live_pct >= LIVENESS_FLOOR_PCT,
        "the liveness floor failed: {live_pct}% — the sweep compares refusals, not programs"
    );
}

/// The class-visibility probe: the generator's units really do carry the
/// class surface (a declaration BEFORE/ BETWEEN/ AFTER patterns, a 0-ary
/// predecessor, backward references). Without this, a silent generator
/// regression would hollow the sweep out (the №621 lesson, mechanical).
#[test]
fn generator_surfaces_the_class_shapes() {
    let mut with_zero_ary_first = 0u32;
    let mut with_decl_between = 0u32;
    let mut with_decl_after = 0u32;
    let mut with_backward_call = 0u32;
    for seed in 0..iterations() as u64 {
        let src = gen_program(seed);
        if src.contains("pattern fp0() -> String") {
            with_zero_ary_first += 1;
        }
        if src.contains("origin fzo")
            || src.contains("skill_index fzs")
            || src.contains("conversation {")
        {
            with_decl_between += 1;
        }
        // The after-position: a declaration between the LAST pattern and
        // the Runner (the gen writes it before "pattern Runner").
        let runner_pos = src.find("pattern Runner").unwrap_or(src.len());
        let head = &src[..runner_pos];
        if (head.matches("profile legacy").count()
            + head.matches("origin fzo").count()
            + head.matches("skill_index fzs").count()
            + head.matches("conversation {").count())
            > 1
            || head
                .rfind("\n\n")
                .map(|p| {
                    head[p..].contains("profile")
                        || head[p..].contains("origin")
                        || head[p..].contains("skill_index")
                        || head[p..].contains("conversation")
                })
                .unwrap_or(false)
        {
            with_decl_after += 1;
        }
        if src.contains("let nested = fp") {
            with_backward_call += 1;
        }
    }
    eprintln!(
        "n834 class surfaces: 0-ary-first {with_zero_ary_first}, decl-bearing {with_decl_between}, \
         multi/late-decl {with_decl_after}, backward-call {with_backward_call} (of {})",
        iterations()
    );
    assert!(
        with_zero_ary_first > 0,
        "the 0-ary predecessor branch never generated — the silent H-1 mode is unmeasured"
    );
    assert!(
        with_decl_between > 0,
        "no unit carries a top-level declaration among the patterns"
    );
    assert!(
        with_decl_after > 0,
        "no unit carries a declaration after the last pattern"
    );
    assert!(
        with_backward_call > 0,
        "no unit carries a backward-reference call"
    );
}
