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

#[allow(dead_code)]
pub(super) fn struct_opt_string_field(fields: &HashMap<String, Value>, key: &str) -> Option<String> {
    match fields.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

#[allow(dead_code)]
pub(super) fn struct_opt_float_field(fields: &HashMap<String, Value>, key: &str) -> Option<f64> {
    match fields.get(key) {
        Some(Value::Float(f)) => Some(*f),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

pub(super) fn fmt_num(n: f64) -> String {
    if !n.is_finite() {
        return "0".to_string();
    }
    let rounded = (n * 1000.0).round() / 1000.0;
    if rounded == rounded.trunc() {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{:.3}", rounded);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
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

pub(super) fn polar_to_xy(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
    (cx + r * angle.cos(), cy + r * angle.sin())
}

pub(super) fn hex_to_hsl(hex: &str) -> Option<(f64, f64, f64)> {
    let s = hex.strip_prefix('#')?;
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
        return Some((0.0, 0.0, l));
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f64::EPSILON {
        ((g - b) / d) % 6.0
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    let h = h * 60.0;
    let h = if h < 0.0 { h + 360.0 } else { h };
    Some((h, s, l))
}

pub(super) fn interpolate_hsl(c1: (f64, f64, f64), c2: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    let (h1, s1, l1) = c1;
    let (h2, s2, l2) = c2;
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

pub(super) fn hsl_to_hex(h: f64, s: f64, l: f64) -> String {
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
        let hue2rgb = |p: f64, q: f64, t: f64| -> f64 {
            let mut t = t;
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 1.0 / 2.0 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        };
        (
            hue2rgb(p, q, h_norm + 1.0 / 3.0),
            hue2rgb(p, q, h_norm),
            hue2rgb(p, q, h_norm - 1.0 / 3.0),
        )
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8
    )
}

pub(super) fn draw_connector(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    style: &HashMap<String, Value>,
) -> String {
    let color = style_token(style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    let dx = x2 - x1;
    let dy = y2 - y1;
    let angle = dy.atan2(dx);
    let ah_len = 8.0_f64;
    let ah_half_w = 3.0_f64;
    let line_end_x = x2 - ah_len * angle.cos();
    let line_end_y = y2 - ah_len * angle.sin();
    let base_x = x2 - ah_len * angle.cos();
    let base_y = y2 - ah_len * angle.sin();
    let perp_x = -angle.sin();
    let perp_y = angle.cos();
    let left_x = base_x + perp_x * ah_half_w;
    let left_y = base_y + perp_y * ah_half_w;
    let right_x = base_x - perp_x * ah_half_w;
    let right_y = base_y - perp_y * ah_half_w;
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

pub(super) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    0.55 * font_size * (text.chars().count() as f64)
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Axis {
    Vertical,
    #[allow(dead_code)]
    Radial,
}

#[derive(Clone, Debug)]
pub(super) struct LabelBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub(super) fn resolve_overlaps(
    labels: &mut [LabelBox],
    axis: Axis,
    max_iterations: usize,
) -> usize {
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
                    let (a, b) = labels.split_at_mut(j);
                    (&mut a[i], &mut b[0])
                };
                let overlap_x = li.x < lj.x + lj.w && lj.x < li.x + li.w;
                let overlap_y = li.y < lj.y + lj.h && lj.y < li.y + li.h;
                if overlap_x && overlap_y {
                    match axis {
                        Axis::Vertical | Axis::Radial => {
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

pub(super) fn escape_attr(s: &str) -> String {
    escape_html_chars(s)
}
