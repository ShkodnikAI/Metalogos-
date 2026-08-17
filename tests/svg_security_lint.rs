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
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("svg_text") && w.contains("script"));
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
        errs.iter()
            .any(|e| e.contains("svg_path") && e.contains("does NOT auto-escape")),
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
        errs.iter()
            .any(|e| e.contains("svg_canvas") && e.contains("does NOT auto-escape")),
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
        errs.iter()
            .any(|e| e.contains("javascript:") || e.contains("dangerous URL")),
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
        errs.iter()
            .any(|e| e.contains("onX") || e.contains("event handler")),
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
        warns
            .iter()
            .any(|w| w.contains("onX") || w.contains("event handler")),
        "onX in svg_text content should WARN (auto-escaped), warnings: {:?}",
        warns
    );
}

// ── 7. Clean SVG program → no security errors/warnings ───────────────

#[test]
fn clean_svg_program_has_no_security_findings() {
    let src = "pattern P(input: String) -> String {\n    let r = svg_rect(10.0, 10.0, 100.0, 50.0, \"#eb6c36\", \"none\")\n    let t = svg_text(20.0, 40.0, \"Hello\", 14.0, \"#2d3142\", \"start\")\n    return svg_canvas(200.0, 100.0, \"0 0 200 100\", [r, t])\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_errors: Vec<_> = errors(&r)
        .into_iter()
        .filter(|e| e.contains("security:"))
        .collect();
    let sec_warnings: Vec<_> = warnings(&r)
        .into_iter()
        .filter(|w| w.contains("security:"))
        .collect();
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
    let sec_errors: Vec<_> = errors(&r)
        .into_iter()
        .filter(|e| e.contains("security:"))
        .collect();
    assert!(
        sec_errors.is_empty(),
        "chart_bar clean program should have no security errors, got: {:?}",
        sec_errors
    );
}

// ── 10. chart_line / chart_area / chart_scatter: <script> in label → WARN ──
//
// Наряд №78 Block 4: scan_chart_labels must cover the three new chart_*
// builtins, including chart_scatter with its non-standard {x, y, label?}
// shape (label is the THIRD field, optional). The scanner looks up
// `label` BY NAME (HashMap key), so field position doesn't matter —
// but this test exists precisely to catch the regression where someone
// adds chart_scatter to SVG_AUTO_ESCAPE_BUILTINS but forgets to extend
// the `if name == ...` branch in walk_expr_for_svg_security.

#[test]
fn chart_line_label_with_script_warns_but_does_not_error() {
    // Data MUST be passed as a direct literal arg (not via let binding) so
    // the AST lint can see the StringLit inside the List<Struct>.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_line([{label: \"<script>alert(1)</script>\", value: 40.0}, {label: \"safe\", value: 60.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    // No hard error (runtime escapes label)
    assert!(
        !errors(&r).iter().any(|e| e.contains("chart_line")),
        "chart_line with <script> in label should NOT error (runtime escapes), errors: {:?}",
        errors(&r)
    );
    // MUST warn — proves scan_chart_labels was extended to cover chart_line
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("chart_line") && w.contains("script"));
    assert!(
        has_warning,
        "chart_line with <script> in label MUST warn (proves scan_chart_labels covers it), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn chart_area_label_with_script_warns_but_does_not_error() {
    // Data MUST be passed as a direct literal arg (not via let binding).
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_area([{label: \"<script>alert(1)</script>\", value: 40.0}, {label: \"safe\", value: 60.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("chart_area")),
        "chart_area with <script> in label should NOT error (runtime escapes), errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("chart_area") && w.contains("script"));
    assert!(
        has_warning,
        "chart_area with <script> in label MUST warn (proves scan_chart_labels covers it), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn chart_scatter_label_with_script_warns_but_does_not_error() {
    // KEY TEST: chart_scatter has shape {x, y, label?} — label is THIRD,
    // not first. If scan_chart_labels were hardcoded to look at field
    // index 0 (instead of by name), this would NOT fire.
    // Data MUST be passed as a direct literal arg (not via let binding).
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_scatter([{x: 1.0, y: 2.0, label: \"<script>alert(1)</script>\"}, {x: 2.0, y: 4.0, label: \"safe\"}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("chart_scatter")),
        "chart_scatter with <script> in label should NOT error (runtime escapes), errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("chart_scatter") && w.contains("script"));
    assert!(
        has_warning,
        "chart_scatter with <script> in label MUST warn (proves scan_chart_labels handles non-first-position label), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn chart_scatter_without_label_passes_lint_cleanly() {
    // chart_scatter data WITHOUT label field — scan_chart_labels must
    // skip these structs gracefully (no false-positive warning, no error).
    // Data passed as a direct literal so the lint can see the struct shape.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_scatter([{x: 1.0, y: 2.0}, {x: 2.0, y: 4.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    // Bind to a local so the iterator chain can borrow it.
    let errs = errors(&r);
    let warns = warnings(&r);
    let sec_findings: Vec<&str> = errs
        .iter()
        .chain(warns.iter())
        .filter(|m| m.contains("security:"))
        .copied()
        .collect();
    assert!(
        sec_findings.is_empty(),
        "chart_scatter without labels should produce NO security findings, got: {:?}",
        sec_findings
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

// ── 10. Наряд №79: chart_radar and chart_boxplot security lint ───────
//
// chart_radar has a DIFFERENT data shape from chart_bar/donut/line/area/
// scatter: it's a Struct { axes: List<String>, series: List<Struct> }
// rather than a List<Struct>. The scanner has a dedicated branch
// (scan_radar_labels) that walks both `axes` strings and `series[].name`
// strings — these tests verify BOTH paths fire.
//
// chart_boxplot has the same List<Struct{label, ...}> shape as chart_bar,
// so the existing scan_chart_labels covers it. We still add a test to
// pin that coverage (regression catch if someone removes chart_boxplot
// from the dispatch branch in walk_expr_for_svg_security).
//
// chart_heatmap is intentionally NOT scanned — its data is pure numeric
// (List<List<Float>>), no user text. A clean program must produce zero
// findings. (Verified in tests/p79_charts.rs::chart_heatmap_no_security_finding_on_numeric_data.)

#[test]
fn chart_radar_script_in_axis_warns_but_does_not_error() {
    // <script> in axes[0] — scan_radar_labels walks the axes list
    // Data MUST be passed as a direct literal arg (not via let binding) so
    // the AST lint can see the StringLit inside the Struct{axes, series}.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_radar({axes: [\"<script>alert(1)</script>\", \"B\", \"C\"], series: [{name: \"S\", values: [1.0, 2.0, 3.0]}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    // No hard error (runtime escapes axis text)
    assert!(
        !errors(&r).iter().any(|e| e.contains("chart_radar")),
        "chart_radar with <script> in axes should NOT error (runtime escapes), errors: {:?}",
        errors(&r)
    );
    // MUST warn — proves scan_radar_labels was extended to cover axes
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("chart_radar") && w.contains("axes") && w.contains("script"));
    assert!(
        has_warning,
        "chart_radar with <script> in axes MUST warn (proves scan_radar_labels covers axes), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn chart_radar_script_in_series_name_warns_but_does_not_error() {
    // <script> in series[0].name — scan_radar_labels walks the series list
    // and looks up `name` BY KEY (not position).
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_radar({axes: [\"A\", \"B\", \"C\"], series: [{name: \"<script>alert(1)</script>\", values: [1.0, 2.0, 3.0]}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("chart_radar")),
        "chart_radar with <script> in series.name should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("chart_radar") && w.contains("series[].name") && w.contains("script"));
    assert!(
        has_warning,
        "chart_radar with <script> in series.name MUST warn (proves scan_radar_labels covers series[].name), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn chart_radar_clean_program_passes_lint() {
    // No <script> anywhere — should produce zero security findings.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_radar({axes: [\"A\", \"B\", \"C\"], series: [{name: \"S1\", values: [1.0, 2.0, 3.0]}, {name: \"S2\", values: [3.0, 2.0, 1.0]}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_findings: Vec<&str> = r
        .errors
        .iter()
        .chain(r.warnings.iter())
        .filter(|m| m.contains("security:"))
        .map(|s| s.as_str())
        .collect();
    assert!(
        sec_findings.is_empty(),
        "chart_radar clean program should produce NO security findings, got: {:?}",
        sec_findings
    );
}

#[test]
fn chart_boxplot_script_in_label_warns_but_does_not_error() {
    // chart_boxplot has shape {label, values} — label is the first field,
    // but scan_chart_labels looks it up BY NAME so position doesn't matter.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_boxplot([{label: \"<script>alert(1)</script>\", values: [1.0, 2.0, 3.0, 4.0, 5.0]}, {label: \"safe\", values: [10.0, 20.0, 30.0, 40.0, 50.0]}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("chart_boxplot")),
        "chart_boxplot with <script> in label should NOT error (runtime escapes), errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("chart_boxplot") && w.contains("script"));
    assert!(
        has_warning,
        "chart_boxplot with <script> in label MUST warn (proves scan_chart_labels covers chart_boxplot), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn chart_boxplot_clean_program_passes_lint() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return chart_boxplot([{label: \"A\", values: [1.0, 2.0, 3.0, 4.0, 5.0]}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_findings: Vec<&str> = r
        .errors
        .iter()
        .chain(r.warnings.iter())
        .filter(|m| m.contains("security:"))
        .map(|s| s.as_str())
        .collect();
    assert!(
        sec_findings.is_empty(),
        "chart_boxplot clean program should produce NO security findings, got: {:?}",
        sec_findings
    );
}

// ── 11. Наряд №92: diagram_* security lint tests ──────────────────────
//
// The diagram_* builtins have been in SVG_AUTO_ESCAPE_BUILTINS with
// dedicated scan_*_labels scanners since наряды №81–84, but had ZERO
// test coverage in this file. These tests pin that coverage for each
// scanner group, testing the most representative builtin per scanner.
//
// Scanner coverage map:
//   scan_flowchart_labels  → diagram_flowchart, diagram_data_flow,
//                             diagram_high_level, diagram_architecture
//   scan_layers_labels     → diagram_layers, diagram_process,
//                             diagram_loop, diagram_pyramid, diagram_nested
//   scan_sequence_labels   → diagram_sequence
//   scan_timeline_labels   → diagram_timeline
//   scan_gantt_labels      → diagram_gantt
//   scan_venn_labels       → diagram_venn
//   scan_quadrant_labels   → diagram_quadrant
//   scan_medallion_labels  → diagram_medallion
//   scan_er_labels         → diagram_er
//   scan_state_labels      → diagram_state
//   scan_swimlane_labels   → diagram_swimlane
//   scan_tree_labels_recursive → diagram_tree, diagram_org_chart

// ── diagram_flowchart: <script> in node label → WARN ─────────────────

#[test]
fn diagram_flowchart_script_in_node_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_flowchart({nodes: [{id: \"a\", label: \"<script>alert(1)</script>\"}, {id: \"b\", label: \"safe\"}], edges: [{from: \"a\", to: \"b\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_flowchart")),
        "diagram_flowchart with <script> in label should NOT error (runtime escapes), errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_flowchart") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_flowchart with <script> in node label MUST warn (proves scan_flowchart_labels), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn diagram_flowchart_script_in_edge_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_flowchart({nodes: [{id: \"a\", label: \"A\"}, {id: \"b\", label: \"B\"}], edges: [{from: \"a\", to: \"b\", label: \"<script>alert(1)</script>\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_flowchart")),
        "diagram_flowchart with <script> in edge label should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_flowchart") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_flowchart with <script> in edge label MUST warn (proves scan_flowchart_labels checks edges), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn diagram_flowchart_clean_passes_lint() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_flowchart({nodes: [{id: \"a\", label: \"Start\"}, {id: \"b\", label: \"End\"}], edges: [{from: \"a\", to: \"b\", label: \"go\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_findings: Vec<&str> = r
        .errors
        .iter()
        .chain(r.warnings.iter())
        .filter(|m| m.contains("security:"))
        .map(|s| s.as_str())
        .collect();
    assert!(
        sec_findings.is_empty(),
        "clean diagram_flowchart should produce NO security findings, got: {:?}",
        sec_findings
    );
}

// ── diagram_data_flow: reuses scan_flowchart_labels ──────────────────

#[test]
fn diagram_data_flow_script_in_node_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_data_flow({nodes: [{id: \"a\", label: \"<script>alert(1)</script>\"}, {id: \"b\", label: \"safe\"}], edges: [{from: \"a\", to: \"b\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_data_flow")),
        "diagram_data_flow with <script> in label should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_data_flow") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_data_flow with <script> in node label MUST warn (proves scan_flowchart_labels reuse), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_architecture: reuses scan_flowchart_labels ───────────────

#[test]
fn diagram_architecture_script_in_node_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_architecture({nodes: [{id: \"a\", label: \"<script>alert(1)</script>\"}, {id: \"b\", label: \"safe\"}], edges: [{from: \"a\", to: \"b\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r)
            .iter()
            .any(|e| e.contains("diagram_architecture")),
        "diagram_architecture with <script> in label should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_architecture") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_architecture with <script> in node label MUST warn, warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_layers: <script> in label → WARN ─────────────────────────

#[test]
fn diagram_layers_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_layers([{label: \"<script>alert(1)</script>\", description: \"safe\"}, {label: \"ok\", description: \"ok\"}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_layers")),
        "diagram_layers with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_layers") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_layers with <script> in label MUST warn (proves scan_layers_labels), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_process: reuses scan_layers_labels ───────────────────────

#[test]
fn diagram_process_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_process([{label: \"<script>alert(1)</script>\"}, {label: \"safe\"}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_process")),
        "diagram_process with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_process") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_process with <script> in label MUST warn (proves scan_layers_labels reuse), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_loop: reuses scan_layers_labels ──────────────────────────

#[test]
fn diagram_loop_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_loop([{label: \"<script>alert(1)</script>\"}, {label: \"safe\"}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_loop")),
        "diagram_loop with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_loop") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_loop with <script> in label MUST warn, warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_pyramid: reuses scan_layers_labels ───────────────────────

#[test]
fn diagram_pyramid_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_pyramid([{label: \"<script>alert(1)</script>\", value: 10.0}, {label: \"safe\", value: 20.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_pyramid")),
        "diagram_pyramid with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_pyramid") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_pyramid with <script> in label MUST warn, warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_nested: reuses scan_layers_labels ────────────────────────

#[test]
fn diagram_nested_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_nested([{label: \"<script>alert(1)</script>\", value: 10.0}, {label: \"safe\", value: 20.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_nested")),
        "diagram_nested with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_nested") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_nested with <script> in label MUST warn, warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_sequence: <script> in actor → WARN ───────────────────────

#[test]
fn diagram_sequence_script_in_actor_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_sequence({actors: [\"<script>alert(1)</script>\", \"B\"], messages: [{from: \"A\", to: \"B\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_sequence")),
        "diagram_sequence with <script> in actor should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_sequence") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_sequence with <script> in actor MUST warn (proves scan_sequence_labels), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn diagram_sequence_clean_passes_lint() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_sequence({actors: [\"A\", \"B\"], messages: [{from: \"A\", to: \"B\", label: \"msg\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    let sec_findings: Vec<&str> = r
        .errors
        .iter()
        .chain(r.warnings.iter())
        .filter(|m| m.contains("security:"))
        .map(|s| s.as_str())
        .collect();
    assert!(
        sec_findings.is_empty(),
        "clean diagram_sequence should produce NO security findings, got: {:?}",
        sec_findings
    );
}

// ── diagram_timeline: <script> in label → WARN ───────────────────────

#[test]
fn diagram_timeline_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_timeline([{date: \"2024\", label: \"<script>alert(1)</script>\"}, {date: \"2025\", label: \"safe\"}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_timeline")),
        "diagram_timeline with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_timeline") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_timeline with <script> in label MUST warn (proves scan_timeline_labels), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_gantt: <script> in task → WARN ───────────────────────────

#[test]
fn diagram_gantt_script_in_task_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_gantt([{task: \"<script>alert(1)</script>\", start: 0.0, duration: 5.0}, {task: \"safe\", start: 5.0, duration: 3.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_gantt")),
        "diagram_gantt with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_gantt") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_gantt with <script> in task MUST warn (proves scan_gantt_labels), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_venn: <script> in circle label AND overlap_label → WARN ─

#[test]
fn diagram_venn_script_in_circle_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_venn({circles: [{label: \"<script>alert(1)</script>\"}, {label: \"B\"}], overlap_label: \"AB\"}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_venn")),
        "diagram_venn with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_venn") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_venn with <script> in circle label MUST warn (proves scan_venn_labels), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn diagram_venn_script_in_overlap_label_warns() {
    // CRITICAL: overlap_label is a TOP-LEVEL field, not inside circles[].
    // This is the "easy to forget" case that scan_venn_labels was
    // specifically written to catch.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_venn({circles: [{label: \"A\"}, {label: \"B\"}], overlap_label: \"<script>alert(1)</script>\"}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_venn")),
        "diagram_venn with <script> in overlap_label should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_venn") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_venn with <script> in overlap_label MUST warn (proves scan_venn_labels catches top-level field), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_quadrant: <script> in axis labels → WARN ─────────────────

#[test]
fn diagram_quadrant_script_in_axis_label_warns() {
    // CRITICAL: x_axis_label and y_axis_label are TOP-LEVEL fields,
    // not inside items[]. Same "easy to forget" pattern as overlap_label.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_quadrant({x_axis_label: \"<script>alert(1)</script>\", y_axis_label: \"Y\", items: [{label: \"item\", x: 0.5, y: 0.5}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_quadrant")),
        "diagram_quadrant with <script> in axis label should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_quadrant") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_quadrant with <script> in axis label MUST warn (proves scan_quadrant_labels catches top-level), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_medallion: <script> in label → WARN ──────────────────────

#[test]
fn diagram_medallion_script_in_label_warns() {
    // icon field is a controlled enum (NOT scanned); label IS scanned.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_medallion([{icon: \"check\", label: \"<script>alert(1)</script>\", value: 10.0}, {label: \"safe\", value: 20.0}], style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_medallion")),
        "diagram_medallion with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_medallion") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_medallion with <script> in label MUST warn (proves scan_medallion_labels), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_er: <script> in entity name AND fields[] → WARN ─────────

#[test]
fn diagram_er_script_in_entity_name_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_er({entities: [{name: \"<script>alert(1)</script>\", fields: [\"id\"]}, {name: \"B\", fields: [\"id\"]}], relations: []}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_er")),
        "diagram_er with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_er") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_er with <script> in entity name MUST warn (proves scan_er_labels), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn diagram_er_script_in_nested_field_warns() {
    // CRITICAL: entities[].fields is a List<String> NESTED INSIDE a struct —
    // the "third nesting form" (top-level List<String>, List<Struct>,
    // and now List<String> inside struct). scan_er_labels must walk both levels.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_er({entities: [{name: \"User\", fields: [\"id\", \"<script>alert(1)</script>\"]}, {name: \"Post\", fields: [\"id\"]}], relations: []}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_er")),
        "diagram_er with <script> in nested field should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_er") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_er with <script> in nested fields[] MUST warn (proves scan_er_labels walks nested List<String>), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_state: <script> in states[] AND initial → WARN ──────────

#[test]
fn diagram_state_script_in_state_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_state({states: [\"<script>alert(1)</script>\", \"B\"], transitions: [{from: \"A\", to: \"B\"}], initial: \"A\"}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_state")),
        "diagram_state with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_state") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_state with <script> in states[] MUST warn (proves scan_state_labels), warnings: {:?}",
        warnings(&r)
    );
}

#[test]
fn diagram_state_script_in_initial_warns() {
    // CRITICAL: `initial` is a TOP-LEVEL String? field (like overlap_label).
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_state({states: [\"A\", \"B\"], transitions: [{from: \"A\", to: \"B\"}], initial: \"<script>alert(1)</script>\"}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_state")),
        "diagram_state with <script> in initial should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_state") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_state with <script> in initial MUST warn (proves scan_state_labels catches top-level field), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_swimlane: <script> in lanes[] AND steps[].label → WARN ──

#[test]
fn diagram_swimlane_script_in_lane_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_swimlane({lanes: [\"<script>alert(1)</script>\", \"B\"], steps: [{lane: \"A\", label: \"step\", order: 1.0}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_swimlane")),
        "diagram_swimlane with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_swimlane") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_swimlane with <script> in lanes[] MUST warn (proves scan_swimlane_labels), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_tree: <script> in recursive label → WARN ────────────────

#[test]
fn diagram_tree_script_in_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_tree({label: \"<script>alert(1)</script>\", children: [{label: \"child\", children: []}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_tree")),
        "diagram_tree with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_tree") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_tree with <script> in label MUST warn (proves scan_tree_labels_recursive), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_org_chart: <script> in title field → WARN ────────────────

#[test]
fn diagram_org_chart_script_in_title_warns() {
    // diagram_org_chart has `title` field (allow_title=true) that
    // diagram_tree does not — this test pins the allow_title=true path.
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_org_chart({label: \"CEO\", title: \"<script>alert(1)</script>\", children: []}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_org_chart")),
        "diagram_org_chart with <script> in title should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_org_chart") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_org_chart with <script> in title MUST warn (proves scan_tree_labels_recursive with allow_title=true), warnings: {:?}",
        warnings(&r)
    );
}

// ── diagram_high_level: reuses scan_flowchart_labels ─────────────────

#[test]
fn diagram_high_level_script_in_node_label_warns() {
    let src = "pattern P(input: String) -> String {\n    let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n    return diagram_high_level({nodes: [{id: \"a\", label: \"<script>alert(1)</script>\"}, {id: \"b\", label: \"safe\"}], edges: [{from: \"a\", to: \"b\"}]}, style)\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let r = check_program(src).unwrap();
    assert!(
        !errors(&r).iter().any(|e| e.contains("diagram_high_level")),
        "diagram_high_level with <script> should NOT error, errors: {:?}",
        errors(&r)
    );
    let has_warning = warnings(&r)
        .iter()
        .any(|w| w.contains("diagram_high_level") && w.contains("script"));
    assert!(
        has_warning,
        "diagram_high_level with <script> in node label MUST warn, warnings: {:?}",
        warnings(&r)
    );
}
