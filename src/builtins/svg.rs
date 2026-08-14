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
