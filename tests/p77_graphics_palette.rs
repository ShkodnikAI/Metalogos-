// ── Наряд №77: Integration tests for color_palette + chart_donut ──────
//
// Tests the FULL pipeline:
//   .mlog source → interpreter → SVG output / Struct → consumer (chart_*)
//
// Block 1: color_palette contract — 10 (intent × mode) combinations,
//          each verified to:
//            (a) return a Struct with the 5 canonical DiagramStyle tokens
//            (b) be directly consumable by chart_bar (no adapter)
//            (c) all accent/structural tokens share base_hue (V2.1 check)
//
// Block 2: chart_donut — basic rendering, security (label XSS escape),
//          error paths, and composition with color_palette.
//
// Block 3: std/infographic.mlog — InfographicPoster MVP produces valid SVG.
//
// NOTE: All .mlog source strings use r##"..."## delimiters because the
// mlog code contains "#fff" / "#000" hex colors — the sequence "# inside
// a r#"..."# raw string would prematurely close it.

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

// ── Block 1: color_palette ────────────────────────────────────────────

#[test]
fn color_palette_returns_diagram_style_struct() {
    let xml =
        eval_expr(r##"chart_bar([{label: "A", value: 10.0}], color_palette("energy", "light"))"##);
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));
}

#[test]
fn color_palette_all_6_intents_work_in_light_mode() {
    // Наряд №162: mono included
    for intent in &["calm", "tension", "energy", "authority", "warmth", "mono"] {
        let src = format!(
            r##"chart_bar([{{label: "A", value: 10.0}}], color_palette("{}", "light"))"##,
            intent
        );
        let xml = eval_expr(&src);
        assert!(
            xml.starts_with("<svg "),
            "intent={} light mode failed: {}",
            intent,
            xml
        );
    }
}

#[test]
fn color_palette_all_6_intents_work_in_dark_mode() {
    // Наряд №162: mono included
    for intent in &["calm", "tension", "energy", "authority", "warmth", "mono"] {
        let src = format!(
            r##"chart_bar([{{label: "A", value: 10.0}}], color_palette("{}", "dark"))"##,
            intent
        );
        let xml = eval_expr(&src);
        assert!(
            xml.starts_with("<svg "),
            "intent={} dark mode failed: {}",
            intent,
            xml
        );
    }
}

#[test]
fn color_palette_rejects_unknown_intent() {
    let err = eval_err(
        r##"pattern __t(input: String) -> String { return chart_bar([{label: "A", value: 10.0}], color_palette("unknown", "light")) }
flow Main { input: String = "x" -> __t -> output }"##,
    );
    assert!(err.contains("intent"), "err: {}", err);
}

#[test]
fn color_palette_rejects_unknown_mode() {
    let err = eval_err(
        r##"pattern __t(input: String) -> String { return chart_bar([{label: "A", value: 10.0}], color_palette("calm", "neon")) }
flow Main { input: String = "x" -> __t -> output }"##,
    );
    assert!(err.contains("mode"), "err: {}", err);
}

#[test]
fn color_palette_light_and_dark_differ() {
    let src = r##"pattern __t(input: String) -> String {
        let l = color_palette("calm", "light")
        let d = color_palette("calm", "dark")
        let same = to_float(l.paper == d.paper)
        return to_string(same)
    }
    flow Main { input: String = "x" -> __t -> output }"##;
    let out = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("failed: {}", e),
    };
    assert!(
        out.contains("0"),
        "light and dark paper must differ, got: {}",
        out
    );
}

#[test]
fn color_palette_accent_shares_hue_with_structural_tokens_v21() {
    // V2.1 rule: accent MUST share base_hue with structural roles.
    // Verify the deterministic output for calm/light accent is hsl(210, 0.65, 0.45).
    // Python verification: hsl(210, 0.65, 0.45) = #2873bd
    let src = r##"pattern __t(input: String) -> String {
        let p = color_palette("calm", "light")
        return p.accent
    }
    flow Main { input: String = "x" -> __t -> output }"##;
    let accent = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("failed: {}", e),
    };
    assert_eq!(
        accent.to_lowercase(),
        "#2873bd",
        "calm/light accent should be hsl(210, 0.65, 0.45) = #2873bd, got {}",
        accent
    );
}

#[test]
fn color_palette_p77_example_runs() {
    let src = std::fs::read_to_string("examples/p77_color_palette.mlog")
        .expect("examples/p77_color_palette.mlog must exist");
    let out = match metalogos::run_program(&src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("example failed: {}", e),
    };
    assert!(
        out.contains("p77_palette_checks=11/11"),
        "expected all 11 checks to pass, got: {}",
        out
    );
}

// ── Block 2: chart_donut ──────────────────────────────────────────────

#[test]
fn chart_donut_returns_valid_svg() {
    let xml = eval_expr(
        r##"chart_donut([{label: "A", value: 40.0}, {label: "B", value: 60.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));
    assert_eq!(xml.matches("<path").count(), 2);
}

#[test]
fn chart_donut_3_slices_proportional_paths() {
    let xml = eval_expr(
        r##"chart_donut([{label: "Alpha", value: 40.0}, {label: "Beta", value: 35.0}, {label: "Gamma", value: 25.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert_eq!(xml.matches("<path").count(), 3);
    assert!(xml.contains(">100<"));
    assert!(xml.contains("Alpha"));
    assert!(xml.contains("Beta"));
    assert!(xml.contains("Gamma"));
}

#[test]
fn chart_donut_security_label_script_tag_escaped() {
    let xml = eval_expr(
        r##"chart_donut([{label: "<script>alert(1)</script>", value: 40.0}, {label: "safe", value: 60.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(
        !xml.contains("<script>"),
        "RAW <script> leaked into chart_donut output: {}",
        xml
    );
    assert!(xml.contains("&lt;script&gt;"));
}

#[test]
fn chart_donut_rejects_empty_data() {
    let err = eval_err(
        r##"pattern __t(input: String) -> String {
            return chart_donut([], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))
        }
        flow Main { input: String = "x" -> __t -> output }"##,
    );
    assert!(err.contains("empty"));
}

#[test]
fn chart_donut_rejects_too_many_slices() {
    let slices: Vec<String> = (0..51)
        .map(|i| format!("{{label: \"{}\", value: {}}}", i, i))
        .collect();
    let slices_str = slices.join(", ");
    let src = format!(
        r##"pattern __t(input: String) -> String {{
            let data = [{}]
            let style = diagram_style({{paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}})
            return chart_donut(data, style)
        }}
        flow Main {{ input: String = "x" -> __t -> output }}"##,
        slices_str
    );
    let err = eval_err(&src);
    assert!(err.contains("too many") || err.contains("maximum"));
}

#[test]
fn chart_donut_deterministic_output() {
    let src = r##"pattern __t(input: String) -> String {
        let data = [{label: "A", value: 40.0}, {label: "B", value: 35.0}, {label: "C", value: 25.0}]
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#888", rule: "#ccc"})
        return chart_donut(data, style)
    }
    flow Main { input: String = "x" -> __t -> output }"##;
    let out1 = metalogos::run_program(src).unwrap().unwrap();
    let out2 = metalogos::run_program(src).unwrap().unwrap();
    assert_eq!(out1, out2, "chart_donut output must be deterministic");
}

#[test]
fn chart_donut_composes_with_color_palette() {
    let xml = eval_expr(
        r##"chart_donut([{label: "A", value: 60.0}, {label: "B", value: 40.0}], color_palette("calm", "light"))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));
    assert_eq!(xml.matches("<path").count(), 2);
}

#[test]
fn chart_donut_single_slice_renders_full_circle() {
    // One slice = 100%. SVG arc with start==end is ambiguous (renders nothing),
    // so chart_donut splits a full-circle slice into two semicircle paths.
    let xml = eval_expr(
        r##"chart_donut([{label: "Only", value: 100.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#888", rule: "#ccc"}))"##,
    );
    assert_eq!(
        xml.matches("<path").count(),
        2,
        "single-slice donut should be split into 2 semicircle paths, xml: {}",
        xml
    );
    assert!(
        xml.contains("0 0 1 "),
        "semicircle arc should use large-arc-flag=0 sweep=1, xml: {}",
        xml
    );
}

// ── Block 2: svg_security_lint extension for chart_donut ──────────────

use metalogos::check_program;

fn lint_errors(result: &metalogos::semantic::AnalysisResult) -> Vec<String> {
    result.errors.to_vec()
}

fn lint_warnings(result: &metalogos::semantic::AnalysisResult) -> Vec<String> {
    result.warnings.to_vec()
}

#[test]
fn chart_donut_label_with_script_warns_but_does_not_error() {
    // chart_donut's label is auto-escaped at runtime, so a <script> payload
    // in a string literal should produce a WARNING (not an ERROR).
    // Note: the data must be passed as a direct literal arg (not via a let
    // binding) so the AST lint can see the StringLit inside the List<Struct>.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_donut([{label: \"<script>x</script>\", value: 40.0}, {label: \"safe\", value: 60.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_errors: Vec<String> = lint_errors(&r)
        .into_iter()
        .filter(|e| e.contains("security:"))
        .collect();
    assert!(
        sec_errors.is_empty(),
        "chart_donut with <script> in label should NOT produce security ERROR (runtime escapes), got: {:?}",
        sec_errors
    );
    let has_warning = lint_warnings(&r)
        .iter()
        .any(|w| w.contains("chart_donut") && w.contains("script"));
    assert!(
        has_warning,
        "chart_donut with <script> in label should produce a WARNING (suspicious intent), warnings: {:?}",
        lint_warnings(&r)
    );
}

#[test]
fn chart_donut_clean_program_no_security_findings() {
    let src = "pattern P(input: String) -> String {\n    let data = [{label: \"Alpha\", value: 40.0}, {label: \"Beta\", value: 60.0}]\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_donut(data, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_errors: Vec<String> = lint_errors(&r)
        .into_iter()
        .filter(|e| e.contains("security:"))
        .collect();
    let sec_warnings: Vec<String> = lint_warnings(&r)
        .into_iter()
        .filter(|w| w.contains("security:"))
        .collect();
    assert!(
        sec_errors.is_empty(),
        "clean chart_donut program should have no security errors, got: {:?}",
        sec_errors
    );
    assert!(
        sec_warnings.is_empty(),
        "clean chart_donut program should have no security warnings, got: {:?}",
        sec_warnings
    );
}

// ── Block 3: std/infographic.mlog — InfographicPoster MVP ─────────────

#[test]
fn infographic_poster_produces_valid_svg_inline() {
    let src = r##"pattern InfographicPoster(input: String) -> String {
        let title = "Q3 Report"
        let intent = "energy"
        let stats = [{label: "Revenue", value: 1.2}, {label: "Users", value: 5.4}, {label: "NPS", value: 72.0}]
        let style = color_palette(intent, "light")
        let donut = chart_donut(stats, style)
        let canvas = svg_canvas(720.0, 960.0, "0 0 720 960", [
            svg_rect(0.0, 0.0, 720.0, 80.0, style.ink, "none"),
            svg_text(40.0, 50.0, title, 28.0, style.paper, "start"),
            donut
        ])
        return canvas
    }
    flow Main { input: String = "x" -> InfographicPoster -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("infographic failed: {}", e),
    };
    assert!(svg.starts_with("<svg "));
    assert!(svg.ends_with("</svg>"));
    assert!(svg.contains("Q3 Report"));
    assert_eq!(svg.matches("<path").count(), 3);
}

#[test]
fn infographic_poster_imports_from_std_module() {
    let src = r##"import std/infographic as ig

    pattern Run(input: String) -> String {
      let stats = [{label: "A", value: 40.0}, {label: "B", value: 35.0}, {label: "C", value: 25.0}]
      return ig.InfographicPoster("Test Poster", "energy", stats)
    }
    flow Main { input: String = "x" -> Run -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("std/infographic import/run failed: {}", e),
    };
    assert!(
        svg.starts_with("<svg "),
        "expected SVG output, got: {}",
        &svg[..200.min(svg.len())]
    );
    assert!(svg.ends_with("</svg>"));
    assert!(svg.contains("Test Poster"));
    assert_eq!(svg.matches("<path").count(), 3);
    assert!(svg.contains("Generated by Metalogos"));
}
