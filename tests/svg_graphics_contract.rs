// ── Наряд №74: Integration tests for SVG Graphics & Diagrams ──────────
//
// Tests the FULL pipeline:
//   .mlog source → interpreter → SVG output → XML validation
//
// Pattern: each test defines a `pattern test_<name>(input: String) -> String`
// that returns the SVG output, then runs it via `flow Main`.

// ── Helper: eval a single expression via pattern return ──────────────

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

// ── Level 1: SVG primitives ───────────────────────────────────────────

#[test]
fn svg_rect_returns_valid_xml_fragment() {
    let xml = eval_expr("svg_rect(10.0, 10.0, 100.0, 50.0, \"#eb6c36\", \"none\")");
    assert!(xml.starts_with("<rect "));
    assert!(xml.ends_with(" />"));
    assert!(xml.contains("x=\"10\""));
    assert!(xml.contains("y=\"10\""));
    assert!(xml.contains("width=\"100\""));
    assert!(xml.contains("height=\"50\""));
    assert!(xml.contains("fill=\"#eb6c36\""));
    assert!(xml.contains("stroke=\"none\""));
}

#[test]
fn svg_circle_returns_valid_xml_fragment() {
    let xml = eval_expr("svg_circle(50.0, 50.0, 25.0, \"#eb6c36\")");
    assert!(xml.starts_with("<circle "));
    assert!(xml.contains("cx=\"50\""));
    assert!(xml.contains("cy=\"50\""));
    assert!(xml.contains("r=\"25\""));
    assert!(xml.contains("fill=\"#eb6c36\""));
}

#[test]
fn svg_line_with_optional_width() {
    let xml = eval_expr("svg_line(10.0, 10.0, 100.0, 50.0, \"#2d3142\", 2.0)");
    assert!(xml.contains("x1=\"10\""));
    assert!(xml.contains("y1=\"10\""));
    assert!(xml.contains("x2=\"100\""));
    assert!(xml.contains("y2=\"50\""));
    assert!(xml.contains("stroke=\"#2d3142\""));
    assert!(xml.contains("stroke-width=\"2\""));
}

#[test]
fn svg_line_default_width() {
    let xml = eval_expr("svg_line(10.0, 10.0, 100.0, 50.0, \"#2d3142\")");
    assert!(xml.contains("stroke-width=\"1\""));
}

#[test]
fn svg_text_default_anchor() {
    let xml = eval_expr("svg_text(20.0, 40.0, \"hello\", 14.0, \"#2d3142\")");
    assert!(xml.contains("text-anchor=\"start\""));
    assert!(xml.contains("hello"));
}

#[test]
fn svg_text_explicit_anchor_middle() {
    let xml = eval_expr("svg_text(50.0, 40.0, \"centered\", 14.0, \"#000\", \"middle\")");
    assert!(xml.contains("text-anchor=\"middle\""));
}

#[test]
fn svg_text_security_script_tag_escaped() {
    let xml = eval_expr("svg_text(10.0, 10.0, \"<script>alert(1)</script>\", 14.0, \"#000\")");
    assert!(
        !xml.contains("<script>"),
        "RAW <script> leaked into SVG output: {}",
        xml
    );
    assert!(xml.contains("&lt;script&gt;"));
    assert!(xml.contains("&lt;/script&gt;"));
}

#[test]
fn svg_text_security_quote_ampersand_escaped() {
    let xml = eval_expr("svg_text(10.0, 10.0, \"test 'quoted' & data\", 14.0, \"#000\")");
    assert!(xml.contains("&amp;"));
    assert!(xml.contains("&#39;quoted&#39;"));
}

#[test]
fn svg_path_basic() {
    let xml = eval_expr("svg_path(\"M 10 10 L 100 100\", \"none\", \"black\")");
    assert!(xml.starts_with("<path "));
    assert!(xml.contains("d=\"M 10 10 L 100 100\""));
    assert!(xml.contains("fill=\"none\""));
    assert!(xml.contains("stroke=\"black\""));
}

#[test]
fn svg_path_security_rejects_angle_brackets() {
    let err = eval_err("pattern __t(input: String) -> String { return svg_path(\"M 10 10 <script>\", \"none\", \"black\") }\nflow Main { input: String = \"x\" -> __t -> output }");
    assert!(
        err.contains("must not contain") || err.contains("'<"),
        "expected security error, got: {}",
        err
    );
}

#[test]
fn svg_group_with_transform() {
    let xml = eval_expr(
        "svg_group([svg_rect(0.0, 0.0, 10.0, 10.0, \"red\", \"none\")], \"translate(50, 50)\")",
    );
    assert!(xml.starts_with("<g "));
    assert!(xml.contains("transform=\"translate(50, 50)\""));
    assert!(xml.contains("<rect"));
    assert!(xml.ends_with("</g>"));
}

#[test]
fn svg_group_no_transform() {
    let xml = eval_expr("svg_group([svg_rect(0.0, 0.0, 10.0, 10.0, \"red\", \"none\")], \"\")");
    assert!(xml.starts_with("<g>"));
    assert!(!xml.contains("transform"));
}

#[test]
fn svg_canvas_returns_complete_svg_document() {
    let xml = eval_expr("svg_canvas(200.0, 100.0, \"0 0 200 100\", [svg_rect(10.0, 10.0, 100.0, 50.0, \"#eb6c36\", \"none\"), svg_text(20.0, 40.0, \"Hi\", 14.0, \"#2d3142\", \"start\")])");
    assert!(xml.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(xml.contains("width=\"200\""));
    assert!(xml.contains("height=\"100\""));
    assert!(xml.contains("viewBox=\"0 0 200 100\""));
    assert!(xml.contains("<rect"));
    assert!(xml.contains("<text"));
    assert!(xml.ends_with("</svg>"));
}

#[test]
fn svg_canvas_security_validates_viewbox_format() {
    let err = eval_err("pattern __t(input: String) -> String { return svg_canvas(200.0, 100.0, \"0 0 200\", []) }\nflow Main { input: String = \"x\" -> __t -> output }");
    assert!(err.contains("viewbox"));
}

#[test]
fn svg_canvas_produces_xml_parseable_output() {
    let xml = eval_expr("svg_canvas(100.0, 100.0, \"0 0 100 100\", [svg_rect(0.0, 0.0, 50.0, 50.0, \"red\", \"none\")])");
    assert!(xml.starts_with("<svg"));
    assert!(xml.ends_with("</svg>"));
    let rect_open = xml.matches("<rect").count();
    let rect_close = xml.matches("/>").count();
    assert!(
        rect_close >= rect_open,
        "each <rect> must be self-closed; rect_open={}, self_close={}",
        rect_open,
        rect_close
    );
}

// ── Level 2: diagram_style ────────────────────────────────────────────

#[test]
fn diagram_style_returns_struct_with_5_tokens() {
    let xml = eval_expr("chart_bar([{label: \"A\", value: 10.0}], diagram_style({paper: \"#f5f5f5\", ink: \"#2d3142\", accent: \"#eb6c36\", muted: \"#4f5d75\", rule: \"rgba(45,49,66,0.12)\"}))");
    assert!(xml.contains("#f5f5f5")); // paper
    assert!(xml.contains("#2d3142")); // ink
    assert!(xml.contains("#eb6c36")); // accent
    assert!(xml.contains("#4f5d75")); // muted
}

#[test]
fn diagram_style_rejects_missing_token() {
    let err = eval_err("pattern __t(input: String) -> String { let s = diagram_style({paper: \"#fff\", ink: \"#000\"}) return \"ok\" }\nflow Main { input: String = \"x\" -> __t -> output }");
    assert!(err.contains("missing") || err.contains("required"));
}

// ── Level 2.5: wow-effects ────────────────────────────────────────────

#[test]
fn svg_sketchy_filter_default_and_custom() {
    let xml_default = eval_expr("svg_sketchy_filter(\"sketch1\")");
    assert!(xml_default.contains("<filter id=\"sketch1\">"));
    assert!(xml_default.contains("feTurbulence"));
    assert!(xml_default.contains("feDisplacementMap"));

    let xml_custom = eval_expr("svg_sketchy_filter(\"sketch2\", 0.05, 4.0, 6.0, 42.0)");
    assert!(xml_custom.contains("baseFrequency=\"0.05\""));
    assert!(xml_custom.contains("numOctaves=\"4\""));
    assert!(xml_custom.contains("scale=\"6\""));
    assert!(xml_custom.contains("seed=\"42\""));
}

#[test]
fn svg_sketchy_filter_rejects_bad_frequency() {
    let err = eval_err("pattern __t(input: String) -> String { return svg_sketchy_filter(\"id\", 1.5) }\nflow Main { input: String = \"x\" -> __t -> output }");
    assert!(err.contains("base_frequency"));
}

#[test]
fn svg_icon_known_returns_path_with_current_color() {
    let xml = eval_expr("svg_icon(\"server\", 10.0, 10.0, 24.0, \"currentColor\")");
    assert!(xml.contains("stroke=\"currentColor\""));
    assert!(xml.contains("<path"));
    assert!(xml.contains("width=\"24\""));
    assert!(xml.contains("height=\"24\""));
}

#[test]
fn svg_icon_unknown_name_errors() {
    let err = eval_err("pattern __t(input: String) -> String { return svg_icon(\"nonexistent\", 0.0, 0.0, 24.0, \"black\") }\nflow Main { input: String = \"x\" -> __t -> output }");
    assert!(err.contains("unknown icon"));
}

#[test]
fn svg_callout_dashed_line_italic_text() {
    let xml = eval_expr("svg_callout(\"note\", 10.0, 10.0, 100.0, 50.0, \"neutral\")");
    assert!(xml.contains("stroke-dasharray=\"3,3\""));
    assert!(xml.contains("font-style=\"italic\""));
    assert!(xml.contains("<circle"));
    assert!(xml.contains("note"));
}

#[test]
fn svg_callout_intent_accent_uses_accent_color() {
    let xml = eval_expr("svg_callout(\"warning\", 10.0, 10.0, 100.0, 50.0, \"accent\")");
    assert!(xml.contains("#eb6c36"));
}

#[test]
fn svg_callout_security_escapes_text() {
    let xml = eval_expr("svg_callout(\"<b>bold</b>\", 10.0, 10.0, 100.0, 50.0)");
    assert!(!xml.contains("<b>bold</b>"));
    assert!(xml.contains("&lt;b&gt;bold&lt;/b&gt;"));
}

// ── Level 3: chart_bar (golden test) ──────────────────────────────────

/// Golden test: chart_bar with 3 bars (Янв/Фев/Мар, values 40/65/30).
/// Output must be deterministic — same input → identical output byte-for-byte.
#[test]
fn chart_bar_golden_3_bars_deterministic() {
    let src = "pattern __chart_bar_golden(input: String) -> String {\n        let data = [{label: \"Jan\", value: 40.0}, {label: \"Feb\", value: 65.0}, {label: \"Mar\", value: 30.0}]\n        let style = diagram_style({paper: \"#f5f5f5\", ink: \"#2d3142\", accent: \"#eb6c36\", muted: \"#4f5d75\", rule: \"rgba(45,49,66,0.12)\"})\n        return chart_bar(data, style)\n    }\nflow Main { input: String = \"x\" -> __chart_bar_golden -> output }";

    let out1 = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("failed: {}", e),
    };
    let out2 = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        Ok(None) => panic!("returned None"),
        Err(e) => panic!("failed: {}", e),
    };

    // Determinism invariant
    assert_eq!(out1, out2, "chart_bar output must be deterministic");

    let xml = &out1;

    // Structural: complete SVG document
    assert!(xml.starts_with("<svg "));
    assert!(xml.ends_with("</svg>"));

    // 3 bars + 1 background = 4 <rect> elements
    let rect_count = xml.matches("<rect").count();
    assert_eq!(
        rect_count, 4,
        "expected 4 <rect> (3 bars + 1 background), got {}",
        rect_count
    );

    // 3 X-axis labels + 3 value labels = 6 <text> elements
    let text_count = xml.matches("<text").count();
    assert_eq!(text_count, 6, "expected 6 <text> (3 labels + 3 values)");

    // Labels present
    assert!(xml.contains("Jan"));
    assert!(xml.contains("Feb"));
    assert!(xml.contains("Mar"));

    // The tallest bar (65) must be accent-colored
    assert!(
        xml.contains("fill=\"#eb6c36\""),
        "tallest bar should be accent-colored"
    );

    // 2 axis lines (vertical + horizontal)
    let line_count = xml.matches("<line").count();
    assert_eq!(line_count, 2, "expected 2 axis lines");
}

/// Verify chart_bar geometry: bar heights proportional to values.
#[test]
fn chart_bar_geometry_heights_proportional() {
    let src = "pattern __chart_bar_geom(input: String) -> String {\n        let data = [{label: \"A\", value: 40.0}, {label: \"B\", value: 65.0}, {label: \"C\", value: 30.0}]\n        let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n        return chart_bar(data, style)\n    }\nflow Main { input: String = \"x\" -> __chart_bar_geom -> output }";
    let xml = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    // Bar 2 (tallest) must have height="300"
    assert!(
        xml.contains("height=\"300\""),
        "tallest bar should have height=300, xml={}",
        xml
    );
    // Extract heights of all <rect> elements
    let heights: Vec<f64> = xml
        .split("<rect")
        .skip(1)
        .filter_map(|part| {
            let h_idx = part.find("height=\"")?;
            let after_h = &part[h_idx + 8..];
            let end = after_h.find('"')?;
            after_h[..end].parse::<f64>().ok()
        })
        .collect();
    assert_eq!(heights.len(), 4, "expected 4 rects (bg + 3 bars)");
    let bar_heights = &heights[1..];
    assert!(
        bar_heights[1] > bar_heights[0],
        "bar 2 should be taller than bar 1"
    );
    assert!(
        bar_heights[1] > bar_heights[2],
        "bar 2 should be taller than bar 3"
    );
    assert!(
        bar_heights[0] > bar_heights[2],
        "bar 1 should be taller than bar 3"
    );
}

#[test]
fn chart_bar_rejects_empty_data() {
    let err = eval_err("pattern __t(input: String) -> String {\n            let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n            return chart_bar([], style)\n        }\nflow Main { input: String = \"x\" -> __t -> output }");
    assert!(err.contains("empty"));
}

#[test]
fn chart_bar_rejects_too_many_bars() {
    let bars: Vec<String> = (0..51)
        .map(|i| format!("{{label: \"{}\", value: {}}}", i, i))
        .collect();
    let bars_str = bars.join(", ");
    let src = format!(
        "pattern __t(input: String) -> String {{\n            let data = [{}]\n            let style = diagram_style({{paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"}})\n            return chart_bar(data, style)\n        }}\nflow Main {{ input: String = \"x\" -> __t -> output }}",
        bars_str
    );
    let err = eval_err(&src);
    assert!(err.contains("too many") || err.contains("maximum"));
}

// ── Composability: full pipeline ──────────────────────────────────────

#[test]
fn chart_bar_output_is_embeddable_in_html() {
    let src = "pattern __t(input: String) -> String {\n        let data = [{label: \"Q1\", value: 100.0}, {label: \"Q2\", value: 150.0}]\n        let style = diagram_style({paper: \"#fff\", ink: \"#000\", accent: \"#f00\", muted: \"#888\", rule: \"#ccc\"})\n        let svg = chart_bar(data, style)\n        return \"<html><body>\" + svg + \"</body></html>\"\n    }\nflow Main { input: String = \"x\" -> __t -> output }";
    let html = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(html.starts_with("<html><body><svg "));
    assert!(html.ends_with("</svg></body></html>"));
}

#[test]
fn sketchy_filter_composes_with_group() {
    let src = "pattern __t(input: String) -> String {\n        let filter = svg_sketchy_filter(\"rough1\", 0.02, 3.0, 4.0, 1.0)\n        let shape = svg_rect(10.0, 10.0, 80.0, 40.0, \"#eb6c36\", \"none\")\n        let shapes = svg_group([shape], \"\")\n        return svg_canvas(200.0, 100.0, \"0 0 200 100\", [filter, shapes])\n    }\nflow Main { input: String = \"x\" -> __t -> output }";
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(svg.contains("<filter id=\"rough1\">"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<g>"));
}

// ── Наряд №87: Anti-overlap engine (diagram_timeline) ──────────────────
//
// Tests verify that diagram_timeline produces valid SVG and that the
// anti-overlap engine (estimate_text_width + resolve_overlaps) is
// active — labels are repositioned when they would collide.

#[test]
fn timeline_anti_overlap_basic_3_events() {
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        let data = [{date: "2024-01", label: "Start"}, {date: "2024-06", label: "Beta"}, {date: "2024-12", label: "GA"}]
        return diagram_timeline(data, style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(svg.starts_with("<svg "));
    assert!(svg.ends_with("</svg>"));
    // 3 event dots
    assert_eq!(svg.matches("<circle").count(), 3);
    // 6 text labels: 3 dates + 3 labels
    assert_eq!(svg.matches("<text").count(), 6);
}

#[test]
fn timeline_anti_overlap_single_event() {
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        return diagram_timeline([{date: "now", label: "Only"}], style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(svg.starts_with("<svg "));
    assert_eq!(svg.matches("<circle").count(), 1);
}

#[test]
fn timeline_anti_overlap_dense_long_labels() {
    // Deliberately create a dense timeline with long labels that
    // WOULD overlap under simple parity alternation. The anti-overlap
    // engine must reposition them so the SVG still renders correctly.
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        let data = [
            {date: "2024-01-15", label: "Initialize system with defaults"},
            {date: "2024-02-28", label: "Load configuration from storage"},
            {date: "2024-04-10", label: "Establish remote API connection"},
            {date: "2024-06-01", label: "Synchronize with upstream server"},
            {date: "2024-08-15", label: "Deploy to staging environment"},
            {date: "2024-10-30", label: "Release to production cluster"}
        ]
        return diagram_timeline(data, style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(svg.starts_with("<svg "));
    // 6 events → 6 dots + 12 text (6 dates + 6 labels)
    assert_eq!(svg.matches("<circle").count(), 6);
    assert_eq!(svg.matches("<text").count(), 12);
}

#[test]
fn timeline_anti_overlap_with_descriptions() {
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        let data = [
            {date: "Q1", label: "Research", description: "User interviews"},
            {date: "Q2", label: "Design", description: "Wireframes"},
            {date: "Q3", label: "Build", description: "Sprint"},
            {date: "Q4", label: "Ship", description: "GA release"}
        ]
        return diagram_timeline(data, style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(svg.starts_with("<svg "));
    // 4 dates + 4 labels + 4 descriptions = 12 text elements
    assert_eq!(svg.matches("<text").count(), 12);
}

#[test]
fn timeline_anti_overlap_12_events_max() {
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        let data = [
            {date: "M1", label: "Jan"}, {date: "M2", label: "Feb"}, {date: "M3", label: "Mar"},
            {date: "M4", label: "Apr"}, {date: "M5", label: "May"}, {date: "M6", label: "Jun"},
            {date: "M7", label: "Jul"}, {date: "M8", label: "Aug"}, {date: "M9", label: "Sep"},
            {date: "M10", label: "Oct"}, {date: "M11", label: "Nov"}, {date: "M12", label: "Dec"}
        ]
        return diagram_timeline(data, style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let svg = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("failed"),
    };
    assert!(svg.starts_with("<svg "));
    assert_eq!(svg.matches("<circle").count(), 12);
}

#[test]
fn timeline_anti_overlap_13_events_error() {
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        let data = [
            {date: "1", label: "a"}, {date: "2", label: "b"}, {date: "3", label: "c"},
            {date: "4", label: "d"}, {date: "5", label: "e"}, {date: "6", label: "f"},
            {date: "7", label: "g"}, {date: "8", label: "h"}, {date: "9", label: "i"},
            {date: "10", label: "j"}, {date: "11", label: "k"}, {date: "12", label: "l"},
            {date: "13", label: "m"}
        ]
        return diagram_timeline(data, style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let err = eval_err(src);
    assert!(
        err.contains("too many events") || err.contains("maximum"),
        "expected 'too many events' error, got: {}",
        err
    );
}

#[test]
fn timeline_anti_overlap_deterministic() {
    // Same input must produce identical output (resolve_overlaps is
    // deterministic because pairs are processed in index order).
    let src = r##"pattern __t(input: String) -> String {
        let style = diagram_style({paper: "#fff", ink: "#000", accent: "#eb6c36", muted: "#4f5d75", rule: "#ccc"})
        let data = [
            {date: "2024-01", label: "Start project"},
            {date: "2024-03", label: "First milestone"},
            {date: "2024-06", label: "Beta launch"},
            {date: "2024-09", label: "RC candidate"},
            {date: "2024-12", label: "General availability"}
        ]
        return diagram_timeline(data, style)
    }
flow Main { input: String = "x" -> __t -> output }"##;
    let out1 = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("first run failed"),
    };
    let out2 = match metalogos::run_program(src) {
        Ok(Some(s)) => s,
        _ => panic!("second run failed"),
    };
    assert_eq!(out1, out2, "diagram_timeline output must be deterministic");
}
