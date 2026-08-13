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
        None => Err(format!(
            "{}: missing required field '{}'",
            struct_name, key
        )),
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
        None => Err(format!(
            "{}: missing required field '{}'",
            struct_name, key
        )),
    }
}

/// Extract an optional string field (returns None if missing or Unit).
#[allow(dead_code)] // reserved for future chart_* types (timeline, pyramid, etc.)
fn struct_opt_string_field(
    fields: &HashMap<String, Value>,
    key: &str,
) -> Option<String> {
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
        return Err(
            "svg_path: path data must not contain '<' or '>' characters".to_string(),
        );
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
                    return Err(format!(
                        "DiagramStyle missing required token '{}'",
                        k
                    ));
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
pub(crate) fn style_token(
    style: &HashMap<String, Value>,
    key: &str,
) -> Result<String, String> {
    match style.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("DiagramStyle: token '{}' missing or not String", key)),
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
    if id.is_empty() || id.contains(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == '\'') {
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
        let child = builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("red"), s("none")])
            .unwrap();
        let out = builtin_svg_canvas(&[f(200.0), f(100.0), s("0 0 200 100"), Value::List(vec![child])])
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
        let out = builtin_svg_icon(&[s("server"), f(10.0), f(10.0), f(24.0), s("currentColor")])
            .unwrap();
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
        let out = builtin_svg_callout(&[
            s("note"),
            f(10.0),
            f(10.0),
            f(100.0),
            f(50.0),
        ])
        .unwrap();
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
        let out = builtin_svg_callout(&[
            s("<b>bold</b>"),
            f(10.0),
            f(10.0),
            f(100.0),
            f(50.0),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(!xml.contains("<b>bold</b>"));
                assert!(xml.contains("&lt;b&gt;bold&lt;/b&gt;"));
            }
            _ => panic!("expected String"),
        }
    }
}
