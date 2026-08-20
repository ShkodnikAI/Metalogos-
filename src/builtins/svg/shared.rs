//! Shared helpers for SVG / chart / diagram builtins.
//! Visibility: `pub(super)` — visible inside the `svg` module only.

use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;
use std::collections::HashMap;

pub(super) fn expect_struct_arg(
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
pub(super) fn struct_string_field(
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
pub(super) fn struct_float_field(
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
pub(super) fn struct_opt_string_field(fields: &HashMap<String, Value>, key: &str) -> Option<String> {
    match fields.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

/// Extract an optional float field (returns None if missing or Unit).
#[allow(dead_code)] // reserved for future chart_* types (timeline, pyramid, etc.)
pub(super) fn struct_opt_float_field(fields: &HashMap<String, Value>, key: &str) -> Option<f64> {
    match fields.get(key) {
        Some(Value::Float(f)) => Some(*f),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

/// Format a float for SVG output. Trims trailing zeros and unnecessary
/// decimal point for cleaner output. NaN/Inf become "0" (defensive).
pub(super) fn fmt_num(n: f64) -> String {
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

pub(super) fn polar_to_xy(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
    (cx + r * angle.cos(), cy + r * angle.sin())
}

/// Build a list of N slice colors that stay within the same color family
/// (accent + ink, alternating). For N=1, return [accent]. For N>1, alternate
/// accent and ink so adjacent slices have different colors but the whole
/// chart stays within the same hue family (palette.md V2.1).
pub(super) fn hex_to_hsl(hex: &str) -> Option<(f64, f64, f64)> {
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
pub(super) fn interpolate_hsl(c1: (f64, f64, f64), c2: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
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
pub(super) fn hsl_to_hex(h: f64, s: f64, l: f64) -> String {
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
pub(super) fn draw_connector(x1: f64, y1: f64, x2: f64, y2: f64, style: &HashMap<String, Value>) -> String {
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

pub(super) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    0.55 * font_size * (text.chars().count() as f64)
}

/// Axis along which to resolve overlaps.
pub(super) enum Axis {
    /// Push overlapping labels apart vertically (y-direction).
    /// Used by timeline, Gantt, and other horizontal-layout diagrams.
    Vertical,
    /// Push overlapping labels apart radially (along a line from center).
    /// Reserved for radar/loop/venn — not wired in this narad.
    #[allow(dead_code)]
    Radial,
}

/// A rectangular bounding box for a text label, used for overlap detection.
pub(super) struct LabelBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Iteratively resolve pairwise overlaps among `labels` along `axis`.
///
/// Algorithm:
///   for each iteration up to max_iterations:
///     found_overlap = false
///     for each pair (i, j) where i < j:
///       if boxes i and j overlap (AABB intersection):
///         push them apart along the axis by half the overlap each
///         found_overlap = true
///     if !found_overlap: break (stable — no overlaps remain)
///
/// For Vertical axis: if two boxes overlap in both x and y, push the
/// lower one down and the upper one up by half the y-overlap.
/// This is deterministic (same input → same output) because we process
/// pairs in index order and always push symmetrically.
///
/// Returns the number of iterations actually performed (useful for
/// diagnostics and testing).
pub(super) fn resolve_overlaps(labels: &mut [LabelBox], axis: Axis, max_iterations: usize) -> usize {
    let n = labels.len();
    if n < 2 {
        return 0;
    }
    let mut iterations = 0;
    for _ in 0..max_iterations {
        let mut found_overlap = false;
        for i in 0..n {
            for j in (i + 1)..n {
                let (li, lj) = {
                    // Borrow two elements simultaneously
                    let (a, b) = labels.split_at_mut(j);
                    (&mut a[i], &mut b[0])
                };
                // AABB overlap test: boxes overlap iff they overlap
                // on BOTH axes.
                let overlap_x = li.x < lj.x + lj.w && lj.x < li.x + li.w;
                let overlap_y = li.y < lj.y + lj.h && lj.y < li.y + li.h;
                if overlap_x && overlap_y {
                    match axis {
                        Axis::Vertical => {
                            // Push apart vertically: compute y-overlap amount
                            let overlap_top = li.y.max(lj.y);
                            let overlap_bottom = (li.y + li.h).min(lj.y + lj.h);
                            let overlap_amount = overlap_bottom - overlap_top;
                            if overlap_amount > 0.0 {
                                let push = overlap_amount / 2.0 + 1.0; // +1px breathing room
                                if li.y <= lj.y {
                                    li.y -= push;
                                    lj.y += push;
                                } else {
                                    li.y += push;
                                    lj.y -= push;
                                }
                            }
                        }
                        Axis::Radial => {
                            // Radial: push apart vertically (same as Vertical
                            // for MVP — true radial push along the spoke would
                            // need angle information, deferred to future narad).
                            let overlap_top = li.y.max(lj.y);
                            let overlap_bottom = (li.y + li.h).min(lj.y + lj.h);
                            let overlap_amount = overlap_bottom - overlap_top;
                            if overlap_amount > 0.0 {
                                let push = overlap_amount / 2.0 + 1.0;
                                if li.y <= lj.y {
                                    li.y -= push;
                                    lj.y += push;
                                } else {
                                    li.y += push;
                                    lj.y -= push;
                                }
                            }
                        }
                    }
                    found_overlap = true;
                }
            }
        }
        iterations += 1;
        if !found_overlap {
            break;
        }
    }
    iterations
}

const TIMELINE_AXIS_Y: f64 = 200.0; // middle of 400px canvas


pub(super) fn intent_to_hue(intent: &str) -> Option<f64> {
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
pub(super) const TIMELINE_MAX_EVENTS: usize = 12;
pub(super) const TIMELINE_AXIS_Y: f64 = 200.0;
pub(super) const TIMELINE_DOT_R: f64 = 5.0;
pub(super) const TIMELINE_LABEL_OFFSET: f64 = 22.0;

pub(super) fn escape_attr(s: &str) -> String {
    escape_html_chars(s)
}
