#![allow(clippy::disallowed_methods)]

// ── Наряд №947 (волна 97 — W-AA, №AA2, P2; issue #1911; ADR-0223, W3
// §13.12) ────────────────────────────────────────────────────────────────
//
// The `spec` declaration in code — grammar/AST/semantic/TW+VM compile +
// the `spec_of` accessor — executes ADR-0223 verbatim (the sister naryad
// №946/PR — the ADR landed first, N1). The contract corpus (NOT the
// golden loop — the golden verdicts do not move):
//
//   (1) a declaration → `spec_of` reads the fields VERBATIM on BOTH
//       backends (TW + VM — the crosscheck parity; the ADR-0223 §2.4
//       Struct form: name/pattern/prompt/distill_after/fallback_if
//       {op,value}/min_accuracy/margin; absent = Unit);
//   (2) the hash equivalence: `canonical_spec_from_parts` over the
//       declaration parts + the test model's parts == `canonical_spec_json`
//       of the model over the SAME declaration (the SSOT is not
//       duplicated — ADR-0223 §2.5: the canonization does not move);
//   (3) a duplicate spec name → the loud semantic refusal (§2.2 guard 1);
//   (4) an unknown (non-learnable) pattern → the loud refusal (§2.2
//       guard 2 — the mutate twin text);
//   (5) a non-confidence `fallback_if` metric → the loud refusal naming
//       the field (§2.2 guard 3 — the AttestSpecInput machine truth);
//   (6) a non-numeric (out-of-usize) `distill_after` → the loud parse
//       refusal (§2.2 guard 5);
//   (7) TW↔VM parity — the same output on both lanes (test 1 IS the
//       parity check; no new interception points — the declaration
//       pass + one accessor arm per lane);
//   (8) a program without `spec` compiles and runs unchanged (the №493
//       posture — the declaration ADDS capability, nothing is taken
//       away; the byte-stability of the wider corpus is the CI's
//       golden/verdict lanes).
//
// Mutation verification (the №382 protocol): pulling the `spec_decl`
// rule from grammar.pest turns (1) red (parse error); pulling the
// registry seeding turns (1) red (`spec_of: spec '…' is not declared`);
// pulling the duplicate-name guard turns (3) red; pulling the learnable
// resolution guard turns (4) red; pulling the confidence pin turns (5)
// red — every red below names its mutation.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), repo_root())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── (1) + (7): the declaration → spec_of reads the fields verbatim ────

const FULL_SPEC_PROG: &str = r#"
spec Attest1 {
  pattern: P1,
  prompt: "what happens",
  distill_after: 3,
  fallback_if: confidence > 0.7,
  min_accuracy: 0.85,
  margin: 0.05
}

learnable pattern P1(q: String) -> String {
  prompt: "answer"
}

pattern Read(_t: String) -> String {
  let s = spec_of("Attest1")
  let fb = s.fallback_if
  return "name=" + s.name + ";pattern=" + s.pattern + ";prompt=" + s.prompt +
         ";after=" + to_string(s.distill_after) + ";op=" + fb.op +
         ";value=" + to_string(fb.value) + ";min=" + to_string(s.min_accuracy) +
         ";margin=" + to_string(s.margin)
}

flow Main { input: String = "go" -> Read -> output }
"#;

const MINIMAL_SPEC_PROG: &str = r#"
spec Attest2 { pattern: P1 }

learnable pattern P1(q: String) -> String {
  prompt: "answer"
}

pattern Read(_t: String) -> String {
  let s = spec_of("Attest2")
  return "pattern=" + s.pattern + ";prompt=" + to_string(s.prompt) +
         ";after=" + to_string(s.distill_after) + ";fb=" + to_string(s.fallback_if) +
         ";min=" + to_string(s.min_accuracy) + ";margin=" + to_string(s.margin)
}

flow Main { input: String = "go" -> Read -> output }
"#;

const EXPECTED_FULL: &str =
    "name=Attest1;pattern=P1;prompt=what happens;after=3;op=gt;value=0.7;min=0.85;margin=0.05";

#[test]
fn spec_of_reads_declared_fields_verbatim_on_both_backends() {
    let _g = lock_env();
    // TW
    let out = run_tw(FULL_SPEC_PROG).expect("the spec program must compile and run on TW");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        EXPECTED_FULL,
        "spec_of must read the declared fields verbatim (TW)"
    );
    // VM — the SAME expected string (the crosscheck parity, no new
    // interception points: the declaration pass + one accessor arm).
    let out_vm = run_vm(FULL_SPEC_PROG).expect("the spec program must compile and run on VM");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        EXPECTED_FULL,
        "TW/VM parity of the spec_of accessor"
    );
}

#[test]
fn absent_fields_read_as_unit_on_both_backends() {
    let _g = lock_env();
    let expected = "pattern=P1;prompt=();after=();fb=();min=();margin=()";
    let out = run_tw(MINIMAL_SPEC_PROG).expect("the minimal spec runs on TW");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        expected,
        "a field absent from the declaration reads as Unit (the language null), TW"
    );
    let out_vm = run_vm(MINIMAL_SPEC_PROG).expect("the minimal spec runs on VM");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        expected,
        "the absent-field Unit form holds on VM"
    );
}

// ── (2): the hash equivalence (the SSOT is not duplicated) ────────────

#[test]
fn declaration_hash_equivalence_through_the_canonical_builder() {
    use metalogos::nn::attest::{canonical_spec_from_parts, canonical_spec_json};
    use metalogos::nn::ReflexModel;

    // The declaration's record, built through the ONE compile-time
    // conversion (from_ast → to_input): parse a real program so the
    // parser+compiler path is exercised, then take the compiled record.
    let declarations = metalogos::parser::parse(
        r#"
spec Attest1 { pattern: P1, prompt: "q", distill_after: 3,
               fallback_if: confidence > 0.7, min_accuracy: 0.85, margin: 0.05 }
learnable pattern P1(q: String) -> String { prompt: "answer" }
"#,
    )
    .expect("parse");
    let mut compiled: Option<(String, metalogos::nn::attest::AttestSpecInput)> = None;
    for d in &declarations {
        if let metalogos::ast::Declaration::Spec(sp) = d {
            let c = metalogos::bytecode::CompiledSpecDecl::from_ast(sp);
            compiled = Some((c.name.clone(), c.to_input()));
        }
    }
    let (name, spec) = compiled.expect("the spec declaration must compile");

    // The test model: the Dense path (build_reflex_model — the shared
    // builder). input 3, dense(8), labels [a,b], seed 7 → the canonical
    // layer descriptor "dense(3,8)" (name(input,output) — the layer
    // descriptor format the SpecSer layers field carries).
    let decl = metalogos::bytecode::CompiledReflexDecl {
        name: name.clone(),
        input_dim: 3,
        layers: vec![metalogos::bytecode::CompiledReflexLayerSpec {
            name: "dense".to_string(),
            args: vec!["8".to_string()],
        }],
        labels: vec!["a".to_string(), "b".to_string()],
        seed: 7,
    };
    let model: ReflexModel =
        metalogos::builtins::build_reflex_model(&decl).expect("the test model builds");

    // The model path (layer_descriptors computed by the machine) vs the
    // parts path (the SAME parts handed in by the test): the two MUST be
    // byte-identical — the declaration feeds the SAME canonical form, no
    // second canonicalization exists (ADR-0223 §2.5).
    let via_model = canonical_spec_json(&model, &spec);
    let via_parts = canonical_spec_from_parts(
        &model.name,
        model.input_size,
        &model.labels,
        model.seed,
        &["dense(3,8)".to_string()],
        &spec,
    );
    assert_eq!(
        via_model, via_parts,
        "the declaration record must hash-identically ride the canonical builder"
    );
    // The record's fallback_if carries the CANONICAL word (the compile-
    // time conversion) — pin it (the ADR-0223 §2.1 vocabulary).
    assert_eq!(
        spec.fallback_if.as_ref().map(|(op, _)| op.as_str()),
        Some("gt"),
        "fallback_if op must be the canonical lowercase word"
    );
}

// ── (3)–(6): the loud guards ──────────────────────────────────────────

#[test]
fn duplicate_spec_name_refuses_loudly() {
    let _g = lock_env();
    let src = r#"
spec A { pattern: P1 }
spec A { pattern: P1 }
learnable pattern P1(q: String) -> String { prompt: "answer" }
flow Main { input: String = "x" -> output }
"#;
    let err = metalogos::check_program(src.trim())
        .expect("check_program answers the analysis")
        .errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        !err.is_empty(),
        "duplicate spec name must refuse at the semantic check"
    );
    assert!(
        err.contains("duplicate spec declaration"),
        "expected the duplicate-name guard (ADR-0223 §2.2 guard 1), got: {err}"
    );
}

#[test]
fn unknown_pattern_refuses_with_the_mutate_twin_text() {
    let _g = lock_env();
    let src = r#"
spec A { pattern: NoSuch }
flow Main { input: String = "x" -> output }
"#;
    let err = metalogos::check_program(src.trim())
        .expect("check_program answers the analysis")
        .errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        err.contains("spec 'A': learnable pattern 'NoSuch' not found"),
        "expected the learnable-resolution guard (ADR-0223 §2.2 guard 2), got: {err}"
    );
}

#[test]
fn non_confidence_fallback_metric_refuses_naming_the_field() {
    let _g = lock_env();
    // The grammar takes an IDENT on the LHS (ADR-0223 §2.1); the semantic
    // check pins it to `confidence` — a non-confidence metric would alias
    // to the same (op, value) record and forge a hash across different
    // declared intents (the §3 rejected alternative's reason).
    let src = r#"
spec A { fallback_if: temperature > 0.5 }
flow Main { input: String = "x" -> output }
"#;
    let err = metalogos::check_program(src.trim())
        .expect("check_program answers the analysis")
        .errors
        .into_iter()
        .map(|e| e.message)
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        err.contains("fallback_if metric 'temperature'") && err.contains("confidence-based"),
        "expected the confidence pin (ADR-0223 §2.2 guard 3), got: {err}"
    );
}

#[test]
fn non_numeric_distill_after_refuses_at_the_parse() {
    let _g = lock_env();
    // The INT grammar accepts the digits; an out-of-usize literal is the
    // loud parse refusal (ADR-0223 §2.2 guard 5).
    let src = r#"
spec A { distill_after: 99999999999999999999999 }
flow Main { input: String = "x" -> output }
"#;
    let err = metalogos::parser::parse(src.trim())
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        !err.is_empty(),
        "an out-of-usize distill_after must refuse at the parse"
    );
    assert!(
        err.contains("distill_after") && err.contains("out of range"),
        "expected the guard-5 parse refusal, got: {err}"
    );
}

#[test]
fn unknown_spec_name_refuses_naming_the_builtin() {
    let _g = lock_env();
    let src = r#"
pattern Read(_t: String) -> String {
  let s = spec_of("Nope")
  return "?"
}
flow Main { input: String = "x" -> Read -> output }
"#;
    for (lane, out) in [("TW", run_tw(src)), ("VM", run_vm(src))] {
        let err = out.expect_err("an unknown spec name must refuse loudly");
        assert!(
            err.contains("spec_of") && err.contains("not declared"),
            "expected the registry-pointer refusal on {lane}, got: {err}"
        );
    }
}

// ── (8): a program without `spec` runs unchanged ──────────────────────

#[test]
fn program_without_spec_compiles_and_runs_unchanged() {
    let _g = lock_env();
    let src = r#"
pattern Greet(name: String) -> String {
  return "hello " + name
}
flow Main { input: String = "world" -> Greet -> output }
"#;
    let expected = "hello world";
    let out = run_tw(src).expect("the plain program runs on TW");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
    let out_vm = run_vm(src).expect("the plain program runs on VM");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
}
