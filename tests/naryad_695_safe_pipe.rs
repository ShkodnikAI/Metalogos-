// ── Naryad №695 (gh#1274, Wave 46): the safe pipe stage ───────────────
//
// ADR-0188 phase 2 (option (б), the owner's verdict gh#1123): the pipe
// grammar `{{ expr | stage }}` lands with the check-time Html typing —
// completing (а). The v1 grammar is ONE stage, `safe`, in the
// DOUBLE-brace interpolated form, and the substitution contract is:
//
//   - `{{ param }}`         — the escaped default (UNCHANGED, №115);
//   - `{{ param | safe }}`  — the LOUD escape-exempt: the value rides
//     raw, and BOTH the declaration (the parameter must be Html —
//     SAFE_PIPE_NOT_HTML, the №685 mirror) and the runtime value (the
//     opaque `Value::Html` — the builtin_render backstop) are typed;
//   - every unrecognized pipe form (an unknown stage, multiple pipes, a
//     pipe in the raw triple-brace form, an expr that is not a declared
//     param) refuses LOUDLY (UNSAFE_PIPE_FORM — the N9 refusal
//     narrowed and shifted left to the semantic gate on BOTH backends).
//
// The ledger gh#967 §7 lane closes: this file LIFTS the last §7 ignore
// (tests/template_integration.rs, `template_safe_pipe_not_implemented`)
// green on BOTH backends; №115 closes with it.
//
//   N695-1: the safe stage composes RAW (an Html-typed param fed by an
//           explicit Html producer); the escaped default in the SAME
//           template still escapes — TW and VM identical.
//   N695-2: the safe stage on a String param refuses at CHECK time
//           (SAFE_PIPE_NOT_HTML) — the TW gate and the VM compile gate
//           (the №673 contour parity).
//   N695-3: the unknown stage / multi-pipe / raw-form pipe / non-param
//           expr refuse at CHECK time (UNSAFE_PIPE_FORM) — both gates.
//   N695-4: the lifted §7 test — the exact ignored source now RUNS
//           green on both backends.
//   N695-5: the taint contract — the safe slot is a RAW position (the
//           html_label seeded map conducts; no sanitizer lift).

use std::sync::Mutex;

/// The GLOBAL_TEMPLATES registry is process-global — tests that register
/// templates serialize on this lock (the n9 file's pattern).
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn vm_run(src: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(src).expect("parse");
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The N695-1 program: the safe slot rides RAW, the escaped slot in the
/// SAME template still escapes. The trusted fragment is built by an
/// explicit Html producer (render("Frag", …)) — the №685 contract shape.
const N695_COMPOSE: &str = concat!(
    "template Frag(b: String) -> Html {\n",
    "  <i>{{ b }}</i>\n",
    "}\n",
    "template T(a: String, h: Html) -> Html {\n",
    "  <p data-a=\"{{ a }}\">{{ h | safe }}</p>\n",
    "}\n",
    "pattern P(i: String) -> String {\n",
    "  return render(\"T\", \"<b>&t</b>\", render(\"Frag\", i))\n",
    "}\n",
    "flow Main { input: String = \"raw\" -> P -> output }"
);

#[test]
fn n695_safe_pipe_composes_raw_and_keeps_the_escaped_default_tw() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let out = metalogos::run_program(N695_COMPOSE)
        .expect("the safe composition runs on TW")
        .unwrap_or_default();
    // The safe slot rides RAW — the producer's own <i> markup lands.
    assert!(
        out.contains("<i>"),
        "the safe slot must ride raw, got: {out}"
    );
    // The escaped default in the SAME template still escapes — the
    // contract without the pipe is untouched.
    assert!(
        out.contains("&lt;b&gt;") || out.contains("&amp;t"),
        "the escaped default must keep escaping, got: {out}"
    );
}

#[test]
fn n695_safe_pipe_composes_raw_and_keeps_the_escaped_default_vm() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let vm_out = vm_run(N695_COMPOSE)
        .expect("the safe composition runs on VM")
        .unwrap_or_default();
    assert!(
        vm_out.contains("<i>"),
        "the safe slot must ride raw on the VM, got: {vm_out}"
    );
    assert!(
        vm_out.contains("&lt;b&gt;") || vm_out.contains("&amp;t"),
        "the escaped default must keep escaping on the VM, got: {vm_out}"
    );
    // The parity postulate: TW == VM on the same program.
    let tw = metalogos::run_program(N695_COMPOSE)
        .expect("TW run")
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        vm_out.trim_end(),
        "TW/VM parity on the safe composition"
    );
}

#[test]
fn n695_safe_pipe_string_param_refuses_check_time_tw() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let src = "template T(x: String) -> Html {\n           <p>{{ x | safe }}</p>\n         }\n         pattern P(i: String) -> String {\n           return render(\"T\", i)\n         }\n         flow Main { input: String = \"x\" -> P -> output }";
    let result = metalogos::check_program(src).expect("check parses");
    assert!(
        !result.is_ok(),
        "the safe stage on a String param must refuse at check time"
    );
    let errs = result.errors_text();
    // The raw message names the class prose; the [SAFE_PIPE_NOT_HTML]
    // stable-code prefix is stamped by the blocking gates (the №523
    // shape) — the compile-path test asserts the prefix.
    assert!(
        errs.contains("safe insert") && errs.contains("must be Html"),
        "the refusal must name the safe insert and the Html requirement: {errs}"
    );
}

#[test]
fn n695_safe_pipe_string_param_refuses_check_time_vm() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let src = "template T(x: String) -> Html {\n           <p>{{ x | safe }}</p>\n         }\n         pattern P(i: String) -> String {\n           return render(\"T\", i)\n         }\n         flow Main { input: String = \"x\" -> P -> output }";
    let err = metalogos::compile_program(src).expect_err("the VM compile gate must refuse");
    assert!(
        err.contains("SAFE_PIPE_NOT_HTML") && err.contains("safe insert"),
        "the VM compile refusal must name the stable code and the safe insert: {err}"
    );
}

#[test]
fn n695_run_path_refuses_safe_on_string_param_both_backends() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The N9-1 successor: the run path blocks through check_program (the
    // №523 gate) — the run refusal carries the first finding's code.
    let src = "template T(x: String, y: String) -> Html {\n           <p>{{ x | safe }}</p>\n         }\n         pattern P(i: String) -> String {\n           return render(\"T\", i, \"b\")\n         }\n         flow Main { input: String = \"x\" -> P -> output }";
    let err = metalogos::run_program(src).expect_err("the pipe on a String param must refuse");
    assert!(
        err.contains("SAFE_PIPE_NOT_HTML") || err.contains("safe insert"),
        "the run refusal must name the class: {err}"
    );
}

#[test]
fn n695_unrecognized_pipe_forms_refuse_check_time_tw() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cases = [
        // An unknown stage.
        "template T(x: String) -> Html {\n  <p>{{ x | upper }}</p>\n}",
        // Multiple pipes.
        "template T(x: String) -> Html {\n  <p>{{ x | safe | upper }}</p>\n}",
        // A pipe in the raw triple-brace form.
        "template T(x: Html) -> Html {\n  <p>{{{ x | safe }}}</p>\n}",
        // The expr is not a declared param.
        "template T(x: String) -> Html {\n  <p>{{ user.name | safe }}</p>\n}",
    ];
    for (i, src) in cases.iter().enumerate() {
        let result = metalogos::check_program(src).expect("check parses");
        assert!(
            !result.is_ok(),
            "case {i}: the unrecognized pipe form must refuse at check time: {src}"
        );
        let errs = result.errors_text();
        assert!(
            errs.contains("not part of the v1 pipe grammar"),
            "case {i}: the refusal must name the v1 grammar: {errs}"
        );
    }
}

#[test]
fn n695_unrecognized_pipe_forms_refuse_check_time_vm() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let src = "template T(x: String) -> Html {\n  <p>{{ x | upper }}</p>\n}";
    let err = metalogos::compile_program(src).expect_err("the VM compile gate must refuse");
    assert!(
        err.contains("not part of the v1 pipe grammar"),
        "the VM compile refusal must name the v1 grammar: {err}"
    );
}

#[test]
fn n695_lifted_ledger_7_ignore_runs_green_both_backends() {
    // The exact ignored test source (tests/template_integration.rs, the
    // gh#967 §7 lane) — now a GREEN program on both backends. The lift
    // is the ledger §7 close: no ignore left, the lane is resolved.
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let src = "template Layout(content: Html) -> Html {\n<body>{{ content | safe }}</body>\n}\ntemplate Frag() -> Html {\n<i>ok</i>\n}\npattern P(i: String) -> String {\n  return render(\"Layout\", render(\"Frag\"))\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let tw = metalogos::run_program(src)
        .expect("the lifted §7 program runs on TW")
        .unwrap_or_default();
    assert!(
        tw.contains("<i>ok</i>"),
        "the safe slot rides raw on TW, got: {tw}"
    );
    let vm_out = vm_run(src)
        .expect("the lifted §7 program runs on VM")
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        vm_out.trim_end(),
        "TW/VM parity on the lifted §7 program"
    );
}

#[test]
fn n695_check_ok_on_honest_safe_composition() {
    // The positive check fact: an honest safe composition passes the
    // semantic check (the gate refuses only the dishonest shapes).
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let result = metalogos::check_program(N695_COMPOSE).expect("check parses");
    assert!(
        result.is_ok(),
        "the honest safe composition must pass the check: {:?}",
        result.errors_text()
    );
}

// The taint contract test (the safe slot is a RAW position in the seeded
// render-raw map) lives as a UNIT test in src/html_label.rs — the map
// and its seeder are pub(crate), the integration tests cannot reach them.

// ── The errors_text helper ────────────────────────────────────────────
//
// The AnalysisResult exposes the per-error messages through its own API;
// the tests read the formatted text (the same lines the gates print).

trait ErrorsText {
    fn errors_text(&self) -> String;
}

impl ErrorsText for metalogos::semantic::AnalysisResult {
    fn errors_text(&self) -> String {
        let mut out = String::new();
        for e in &self.errors {
            out.push_str(&e.message);
            out.push('\n');
        }
        out
    }
}
