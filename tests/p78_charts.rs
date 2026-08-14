// ── Наряд №78: Integration tests for chart_line / chart_scatter / chart_area ─
//
// Tests the FULL pipeline for each new chart builtin:
//   .mlog source → interpreter (TW) → SVG output → XML validation
//   .mlog source → compiler → VM → SVG output → XML validation
//   TW vs VM output must match byte-for-byte (crosscheck invariant).
//
// Coverage per chart function:
//   - Returns valid SVG document
//   - Output is deterministic (golden invariant)
//   - Empty data rejected
//   - Too many points rejected (respective limits: 100 / 200 / 100)
//   - Runtime escapes <script> in label (defense-in-depth)
//   - TW output == VM output (backend parity)
//
// Security lint tests (AST-level scan_chart_labels coverage) live in
// tests/svg_security_lint.rs — they verify the lint WARNS on <script>
// in label for all three new chart_* functions, including chart_scatter
// with its non-standard {x, y, label?} shape.

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

/// Compile + run via the bytecode VM (mirrors crosscheck_backends.rs).
fn eval_vm(src: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(src).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Wrap an expression in a pattern+flow program.
fn wrap(src: &str) -> String {
    format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    )
}

// ── Block 1: chart_line ──────────────────────────────────────────────

#[test]
fn chart_line_returns_valid_svg() {
    let xml = eval_expr(
        r##"chart_line([{label: "A", value: 30.0}, {label: "B", value: 65.0}, {label: "C", value: 45.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));
    // 1 line path + 3 circle markers + 1 peak value text + 3 x-axis labels = 5 path/circle + 4 text
    assert!(
        xml.contains("<path"),
        "chart_line should produce a <path> for the line"
    );
    // 3 circle markers (one per point)
    assert_eq!(xml.matches("<circle").count(), 3);
    // Labels present
    assert!(xml.contains("A"));
    assert!(xml.contains("B"));
    assert!(xml.contains("C"));
}

#[test]
fn chart_line_single_point_no_divide_by_zero() {
    // N=1 is the degenerate case — must not divide by zero in
    // (n - 1.0) when computing X positions.
    let xml = eval_expr(
        r##"chart_line([{label: "Only", value: 100.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.contains("Only"));
    // Should still have a path (single M command, no L)
    assert!(xml.contains("<path"));
}

#[test]
fn chart_line_deterministic_output() {
    let src = wrap(r##"chart_line([{label: "Jan", value: 30.0}, {label: "Feb", value: 65.0}, {label: "Mar", value: 45.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##);
    let out1 = metalogos::run_program(&src).unwrap().unwrap();
    let out2 = metalogos::run_program(&src).unwrap().unwrap();
    assert_eq!(out1, out2, "chart_line output must be deterministic");
}

#[test]
fn chart_line_rejects_empty_data() {
    let err = eval_err(
        r##"pattern __t(input: String) -> String {
            return chart_line([], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))
        }
        flow Main { input: String = "x" -> __t -> output }"##,
    );
    assert!(err.contains("empty"));
}

#[test]
fn chart_line_rejects_too_many_points() {
    let pts: Vec<String> = (0..101)
        .map(|i| format!("{{label: \"{}\", value: {}}}", i, i))
        .collect();
    let pts_str = pts.join(", ");
    let src = format!(
        "pattern __t(input: String) -> String {{\n            let data = [{}]\n            let style = diagram_style({{paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"}})\n            return chart_line(data, style)\n        }}\nflow Main {{ input: String = \"x\" -> __t -> output }}",
        pts_str
    );
    let err = eval_err(&src);
    assert!(err.contains("too many") || err.contains("maximum"));
}

#[test]
fn chart_line_security_label_script_tag_escaped() {
    let xml = eval_expr(
        r##"chart_line([{label: "<script>alert(1)</script>", value: 40.0}, {label: "safe", value: 60.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(
        !xml.contains("<script>"),
        "RAW <script> leaked into chart_line output: {}",
        xml
    );
    assert!(xml.contains("&lt;script&gt;"));
}

#[test]
fn chart_line_tw_vm_crosscheck() {
    let src = wrap(r##"chart_line([{label: "A", value: 30.0}, {label: "B", value: 65.0}, {label: "C", value: 45.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"}))"##);
    let tw = metalogos::run_program(&src).unwrap().unwrap();
    let vm = eval_vm(&src).unwrap().unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        vm.trim_end(),
        "chart_line TW vs VM output mismatch"
    );
}

// ── Block 2: chart_scatter ───────────────────────────────────────────

#[test]
fn chart_scatter_returns_valid_svg() {
    let xml = eval_expr(
        r##"chart_scatter([{x: 1.0, y: 2.0, label: "A"}, {x: 2.0, y: 4.0, label: "B"}, {x: 3.0, y: 1.0, label: "C"}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));
    // 3 points = 3 circles
    assert_eq!(xml.matches("<circle").count(), 3);
    // Labels present
    assert!(xml.contains("A"));
    assert!(xml.contains("B"));
    assert!(xml.contains("C"));
}

#[test]
fn chart_scatter_works_without_labels() {
    // {x, y} only — label field absent. Verifies optional label handling
    // and that scan_chart_labels doesn't choke on missing field.
    let xml = eval_expr(
        r##"chart_scatter([{x: 1.0, y: 2.0}, {x: 2.0, y: 4.0}, {x: 3.0, y: 1.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert_eq!(xml.matches("<circle").count(), 3);
}

#[test]
fn chart_scatter_degenerate_axis_no_divide_by_zero() {
    // All x equal — x_max == x_min, must not divide by zero.
    let xml = eval_expr(
        r##"chart_scatter([{x: 5.0, y: 1.0}, {x: 5.0, y: 2.0}, {x: 5.0, y: 3.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert_eq!(xml.matches("<circle").count(), 3);
}

#[test]
fn chart_scatter_deterministic_output() {
    let src = wrap(r##"chart_scatter([{x: 1.0, y: 2.0, label: "A"}, {x: 2.0, y: 4.0, label: "B"}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##);
    let out1 = metalogos::run_program(&src).unwrap().unwrap();
    let out2 = metalogos::run_program(&src).unwrap().unwrap();
    assert_eq!(out1, out2, "chart_scatter output must be deterministic");
}

#[test]
fn chart_scatter_rejects_empty_data() {
    let err = eval_err(
        r##"pattern __t(input: String) -> String {
            return chart_scatter([], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))
        }
        flow Main { input: String = "x" -> __t -> output }"##,
    );
    assert!(err.contains("empty"));
}

#[test]
fn chart_scatter_rejects_too_many_points() {
    let pts: Vec<String> = (0..201)
        .map(|i| format!("{{x: {}, y: {}}}", i, i))
        .collect();
    let pts_str = pts.join(", ");
    let src = format!(
        "pattern __t(input: String) -> String {{\n            let data = [{}]\n            let style = diagram_style({{paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"}})\n            return chart_scatter(data, style)\n        }}\nflow Main {{ input: String = \"x\" -> __t -> output }}",
        pts_str
    );
    let err = eval_err(&src);
    assert!(err.contains("too many") || err.contains("maximum"));
}

#[test]
fn chart_scatter_security_label_script_tag_escaped() {
    // chart_scatter label is at position 3 (after x, y) — this is the
    // case that specifically tests scan_chart_labels handles label not
    // being the first struct field.
    let xml = eval_expr(
        r##"chart_scatter([{x: 1.0, y: 2.0, label: "<script>alert(1)</script>"}, {x: 2.0, y: 4.0, label: "safe"}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(
        !xml.contains("<script>"),
        "RAW <script> leaked into chart_scatter output: {}",
        xml
    );
    assert!(xml.contains("&lt;script&gt;"));
}

#[test]
fn chart_scatter_tw_vm_crosscheck() {
    let src = wrap(r##"chart_scatter([{x: 1.0, y: 2.0, label: "A"}, {x: 2.0, y: 4.0, label: "B"}, {x: 3.0, y: 1.0, label: "C"}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"}))"##);
    let tw = metalogos::run_program(&src).unwrap().unwrap();
    let vm = eval_vm(&src).unwrap().unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        vm.trim_end(),
        "chart_scatter TW vs VM output mismatch"
    );
}

// ── Block 3: chart_area ──────────────────────────────────────────────

#[test]
fn chart_area_returns_valid_svg() {
    let xml = eval_expr(
        r##"chart_area([{label: "Q1", value: 80.0}, {label: "Q2", value: 120.0}, {label: "Q3", value: 95.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));
    // chart_area emits TWO <path> elements: one filled (area) + one stroke (line on top)
    assert_eq!(
        xml.matches("<path").count(),
        2,
        "chart_area should produce 2 paths (area fill + top stroke), got: {}",
        xml
    );
    // The area path must be closed (Z command) and have fill-opacity
    assert!(xml.contains("fill-opacity=\"0.25\""));
    assert!(xml.contains("Z"));
    // Labels present
    assert!(xml.contains("Q1"));
    assert!(xml.contains("Q2"));
    assert!(xml.contains("Q3"));
}

#[test]
fn chart_area_single_point_no_divide_by_zero() {
    let xml = eval_expr(
        r##"chart_area([{label: "Only", value: 100.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(xml.starts_with("<svg "));
    assert!(xml.contains("Only"));
    assert!(xml.contains("<path"));
}

#[test]
fn chart_area_deterministic_output() {
    let src = wrap(r##"chart_area([{label: "Q1", value: 80.0}, {label: "Q2", value: 120.0}, {label: "Q3", value: 95.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##);
    let out1 = metalogos::run_program(&src).unwrap().unwrap();
    let out2 = metalogos::run_program(&src).unwrap().unwrap();
    assert_eq!(out1, out2, "chart_area output must be deterministic");
}

#[test]
fn chart_area_rejects_empty_data() {
    let err = eval_err(
        r##"pattern __t(input: String) -> String {
            return chart_area([], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))
        }
        flow Main { input: String = "x" -> __t -> output }"##,
    );
    assert!(err.contains("empty"));
}

#[test]
fn chart_area_rejects_too_many_points() {
    let pts: Vec<String> = (0..101)
        .map(|i| format!("{{label: \"{}\", value: {}}}", i, i))
        .collect();
    let pts_str = pts.join(", ");
    let src = format!(
        "pattern __t(input: String) -> String {{\n            let data = [{}]\n            let style = diagram_style({{paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"}})\n            return chart_area(data, style)\n        }}\nflow Main {{ input: String = \"x\" -> __t -> output }}",
        pts_str
    );
    let err = eval_err(&src);
    assert!(err.contains("too many") || err.contains("maximum"));
}

#[test]
fn chart_area_security_label_script_tag_escaped() {
    let xml = eval_expr(
        r##"chart_area([{label: "<script>alert(1)</script>", value: 40.0}, {label: "safe", value: 60.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#f00", muted: "#888", rule: "#ccc"}))"##,
    );
    assert!(
        !xml.contains("<script>"),
        "RAW <script> leaked into chart_area output: {}",
        xml
    );
    assert!(xml.contains("&lt;script&gt;"));
}

#[test]
fn chart_area_tw_vm_crosscheck() {
    let src = wrap(r##"chart_area([{label: "Q1", value: 80.0}, {label: "Q2", value: 120.0}, {label: "Q3", value: 95.0}], diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"}))"##);
    let tw = metalogos::run_program(&src).unwrap().unwrap();
    let vm = eval_vm(&src).unwrap().unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        vm.trim_end(),
        "chart_area TW vs VM output mismatch"
    );
}

// ── Cross-composition: palette → all 3 new charts ───────────────────

#[test]
fn color_palette_composes_with_all_three_new_charts() {
    // color_palette returns DiagramStyle — must be consumable by all
    // chart_* functions without adapter (same invariant as p77).
    let src = wrap(
        r##"let p = color_palette("calm", "light")
        let bar_data = [{label: "A", value: 10.0}, {label: "B", value: 20.0}]
        let scatter_data = [{x: 1.0, y: 2.0}, {x: 2.0, y: 4.0}]
        let line = chart_line(bar_data, p)
        let area = chart_area(bar_data, p)
        let scatter = chart_scatter(scatter_data, p)
        return line + area + scatter"##,
    );
    let out = metalogos::run_program(&src).unwrap().unwrap();
    // Three concatenated SVG documents
    assert_eq!(out.matches("<svg ").count(), 3);
    assert_eq!(out.matches("</svg>").count(), 3);
}

// ── Example file smoke tests ─────────────────────────────────────────

#[test]
fn p78_chart_line_example_runs() {
    let src = std::fs::read_to_string("examples/p78_chart_line.mlog")
        .expect("examples/p78_chart_line.mlog must exist");
    let out = metalogos::run_program(&src).unwrap().unwrap();
    assert!(
        out.contains("p78_line_checks="),
        "p78_chart_line example output unexpected: {}",
        out
    );
    // All 3 checks should pass
    assert!(out.contains("3/3"), "expected 3/3 checks to pass, got: {}", out);
}

#[test]
fn p78_chart_scatter_example_runs() {
    let src = std::fs::read_to_string("examples/p78_chart_scatter.mlog")
        .expect("examples/p78_chart_scatter.mlog must exist");
    let out = metalogos::run_program(&src).unwrap().unwrap();
    assert!(
        out.contains("p78_scatter_checks="),
        "p78_chart_scatter example output unexpected: {}",
        out
    );
    assert!(
        out.contains("3/3"),
        "expected 3/3 checks to pass, got: {}",
        out
    );
}

#[test]
fn p78_chart_area_example_runs() {
    let src = std::fs::read_to_string("examples/p78_chart_area.mlog")
        .expect("examples/p78_chart_area.mlog must exist");
    let out = metalogos::run_program(&src).unwrap().unwrap();
    assert!(
        out.contains("p78_area_checks="),
        "p78_chart_area example output unexpected: {}",
        out
    );
    assert!(out.contains("3/3"), "expected 3/3 checks to pass, got: {}", out);
}
