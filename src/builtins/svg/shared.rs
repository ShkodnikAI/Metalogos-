//! Shared helpers for SVG / chart / diagram builtins.
//! Visibility: `pub(super)` — visible inside the `svg` module only.

use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
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
    context: &str,
    fields: &HashMap<String, Value>,
    key: &str,
) -> Result<String, String> {
    match fields.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!(
            "{}: field '{}' must be String, got {}",
            context, key, other.type_name()
        )),
        None => Err(format!("{}: missing required field '{}'", context, key)),
    }
}

pub(super) fn struct_float_field(
    context: &str,
    fields: &HashMap<String, Value>,
    key: &str,
) -> Result<f64, String> {
    match fields.get(key) {
        Some(Value::Float(f)) => Ok(*f),
        Some(other) => Err(format!(
            "{}: field '{}' must be Float, got {}",
            context, key, other.type_name()
        )),
        None => Err(format!("{}: missing required field '{}'", context, key)),
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

pub(super) fn escape_attr(s: &str) -> String {
    escape_html_chars(s)
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

pub(super) fn interpolate_hsl(c1: (f64, f64, f64), c2: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    let (h1, s1, l1) = c1;
    let (h2, s2, l2) = c2;
    let dh = if (h2 - h1).abs() > 180.0 {
        if h2 > h1 { h2 - h1 - 360.0 } else { h2 - h1 + 360.0 }
    } else {
        h2 - h1
    };
    let h = (h1 + t * dh + 360.0) % 360.0;
    let s = s1 + t * (s2 - s1);
    let l = l1 + t * (l2 - l1);
    (h, s, l)
}

pub(super) fn draw_connector(
    x1: f64, y1: f64, x2: f64, y2: f64, style: &HashMap<String, Value>,
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
    let perp_x = (-angle.sin()) * ah_half_w;
    let perp_y = angle.cos() * ah_half_w;
    let left_x = base_x + perp_x;
    let left_y = base_y + perp_y;
    let right_x = base_x - perp_x;
    let right_y = base_y - perp_y;
    format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" />"#,
        fmt_num(x1), fmt_num(y1), fmt_num(line_end_x), fmt_num(line_end_y), escape_attr(&color)
    ) + &format!(
        r#"<path d="M {} {} L {} {} L {} {} Z" fill="{}" stroke="none" />"#,
        fmt_num(x2), fmt_num(y2), fmt_num(left_x), fmt_num(left_y), fmt_num(right_x), fmt_num(right_y), escape_attr(&color)
    )
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

pub(super) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    0.55 * font_size * text.chars().count() as f64
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
                    found_overlap = true;
                    match axis {
                        Axis::Vertical => {
                            let overlap = (li.y + li.h).min(lj.y + lj.h) - li.y.max(lj.y);
                            if li.y < lj.y {
                                li.y -= overlap / 2.0;
                                lj.y += overlap / 2.0;
                            } else {
                                lj.y -= overlap / 2.0;
                                li.y += overlap / 2.0;
                            }
                        }
                        Axis::Radial => {}
                    }
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
