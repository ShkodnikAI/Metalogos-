// Наряд №110: SVG-примитивы + палитра + токены стиля.
use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;
use std::collections::HashMap;

use super::shared::*;

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
