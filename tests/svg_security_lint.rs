// ── Наряд №74: Integration tests for SVG/HTML Security Lint ──────────
//
// Verifies that `mlog check` (semantic.rs::svg_security_lint) catches
// potential XSS/injection vectors that could bypass runtime escaping.
//
// Test categories:
//   1. svg_text content with <script> → WARNING (runtime escapes it)
//   2. svg_path d with <script> → ERROR (runtime does NOT escape d)
//   3. svg_canvas viewbox with <script> → ERROR (structural arg)
//   4. svg_callout text with <script> → WARNING (runtime escapes)
//   5. javascript: URL scheme → ERROR (any builtin)
//   6. onX event handler attributes → ERROR/WARNING depending on builtin
//   7. Clean SVG program → no errors, no warnings

use metalogos::check_program;

fn errors(result: &metalogos::semantic::AnalysisResult) -> Vec<&str> {
    result.errors.iter().map(|s| s.as_str()).collect()
}

fn warnings(result: &metalogos::semantic::AnalysisResult) -> Vec<&str> {
    result.warnings.iter().map(|s| s.as_str()).collect()
}

// ── 1. svg_text content with <script> → WARNING (auto-escaped) ───────

#[test]
fn svg_text_script_literal_warns_but_does_not_error() {
    let src = "pattern P(input: String) -> String {\n    return svg_text(10.0, 10.0, \"<script>alert(1)</script>\", 14.0, \"#000\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    // Should NOT be a hard error — runtime will escape <script> in svg_text content
    assert!(
        !errors(&r).iter().any(|e| e.contains("svg_text")),
        "svg_text with <script> should not produce an error (runtime escapes), got: {:?}",
        errors(&r)
    );
    // MAY produce a warning (suspicious intent — worth reviewing)
    let has_warning = warnings(&r).iter().any(|w| w.contains("svg_text") && w.contains("script"));
    assert!(
        has_warning,
        "svg_text with <script> should produce a warning (suspicious intent), warnings: {:?}",
        warnings(&r)
    );
}

// ── 2. svg_path d with <script> → ERROR (NOT escaped, structural) ────

#[test]
fn svg_path_d_with_script_literal_errors() {
    let src = "pattern P(input: String) -> String {\n    return svg_path(\"M 10 10 <script>\", \"none\", \"black\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let errs = errors(&r);
    assert!(
        errs.iter().any(|e| e.contains("svg_path") && e.contains("does NOT auto-escape")),
        "svg_path with <script> in d arg should ERROR (no auto-escape), errors: {:?}",
        errs
    );
}

// ── 3. svg_canvas viewbox with <script> → ERROR (structural arg) ─────

#[test]
fn svg_canvas_viewbox_with_script_errors() {
    let src = "pattern P(input: String) -> String {\n    return svg_canvas(200.0, 100.0, \"0 0 200 <script>\", [])\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let errs = errors(&r);
    assert!(
        errs.iter().any(|e| e.contains("svg_canvas") && e.contains("does NOT auto-escape")),
        "svg_canvas with <script> in viewbox should ERROR, errors: {:?}",
        errs
    );
}

// ── 4. svg_callout text with <script> → WARNING (auto-escaped) ───────

#[test]
fn svg_callout_text_with_script_warns() {
    let src = "pattern P(input: String) -> String {\n    return svg_callout(\"<script>x</script>\", 10.0, 10.0, 100.0, 50.0)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("svg_callout")),
        "svg_callout text with <script> should not error (auto-escaped), errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("svg_callout") && w.contains("script"));
    assert!(
        has_warning,
        "svg_callout with <script> should warn, warnings: {:?}",
        warnings(&r)
    );
}

// ── 5. javascript: URL scheme → ERROR ────────────────────────────────

#[test]
fn javascript_url_scheme_in_svg_arg_errors() {
    let src = "pattern P(input: String) -> String {\n    return svg_text(10.0, 10.0, \"hello\", 14.0, \"javascript:alert(1)\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let errs = errors(&r);
    assert!(
        errs.iter().any(|e| e.contains("javascript:") || e.contains("dangerous URL")),
        "javascript: URL scheme should ERROR, errors: {:?}",
        errs
    );
}

// ── 6. onX event handler attributes ──────────────────────────────────

#[test]
fn onx_attribute_in_svg_path_errors() {
    // svg_path d arg contains onclick= → ERROR
    let src = "pattern P(input: String) -> String {\n    return svg_path(\"M 10 10 onload=alert(1)\", \"none\", \"black\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let errs = errors(&r);
    assert!(
        errs.iter().any(|e| e.contains("onX") || e.contains("event handler")),
        "onX attribute in svg_path should ERROR, errors: {:?}",
        errs
    );
}

#[test]
fn onx_attribute_in_svg_text_warns() {
    // svg_text content arg contains onclick= → WARNING (auto-escaped)
    let src = "pattern P(input: String) -> String {\n    return svg_text(10.0, 10.0, \"hello onload=alert(1)\", 14.0, \"#000\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let warns = warnings(&r);
    assert!(
        warns.iter().any(|w| w.contains("onX") || w.contains("event handler")),
        "onX in svg_text content should WARN (auto-escaped), warnings: {:?}",
        warns
    );
}

// ── 7. Clean SVG program → no security errors/warnings ───────────────

#[test]
fn clean_svg_program_has_no_security_findings() {
    let src = "pattern P(input: String) -> String {\n    let r = svg_rect(10.0, 10.0, 100.0, 50.0, \"#eb6c36\", \"none\")\n    let t = svg_text(20.0, 40.0, \"Hello\", 14.0, \"#2d3142\", \"start\")\n    return svg_canvas(200.0, 100.0, \"0 0 200 100\", [r, t])\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_errors: Vec<_> = errors(&r).into_iter().filter(|e| e.contains("security:")).collect();
    let sec_warnings: Vec<_> = warnings(&r).into_iter().filter(|w| w.contains("security:")).collect();
    assert!(
        sec_errors.is_empty(),
        "clean SVG program should have no security errors, got: {:?}",
        sec_errors
    );
    assert!(
        sec_warnings.is_empty(),
        "clean SVG program should have no security warnings, got: {:?}",
        sec_warnings
    );
}

// ── 8. chart_bar with normal data → no findings ──────────────────────

#[test]
fn chart_bar_clean_program_passes_lint() {
    let src = "pattern P(input: String) -> String {\n    let data = [{label: \"A\", value: 10.0}, {label: \"B\", value: 20.0}]\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_bar(data, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_errors: Vec<_> = errors(&r).into_iter().filter(|e| e.contains("security:")).collect();
    assert!(
        sec_errors.is_empty(),
        "chart_bar clean program should have no security errors, got: {:?}",
        sec_errors
    );
}

// ── 9. Defense-in-depth: script in concat → checked on both sides ────

#[test]
fn script_in_string_concat_to_svg_path_errors() {
    // svg_path("M 10 10 " + "<script>", "none", "black") — the <script> is in a concat
    // We can't fully trace concat flow without type info, but we DO scan all
    // string literals in the call tree.
    let src = "pattern P(input: String) -> String {\n    return svg_path(\"M 10 10 \" + \"<script>\", \"none\", \"black\")\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    // The lint walks BinaryOp and scans all string literals — should catch the <script>
    let found = errors(&r)
        .iter()
        .chain(warnings(&r).iter())
        .any(|m| m.contains("script"));
    assert!(
        found,
        "script in concat should be detected somewhere, errors: {:?}, warnings: {:?}",
        errors(&r),
        warnings(&r)
    );
}
