// ── Naryad N9 (gh#1219, PE-2, compiler): render() and | safe ─────────
//
// The owner's finding: `render()` always escapes, `| safe` does not
// work, and the example used `{{ content | safe }}` — nested HTML
// escaped (or worse: the №115 substitution left the placeholder
// VERBATIM in the output; the №86 engine silently rendered it empty).
// The silent mismatch between the example syntax and the runtime is
// what the naryad forbids.
//
// THE DECISION (the naryad's menu, options 1+2+3 combined, recorded
// here and in REFERENCE):
//   - `| safe` is a LOUD refusal at COMPILE time (the registration
//     gate: the №250 compiler registration + the interpreter
//     declaration walk) and at template_render parse time — the
//     Jinja-style filter pipe was never implemented, the placeholder
//     grammar has no filters;
//   - the RAW insert is the triple-brace form `{{{ var }}}` — now
//     implemented in render() TOO (the template_render contract
//     mirrored), safe-by-construction: the parameters are
//     program-provided, the egress gates still apply downstream;
//   - the example (p6_template_composition) uses the raw form and
//     actually renders: the title rides the ESCAPED default (the
//     script-neutralization is visible in the .expected), the trusted
//     fragment rides raw.
//
//   N9-1: render() with `{{ var | safe }}` → the compile-time refusal
//         naming the triple-brace alternative.
//   N9-2: template_render with the pipe → the loud parse refusal.
//   N9-3: render() `{{{ var }}}` renders RAW; `{{ var }}` escapes.
//   N9-4: template_render `{{{ var }}}` raw (the pre-existing form).
//   N9-5: the pipe inside a CONDITION literal is legal
//         ({{#if x == "a|b"}} — the construct grammar, not a filter).
//   N9-6: the TW/VM parity — both backends refuse and render
//         identically.
//   N9-7: the golden pair p6_template_composition (the .expected pins
//         the escaped title + the raw body).

// The test harness reads the golden example pair from examples/ — the
// sanctioned test-harness exemption (the golden.rs / vm_golden.rs
// pattern; the PROGRAM-INFLUENCED I/O stays behind the fs_gate).
#![allow(clippy::disallowed_methods)]

fn eval_expr(src: &str) -> Result<String, String> {
    let body = if src.trim_start().starts_with("let ") || src.contains("return") {
        format!("pattern __eval(input: String) -> String {{ {} }}", src)
    } else {
        format!(
            "pattern __eval(input: String) -> String {{ return {} }}",
            src
        )
    };
    let full = format!(
        "{}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        body
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err("eval returned None".to_string()),
        Err(e) => Err(e),
    }
}

// ── N9-1: render() — the pipe is a compile-time refusal ──────────────

#[test]
fn n9_render_pipe_is_compile_refusal() {
    let src = "template T(x: String, y: String) -> Html {\n           <p>{{ x | safe }}</p>\n         }\n         pattern P(i: String) -> String {\n           return render(\"T\", i, \"b\")\n         }\n         flow Main { input: String = \"x\" -> P -> output }";
    let err = metalogos::run_program(src).expect_err("the pipe must refuse");
    assert!(
        err.contains("NOT supported") && err.contains("{{{"),
        "the refusal must name the unsupported pipe and the raw form: {err}"
    );
}

// ── N9-2: template_render — the pipe is a loud parse refusal ─────────

#[test]
fn n9_template_render_pipe_is_loud() {
    let src = "return template_render(\"<p>{{ x | safe }}</p>\", \"{\\\"x\\\": \\\"v\\\"}\")";
    let err = eval_expr(src).expect_err("the pipe must refuse");
    assert!(
        err.contains("NOT supported"),
        "the refusal must be loud: {err}"
    );
}

// ── N9-3: render() — the raw triple-brace + the escaped default ──────

#[test]
fn n9_render_raw_and_escaped_forms() {
    let src = "template T(a: String, b: String) -> Html {\n  <p data-a=\"{{ a }}\">{{{ b }}}</p>\n}\npattern P(i: String) -> String {\n  return render(\"T\", \"<b>&t</b>\", \"<i>raw</i>\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let out = metalogos::run_program(src)
        .expect("render must succeed")
        .unwrap_or_default();
    assert!(
        out.contains("data-a=\"&lt;b&gt;&amp;t&lt;/b&gt;\""),
        "the {{ var }} default ESCAPES: {out}"
    );
    assert!(
        out.contains("<i>raw</i>"),
        "the triple-brace form inserts RAW: {out}"
    );
    assert!(
        !out.contains("{{{") && !out.contains("}}}"),
        "no placeholder markup leaks into the output: {out}"
    );
}

// ── N9-4: template_render — the raw form (the pre-existing contract) ─

#[test]
fn n9_template_render_raw_form() {
    let src = "let d = parse_json(\"{\\\"x\\\": \\\"<i>v</i>\\\"}\")\n           return template_render(\"{{{ x }}}|{{ x }}\", d)";
    let out = eval_expr(src).expect("template_render must succeed");
    assert_eq!(
        out.trim_end(),
        "<i>v</i>|&lt;i&gt;v&lt;/i&gt;",
        "raw vs escaped, both engines one contract: {out}"
    );
}

// ── N9-5: the pipe inside a condition literal is legal ───────────────

#[test]
fn n9_pipe_in_condition_literal_is_legal() {
    let src = "let d = parse_json(\"{\\\"x\\\": \\\"a|b\\\"}\")\n           return template_render(\"{{#if x == \\\"a|b\\\"}}yes{{else}}no{{/if}}\", d)";
    let out = eval_expr(src).expect("a pipe inside the condition literal is the construct grammar");
    assert_eq!(out.trim_end(), "yes", "{out}");
}

// ── N9-6: the TW/VM parity on the render forms ───────────────────────

#[test]
fn n9_vm_parity_raw_and_escaped() {
    let src = "template T(a: String, b: String) -> Html {\n  <p data-a=\"{{ a }}\">{{{ b }}}</p>\n}\npattern P(i: String) -> String {\n  return render(\"T\", \"<b>&t</b>\", \"<i>raw</i>\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let tw = metalogos::run_program(src).expect("TW render");
    let declarations = metalogos::parser::parse(src).expect("parse");
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(program).expect("VM render").unwrap_or_default();
    assert_eq!(
        tw.unwrap_or_default().trim_end(),
        vm_out.trim_end(),
        "TW/VM parity on the raw + escaped composition"
    );
}

// ── N9-7: the golden pair ────────────────────────────────────────────

#[test]
fn n9_golden_pair_p6_composition() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(base.join("examples/p6_template_composition.mlog"))
        .expect("the golden example exists");
    let expected = std::fs::read_to_string(base.join("examples/p6_template_composition.expected"))
        .expect("the golden expected exists");

    let tw = metalogos::run_program(&source)
        .expect("the golden example runs on TW")
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        expected.trim_end(),
        "the TW output must equal the .expected file (the escaped title + the raw body)"
    );

    let declarations = metalogos::parser::parse(&source).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(program).expect("the VM run").unwrap_or_default();
    assert_eq!(
        vm_out.trim_end(),
        expected.trim_end(),
        "the VM output must equal the .expected file too"
    );
}
