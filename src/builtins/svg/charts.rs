//! Chart builtins (chart_*)
//! Category: "chart" (from registry.rs)

use super::shared::*;
use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;
use std::collections::HashMap;

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