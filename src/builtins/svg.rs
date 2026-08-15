// ── Наряд №74: Native SVG Graphics & Diagrams ────────────────────────
//
// Three-layer architecture (ADR-0102):
//
//   Level 1 — SVG primitives (svg_rect, svg_circle, svg_line, svg_text,
//             svg_path, svg_group, svg_canvas) — return XML fragments.
//   Level 2 — diagram_style({...}) — returns a Style Struct with semantic
//             color tokens (paper/ink/accent/muted/rule).
//   Level 2.5 — Wow-effects: svg_sketchy_filter, svg_icon, svg_callout.
//   Level 3 — High-level chart types: chart_bar (this narad).
//
// Security invariant: ALL user-supplied text that ends up between `<...>`
// is XML-escaped via `escape_html_chars` (the same function used by
// escape_html builtin). This makes `<script>` injection impossible at
// the runtime level, regardless of what the .mlog program passes.
// Additional AST-level checking is performed by `mlog check`
// (see src/semantic.rs — svg_security_lint).

use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;

use std::collections::HashMap;

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract a struct argument as a HashMap<String, Value>.
/// Accepts Value::Struct with any type_name (we don't enforce a specific
/// type tag — duck-typing is more flexible and matches how diagram_style
/// is constructed via literal `{ key: value, ... }`).
fn expect_struct_arg(
    fn_name: &str,
    args: &[Value],
    idx: usize,
) -> Result<HashMap<String, Value>, String> {
    if idx >= args.len() {
        return Err(format!(
            "{}() requires an argument at position {}",
            fn_name,
            idx + 1
        ));
    }
    match &args[idx] {
        Value::Struct { fields, .. } => Ok(fields.clone()),
        other => Err(format!(
            "{}() expected Struct argument at position {}, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
    }
}

/// Extract a string field from a struct (HashMap). Returns Err if missing
/// or not a string.
fn struct_string_field(
    struct_name: &str,
    fields: &HashMap<String, Value>,
    key: &str,
) -> Result<String, String> {
    match fields.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!(
            "{}: field '{}' must be String, got {}",
            struct_name,
            key,
            other.type_name()
        )),
        None => Err(format!("{}: missing required field '{}'", struct_name, key)),
    }
}

/// Extract a float field from a struct. Returns Err if missing or not a float.
fn struct_float_field(
    struct_name: &str,
    fields: &HashMap<String, Value>,
    key: &str,
) -> Result<f64, String> {
    match fields.get(key) {
        Some(Value::Float(f)) => Ok(*f),
        Some(other) => Err(format!(
            "{}: field '{}' must be Float, got {}",
            struct_name,
            key,
            other.type_name()
        )),
        None => Err(format!("{}: missing required field '{}'", struct_name, key)),
    }
}

/// Extract an optional string field (returns None if missing or Unit).
#[allow(dead_code)] // reserved for future chart_* types (timeline, pyramid, etc.)
fn struct_opt_string_field(fields: &HashMap<String, Value>, key: &str) -> Option<String> {
    match fields.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

/// Extract an optional float field (returns None if missing or Unit).
#[allow(dead_code)] // reserved for future chart_* types (timeline, pyramid, etc.)
fn struct_opt_float_field(fields: &HashMap<String, Value>, key: &str) -> Option<f64> {
    match fields.get(key) {
        Some(Value::Float(f)) => Some(*f),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

/// Format a float for SVG output. Trims trailing zeros and unnecessary
/// decimal point for cleaner output. NaN/Inf become "0" (defensive).
fn fmt_num(n: f64) -> String {
    if !n.is_finite() {
        return "0".to_string();
    }
    // Round to 3 decimal places to avoid float artifacts like 10.0000000001
    let rounded = (n * 1000.0).round() / 1000.0;
    if rounded == rounded.trunc() {
        format!("{}", rounded as i64)
    } else {
        // Trim trailing zeros
        let s = format!("{:.3}", rounded);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ── Level 1: SVG Primitives ──────────────────────────────────────────
//
// Each primitive returns an XML fragment (String). Composing them via
// svg_group / svg_canvas produces a complete <svg> document.
//
// Security: text content (svg_text content arg, svg_callout text arg)
// is ALWAYS escaped via escape_html_chars. Attribute values that could
// contain user input (fill, stroke, anchor, transform, id) are also
// escaped — defense in depth, even though they typically come from
// trusted .mlog source.

/// `svg_rect(x, y, width, height, fill, stroke) -> String`
/// fill and stroke are color strings; use "none" for no fill/stroke.
pub fn builtin_svg_rect(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("svg_rect", args, 0)?;
    let y = expect_float_arg("svg_rect", args, 1)?;
    let w = expect_float_arg("svg_rect", args, 2)?;
    let h = expect_float_arg("svg_rect", args, 3)?;
    let fill = expect_string_arg("svg_rect", args, 4)?;
    let stroke = if args.len() > 5 {
        expect_string_arg("svg_rect", args, 5)?
    } else {
        "none".to_string()
    };
    if w <= 0.0 || h <= 0.0 {
        return Err(format!(
            "svg_rect: width and height must be positive (got w={}, h={})",
            w, h
        ));
    }
    Ok(Value::String(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" />"#,
        fmt_num(x),
        fmt_num(y),
        fmt_num(w),
        fmt_num(h),
        escape_attr(&fill),
        escape_attr(&stroke)
    )))
}

/// `svg_circle(cx, cy, r, fill) -> String`
pub fn builtin_svg_circle(args: &[Value]) -> Result<Value, String> {
    let cx = expect_float_arg("svg_circle", args, 0)?;
    let cy = expect_float_arg("svg_circle", args, 1)?;
    let r = expect_float_arg("svg_circle", args, 2)?;
    let fill = expect_string_arg("svg_circle", args, 3)?;
    if r <= 0.0 {
        return Err(format!("svg_circle: radius must be positive (got {})", r));
    }
    Ok(Value::String(format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="{}" />"#,
        fmt_num(cx),
        fmt_num(cy),
        fmt_num(r),
        escape_attr(&fill)
    )))
}

/// `svg_line(x1, y1, x2, y2, stroke, width) -> String`
pub fn builtin_svg_line(args: &[Value]) -> Result<Value, String> {
    let x1 = expect_float_arg("svg_line", args, 0)?;
    let y1 = expect_float_arg("svg_line", args, 1)?;
    let x2 = expect_float_arg("svg_line", args, 2)?;
    let y2 = expect_float_arg("svg_line", args, 3)?;
    let stroke = expect_string_arg("svg_line", args, 4)?;
    let width = if args.len() > 5 {
        expect_float_arg("svg_line", args, 5)?
    } else {
        1.0
    };
    if width <= 0.0 {
        return Err(format!(
            "svg_line: stroke width must be positive (got {})",
            width
        ));
    }
    Ok(Value::String(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"#,
        fmt_num(x1),
        fmt_num(y1),
        fmt_num(x2),
        fmt_num(y2),
        escape_attr(&stroke),
        fmt_num(width)
    )))
}

/// `svg_text(x, y, content, font_size, fill, anchor) -> String`
/// content is ALWAYS XML-escaped (security invariant).
/// anchor: "start" | "middle" | "end" (default "start").
pub fn builtin_svg_text(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("svg_text", args, 0)?;
    let y = expect_float_arg("svg_text", args, 1)?;
    let content = expect_string_arg("svg_text", args, 2)?;
    let font_size = expect_float_arg("svg_text", args, 3)?;
    let fill = expect_string_arg("svg_text", args, 4)?;
    let anchor = if args.len() > 5 {
        expect_string_arg("svg_text", args, 5)?
    } else {
        "start".to_string()
    };
    if !matches!(anchor.as_str(), "start" | "middle" | "end") {
        return Err(format!(
            r#"svg_text: anchor must be "start", "middle", or "end" (got "{}")"#,
            anchor
        ));
    }
    if font_size <= 0.0 {
        return Err(format!(
            "svg_text: font_size must be positive (got {})",
            font_size
        ));
    }
    Ok(Value::String(format!(
        r#"<text x="{}" y="{}" font-size="{}" fill="{}" text-anchor="{}">{}</text>"#,
        fmt_num(x),
        fmt_num(y),
        fmt_num(font_size),
        escape_attr(&fill),
        escape_attr(&anchor),
        escape_html_chars(&content)
    )))
}

/// `svg_path(d, fill, stroke) -> String`
/// d is the path data string (NOT escaped — it's a domain-specific mini-language,
/// escaping it would break the path syntax). fill/stroke ARE escaped.
pub fn builtin_svg_path(args: &[Value]) -> Result<Value, String> {
    let d = expect_string_arg("svg_path", args, 0)?;
    let fill = expect_string_arg("svg_path", args, 1)?;
    let stroke = if args.len() > 2 {
        expect_string_arg("svg_path", args, 2)?
    } else {
        "none".to_string()
    };
    // Sanity: reject path data that contains < or > (would break XML structure).
    if d.contains('<') || d.contains('>') {
        return Err("svg_path: path data must not contain '<' or '>' characters".to_string());
    }
    Ok(Value::String(format!(
        r#"<path d="{}" fill="{}" stroke="{}" />"#,
        d,
        escape_attr(&fill),
        escape_attr(&stroke)
    )))
}

/// `svg_group(children: List<String>, transform) -> String`
/// transform is optional (pass "" or omit for no transform).
/// Children are concatenated as-is (they are already-rendered XML fragments).
pub fn builtin_svg_group(args: &[Value]) -> Result<Value, String> {
    let children = expect_list_arg("svg_group", args, 0)?;
    let transform = if args.len() > 1 {
        let t = expect_string_arg("svg_group", args, 1)?;
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    } else {
        None
    };
    let mut body = String::new();
    for (i, child) in children.iter().enumerate() {
        match child {
            Value::String(s) => {
                body.push_str(s);
                if i + 1 < children.len() {
                    body.push('\n');
                }
            }
            other => {
                return Err(format!(
                    "svg_group: child {} must be String (XML fragment), got {}",
                    i + 1,
                    other.type_name()
                ));
            }
        }
    }
    let transform_attr = match &transform {
        Some(t) => format!(r#" transform="{}""#, escape_attr(t)),
        None => String::new(),
    };
    Ok(Value::String(format!(
        "<g{}>\n{}\n</g>",
        transform_attr, body
    )))
}

/// `svg_canvas(width, height, viewbox, children: List<String>) -> String`
/// viewbox is a string like "0 0 200 100" (min_x min_y width height).
/// Returns a complete `<svg>...</svg>` document.
pub fn builtin_svg_canvas(args: &[Value]) -> Result<Value, String> {
    let width = expect_float_arg("svg_canvas", args, 0)?;
    let height = expect_float_arg("svg_canvas", args, 1)?;
    let viewbox = expect_string_arg("svg_canvas", args, 2)?;
    let children = expect_list_arg("svg_canvas", args, 3)?;
    if width <= 0.0 || height <= 0.0 {
        return Err(format!(
            "svg_canvas: width and height must be positive (got w={}, h={})",
            width, height
        ));
    }
    // viewbox must be 4 numbers separated by spaces
    let vb_parts: Vec<&str> = viewbox.split_whitespace().collect();
    if vb_parts.len() != 4 {
        return Err(format!(
            r#"svg_canvas: viewbox must be "min_x min_y width height" (4 numbers, got "{}")"#,
            viewbox
        ));
    }
    for p in &vb_parts {
        if p.parse::<f64>().is_err() {
            return Err(format!(
                r#"svg_canvas: viewbox component "{}" is not a number"#,
                p
            ));
        }
    }
    let mut body = String::new();
    for (i, child) in children.iter().enumerate() {
        match child {
            Value::String(s) => {
                body.push_str(s);
                if i + 1 < children.len() {
                    body.push('\n');
                }
            }
            other => {
                return Err(format!(
                    "svg_canvas: child {} must be String (XML fragment), got {}",
                    i + 1,
                    other.type_name()
                ));
            }
        }
    }
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{}">{}</svg>"#,
        fmt_num(width),
        fmt_num(height),
        escape_attr(&viewbox),
        body
    )))
}

// ── Level 2: Design Tokens ───────────────────────────────────────────

/// `diagram_style({ paper, ink, accent, muted, rule }) -> Struct`
/// Returns a Struct with type_name "DiagramStyle" and the 5 canonical
/// semantic color tokens. Used as the `style` argument to chart_* functions.
pub fn builtin_diagram_style(args: &[Value]) -> Result<Value, String> {
    let fields = expect_struct_arg("diagram_style", args, 0)?;
    // Validate presence of all 5 canonical tokens
    let paper = struct_string_field("diagram_style", &fields, "paper")?;
    let ink = struct_string_field("diagram_style", &fields, "ink")?;
    let accent = struct_string_field("diagram_style", &fields, "accent")?;
    let muted = struct_string_field("diagram_style", &fields, "muted")?;
    let rule = struct_string_field("diagram_style", &fields, "rule")?;
    // Build the canonical struct with exactly the 5 tokens
    let mut style_fields = HashMap::new();
    style_fields.insert("paper".to_string(), Value::String(paper));
    style_fields.insert("ink".to_string(), Value::String(ink));
    style_fields.insert("accent".to_string(), Value::String(accent));
    style_fields.insert("muted".to_string(), Value::String(muted));
    style_fields.insert("rule".to_string(), Value::String(rule));
    Ok(Value::Struct {
        type_name: "DiagramStyle".to_string(),
        fields: style_fields,
    })
}

/// Extract a DiagramStyle from a Value (helper for chart_* functions).
pub(crate) fn extract_style(value: &Value) -> Result<HashMap<String, Value>, String> {
    match value {
        Value::Struct { type_name, fields } => {
            if type_name != "DiagramStyle" {
                return Err(format!(
                    "expected DiagramStyle struct, got struct with type_name '{}'",
                    type_name
                ));
            }
            // Verify all 5 canonical tokens are present
            for k in &["paper", "ink", "accent", "muted", "rule"] {
                if !fields.contains_key(*k) {
                    return Err(format!("DiagramStyle missing required token '{}'", k));
                }
            }
            Ok(fields.clone())
        }
        other => Err(format!(
            "expected DiagramStyle struct, got {}",
            other.type_name()
        )),
    }
}

/// Get a color token from a style HashMap. Returns the string value.
pub(crate) fn style_token(style: &HashMap<String, Value>, key: &str) -> Result<String, String> {
    match style.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!(
            "DiagramStyle: token '{}' missing or not String",
            key
        )),
    }
}

// ── Level 3 (item 1): chart_bar ──────────────────────────────────────
//
// Bar chart with pure parametric geometry (no graph-layout algorithm).
// Layout:
//   - canvas: width=600, height=400, with 40px padding for axes/labels
//   - chart area: x=[80, 580], y=[40, 340]  (500w × 300h)
//   - each bar: width = chart_w / N - gap (gap = 20)
//   - bar height = (value / max_value) * chart_h
//   - x position = chart_x + i * (bar_w + gap) + gap/2
//   - y position = chart_y_bottom - bar_h
// Deterministic: same inputs → identical output (golden-test invariant).

pub fn builtin_chart_bar(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_bar", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_bar: data list must not be empty".to_string());
    }
    if data.len() > 50 {
        return Err(format!(
            "chart_bar: too many bars ({}), maximum is 50",
            data.len()
        ));
    }

    // Extract {label, value} from each item
    let mut items: Vec<(String, f64)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_bar: data[{}] must be Struct {{label, value}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("chart_bar item", fields, "label")?;
        let value = struct_float_field("chart_bar item", fields, "value")?;
        items.push((label, value));
    }

    // Geometry constants
    let canvas_w = 600.0_f64;
    let canvas_h = 400.0_f64;
    let chart_x = 80.0_f64;
    let chart_y_top = 40.0_f64;
    let chart_w = 500.0_f64;
    let chart_h = 300.0_f64;
    let chart_y_bottom = chart_y_top + chart_h; // 340
    let gap = 20.0_f64;
    let bar_w = (chart_w / data.len() as f64) - gap;
    if bar_w < 10.0 {
        return Err(format!(
            "chart_bar: too many bars ({}) for canvas width — bar width would be {}",
            data.len(),
            bar_w
        ));
    }

    let max_value = items
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    if max_value <= 0.0 {
        return Err(format!(
            "chart_bar: max value must be positive (got {})",
            max_value
        ));
    }

    let ink = style_token(&style, "ink")?;
    let accent = style_token(&style, "accent")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;
    let paper = style_token(&style, "paper")?;

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Axis (rule color)
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(chart_x),
        fmt_num(chart_y_top),
        fmt_num(chart_x),
        fmt_num(chart_y_bottom),
        escape_attr(&rule)
    ));
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(chart_x),
        fmt_num(chart_y_bottom),
        fmt_num(chart_x + chart_w),
        fmt_num(chart_y_bottom),
        escape_attr(&rule)
    ));

    // Bars + labels
    for (i, (label, value)) in items.iter().enumerate() {
        let bar_h = (value / max_value) * chart_h;
        let x = chart_x + (i as f64) * (bar_w + gap) + gap / 2.0;
        let y = chart_y_bottom - bar_h;

        // Bar (accent color for the tallest, ink for others — visual hierarchy)
        let bar_color = if (value - max_value).abs() < f64::EPSILON {
            &accent
        } else {
            &ink
        };
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(bar_w),
            fmt_num(bar_h),
            escape_attr(bar_color)
        ));

        // Value label above bar (muted color)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x + bar_w / 2.0),
            fmt_num(y - 5.0),
            escape_attr(&muted),
            escape_html_chars(&format!("{}", value))
        ));

        // X-axis label (ink color)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x + bar_w / 2.0),
            fmt_num(chart_y_bottom + 18.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }

    // Wrap in <svg> canvas
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Level 3 (item 2): chart_donut ────────────────────────────────────
//
// Narad №77 Block 2: donut chart with pure parametric geometry.
//
// Layout (deterministic, no graph-layout algorithm):
//   - canvas: width=600, height=400
//   - center: (200, 200) — leaves right half for legend
//   - outer radius: 140
//   - inner radius: 70  (50% — typical donut hole)
//   - legend area: x=[380, 580], y=[80, 320]
//
// Each slice is a <path> built with two arc commands:
//   M outer_start  A r_out r_out 0 large 1 outer_end
//   L inner_end    A r_in  r_in  0 large 0 inner_start  Z
//
// Where angles start at -π/2 (top of circle) and proceed clockwise.
// `large` is 1 if the slice angle > π, else 0.
//
// Security: ALL labels (both slice labels and legend text) are XML-escaped
// via escape_html_chars — same invariant as chart_bar. chart_donut is
// registered in SVG_AUTO_ESCAPE_BUILTINS in semantic.rs so the AST lint
// also scans for `<script>` payloads in label string literals.
//
// Determinism: same inputs → identical output (golden-test invariant).

pub fn builtin_chart_donut(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_donut", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_donut: data list must not be empty".to_string());
    }
    if data.len() > 50 {
        return Err(format!(
            "chart_donut: too many slices ({}), maximum is 50",
            data.len()
        ));
    }

    // Extract {label, value} from each item
    let mut items: Vec<(String, f64)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_donut: data[{}] must be Struct {{label, value}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("chart_donut item", fields, "label")?;
        let value = struct_float_field("chart_donut item", fields, "value")?;
        if value < 0.0 {
            return Err(format!(
                "chart_donut: data[{}] value must be non-negative (got {})",
                i, value
            ));
        }
        items.push((label, value));
    }

    let total: f64 = items.iter().map(|(_, v)| *v).sum();
    if total <= 0.0 {
        return Err(format!(
            "chart_donut: sum of values must be positive (got {})",
            total
        ));
    }

    // Geometry constants
    let canvas_w = 600.0_f64;
    let canvas_h = 400.0_f64;
    let cx = 200.0_f64;
    let cy = 200.0_f64;
    let r_out = 140.0_f64;
    let r_in = 70.0_f64;
    // Legend column
    let legend_x = 380.0_f64;
    let legend_y_start = 90.0_f64;
    let legend_row_h = 22.0_f64;
    let legend_swatch = 14.0_f64;

    let paper = style_token(&style, "paper")?;
    let ink = style_token(&style, "ink")?;
    let accent = style_token(&style, "accent")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;

    // Color palette for slices: derive N shades from accent + ink.
    // We alternate accent (vivid) and ink-derived shades (structural),
    // so the chart stays within the same color family (palette.md V2.1).
    // For a single-slice donut, use accent (whole pie = accent).
    // For multi-slice, alternate: accent, ink, accent-lightened, ink-lightened.
    let slice_colors = build_slice_colors(&accent, &ink, items.len());

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Compute slice angles (radians). Start at -π/2 (top), go clockwise.
    let start_angle = -std::f64::consts::PI / 2.0;
    let mut current_angle = start_angle;

    // Threshold: if a slice angle is within epsilon of 2π, it's a full circle.
    // SVG arc command with start==end is ambiguous (renders nothing), so we
    // split full-circle slices into two semicircle arcs.
    const FULL_CIRCLE_EPS: f64 = 1e-9;

    for (i, (_label, value)) in items.iter().enumerate() {
        let slice_angle = (value / total) * 2.0 * std::f64::consts::PI;
        let end_angle = current_angle + slice_angle;
        let color = &slice_colors[i];

        if slice_angle >= 2.0 * std::f64::consts::PI - FULL_CIRCLE_EPS {
            // Full-circle slice (only happens when N=1): split into two
            // semicircle arcs to avoid SVG's ambiguous start==end case.
            let mid_angle = current_angle + std::f64::consts::PI;
            // First semicircle: current_angle → mid_angle
            let (ox1, oy1) = polar_to_xy(cx, cy, r_out, current_angle);
            let (ox2, oy2) = polar_to_xy(cx, cy, r_out, mid_angle);
            let (ix1, iy1) = polar_to_xy(cx, cy, r_in, mid_angle);
            let (ix2, iy2) = polar_to_xy(cx, cy, r_in, current_angle);
            parts.push(format!(
                r#"<path d="M {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 0 {} {} Z" fill="{}" stroke="{}" stroke-width="1" />"#,
                fmt_num(ox1),
                fmt_num(oy1),
                fmt_num(r_out),
                fmt_num(r_out),
                fmt_num(ox2),
                fmt_num(oy2),
                fmt_num(ix1),
                fmt_num(iy1),
                fmt_num(r_in),
                fmt_num(r_in),
                fmt_num(ix2),
                fmt_num(iy2),
                escape_attr(color),
                escape_attr(&paper)
            ));
            // Second semicircle: mid_angle → end_angle (== current_angle modulo 2π)
            let (ox3, oy3) = polar_to_xy(cx, cy, r_out, mid_angle);
            let (ox4, oy4) = polar_to_xy(cx, cy, r_out, end_angle);
            let (ix3, iy3) = polar_to_xy(cx, cy, r_in, end_angle);
            let (ix4, iy4) = polar_to_xy(cx, cy, r_in, mid_angle);
            parts.push(format!(
                r#"<path d="M {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 0 {} {} Z" fill="{}" stroke="{}" stroke-width="1" />"#,
                fmt_num(ox3),
                fmt_num(oy3),
                fmt_num(r_out),
                fmt_num(r_out),
                fmt_num(ox4),
                fmt_num(oy4),
                fmt_num(ix3),
                fmt_num(iy3),
                fmt_num(r_in),
                fmt_num(r_in),
                fmt_num(ix4),
                fmt_num(iy4),
                escape_attr(color),
                escape_attr(&paper)
            ));
        } else {
            // Outer arc endpoints
            let (ox1, oy1) = polar_to_xy(cx, cy, r_out, current_angle);
            let (ox2, oy2) = polar_to_xy(cx, cy, r_out, end_angle);
            // Inner arc endpoints (note: reversed direction for inner arc)
            let (ix1, iy1) = polar_to_xy(cx, cy, r_in, end_angle);
            let (ix2, iy2) = polar_to_xy(cx, cy, r_in, current_angle);

            let large_arc = if slice_angle > std::f64::consts::PI {
                1
            } else {
                0
            };

            // Donut slice path:
            //   M outer_start
            //   A r_out r_out 0 large 1 outer_end   (clockwise outer arc)
            //   L inner_end
            //   A r_in  r_in  0 large 0 inner_start (counter-clockwise inner arc)
            //   Z
            parts.push(format!(
                r#"<path d="M {} {} A {} {} 0 {} 1 {} {} L {} {} A {} {} 0 {} 0 {} {} Z" fill="{}" stroke="{}" stroke-width="1" />"#,
                fmt_num(ox1),
                fmt_num(oy1),
                fmt_num(r_out),
                fmt_num(r_out),
                large_arc,
                fmt_num(ox2),
                fmt_num(oy2),
                fmt_num(ix1),
                fmt_num(iy1),
                fmt_num(r_in),
                fmt_num(r_in),
                large_arc,
                fmt_num(ix2),
                fmt_num(iy2),
                escape_attr(color),
                escape_attr(&paper) // stroke = paper creates visual separation between slices
            ));
        }

        // Slice percentage label inside the slice (at midpoint, between r_in and r_out)
        // Only render if slice is big enough to hold a label (>=5% of total)
        if value / total >= 0.05 {
            let mid_angle = current_angle + slice_angle / 2.0;
            let label_r = (r_out + r_in) / 2.0;
            let (lx, ly) = polar_to_xy(cx, cy, label_r, mid_angle);
            let pct = (value / total) * 100.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle" dominant-baseline="middle">{}</text>"#,
                fmt_num(lx),
                fmt_num(ly),
                escape_attr(&paper), // percentage text contrasts with slice color
                escape_html_chars(&format!("{}%", (pct.round() as i64)))
            ));
        }

        current_angle = end_angle;
    }

    // Legend (right column): swatch + label for each slice
    for (i, (label, _value)) in items.iter().enumerate() {
        let ly = legend_y_start + (i as f64) * legend_row_h;
        let color = &slice_colors[i];
        // Swatch
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
            fmt_num(legend_x),
            fmt_num(ly),
            fmt_num(legend_swatch),
            fmt_num(legend_swatch),
            escape_attr(color)
        ));
        // Label
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" dominant-baseline="middle">{}</text>"#,
            fmt_num(legend_x + legend_swatch + 8.0),
            fmt_num(ly + legend_swatch / 2.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }

    // Title rule line (subtle separator above legend)
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(legend_x),
        fmt_num(legend_y_start - 12.0),
        fmt_num(canvas_w - 20.0),
        fmt_num(legend_y_start - 12.0),
        escape_attr(&rule)
    ));

    // Center text: total value (in the donut hole)
    let total_label = format!("{}", (total.round() as i64));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="20" font-weight="bold" fill="{}" text-anchor="middle" dominant-baseline="middle">{}</text>"#,
        fmt_num(cx),
        fmt_num(cy - 6.0),
        escape_attr(&ink),
        escape_html_chars(&total_label)
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle" dominant-baseline="middle">total</text>"#,
        fmt_num(cx),
        fmt_num(cy + 12.0),
        escape_attr(&muted)
    ));

    // Wrap in <svg> canvas
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

/// Convert polar coordinates (center + angle in radians) to SVG cartesian.
/// SVG Y-axis points down, so we use sin(angle) directly (no negation).
/// angle = -π/2 corresponds to the top of the circle.
fn polar_to_xy(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
    (cx + r * angle.cos(), cy + r * angle.sin())
}

/// Build a list of N slice colors that stay within the same color family
/// (accent + ink, alternating). For N=1, return [accent]. For N>1, alternate
/// accent and ink so adjacent slices have different colors but the whole
/// chart stays within the same hue family (palette.md V2.1).
fn build_slice_colors(accent: &str, ink: &str, n: usize) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![accent.to_string()];
    }
    // For 2+ slices, alternate accent and ink.
    // This gives clear visual separation between adjacent slices while
    // staying within the same color family (both come from base_hue).
    let mut colors = Vec::with_capacity(n);
    for i in 0..n {
        if i % 2 == 0 {
            colors.push(accent.to_string());
        } else {
            colors.push(ink.to_string());
        }
    }
    colors
}

// ── Level 3 (items 3-5): chart_line, chart_scatter, chart_area ──────
//
// Narad №78 Block 1-3: three additional high-level chart types.
//
// Design constraints (per spec):
//   - chart_line / chart_area reuse the SAME canvas geometry constants
//     as chart_bar (canvas=600×400, chart area x=[80,580] y=[40,340]).
//     Visual consistency between chart types is more important than
//     per-type optimization.
//   - chart_scatter uses the same canvas but scales BOTH axes from data
//     (independent min/max), since scatter requires two numeric dims.
//   - All three escape user-supplied label text via escape_html_chars
//     (same invariant as chart_bar / chart_donut). All three are
//     registered in SVG_AUTO_ESCAPE_BUILTINS in semantic.rs, and the
//     scan_chart_labels branch is extended to cover them.
//
// Upper bound for points/bars: 50 for chart_bar/donut (bar width ≥ 10px).
// chart_line / chart_area raise this to 100 (points have no width, so
// overlap is not the limiting factor — SVG path length and label clutter
// are). chart_scatter raises it to 200 (scatter points are small circles
// and tolerate overlap; the chart is intrinsically denser).

/// Canvas geometry constants shared by chart_bar / chart_line / chart_area.
/// Defined once here so any future chart type that wants visual parity
/// with the bar chart can reference the same numbers.
const CHART_CANVAS_W: f64 = 600.0;
const CHART_CANVAS_H: f64 = 400.0;
const CHART_X: f64 = 80.0;
const CHART_Y_TOP: f64 = 40.0;
const CHART_W: f64 = 500.0;
const CHART_H: f64 = 300.0;
const CHART_Y_BOTTOM: f64 = CHART_Y_TOP + CHART_H; // 340

/// chart_line: line chart with one `<path>` through all points.
///
/// Data shape: `List<Struct{label, value}>` — same as chart_bar.
/// X position: evenly spaced across chart_w (point i at
///   chart_x + i * chart_w / (N-1) for N>1; for N=1, single point at
///   chart_x + chart_w/2).
/// Y position: chart_y_bottom - (value / max_value) * chart_h.
/// Geometry constants reused from chart_bar (visual parity).
pub fn builtin_chart_line(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_line", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_line: data list must not be empty".to_string());
    }
    // Upper bound 100: line points have no width, so overlap is not the
    // constraint — SVG path length and label clutter are. 100 is generous
    // for typical line charts (daily metric for a quarter = ~90 points).
    if data.len() > 100 {
        return Err(format!(
            "chart_line: too many points ({}), maximum is 100",
            data.len()
        ));
    }

    // Extract {label, value} from each item
    let mut items: Vec<(String, f64)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_line: data[{}] must be Struct {{label, value}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("chart_line item", fields, "label")?;
        let value = struct_float_field("chart_line item", fields, "value")?;
        items.push((label, value));
    }

    let max_value = items
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    if max_value <= 0.0 {
        return Err(format!(
            "chart_line: max value must be positive (got {})",
            max_value
        ));
    }

    let ink = style_token(&style, "ink")?;
    let accent = style_token(&style, "accent")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;
    let paper = style_token(&style, "paper")?;

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        escape_attr(&paper)
    ));

    // Axes (same as chart_bar)
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_TOP),
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM),
        escape_attr(&rule)
    ));
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM),
        fmt_num(CHART_X + CHART_W),
        fmt_num(CHART_Y_BOTTOM),
        escape_attr(&rule)
    ));

    // Compute point positions
    let n = items.len() as f64;
    let points: Vec<(f64, f64, String, f64)> = items
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let x = if items.len() == 1 {
                CHART_X + CHART_W / 2.0
            } else {
                CHART_X + (i as f64) * CHART_W / (n - 1.0)
            };
            let y = CHART_Y_BOTTOM - (value / max_value) * CHART_H;
            (x, y, label.clone(), *value)
        })
        .collect();

    // Build the line path: M x0 y0 L x1 y1 L x2 y2 ...
    let mut path_d = format!("M {} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    for (x, y, _, _) in points.iter().skip(1) {
        path_d.push_str(&format!(" L {} {}", fmt_num(*x), fmt_num(*y)));
    }
    parts.push(format!(
        r#"<path d="{}" fill="none" stroke="{}" stroke-width="2" />"#,
        path_d,
        escape_attr(&accent)
    ));

    // Optional markers: small circle at each data point (radius 3)
    // Helps visibility when lines cross or values cluster.
    for (x, y, _, _) in &points {
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="3" fill="{}" />"#,
            fmt_num(*x),
            fmt_num(*y),
            escape_attr(&accent)
        ));
    }

    // X-axis labels (ink color) — only render if N ≤ 20 to avoid clutter.
    // Above 20 points, labels overlap and become unreadable; the line
    // shape itself carries the information.
    if items.len() <= 20 {
        for (x, _, label, _) in &points {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(*x),
                fmt_num(CHART_Y_BOTTOM + 18.0),
                escape_attr(&ink),
                escape_html_chars(label)
            ));
        }
    }

    // Value label for the peak point (muted color) — single annotation,
    // not per-point, to avoid clutter.
    let peak_idx = items
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    if let Some((px, py, _, pv)) = points.get(peak_idx) {
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*px),
            fmt_num(*py - 8.0),
            escape_attr(&muted),
            escape_html_chars(&format!("{}", pv))
        ));
    }

    // Wrap in <svg> canvas
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        body
    )))
}

/// chart_scatter: scatter plot with two independent numeric axes.
///
/// Data shape: `List<Struct{x, y, label?}>` — DIFFERENT from chart_bar.
/// Both x and y are Float. `label` is optional String (absent → no text).
/// Both axes scaled independently: x → [chart_x, chart_x+chart_w],
/// y → [chart_y_bottom, chart_y_top] (inverted, SVG Y points down).
///
/// Edge case: if all x values are equal (or all y), we still render —
/// points are placed at the chart center along that axis. This avoids
/// a divide-by-zero and gives a sensible visual (vertical/horizontal line).
pub fn builtin_chart_scatter(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_scatter", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_scatter: data list must not be empty".to_string());
    }
    // Upper bound 200: scatter points are small circles (r=4), overlap is
    // expected and even informative (density clustering). 200 keeps the
    // SVG file size reasonable and matches typical scatter use cases.
    if data.len() > 200 {
        return Err(format!(
            "chart_scatter: too many points ({}), maximum is 200",
            data.len()
        ));
    }

    // Extract {x, y, label?} from each item — note the different shape
    let mut items: Vec<(f64, f64, Option<String>)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_scatter: data[{}] must be Struct {{x, y, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let x = struct_float_field("chart_scatter item", fields, "x")?;
        let y = struct_float_field("chart_scatter item", fields, "y")?;
        // label is OPTIONAL — use struct_opt_string_field (already exists
        // in this module, was reserved for future chart_* types since №74).
        let label = struct_opt_string_field(fields, "label");
        items.push((x, y, label));
    }

    let x_min = items
        .iter()
        .map(|(x, _, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let x_max = items
        .iter()
        .map(|(x, _, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_min = items
        .iter()
        .map(|(_, y, _)| *y)
        .fold(f64::INFINITY, f64::min);
    let y_max = items
        .iter()
        .map(|(_, y, _)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
        return Err("chart_scatter: data contains non-finite x or y value".to_string());
    }

    let ink = style_token(&style, "ink")?;
    let accent = style_token(&style, "accent")?;
    let rule = style_token(&style, "rule")?;
    let paper = style_token(&style, "paper")?;

    // Scale functions — handle the degenerate single-value axis by mapping
    // to the chart center along that axis (avoids div-by-zero, gives a
    // visually sensible vertical or horizontal line of points).
    let scale_x = |x: f64| -> f64 {
        let span = x_max - x_min;
        if span.abs() < f64::EPSILON {
            CHART_X + CHART_W / 2.0
        } else {
            CHART_X + (x - x_min) / span * CHART_W
        }
    };
    let scale_y = |y: f64| -> f64 {
        let span = y_max - y_min;
        if span.abs() < f64::EPSILON {
            CHART_Y_TOP + CHART_H / 2.0
        } else {
            // Y inverted: higher data value → smaller SVG Y (toward top)
            CHART_Y_BOTTOM - (y - y_min) / span * CHART_H
        }
    };

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        escape_attr(&paper)
    ));

    // Axes
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_TOP),
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM),
        escape_attr(&rule)
    ));
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM),
        fmt_num(CHART_X + CHART_W),
        fmt_num(CHART_Y_BOTTOM),
        escape_attr(&rule)
    ));

    // Axis range labels (corners) — show min/max so the chart is readable
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM + 32.0),
        escape_attr(&ink),
        escape_html_chars(&format!("{}", x_min))
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
        fmt_num(CHART_X + CHART_W),
        fmt_num(CHART_Y_BOTTOM + 32.0),
        escape_attr(&ink),
        escape_html_chars(&format!("{}", x_max))
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(CHART_X - 6.0),
        fmt_num(CHART_Y_BOTTOM + 4.0),
        escape_attr(&ink),
        escape_html_chars(&format!("{}", y_min))
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(CHART_X - 6.0),
        fmt_num(CHART_Y_TOP + 4.0),
        escape_attr(&ink),
        escape_html_chars(&format!("{}", y_max))
    ));

    // Plot each point: circle (r=4, accent fill) + optional label
    for (x, y, label) in &items {
        let px = scale_x(*x);
        let py = scale_y(*y);
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="4" fill="{}" />"#,
            fmt_num(px),
            fmt_num(py),
            escape_attr(&accent)
        ));
        if let Some(lbl) = label {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}">{}</text>"#,
                fmt_num(px + 6.0),
                fmt_num(py - 6.0),
                escape_attr(&ink),
                escape_html_chars(lbl)
            ));
        }
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        body
    )))
}

/// chart_area: area chart — same shape as chart_line, but the path closes
/// down to the baseline (chart_y_bottom) and is filled with a translucent
/// accent color. A solid stroke line is drawn on top for definition.
///
/// The fill uses `fill-opacity="0.25"` rather than recomputing a tinted
/// color (no color math needed; the accent color stays within the palette
/// family, just rendered at lower opacity). This matches how `chart_donut`
/// handles visual emphasis — via opacity / stroke, not via separate colors.
pub fn builtin_chart_area(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_area", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_area: data list must not be empty".to_string());
    }
    // Same upper bound as chart_line — area is a line + fill, identical
    // point-density considerations apply.
    if data.len() > 100 {
        return Err(format!(
            "chart_area: too many points ({}), maximum is 100",
            data.len()
        ));
    }

    // Extract {label, value} from each item — same shape as chart_line
    let mut items: Vec<(String, f64)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_area: data[{}] must be Struct {{label, value}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("chart_area item", fields, "label")?;
        let value = struct_float_field("chart_area item", fields, "value")?;
        items.push((label, value));
    }

    let max_value = items
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    if max_value <= 0.0 {
        return Err(format!(
            "chart_area: max value must be positive (got {})",
            max_value
        ));
    }

    let ink = style_token(&style, "ink")?;
    let accent = style_token(&style, "accent")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;
    let paper = style_token(&style, "paper")?;

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        escape_attr(&paper)
    ));

    // Axes
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_TOP),
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM),
        escape_attr(&rule)
    ));
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(CHART_X),
        fmt_num(CHART_Y_BOTTOM),
        fmt_num(CHART_X + CHART_W),
        fmt_num(CHART_Y_BOTTOM),
        escape_attr(&rule)
    ));

    // Compute point positions
    let n = items.len() as f64;
    let points: Vec<(f64, f64, String, f64)> = items
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let x = if items.len() == 1 {
                CHART_X + CHART_W / 2.0
            } else {
                CHART_X + (i as f64) * CHART_W / (n - 1.0)
            };
            let y = CHART_Y_BOTTOM - (value / max_value) * CHART_H;
            (x, y, label.clone(), *value)
        })
        .collect();

    // Build the AREA path:
    //   M x0 y0 L x1 y1 ... L xN yN L xN chart_y_bottom L x0 chart_y_bottom Z
    // This closes the line down to the baseline, forming a filled area.
    let mut area_d = format!("M {} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    for (x, y, _, _) in points.iter().skip(1) {
        area_d.push_str(&format!(" L {} {}", fmt_num(*x), fmt_num(*y)));
    }
    // Close down to baseline: from last point → (last_x, chart_y_bottom)
    // → (first_x, chart_y_bottom) → Z (back to first point)
    // `points` is guaranteed non-empty (checked at function entry), so
    // direct indexing is safe here. We avoid `.last().unwrap()` because
    // the crate denies `clippy::unwrap_used` in non-test code.
    let last_x = points[points.len() - 1].0;
    let first_x = points[0].0;
    area_d.push_str(&format!(
        " L {} {} L {} {} Z",
        fmt_num(last_x),
        fmt_num(CHART_Y_BOTTOM),
        fmt_num(first_x),
        fmt_num(CHART_Y_BOTTOM)
    ));
    // Filled area: accent color at 25% opacity (translucent, so axis/grid
    // shows through). stroke="none" on the fill — we draw a separate
    // stroke line on top for definition.
    parts.push(format!(
        r#"<path d="{}" fill="{}" fill-opacity="0.25" stroke="none" />"#,
        area_d,
        escape_attr(&accent)
    ));

    // Top stroke line (solid accent, no fill) — same M..L path as chart_line
    let mut line_d = format!("M {} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    for (x, y, _, _) in points.iter().skip(1) {
        line_d.push_str(&format!(" L {} {}", fmt_num(*x), fmt_num(*y)));
    }
    parts.push(format!(
        r#"<path d="{}" fill="none" stroke="{}" stroke-width="2" />"#,
        line_d,
        escape_attr(&accent)
    ));

    // X-axis labels — same logic as chart_line (cap at 20 to avoid clutter)
    if items.len() <= 20 {
        for (x, _, label, _) in &points {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(*x),
                fmt_num(CHART_Y_BOTTOM + 18.0),
                escape_attr(&ink),
                escape_html_chars(label)
            ));
        }
    }

    // Peak value annotation (muted)
    let peak_idx = items
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    if let Some((px, py, _, pv)) = points.get(peak_idx) {
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*px),
            fmt_num(*py - 8.0),
            escape_attr(&muted),
            escape_html_chars(&format!("{}", pv))
        ));
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        fmt_num(CHART_CANVAS_W),
        fmt_num(CHART_CANVAS_H),
        body
    )))
}

// ── Level 3 (items 6-8): chart_radar, chart_heatmap, chart_boxplot ──
//
// Narad №79: three additional chart types with distinct math:
//   - chart_radar: multi-series polar coordinates (N axes, M series)
//   - chart_heatmap: HSL color interpolation across a numeric grid
//   - chart_boxplot: statistical summary (R-7 quartiles, IQR whiskers)
//
// Security: chart_radar and chart_boxplot accept user-supplied text
// (axes / series.name for radar, label for boxplot) and are registered
// in SVG_AUTO_ESCAPE_BUILTINS. chart_heatmap data is purely numeric
// (List<List<Float>>) — intentionally NOT added to the lint list (there
// is nothing to scan). All text is escaped via escape_html_chars at
// runtime, same invariant as chart_bar / chart_donut / chart_line.

/// Fixed desaturated 5-color palette for chart_radar series.
///
/// Per spec (Наряд №79 Блок 1): NOT a new DiagramStyle token, just a
/// small hardcoded set of muted hues. Hue spread covers the wheel so
/// adjacent series are visually distinguishable; saturation is held
/// back (~0.45–0.55) so the chart does not scream. If a future narad
/// adds customizable series palettes, this constant can be replaced
/// with a style-derived lookup — the function signature stays stable.
///
/// Colors are produced via the existing `hsl_to_hex` (same code path
/// as `color_palette` in Наряд №77, no duplication of HSL→RGB math):
///   slot 0: hsl(  0, 0.55, 0.55)  muted red-coral
///   slot 1: hsl( 45, 0.55, 0.50)  muted amber
///   slot 2: hsl(135, 0.40, 0.45)  muted green
///   slot 3: hsl(205, 0.50, 0.55)  muted blue
///   slot 4: hsl(285, 0.45, 0.60)  muted violet
fn radar_series_palette() -> Vec<String> {
    vec![
        hsl_to_hex(0.0, 0.55, 0.55),
        hsl_to_hex(45.0, 0.55, 0.50),
        hsl_to_hex(135.0, 0.40, 0.45),
        hsl_to_hex(205.0, 0.50, 0.55),
        hsl_to_hex(285.0, 0.45, 0.60),
    ]
}

/// chart_radar: multi-series radar chart in polar coordinates.
///
/// Data shape (different from bar/line/area):
///   Struct {
///     axes:  List<String>,                  // 3..=12 axis names
///     series: List<Struct{name, values}>,   // 1..=5 series
///   }
///   where each `series.values` length MUST equal `axes.len()`.
///
/// Geometry:
///   - canvas 600×400, center (200, 200), max_radius=130
///   - axis i angle = 2π * i / N - π/2 (start at top, clockwise — the
///     standard radar orientation, matches `chart_donut` start angle)
///   - radius for value v = (v / max_value) * max_radius, where
///     max_value is computed across ALL series (NOT per-series —
///     per-series normalization would make series non-comparable,
///     defeating the entire point of a radar chart)
///   - Each series = one closed `<path>` (Z command), fill-opacity=0.25,
///     solid stroke in the series color (from `radar_series_palette`)
///   - 4 concentric reference rings at 25/50/75/100% (rule color, faint)
///   - N axis spokes from center to perimeter (rule color, faint)
///   - Axis labels at r=148 (outside perimeter), anchor by quadrant
///   - Legend column on the right (same layout as chart_donut)
pub fn builtin_chart_radar(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    // Extract top-level Struct { axes, series }
    let data_fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "chart_radar: data must be Struct {{axes, series}}, got {}",
                other.type_name()
            ));
        }
    };

    // axes: List<String>
    let axes_value = data_fields
        .get("axes")
        .ok_or_else(|| "chart_radar: missing required field 'axes'".to_string())?;
    let axes: Vec<String> = match axes_value {
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, v) in items.iter().enumerate() {
                match v {
                    Value::String(s) => out.push(s.clone()),
                    other => {
                        return Err(format!(
                            "chart_radar: axes[{}] must be String, got {}",
                            i,
                            other.type_name()
                        ));
                    }
                }
            }
            out
        }
        other => {
            return Err(format!(
                "chart_radar: 'axes' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };

    // axes bounds: 3..=12 (radar with <3 axes is not meaningful;
    // >12 axes causes label overlap on a 600px canvas)
    if axes.len() < 3 {
        return Err(format!(
            "chart_radar: at least 3 axes required (got {}) — radar with <3 axes is not meaningful",
            axes.len()
        ));
    }
    if axes.len() > 12 {
        return Err(format!(
            "chart_radar: too many axes ({}), maximum is 12 — labels would overlap",
            axes.len()
        ));
    }

    // series: List<Struct{name, values}>
    let series_value = data_fields
        .get("series")
        .ok_or_else(|| "chart_radar: missing required field 'series'".to_string())?;
    let series_items = match series_value {
        Value::List(items) => items.clone(),
        other => {
            return Err(format!(
                "chart_radar: 'series' must be List<Struct>, got {}",
                other.type_name()
            ));
        }
    };

    if series_items.is_empty() {
        return Err("chart_radar: series list must not be empty".to_string());
    }
    // Palette limit: 5 slots. Per spec: do NOT silently cycle colors —
    // surface a clear error so the caller knows they hit the cap.
    if series_items.len() > 5 {
        return Err(format!(
            "chart_radar: too many series ({}), maximum is 5 — slot palette exhausted",
            series_items.len()
        ));
    }

    // Extract each series: {name: String, values: List<Float>}.
    // values.len() MUST equal axes.len() — otherwise the polygon would
    // not close against the right number of axes.
    let n_axes = axes.len();
    let mut series: Vec<(String, Vec<f64>)> = Vec::with_capacity(series_items.len());
    for (i, item) in series_items.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_radar: series[{}] must be Struct {{name, values}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let name = struct_string_field("chart_radar series", fields, "name")?;
        let values_value = fields
            .get("values")
            .ok_or_else(|| "chart_radar: series missing required field 'values'".to_string())?;
        let values: Vec<f64> = match values_value {
            Value::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (j, v) in items.iter().enumerate() {
                    match v {
                        Value::Float(f) => out.push(*f),
                        other => {
                            return Err(format!(
                                "chart_radar: series[{}].values[{}] must be Float, got {}",
                                i,
                                j,
                                other.type_name()
                            ));
                        }
                    }
                }
                out
            }
            other => {
                return Err(format!(
                    "chart_radar: series[{}].values must be List<Float>, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if values.len() != n_axes {
            return Err(format!(
                "chart_radar: series[{}] '{}' has {} values, expected {} (must match axes.len())",
                i,
                name,
                values.len(),
                n_axes
            ));
        }
        series.push((name, values));
    }

    // Compute global max_value across ALL series (so series are comparable).
    // Per-spec: do NOT normalize per-series — that destroys the comparison.
    let max_value = series
        .iter()
        .flat_map(|(_, vs)| vs.iter())
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    if max_value <= 0.0 {
        return Err(format!(
            "chart_radar: max value across all series must be positive (got {})",
            max_value
        ));
    }

    let paper = style_token(&style, "paper")?;
    let ink = style_token(&style, "ink")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;

    let palette = radar_series_palette();

    // Geometry (centered on the left half — right half reserved for legend,
    // same layout decision as chart_donut for visual parity).
    let canvas_w = 600.0_f64;
    let canvas_h = 400.0_f64;
    let cx = 200.0_f64;
    let cy = 200.0_f64;
    let max_radius = 130.0_f64;
    // Legend (right column, identical to chart_donut geometry)
    let legend_x = 380.0_f64;
    let legend_y_start = 90.0_f64;
    let legend_row_h = 22.0_f64;
    let legend_swatch = 14.0_f64;

    let n = n_axes as f64;

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // 4 concentric reference rings at 25/50/75/100% of max_radius.
    // Rendered first so series polygons draw on top.
    for frac in &[0.25_f64, 0.50, 0.75, 1.00] {
        let r = max_radius * frac;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.3" />"#,
            fmt_num(cx),
            fmt_num(cy),
            fmt_num(r),
            escape_attr(&rule)
        ));
        // Scale label at the top of each ring (small, muted) — gives the
        // viewer a numeric anchor for the radial scale.
        let val_label = max_value * frac;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}" text-anchor="end">{}</text>"#,
            fmt_num(cx - 4.0),
            fmt_num(cy - r + 3.0),
            escape_attr(&muted),
            escape_html_chars(&fmt_num(val_label))
        ));
    }

    // N axis spokes (from center to each perimeter vertex)
    for i in 0..n_axes {
        let angle = 2.0 * std::f64::consts::PI * (i as f64) / n - std::f64::consts::PI / 2.0;
        let (px, py) = polar_to_xy(cx, cy, max_radius, angle);
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" stroke-opacity="0.3" />"#,
            fmt_num(cx),
            fmt_num(cy),
            fmt_num(px),
            fmt_num(py),
            escape_attr(&rule)
        ));
    }

    // Axis labels (at r = max_radius + 18, outside perimeter).
    // Anchor depends on cos(angle): right side → start, left → end,
    // top/bottom → middle. SVG text-anchor keeps labels readable
    // regardless of which side of the wheel they sit on.
    for (i, axis_name) in axes.iter().enumerate() {
        let angle = 2.0 * std::f64::consts::PI * (i as f64) / n - std::f64::consts::PI / 2.0;
        let label_r = max_radius + 18.0;
        let (lx, ly) = polar_to_xy(cx, cy, label_r, angle);
        let anchor = {
            let c = angle.cos();
            if c > 0.3 {
                "start"
            } else if c < -0.3 {
                "end"
            } else {
                "middle"
            }
        };
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="{}">{}</text>"#,
            fmt_num(lx),
            fmt_num(ly + 4.0),
            escape_attr(&ink),
            anchor,
            escape_html_chars(axis_name)
        ));
    }

    // Each series: one closed <path> through its scaled points.
    // Rendered in order series[0..N]; later series draw on top of earlier
    // ones. With fill-opacity=0.25, overlaps remain visible.
    for (idx, (sname, svalues)) in series.iter().enumerate() {
        let color = &palette[idx];
        // Compute (x, y) for each axis point
        let pts: Vec<(f64, f64)> = svalues
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let angle =
                    2.0 * std::f64::consts::PI * (i as f64) / n - std::f64::consts::PI / 2.0;
                let radius = (*v / max_value) * max_radius;
                polar_to_xy(cx, cy, radius, angle)
            })
            .collect();
        // Build path: M x0 y0 L x1 y1 ... L xN yN Z (closed polygon)
        let mut d = format!("M {} {}", fmt_num(pts[0].0), fmt_num(pts[0].1));
        for (x, y) in pts.iter().skip(1) {
            d.push_str(&format!(" L {} {}", fmt_num(*x), fmt_num(*y)));
        }
        d.push_str(" Z");
        parts.push(format!(
            r#"<path d="{}" fill="{}" fill-opacity="0.25" stroke="{}" stroke-width="2" />"#,
            d,
            escape_attr(color),
            escape_attr(color)
        ));
        // Small dot at each vertex — helps readability when polygons overlap.
        for (x, y) in &pts {
            parts.push(format!(
                r#"<circle cx="{}" cy="{}" r="2.5" fill="{}" />"#,
                fmt_num(*x),
                fmt_num(*y),
                escape_attr(color)
            ));
        }
        // Legend entry: swatch + series name
        let ly = legend_y_start + (idx as f64) * legend_row_h;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
            fmt_num(legend_x),
            fmt_num(ly),
            fmt_num(legend_swatch),
            fmt_num(legend_swatch),
            escape_attr(color)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" dominant-baseline="middle">{}</text>"#,
            fmt_num(legend_x + legend_swatch + 8.0),
            fmt_num(ly + legend_swatch / 2.0),
            escape_attr(&ink),
            escape_html_chars(sname)
        ));
    }

    // Legend separator line (matches chart_donut visual treatment)
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(legend_x),
        fmt_num(legend_y_start - 12.0),
        fmt_num(canvas_w - 20.0),
        fmt_num(legend_y_start - 12.0),
        escape_attr(&rule)
    ));

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

/// Parse "#rrggbb" hex color to (h, s, l) in HSL space.
/// h: 0..=360 degrees, s: 0..=1, l: 0..=1.
/// Returns None if the string is malformed (wrong prefix, wrong length,
/// or non-hex digits).
///
/// Used by chart_heatmap (Наряд №79 Блок 2) to convert the `paper` and
/// `accent` style tokens into HSL so we can interpolate in HSL space
/// (avoiding the muddy browns that RGB interpolation produces between
/// complementary hues). The forward direction `hsl_to_hex` already
/// exists for `color_palette` (Наряд №77); this is the inverse.
fn hex_to_hsl(hex: &str) -> Option<(f64, f64, f64)> {
    let s = hex.strip_prefix('#')?;
    // Accept both 6-digit (#rrggbb) and 3-digit (#rgb) shorthand.
    // The 3-digit form is expanded by doubling each digit: #fff → #ffffff,
    // #abc → #aabbcc. This matches CSS / SVG color parsing conventions.
    let expanded: String = if s.len() == 3 {
        let chars = s.chars().collect::<Vec<_>>();
        format!(
            "{}{}{}{}{}{}",
            chars[0], chars[0], chars[1], chars[1], chars[2], chars[2]
        )
    } else if s.len() == 6 {
        s.to_string()
    } else {
        return None;
    };
    let r = u8::from_str_radix(&expanded[0..2], 16).ok()?;
    let g = u8::from_str_radix(&expanded[2..4], 16).ok()?;
    let b = u8::from_str_radix(&expanded[4..6], 16).ok()?;
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        // Achromatic (gray) — hue undefined, saturation 0
        return Some((0.0, 0.0, l));
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    let h = h * 60.0;
    let h = if h < 0.0 { h + 360.0 } else { h };
    Some((h, s, l))
}

/// Linear interpolation between two HSL colors. `t` in [0, 1].
/// Hue takes the shorter arc around the wheel (handles wraparound,
/// so interpolating from h=350 to h=10 goes forward through 0, not
/// backward through 180).
fn interpolate_hsl(c1: (f64, f64, f64), c2: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    let (h1, s1, l1) = c1;
    let (h2, s2, l2) = c2;
    // Shorter hue arc
    let dh = if (h2 - h1).abs() > 180.0 {
        if h2 > h1 {
            h2 - h1 - 360.0
        } else {
            h2 - h1 + 360.0
        }
    } else {
        h2 - h1
    };
    let h = (h1 + t * dh + 360.0) % 360.0;
    let s = s1 + t * (s2 - s1);
    let l = l1 + t * (l2 - l1);
    (h, s, l)
}

/// chart_heatmap: numeric grid rendered with HSL color interpolation.
///
/// Data shape (pure numeric — no user text, intentionally NOT in the
/// security lint list):
///   List<List<Float>>   // rows of equal length; row count and col count
///                       // both must be in 1..=30
///
/// Color: each cell value is normalized to [min, max] across the entire
/// grid, then linearly interpolated in HSL space between `style.paper`
/// (low value) and `style.accent` (high value). HSL chosen over RGB to
/// avoid the muddy intermediate hues RGB produces between complements.
///
/// The HSL helpers `hex_to_hsl` and `interpolate_hsl` are private to
/// this module and are NOT exposed to .mlog programs — the surface area
/// for the spec is "give us a grid, get a heatmap SVG", nothing more.
///
/// Geometry: same canvas constants as chart_bar (600×400, chart_x=80,
/// chart_y_top=40, chart_w=500, chart_h=300). Each cell is a `<rect>`
/// of size (chart_w/cols) × (chart_h/rows) with a 1px right/bottom gap
/// for visual separation (last row/col fills to the border so the outer
/// rectangle stays clean). Below the chart: min/max value labels and a
/// small color-scale strip as a visual key.
pub fn builtin_chart_heatmap(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_heatmap", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_heatmap: data list must not be empty".to_string());
    }

    // Extract rows of floats. All rows must have equal length — otherwise
    // the grid is not rectangular and cannot be rendered as a heatmap.
    let mut grid: Vec<Vec<f64>> = Vec::with_capacity(data.len());
    let mut cols: usize = 0;
    for (i, row_value) in data.iter().enumerate() {
        let row = match row_value {
            Value::List(items) => items,
            other => {
                return Err(format!(
                    "chart_heatmap: row {} must be List<Float>, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if row.is_empty() {
            return Err(format!(
                "chart_heatmap: row {} is empty — cannot render zero-width grid",
                i
            ));
        }
        if i == 0 {
            cols = row.len();
        } else if row.len() != cols {
            return Err(format!(
                "chart_heatmap: row {} has length {}, expected {} (rows must be equal length)",
                i,
                row.len(),
                cols
            ));
        }
        let mut row_floats: Vec<f64> = Vec::with_capacity(row.len());
        for (j, v) in row.iter().enumerate() {
            match v {
                Value::Float(f) => row_floats.push(*f),
                other => {
                    return Err(format!(
                        "chart_heatmap: row {} col {} must be Float, got {}",
                        i,
                        j,
                        other.type_name()
                    ));
                }
            }
        }
        grid.push(row_floats);
    }

    let rows = grid.len();
    if rows > 30 {
        return Err(format!(
            "chart_heatmap: too many rows ({}), maximum is 30 — cells become unreadably small",
            rows
        ));
    }
    if cols > 30 {
        return Err(format!(
            "chart_heatmap: too many cols ({}), maximum is 30 — cells become unreadably small",
            cols
        ));
    }

    // Global min/max across the entire grid (single scale, not per-row).
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for row in &grid {
        for &v in row {
            if v < min_value {
                min_value = v;
            }
            if v > max_value {
                max_value = v;
            }
        }
    }
    if !min_value.is_finite() || !max_value.is_finite() {
        return Err("chart_heatmap: grid contains non-finite value".to_string());
    }

    let paper = style_token(&style, "paper")?;
    let accent = style_token(&style, "accent")?;
    let ink = style_token(&style, "ink")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;

    // Precompute HSL endpoints for interpolation. Reusing hex_to_hsl here
    // (added by this narad) — the spec explicitly says to check whether
    // HSL interpolation was already present from Наряд №77 and reuse it.
    // The forward direction (hsl_to_hex) IS from №77; the inverse
    // (hex_to_hsl) is new in this narad because №77 only generates
    // colors from hue/sat/light, never the reverse.
    let paper_hsl = hex_to_hsl(&paper).ok_or_else(|| {
        format!(
            "chart_heatmap: paper token {:?} is not a valid #rrggbb hex color",
            paper
        )
    })?;
    let accent_hsl = hex_to_hsl(&accent).ok_or_else(|| {
        format!(
            "chart_heatmap: accent token {:?} is not a valid #rrggbb hex color",
            accent
        )
    })?;

    let span = max_value - min_value;

    // Geometry constants (shared with chart_bar / chart_line / chart_area
    // for visual parity across chart types).
    let canvas_w = CHART_CANVAS_W;
    let canvas_h = CHART_CANVAS_H;
    let chart_x = CHART_X;
    let chart_y_top = CHART_Y_TOP;
    let chart_w = CHART_W;
    let chart_h = CHART_H;
    let chart_y_bottom = CHART_Y_BOTTOM;

    let cell_w = chart_w / cols as f64;
    let cell_h = chart_h / rows as f64;

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Plot border (rule color)
    parts.push(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(chart_x),
        fmt_num(chart_y_top),
        fmt_num(chart_w),
        fmt_num(chart_h),
        escape_attr(&rule)
    ));

    // Each cell: rect with interpolated color.
    // 1px right/bottom gap for visual separation (cell rendered at w-1, h-1).
    // The last column / last row get the full remaining size so the outer
    // border stays flush with the plot rectangle.
    for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            let x = chart_x + (c as f64) * cell_w;
            let y = chart_y_top + (r as f64) * cell_h;
            let w = if c + 1 == cols {
                chart_x + chart_w - x
            } else {
                cell_w - 1.0
            };
            let h = if r + 1 == rows {
                chart_y_top + chart_h - y
            } else {
                cell_h - 1.0
            };
            // Degenerate case (all values equal): pick mid-color so the
            // chart still renders visibly, rather than dividing by zero.
            let t = if span.abs() < f64::EPSILON {
                0.5
            } else {
                ((v - min_value) / span).clamp(0.0, 1.0)
            };
            let (h_h, h_s, h_l) = interpolate_hsl(paper_hsl, accent_hsl, t);
            let fill = hsl_to_hex(h_h, h_s, h_l);
            parts.push(format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
                fmt_num(x),
                fmt_num(y),
                fmt_num(w),
                fmt_num(h),
                escape_attr(&fill)
            ));
        }
    }

    // Min/max value labels (below the chart, anchored to the corners)
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}">{}</text>"#,
        fmt_num(chart_x),
        fmt_num(chart_y_bottom + 18.0),
        escape_attr(&muted),
        escape_html_chars(&format!("min={}", fmt_num(min_value)))
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(chart_x + chart_w),
        fmt_num(chart_y_bottom + 18.0),
        escape_attr(&muted),
        escape_html_chars(&format!("max={}", fmt_num(max_value)))
    ));

    // Color-scale strip (below the chart, centered): 20 swatches showing
    // the paper→accent gradient as a visual key for the cell colors.
    let strip_y = chart_y_bottom + 26.0;
    let strip_h = 8.0;
    let strip_w = 80.0;
    let strip_x = chart_x + (chart_w - strip_w) / 2.0;
    for i in 0..20 {
        let t = i as f64 / 19.0;
        let (h_h, h_s, h_l) = interpolate_hsl(paper_hsl, accent_hsl, t);
        let fill = hsl_to_hex(h_h, h_s, h_l);
        let sx = strip_x + t * strip_w;
        let sw = strip_w / 20.0 + 1.0; // +1px to avoid antialiasing gaps
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
            fmt_num(sx),
            fmt_num(strip_y),
            fmt_num(sw),
            fmt_num(strip_h),
            escape_attr(&fill)
        ));
    }
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="9" fill="{}" text-anchor="middle">{}</text>"#,
        fmt_num(strip_x + strip_w / 2.0),
        fmt_num(strip_y + strip_h + 10.0),
        escape_attr(&ink),
        "value scale"
    ));

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

/// R-7 percentile method (linear interpolation between closest ranks).
/// Same as `numpy.percentile` with `interpolation='linear'` (the default),
/// same as R's `quantile(type=7)`, same as Excel's `PERCENTILE.INC`.
///
/// Input MUST be sorted ascending. p in [0, 100]. Returned value is a
/// linear interpolation between two adjacent data points when the rank
/// is not an integer; otherwise it's the exact data point at that rank.
///
/// Reference:
///   rank = (p/100) * (n - 1)         // 0-indexed position in the sorted array
///   lo = floor(rank), hi = ceil(rank)
///   if lo == hi: x[lo]
///   else: x[lo] + (rank - lo) * (x[hi] - x[lo])
///
/// Why R-7 (and not R-6/exclusive or nearest-rank): R-7 is the de-facto
/// default in both numpy and R. On small samples the methods diverge
/// visibly (e.g. n=4 → R-6 Q1 = x[0]+0.25*(x[1]-x[0]); R-7 Q1 = x[0]),
/// and the contract test in tests/p79_charts.rs pins specific expected
/// numbers that are only correct under R-7. Changing the method without
/// updating the contract would silently break the test.
fn percentile_r7(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return sorted[0];
    }
    let rank = (p / 100.0) * (n as f64 - 1.0);
    let lo = rank.floor();
    let hi = rank.ceil();
    if lo == hi {
        // rank is an integer — direct index (lo is an integer here).
        sorted[lo as usize]
    } else {
        let lo_i = lo as usize;
        let hi_i = hi as usize;
        sorted[lo_i] + (rank - lo) * (sorted[hi_i] - sorted[lo_i])
    }
}

/// chart_boxplot: per-label statistical box-and-whisker plot.
///
/// Data shape (same outer List<Struct> pattern as chart_bar, but the
/// inner struct has `values: List<Float>` instead of `value: Float`):
///   List<Struct{ label: String, values: List<Float> }>
///
/// Statistics are computed INSIDE the function from raw `values` — the
/// caller does NOT pass pre-computed quartiles. This is important: the
/// contract test in tests/p79_charts.rs independently verifies the
/// quartile numbers, and that verification is only meaningful if the
/// function does the math itself.
///
/// Method (pinned, do not change without updating the contract):
///   - Q1 = percentile_r7(sorted, 25)
///   - median = percentile_r7(sorted, 50)
///   - Q3 = percentile_r7(sorted, 75)
///   - IQR = Q3 - Q1
///   - whisker_low  = min(v in values where v >= Q1 - 1.5*IQR)
///   - whisker_high = max(v in values where v <= Q3 + 1.5*IQR)
///   - outliers     = all v with v < Q1 - 1.5*IQR OR v > Q3 + 1.5*IQR
///
/// Geometry: same canvas constants as chart_bar (600×400, chart_x=80,
/// chart_y_top=40, chart_w=500, chart_h=300). Box width = chart_w/N - 20
/// (gap = 20, same as chart_bar). Y axis is scaled across ALL values in
/// ALL boxes (global min/max), so boxes are visually comparable.
pub fn builtin_chart_boxplot(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("chart_boxplot", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    if data.is_empty() {
        return Err("chart_boxplot: data list must not be empty".to_string());
    }
    if data.len() > 20 {
        return Err(format!(
            "chart_boxplot: too many boxes ({}), maximum is 20",
            data.len()
        ));
    }

    // Per-box precomputed statistics. Defined at module scope would also
    // work, but keeping it local to the function makes the data flow
    // explicit: nothing else in the module needs this struct.
    struct BoxData {
        label: String,
        sorted: Vec<f64>,
        q1: f64,
        median: f64,
        q3: f64,
        whisker_low: f64,
        whisker_high: f64,
        outliers: Vec<f64>,
    }

    let mut boxes: Vec<BoxData> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let fields = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "chart_boxplot: data[{}] must be Struct {{label, values}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("chart_boxplot item", fields, "label")?;
        let values_value = fields
            .get("values")
            .ok_or_else(|| "chart_boxplot: item missing required field 'values'".to_string())?;
        let values: Vec<f64> = match values_value {
            Value::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (j, v) in items.iter().enumerate() {
                    match v {
                        Value::Float(f) => out.push(*f),
                        other => {
                            return Err(format!(
                                "chart_boxplot: data[{}].values[{}] must be Float, got {}",
                                i,
                                j,
                                other.type_name()
                            ));
                        }
                    }
                }
                out
            }
            other => {
                return Err(format!(
                    "chart_boxplot: data[{}].values must be List<Float>, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        // Min sample size 4: below that, quartiles are not meaningful
        // (R-7 on n=3 gives Q1 = x[0], Q3 = x[2], IQR = x[2]-x[0] —
        // degenerate; whiskers would equal the data range, no outliers
        // ever). The error message names the offending label so the
        // caller can locate the bad input.
        if values.len() < 4 {
            return Err(format!(
                "chart_boxplot: data[{}] '{}' has {} values, minimum is 4 — quartiles not meaningful on smaller samples",
                i,
                label,
                values.len()
            ));
        }
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let q1 = percentile_r7(&sorted, 25.0);
        let median = percentile_r7(&sorted, 50.0);
        let q3 = percentile_r7(&sorted, 75.0);
        let iqr = q3 - q1;
        let low_fence = q1 - 1.5 * iqr;
        let high_fence = q3 + 1.5 * iqr;
        // whisker_low = min(v where v >= low_fence) — nearest data point
        // inside the fence, NOT the fence value itself.
        let whisker_low = sorted
            .iter()
            .copied()
            .filter(|v| *v >= low_fence)
            .fold(f64::INFINITY, f64::min);
        // whisker_high = max(v where v <= high_fence)
        let whisker_high = sorted
            .iter()
            .copied()
            .filter(|v| *v <= high_fence)
            .fold(f64::NEG_INFINITY, f64::max);
        // Outliers: data points strictly outside the fences (not on them).
        let outliers: Vec<f64> = sorted
            .iter()
            .copied()
            .filter(|v| *v < low_fence || *v > high_fence)
            .collect();
        boxes.push(BoxData {
            label,
            sorted,
            q1,
            median,
            q3,
            whisker_low,
            whisker_high,
            outliers,
        });
    }

    // Global min/max across ALL values in ALL boxes — drives the Y axis
    // scale so boxes are visually comparable to each other.
    let mut global_min = f64::INFINITY;
    let mut global_max = f64::NEG_INFINITY;
    for b in &boxes {
        for &v in &b.sorted {
            if v < global_min {
                global_min = v;
            }
            if v > global_max {
                global_max = v;
            }
        }
    }
    if !global_min.is_finite() || !global_max.is_finite() {
        return Err("chart_boxplot: data contains non-finite value".to_string());
    }
    let span = global_max - global_min;
    if span.abs() < f64::EPSILON {
        return Err(format!(
            "chart_boxplot: all values are identical (={}) — cannot scale Y axis",
            global_min
        ));
    }

    let paper = style_token(&style, "paper")?;
    let ink = style_token(&style, "ink")?;
    let accent = style_token(&style, "accent")?;
    let muted = style_token(&style, "muted")?;
    let rule = style_token(&style, "rule")?;

    // Geometry (shared canvas constants with chart_bar / chart_line)
    let canvas_w = CHART_CANVAS_W;
    let canvas_h = CHART_CANVAS_H;
    let chart_x = CHART_X;
    let chart_y_top = CHART_Y_TOP;
    let chart_w = CHART_W;
    let chart_h = CHART_H;
    let chart_y_bottom = CHART_Y_BOTTOM;
    let gap = 20.0_f64;
    let n = boxes.len() as f64;
    let box_w = (chart_w / n) - gap;
    if box_w < 10.0 {
        return Err(format!(
            "chart_boxplot: too many boxes ({}) for canvas width — box width would be {}",
            boxes.len(),
            box_w
        ));
    }

    // Y scale: value → SVG y coordinate (Y inverted — higher value = smaller y)
    let scale_y = |v: f64| -> f64 { chart_y_bottom - (v - global_min) / span * chart_h };

    let mut parts: Vec<String> = Vec::new();

    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Axes
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(chart_x),
        fmt_num(chart_y_top),
        fmt_num(chart_x),
        fmt_num(chart_y_bottom),
        escape_attr(&rule)
    ));
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(chart_x),
        fmt_num(chart_y_bottom),
        fmt_num(chart_x + chart_w),
        fmt_num(chart_y_bottom),
        escape_attr(&rule)
    ));

    // Y axis range labels (min at bottom, max at top)
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(chart_x - 6.0),
        fmt_num(chart_y_bottom + 4.0),
        escape_attr(&ink),
        escape_html_chars(&fmt_num(global_min))
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(chart_x - 6.0),
        fmt_num(chart_y_top + 4.0),
        escape_attr(&ink),
        escape_html_chars(&fmt_num(global_max))
    ));

    // Each box: rect (Q1..Q3) + median line + whiskers + caps + outliers
    for (i, b) in boxes.iter().enumerate() {
        let x_left = chart_x + (i as f64) * (box_w + gap) + gap / 2.0;
        let x_right = x_left + box_w;
        let x_center = (x_left + x_right) / 2.0;

        let y_q1 = scale_y(b.q1);
        let y_q3 = scale_y(b.q3);
        let y_median = scale_y(b.median);
        let y_wlow = scale_y(b.whisker_low);
        let y_whigh = scale_y(b.whisker_high);

        // Box: rect from Q1 (bottom) to Q3 (top). Q3 > Q1 in value space,
        // but in SVG y-space y_q3 < y_q1 (Y inverted). Box height = y_q1 - y_q3.
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" fill-opacity="0.25" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(x_left),
            fmt_num(y_q3),
            fmt_num(box_w),
            fmt_num(y_q1 - y_q3),
            escape_attr(&accent),
            escape_attr(&accent)
        ));

        // Median line (horizontal, inside the box)
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" />"#,
            fmt_num(x_left),
            fmt_num(y_median),
            fmt_num(x_right),
            fmt_num(y_median),
            escape_attr(&accent)
        ));

        // Whisker: vertical line from box center, top to whisker_high,
        // bottom to whisker_low. Two segments (above box, below box) so
        // they don't draw through the box fill.
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(x_center),
            fmt_num(y_q3),
            fmt_num(x_center),
            fmt_num(y_whigh),
            escape_attr(&rule)
        ));
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(x_center),
            fmt_num(y_q1),
            fmt_num(x_center),
            fmt_num(y_wlow),
            escape_attr(&rule)
        ));

        // Whisker caps (short horizontal lines at top/bottom of whiskers)
        let cap_half = box_w / 4.0;
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(x_center - cap_half),
            fmt_num(y_whigh),
            fmt_num(x_center + cap_half),
            fmt_num(y_whigh),
            escape_attr(&rule)
        ));
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(x_center - cap_half),
            fmt_num(y_wlow),
            fmt_num(x_center + cap_half),
            fmt_num(y_wlow),
            escape_attr(&rule)
        ));

        // Outliers: hollow circles (no fill, muted stroke) at (box_center, y_v)
        for &v in &b.outliers {
            let y_v = scale_y(v);
            parts.push(format!(
                r#"<circle cx="{}" cy="{}" r="3" fill="none" stroke="{}" stroke-width="1" />"#,
                fmt_num(x_center),
                fmt_num(y_v),
                escape_attr(&muted)
            ));
        }

        // X-axis label (below the chart)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x_center),
            fmt_num(chart_y_bottom + 18.0),
            escape_attr(&ink),
            escape_html_chars(&b.label)
        ));
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Level 2.6: color_palette — derived DiagramStyle from intent + mode ──
//
// Narad №77 Block 1: generates a 5-token DiagramStyle (paper/ink/accent/
// muted/rule) from a named intent and a light/dark mode.
//
// Source of intent→hue: skills/pdf/scripts/design_engine.py::INTENT_HUES
//   calm      → 210  (steel blue-grey)
//   tension   → 0    (warm near-black vs cold)
//   energy    → 30   (amber undertone)
//   authority → 280  (muted violet, formal)
//   warmth    → 20   (terracotta)
//
// Derivation formulas: skills/pdf/typesetting/palette.md
//   Primary:       hsl(H, S, L)
//   Dark variant:  hsl(H, S, L-15%)
//   Light variant: hsl(H, S-10%, L+25%)
//   Ultra-light bg: hsl(H, S-20%, 96%)
//   Accent:        hsl(H+15, S, L)  ← overridden by V2.1 rule
//
// V2.1 rule (palette.md): accent MUST share base_hue with structural roles.
// Only S/L differ. This makes paper/ink/accent/muted/rule visibly belong to
// the same color family.
//
// Tier system (palette.md): area ∝ 1/saturation.
//   XL (>50%):  paper    → S ≤ 0.08 (light) / S ≤ 0.10 (dark, near-black)
//   L  (20-50%): rule     → S ≤ 0.15-0.20
//   M  (5-20%):  ink      → S ≤ 0.30 (light) / S ≤ 0.05 (dark, near-white)
//   S  (1-5%):   muted    → S ≤ 0.50
//   XS (<1%):    accent   → S ≤ 0.75 (typically 0.55-0.65)
//
// Output: Value::Struct { type_name: "DiagramStyle", fields: {paper,ink,accent,muted,rule} }
// Same shape as builtin_diagram_style — directly consumable by chart_bar,
// chart_donut, and any future chart_* without adaptation.

/// Intent name → base hue (degrees on HSL wheel).
/// Values sourced verbatim from design_engine.py::INTENT_HUES.
fn intent_to_hue(intent: &str) -> Option<f64> {
    match intent {
        "calm" => Some(210.0),
        "tension" => Some(0.0),
        "energy" => Some(30.0),
        "authority" => Some(280.0),
        "warmth" => Some(20.0),
        _ => None,
    }
}

/// Convert HSL color to hex string (#rrggbb).
/// h: 0-360 degrees, s: 0.0-1.0, l: 0.0-1.0.
fn hsl_to_hex(h: f64, s: f64, l: f64) -> String {
    // Normalize hue to [0, 360)
    let h_norm = ((h % 360.0) + 360.0) % 360.0 / 360.0;
    let s_clamped = s.clamp(0.0, 1.0);
    let l_clamped = l.clamp(0.0, 1.0);

    let (r, g, b) = if s_clamped == 0.0 {
        (l_clamped, l_clamped, l_clamped)
    } else {
        let q = if l_clamped < 0.5 {
            l_clamped * (1.0 + s_clamped)
        } else {
            l_clamped + s_clamped - l_clamped * s_clamped
        };
        let p = 2.0 * l_clamped - q;

        let hue2rgb = |p: f64, q: f64, mut t: f64| -> f64 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 1.0 / 2.0 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };

        (
            hue2rgb(p, q, h_norm + 1.0 / 3.0),
            hue2rgb(p, q, h_norm),
            hue2rgb(p, q, h_norm - 1.0 / 3.0),
        )
    };

    let to_u8 = |c: f64| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", to_u8(r), to_u8(g), to_u8(b))
}

/// `color_palette(intent: String, mode: String) -> Struct`
/// Returns DiagramStyle { paper, ink, accent, muted, rule } derived from
/// intent (calm/tension/energy/authority/warmth) and mode (light/dark).
pub fn builtin_color_palette(args: &[Value]) -> Result<Value, String> {
    let intent = expect_string_arg("color_palette", args, 0)?;
    let mode = expect_string_arg("color_palette", args, 1)?;

    let base_hue = intent_to_hue(&intent).ok_or_else(|| {
        format!(
            "color_palette: intent must be one of calm/tension/energy/authority/warmth (got {:?})",
            intent
        )
    })?;

    // Derive 5 tokens. All structural roles use base_hue (V2.1 rule).
    // Only saturation/lightness differ. Tier caps from palette.md are respected.
    let (paper, ink, accent, muted, rule) = match mode.as_str() {
        "light" => {
            // Light mode: paper is ultra-light tinted bg, ink is dark text,
            // accent is vibrant (XS tier), muted is mid-L gray, rule is
            // a subtle L-tier divider.
            // Per palette.md: paper = hsl(H, S-20%, 96%) (S very low).
            //                 ink   = hsl(H, S, L_low).
            //                 accent= hsl(H, S_high, L_mid)  [V2.1: same hue].
            let paper = hsl_to_hex(base_hue, 0.06, 0.96);
            let ink = hsl_to_hex(base_hue, 0.30, 0.15);
            let accent = hsl_to_hex(base_hue, 0.65, 0.45);
            let muted = hsl_to_hex(base_hue, 0.10, 0.50);
            let rule = hsl_to_hex(base_hue, 0.15, 0.85);
            (paper, ink, accent, muted, rule)
        }
        "dark" => {
            // Dark mode: paper is very dark (XL tier, near-black with slight
            // hue tint), ink is near-white text, accent is vibrant on dark
            // (XS tier, higher L for visibility), muted is mid-L gray,
            // rule is a visible S-tier divider on dark bg.
            let paper = hsl_to_hex(base_hue, 0.10, 0.06);
            let ink = hsl_to_hex(base_hue, 0.05, 0.92);
            let accent = hsl_to_hex(base_hue, 0.60, 0.58);
            let muted = hsl_to_hex(base_hue, 0.10, 0.55);
            let rule = hsl_to_hex(base_hue, 0.20, 0.18);
            (paper, ink, accent, muted, rule)
        }
        _ => {
            return Err(format!(
                "color_palette: mode must be \"light\" or \"dark\" (got {:?})",
                mode
            ));
        }
    };

    let mut style_fields = HashMap::new();
    style_fields.insert("paper".to_string(), Value::String(paper));
    style_fields.insert("ink".to_string(), Value::String(ink));
    style_fields.insert("accent".to_string(), Value::String(accent));
    style_fields.insert("muted".to_string(), Value::String(muted));
    style_fields.insert("rule".to_string(), Value::String(rule));
    Ok(Value::Struct {
        type_name: "DiagramStyle".to_string(),
        fields: style_fields,
    })
}

// ── Level 2.5a: svg_sketchy_filter ───────────────────────────────────
//
// Returns a <filter> element to be placed in <defs>. Apply via
// filter="url(#id)" on a group of SHAPES (NEVER on text).
// Implementation: standard SVG filter feTurbulence + feDisplacementMap.

pub fn builtin_svg_sketchy_filter(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("svg_sketchy_filter", args, 0)?;
    let base_frequency = if args.len() > 1 {
        expect_float_arg("svg_sketchy_filter", args, 1)?
    } else {
        0.02
    };
    let num_octaves = if args.len() > 2 {
        expect_float_arg("svg_sketchy_filter", args, 2)?
    } else {
        3.0
    };
    let scale = if args.len() > 3 {
        expect_float_arg("svg_sketchy_filter", args, 3)?
    } else {
        4.0
    };
    let seed = if args.len() > 4 {
        expect_float_arg("svg_sketchy_filter", args, 4)?
    } else {
        1.0
    };
    if base_frequency <= 0.0 || base_frequency > 1.0 {
        return Err(format!(
            "svg_sketchy_filter: base_frequency must be in (0, 1], got {}",
            base_frequency
        ));
    }
    if !(1.0..=8.0).contains(&num_octaves) {
        return Err(format!(
            "svg_sketchy_filter: num_octaves must be in [1, 8], got {}",
            num_octaves
        ));
    }
    // Validate id (must be a valid XML ID — no spaces, no <, >, ", ')
    if id.is_empty()
        || id.contains(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == '\'')
    {
        return Err(format!(
            "svg_sketchy_filter: id must be a non-empty XML-safe identifier (got {:?})",
            id
        ));
    }
    Ok(Value::String(format!(
        r#"<filter id="{}"><feTurbulence type="fractalNoise" baseFrequency="{}" numOctaves="{}" seed="{}" result="noise" /><feDisplacementMap in="SourceGraphic" in2="noise" scale="{}" /></filter>"#,
        escape_attr(&id),
        fmt_num(base_frequency),
        num_octaves as i64,
        fmt_num(seed),
        fmt_num(scale)
    )))
}

// ── Level 2.5b: svg_icon (Tabler-compatible MIT icon set) ────────────
//
// 10 starter icons, 24x24 viewBox, stroke="currentColor" so they inherit
// color from the parent group. Path data sourced from Tabler Icons (MIT).
// Returns a complete <svg> fragment that can be placed via <use> or
// embedded directly in a <g transform="translate(x,y) scale(s)">.

/// Map icon name to its path data. Returns None for unknown names.
fn icon_path_data(name: &str) -> Option<&'static str> {
    match name {
        "server" => Some("M3 4m0 3 0 8a1 1 0 0 0 1 1h16a1 1 0 0 0 1 -1v-8a1 1 0 0 0 -1 -1h-16a1 1 0 0 0 -1 1zm0 -3.5m0 1a1 1 0 0 1 1 -1h16a1 1 0 0 1 1 1v0a1 1 0 0 1 -1 1h-16a1 1 0 0 1 -1 -1zm7 4.5l0 .01m-7 4l0 .01m7 0l0 .01"),
        "laptop" => Some("M3 19h18M5 6m0 1a1 1 0 0 1 1 -1h12a1 1 0 0 1 1 1v8a1 1 0 0 1 -1 1h-12a1 1 0 0 1 -1 -1zm3 12h8"),
        "phone" => Some("M5 4m0 2a2 2 0 0 1 2 -2h10a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-10a2 2 0 0 1 -2 -2zm6 1l2 0"),
        "database" => Some("M4 6m0 3a8 3 0 1 0 16 0a8 3 0 1 0 -16 0M4 6v6a8 3 0 0 0 16 0v-6M4 12v6a8 3 0 0 0 16 0v-6"),
        "cloud" => Some("M7 18a4 4 0 0 1 0 -8a6 6 0 0 1 11.5 1.5a3.5 3.5 0 0 1 -.5 6.5z"),
        "arrow-right" => Some("M5 12l14 0m-4 -4l4 4l-4 4"),
        "check" => Some("M5 12l5 5l9 -10"),
        "warning" => Some("M12 9v4m0 4l0 .01M10.285 3.875l-7.285 12.625a1 1 0 0 0 .86 1.5h14.29a1 1 0 0 0 .86 -1.5l-7.285 -12.625a1 1 0 0 0 -1.72 0z"),
        "user" => Some("M12 7m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0M5.5 21a6.5 6.5 0 0 1 13 0"),
        "document" => Some("M14 3v4a1 1 0 0 0 1 1h4M5 3m0 2a2 2 0 0 1 2 -2h7l5 5v11a2 2 0 0 1 -2 2h-10a2 2 0 0 1 -2 -2zm5 8h4m-4 4h4"),
        _ => None,
    }
}

/// `svg_icon(name: String, x, y, size, color) -> String`
/// Returns an <svg> fragment (24x24 viewBox, scaled to `size`).
pub fn builtin_svg_icon(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("svg_icon", args, 0)?;
    let x = expect_float_arg("svg_icon", args, 1)?;
    let y = expect_float_arg("svg_icon", args, 2)?;
    let size = expect_float_arg("svg_icon", args, 3)?;
    let color = expect_string_arg("svg_icon", args, 4)?;
    if size <= 0.0 {
        return Err(format!("svg_icon: size must be positive (got {})", size));
    }
    let path_data = icon_path_data(&name).ok_or_else(|| {
        format!(
            "svg_icon: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
            name
        )
    })?;
    // Wrap in nested svg with x/y positioning. Outer svg is positioned,
    // inner svg (with viewBox 0 0 24 24) does the scaling.
    let scale = size / 24.0;
    Ok(Value::String(format!(
        r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 24 24"><g transform="scale({})"><path d="{}" stroke="{}" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></g></svg>"#,
        fmt_num(x),
        fmt_num(y),
        fmt_num(size),
        fmt_num(size),
        fmt_num(scale),
        path_data,
        escape_attr(&color)
    )))
}

// ── Level 2.5c: svg_callout ──────────────────────────────────────────
//
// Editorial annotation: italic text + dashed Bezier curve + anchor dot.
// Visually distinct from regular diagram connections (those are solid).

pub fn builtin_svg_callout(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("svg_callout", args, 0)?;
    let from_x = expect_float_arg("svg_callout", args, 1)?;
    let from_y = expect_float_arg("svg_callout", args, 2)?;
    let to_x = expect_float_arg("svg_callout", args, 3)?;
    let to_y = expect_float_arg("svg_callout", args, 4)?;
    let intent = if args.len() > 5 {
        expect_string_arg("svg_callout", args, 5)?
    } else {
        "neutral".to_string()
    };
    if !matches!(intent.as_str(), "neutral" | "accent" | "muted") {
        return Err(format!(
            r#"svg_callout: intent must be "neutral", "accent", or "muted" (got "{}")"#,
            intent
        ));
    }
    // Default colors (callout does NOT require a style — uses sensible defaults)
    let (line_color, text_color) = match intent.as_str() {
        "accent" => ("#eb6c36", "#eb6c36"),
        "muted" => ("#4f5d75", "#4f5d75"),
        _ => ("#2d3142", "#2d3142"),
    };
    // Bezier control point: midpoint with a slight upward bend
    let mid_x = (from_x + to_x) / 2.0;
    let mid_y = (from_y + to_y) / 2.0 - 20.0;
    // Text is placed at (to_x + 4, to_y) — slightly offset from anchor
    let text_x = to_x + 4.0;
    let text_y = to_y + 4.0;
    Ok(Value::String(format!(
        r#"<g><path d="M {} {} Q {} {} {} {}" stroke="{}" stroke-width="1" stroke-dasharray="3,3" fill="none" /><circle cx="{}" cy="{}" r="2.5" fill="{}" /><text x="{}" y="{}" font-size="11" font-style="italic" fill="{}">{}</text></g>"#,
        fmt_num(from_x),
        fmt_num(from_y),
        fmt_num(mid_x),
        fmt_num(mid_y),
        fmt_num(to_x),
        fmt_num(to_y),
        escape_attr(line_color),
        fmt_num(to_x),
        fmt_num(to_y),
        escape_attr(line_color),
        fmt_num(text_x),
        fmt_num(text_y),
        escape_attr(text_color),
        escape_html_chars(&text)
    )))
}

// ── Level 2.6: Procedural backgrounds (Наряд №80) ────────────────────
//
// Three deterministic background generators that produce SVG fragments
// (NOT complete <svg> documents — they're meant to be embedded inside
// svg_canvas as the first child, behind other content):
//
//   svg_generate("flow",  intent, w, h) -> String
//   svg_generate("grid",  intent, w, h) -> String
//   svg_generate("noise", intent, w, h) -> String
//
// All three are PURE functions of (kind, intent, w, h). Same inputs →
// byte-identical output (verified by p80_svg_generate_noise contract).
// No `rand`, no system clock, no external state.
//
// Intent validation: `intent` MUST be one of the 5 known values
// (calm/tension/energy/authority/warmth) — the same set used by
// `color_palette`. Validated via `intent_to_hue`. Unknown intent → Err.
// This makes `intent` NOT an injection vector (it's a known-list enum,
// not free-form user text), so `svg_generate` is intentionally NOT in
// SVG_AUTO_ESCAPE_BUILTINS or SVG_NO_ESCAPE_BUILTINS. See Block 5 of
// the narazd report for the full reasoning.
//
// Color reuse: instead of inventing new palette tokens, the generators
// call `color_palette(intent, "light")` internally and derive their
// colors from the 5 existing tokens (paper/ink/accent/muted/rule).
// The HSL helpers `hex_to_hsl` / `interpolate_hsl` (from Наряд №79's
// chart_heatmap) are reused for hue-shifted variants.

/// Validate `intent` and return the corresponding DiagramStyle (light mode).
/// All three background generators share this — it gives them a consistent
/// color family derived from the same base_hue as `color_palette`.
fn background_style(intent: &str) -> Result<HashMap<String, Value>, String> {
    // Validate intent via intent_to_hue (same known-list as color_palette).
    // This is the security boundary: anything that's not one of the 5
    // known intents is rejected before any string reaches SVG output.
    if intent_to_hue(intent).is_none() {
        return Err(format!(
            "svg_generate: intent must be one of calm/tension/energy/authority/warmth (got {:?})",
            intent
        ));
    }
    let palette = builtin_color_palette(&[
        Value::String(intent.to_string()),
        Value::String("light".to_string()),
    ])?;
    extract_style(&palette)
}

/// `svg_generate(kind, intent, w, h) -> String`
///
/// `kind`   — one of "flow" / "grid" / "noise"
/// `intent` — one of calm/tension/energy/authority/warmth (validated)
/// `w`, `h` — canvas dimensions in SVG user units (positive floats)
///
/// Returns a `<g>` fragment suitable for embedding as the first child
/// of svg_canvas (i.e. rendered behind subsequent children). The
/// fragment does NOT include a background `<rect>` — callers who want
/// a solid fill behind the pattern should add their own `<rect>` first.
pub fn builtin_svg_generate(args: &[Value]) -> Result<Value, String> {
    let kind = expect_string_arg("svg_generate", args, 0)?;
    let intent = expect_string_arg("svg_generate", args, 1)?;
    let w = expect_float_arg("svg_generate", args, 2)?;
    let h = expect_float_arg("svg_generate", args, 3)?;
    if w <= 0.0 || h <= 0.0 {
        return Err(format!(
            "svg_generate: w and h must be positive (got w={}, h={})",
            w, h
        ));
    }
    // Cap dimensions to prevent pathological output sizes (a 100000×100000
    // noise canvas would emit ~250k circles). 10000 is generous (covers
    // all 5 presets in canvas_preset plus headroom) and matches the kind
    // of cap that svg_canvas itself implies via SVG viewport limits.
    if w > 10000.0 || h > 10000.0 {
        return Err(format!(
            "svg_generate: w and h must be ≤ 10000 (got w={}, h={})",
            w, h
        ));
    }
    let style = background_style(&intent)?;
    let body = match kind.as_str() {
        "flow" => generate_flow(&style, w, h, &intent),
        "grid" => generate_grid(&style, w, h),
        "noise" => generate_noise(&style, w, h, &intent),
        _ => {
            return Err(format!(
                "svg_generate: kind must be one of \"flow\" / \"grid\" / \"noise\" (got {:?})",
                kind
            ));
        }
    };
    // Wrap in <g> so the fragment is a single root element (composes
    // cleanly with svg_group / svg_canvas children semantics).
    Ok(Value::String(format!("<g>\n{}\n</g>", body)))
}

/// Block 2 — grid: regular grid of horizontal + vertical lines.
///
/// Step = max(w, h) / 12, clamped to [20, 100] to avoid both
/// over-dense grids on small canvases and invisible grids on huge ones.
/// Color = style.rule (the existing "subtle divider" token) at 0.35
/// opacity — visible but recessive.
///
/// Implementation: one `<path>` with many `M`/`L` commands is more
/// compact than N separate `<line>` elements, but harder to read in
/// SVG output. We emit separate `<line>` elements for clarity — the
/// size penalty is ~30 bytes/line, negligible for ≤60 lines.
fn generate_grid(style: &HashMap<String, Value>, w: f64, h: f64) -> String {
    let rule = style_token(style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    let step = (w.max(h) / 12.0).clamp(20.0, 100.0);
    let mut out = String::new();
    // Vertical lines: x = 0, step, 2*step, ... ≤ w
    let mut x = 0.0;
    while x <= w + 0.5 {
        out.push_str(&format!(
            r#"<line x1="{}" y1="0" x2="{}" y2="{}" stroke="{}" stroke-width="1" opacity="0.35" />"#,
            fmt_num(x),
            fmt_num(x),
            fmt_num(h),
            escape_attr(&rule)
        ));
        out.push('\n');
        x += step;
    }
    // Horizontal lines: y = 0, step, 2*step, ... ≤ h
    let mut y = 0.0;
    while y <= h + 0.5 {
        out.push_str(&format!(
            r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" opacity="0.35" />"#,
            fmt_num(y),
            fmt_num(w),
            fmt_num(y),
            escape_attr(&rule)
        ));
        out.push('\n');
        y += step;
    }
    out.trim_end_matches('\n').to_string()
}

/// Block 1 — flow: organic background of 3–5 cubic Bezier curves.
///
/// Each curve spans the full width of the canvas (left edge to right
/// edge) at a different y baseline. Control points are derived
/// deterministically from `intent` + curve index — same inputs always
/// produce the same control points (no `rand`, no system clock).
///
/// Color: each curve gets a hue-shifted variant of `style.muted`.
/// Hue shift = (curve_index - n/2) * 12° — so the middle curve uses
/// the base hue and the outer curves drift in opposite directions.
/// This produces a family of related tones without inventing new
/// palette tokens.
///
/// Stroke width: 1.5px, opacity 0.25 — recessive, doesn't compete
/// with foreground content.
fn generate_flow(style: &HashMap<String, Value>, w: f64, h: f64, intent: &str) -> String {
    let muted = style_token(style, "muted").unwrap_or_else(|_| "#888888".to_string());
    let base_hue = intent_to_hue(intent).unwrap_or(210.0);
    // Derive curve count from canvas aspect: wider canvases get more
    // curves (3–5 range, deterministic from h).
    // count = clamp(round(h / 100), 3, 5)
    let count = (h / 100.0).round().clamp(3.0, 5.0) as i64;
    let n = count as f64;
    let mut out = String::new();
    for i in 0..count {
        let fi = i as f64;
        // Baseline y for this curve: evenly spaced, inset from top/bottom
        let y_base = h * (fi + 1.0) / (n + 1.0);
        // Amplitude: ~15% of canvas height, varied per curve so they
        // don't look like parallel translations of each other.
        let amp = h * 0.15 * (1.0 + 0.3 * (fi - n / 2.0).sin());
        // Phase offset derived from intent + curve index — deterministic.
        // Combining base_hue (intent) with i ensures different intents
        // produce visibly different flows, and different curves within
        // the same flow are also offset.
        let phase = (base_hue + fi * 47.0).to_radians();
        // Cubic Bezier: M (0, y_base) C (w*0.33, y_base + amp*sin(phase)),
        //                           (w*0.67, y_base + amp*sin(phase + π/2)),
        //                           (w,     y_base + amp*sin(phase + π))
        let c1_y = y_base + amp * phase.sin();
        let c2_y = y_base + amp * (phase + std::f64::consts::FRAC_PI_2).sin();
        let end_y = y_base + amp * (phase + std::f64::consts::PI).sin();
        // Hue shift: middle curve = base_hue, outer curves drift by ±12°/step
        let hue_shift = (fi - (n - 1.0) / 2.0) * 12.0;
        let curve_hue = (base_hue + hue_shift + 360.0) % 360.0;
        // Reuse muted's saturation/lightness, swap hue.
        let (_h, s, l) = hex_to_hsl(&muted).unwrap_or((base_hue, 0.1, 0.5));
        let stroke = hsl_to_hex(curve_hue, s, l);
        out.push_str(&format!(
            r#"<path d="M 0 {} C {} {} {} {} {} {}" fill="none" stroke="{}" stroke-width="1.5" opacity="0.25" />"#,
            fmt_num(y_base),
            fmt_num(w * 0.33),
            fmt_num(c1_y),
            fmt_num(w * 0.67),
            fmt_num(c2_y),
            fmt_num(w),
            fmt_num(end_y),
            escape_attr(&stroke)
        ));
        out.push('\n');
    }
    out.trim_end_matches('\n').to_string()
}

/// Block 3 — noise: deterministic pseudo-random dot pattern.
///
/// Variant A (per Наряд №80 spec): a classic hash-based procedural
/// noise function, no external crate. The hash
///   `((x * 12.9898 + y * 78.233).sin() * 43758.5453).fract()`
/// is the canonical "1-line noise" popularized by GPU shaders — it
/// produces acceptable visual noise in a few lines of code, with zero
/// new dependencies. Limitation: not as smooth as Perlin/Simplex
/// (visible grid artifacts at high densities), but adequate for
/// background texture. If visual quality is insufficient, variant B
/// (the `noise` crate) can be added in a separate narazd.
///
/// Density: one dot per ~12×12 cell — so a 600×400 canvas produces
/// ~50×33 ≈ 1650 dots. Each dot is a `<circle>` with radius derived
/// from the hash (0.5–2.0px) and opacity 0.05–0.25.
///
/// Color: dots are interpolated in HSL space between `style.paper`
/// (low hash values) and `style.accent` (high hash values), so the
/// pattern picks up the intent's color family. Reuses `hex_to_hsl`
/// and `interpolate_hsl` from Наряд №79 — no new color code.
///
/// Determinism: same (kind, intent, w, h) always produces byte-identical
/// output. Verified by p80_svg_generate_noise contract: two consecutive
/// calls to svg_generate("noise", "energy", 600, 400) MUST produce the
/// same string. This is a direct consequence of using only arithmetic
/// on the input parameters (no thread-local RNG, no clock, no I/O).
fn generate_noise(style: &HashMap<String, Value>, w: f64, h: f64, intent: &str) -> String {
    let paper = style_token(style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(style, "accent").unwrap_or_else(|_| "#000000".to_string());
    let c1 = hex_to_hsl(&paper).unwrap_or((0.0, 0.0, 1.0));
    let c2 = hex_to_hsl(&accent).unwrap_or((0.0, 0.0, 0.0));
    // Seed the hash with the intent's base_hue so different intents
    // produce different patterns (not just different colors).
    let seed = intent_to_hue(intent).unwrap_or(210.0) / 360.0;
    let cell = 12.0;
    let cols = (w / cell).round() as i64;
    let rows = (h / cell).round() as i64;
    let mut out = String::new();
    for row in 0..rows {
        for col in 0..cols {
            // Cell center in SVG coords
            let cx = (col as f64 + 0.5) * cell;
            let cy = (row as f64 + 0.5) * cell;
            // Hash: combine cell coords with intent seed. The classical
            // 1-line noise function. sin() is deterministic and IEEE-754
            // portable across platforms (same input → same bits).
            let h_input = (col as f64) * 12.9898 + (row as f64) * 78.233 + seed * 311.0;
            let v = ((h_input.sin() * 43758.5453).fract() + 1.0) * 0.5; // → [0, 1]
                                                                        // Radius: 0.5–2.0px (proportional to v so dots have visible size variation)
            let r = 0.5 + v * 1.5;
            // Opacity: 0.05–0.25 (recessive, doesn't dominate foreground)
            let opacity = 0.05 + v * 0.20;
            // Color: interpolate paper→accent in HSL (reuses Н79 helpers)
            let (hh, hs, hl) = interpolate_hsl(c1, c2, v);
            let fill = hsl_to_hex(hh, hs, hl);
            out.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" opacity="{}" />"#,
                fmt_num(cx),
                fmt_num(cy),
                fmt_num(r),
                escape_attr(&fill),
                fmt_num(opacity)
            ));
            out.push('\n');
        }
    }
    out.trim_end_matches('\n').to_string()
}

// ── Level 2.7: Canvas presets (Наряд №80 Block 4) ────────────────────
//
// Named constants for common SVG canvas sizes, borrowed from
// `diagram-design` skill conventions. Replaces magic-number pairs
// at call sites:
//   svg_canvas(1280, 720, "0 0 1280 720", children)
// becomes:
//   svg_canvas_preset("slide_16x9", "0 0 1280 720", children)
//
// Backward compatibility: the existing `svg_canvas` signature is
// UNCHANGED. `svg_canvas_preset` is a new wrapper that looks up the
// preset, calls `svg_canvas` with the resolved (w, h), and passes
// through viewbox + children verbatim.

/// Resolve a preset name to (width, height). Returns None for unknown.
///
/// Sizes are in SVG user units (which correspond 1:1 to pixels at 96dpi
/// in browser rendering). All five presets are common print/screen sizes:
///   - doc_inline:         960×600    — inline doc figure
///   - slide_16x9:         1280×720   — 16:9 slide
///   - social_og:          1200×632   — Open Graph image (Facebook/X)
///   - print_a4_landscape: 1122×793   — A4 landscape @ 96dpi
///   - print_a4_portrait:  793×1122   — A4 portrait @ 96dpi
pub(crate) fn canvas_preset(name: &str) -> Option<(f64, f64)> {
    match name {
        "doc_inline" => Some((960.0, 600.0)),
        "slide_16x9" => Some((1280.0, 720.0)),
        "social_og" => Some((1200.0, 632.0)),
        "print_a4_landscape" => Some((1122.0, 793.0)),
        "print_a4_portrait" => Some((793.0, 1122.0)),
        _ => None,
    }
}

/// `svg_canvas_preset(preset_name, viewbox, children: List<String>) -> String`
///
/// Same semantics as `svg_canvas` but resolves (w, h) from a named
/// preset. Unknown preset → Err with the list of available names
/// (no silent fallthrough).
pub fn builtin_svg_canvas_preset(args: &[Value]) -> Result<Value, String> {
    let preset_name = expect_string_arg("svg_canvas_preset", args, 0)?;
    let (w, h) = canvas_preset(&preset_name).ok_or_else(|| {
        format!(
            "svg_canvas_preset: unknown preset {:?}. Available: doc_inline, slide_16x9, social_og, print_a4_landscape, print_a4_portrait",
            preset_name
        )
    })?;
    // Delegate to builtin_svg_canvas — same validation of viewbox + children.
    builtin_svg_canvas(&[
        Value::Float(w),
        Value::Float(h),
        args.get(1).cloned().unwrap_or(Value::Unit),
        args.get(2).cloned().unwrap_or(Value::Unit),
    ])
}

// ── Наряд №81: Diagrams (hierarchies & flows) ────────────────────────
//
// Four high-level diagram builtins built on top of the existing SVG
// primitives (svg_rect, svg_text, svg_path, svg_group, svg_canvas).
//
//   diagram_tree(data, style)        — recursive tree layout
//   diagram_org_chart(data, style)   — same algorithm + title field
//   diagram_flowchart(data, style)   — layered DAG via topological sort
//   diagram_layers(data, style)      — horizontal stripes (simplest)
//
// All four return a complete <svg>...</svg> document. All user-supplied
// text (label / title / description) is XML-escaped at runtime via
// escape_html_chars — defense-in-depth invariant identical to chart_*.
// AST-level lint (semantic.rs → SVG_AUTO_ESCAPE_BUILTINS) scans for
// <script> payloads in label literals.
//
// Common canvas constants (chosen to match chart_bar's geometry so
// diagrams compose with other chart_* outputs in a grid layout):
//   canvas: 600 × 400
//   padding: 40px on all sides
//
// ── Block 1: draw_connector (internal helper) ──────────────────────
//
// Line from (x1,y1) to (x2,y2) plus a triangular arrowhead at (x2,y2)
// rotated by atan2(dy, dx) — the angle of the line itself. Used by
// diagram_tree / diagram_org_chart (parent → child) and diagram_flowchart
// (from → to). NOT a public builtin (internal helper, per spec).
//
// Arrowhead geometry:
//   - 8px long, 6px wide at the base (isosceles triangle)
//   - tip at (x2, y2); base center at (x2 - 8·cos θ, y2 - 8·sin θ)
//   - base corners = base center ± 3·(−sin θ, cos θ) (perpendicular)
//
// Color: style.rule for visual consistency with the existing axis
// divider lines in chart_bar (which also use `rule`). The line stroke
// width is 1.5 — slightly heavier than the grid (1px @ 0.35 opacity)
// to read as a deliberate connector, not background decoration.
fn draw_connector(x1: f64, y1: f64, x2: f64, y2: f64, style: &HashMap<String, Value>) -> String {
    let color = style_token(style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    let dx = x2 - x1;
    let dy = y2 - y1;
    let angle = dy.atan2(dx);
    // Arrowhead dimensions
    let ah_len = 8.0_f64;
    let ah_half_w = 3.0_f64;
    // Pull the line back by ah_len so the line tip doesn't poke through
    // the arrowhead tip (visual cleanliness).
    let line_end_x = x2 - ah_len * angle.cos();
    let line_end_y = y2 - ah_len * angle.sin();
    // Arrowhead base center (8px back from tip along the line direction)
    let base_x = x2 - ah_len * angle.cos();
    let base_y = y2 - ah_len * angle.sin();
    // Perpendicular offset (90° rotation): (-sin θ, cos θ)
    let perp_x = -angle.sin();
    let perp_y = angle.cos();
    let left_x = base_x + perp_x * ah_half_w;
    let left_y = base_y + perp_y * ah_half_w;
    let right_x = base_x - perp_x * ah_half_w;
    let right_y = base_y - perp_y * ah_half_w;
    // Line + closed triangular path (fill=color, no stroke on arrowhead)
    format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" />"#,
        fmt_num(x1),
        fmt_num(y1),
        fmt_num(line_end_x),
        fmt_num(line_end_y),
        escape_attr(&color)
    ) + &format!(
        r#"<path d="M {} {} L {} {} L {} {} Z" fill="{}" stroke="none" />"#,
        fmt_num(x2),
        fmt_num(y2),
        fmt_num(left_x),
        fmt_num(left_y),
        fmt_num(right_x),
        fmt_num(right_y),
        escape_attr(&color)
    )
}

// ── Block 2/3 shared: recursive tree node ──────────────────────────
//
// Internal representation of a tree node, extracted from a user Struct.
// `title` is None for diagram_tree, Some for diagram_org_chart. Both
// use the SAME layout algorithm — diagram_org_chart only overrides the
// per-node render to emit a second <text> line when title is present.
struct TreeNode {
    label: String,
    title: Option<String>,
    children: Vec<TreeNode>,
}

/// Extract a TreeNode from a Value::Struct. Recurses into `children`.
/// `title` field is optional — None if missing/Unit (diagram_tree case).
/// Enforces the depth + total node count limits at extraction time so
/// the layout function can assume well-bounded input.
fn extract_tree_node(
    value: &Value,
    path: &str,
    allow_title: bool,
    depth: usize,
    node_count: &mut usize,
) -> Result<TreeNode, String> {
    const MAX_DEPTH: usize = 6;
    const MAX_NODES: usize = 40;
    if depth > MAX_DEPTH {
        return Err(format!(
            "diagram_tree: depth exceeds maximum of {} (path: {})",
            MAX_DEPTH, path
        ));
    }
    *node_count += 1;
    if *node_count > MAX_NODES {
        return Err(format!(
            "diagram_tree: total node count exceeds maximum of {} (path: {})",
            MAX_NODES, path
        ));
    }
    let fields = match value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_tree: node at {} must be Struct, got {}",
                path,
                other.type_name()
            ));
        }
    };
    let label = struct_string_field("diagram_tree node", fields, "label")?;
    let title = if allow_title {
        struct_opt_string_field(fields, "title")
    } else {
        None
    };
    // children is required (empty list = leaf). Missing field is an error.
    let children_val = fields
        .get("children")
        .ok_or_else(|| format!("diagram_tree: node at {} missing 'children' field", path))?;
    // Handle Value::Unit as empty list (defensive — a caller might pass
    // Unit instead of [] for leaf nodes). Use a static empty slice to
    // avoid lifetime issues with temporary Vec.
    let empty_list: Vec<Value> = Vec::new();
    let child_list: &[Value] = match children_val {
        Value::List(items) => items,
        Value::Unit => &empty_list,
        other => {
            return Err(format!(
                "diagram_tree: 'children' at {} must be List, got {}",
                path,
                other.type_name()
            ));
        }
    };
    let mut children = Vec::with_capacity(child_list.len());
    for (i, child) in child_list.iter().enumerate() {
        let child_path = format!("{}.children[{}]", path, i);
        children.push(extract_tree_node(
            child,
            &child_path,
            allow_title,
            depth + 1,
            node_count,
        )?);
    }
    Ok(TreeNode {
        label,
        title,
        children,
    })
}

/// Layout result for a single node: center-x, top-y (top edge of the box).
struct LaidOutNode {
    /// x-center of the box.
    cx: f64,
    /// y-top of the box.
    y: f64,
    /// Subtree width (for parent centering).
    subtree_w: f64,
    /// Node box dimensions (constant across all nodes — kept here for
    /// readability, the layout function uses the constants directly).
    /// Children (laid out recursively).
    children: Vec<LaidOutNode>,
    /// Reference to the source tree node (for rendering label/title).
    /// Stored as label + title snapshot to avoid lifetime entanglement.
    label: String,
    title: Option<String>,
}

/// Standard node box dimensions for tree/org-chart. Width=120, height=40
/// (or 56 if title is present — second line of text needs more room).
const TREE_NODE_W: f64 = 120.0;
const TREE_NODE_H_NO_TITLE: f64 = 40.0;
const TREE_NODE_H_WITH_TITLE: f64 = 56.0;
/// Horizontal gap between sibling subtrees.
const TREE_SIBLING_GAP: f64 = 24.0;
/// Vertical gap between levels (parent box bottom → child box top).
const TREE_LEVEL_GAP: f64 = 50.0;
/// Top padding for the first level.
const TREE_TOP_PAD: f64 = 30.0;

/// Recursive layout. Returns a LaidOutNode with cx relative to the
/// subtree's left edge (0.0). The caller translates the whole tree to
/// its final position by adding an x-offset.
///
/// Algorithm: classic separate layout. For a leaf, subtree_w = node_w.
/// For an internal node, subtree_w = sum(child subtree widths) + gaps.
/// Parent cx = midpoint between leftmost and rightmost child cx.
fn layout_tree(node: &TreeNode, depth: usize) -> LaidOutNode {
    let node_h = if node.title.is_some() {
        TREE_NODE_H_WITH_TITLE
    } else {
        TREE_NODE_H_NO_TITLE
    };
    let y = TREE_TOP_PAD + (depth as f64) * (node_h + TREE_LEVEL_GAP);
    if node.children.is_empty() {
        // Leaf: subtree width = own width, cx = center of own box
        return LaidOutNode {
            cx: TREE_NODE_W / 2.0,
            y,
            subtree_w: TREE_NODE_W,
            children: Vec::new(),
            label: node.label.clone(),
            title: node.title.clone(),
        };
    }
    // Recurse on children, accumulating x-offset
    let mut laid_children: Vec<LaidOutNode> = Vec::with_capacity(node.children.len());
    let mut x_offset = 0.0_f64;
    for (i, child) in node.children.iter().enumerate() {
        let mut lc = layout_tree(child, depth + 1);
        // Translate child by current x_offset
        lc.cx += x_offset;
        // Also translate all descendants (their cx is relative to subtree left,
        // but we keep them relative for now — we translate at render time using
        // a transform group instead of mutating deeply).
        // Actually, simpler: store absolute cx. We translate descendants below
        // by walking the laid-out tree once more.
        laid_children.push(lc);
        x_offset += laid_children[i].subtree_w + TREE_SIBLING_GAP;
    }
    // Remove trailing gap from total width
    let total_w = x_offset - TREE_SIBLING_GAP;
    // Parent cx = midpoint between first and last child centers.
    // SAFETY: we returned early for the leaf case (empty children),
    // so laid_children is non-empty here. We use if-let with a fallback
    // (which is unreachable but satisfies clippy::expect_used — the
    // project denies both unwrap_used and expect_used in non-test code).
    let parent_cx = match (laid_children.first(), laid_children.last()) {
        (Some(first), Some(last)) => (first.cx + last.cx) / 2.0,
        // Unreachable: laid_children is non-empty here (we returned early
        // for leaves above). The fallback is a defensive default that
        // would only trigger if the invariant above is broken.
        _ => TREE_NODE_W / 2.0,
    };
    LaidOutNode {
        cx: parent_cx,
        y,
        subtree_w: total_w.max(TREE_NODE_W),
        children: laid_children,
        label: node.label.clone(),
        title: node.title.clone(),
    }
}

/// Render a laid-out tree node + its children + connectors. Returns
/// a list of SVG fragment strings (each is a child element). The caller
/// wraps them in a <g transform="translate(x_offset, 0)"> for the
/// top-level tree, OR includes them directly (already absolute coords).
///
/// `is_org_chart` controls the per-node render: if true and title is
/// present, emit a second <text> line below the label.
fn render_tree_node(
    node: &LaidOutNode,
    style: &HashMap<String, Value>,
    is_org_chart: bool,
    parts: &mut Vec<String>,
) {
    let ink = style_token(style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    let node_h = if node.title.is_some() && is_org_chart {
        TREE_NODE_H_WITH_TITLE
    } else {
        TREE_NODE_H_NO_TITLE
    };
    let box_x = node.cx - TREE_NODE_W / 2.0;
    // Node box — paper fill, rule border (so it reads as a container, not a button)
    parts.push(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
        fmt_num(box_x),
        fmt_num(node.y),
        fmt_num(TREE_NODE_W),
        fmt_num(node_h),
        escape_attr(&paper),
        escape_attr(&rule)
    ));
    // Label — centered horizontally, baseline at vertical midpoint
    let label_y = if node.title.is_some() && is_org_chart {
        node.y + 22.0
    } else {
        node.y + node_h / 2.0 + 4.0
    };
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="13" fill="{}" text-anchor="middle">{}</text>"#,
        fmt_num(node.cx),
        fmt_num(label_y),
        escape_attr(&ink),
        escape_html_chars(&node.label)
    ));
    // Title (org chart only) — second line, muted color, smaller font
    if is_org_chart {
        if let Some(title) = &node.title {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(node.cx),
                fmt_num(node.y + 40.0),
                escape_attr(&muted),
                escape_html_chars(title)
            ));
        }
    }
    // Connectors to children + recurse
    let parent_bottom_y = node.y + node_h;
    for child in &node.children {
        // Connector starts at parent bottom center, ends at child top center
        let connector = draw_connector(node.cx, parent_bottom_y, child.cx, child.y, style);
        parts.push(connector);
        render_tree_node(child, style, is_org_chart, parts);
    }
    // Unused imports guard — accent is here for future use (e.g. highlight
    // root node with accent border). Currently no-op.
    let _ = &accent;
}

/// Diagram canvas: 600 × 400 (matches chart_bar). Tree layout may exceed
/// this horizontally for wide trees — we scale the viewBox to fit the
/// actual laid-out tree width, so wide trees are rendered fully (no
/// clipping). Height is fixed (depth ≤ 6 → max ~6 * 90px = 540px).
const DIAGRAM_CANVAS_W: f64 = 600.0;
const DIAGRAM_CANVAS_H: f64 = 400.0;

/// `diagram_tree(data, style) -> String`
///
/// Renders a recursive tree. `data` is `Struct { label, children }` —
/// children is the same shape recursively. Empty children list = leaf.
///
/// Limits: depth ≤ 6, total nodes ≤ 40 (returns Err otherwise).
pub fn builtin_diagram_tree(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let mut node_count = 0usize;
    let root = extract_tree_node(&data_value, "root", false, 0, &mut node_count)?;
    let laid = layout_tree(&root, 0);
    // Compute total width: subtree_w of root. Center the tree horizontally
    // in the canvas with at least 20px left padding.
    let tree_w = laid.subtree_w.max(TREE_NODE_W);
    let canvas_w = DIAGRAM_CANVAS_W.max(tree_w + 40.0);
    // x-offset to center tree in canvas
    let x_offset = (canvas_w - tree_w) / 2.0;
    // Translate root cx (which is relative to subtree left) to absolute
    let mut parts: Vec<String> = Vec::new();
    // Render into a translate group so all relative coords become absolute
    let mut absolute_node = laid;
    absolute_node.cx += x_offset;
    translate_subtree(&mut absolute_node, x_offset);
    // Background
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        escape_attr(&paper)
    ));
    render_tree_node(&absolute_node, &style, false, &mut parts);
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        body
    )))
}

/// Walk a LaidOutNode and add `dx` to every cx (in place). Used to
/// convert relative-to-subtree coords to absolute canvas coords.
fn translate_subtree(node: &mut LaidOutNode, dx: f64) {
    node.cx += dx;
    for child in &mut node.children {
        translate_subtree(child, dx);
    }
}

/// `diagram_org_chart(data, style) -> String`
///
/// Thin wrapper over diagram_tree's layout algorithm. The ONLY
/// difference is the per-node render: when a `title` field is present,
/// the node box is taller and a second <text> line is emitted. The
/// layout algorithm (subtree width, parent centering, depth spacing)
/// is identical — we call the same `extract_tree_node` with
/// `allow_title=true`, the same `layout_tree`, and the same
/// `render_tree_node` with `is_org_chart=true`.
///
/// `data` is `Struct { label, title?, children }`.
pub fn builtin_diagram_org_chart(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let mut node_count = 0usize;
    let root = extract_tree_node(&data_value, "root", true, 0, &mut node_count)?;
    let laid = layout_tree(&root, 0);
    let tree_w = laid.subtree_w.max(TREE_NODE_W);
    let canvas_w = DIAGRAM_CANVAS_W.max(tree_w + 40.0);
    let x_offset = (canvas_w - tree_w) / 2.0;
    let mut absolute_node = laid;
    absolute_node.cx += x_offset;
    translate_subtree(&mut absolute_node, x_offset);
    let mut parts: Vec<String> = Vec::new();
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        escape_attr(&paper)
    ));
    render_tree_node(&absolute_node, &style, true, &mut parts);
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        body
    )))
}

// ── Block 4: diagram_flowchart ─────────────────────────────────────
//
// MVP layout: topological sort into horizontal layers (BFS from nodes
// with no incoming edges). All nodes in layer N share a Y coordinate;
// nodes within a layer are spread evenly across the canvas width.
// Edges are drawn with draw_connector; optional edge label placed at
// the midpoint of the line, offset slightly above to avoid overlap.
//
// Cycle handling: if topological sort cannot drain all nodes, the
// remaining nodes form a cycle. We return a structured Err mentioning
// the offending node IDs (e.g. "flowchart contains a cycle: A→B→C→A").
//
// Limits: nodes.len() ≤ 25 (otherwise canvas becomes unreadable).
const FLOWCHART_MAX_NODES: usize = 25;
const FLOWCHART_NODE_W: f64 = 110.0;
const FLOWCHART_NODE_H: f64 = 44.0;

/// Internal flowchart node representation.
struct FlowNode {
    id: String,
    label: String,
}
struct FlowEdge {
    from: String,
    to: String,
    label: Option<String>,
}

/// Extract nodes + edges from the input Struct. Validates that all
/// edge endpoints reference existing node IDs.
fn extract_flowchart(data_value: &Value) -> Result<(Vec<FlowNode>, Vec<FlowEdge>), String> {
    let fields = match data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_flowchart: data must be Struct {{nodes, edges}}, got {}",
                other.type_name()
            ));
        }
    };
    let nodes_val = fields
        .get("nodes")
        .ok_or_else(|| "diagram_flowchart: missing 'nodes' field".to_string())?;
    let edges_val = fields
        .get("edges")
        .ok_or_else(|| "diagram_flowchart: missing 'edges' field".to_string())?;
    let nodes_list = match nodes_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_flowchart: 'nodes' must be List, got {}",
                other.type_name()
            ));
        }
    };
    let edges_list = match edges_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_flowchart: 'edges' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if nodes_list.is_empty() {
        return Err("diagram_flowchart: nodes list must not be empty".to_string());
    }
    if nodes_list.len() > FLOWCHART_MAX_NODES {
        return Err(format!(
            "diagram_flowchart: too many nodes ({}), maximum is {}",
            nodes_list.len(),
            FLOWCHART_MAX_NODES
        ));
    }
    let mut nodes: Vec<FlowNode> = Vec::with_capacity(nodes_list.len());
    let mut node_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in nodes_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_flowchart: nodes[{}] must be Struct {{id, label}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let id = struct_string_field("diagram_flowchart node", f, "id")?;
        let label = struct_string_field("diagram_flowchart node", f, "label")?;
        if !node_ids.insert(id.clone()) {
            return Err(format!(
                "diagram_flowchart: duplicate node id {:?} at nodes[{}]",
                id, i
            ));
        }
        nodes.push(FlowNode { id, label });
    }
    let mut edges: Vec<FlowEdge> = Vec::with_capacity(edges_list.len());
    for (i, item) in edges_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_flowchart: edges[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_flowchart edge", f, "from")?;
        let to = struct_string_field("diagram_flowchart edge", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        if !node_ids.contains(&from) {
            return Err(format!(
                "diagram_flowchart: edges[{}].from references unknown node {:?}",
                i, from
            ));
        }
        if !node_ids.contains(&to) {
            return Err(format!(
                "diagram_flowchart: edges[{}].to references unknown node {:?}",
                i, to
            ));
        }
        edges.push(FlowEdge { from, to, label });
    }
    Ok((nodes, edges))
}

/// Compute in-degree for each node, then BFS from nodes with in-degree 0.
/// Returns (layers, ordered_node_positions) on success, or Err with the
/// cycle node IDs on cycle detection.
///
/// The "layers" are 0-indexed: layer 0 = roots (no incoming edges),
/// layer N = nodes whose all predecessors are in layers < N. A node
/// joins layer max(predecessor layers) + 1 — this is the "longest path
/// from a root" layering, which tends to produce wider, shallower
/// diagrams than naive BFS layering and avoids unnecessarily deep
/// layouts for graphs with merges.
///
/// **Наряд №84 Block 4 — generalized signature.** Previously this
/// function took `&[FlowNode]` + `&[FlowEdge]` (typed structs). The
/// narazd №84 spec calls for generalizing it so that `diagram_flowchart`
/// (Н81), `diagram_high_level`, and `diagram_architecture` (both Н84)
/// can all share one implementation. The new signature takes plain
/// `&[String]` for node IDs and `&[(String, String)]` for edge pairs,
/// with no payload (label/icon) — those are looked up separately by
/// callers via the position map this function returns.
///
/// Behavior is unchanged for the existing `diagram_flowchart` caller:
/// same longest-path layering, same cycle error text, same self-loop
/// rejection. The p81_diagram_flowchart contract continues to pass
/// without modification (verified by the regression contract
/// p84_topological_layers_regression.mlog).
fn topological_layers(
    node_ids: &[String],
    edges: &[(String, String)],
) -> Result<Vec<Vec<String>>, String> {
    // Index nodes by id for fast lookup
    let id_to_idx: std::collections::HashMap<&String, usize> =
        node_ids.iter().enumerate().map(|(i, n)| (n, i)).collect();
    let n = node_ids.len();
    // Build adjacency: predecessors[idx] = list of predecessor idxs
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (from, to) in edges {
        let from_idx = *id_to_idx.get(from).ok_or_else(|| {
            format!(
                "diagram_flowchart: internal error — edge.from {:?} not in index",
                from
            )
        })?;
        let to_idx = *id_to_idx.get(to).ok_or_else(|| {
            format!(
                "diagram_flowchart: internal error — edge.to {:?} not in index",
                to
            )
        })?;
        if from_idx == to_idx {
            // Self-loop — that's a trivial cycle.
            return Err(format!(
                "flowchart contains a cycle: {}→{} (self-loop)",
                from, to
            ));
        }
        successors[from_idx].push(to_idx);
        predecessors[to_idx].push(from_idx);
    }
    // Longest-path layering:
    //   layer[idx] = 0 if no predecessors
    //   layer[idx] = 1 + max(layer[p] for p in predecessors) otherwise
    // We compute this by processing nodes in topological order. Use
    // Kahn's algorithm to get the order, then assign layers.
    let mut in_degree: Vec<usize> = predecessors.iter().map(|p| p.len()).collect();
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for (idx, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            queue.push_back(idx);
        }
    }
    let mut order: Vec<usize> = Vec::with_capacity(n);
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &succ in &successors[idx] {
            in_degree[succ] -= 1;
            if in_degree[succ] == 0 {
                queue.push_back(succ);
            }
        }
    }
    if order.len() < n {
        // Cycle detected — find the nodes still with in_degree > 0
        let mut cycle_nodes: Vec<String> = Vec::new();
        for (idx, &deg) in in_degree.iter().enumerate() {
            if deg > 0 {
                cycle_nodes.push(node_ids[idx].clone());
            }
        }
        return Err(format!(
            "flowchart contains a cycle involving nodes: {}",
            cycle_nodes.join(", ")
        ));
    }
    // Assign layers in topological order
    let mut layer: Vec<usize> = vec![0; n];
    for &idx in &order {
        let max_pred_layer = predecessors[idx]
            .iter()
            .map(|&p| layer[p])
            .max()
            .unwrap_or(0);
        layer[idx] = if predecessors[idx].is_empty() {
            0
        } else {
            max_pred_layer + 1
        };
    }
    // Group by layer
    let max_layer = *layer.iter().max().unwrap_or(&0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_layer + 1];
    for (idx, &l) in layer.iter().enumerate() {
        layers[l].push(node_ids[idx].clone());
    }
    Ok(layers)
}

/// **Наряд №84 Block 2/5 — BFS layering that tolerates cycles.**
///
/// Used by `diagram_state` and `diagram_data_flow`, where the graph is
/// expected to contain cycles (state machines cycle, data flows have
/// feedback loops). Unlike `topological_layers`, this function NEVER
/// returns an error on a cycle — it lays out the graph by BFS distance
/// from a chosen `root` node, treating edges as UNDIRECTED for layering
/// purposes (a cycle A→B→A places both A and B at distance ≤1 from any
/// chosen root, which is what we want for visualization).
///
/// Self-loops (A→A) are silently ignored — they don't affect layering
/// (a node is always at distance 0 from itself), and they're valid
/// transitions in state machines per the Н84 spec.
///
/// `root` MUST be a member of `node_ids` (callers validate this before
/// calling). If a node is not reachable from `root` via undirected BFS
/// (disconnected component), it's placed at layer `max_reachable_layer + 1`
/// so disconnected subgraphs appear at the bottom of the diagram rather
/// than being silently dropped.
///
/// Returns a non-empty Vec<Vec<String>> (at least one layer containing
/// `root`) — never returns Err.
fn bfs_layers_with_cycles(
    node_ids: &[String],
    edges: &[(String, String)],
    root: &str,
) -> Vec<Vec<String>> {
    let id_to_idx: std::collections::HashMap<&String, usize> =
        node_ids.iter().enumerate().map(|(i, n)| (n, i)).collect();
    let n = node_ids.len();
    // Build UNDIRECTED adjacency (treat each directed edge as bidirectional
    // for layering purposes — this is what makes cycles lay out sanely).
    let mut adj: Vec<std::collections::HashSet<usize>> = vec![std::collections::HashSet::new(); n];
    for (from, to) in edges {
        if let (Some(&i), Some(&j)) = (id_to_idx.get(from), id_to_idx.get(to)) {
            if i != j {
                // Skip self-loops — they don't change reachability
                adj[i].insert(j);
                adj[j].insert(i);
            }
        }
    }
    // BFS from root, recording distance (layer) for each visited node.
    // layer[i] == -1 means "not yet visited".
    let mut layer: Vec<i32> = vec![-1; n];
    let root_idx = id_to_idx.get(&root.to_string()).copied().unwrap_or(0);
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    queue.push_back(root_idx);
    layer[root_idx] = 0;
    while let Some(idx) = queue.pop_front() {
        // Iterate over a snapshot to satisfy borrow checker
        let neighbors: Vec<usize> = adj[idx].iter().copied().collect();
        for nbr in neighbors {
            if layer[nbr] == -1 {
                layer[nbr] = layer[idx] + 1;
                queue.push_back(nbr);
            }
        }
    }
    // Unreachable nodes (layer == -1) are placed at max_reachable + 1.
    // They get their own bottom layer — preserves them in the output
    // without polluting the BFS-derived layers.
    let max_reachable = layer
        .iter()
        .filter(|&&l| l >= 0)
        .max()
        .copied()
        .unwrap_or(0);
    for l in layer.iter_mut() {
        if *l == -1 {
            *l = max_reachable + 1;
        }
    }
    // Group by layer
    let max_layer = *layer.iter().max().unwrap_or(&0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); (max_layer + 1) as usize];
    for (i, &l) in layer.iter().enumerate() {
        layers[l as usize].push(node_ids[i].clone());
    }
    layers
}

/// `diagram_flowchart(data, style) -> String`
///
/// Renders a flowchart with layered topological layout. Nodes in the
/// same layer share a Y coordinate; layers are stacked vertically.
/// Edges use draw_connector; optional edge labels are placed at the
/// midpoint, offset slightly above the line.
///
/// `data` is `Struct { nodes: List<{id, label}>, edges: List<{from, to, label?}> }`.
///
/// Returns Err with "flowchart contains a cycle: ..." if the graph
/// has a cycle (topological sort cannot complete).
pub fn builtin_diagram_flowchart(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_flowchart(&data_value)?;
    // Н84 Block 4: topological_layers now takes plain &[String] + &[(String,String)].
    // Derive the inputs from the parsed FlowNode/FlowEdge structs — behavior
    // is unchanged for flowchart (regression-checked by p84_topological_layers_regression.mlog).
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    let layers = topological_layers(&node_ids, &edge_pairs)?;
    // Layout: each layer is one horizontal row. Within a row, nodes
    // are spread evenly across the canvas width (with side padding).
    let n_layers = layers.len();
    // Y position per layer: distribute vertically across canvas height
    // with top/bottom padding. Layer 0 at top.
    let layer_h = (DIAGRAM_CANVAS_H - 80.0) / (n_layers as f64).max(1.0);
    let mut id_to_pos: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::new();
    let mut id_to_label: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for n in &nodes {
        id_to_label.insert(n.id.clone(), n.label.as_str());
    }
    for (layer_idx, layer_nodes) in layers.iter().enumerate() {
        let count = layer_nodes.len();
        let y_center = 40.0 + (layer_idx as f64 + 0.5) * layer_h;
        // Spread nodes: if 1 node, center; else distribute evenly.
        let total_w = DIAGRAM_CANVAS_W - 80.0; // 40px padding each side
        let step = if count > 1 {
            total_w / (count as f64 - 1.0)
        } else {
            0.0
        };
        let start_x = if count > 1 {
            40.0
        } else {
            DIAGRAM_CANVAS_W / 2.0
        };
        for (i, id) in layer_nodes.iter().enumerate() {
            let x_center = start_x + (i as f64) * step;
            id_to_pos.insert(id.clone(), (x_center, y_center));
        }
    }
    let mut parts: Vec<String> = Vec::new();
    // Background
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(DIAGRAM_CANVAS_W),
        fmt_num(DIAGRAM_CANVAS_H),
        escape_attr(&paper)
    ));
    // Edges first (so node boxes render on top of any line that clips)
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    for edge in &edges {
        // Look up positions — both endpoints must be in id_to_pos
        // (we built the map from layers, which contains every node).
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_flowchart: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_flowchart: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        // Trim endpoints so connectors start/end at the box edges, not centers
        let (sx, sy) = box_edge_point(
            from_x,
            from_y,
            to_x,
            to_y,
            FLOWCHART_NODE_W,
            FLOWCHART_NODE_H,
        );
        let (ex, ey) = box_edge_point(
            to_x,
            to_y,
            from_x,
            from_y,
            FLOWCHART_NODE_W,
            FLOWCHART_NODE_H,
        );
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        // Optional edge label at midpoint, offset above the line
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // Nodes
    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    for (id, (cx, cy)) in &id_to_pos {
        let label = id_to_label.get(id).copied().unwrap_or("");
        let box_x = cx - FLOWCHART_NODE_W / 2.0;
        let box_y = cy - FLOWCHART_NODE_H / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(FLOWCHART_NODE_W),
            fmt_num(FLOWCHART_NODE_H),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(*cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(DIAGRAM_CANVAS_W),
        fmt_num(DIAGRAM_CANVAS_H),
        fmt_num(DIAGRAM_CANVAS_W),
        fmt_num(DIAGRAM_CANVAS_H),
        body
    )))
}

/// Compute the point where the line from (cx,cy) to (tx,ty) intersects
/// the boundary of a box centered at (cx,cy) with width w and height h.
/// Used to make connectors touch the box edge instead of the center.
fn box_edge_point(cx: f64, cy: f64, tx: f64, ty: f64, w: f64, h: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let half_w = w / 2.0;
    let half_h = h / 2.0;
    // Scale factors to reach each edge
    let sx = if dx != 0.0 {
        half_w / dx.abs()
    } else {
        f64::INFINITY
    };
    let sy = if dy != 0.0 {
        half_h / dy.abs()
    } else {
        f64::INFINITY
    };
    let s = sx.min(sy);
    (cx + dx * s, cy + dy * s)
}

// ── Block 5: diagram_layers ─────────────────────────────────────────
//
// Simplest of the four — no draw_connector, no tree algorithm. Just
// horizontal stripes of equal height stacked top-to-bottom.
//
// Layout:
//   - canvas: 600 × 400
//   - N layers, each height = canvas_h / N
//   - label left-aligned with 16px left padding, vertically centered
//   - optional description right-aligned with 16px right padding,
//     smaller font, muted color
//
// Limits: data.len() ≤ 10 (otherwise stripes become too narrow).

/// `diagram_layers(data, style) -> String`
///
/// `data` is `List<Struct { label, description? }>`. Renders horizontal
/// stripes top-to-bottom.
pub fn builtin_diagram_layers(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_layers", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_layers: data list must not be empty".to_string());
    }
    if data.len() > 10 {
        return Err(format!(
            "diagram_layers: too many layers ({}), maximum is 10",
            data.len()
        ));
    }
    // Extract items
    let mut items: Vec<(String, Option<String>)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_layers: data[{}] must be Struct {{label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_layers item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push((label, description));
    }
    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let layer_h = canvas_h / (items.len() as f64);
    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    for (i, (label, description)) in items.iter().enumerate() {
        let y = (i as f64) * layer_h;
        // Alternating fill: even=index paper, odd=very light rule tint
        // (we don't have a tint primitive, so we use paper for even and
        // a manually lightened version of rule for odd). Simpler: use
        // paper for even layers, rule at 0.15 opacity for odd.
        let is_odd = i % 2 == 1;
        if is_odd {
            parts.push(format!(
                r#"<rect x="0" y="{}" width="{}" height="{}" fill="{}" opacity="0.18" />"#,
                fmt_num(y),
                fmt_num(canvas_w),
                fmt_num(layer_h),
                escape_attr(&rule)
            ));
        }
        // Top border (rule) — separates layers visually
        parts.push(format!(
            r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(y),
            fmt_num(canvas_w),
            fmt_num(y),
            escape_attr(&rule)
        ));
        // Label — left-aligned, vertically centered
        let label_y = y + layer_h / 2.0 + 4.0;
        // Accent left bar (3px wide) — visual anchor on the left edge
        parts.push(format!(
            r#"<rect x="0" y="{}" width="3" height="{}" fill="{}" />"#,
            fmt_num(y),
            fmt_num(layer_h),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="16" y="{}" font-size="14" fill="{}">{}</text>"#,
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
        // Description — right-aligned, smaller, muted
        if let Some(desc) = description {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="end">{}</text>"#,
                fmt_num(canvas_w - 16.0),
                fmt_num(label_y),
                escape_attr(&muted),
                escape_html_chars(desc)
            ));
        }
    }
    // Bottom border (rule)
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Наряд №82: Diagrams, part 2 — temporal & process ────────────────
//
// Five additional diagram builtins, all built on top of the geometric
// primitives delivered by Н81 (draw_connector, polar_to_xy) — no new
// geometry is invented here.
//
//   Block 1 — diagram_sequence  (uses draw_connector)
//   Block 2 — diagram_timeline  (uses svg_circle inline + horizontal line)
//   Block 3 — diagram_gantt     (uses svg_rect inline)
//   Block 4 — diagram_process   (uses draw_connector; NOT flowchart — strictly linear)
//   Block 5 — diagram_loop      (uses polar_to_xy + draw_connector; closed cycle)
//
// All five reuse the DIAGRAM_CANVAS_W/H constants (600×400) defined above
// for visual consistency with the Н81 diagram suite.

// ── Block 1: diagram_sequence ──────────────────────────────────────
//
// UML-style sequence diagram: vertical "lifelines" for each actor,
// horizontal arrows between lifelines for each message.
//
// Data shape:
//   Struct {
//     actors:   List<String>,                          // lifeline names
//     messages: List<Struct { from, to, label? }>,     // arrows
//   }
//
// Layout:
//   - N actors → evenly spaced columns across canvas_w
//   - Each actor: vertical dashed line top→bottom + name at top
//   - Each message: horizontal arrow from actor[from] to actor[to]
//     at Y = top_pad + msg_idx × step (top-down chronological order)
//   - Non-adjacent messages (e.g. actor 0 → actor 3) draw a longer
//     diagonal line — this is the spec's "проверить, что диагональные
//     стрелки строятся корректно" requirement.
//
// Limits: actors.len() ≤ 8, messages.len() ≤ 30.

const SEQ_MAX_ACTORS: usize = 8;
const SEQ_MAX_MESSAGES: usize = 30;
const SEQ_TOP_PAD: f64 = 50.0; // space for actor name labels at top
const SEQ_BOTTOM_PAD: f64 = 30.0;
const SEQ_LIFELINE_HALF_H: f64 = 12.0; // half-height of actor head box

/// `diagram_sequence(data, style) -> String`
///
/// Renders a UML-style sequence diagram. `data` is
/// `Struct { actors: List<String>, messages: List<Struct{from, to, label?}> }`.
///
/// Returns Err if:
///   - actors is empty or > 8
///   - messages > 30
///   - a message references an unknown actor name
pub fn builtin_diagram_sequence(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    // Extract top-level Struct { actors, messages }
    let data_fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_sequence: data must be Struct {{actors, messages}}, got {}",
                other.type_name()
            ));
        }
    };

    // actors: List<String>
    let actors_value = data_fields
        .get("actors")
        .ok_or_else(|| "diagram_sequence: missing required field 'actors'".to_string())?;
    let actors: Vec<String> = match actors_value {
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, v) in items.iter().enumerate() {
                match v {
                    Value::String(s) => out.push(s.clone()),
                    other => {
                        return Err(format!(
                            "diagram_sequence: actors[{}] must be String, got {}",
                            i,
                            other.type_name()
                        ));
                    }
                }
            }
            out
        }
        other => {
            return Err(format!(
                "diagram_sequence: 'actors' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };
    if actors.is_empty() {
        return Err("diagram_sequence: actors list must not be empty".to_string());
    }
    if actors.len() > SEQ_MAX_ACTORS {
        return Err(format!(
            "diagram_sequence: too many actors ({}), maximum is {} — lifelines become too narrow",
            actors.len(),
            SEQ_MAX_ACTORS
        ));
    }

    // messages: List<Struct{from, to, label?}>
    let messages_value = data_fields
        .get("messages")
        .ok_or_else(|| "diagram_sequence: missing required field 'messages'".to_string())?;
    let messages_list = match messages_value {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_sequence: 'messages' must be List<Struct>, got {}",
                other.type_name()
            ));
        }
    };
    if messages_list.len() > SEQ_MAX_MESSAGES {
        return Err(format!(
            "diagram_sequence: too many messages ({}), maximum is {}",
            messages_list.len(),
            SEQ_MAX_MESSAGES
        ));
    }
    // Extract messages — validate that from/to reference known actors
    struct SeqMessage {
        from_idx: usize,
        to_idx: usize,
        label: Option<String>,
    }
    let mut messages: Vec<SeqMessage> = Vec::with_capacity(messages_list.len());
    for (i, item) in messages_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_sequence: messages[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_sequence message", f, "from")?;
        let to = struct_string_field("diagram_sequence message", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        let from_idx = actors.iter().position(|a| a == &from).ok_or_else(|| {
            format!(
                "diagram_sequence: messages[{}].from references unknown actor {:?}",
                i, from
            )
        })?;
        let to_idx = actors.iter().position(|a| a == &to).ok_or_else(|| {
            format!(
                "diagram_sequence: messages[{}].to references unknown actor {:?}",
                i, to
            )
        })?;
        messages.push(SeqMessage {
            from_idx,
            to_idx,
            label,
        });
    }

    // Geometry
    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n_actors = actors.len();
    // Evenly space actors across canvas width with side padding
    let pad_x = 60.0_f64;
    let usable_w = canvas_w - 2.0 * pad_x;
    let actor_step = if n_actors > 1 {
        usable_w / (n_actors as f64 - 1.0)
    } else {
        0.0
    };
    let actor_x: Vec<f64> = (0..n_actors)
        .map(|i| {
            if n_actors > 1 {
                pad_x + (i as f64) * actor_step
            } else {
                canvas_w / 2.0
            }
        })
        .collect();

    let lifeline_top = SEQ_TOP_PAD + SEQ_LIFELINE_HALF_H;
    let lifeline_bottom = canvas_h - SEQ_BOTTOM_PAD;
    // Message Y positions: distribute between lifeline_top+10 and lifeline_bottom-10
    let msg_top = lifeline_top + 20.0;
    let msg_bottom = lifeline_bottom - 10.0;
    let msg_step = if messages.is_empty() {
        0.0
    } else if messages.len() > 1 {
        (msg_bottom - msg_top) / (messages.len() as f64 - 1.0)
    } else {
        0.0
    };

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Lifelines + actor labels
    for (i, name) in actors.iter().enumerate() {
        let x = actor_x[i];
        // Actor head box — small rounded rect with name centered
        let head_w = 90.0_f64.min(actor_step.max(80.0) - 12.0).max(60.0);
        let head_x = x - head_w / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
            fmt_num(head_x),
            fmt_num(SEQ_TOP_PAD - SEQ_LIFELINE_HALF_H),
            fmt_num(head_w),
            fmt_num(2.0 * SEQ_LIFELINE_HALF_H),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(SEQ_TOP_PAD + 4.0),
            escape_attr(&ink),
            escape_html_chars(name)
        ));
        // Vertical dashed lifeline below the head box
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" stroke-dasharray="4 4" />"#,
            fmt_num(x),
            fmt_num(lifeline_top),
            fmt_num(x),
            fmt_num(lifeline_bottom),
            escape_attr(&rule)
        ));
    }

    // Messages — horizontal arrows from actor[from] to actor[to]
    for (i, msg) in messages.iter().enumerate() {
        let y = msg_top + (i as f64) * msg_step;
        let x1 = actor_x[msg.from_idx];
        let x2 = actor_x[msg.to_idx];
        // Skip self-messages (from==to) drawn as a small loop — for MVP
        // we still emit a tiny U-shaped arrow. Simpler: skip the line and
        // just place a small note. For correctness of the contract test
        // "messages not only between neighbors", we handle the diagonal
        // case (different actors) here.
        if msg.from_idx == msg.to_idx {
            // Self-message: small loop on the lifeline
            let loop_w = 24.0_f64;
            let loop_h = 14.0_f64;
            // Draw a tiny rectangular loop returning to the same lifeline
            parts.push(format!(
                r#"<path d="M {} {} L {} {} L {} {} L {} {}" fill="none" stroke="{}" stroke-width="1.5" />"#,
                fmt_num(x1),
                fmt_num(y),
                fmt_num(x1 + loop_w),
                fmt_num(y),
                fmt_num(x1 + loop_w),
                fmt_num(y + loop_h),
                fmt_num(x1),
                fmt_num(y + loop_h),
                escape_attr(&rule)
            ));
            // Arrowhead at the end (pointing left into the lifeline)
            parts.push(format!(
                r#"<path d="M {} {} L {} {} L {} {} Z" fill="{}" stroke="none" />"#,
                fmt_num(x1),
                fmt_num(y + loop_h),
                fmt_num(x1 + 7.0),
                fmt_num(y + loop_h - 3.0),
                fmt_num(x1 + 7.0),
                fmt_num(y + loop_h + 3.0),
                escape_attr(&rule)
            ));
        } else {
            // Different actors — connector from (x1, y) to (x2, y)
            parts.push(draw_connector(x1, y, x2, y, &style));
        }
        // Optional label above the arrow midpoint
        if let Some(label) = &msg.label {
            let mid_x = (x1 + x2) / 2.0;
            let label_y = y - 6.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(label_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // accent unused for now — kept for visual parity with other diagrams
    let _ = &accent;

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 2: diagram_timeline ──────────────────────────────────────
//
// Horizontal timeline with event dots. MVP — no real date parsing,
// the `date` field is just a textual label; list order = timeline order.
//
// Data shape:
//   List<Struct { date: String, label: String, description?: String }>
//
// Layout:
//   - Horizontal axis line across the middle of the canvas
//   - N events → evenly spaced across chart_w (point i at
//     chart_x + i × chart_w / (N-1) for N>1; for N=1, single point at
//     chart_x + chart_w/2)
//   - Small circle (r=5) at each event position via inline <circle>
//     (we don't call builtin_svg_circle because we'd need to round-trip
//     through Value::String — direct format! is simpler and matches the
//     pattern used by chart_radar's vertex dots).
//   - `date` label ABOVE the dot for even-indexed events, BELOW for odd.
//     This is the "alternating by parity" rule from the spec — not a
//     real anti-overlap engine (that's Н87).
//   - `label` and `description` go on the OPPOSITE side of the dot
//     from `date`, so each event has at most: date (one side) +
//     label/description (other side).
//
// Limits: data.len() ≤ 12.

const TIMELINE_MAX_EVENTS: usize = 12;
const TIMELINE_AXIS_Y: f64 = 200.0; // middle of 400px canvas
const TIMELINE_DOT_R: f64 = 5.0;
const TIMELINE_LABEL_OFFSET: f64 = 22.0; // distance from dot to label

/// `diagram_timeline(data, style) -> String`
///
/// `data` is `List<Struct{date, label, description?}>`. Renders a horizontal
/// timeline with event dots. Labels alternate above/below by index parity
/// (simple alternation — not a full anti-overlap engine, that's Н87).
pub fn builtin_diagram_timeline(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_timeline", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_timeline: data list must not be empty".to_string());
    }
    if data.len() > TIMELINE_MAX_EVENTS {
        return Err(format!(
            "diagram_timeline: too many events ({}), maximum is {}",
            data.len(),
            TIMELINE_MAX_EVENTS
        ));
    }
    // Extract items
    struct TlEvent {
        date: String,
        label: String,
        description: Option<String>,
    }
    let mut items: Vec<TlEvent> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_timeline: data[{}] must be Struct {{date, label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let date = struct_string_field("diagram_timeline item", f, "date")?;
        let label = struct_string_field("diagram_timeline item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push(TlEvent {
            date,
            label,
            description,
        });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let chart_x = 60.0_f64;
    let chart_w = canvas_w - 2.0 * chart_x;
    let n = items.len();
    let step = if n > 1 {
        chart_w / (n as f64 - 1.0)
    } else {
        0.0
    };

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Horizontal axis line
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" />"#,
        fmt_num(chart_x - 10.0),
        fmt_num(TIMELINE_AXIS_Y),
        fmt_num(canvas_w - chart_x + 10.0),
        fmt_num(TIMELINE_AXIS_Y),
        escape_attr(&rule)
    ));
    // End caps (small ticks)
    for cap_x in &[chart_x - 10.0, canvas_w - chart_x + 10.0] {
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" />"#,
            fmt_num(*cap_x),
            fmt_num(TIMELINE_AXIS_Y - 6.0),
            fmt_num(*cap_x),
            fmt_num(TIMELINE_AXIS_Y + 6.0),
            escape_attr(&rule)
        ));
    }

    // Events
    for (i, ev) in items.iter().enumerate() {
        let x = if n > 1 {
            chart_x + (i as f64) * step
        } else {
            canvas_w / 2.0
        };
        let y = TIMELINE_AXIS_Y;
        // Event dot — accent fill
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(TIMELINE_DOT_R),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Alternate label position by parity:
        //   even index → date ABOVE, label/description BELOW
        //   odd  index → date BELOW, label/description ABOVE
        let date_above = i % 2 == 0;
        let date_y = if date_above {
            y - TIMELINE_LABEL_OFFSET
        } else {
            y + TIMELINE_LABEL_OFFSET + 4.0
        };
        let label_y = if date_above {
            y + TIMELINE_LABEL_OFFSET + 4.0
        } else {
            y - TIMELINE_LABEL_OFFSET
        };
        // Small tick connecting dot to date label
        let tick_y1 = if date_above {
            y - TIMELINE_DOT_R
        } else {
            y + TIMELINE_DOT_R
        };
        let tick_y2 = if date_above {
            date_y + 4.0
        } else {
            date_y - 8.0
        };
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(x),
            fmt_num(tick_y1),
            fmt_num(x),
            fmt_num(tick_y2),
            escape_attr(&rule)
        ));
        // Date label (accent color, slightly bold via font-size)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(date_y),
            escape_attr(&accent),
            escape_html_chars(&ev.date)
        ));
        // Event label (ink, primary)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&ev.label)
        ));
        // Optional description (muted, smaller, below/above label)
        if let Some(desc) = &ev.description {
            let desc_y = if date_above {
                label_y + 14.0
            } else {
                label_y - 14.0
            };
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(x),
                fmt_num(desc_y),
                escape_attr(&muted),
                escape_html_chars(desc)
            ));
        }
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 3: diagram_gantt ─────────────────────────────────────────
//
// Gantt chart: one horizontal bar per task, scaled to fit canvas.
// `start` and `duration` are abstract numeric units (days/weeks/etc —
// MVP does not bind to a calendar).
//
// Data shape:
//   List<Struct { task: String, start: Float, duration: Float }>
//
// Layout:
//   - canvas 600×400, chart area x=[140, 580] (left 140px reserved for
//     task labels), y=[40, 360]
//   - row_h = chart_h / N (each task gets equal vertical space)
//   - bar_y = chart_y_top + i × row_h + row_h × 0.25
//     (top + i×row + 25% inset so bars don't touch)
//   - bar_h = row_h × 0.5 (half the row height — leaves breathing room)
//   - bar_x = chart_x + (start / max_end) × chart_w
//   - bar_w = (duration / max_end) × chart_w
//     where max_end = max(start + duration) across all tasks
//   - Task label left-aligned to the right of the left margin
//     (i.e. at x = chart_x - 8, right-anchored)
//
// Limits: data.len() ≤ 15. duration ≤ 0 → Err (invalid input, not a
// silent zero-width bar).

const GANTT_MAX_TASKS: usize = 15;
const GANTT_CHART_X: f64 = 140.0; // left margin for task labels
const GANTT_CHART_W: f64 = 440.0; // 580 - 140
const GANTT_CHART_Y_TOP: f64 = 40.0;
const GANTT_CHART_H: f64 = 320.0; // 360 - 40

/// `diagram_gantt(data, style) -> String`
///
/// `data` is `List<Struct{task, start, duration}>`. Renders a Gantt chart
/// with one horizontal bar per task. The horizontal scale is derived from
/// `max(start + duration)` across all tasks.
pub fn builtin_diagram_gantt(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_gantt", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_gantt: data list must not be empty".to_string());
    }
    if data.len() > GANTT_MAX_TASKS {
        return Err(format!(
            "diagram_gantt: too many tasks ({}), maximum is {}",
            data.len(),
            GANTT_MAX_TASKS
        ));
    }
    // Extract items — validate duration > 0
    struct GanttTask {
        task: String,
        start: f64,
        duration: f64,
    }
    let mut items: Vec<GanttTask> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_gantt: data[{}] must be Struct {{task, start, duration}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let task = struct_string_field("diagram_gantt item", f, "task")?;
        let start = struct_float_field("diagram_gantt item", f, "start")?;
        let duration = struct_float_field("diagram_gantt item", f, "duration")?;
        if duration <= 0.0 {
            return Err(format!(
                "diagram_gantt: data[{}].duration must be positive (got {}) — invalid input, not a zero-width bar",
                i, duration
            ));
        }
        if start < 0.0 {
            return Err(format!(
                "diagram_gantt: data[{}].start must be non-negative (got {})",
                i, start
            ));
        }
        items.push(GanttTask {
            task,
            start,
            duration,
        });
    }
    // Scale: max(start + duration) across all tasks
    let max_end = items
        .iter()
        .map(|t| t.start + t.duration)
        .fold(f64::NEG_INFINITY, f64::max);
    if max_end <= 0.0 {
        return Err(format!(
            "diagram_gantt: max(start + duration) must be positive (got {})",
            max_end
        ));
    }
    let scale = GANTT_CHART_W / max_end;

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n = items.len();
    let row_h = GANTT_CHART_H / (n as f64);
    let bar_h = (row_h * 0.5).clamp(8.0, 28.0);

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Header rule (above first bar) — separates "title row" from bars
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(GANTT_CHART_Y_TOP - 8.0),
        fmt_num(canvas_w),
        fmt_num(GANTT_CHART_Y_TOP - 8.0),
        escape_attr(&rule)
    ));
    // Vertical separator between task labels and bar area
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(GANTT_CHART_X - 8.0),
        fmt_num(GANTT_CHART_Y_TOP - 8.0),
        fmt_num(GANTT_CHART_X - 8.0),
        fmt_num(GANTT_CHART_Y_TOP + GANTT_CHART_H + 8.0),
        escape_attr(&rule)
    ));
    // Bottom rule
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(GANTT_CHART_Y_TOP + GANTT_CHART_H + 8.0),
        fmt_num(canvas_w),
        fmt_num(GANTT_CHART_Y_TOP + GANTT_CHART_H + 8.0),
        escape_attr(&rule)
    ));

    // Bars + labels
    for (i, t) in items.iter().enumerate() {
        let row_y = GANTT_CHART_Y_TOP + (i as f64) * row_h;
        let bar_y = row_y + (row_h - bar_h) / 2.0;
        let bar_x = GANTT_CHART_X + t.start * scale;
        let bar_w = t.duration * scale;
        // Alternating row tint for readability
        if i % 2 == 1 {
            parts.push(format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="0.12" />"#,
                fmt_num(GANTT_CHART_X),
                fmt_num(row_y),
                fmt_num(GANTT_CHART_W),
                fmt_num(row_h),
                escape_attr(&rule)
            ));
        }
        // Task label (right-aligned, ink)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="end">{}</text>"#,
            fmt_num(GANTT_CHART_X - 12.0),
            fmt_num(bar_y + bar_h / 2.0 + 4.0),
            escape_attr(&ink),
            escape_html_chars(&t.task)
        ));
        // Bar — accent fill, paper stroke (subtle outline)
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1" rx="2" ry="2" />"#,
            fmt_num(bar_x),
            fmt_num(bar_y),
            fmt_num(bar_w),
            fmt_num(bar_h),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Duration label inside bar (if bar is wide enough)
        if bar_w > 40.0 {
            let dur_label = format!("{:.1}", t.duration);
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(bar_x + bar_w / 2.0),
                fmt_num(bar_y + bar_h / 2.0 + 3.0),
                escape_attr(&paper),
                escape_html_chars(&dur_label)
            ));
        }
    }
    // muted is used for nothing here but kept for style-token parity
    let _ = &muted;

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 4: diagram_process ───────────────────────────────────────
//
// Strictly LINEAR chain of numbered steps — NOT a flowchart.
//
// Difference from diagram_flowchart (Н81 Block 4):
//   - flowchart: arbitrary graph with branches/merges, topological sort
//     into layers, nodes/edges data shape.
//   - process: linear chain only, no branches, List<Struct{label, ...}>
//     data shape (NOT nodes/edges). Each step has a numbered badge
//     (1, 2, 3, ...) and is connected to the next via draw_connector.
//
// Data shape:
//   List<Struct { label: String, description?: String }>
//
// Layout:
//   - Horizontal chain of boxes left→right
//   - N steps → evenly spaced across chart_w
//   - Each box: 80w × 50h (or 60w × 60h if N is large) with rounded
//     corners, paper fill, rule border
//   - Numbered badge: small circle (r=10) in the top-left corner of
//     each box, filled with accent, containing the step number (1-indexed)
//   - Connectors between consecutive boxes via draw_connector
//
// Limits: data.len() ≤ 8.

const PROCESS_MAX_STEPS: usize = 8;
const PROCESS_BOX_W: f64 = 90.0;
const PROCESS_BOX_H: f64 = 56.0;
const PROCESS_BADGE_R: f64 = 10.0;

/// `diagram_process(data, style) -> String`
///
/// `data` is `List<Struct{label, description?}>`. Renders a strictly
/// linear chain of numbered steps connected by arrows. This is NOT the
/// same as diagram_flowchart — process has no branches/merges and a
/// different data shape (List<Struct>, not Struct{nodes, edges}).
pub fn builtin_diagram_process(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_process", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_process: data list must not be empty".to_string());
    }
    if data.len() > PROCESS_MAX_STEPS {
        return Err(format!(
            "diagram_process: too many steps ({}), maximum is {} — linear chain longer than 8 doesn't fit a reasonable canvas",
            data.len(),
            PROCESS_MAX_STEPS
        ));
    }
    // Extract items
    struct ProcStep {
        label: String,
        description: Option<String>,
    }
    let mut items: Vec<ProcStep> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_process: data[{}] must be Struct {{label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_process item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push(ProcStep { label, description });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n = items.len();
    // Center the chain horizontally — each box center at:
    //   x_i = pad + (i + 0.5) × (canvas_w - 2×pad) / n
    // where pad reserves space for half a box on each side.
    let pad = PROCESS_BOX_W / 2.0 + 12.0;
    let usable_w = canvas_w - 2.0 * pad;
    let step_w = usable_w / (n as f64);
    let box_cy = canvas_h / 2.0;
    let box_y = box_cy - PROCESS_BOX_H / 2.0;

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Connectors FIRST (so boxes render on top of any line tips)
    for i in 1..n {
        let prev_cx = pad + ((i - 1) as f64 + 0.5) * step_w;
        let curr_cx = pad + (i as f64 + 0.5) * step_w;
        let start_x = prev_cx + PROCESS_BOX_W / 2.0;
        let end_x = curr_cx - PROCESS_BOX_W / 2.0;
        // Horizontal connector at box vertical midpoint
        parts.push(draw_connector(start_x, box_cy, end_x, box_cy, &style));
    }

    // Boxes + badges + labels
    for (i, step) in items.iter().enumerate() {
        let cx = pad + (i as f64 + 0.5) * step_w;
        let box_x = cx - PROCESS_BOX_W / 2.0;
        // Box — paper fill, rule border, rounded
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(PROCESS_BOX_W),
            fmt_num(PROCESS_BOX_H),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        // Numbered badge — top-left corner, accent fill, paper text
        let badge_cx = box_x + 4.0 + PROCESS_BADGE_R;
        let badge_cy = box_y + 4.0 + PROCESS_BADGE_R;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy),
            fmt_num(PROCESS_BADGE_R),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Step number (1-indexed)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy + 4.0),
            escape_attr(&paper),
            i + 1
        ));
        // Label — centered horizontally, slightly below badge
        let label_y = box_y + PROCESS_BOX_H / 2.0 + 4.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&step.label)
        ));
        // Optional description below the box (small, muted)
        if let Some(desc) = &step.description {
            let desc_y = box_y + PROCESS_BOX_H + 14.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(desc_y),
                escape_attr(&muted),
                escape_html_chars(desc)
            ));
        }
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 5: diagram_loop ──────────────────────────────────────────
//
// Closed-loop (flywheel) diagram — steps arranged on a circle, last
// step connects back to the first to close the cycle.
//
// Data shape:
//   List<Struct { label: String, description?: String }>
//
// Layout:
//   - N steps placed on a circle using polar_to_xy
//   - angle_i = 2π × i / N − π/2  (start at top, same orientation as
//     chart_radar from Н79 — preserved for visual consistency between
//     circular functions in the graphics suite)
//   - Each step rendered as a small box at polar_to_xy(cx, cy, r, angle_i)
//   - Connectors via draw_connector from step i to step i+1
//   - Last step (i = N-1) connects back to step 0 (closed loop)
//
// Limits: 3 ≤ N ≤ 8 (N < 3 → visually meaningless cycle; N > 8 → labels
// overlap on the circle's circumference).

const LOOP_MIN_STEPS: usize = 3;
const LOOP_MAX_STEPS: usize = 8;
const LOOP_BOX_W: f64 = 90.0;
const LOOP_BOX_H: f64 = 44.0;

/// `diagram_loop(data, style) -> String`
///
/// `data` is `List<Struct{label, description?}>`. Renders a closed-loop
/// (flywheel) diagram with steps arranged on a circle. The last step
/// connects back to the first to close the cycle.
pub fn builtin_diagram_loop(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_loop", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_loop: data list must not be empty".to_string());
    }
    if data.len() < LOOP_MIN_STEPS {
        return Err(format!(
            "diagram_loop: too few steps ({}), minimum is {} — a cycle with <3 steps is visually meaningless",
            data.len(),
            LOOP_MIN_STEPS
        ));
    }
    if data.len() > LOOP_MAX_STEPS {
        return Err(format!(
            "diagram_loop: too many steps ({}), maximum is {} — labels would overlap on the circle",
            data.len(),
            LOOP_MAX_STEPS
        ));
    }
    // Extract items
    struct LoopStep {
        label: String,
        description: Option<String>,
    }
    let mut items: Vec<LoopStep> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_loop: data[{}] must be Struct {{label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_loop item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push(LoopStep { label, description });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;
    // Reserve room for labels OUTSIDE the boxes — reduce radius so boxes
    // fit comfortably inside the canvas (box half-diagonal ≈ 50px).
    let r = (canvas_h / 2.0 - 70.0).min(canvas_w / 2.0 - 80.0);
    let n = items.len();

    // Compute box centers via polar_to_xy
    let centers: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            // angle_i = 2π × i / N − π/2  (start at top)
            let angle =
                2.0 * std::f64::consts::PI * (i as f64) / (n as f64) - std::f64::consts::PI / 2.0;
            polar_to_xy(cx, cy, r, angle)
        })
        .collect();

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Faint reference circle (visual anchor — shows the conceptual loop)
    parts.push(format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.25" stroke-dasharray="3 3" />"#,
        fmt_num(cx),
        fmt_num(cy),
        fmt_num(r),
        escape_attr(&rule)
    ));

    // Connectors FIRST (so boxes render on top)
    // For each step i, draw connector from centers[i] to centers[(i+1) % n].
    // The modular +1 ensures the last step connects back to the first —
    // this is the closed-loop invariant from the narazd spec.
    for i in 0..n {
        let (x1, y1) = centers[i];
        let (x2, y2) = centers[(i + 1) % n];
        // Trim endpoints so connectors touch box edges, not centers
        let (sx, sy) = box_edge_point(x1, y1, x2, y2, LOOP_BOX_W, LOOP_BOX_H);
        let (ex, ey) = box_edge_point(x2, y2, x1, y1, LOOP_BOX_W, LOOP_BOX_H);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
    }

    // Boxes + labels
    for (i, step) in items.iter().enumerate() {
        let (bx, by) = centers[i];
        let box_x = bx - LOOP_BOX_W / 2.0;
        let box_y = by - LOOP_BOX_H / 2.0;
        // Box — paper fill, accent border (loop is "the protagonist" so
        // its border is accent rather than rule — visual emphasis)
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(LOOP_BOX_W),
            fmt_num(LOOP_BOX_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        // Step number (small badge in top-left, similar to diagram_process)
        let badge_cx = box_x + 12.0;
        let badge_cy = box_y + 12.0;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="8" fill="{}" />"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="10" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy + 3.0),
            escape_attr(&paper),
            i + 1
        ));
        // Label — centered horizontally, vertically just below center
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(bx),
            fmt_num(by + 4.0),
            escape_attr(&ink),
            escape_html_chars(&step.label)
        ));
        // Optional description below the box, anchored to the side facing
        // away from the center (so it doesn't overlap the loop interior).
        if let Some(desc) = &step.description {
            // Direction from center to box → outward normal
            let dx = bx - cx;
            let dy = by - cy;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let nx = dx / len;
            let ny = dy / len;
            let desc_x = bx + nx * (LOOP_BOX_W / 2.0 + 14.0);
            let desc_y = by + ny * (LOOP_BOX_H / 2.0 + 14.0) + 4.0;
            // Anchor depends on which side of the circle we're on
            let anchor = if nx > 0.3 {
                "start"
            } else if nx < -0.3 {
                "end"
            } else {
                "middle"
            };
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="{}">{}</text>"#,
                fmt_num(desc_x),
                fmt_num(desc_y),
                escape_attr(&muted),
                anchor,
                escape_html_chars(desc)
            ));
        }
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Наряд №83: Diagrams, part 3 — sets & comparisons ────────────────
//
// Five additional diagram builtins where area/intersection carries the
// meaning (as opposed to nodes/edges in Н81–82). One of them
// (diagram_medallion) reuses the existing svg_icon builtin's icon path
// table — no new icon geometry is invented here.
//
//   Block 1 — diagram_venn     (2 or 3 semi-transparent circles)
//   Block 2 — diagram_quadrant  (cross axes + scattered points in [-1,1]²)
//   Block 3 — diagram_pyramid   (stacked trapezoids, top = apex)
//   Block 4 — diagram_nested    (concentric circles, outer = first)
//   Block 5 — diagram_medallion (row of round badges, reuses svg_icon)
//
// All five reuse the DIAGRAM_CANVAS_W/H constants (600×400) defined above
// for visual consistency with the Н81–82 diagram suite.

// ── Block 1: diagram_venn ───────────────────────────────────────────
//
// Venn-style overlap diagram. DELIBERATELY restricted to 2 or 3 circles
// — the general N-circle case requires polygon intersection math and is
// explicitly out of scope (see the narazd spec: "Не решать общую задачу
// N-кругового Venn — строго 2 или 3, фиксированные симметричные позиции").
//
// Data shape:
//   Struct {
//     circles:      List<Struct { label: String, value: Float? }>,  // len == 2 or 3
//     overlap_label: String?,                                       // optional center label
//   }
//
// Geometry (fixed symmetric layouts — no overlap area computation):
//   - 2 circles: centers offset horizontally by ±0.3×radius from canvas
//     center; both have the same radius. The visible intersection is a
//     symmetric lens shape.
//   - 3 circles: centers at vertices of an equilateral triangle inscribed
//     in a circle of radius `0.7×R / √3` around the canvas center, where
//     R is the circle radius. Standard 3-set Venn layout — produces a
//     visible central triple-overlap region.
//
// Circles use semi-transparent accent fill (opacity 0.35) so overlap
// regions are visible as darker tones — same approach as chart_area's
// translucent fill. Labels render at a fixed offset from each circle's
// center (the spec explicitly says we don't compute non-overlapping
// label regions — MVP).
//
// Limits: circles.len() must be 2 or 3 (any other count → Err).

const VENN_CIRCLE_R: f64 = 110.0;
const VENN_2_OFFSET: f64 = 66.0; // 0.6 × R / 2 — centers at ±0.3R
const VENN_3_RING_R: f64 = 64.0; // ~ 0.7 × R / √3 — equilateral triangle circumradius

/// `diagram_venn(data, style) -> String`
///
/// Renders a 2- or 3-circle Venn diagram with semi-transparent fills.
/// `data` is `Struct { circles: List<Struct{label, value?}>, overlap_label? }`.
///
/// Returns Err if:
///   - circles.len() is not 2 or 3
///   - any circle is missing the `label` field
pub fn builtin_diagram_venn(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let data = match &data_value {
        Value::Struct { fields, .. } => fields.clone(),
        other => {
            return Err(format!(
                "diagram_venn: data must be Struct {{circles, overlap_label?}}, got {}",
                other.type_name()
            ));
        }
    };
    // Extract circles list
    let circles_value = match data.get("circles") {
        Some(Value::List(l)) => l.clone(),
        Some(other) => {
            return Err(format!(
                "diagram_venn: circles must be List<Struct{{label, value?}}>, got {}",
                other.type_name()
            ));
        }
        None => {
            return Err(
                "diagram_venn: missing required field 'circles' (List<Struct{label, value?}>)"
                    .to_string(),
            );
        }
    };
    // Validate count — explicitly restricted to 2 or 3 (no general N-case)
    if circles_value.len() != 2 && circles_value.len() != 3 {
        return Err(format!(
            "diagram_venn: supports exactly 2 or 3 circles, got {}",
            circles_value.len()
        ));
    }
    // Extract each circle's label and optional value
    struct VennCircle {
        label: String,
        value: Option<f64>,
    }
    let mut circles: Vec<VennCircle> = Vec::with_capacity(circles_value.len());
    for (i, item) in circles_value.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_venn: circles[{}] must be Struct {{label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_venn circle", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        circles.push(VennCircle { label, value });
    }
    // Optional overlap_label — top-level field, not inside the list
    let overlap_label = struct_opt_string_field(&data, "overlap_label");

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;

    // Compute circle centers for the requested layout.
    let centers: Vec<(f64, f64)> = match circles.len() {
        2 => vec![(cx - VENN_2_OFFSET, cy), (cx + VENN_2_OFFSET, cy)],
        3 => {
            // Equilateral triangle: angles 90°, 210°, 330° (measured from
            // +x axis) — but we want one vertex pointing UP, so we use
            // -π/2 (top), -π/2 + 2π/3 (lower-left), -π/2 + 4π/3 (lower-right).
            // Same orientation convention as chart_radar / diagram_loop.
            (0..3)
                .map(|i| {
                    let angle =
                        -std::f64::consts::PI / 2.0 + 2.0 * std::f64::consts::PI * (i as f64) / 3.0;
                    polar_to_xy(cx, cy, VENN_3_RING_R, angle)
                })
                .collect()
        }
        // Unreachable: count validated above, but the compiler doesn't know.
        _ => unreachable!("diagram_venn count validated above"),
    };

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Circles — semi-transparent accent fill so overlaps are visible as
    // darker tones. Stroke with accent at full opacity for crisp edges.
    for (i, (ccx, ccy)) in centers.iter().enumerate() {
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" fill-opacity="0.35" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(*ccx),
            fmt_num(*ccy),
            fmt_num(VENN_CIRCLE_R),
            escape_attr(&accent),
            escape_attr(&accent)
        ));
        // Circle label — placed at a fixed offset from the circle center,
        // AWAY from the canvas center, so the label sits outside the
        // densest overlap area. Direction = (center - canvas_center).
        let dx = *ccx - cx;
        let dy = *ccy - cy;
        let len = (dx * dx + dy * dy).sqrt();
        let (nx, ny) = if len < 0.001 {
            (0.0, -1.0) // fallback for 2-circle case where centers are horizontal
        } else {
            (dx / len, dy / len)
        };
        let label_x = *ccx + nx * (VENN_CIRCLE_R * 0.55);
        let label_y = ccy + ny * (VENN_CIRCLE_R * 0.55) + 4.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(label_x),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&circles[i].label)
        ));
        // Optional value — small muted text just below the label
        if let Some(v) = circles[i].value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(label_x),
                fmt_num(label_y + 14.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Optional overlap_label — at canvas center (the visual centroid of
    // all intersections for both 2- and 3-circle layouts).
    if let Some(ol) = &overlap_label {
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" font-style="italic" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(cy + 4.0),
            escape_attr(&muted),
            escape_html_chars(ol)
        ));
    }

    // Faint border around canvas — visual frame consistent with other diagrams
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 2: diagram_quadrant ───────────────────────────────────────
//
// 2×2 strategic quadrant chart (BCG-matrix style). Cross-shaped axes
// through the canvas center, points scattered in the [-1, 1] × [-1, 1]
// logical space.
//
// Data shape:
//   Struct {
//     x_axis_label: String,
//     y_axis_label: String,
//     items: List<Struct { label: String, x: Float, y: Float }>,
//   }
//
// Geometry:
//   - Horizontal axis: full-width line at canvas vertical center.
//   - Vertical axis: full-height line at canvas horizontal center.
//   - Axis labels: at the right end (x) and top end (y) of each axis.
//   - For each item: pixel_x = cx + x × half_w, pixel_y = cy − y × half_h
//     (y is inverted because SVG y grows downward). Circle marker +
//     label text anchored to the right of the marker.
//
// Limits:
//   - Any item.x or item.y outside [-1.0, 1.0] → Err
//   - items.len() > 20 → Err (points would be visually indistinguishable)

const QUADRANT_MAX_ITEMS: usize = 20;
const QUADRANT_HALF_W: f64 = 250.0; // cx ± half_w → x range [50, 550] on 600 canvas
const QUADRANT_HALF_H: f64 = 160.0; // cy ± half_h → y range [40, 360] on 400 canvas

/// `diagram_quadrant(data, style) -> String`
///
/// Renders a 2×2 quadrant chart with cross axes and scattered points.
/// `data` is `Struct { x_axis_label, y_axis_label, items: List<Struct{label, x, y}> }`.
///
/// Returns Err if:
///   - any item.x or item.y is outside [-1.0, 1.0]
///   - items.len() > 20
///   - missing required fields
pub fn builtin_diagram_quadrant(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let data = match &data_value {
        Value::Struct { fields, .. } => fields.clone(),
        other => {
            return Err(format!(
                "diagram_quadrant: data must be Struct {{x_axis_label, y_axis_label, items}}, got {}",
                other.type_name()
            ));
        }
    };
    // Both axis labels are TOP-LEVEL fields (not inside the items list) —
    // the spec explicitly calls this out as a category that's easy to
    // forget in the security scanner.
    let x_axis_label = struct_string_field("diagram_quadrant", &data, "x_axis_label")?;
    let y_axis_label = struct_string_field("diagram_quadrant", &data, "y_axis_label")?;
    let items_value = match data.get("items") {
        Some(Value::List(l)) => l.clone(),
        Some(other) => {
            return Err(format!(
                "diagram_quadrant: items must be List<Struct{{label, x, y}}>, got {}",
                other.type_name()
            ));
        }
        None => {
            return Err(
                "diagram_quadrant: missing required field 'items' (List<Struct{label, x, y}>)"
                    .to_string(),
            );
        }
    };
    if items_value.is_empty() {
        return Err("diagram_quadrant: items list must not be empty".to_string());
    }
    if items_value.len() > QUADRANT_MAX_ITEMS {
        return Err(format!(
            "diagram_quadrant: too many items ({}), maximum is {} — points would be visually indistinguishable",
            items_value.len(),
            QUADRANT_MAX_ITEMS
        ));
    }

    struct QuadItem {
        label: String,
        x: f64,
        y: f64,
    }
    let mut items: Vec<QuadItem> = Vec::with_capacity(items_value.len());
    for (i, item) in items_value.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_quadrant: items[{}] must be Struct {{label, x, y}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_quadrant item", f, "label")?;
        let x = struct_float_field("diagram_quadrant item", f, "x")?;
        let y = struct_float_field("diagram_quadrant item", f, "y")?;
        // Range check — explicit error per the spec
        if !(-1.0..=1.0).contains(&x) {
            return Err(format!(
                "diagram_quadrant: items[{}].x = {} is out of range [-1.0, 1.0]",
                i, x
            ));
        }
        if !(-1.0..=1.0).contains(&y) {
            return Err(format!(
                "diagram_quadrant: items[{}].y = {} is out of range [-1.0, 1.0]",
                i, y
            ));
        }
        items.push(QuadItem { label, x, y });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Faint quadrant divider tint — very light accent wash to suggest the
    // four regions without obscuring the points. This is purely cosmetic;
    // the axes themselves carry the structural meaning.
    parts.push(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" fill-opacity="0.05" />"#,
        fmt_num(cx),
        fmt_num(40.0),
        fmt_num(QUADRANT_HALF_W),
        fmt_num(QUADRANT_HALF_H),
        escape_attr(&accent)
    ));
    // Horizontal axis (x-axis) — through vertical center
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" />"#,
        fmt_num(40.0),
        fmt_num(cy),
        fmt_num(canvas_w - 40.0),
        fmt_num(cy),
        escape_attr(&rule)
    ));
    // Vertical axis (y-axis) — through horizontal center
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" />"#,
        fmt_num(cx),
        fmt_num(40.0),
        fmt_num(cx),
        fmt_num(canvas_h - 40.0),
        escape_attr(&rule)
    ));
    // Axis labels — at the ends of each axis (x: right end, y: top end).
    // These are top-level fields per the spec; they are NOT items[].label.
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(canvas_w - 40.0),
        fmt_num(cy - 8.0),
        escape_attr(&ink),
        escape_html_chars(&x_axis_label)
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
        fmt_num(cx),
        fmt_num(32.0),
        escape_attr(&ink),
        escape_html_chars(&y_axis_label)
    ));

    // Items — circle marker + label
    for item in items.iter() {
        let px = cx + item.x * QUADRANT_HALF_W;
        // SVG y grows downward, so positive logical y → smaller pixel y
        let py = cy - item.y * QUADRANT_HALF_H;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="5" fill="{}" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(px),
            fmt_num(py),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Label slightly offset to the right of the marker — simple MVP
        // placement (the spec doesn't require collision avoidance).
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}">{}</text>"#,
            fmt_num(px + 8.0),
            fmt_num(py + 4.0),
            escape_attr(&muted),
            escape_html_chars(&item.label)
        ));
    }

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 3: diagram_pyramid ────────────────────────────────────────
//
// Stacked trapezoidal layers forming a pyramid. CRITICAL ORDERING RULE:
// the FIRST list element is the TOP (apex) — the narrowest layer. The
// LAST element is the BOTTOM (base) — the widest layer. This matches the
// natural top-down description of hierarchies (e.g. Maslow: "self-
// actualization" first = top of pyramid). DO NOT FLIP.
//
// Data shape:
//   List<Struct { label: String, value: Float? }>
//
// Geometry:
//   - Pyramid centered horizontally at cx = canvas_w / 2.
//   - Vertical extent: y ∈ [40, 360] (320 px tall, leaving 40 px margin
//     top/bottom on the 400 px canvas).
//   - Layer i (0-indexed from top):
//       top_y    = 40 + i × layer_h
//       bot_y    = 40 + (i+1) × layer_h
//       top_w    = (i / N) × max_w     ← linearly proportional to position
//       bot_w    = ((i+1) / N) × max_w
//     When i=0 (apex), top_w=0 → the apex is a single point (degenerate
//     trapezoid that's actually a triangle). This is the classic pyramid
//     silhouette.
//   - Trapezoid rendered as <path d="M ... L ... L ... L ... Z"> with
//     4 explicit corner points (NOT a <rect> — the spec forbids that).
//   - Label centered in each layer; optional value rendered as smaller
//     muted text just below the label.
//
// Limits:
//   - data.len() < 2 → Err (a 1-layer pyramid is meaningless)
//   - data.len() > 6 → Err (layers become too thin vertically)

const PYRAMID_MIN_LAYERS: usize = 2;
const PYRAMID_MAX_LAYERS: usize = 6;
const PYRAMID_TOP_Y: f64 = 40.0;
const PYRAMID_BOT_Y: f64 = 360.0;
const PYRAMID_MAX_W: f64 = 480.0; // canvas_w − 2 × 60 margin

/// `diagram_pyramid(data, style) -> String`
///
/// Renders a pyramid of stacked trapezoids. `data` is
/// `List<Struct{label, value?}>`. **The first element is the TOP (apex)
/// of the pyramid** — see the ordering rule in the file-level comment.
///
/// Returns Err if:
///   - data.len() < 2 or > 6
pub fn builtin_diagram_pyramid(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_pyramid", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.len() < PYRAMID_MIN_LAYERS {
        return Err(format!(
            "diagram_pyramid: too few layers ({}), minimum is {} — a single-layer pyramid is meaningless",
            data.len(),
            PYRAMID_MIN_LAYERS
        ));
    }
    if data.len() > PYRAMID_MAX_LAYERS {
        return Err(format!(
            "diagram_pyramid: too many layers ({}), maximum is {} — layers would become too thin vertically",
            data.len(),
            PYRAMID_MAX_LAYERS
        ));
    }
    struct PyramidLayer {
        label: String,
        value: Option<f64>,
    }
    let mut layers: Vec<PyramidLayer> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_pyramid: data[{}] must be Struct {{label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_pyramid item", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        layers.push(PyramidLayer { label, value });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let n = layers.len();
    let pyramid_h = PYRAMID_BOT_Y - PYRAMID_TOP_Y;
    let layer_h = pyramid_h / (n as f64);

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Layers — top-down order is critical: layers[0] is the apex (top).
    for (i, layer) in layers.iter().enumerate() {
        let top_y = PYRAMID_TOP_Y + (i as f64) * layer_h;
        let bot_y = PYRAMID_TOP_Y + ((i + 1) as f64) * layer_h;
        // Linearly proportional widths: top_w = (i/N) × max_w
        // For i=0 (apex): top_w = 0 → triangle silhouette
        let top_w = (i as f64 / n as f64) * PYRAMID_MAX_W;
        let bot_w = ((i + 1) as f64 / n as f64) * PYRAMID_MAX_W;
        // 4 corner points of the trapezoid (clockwise from top-left)
        let tl_x = cx - top_w / 2.0;
        let tr_x = cx + top_w / 2.0;
        let br_x = cx + bot_w / 2.0;
        let bl_x = cx - bot_w / 2.0;
        // Alternate fill: even layers get accent at low opacity, odd get
        // muted at low opacity — visual differentiation without heavy
        // color noise. (Same alternating pattern as diagram_layers.)
        let fill_color = if i % 2 == 0 { &accent } else { &muted };
        // Trapezoid as <path> with 4 explicit points + Z (NOT a <rect>).
        parts.push(format!(
            r#"<path d="M {} {} L {} {} L {} {} L {} {} Z" fill="{}" fill-opacity="0.18" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(tl_x),
            fmt_num(top_y),
            fmt_num(tr_x),
            fmt_num(top_y),
            fmt_num(br_x),
            fmt_num(bot_y),
            fmt_num(bl_x),
            fmt_num(bot_y),
            escape_attr(fill_color),
            escape_attr(fill_color)
        ));
        // Label — centered horizontally, vertically at the middle of the
        // layer. For very narrow apex layers (i=0 with top_w=0), the
        // label may overflow horizontally — we accept that as MVP.
        let label_y = (top_y + bot_y) / 2.0 + 4.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&layer.label)
        ));
        // Optional value — small muted text below the label
        if let Some(v) = layer.value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(label_y + 14.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Faint canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 4: diagram_nested ─────────────────────────────────────────
//
// Concentric circles — outermost ring is the FIRST list element,
// innermost is the LAST. Useful for "onion" or "scope" diagrams where
// each layer wraps the ones inside it.
//
// Data shape:
//   List<Struct { label: String, value: Float? }>
//
// Geometry:
//   - All circles centered at canvas center (cx, cy).
//   - Outermost radius = max_r; innermost = max_r / N (linear steps).
//   - Ring i radius: r_i = max_r × (N − i) / N
//     so r_0 = max_r (outermost) and r_{N-1} = max_r / N (innermost).
//   - Light fill so inner circles remain visible — alternating accent
//     and muted at very low opacity (0.08). Stroke at full opacity.
//   - Labels placed at the top of each ring (12 o'clock position),
//     stacked vertically as the rings get smaller. This is the MVP
//     placement — pointer lines to a side legend would be the polished
//     version, deferred per the spec ("решить по месту — MVP").
//
// Limits:
//   - data.len() > 5 → Err (rings become indistinguishably thin)

const NESTED_MAX_RINGS: usize = 5;
const NESTED_MAX_R: f64 = 160.0; // limited by canvas_h/2 − 40 margin

/// `diagram_nested(data, style) -> String`
///
/// Renders concentric circles. `data` is `List<Struct{label, value?}>`.
/// The FIRST element is the OUTERMOST ring; the LAST is the innermost.
///
/// Returns Err if:
///   - data is empty
///   - data.len() > 5
pub fn builtin_diagram_nested(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_nested", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_nested: data list must not be empty".to_string());
    }
    if data.len() > NESTED_MAX_RINGS {
        return Err(format!(
            "diagram_nested: too many rings ({}), maximum is {} — rings would become indistinguishably thin",
            data.len(),
            NESTED_MAX_RINGS
        ));
    }
    struct NestedRing {
        label: String,
        value: Option<f64>,
    }
    let mut rings: Vec<NestedRing> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_nested: data[{}] must be Struct {{label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_nested item", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        rings.push(NestedRing { label, value });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;
    let n = rings.len();

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Rings — draw OUTERMOST first (so inner rings render on top of it).
    // rings[0] = outermost, so iterating in natural order gives the right
    // z-order: outer fill is laid down first, inner strokes overwrite it.
    for (i, ring) in rings.iter().enumerate() {
        let r = NESTED_MAX_R * (n as f64 - i as f64) / (n as f64);
        // Alternating fill — very low opacity so nested rings remain
        // distinguishable without darkening the center excessively.
        let fill_color = if i % 2 == 0 { &accent } else { &muted };
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" fill-opacity="0.08" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(cx),
            fmt_num(cy),
            fmt_num(r),
            escape_attr(fill_color),
            escape_attr(fill_color)
        ));
        // Label at top of each ring (12 o'clock position). Stacked
        // vertically as rings get smaller — simple MVP placement.
        let label_y = cy - r + 14.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&ring.label)
        ));
        // Optional value — small muted text below the label
        if let Some(v) = ring.value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="9" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(label_y + 12.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Faint canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 5: diagram_medallion ──────────────────────────────────────
//
// Row of round "medallion" badges. Each medallion optionally contains
// an icon (reusing the existing svg_icon builtin's path table — see
// icon_path_data) or the first letter of the label as a fallback.
//
// Data shape:
//   List<Struct { icon: String?, label: String, value: Float? }>
//
// `icon` is optional. When present, it MUST be one of the 10 known
// svg_icon names (server, laptop, phone, database, cloud, arrow-right,
// check, warning, user, document). Unknown names produce the SAME ERROR
// TEXT that svg_icon itself produces — we deliberately reuse icon_path_data
// for validation rather than duplicating the name list here (per the
// spec: "не дублировать список имён иконок").
//
// Geometry:
//   - Row of N medallions, centered horizontally on the canvas.
//   - Each medallion: 60 px diameter circle, 24 px gap between centers.
//   - Medallion center Y = 160 (leaves room for label + value below).
//   - If icon specified: 24×24 icon centered inside the medallion.
//   - Else: first character of label, large bold text, centered.
//   - Label below medallion (12 px font, centered).
//   - Optional value: smaller muted text below label.
//
// Limits:
//   - data.len() > 6 → Err (medallions won't fit horizontally)

const MEDALLION_MAX_ITEMS: usize = 6;
const MEDALLION_D: f64 = 60.0; // diameter
const MEDALLION_GAP: f64 = 24.0; // center-to-center gap above diameter
const MEDALLION_CY: f64 = 160.0;
const MEDALLION_ICON_SIZE: f64 = 28.0; // icon fits inside the 60px circle

/// `diagram_medallion(data, style) -> String`
///
/// Renders a row of round medallion badges. `data` is
/// `List<Struct{icon?, label, value?}>`. Icons reuse the svg_icon builtin's
/// validation (no name list duplication).
///
/// Returns Err if:
///   - data is empty
///   - data.len() > 6
///   - icon is present but not one of the 10 known names (error text
///     comes from icon_path_data's validation, identical to svg_icon)
pub fn builtin_diagram_medallion(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_medallion", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_medallion: data list must not be empty".to_string());
    }
    if data.len() > MEDALLION_MAX_ITEMS {
        return Err(format!(
            "diagram_medallion: too many items ({}), maximum is {} — medallions won't fit horizontally on the canvas",
            data.len(),
            MEDALLION_MAX_ITEMS
        ));
    }
    struct Medallion {
        icon: Option<String>,
        label: String,
        value: Option<f64>,
    }
    let mut medallions: Vec<Medallion> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_medallion: data[{}] must be Struct {{icon?, label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let icon = struct_opt_string_field(f, "icon");
        let label = struct_string_field("diagram_medallion item", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        // Validate icon name by reusing icon_path_data — this is the
        // spec-mandated "don't duplicate the icon name list" pattern.
        // If icon is Some(name) and name is unknown, we return the same
        // shape of error that builtin_svg_icon would, but with the
        // diagram_medallion: prefix so callers can attribute the failure
        // to the builtin they actually called.
        if let Some(ref name) = icon {
            if icon_path_data(name).is_none() {
                return Err(format!(
                    "diagram_medallion: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                    name
                ));
            }
        }
        medallions.push(Medallion { icon, label, value });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n = medallions.len();
    // total_width = N × D + (N−1) × GAP (center-to-center spacing = D + GAP)
    let total_width = (n as f64) * MEDALLION_D + ((n as f64) - 1.0) * MEDALLION_GAP;
    let start_x = (canvas_w - total_width) / 2.0 + MEDALLION_D / 2.0;

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    for (i, m) in medallions.iter().enumerate() {
        let cx = start_x + (i as f64) * (MEDALLION_D + MEDALLION_GAP);
        let cy = MEDALLION_CY;
        let r = MEDALLION_D / 2.0;
        // Medallion circle — paper fill, accent stroke (visual emphasis)
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="2" />"#,
            fmt_num(cx),
            fmt_num(cy),
            fmt_num(r),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        // Content: icon (if specified) OR first letter of label (fallback).
        if let Some(ref icon_name) = m.icon {
            // Reuse icon_path_data with proper error propagation — same
            // pattern as builtin_svg_icon (line 2879). The early validation
            // in the parse loop above already rejects unknown names, so this
            // branch is effectively unreachable for valid inputs; but we
            // still propagate via `?` rather than `unwrap()` because the
            // project denies clippy::unwrap_used unconditionally.
            let path_data = icon_path_data(icon_name).ok_or_else(|| {
                format!(
                    "diagram_medallion: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                    icon_name
                )
            })?;
            let scale = MEDALLION_ICON_SIZE / 24.0;
            let icon_x = cx - MEDALLION_ICON_SIZE / 2.0;
            let icon_y = cy - MEDALLION_ICON_SIZE / 2.0;
            // Inline the same <svg> wrapper that builtin_svg_icon produces,
            // so the icon is positioned correctly inside the medallion.
            // We do NOT call builtin_svg_icon directly because it returns
            // a Value::String (extra unwrap) and we already have the path
            // data from the validation step above.
            parts.push(format!(
                r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 24 24"><g transform="scale({})"><path d="{}" stroke="{}" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></g></svg>"#,
                fmt_num(icon_x),
                fmt_num(icon_y),
                fmt_num(MEDALLION_ICON_SIZE),
                fmt_num(MEDALLION_ICON_SIZE),
                fmt_num(scale),
                path_data,
                escape_attr(&ink)
            ));
        } else {
            // Fallback: first character of the label, large bold text.
            // Char-based slicing is safe here because m.label is a valid
            // UTF-8 String; chars().next() gives us the first grapheme.
            let first_char = m.label.chars().next().unwrap_or('?');
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="24" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(cy + 8.0),
                escape_attr(&accent),
                escape_html_chars(&first_char.to_string())
            ));
        }
        // Label below medallion
        let label_y = cy + r + 18.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&m.label)
        ));
        // Optional value — smaller muted text below label
        if let Some(v) = m.value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(label_y + 14.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Faint canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));

    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Наряд №84: Diagrams, part 4 — data & state ───────────────────────
//
// Six new diagram builtins (the largest single narazd in the diagram
// series). Three of them (data_flow, high_level, architecture) share
// the same `Struct{nodes, edges}` graph shape and the generalized
// topological_layers / bfs_layers_with_cycles helpers from Block 4;
// they differ only in (a) which layer function they call and (b) how
// they render each node box (plain rect, large labeled block, or
// block with svg_icon).
//
//   Block 1 — diagram_er         (entity-relation grid; no graph layout)
//   Block 2 — diagram_state      (state machine, BFS layout, cycles OK)
//   Block 3 — diagram_swimlane   (vertical lanes, steps positioned by order)
//   Block 5 — diagram_data_flow  (graph, BFS layout, cycles OK)
//   Block 6 — diagram_high_level + diagram_architecture
//                              (graph, topological, no cycles; arch has icons)
//
// All reuse DIAGRAM_CANVAS_W/H (600×400) for visual consistency with
// the Н81–83 diagram suite.

// ── Block 1: diagram_er ─────────────────────────────────────────────
//
// Entity-Relationship diagram. Each entity is a rectangle split into
// a header (entity name) and a body listing its fields. Relations are
// drawn as connectors between entity box edges, with the optional
// relation label (e.g. "1:N", "1:1") placed at the line midpoint.
//
// Layout is a SIMPLE GRID (3 per row) — the spec is explicit:
// "Не решать общую задачу graph layout для diagram_er — простая сетка,
// не анализ связей для позиционирования." ER diagrams routinely
// contain cycles (bidirectional relationships, many-to-many), so the
// topological sort from Block 4 doesn't apply here.
//
// Limits: entities.len() ≤ 12, fields.len() ≤ 8 per entity.

const ER_MAX_ENTITIES: usize = 12;
const ER_MAX_FIELDS: usize = 8;
const ER_PER_ROW: usize = 3;
const ER_BOX_W: f64 = 160.0;
const ER_BOX_HEADER_H: f64 = 22.0;
const ER_FIELD_H: f64 = 14.0;
const ER_BOX_PADDING: f64 = 14.0;
const ER_GRID_GAP_X: f64 = 30.0;
const ER_GRID_GAP_Y: f64 = 30.0;
const ER_GRID_TOP: f64 = 30.0;

struct ErEntity {
    name: String,
    fields: Vec<String>,
}

struct ErRelation {
    from: String,
    to: String,
    label: Option<String>,
}

/// `diagram_er(data, style) -> String`
///
/// `data` is `Struct { entities: List<Struct{name, fields: List<String>}>,
/// relations: List<Struct{from, to, label?}> }`.
///
/// Returns Err if entities.len() > 12, fields.len() > 8 for any entity,
/// or a relation endpoint doesn't match any entity name.
pub fn builtin_diagram_er(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_er: data must be Struct {{entities, relations}}, got {}",
                other.type_name()
            ));
        }
    };
    let entities_val = fields
        .get("entities")
        .ok_or_else(|| "diagram_er: missing 'entities' field".to_string())?;
    let relations_val = fields
        .get("relations")
        .ok_or_else(|| "diagram_er: missing 'relations' field".to_string())?;
    let entities_list = match entities_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_er: 'entities' must be List, got {}",
                other.type_name()
            ));
        }
    };
    let relations_list = match relations_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_er: 'relations' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if entities_list.is_empty() {
        return Err("diagram_er: entities list must not be empty".to_string());
    }
    if entities_list.len() > ER_MAX_ENTITIES {
        return Err(format!(
            "diagram_er: too many entities ({}), maximum is {} — grid would overflow the canvas",
            entities_list.len(),
            ER_MAX_ENTITIES
        ));
    }
    let mut entities: Vec<ErEntity> = Vec::with_capacity(entities_list.len());
    let mut entity_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in entities_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_er: entities[{}] must be Struct {{name, fields}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let name = struct_string_field("diagram_er entity", f, "name")?;
        if !entity_names.insert(name.clone()) {
            return Err(format!(
                "diagram_er: duplicate entity name {:?} at entities[{}]",
                name, i
            ));
        }
        let fields_list = match f.get("fields") {
            Some(Value::List(items)) => items,
            Some(other) => {
                return Err(format!(
                    "diagram_er: entities[{}].fields must be List<String>, got {}",
                    i,
                    other.type_name()
                ));
            }
            None => {
                return Err(format!(
                    "diagram_er: entities[{}] missing required 'fields' field",
                    i
                ));
            }
        };
        if fields_list.len() > ER_MAX_FIELDS {
            return Err(format!(
                "diagram_er: entities[{}].fields has {} entries, maximum is {} — box would be too tall",
                i,
                fields_list.len(),
                ER_MAX_FIELDS
            ));
        }
        let mut fields_vec: Vec<String> = Vec::with_capacity(fields_list.len());
        for (j, f_item) in fields_list.iter().enumerate() {
            let field_name = match f_item {
                Value::String(s) => s.clone(),
                other => {
                    return Err(format!(
                        "diagram_er: entities[{}].fields[{}] must be String, got {}",
                        i,
                        j,
                        other.type_name()
                    ));
                }
            };
            fields_vec.push(field_name);
        }
        entities.push(ErEntity {
            name,
            fields: fields_vec,
        });
    }
    let mut relations: Vec<ErRelation> = Vec::with_capacity(relations_list.len());
    for (i, item) in relations_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_er: relations[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_er relation", f, "from")?;
        let to = struct_string_field("diagram_er relation", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        if !entity_names.contains(&from) {
            return Err(format!(
                "diagram_er: relations[{}].from references unknown entity {:?}",
                i, from
            ));
        }
        if !entity_names.contains(&to) {
            return Err(format!(
                "diagram_er: relations[{}].to references unknown entity {:?}",
                i, to
            ));
        }
        relations.push(ErRelation { from, to, label });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    // Simple grid: ER_PER_ROW entities per row, fixed box width.
    let n = entities.len();
    let n_rows = n.div_ceil(ER_PER_ROW).max(1);
    // Compute box heights (depend on field count per entity).
    let box_height =
        |e: &ErEntity| ER_BOX_HEADER_H + (e.fields.len() as f64) * ER_FIELD_H + ER_BOX_PADDING;
    let row_h: Vec<f64> = (0..n_rows)
        .map(|r| {
            (0..ER_PER_ROW)
                .filter_map(|c| {
                    let idx = r * ER_PER_ROW + c;
                    if idx < n {
                        Some(box_height(&entities[idx]))
                    } else {
                        None
                    }
                })
                .fold(0.0_f64, f64::max)
        })
        .collect();
    let total_grid_h: f64 = row_h.iter().sum::<f64>() + (n_rows as f64 - 1.0) * ER_GRID_GAP_Y;
    // Center the grid vertically.
    let grid_top = ((canvas_h - total_grid_h) / 2.0).max(ER_GRID_TOP);
    // Position each entity box on the grid.
    let mut name_to_box: std::collections::HashMap<String, (f64, f64, f64, f64)> =
        std::collections::HashMap::new();
    let mut cursor_y = grid_top;
    let total_row_w = (ER_PER_ROW as f64) * ER_BOX_W + ((ER_PER_ROW as f64) - 1.0) * ER_GRID_GAP_X;
    let grid_left = ((canvas_w - total_row_w) / 2.0).max(ER_GRID_GAP_X);
    for (r, row_max_h) in row_h.iter().enumerate() {
        for c in 0..ER_PER_ROW {
            let idx = r * ER_PER_ROW + c;
            if idx >= n {
                break;
            }
            let x = grid_left + (c as f64) * (ER_BOX_W + ER_GRID_GAP_X);
            let h = box_height(&entities[idx]);
            // Vertically center each box in its row cell.
            let y = cursor_y + ((*row_max_h) - h) / 2.0;
            name_to_box.insert(entities[idx].name.clone(), (x, y, ER_BOX_W, h));
        }
        cursor_y += *row_max_h + ER_GRID_GAP_Y;
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Relations first (so entity boxes render on top of any clipped line).
    for rel in &relations {
        let (fx, fy, fw, fh) = name_to_box.get(&rel.from).cloned().ok_or_else(|| {
            format!(
                "diagram_er: internal error — entity {:?} not in position map",
                rel.from
            )
        })?;
        let (tx, ty, tw, th) = name_to_box.get(&rel.to).cloned().ok_or_else(|| {
            format!(
                "diagram_er: internal error — entity {:?} not in position map",
                rel.to
            )
        })?;
        // Use box centers as connector endpoints; box_edge_point trims
        // the line back to the actual box boundary.
        let from_cx = fx + fw / 2.0;
        let from_cy = fy + fh / 2.0;
        let to_cx = tx + tw / 2.0;
        let to_cy = ty + th / 2.0;
        let (sx, sy) = box_edge_point(from_cx, from_cy, to_cx, to_cy, fw, fh);
        let (ex, ey) = box_edge_point(to_cx, to_cy, from_cx, from_cy, tw, th);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &rel.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 6.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // Entity boxes
    for entity in &entities {
        let (x, y, w, h) = name_to_box.get(&entity.name).cloned().ok_or_else(|| {
            format!(
                "diagram_er: internal error — entity {:?} not in position map (render pass)",
                entity.name
            )
        })?;
        // Box body (paper fill, rule stroke).
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(w),
            fmt_num(h),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        // Header bar (accent fill, paper text) — visually separates name from fields.
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(w),
            fmt_num(ER_BOX_HEADER_H),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x + w / 2.0),
            fmt_num(y + ER_BOX_HEADER_H - 6.0),
            escape_attr(&paper),
            escape_html_chars(&entity.name)
        ));
        // Fields listed below the header, one per line.
        for (i, field) in entity.fields.iter().enumerate() {
            let fy = y + ER_BOX_HEADER_H + (i as f64 + 1.0) * ER_FIELD_H;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="11" fill="{}">{}</text>"#,
                fmt_num(x + 8.0),
                fmt_num(fy),
                escape_attr(&ink),
                escape_html_chars(field)
            ));
        }
    }
    // Canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 2: diagram_state ──────────────────────────────────────────
//
// State machine diagram. States are rendered as rounded rectangles
// (radius 14 — visually distinct from flowchart's 4 to signal "this is
// a state, not a step"). Transitions use draw_connector; self-loops
// (A→A) are VALID here (a common state-machine construct: a state that
// transitions to itself on a specific event) — unlike diagram_flowchart
// where a self-loop is rejected as a trivial cycle.
//
// Layout: BFS from the `initial` state (or first state if `initial` not
// specified), treating edges as undirected for layering — this lets
// cyclic state machines lay out sanely. See bfs_layers_with_cycles
// (Block 4) for the algorithm.
//
// If `initial` is specified, we draw a small "entry arrow" — a short
// arrow with no source, terminating at the initial state's left edge.
// This is the classical state-machine notation for "the start state".
//
// Limits: states.len() ≤ 10.

const STATE_MAX_STATES: usize = 10;
const STATE_NODE_W: f64 = 110.0;
const STATE_NODE_H: f64 = 40.0;
const STATE_NODE_RX: f64 = 14.0;

struct StateTransition {
    from: String,
    to: String,
    label: Option<String>,
}

/// `diagram_state(data, style) -> String`
///
/// `data` is `Struct { states: List<String>, transitions: List<Struct{from, to, label?}>, initial: String? }`.
///
/// Cycles and self-loops in transitions are VALID (state machines are
/// inherently cyclic). `initial`, if specified, must be one of `states`.
pub fn builtin_diagram_state(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_state: data must be Struct {{states, transitions, initial?}}, got {}",
                other.type_name()
            ));
        }
    };
    let states_val = fields
        .get("states")
        .ok_or_else(|| "diagram_state: missing 'states' field".to_string())?;
    let transitions_val = fields
        .get("transitions")
        .ok_or_else(|| "diagram_state: missing 'transitions' field".to_string())?;
    let states_list = match states_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_state: 'states' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };
    let transitions_list = match transitions_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_state: 'transitions' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if states_list.is_empty() {
        return Err("diagram_state: states list must not be empty".to_string());
    }
    if states_list.len() > STATE_MAX_STATES {
        return Err(format!(
            "diagram_state: too many states ({}), maximum is {} — diagram would be unreadable",
            states_list.len(),
            STATE_MAX_STATES
        ));
    }
    let mut states: Vec<String> = Vec::with_capacity(states_list.len());
    let mut state_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in states_list.iter().enumerate() {
        let name = match item {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "diagram_state: states[{}] must be String, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if !state_set.insert(name.clone()) {
            return Err(format!(
                "diagram_state: duplicate state name {:?} at states[{}]",
                name, i
            ));
        }
        states.push(name);
    }
    let mut transitions: Vec<StateTransition> = Vec::with_capacity(transitions_list.len());
    for (i, item) in transitions_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_state: transitions[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_state transition", f, "from")?;
        let to = struct_string_field("diagram_state transition", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        if !state_set.contains(&from) {
            return Err(format!(
                "diagram_state: transitions[{}].from references unknown state {:?}",
                i, from
            ));
        }
        if !state_set.contains(&to) {
            return Err(format!(
                "diagram_state: transitions[{}].to references unknown state {:?}",
                i, to
            ));
        }
        transitions.push(StateTransition { from, to, label });
    }
    let initial = struct_opt_string_field(fields, "initial");
    if let Some(ref init) = initial {
        if !state_set.contains(init) {
            return Err(format!(
                "diagram_state: initial {:?} is not in states list",
                init
            ));
        }
    }
    let root = initial
        .clone()
        .unwrap_or_else(|| states.first().cloned().unwrap_or_default());
    // Build edge pairs for the layering function.
    let edge_pairs: Vec<(String, String)> = transitions
        .iter()
        .map(|t| (t.from.clone(), t.to.clone()))
        .collect();
    let layers = bfs_layers_with_cycles(&states, &edge_pairs, &root);

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n_layers = layers.len();
    let layer_h = (canvas_h - 80.0) / (n_layers as f64).max(1.0);
    let mut id_to_pos: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::new();
    for (layer_idx, layer_states) in layers.iter().enumerate() {
        let count = layer_states.len();
        let y_center = 40.0 + (layer_idx as f64 + 0.5) * layer_h;
        let total_w = canvas_w - 80.0;
        let step = if count > 1 {
            total_w / (count as f64 - 1.0)
        } else {
            0.0
        };
        let start_x = if count > 1 { 40.0 } else { canvas_w / 2.0 };
        for (i, id) in layer_states.iter().enumerate() {
            let x_center = start_x + (i as f64) * step;
            id_to_pos.insert(id.clone(), (x_center, y_center));
        }
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Entry arrow for `initial` — short horizontal arrow ending at the
    // state's left edge, with no source (visually "from nowhere").
    if let Some(ref init) = initial {
        if let Some(&(cx, cy)) = id_to_pos.get(init) {
            let ex = cx - STATE_NODE_W / 2.0;
            let sx = ex - 30.0;
            parts.push(draw_connector(sx, cy, ex, cy, &style));
        }
    }
    // Transitions — including self-loops (A→A), which we render as a
    // small curved arrow above the state box. Self-loops are VALID in
    // state machines (unlike flowchart where they're a hard error).
    for t in &transitions {
        if t.from == t.to {
            // Self-loop: small loop above the node.
            if let Some(&(cx, cy)) = id_to_pos.get(&t.from) {
                let top_y = cy - STATE_NODE_H / 2.0;
                let loop_r = 12.0;
                let arc_cx = cx;
                let arc_cy = top_y - loop_r;
                // Half-circle path from left base to right base, drawn
                // ABOVE the node. Arrowhead points down at the right base.
                parts.push(format!(
                    r#"<path d="M {} {} A {} {} 0 0 1 {} {}" fill="none" stroke="{}" stroke-width="1.5" />"#,
                    fmt_num(cx - loop_r),
                    fmt_num(top_y),
                    fmt_num(loop_r),
                    fmt_num(loop_r),
                    fmt_num(cx + loop_r),
                    fmt_num(top_y),
                    escape_attr(&rule)
                ));
                // Arrowhead at the right base, pointing down into the node.
                parts.push(format!(
                    r#"<path d="M {} {} L {} {} L {} {} Z" fill="{}" stroke="none" />"#,
                    fmt_num(cx + loop_r),
                    fmt_num(top_y),
                    fmt_num(cx + loop_r - 4.0),
                    fmt_num(top_y - 6.0),
                    fmt_num(cx + loop_r + 4.0),
                    fmt_num(top_y - 6.0),
                    escape_attr(&rule)
                ));
                if let Some(label) = &t.label {
                    parts.push(format!(
                        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                        fmt_num(arc_cx),
                        fmt_num(arc_cy - 4.0),
                        escape_attr(&muted),
                        escape_html_chars(label)
                    ));
                }
            }
            continue;
        }
        let (from_x, from_y) = id_to_pos.get(&t.from).cloned().ok_or_else(|| {
            format!(
                "diagram_state: internal error — state {:?} not in position map",
                t.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&t.to).cloned().ok_or_else(|| {
            format!(
                "diagram_state: internal error — state {:?} not in position map",
                t.to
            )
        })?;
        let (sx, sy) = box_edge_point(from_x, from_y, to_x, to_y, STATE_NODE_W, STATE_NODE_H);
        let (ex, ey) = box_edge_point(to_x, to_y, from_x, from_y, STATE_NODE_W, STATE_NODE_H);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &t.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 6.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // State boxes (rounded rects, larger rx than flowchart for visual distinction).
    for (id, (cx, cy)) in &id_to_pos {
        let is_initial = initial.as_deref() == Some(id.as_str());
        let box_x = cx - STATE_NODE_W / 2.0;
        let box_y = cy - STATE_NODE_H / 2.0;
        // Initial state gets an accent border + bolder outline (visual emphasis).
        let stroke = if is_initial { &accent } else { &rule };
        let stroke_w = if is_initial { 2.5 } else { 1.5 };
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(STATE_NODE_W),
            fmt_num(STATE_NODE_H),
            fmt_num(STATE_NODE_RX),
            fmt_num(STATE_NODE_RX),
            escape_attr(&paper),
            escape_attr(stroke),
            fmt_num(stroke_w)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(id)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 3: diagram_swimlane ───────────────────────────────────────
//
// Swimlane diagram. Lanes are horizontal bands stacked vertically
// (similar to diagram_layers from Н81, but here each band contains
// positioned steps rather than a single label). Each lane has a name
// label on the left; steps within a lane are positioned horizontally
// by their `order` field (a Float — NOT their list index), so steps
// in different lanes can be visually aligned by time/order.
//
// `order` values across all steps are normalized to [0, 1] for x
// positioning: min_order → x=left_padding, max_order → x=right_padding.
// Steps in the same lane at the same order would overlap — we don't
// de-duplicate, the caller is responsible for sensible input.
//
// Optional connectors between consecutive-by-order steps in the same
// lane: if two steps share a lane and have consecutive order values,
// we draw a faint dashed arrow between them. This makes the temporal
// flow visible without cluttering cross-lane relationships.
//
// Limits: lanes.len() ≤ 6, steps.len() ≤ 30.

const SWIMLANE_MAX_LANES: usize = 6;
const SWIMLANE_MAX_STEPS: usize = 30;
const SWIMLANE_LABEL_W: f64 = 80.0;
const SWIMLANE_PAD_X: f64 = 16.0;
const SWIMLANE_PAD_Y: f64 = 16.0;

struct SwimlaneStep {
    lane: String,
    label: String,
    order: f64,
}

/// `diagram_swimlane(data, style) -> String`
///
/// `data` is `Struct { lanes: List<String>, steps: List<Struct{lane, label, order}> }`.
/// `order` is a Float — NOT the list index — that determines horizontal
/// position (so steps in different lanes can be aligned by time).
pub fn builtin_diagram_swimlane(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_swimlane: data must be Struct {{lanes, steps}}, got {}",
                other.type_name()
            ));
        }
    };
    let lanes_val = fields
        .get("lanes")
        .ok_or_else(|| "diagram_swimlane: missing 'lanes' field".to_string())?;
    let steps_val = fields
        .get("steps")
        .ok_or_else(|| "diagram_swimlane: missing 'steps' field".to_string())?;
    let lanes_list = match lanes_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_swimlane: 'lanes' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };
    let steps_list = match steps_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_swimlane: 'steps' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if lanes_list.is_empty() {
        return Err("diagram_swimlane: lanes list must not be empty".to_string());
    }
    if lanes_list.len() > SWIMLANE_MAX_LANES {
        return Err(format!(
            "diagram_swimlane: too many lanes ({}), maximum is {} — lanes would be too narrow",
            lanes_list.len(),
            SWIMLANE_MAX_LANES
        ));
    }
    if steps_list.len() > SWIMLANE_MAX_STEPS {
        return Err(format!(
            "diagram_swimlane: too many steps ({}), maximum is {}",
            steps_list.len(),
            SWIMLANE_MAX_STEPS
        ));
    }
    let mut lanes: Vec<String> = Vec::with_capacity(lanes_list.len());
    let mut lane_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in lanes_list.iter().enumerate() {
        let name = match item {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "diagram_swimlane: lanes[{}] must be String, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if !lane_set.insert(name.clone()) {
            return Err(format!(
                "diagram_swimlane: duplicate lane name {:?} at lanes[{}]",
                name, i
            ));
        }
        lanes.push(name);
    }
    let mut steps: Vec<SwimlaneStep> = Vec::with_capacity(steps_list.len());
    for (i, item) in steps_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_swimlane: steps[{}] must be Struct {{lane, label, order}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let lane = struct_string_field("diagram_swimlane step", f, "lane")?;
        let label = struct_string_field("diagram_swimlane step", f, "label")?;
        let order = struct_float_field("diagram_swimlane step", f, "order")?;
        if !lane_set.contains(&lane) {
            return Err(format!(
                "diagram_swimlane: steps[{}].lane {:?} is not in lanes list",
                i, lane
            ));
        }
        steps.push(SwimlaneStep { lane, label, order });
    }
    // Compute order range for normalization. If all orders are equal
    // (degenerate case), place everything at the left padding.
    let (min_order, max_order) = steps
        .iter()
        .map(|s| s.order)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), o| {
            (mn.min(o), mx.max(o))
        });
    let order_range = (max_order - min_order).max(1e-9);

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n_lanes = lanes.len();
    let lane_h = (canvas_h - 2.0 * SWIMLANE_PAD_Y) / (n_lanes as f64);
    let step_area_x = SWIMLANE_LABEL_W + SWIMLANE_PAD_X;
    let step_area_w = canvas_w - step_area_x - SWIMLANE_PAD_X;

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Lane bands with labels (left column).
    for (i, lane_name) in lanes.iter().enumerate() {
        let y = SWIMLANE_PAD_Y + (i as f64) * lane_h;
        // Alternating tint for readability.
        if i % 2 == 1 {
            parts.push(format!(
                r#"<rect x="0" y="{}" width="{}" height="{}" fill="{}" opacity="0.18" />"#,
                fmt_num(y),
                fmt_num(canvas_w),
                fmt_num(lane_h),
                escape_attr(&rule)
            ));
        }
        // Lane separator line (top).
        parts.push(format!(
            r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(y),
            fmt_num(canvas_w),
            fmt_num(y),
            escape_attr(&rule)
        ));
        // Left label column — accent strip + lane name.
        parts.push(format!(
            r#"<rect x="0" y="{}" width="3" height="{}" fill="{}" />"#,
            fmt_num(y),
            fmt_num(lane_h),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}">{}</text>"#,
            fmt_num(12.0),
            fmt_num(y + lane_h / 2.0 + 4.0),
            escape_attr(&ink),
            escape_html_chars(lane_name)
        ));
        // Vertical separator between label column and step area.
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(SWIMLANE_LABEL_W),
            fmt_num(y),
            fmt_num(SWIMLANE_LABEL_W),
            fmt_num(y + lane_h),
            escape_attr(&rule)
        ));
    }
    // Bottom separator.
    let bottom_y = SWIMLANE_PAD_Y + (n_lanes as f64) * lane_h;
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(bottom_y),
        fmt_num(canvas_w),
        fmt_num(bottom_y),
        escape_attr(&rule)
    ));
    // Compute step positions per lane, sorted by order — for connectors.
    let mut lane_to_steps: std::collections::HashMap<String, Vec<(f64, String)>> =
        std::collections::HashMap::new();
    for step in &steps {
        let norm_x = (step.order - min_order) / order_range;
        let x = step_area_x + norm_x * step_area_w;
        let lane_idx = lanes.iter().position(|l| l == &step.lane).ok_or_else(|| {
            format!(
                "diagram_swimlane: internal error — lane {:?} not found",
                step.lane
            )
        })?;
        let y_center = SWIMLANE_PAD_Y + (lane_idx as f64 + 0.5) * lane_h;
        // Step pill — paper fill, accent border, rounded.
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="6" ry="6" fill="{}" stroke="{}" stroke-width="1.2" />"#,
            fmt_num(x - 35.0),
            fmt_num(y_center - 12.0),
            fmt_num(70.0),
            fmt_num(24.0),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(y_center + 4.0),
            escape_attr(&ink),
            escape_html_chars(&step.label)
        ));
        lane_to_steps
            .entry(step.lane.clone())
            .or_default()
            .push((x, step.label.clone()));
    }
    // Optional: connect consecutive-by-order steps in the same lane with
    // a faint dashed arrow (visualizes temporal flow within a lane).
    for (_lane, mut lane_steps) in lane_to_steps {
        if lane_steps.len() < 2 {
            continue;
        }
        lane_steps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for window in lane_steps.windows(2) {
            let (x1, _) = window[0];
            let (x2, _) = window[1];
            // Faint dashed line — NOT a draw_connector (we want it lighter
            // than the cross-lane relationships).
            parts.push(format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.8" stroke-dasharray="3 2" opacity="0.5" />"#,
                fmt_num(x1 + 35.0),
                fmt_num(0.0), // y set per lane below
                fmt_num(x2 - 35.0),
                fmt_num(0.0),
                escape_attr(&muted)
            ));
        }
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 5/6 shared: extract_graph ─────────────────────────────────
//
// All three of diagram_data_flow / diagram_high_level / diagram_architecture
// share the same `Struct{nodes, edges}` shape. The only difference is
// whether the `icon` field is allowed on nodes (architecture only).
// We parse all three into a common (GraphNode, GraphEdge) representation
// and dispatch to the appropriate layer function at the call site.

struct GraphNode {
    id: String,
    label: String,
    icon: Option<String>,
}

struct GraphEdge {
    from: String,
    to: String,
    label: Option<String>,
}

/// Parse `Struct{nodes: [{id, label, icon?}], edges: [{from, to, label?}]}`
/// into (Vec<GraphNode>, Vec<GraphEdge>). `allow_icon` controls whether
/// the `icon` field is read on each node — diagram_architecture passes
/// true; data_flow and high_level pass false (the field is silently
/// ignored if present, matching the spec's "icon not used" wording).
fn extract_graph(
    data_value: &Value,
    fn_name: &str,
    allow_icon: bool,
) -> Result<(Vec<GraphNode>, Vec<GraphEdge>), String> {
    let fields = match data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "{}: data must be Struct {{nodes, edges}}, got {}",
                fn_name,
                other.type_name()
            ));
        }
    };
    let nodes_val = fields
        .get("nodes")
        .ok_or_else(|| format!("{}: missing 'nodes' field", fn_name))?;
    let edges_val = fields
        .get("edges")
        .ok_or_else(|| format!("{}: missing 'edges' field", fn_name))?;
    let nodes_list = match nodes_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "{}: 'nodes' must be List, got {}",
                fn_name,
                other.type_name()
            ));
        }
    };
    let edges_list = match edges_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "{}: 'edges' must be List, got {}",
                fn_name,
                other.type_name()
            ));
        }
    };
    if nodes_list.is_empty() {
        return Err(format!("{}: nodes list must not be empty", fn_name));
    }
    if nodes_list.len() > FLOWCHART_MAX_NODES {
        return Err(format!(
            "{}: too many nodes ({}), maximum is {}",
            fn_name,
            nodes_list.len(),
            FLOWCHART_MAX_NODES
        ));
    }
    let mut nodes: Vec<GraphNode> = Vec::with_capacity(nodes_list.len());
    let mut node_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in nodes_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "{}: nodes[{}] must be Struct {{id, label{}}}, got {}",
                    fn_name,
                    i,
                    if allow_icon { ", icon?" } else { "" },
                    other.type_name()
                ));
            }
        };
        let id = struct_string_field(&format!("{} node", fn_name), f, "id")?;
        let label = struct_string_field(&format!("{} node", fn_name), f, "label")?;
        if !node_ids.insert(id.clone()) {
            return Err(format!(
                "{}: duplicate node id {:?} at nodes[{}]",
                fn_name, id, i
            ));
        }
        let icon = if allow_icon {
            let icon_name = struct_opt_string_field(f, "icon");
            // Validate icon name eagerly so we fail before doing layout work.
            if let Some(ref name) = icon_name {
                if icon_path_data(name).is_none() {
                    return Err(format!(
                        "{}: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                        fn_name, name
                    ));
                }
            }
            icon_name
        } else {
            None
        };
        nodes.push(GraphNode { id, label, icon });
    }
    let mut edges: Vec<GraphEdge> = Vec::with_capacity(edges_list.len());
    for (i, item) in edges_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "{}: edges[{}] must be Struct {{from, to, label?}}, got {}",
                    fn_name,
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field(&format!("{} edge", fn_name), f, "from")?;
        let to = struct_string_field(&format!("{} edge", fn_name), f, "to")?;
        let label = struct_opt_string_field(f, "label");
        if !node_ids.contains(&from) {
            return Err(format!(
                "{}: edges[{}].from references unknown node {:?}",
                fn_name, i, from
            ));
        }
        if !node_ids.contains(&to) {
            return Err(format!(
                "{}: edges[{}].to references unknown node {:?}",
                fn_name, i, to
            ));
        }
        edges.push(GraphEdge { from, to, label });
    }
    Ok((nodes, edges))
}

/// Compute (x, y) center positions for each node ID, given a layering.
/// Shared by diagram_data_flow / high_level / architecture. The layering
/// function (topological_layers or bfs_layers_with_cycles) is chosen by
/// the caller; this helper just places nodes on the canvas.
fn layout_layered_nodes(
    layers: &[Vec<String>],
    canvas_w: f64,
    canvas_h: f64,
) -> std::collections::HashMap<String, (f64, f64)> {
    let n_layers = layers.len();
    let layer_h = (canvas_h - 80.0) / (n_layers as f64).max(1.0);
    let mut id_to_pos: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::new();
    for (layer_idx, layer_nodes) in layers.iter().enumerate() {
        let count = layer_nodes.len();
        let y_center = 40.0 + (layer_idx as f64 + 0.5) * layer_h;
        let total_w = canvas_w - 80.0;
        let step = if count > 1 {
            total_w / (count as f64 - 1.0)
        } else {
            0.0
        };
        let start_x = if count > 1 { 40.0 } else { canvas_w / 2.0 };
        for (i, id) in layer_nodes.iter().enumerate() {
            let x_center = start_x + (i as f64) * step;
            id_to_pos.insert(id.clone(), (x_center, y_center));
        }
    }
    id_to_pos
}

// ── Block 5: diagram_data_flow ──────────────────────────────────────
//
// Same data shape as diagram_flowchart (Struct{nodes, edges}), but:
//   - Cycles are VALID (data flows have feedback loops). Uses
//     bfs_layers_with_cycles instead of topological_layers.
//   - Root for the BFS is the first node in `nodes` (data_flow has no
//     `initial` field, unlike state — pick the first listed node).
//   - Nodes are plain rectangles (no decision-shape semantics, unlike
//     flowchart's diamond conventions).
//
// Limits: same as flowchart (FLOWCHART_MAX_NODES = 25).

const DATAFLOW_NODE_W: f64 = 110.0;
const DATAFLOW_NODE_H: f64 = 44.0;

/// `diagram_data_flow(data, style) -> String`
///
/// `data` is `Struct { nodes: List<{id, label}>, edges: List<{from, to, label?}> }`.
/// Cycles in edges are VALID (data may circulate, feedback loops).
pub fn builtin_diagram_data_flow(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_graph(&data_value, "diagram_data_flow", false)?;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    // Pick the first node as BFS root — data_flow has no `initial` field.
    let root = node_ids
        .first()
        .cloned()
        .ok_or_else(|| "diagram_data_flow: nodes list is empty".to_string())?;
    let layers = bfs_layers_with_cycles(&node_ids, &edge_pairs, &root);

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let id_to_pos = layout_layered_nodes(&layers, canvas_w, canvas_h);
    let mut id_to_label: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for n in &nodes {
        id_to_label.insert(n.id.clone(), n.label.as_str());
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Edges first
    for edge in &edges {
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_data_flow: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_data_flow: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        let (sx, sy) = box_edge_point(from_x, from_y, to_x, to_y, DATAFLOW_NODE_W, DATAFLOW_NODE_H);
        let (ex, ey) = box_edge_point(to_x, to_y, from_x, from_y, DATAFLOW_NODE_W, DATAFLOW_NODE_H);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // Nodes
    for (id, (cx, cy)) in &id_to_pos {
        let label = id_to_label.get(id).copied().unwrap_or("");
        let box_x = cx - DATAFLOW_NODE_W / 2.0;
        let box_y = cy - DATAFLOW_NODE_H / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(DATAFLOW_NODE_W),
            fmt_num(DATAFLOW_NODE_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Block 6: diagram_high_level + diagram_architecture ──────────────
//
// Two SEPARATE public APIs (per the spec: "Реализовать как две отдельные
// функции, не одну с параметром «режим»"). They share the same data
// shape and the same internal extract_graph + topological_layers
// pipeline; they differ in node rendering:
//   - high_level: large labeled blocks (no icons), bolder visual
//   - architecture: same blocks + svg_icon when an `icon` field is
//     specified on a node
//
// Both REJECT cycles (architectural diagrams should be acyclic — a
// cycle here is treated as an input error, same as flowchart). If real-
// world usage shows legitimate bidirectional architecture (e.g. service
// pairs that call each other), we'd revisit this; for now, the spec
// says: "архитектурные схемы обычно ациклические, цикл здесь скорее
// ошибка входных данных, как в diagram_flowchart."

const HIGHLEVEL_NODE_W: f64 = 130.0;
const HIGHLEVEL_NODE_H: f64 = 56.0;
const ARCH_ICON_SIZE: f64 = 20.0;

/// `diagram_high_level(data, style) -> String`
///
/// `data` is `Struct { nodes: List<{id, label}>, edges: List<{from, to, label?}> }`.
/// Cycles → Err (architectural diagrams should be acyclic). Uses
/// topological_layers (same as diagram_flowchart) for layout.
pub fn builtin_diagram_high_level(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_graph(&data_value, "diagram_high_level", false)?;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    // High-level diagrams are acyclic — propagate the cycle error from
    // topological_layers (returns Err with "flowchart contains a cycle: ..."
    // — the message is generic enough; we don't override it).
    let layers = topological_layers(&node_ids, &edge_pairs)?;

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let id_to_pos = layout_layered_nodes(&layers, canvas_w, canvas_h);
    let mut id_to_label: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for n in &nodes {
        id_to_label.insert(n.id.clone(), n.label.as_str());
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    for edge in &edges {
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_high_level: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_high_level: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        let (sx, sy) = box_edge_point(
            from_x,
            from_y,
            to_x,
            to_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        let (ex, ey) = box_edge_point(
            to_x,
            to_y,
            from_x,
            from_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    for (id, (cx, cy)) in &id_to_pos {
        let label = id_to_label.get(id).copied().unwrap_or("");
        let box_x = cx - HIGHLEVEL_NODE_W / 2.0;
        let box_y = cy - HIGHLEVEL_NODE_H / 2.0;
        // Larger, bolder block than flowchart/data_flow — visually signals
        // "high-level architectural component".
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="2" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(HIGHLEVEL_NODE_W),
            fmt_num(HIGHLEVEL_NODE_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

/// `diagram_architecture(data, style) -> String`
///
/// `data` is `Struct { nodes: List<{id, label, icon?}>, edges: List<{from, to, label?}> }`.
/// Same as diagram_high_level, but each node MAY specify an `icon`
/// (validated against the 10 svg_icon names — same delegation pattern
/// as diagram_medallion). When icon is present, it renders inside the
/// node box to the left of the label; when absent, the node looks
/// identical to a high_level block.
///
/// Cycles → Err (architectural diagrams should be acyclic).
pub fn builtin_diagram_architecture(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_graph(&data_value, "diagram_architecture", true)?;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    let layers = topological_layers(&node_ids, &edge_pairs)?;

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let id_to_pos = layout_layered_nodes(&layers, canvas_w, canvas_h);
    let mut id_to_node: std::collections::HashMap<String, &GraphNode> =
        std::collections::HashMap::new();
    for n in &nodes {
        id_to_node.insert(n.id.clone(), n);
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    for edge in &edges {
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_architecture: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_architecture: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        let (sx, sy) = box_edge_point(
            from_x,
            from_y,
            to_x,
            to_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        let (ex, ey) = box_edge_point(
            to_x,
            to_y,
            from_x,
            from_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    for (id, (cx, cy)) in &id_to_pos {
        let node = id_to_node.get(id).copied().ok_or_else(|| {
            format!(
                "diagram_architecture: internal error — node {:?} not in node map",
                id
            )
        })?;
        let box_x = cx - HIGHLEVEL_NODE_W / 2.0;
        let box_y = cy - HIGHLEVEL_NODE_H / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="2" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(HIGHLEVEL_NODE_W),
            fmt_num(HIGHLEVEL_NODE_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        // Render icon if specified, then shift label right of the icon.
        let label_x = if let Some(ref icon_name) = node.icon {
            let icon_x = box_x + 8.0;
            let icon_y = cy - ARCH_ICON_SIZE / 2.0;
            // Delegate to icon_path_data with proper error propagation
            // (same pattern as builtin_svg_icon line 2879 + diagram_medallion).
            let path_data = icon_path_data(icon_name).ok_or_else(|| {
                format!(
                    "diagram_architecture: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                    icon_name
                )
            })?;
            let scale = ARCH_ICON_SIZE / 24.0;
            parts.push(format!(
                r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 24 24"><g transform="scale({})"><path d="{}" stroke="{}" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></g></svg>"#,
                fmt_num(icon_x),
                fmt_num(icon_y),
                fmt_num(ARCH_ICON_SIZE),
                fmt_num(ARCH_ICON_SIZE),
                fmt_num(scale),
                path_data,
                escape_attr(&ink)
            ));
            cx + ARCH_ICON_SIZE / 2.0
        } else {
            *cx
        };
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(label_x),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(&node.label)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&rule)
    ));
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        body
    )))
}

// ── Internal: escape XML attribute values ────────────────────────────
//
// For attribute values (inside "..."), we must escape: & < > " '
// We reuse escape_html_chars which already handles all 5.
fn escape_attr(s: &str) -> String {
    escape_html_chars(s)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::Value;

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }
    fn f(n: f64) -> Value {
        Value::Float(n)
    }

    #[test]
    fn svg_rect_basic() {
        let out = builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("#eb6c36"), s("none")])
            .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<rect"#));
                assert!(xml.contains(r#"x="10""#));
                assert!(xml.contains(r#"width="100""#));
                assert!(xml.contains(r#"height="50""#));
                assert!(xml.contains(r##"fill="#eb6c36""##));
                assert!(xml.contains(r#"stroke="none""#));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_rect_rejects_zero_dimensions() {
        let r = builtin_svg_rect(&[f(0.0), f(0.0), f(0.0), f(50.0), s("red"), s("none")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_text_escapes_script_tag() {
        let out = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("<script>alert(1)</script>"),
            f(14.0),
            s("#2d3142"),
            s("start"),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                // Critical security invariant: < and > MUST be escaped
                assert!(!xml.contains("<script>"));
                assert!(xml.contains("&lt;script&gt;"));
                assert!(xml.contains("&lt;/script&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_text_escapes_quotes_and_ampersand() {
        let out = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("test \"quoted\" & <tag>"),
            f(14.0),
            s("#000"),
            s("start"),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains("&amp;"));
                assert!(xml.contains("&lt;tag&gt;"));
                assert!(xml.contains("&quot;quoted&quot;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_text_rejects_invalid_anchor() {
        let r = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("hello"),
            f(14.0),
            s("#000"),
            s("center"),
        ]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_canvas_returns_valid_xml_skeleton() {
        let child =
            builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("red"), s("none")]).unwrap();
        let out = builtin_svg_canvas(&[
            f(200.0),
            f(100.0),
            s("0 0 200 100"),
            Value::List(vec![child]),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
                assert!(xml.contains(r#"width="200""#));
                assert!(xml.contains(r#"height="100""#));
                assert!(xml.contains(r#"viewBox="0 0 200 100""#));
                assert!(xml.contains("<rect"));
                assert!(xml.ends_with("</svg>"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_canvas_rejects_invalid_viewbox() {
        let r = builtin_svg_canvas(&[f(200.0), f(100.0), s("0 0 200"), Value::List(vec![])]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_path_rejects_angle_brackets() {
        let r = builtin_svg_path(&[s("M 10 10 <script>"), s("none"), s("black")]);
        assert!(r.is_err());
    }

    #[test]
    fn diagram_style_returns_struct_with_5_tokens() {
        let mut fields = HashMap::new();
        fields.insert("paper".to_string(), s("#f5f5f5"));
        fields.insert("ink".to_string(), s("#2d3142"));
        fields.insert("accent".to_string(), s("#eb6c36"));
        fields.insert("muted".to_string(), s("#4f5d75"));
        fields.insert("rule".to_string(), s("rgba(45,49,66,0.12)"));
        let style_arg = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields,
        };
        let out = builtin_diagram_style(&[style_arg]).unwrap();
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DiagramStyle");
                assert_eq!(fields.len(), 5);
                assert!(fields.contains_key("paper"));
                assert!(fields.contains_key("ink"));
                assert!(fields.contains_key("accent"));
                assert!(fields.contains_key("muted"));
                assert!(fields.contains_key("rule"));
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn diagram_style_rejects_missing_token() {
        let mut fields = HashMap::new();
        fields.insert("paper".to_string(), s("#f5f5f5"));
        fields.insert("ink".to_string(), s("#2d3142"));
        // missing accent, muted, rule
        let style_arg = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields,
        };
        let r = builtin_diagram_style(&[style_arg]);
        assert!(r.is_err());
    }

    #[test]
    fn chart_bar_basic_3_bars() {
        let mut fields1 = HashMap::new();
        fields1.insert("label".to_string(), s("Янв"));
        fields1.insert("value".to_string(), f(40.0));
        let item1 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields1,
        };
        let mut fields2 = HashMap::new();
        fields2.insert("label".to_string(), s("Фев"));
        fields2.insert("value".to_string(), f(65.0));
        let item2 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields2,
        };
        let mut fields3 = HashMap::new();
        fields3.insert("label".to_string(), s("Мар"));
        fields3.insert("value".to_string(), f(30.0));
        let item3 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields3,
        };
        let data = Value::List(vec![item1, item2, item3]);

        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#f5f5f5"));
        style_fields.insert("ink".to_string(), s("#2d3142"));
        style_fields.insert("accent".to_string(), s("#eb6c36"));
        style_fields.insert("muted".to_string(), s("#4f5d75"));
        style_fields.insert("rule".to_string(), s("rgba(45,49,66,0.12)"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };

        let out = builtin_chart_bar(&[data, style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with(r#"<svg "#));
                assert!(xml.ends_with("</svg>"));
                // 3 bars (each contains <rect)
                let rect_count = xml.matches("<rect").count();
                assert!(rect_count >= 4); // 3 bars + 1 background = 4
                                          // Labels present and not escaped (Cyrillic is fine in XML UTF-8)
                assert!(xml.contains("Янв"));
                assert!(xml.contains("Фев"));
                assert!(xml.contains("Мар"));
                // The tallest bar (65) should be accent-colored
                assert!(xml.contains("fill=\"#eb6c36\""));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_bar_rejects_empty_data() {
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_bar(&[Value::List(vec![]), style]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_sketchy_filter_default_params() {
        let out = builtin_svg_sketchy_filter(&[s("sketch1")]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<filter id="sketch1">"#));
                assert!(xml.contains("feTurbulence"));
                assert!(xml.contains("feDisplacementMap"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_sketchy_filter_rejects_bad_id() {
        let r = builtin_svg_sketchy_filter(&[s("id with spaces")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_icon_known_name() {
        let out =
            builtin_svg_icon(&[s("server"), f(10.0), f(10.0), f(24.0), s("currentColor")]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<svg "#));
                assert!(xml.contains(r#"x="10""#));
                assert!(xml.contains(r#"y="10""#));
                assert!(xml.contains(r#"width="24""#));
                assert!(xml.contains(r#"height="24""#));
                assert!(xml.contains(r#"stroke="currentColor""#));
                assert!(xml.contains("<path"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_icon_unknown_name_errors() {
        let r = builtin_svg_icon(&[s("nonexistent"), f(0.0), f(0.0), f(24.0), s("black")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_callout_default_intent() {
        let out = builtin_svg_callout(&[s("note"), f(10.0), f(10.0), f(100.0), f(50.0)]).unwrap();
        match out {
            Value::String(xml) => {
                // Dashed line (callout invariant)
                assert!(xml.contains(r#"stroke-dasharray="3,3""#));
                // Italic text
                assert!(xml.contains(r#"font-style="italic""#));
                // Anchor dot
                assert!(xml.contains("<circle"));
                // Text content
                assert!(xml.contains("note"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_callout_escapes_text() {
        let out =
            builtin_svg_callout(&[s("<b>bold</b>"), f(10.0), f(10.0), f(100.0), f(50.0)]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(!xml.contains("<b>bold</b>"));
                assert!(xml.contains("&lt;b&gt;bold&lt;/b&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    // ── Наряд №77: color_palette + chart_donut unit tests ──

    #[test]
    fn color_palette_returns_diagram_style_struct_with_5_tokens() {
        let out = builtin_color_palette(&[s("energy"), s("light")]).unwrap();
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DiagramStyle");
                assert_eq!(fields.len(), 5);
                for k in &["paper", "ink", "accent", "muted", "rule"] {
                    assert!(fields.contains_key(*k), "missing token {}", k);
                }
                // Each token must be a hex string of form #rrggbb
                for k in &["paper", "ink", "accent", "muted", "rule"] {
                    if let Some(Value::String(v)) = fields.get(*k) {
                        assert!(v.starts_with('#'), "{} should start with #", k);
                        assert_eq!(v.len(), 7, "{} should be #rrggbb (7 chars)", k);
                    }
                }
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn color_palette_rejects_unknown_intent() {
        let r = builtin_color_palette(&[s("unknown"), s("light")]);
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(err.contains("intent"), "err: {}", err);
    }

    #[test]
    fn color_palette_rejects_unknown_mode() {
        let r = builtin_color_palette(&[s("calm"), s("neon")]);
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(err.contains("mode"), "err: {}", err);
    }

    #[test]
    fn color_palette_light_vs_dark_produce_different_tokens() {
        let light = builtin_color_palette(&[s("authority"), s("light")]).unwrap();
        let dark = builtin_color_palette(&[s("authority"), s("dark")]).unwrap();
        if let (Value::Struct { fields: lf, .. }, Value::Struct { fields: df, .. }) = (light, dark)
        {
            // Light paper should be much lighter than dark paper.
            // Value doesn't impl PartialEq, so extract strings and compare those.
            let lp = match lf.get("paper").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("light paper not String"),
            };
            let dp = match df.get("paper").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("dark paper not String"),
            };
            assert_ne!(lp, dp, "light vs dark paper must differ");
            let li = match lf.get("ink").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("light ink not String"),
            };
            let di = match df.get("ink").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("dark ink not String"),
            };
            assert_ne!(li, di, "light vs dark ink must differ");
        }
    }

    #[test]
    fn color_palette_all_5_intents_all_2_modes_produce_valid_hex() {
        for intent in &["calm", "tension", "energy", "authority", "warmth"] {
            for mode in &["light", "dark"] {
                let out = builtin_color_palette(&[s(intent), s(mode)]).unwrap();
                if let Value::Struct { fields, .. } = out {
                    for k in &["paper", "ink", "accent", "muted", "rule"] {
                        if let Some(Value::String(v)) = fields.get(*k) {
                            assert!(
                                v.starts_with('#') && v.len() == 7,
                                "intent={} mode={} token={} got {:?}",
                                intent,
                                mode,
                                k,
                                v
                            );
                            // Hex digits only after #
                            let hex = &v[1..];
                            assert!(
                                hex.chars().all(|c| c.is_ascii_hexdigit()),
                                "non-hex char in {} for intent={} mode={}",
                                k,
                                intent,
                                mode
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn color_palette_result_passes_extract_style() {
        // Critical: color_palette output must be consumable by extract_style
        // (the helper used by chart_bar / chart_donut).
        let out = builtin_color_palette(&[s("warmth"), s("light")]).unwrap();
        let extracted = extract_style(&out);
        assert!(extracted.is_ok(), "extract_style failed: {:?}", extracted);
        let style = extracted.unwrap();
        assert_eq!(style.len(), 5);
        for k in &["paper", "ink", "accent", "muted", "rule"] {
            assert!(style.contains_key(*k));
        }
    }

    #[test]
    fn color_palette_result_works_with_chart_bar() {
        // End-to-end: color_palette → chart_bar (no manual diagram_style needed)
        let palette = builtin_color_palette(&[s("energy"), s("dark")]).unwrap();
        let mut item_fields = HashMap::new();
        item_fields.insert("label".to_string(), s("Q1"));
        item_fields.insert("value".to_string(), f(40.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: item_fields,
        };
        let data = Value::List(vec![item]);
        let out = builtin_chart_bar(&[data, palette]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_basic_3_slices() {
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("Alpha"));
        f1.insert("value".to_string(), f(40.0));
        let item1 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut f2 = HashMap::new();
        f2.insert("label".to_string(), s("Beta"));
        f2.insert("value".to_string(), f(35.0));
        let item2 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f2,
        };
        let mut f3 = HashMap::new();
        f3.insert("label".to_string(), s("Gamma"));
        f3.insert("value".to_string(), f(25.0));
        let item3 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f3,
        };
        let data = Value::List(vec![item1, item2, item3]);

        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#eb6c36"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };

        let out = builtin_chart_donut(&[data, style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
                // 3 slices = 3 <path> elements (each donut slice is one path)
                let path_count = xml.matches("<path").count();
                assert_eq!(path_count, 3, "expected 3 slice paths");
                // Background rect
                assert!(xml.contains("<rect"));
                // Labels present (escaped if needed — Alpha/Beta/Gamma are safe)
                assert!(xml.contains("Alpha"));
                assert!(xml.contains("Beta"));
                assert!(xml.contains("Gamma"));
                // Center total: 40+35+25=100
                assert!(xml.contains(">100<"));
                // Legend swatches: 3 (one per slice)
                let rect_count = xml.matches("<rect").count();
                assert!(
                    rect_count >= 4,
                    "expected 4+ rects (1 bg + 3 legend swatches)"
                );
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_rejects_empty_data() {
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_donut(&[Value::List(vec![]), style]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("empty"));
    }

    #[test]
    fn chart_donut_rejects_negative_value() {
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("A"));
        f1.insert("value".to_string(), f(-10.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_donut(&[Value::List(vec![item]), style]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("non-negative"));
    }

    #[test]
    fn chart_donut_escapes_label_with_script_tag() {
        // Critical security invariant: <script> in label must NOT leak raw
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("<script>alert(1)</script>"));
        f1.insert("value".to_string(), f(40.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item]), style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(
                    !xml.contains("<script>"),
                    "RAW <script> leaked into chart_donut output: {}",
                    xml
                );
                assert!(xml.contains("&lt;script&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_single_slice_uses_accent() {
        // One slice = whole pie = accent color
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("Only"));
        f1.insert("value".to_string(), f(100.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#ff8800"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item]), style]).unwrap();
        match out {
            Value::String(xml) => {
                // The single slice should be filled with accent color
                assert!(
                    xml.contains(r##"fill="#ff8800""##),
                    "single slice should be accent-colored, xml: {}",
                    xml
                );
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_works_with_color_palette_output() {
        // End-to-end: color_palette → chart_donut
        let palette = builtin_color_palette(&[s("calm"), s("light")]).unwrap();
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("A"));
        f1.insert("value".to_string(), f(60.0));
        let item1 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut f2 = HashMap::new();
        f2.insert("label".to_string(), s("B"));
        f2.insert("value".to_string(), f(40.0));
        let item2 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f2,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item1, item2]), palette]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
                assert_eq!(xml.matches("<path").count(), 2, "expected 2 slice paths");
            }
            _ => panic!("expected String"),
        }
    }
}
