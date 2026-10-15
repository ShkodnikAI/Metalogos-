// ── Naryad N9 (gh#1219, PE-2, compiler): render() and | safe ─────────
//
// THE N9 DECISION (the naryad's menu, options 1+2+3 combined) stood
// until №695: the pipe was a LOUD refusal at COMPILE time, the raw
// insert was the triple-brace form, and the example (p6) used the raw
// form. №695 (gh#1274, В46 — ADR-0188 phase 2) NARROWS the refusal:
//   - the safe stage `{{ param | safe }}` in template DECLARATIONS now
//     COMPOSES (the check-time Html typing — SAFE_PIPE_NOT_HTML on a
//     non-Html param, the №685 mirror; the runtime backstop requires
//     the opaque Value::Html);
//   - every UNRECOGNIZED pipe form still refuses LOUDLY — shifted left
//     to the semantic gate (UNSAFE_PIPE_FORM, both backends);
//   - template_render STRINGS still refuse every pipe (a runtime string
//     has no declarations to type against — the honest answer).
// The N9 tests below are ACTUALIZED to the narrowed contract (the
// refusal classes moved, the loudness stayed); the new composition
// contract lives in tests/naryad_695_safe_pipe.rs.
//   N9-1: render() with the pipe on a NON-Html param → the check-time
//         refusal (SAFE_PIPE_NOT_HTML, the №695 gate — the run path
//         blocks through check_program).
//   N9-2: template_render with the pipe → the loud parse refusal
//         (the strings have no declarations to type against).
//   N9-3: render() `{{{ var }}}` renders RAW (an Html-typed parameter
//         built by an explicit producer); `{{ var }}` escapes.
//   N9-4: template_render `{{{ var }}}` raw (the pre-existing form).
//   N9-5: the pipe inside a CONDITION literal is legal
//         ({{#if x == "a|b"}} — the construct grammar, not a filter).
//   N9-6: the TW/VM parity — both backends refuse and render
//         identically.
//   N9-7: the golden pair p6_template_composition (the .expected pins
//         the escaped title + the raw body).
//   №685-1: a String raw parameter refuses (RAW_INSERT_NOT_HTML) —
//         the actualization of N9-3 under the fail-closed contract.

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

// ── N9-1: render() — the pipe on a non-Html param is a check-time
//       refusal (№695 narrowed the N9 refusal to the unrecognized
//       forms; the String param in the safe slot refuses through
//       SAFE_PIPE_NOT_HTML) ──────────────────────────────────────────

#[test]
fn n9_render_pipe_is_compile_refusal() {
    let src = "template T(x: String, y: String) -> Html {\n           <p>{{ x | safe }}</p>\n         }\n         pattern P(i: String) -> String {\n           return render(\"T\", i, \"b\")\n         }\n         flow Main { input: String = \"x\" -> P -> output }";
    let err = metalogos::run_program(src).expect_err("the pipe on a String param must refuse");
    assert!(
        err.contains("SAFE_PIPE_NOT_HTML") || err.contains("safe insert"),
        "the refusal must name the safe-insert class: {err}"
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

// ── N9-3: render() — the raw triple-brace (an Html-typed parameter,
//       built by an explicit producer) + the escaped default ──────────

#[test]
fn n9_render_raw_and_escaped_forms() {
    let src = "template Frag(b: String) -> Html {\n  <i>{{ b }}</i>\n}\ntemplate T(a: String, h: Html) -> Html {\n  <p data-a=\"{{ a }}\">{{{ h }}}</p>\n}\npattern P(i: String) -> String {\n  return render(\"T\", \"<b>&t</b>\", render(\"Frag\", i))\n}\nflow Main { input: String = \"raw\" -> P -> output }";
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

// ── №685-1: a String raw parameter refuses — the fail-closed contract

#[test]
fn n685_raw_insert_string_param_refuses() {
    // The audit's S-1 scenario shape: the raw parameter typed String.
    let src = "template T(a: String, b: String) -> Html {\n  <p data-a=\"{{ a }}\">{{{ b }}}</p>\n}\npattern P(i: String) -> String {\n  return render(\"T\", \"x\", \"<i>raw</i>\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let err = metalogos::run_program(src).expect_err("a String raw parameter must refuse");
    assert!(
        err.contains("RAW_INSERT_NOT_HTML"),
        "the stable code stamps the refusal first: {err}"
    );
    assert!(
        err.contains("'b'") && err.contains("Html"),
        "the refusal names the parameter and the requirement: {err}"
    );
}

// ── №685-2: a String-SHAPED argument into an Html parameter refuses —
//       the implicit coercion the №662 rule-1 twin catches

#[test]
fn n685_implicit_string_to_html_coercion_refuses() {
    // The template is HONEST (h: Html raw), but the caller passes a
    // string literal — the implicit String→Html coercion refuses at the
    // call site (the №662 rule-1 shape).
    let src = "template T(a: String, h: Html) -> Html {\n  <p>{{ a }}{{{ h }}}</p>\n}\npattern P(i: String) -> String {\n  return render(\"T\", i, \"<i>not a producer</i>\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let err = metalogos::run_program(src).expect_err("a string literal must not coerce to Html");
    assert!(
        err.contains("implicit String→Html coercion"),
        "the coercion refusal names the rule: {err}"
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
    let src = "template Frag(b: String) -> Html {\n  <i>{{ b }}</i>\n}\ntemplate T(a: String, h: Html) -> Html {\n  <p data-a=\"{{ a }}\">{{{ h }}}</p>\n}\npattern P(i: String) -> String {\n  return render(\"T\", \"<b>&t</b>\", render(\"Frag\", i))\n}\nflow Main { input: String = \"raw\" -> P -> output }";
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
