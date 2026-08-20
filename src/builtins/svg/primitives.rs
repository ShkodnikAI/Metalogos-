//! SVG primitives, design tokens (diagram_style), icons, callout, generate, canvas_preset, color_palette
//!
//! Category mapping (from registry.rs):
//! - "svg" : most functions here
//! - "tokens" : diagram_style

use super::shared::*;
use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;
use std::collections::HashMap;

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
pub(crate) fn icon_path_data(name: &str) -> Option<&'static str> {
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
