// ── Наряд №80: Integration tests for procedural backgrounds + canvas presets ──
//
// Tests the FULL pipeline for each new builtin:
//   .mlog source → interpreter (TW) → SVG output → XML validation
//   .mlog source → compiler → VM → SVG output → XML validation
//   TW vs VM output must match byte-for-byte (crosscheck invariant).
//
// Coverage:
//   svg_generate:
//     - "flow" / "grid" / "noise" all return <g>-wrapped fragments
//     - Determinism: same inputs → byte-identical output (critical
//       invariant from Block 3 of the narazd spec — explicitly tested)
//     - Unknown intent rejected (security boundary — intent is a
//       known-list enum, not free-form user text)
//     - Unknown kind rejected
//     - Zero / negative dimensions rejected
//     - Dimensions > 10000 rejected (pathological output cap)
//     - Different intents produce different outputs (proves intent
//       affects the hash / hue, not just accepted as a no-op)
//     - TW output == VM output (backend parity)
//
//   svg_canvas_preset:
//     - All 5 presets produce <svg> with correct width attribute
//     - Unknown preset rejected with error mentioning available names
//     - Backward compat: svg_canvas still works identically
//     - TW output == VM output (backend parity)
//
// Security lint (Block 5): svg_generate and svg_canvas_preset are
// intentionally NOT in SVG_AUTO_ESCAPE_BUILTINS or SVG_NO_ESCAPE_BUILTINS.
// The reasoning: `intent` is validated against a known-list of 5 values
// (calm/tension/energy/authority/warmth) before any string reaches SVG
// output, and `kind` / `preset_name` are also validated against known
// lists. No free-form user text is inserted into the SVG markup as-is.
// This is the same defense model as color_palette (which is also NOT in
// the lint lists). See the inline comment in svg.rs::background_style
// for the canonical statement of this reasoning.

// ── Helper: eval a single expression via TW (tree-walking interpreter) ──

fn eval_expr(src: &str) -> String {
    let full = format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("eval returned None for source: {}", src),
        Err(e) => panic!("eval failed for source: {}\nerror: {}", src, e),
    }
}

fn eval_err(src: &str) -> String {
    match metalogos::run_program(src) {
        Ok(_) => panic!("expected error, got Ok: {}", src),
        Err(e) => e,
    }
}

/// Compile + run via the bytecode VM (mirrors p79_charts.rs::eval_vm).
fn eval_vm(src: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(src).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn wrap(src: &str) -> String {
    format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    )
}

// ── Block 1: svg_generate("flow", ...) ───────────────────────────────

#[test]
fn svg_generate_flow_returns_g_fragment() {
    let xml = eval_expr(r#"svg_generate("flow", "energy", 960.0, 600.0)"#);
    assert!(xml.starts_with("<g>"));
    assert!(xml.ends_with("</g>"));
    // Flow produces 3–5 cubic Bezier <path> elements.
    // For h=600, count = round(600/100) = 6 clamped to 5 → 5 paths.
    assert!(
        xml.matches("<path d=\"M 0").count() >= 3,
        "flow should produce ≥3 <path> elements, got: {}",
        xml.matches("<path").count()
    );
    // Each path uses cubic Bezier (C command) — not quadratic or lines.
    assert!(
        xml.contains(" C "),
        "flow paths should use cubic Bezier C command"
    );
    // Stroke width and opacity should be present (recessive appearance).
    assert!(xml.contains("opacity=\"0.25\""));
    assert!(xml.contains("stroke-width=\"1.5\""));
}

#[test]
fn svg_generate_flow_deterministic_output() {
    // Critical invariant from the narazd spec: same inputs → identical output.
    let src = wrap(r#"svg_generate("flow", "calm", 600.0, 400.0)"#);
    let out1 = metalogos::run_program(&src).unwrap().unwrap();
    let out2 = metalogos::run_program(&src).unwrap().unwrap();
    assert_eq!(out1, out2, "svg_generate flow output must be deterministic");
}

#[test]
fn svg_generate_flow_different_intents_produce_different_output() {
    // If two different intents produced the same output, intent wouldn't
    // affect the result (regression in the hue-shift or seed logic).
    let calm = eval_expr(r#"svg_generate("flow", "calm", 600.0, 400.0)"#);
    let energy = eval_expr(r#"svg_generate("flow", "energy", 600.0, 400.0)"#);
    assert_ne!(
        calm, energy,
        "different intents should produce different flow outputs"
    );
}

#[test]
fn svg_generate_flow_rejects_unknown_intent() {
    let err = eval_err(&wrap(
        r#"svg_generate("flow", "INVALID_INTENT", 600.0, 400.0)"#,
    ));
    assert!(
        err.contains("intent must be one of") && err.contains("INVALID_INTENT"),
        "expected intent validation error, got: {}",
        err
    );
}

// ── Block 2: svg_generate("grid", ...) ───────────────────────────────

#[test]
fn svg_generate_grid_returns_g_fragment_with_lines() {
    let xml = eval_expr(r#"svg_generate("grid", "calm", 600.0, 400.0)"#);
    assert!(xml.starts_with("<g>"));
    assert!(xml.ends_with("</g>"));
    // 600×400 canvas, step = max(600,400)/12 = 50, clamped to [20,100] → 50.
    // Vertical lines: 0, 50, 100, ..., 600 → 13 lines.
    // Horizontal lines: 0, 50, 100, ..., 400 → 9 lines.
    // Total: 22 lines.
    let line_count = xml.matches("<line ").count();
    assert!(
        (20..=30).contains(&line_count),
        "expected ~22 <line> elements for 600×400 grid, got {}",
        line_count
    );
    // Grid lines have reduced opacity (recessive appearance).
    assert!(xml.contains("opacity=\"0.35\""));
}

#[test]
fn svg_generate_grid_deterministic_output() {
    let src = wrap(r#"svg_generate("grid", "energy", 600.0, 400.0)"#);
    let out1 = metalogos::run_program(&src).unwrap().unwrap();
    let out2 = metalogos::run_program(&src).unwrap().unwrap();
    assert_eq!(out1, out2, "svg_generate grid output must be deterministic");
}

#[test]
fn svg_generate_grid_rejects_unknown_intent() {
    let err = eval_err(&wrap(r#"svg_generate("grid", "INVALID", 600.0, 400.0)"#));
    assert!(err.contains("intent must be one of"));
}

// ── Block 3: svg_generate("noise", ...) — critical determinism test ──

#[test]
fn svg_generate_noise_returns_g_fragment_with_circles() {
    let xml = eval_expr(r#"svg_generate("noise", "energy", 600.0, 400.0)"#);
    assert!(xml.starts_with("<g>"));
    assert!(xml.ends_with("</g>"));
    // 600×400 canvas, cell=12 → cols=50, rows=33 → 1650 dots.
    let circle_count = xml.matches("<circle ").count();
    assert!(
        (1500..=1800).contains(&circle_count),
        "expected ~1650 <circle> elements for 600×400 noise, got {}",
        circle_count
    );
}

#[test]
fn svg_generate_noise_byte_identical_across_calls() {
    // THE critical test from the narazd spec (Block 6):
    //   "Два вызова svg_generate("noise", "energy", 600, 400) подряд
    //    должны дать ИДЕНТИЧНЫЙ результат, побайтово."
    let src = wrap(r#"svg_generate("noise", "energy", 600.0, 400.0)"#);
    let out1 = metalogos::run_program(&src).unwrap().unwrap();
    let out2 = metalogos::run_program(&src).unwrap().unwrap();
    assert_eq!(
        out1, out2,
        "svg_generate noise MUST be byte-identical across calls (determinism invariant)"
    );
}

#[test]
fn svg_generate_noise_different_intents_produce_different_patterns() {
    // The intent seeds the hash (via intent_to_hue / 360.0). If two
    // different intents produced the same dot pattern, the seed wouldn't
    // be incorporated into the hash input.
    let calm = eval_expr(r#"svg_generate("noise", "calm", 600.0, 400.0)"#);
    let tension = eval_expr(r#"svg_generate("noise", "tension", 600.0, 400.0)"#);
    assert_ne!(
        calm, tension,
        "different intents should produce different noise patterns (seed must affect output)"
    );
}

#[test]
fn svg_generate_noise_uses_hsl_interpolated_colors() {
    // Block 3 spec: noise dots must be in shades of the palette intent,
    // not arbitrary colors. We verify by checking that the output
    // contains hex color strings (paper→accent interpolation).
    let xml = eval_expr(r#"svg_generate("noise", "energy", 100.0, 100.0)"#);
    // Should contain at least one fill="#......" attribute (6 hex digits).
    // We avoid pulling in regex as a dev-dep by doing a simple substring
    // check: every <circle> element has a fill attribute, and all fills
    // are hsl_to_hex output (#rrggbb).
    let circle_count = xml.matches("<circle ").count();
    let fill_count = xml.matches("fill=\"#").count();
    assert_eq!(
        circle_count, fill_count,
        "every <circle> should have a fill=\"#......\" attribute"
    );
    // All fills should be 6-digit hex (hsl_to_hex output format).
    // We sample-check the first one.
    if let Some(pos) = xml.find("fill=\"#") {
        let after = &xml[pos + 7..];
        let hex: String = after.chars().take(6).collect();
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "fill color should be 6 hex digits, got: {}",
            hex
        );
    }
}

// ── Cross-kind tests ─────────────────────────────────────────────────

#[test]
fn svg_generate_rejects_unknown_kind() {
    let err = eval_err(&wrap(r#"svg_generate("UNKNOWN", "calm", 600.0, 400.0)"#));
    assert!(
        err.contains("kind must be one of") && err.contains("UNKNOWN"),
        "expected kind validation error, got: {}",
        err
    );
}

#[test]
fn svg_generate_rejects_zero_dimensions() {
    let err = eval_err(&wrap(r#"svg_generate("grid", "calm", 0.0, 400.0)"#));
    assert!(err.contains("must be positive"), "got: {}", err);
    let err = eval_err(&wrap(r#"svg_generate("grid", "calm", 600.0, 0.0)"#));
    assert!(err.contains("must be positive"));
}

#[test]
fn svg_generate_rejects_huge_dimensions() {
    // Cap at 10000 — prevents pathological output sizes.
    let err = eval_err(&wrap(r#"svg_generate("noise", "calm", 10001.0, 400.0)"#));
    assert!(
        err.contains("≤ 10000"),
        "expected dimension cap error, got: {}",
        err
    );
}

#[test]
fn svg_generate_tw_vm_crosscheck() {
    // Backend parity: TW and VM must produce byte-identical output.
    // This is the standard crosscheck invariant from Н78/Н79.
    let src = wrap(r#"svg_generate("noise", "energy", 600.0, 400.0)"#);
    let tw_out = metalogos::run_program(&src).unwrap().unwrap();
    let vm_out = eval_vm(&src).unwrap().unwrap();
    assert_eq!(
        tw_out, vm_out,
        "TW and VM must produce byte-identical output for svg_generate"
    );
}

// ── Block 4: svg_canvas_preset ───────────────────────────────────────

#[test]
fn svg_canvas_preset_doc_inline() {
    let xml = eval_expr(
        r#"svg_canvas_preset("doc_inline", "0 0 960 600", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    assert!(xml.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\""));
    assert!(xml.contains("height=\"600\""));
    assert!(xml.ends_with("</svg>"));
}

#[test]
fn svg_canvas_preset_slide_16x9() {
    let xml = eval_expr(
        r#"svg_canvas_preset("slide_16x9", "0 0 1280 720", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    assert!(xml.contains("width=\"1280\""));
    assert!(xml.contains("height=\"720\""));
}

#[test]
fn svg_canvas_preset_social_og() {
    let xml = eval_expr(
        r#"svg_canvas_preset("social_og", "0 0 1200 632", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    assert!(xml.contains("width=\"1200\""));
    assert!(xml.contains("height=\"632\""));
}

#[test]
fn svg_canvas_preset_print_a4_landscape() {
    let xml = eval_expr(
        r#"svg_canvas_preset("print_a4_landscape", "0 0 1122 793", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    assert!(xml.contains("width=\"1122\""));
    assert!(xml.contains("height=\"793\""));
}

#[test]
fn svg_canvas_preset_print_a4_portrait() {
    let xml = eval_expr(
        r#"svg_canvas_preset("print_a4_portrait", "0 0 793 1122", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    assert!(xml.contains("width=\"793\""));
    assert!(xml.contains("height=\"1122\""));
}

#[test]
fn svg_canvas_preset_rejects_unknown_name() {
    let err = eval_err(&wrap(
        r#"svg_canvas_preset("UNKNOWN_PRESET", "0 0 100 100", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    ));
    assert!(
        err.contains("unknown preset") && err.contains("UNKNOWN_PRESET"),
        "expected unknown preset error, got: {}",
        err
    );
    // Error message MUST list the available preset names (no silent fallthrough).
    assert!(err.contains("doc_inline"));
    assert!(err.contains("slide_16x9"));
    assert!(err.contains("social_og"));
    assert!(err.contains("print_a4_landscape"));
    assert!(err.contains("print_a4_portrait"));
}

#[test]
fn svg_canvas_preset_backward_compat_svg_canvas_unchanged() {
    // The original svg_canvas signature must still work identically.
    // svg_canvas_preset is a pure wrapper — it must not alter svg_canvas.
    let direct = eval_expr(
        r#"svg_canvas(960.0, 600.0, "0 0 960 600", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    let via_preset = eval_expr(
        r#"svg_canvas_preset("doc_inline", "0 0 960 600", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    assert_eq!(
        direct, via_preset,
        "svg_canvas_preset must produce identical output to svg_canvas with same w/h"
    );
}

#[test]
fn svg_canvas_preset_tw_vm_crosscheck() {
    let src = wrap(
        r#"svg_canvas_preset("slide_16x9", "0 0 1280 720", [svg_rect(0.0, 0.0, 10.0, 10.0, "white", "none")])"#,
    );
    let tw_out = metalogos::run_program(&src).unwrap().unwrap();
    let vm_out = eval_vm(&src).unwrap().unwrap();
    assert_eq!(
        tw_out, vm_out,
        "TW and VM must agree on svg_canvas_preset output"
    );
}
