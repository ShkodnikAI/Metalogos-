//! Layout helpers shared across diagram groups.
//!
//! Наряд №169: extracted from former diagrams.rs. No logic changes —
//! these functions were already module-private (`fn`, not `pub fn`)
//! and used by multiple diagram groups. They live here so all groups
//! can import them via `super::layout::*`.
//!
//! Includes:
//! - `box_edge_point` — geometry helper: where does a line from one box
//!   center to another box center hit each box's edge?
//! - color utilities (`parse_hex_color`, `relative_luminance`,
//!   `contrast_ratio`, `rgb_to_hsl`, `extract_svg_colors`) — used by
//!   `builtin_infographic_qa` and reusable for future QA / palette tools.
//! - `count_svg_elements`, `extract_canvas_dimensions` — SVG introspection
//!   used by `builtin_infographic_qa`.
//! - `builtin_infographic_qa` — returns quality-assessment struct for
//!   a rendered SVG (contrast ratio, color count, canvas size).

use crate::builtins::core::expect_string_arg;
use crate::interpreter::Value;
use std::collections::HashMap;

/// Compute the point on a rectangle's edge where a line from the
/// rectangle's center toward (tx, ty) hits the boundary.
///
/// Used by multiple diagram groups (flowchart, loop, er, state,
/// data_flow, high_level, architecture) to trim connector lines
/// so they start/end at the box edge rather than the box center.
pub(super) fn box_edge_point(cx: f64, cy: f64, tx: f64, ty: f64, w: f64, h: f64) -> (f64, f64) {
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
// We reuse escape_html_chars which already handles all 5.
fn parse_hex_color(hex: &str) -> Result<(f64, f64, f64), String> {
    let h = hex.trim_start_matches('#');
    match h.len() {
        3 => {
            let r = u8::from_str_radix(&h[0..1].repeat(2), 16);
            let g = u8::from_str_radix(&h[1..2].repeat(2), 16);
            let b = u8::from_str_radix(&h[2..3].repeat(2), 16);
            match (r, g, b) {
                (Ok(rv), Ok(gv), Ok(bv)) => {
                    Ok((rv as f64 / 255.0, gv as f64 / 255.0, bv as f64 / 255.0))
                }
                _ => Err(format!("infographic_qa: invalid hex color '{}'", hex)),
            }
        }
        6 => {
            let r = u8::from_str_radix(&h[0..2], 16);
            let g = u8::from_str_radix(&h[2..4], 16);
            let b = u8::from_str_radix(&h[4..6], 16);
            match (r, g, b) {
                (Ok(rv), Ok(gv), Ok(bv)) => {
                    Ok((rv as f64 / 255.0, gv as f64 / 255.0, bv as f64 / 255.0))
                }
                _ => Err(format!("infographic_qa: invalid hex color '{}'", hex)),
            }
        }
        _ => Err(format!(
            "infographic_qa: hex color must be #RGB or #RRGGBB, got '{}'",
            hex
        )),
    }
}

/// WCAG 2.0 relative luminance formula.
/// L = 0.2126 * R_lin + 0.7152 * G_lin + 0.0722 * B_lin
/// where channel_lin = channel/12.92 if channel <= 0.03928,
///                       else ((channel + 0.055) / 1.055)^2.4
fn relative_luminance(hex: &str) -> Result<f64, String> {
    let (r, g, b) = parse_hex_color(hex)?;
    let lin = |c: f64| -> f64 {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Ok(0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b))
}

/// WCAG 2.0 contrast ratio between two hex colors.
/// ratio = (L_lighter + 0.05) / (L_darker + 0.05)
fn contrast_ratio(hex1: &str, hex2: &str) -> Result<f64, String> {
    let l1 = relative_luminance(hex1)?;
    let l2 = relative_luminance(hex2)?;
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    Ok((lighter + 0.05) / (darker + 0.05))
}

/// Convert RGB (0.0–1.0) to HSL. Returns (h, s, l) where
/// h in 0–360, s in 0–1, l in 0–1.
fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-10 {
        return (0.0, 0.0, l); // achromatic
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < 1e-10 {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-10 {
        ((b - r) / d) + 2.0
    } else {
        ((r - g) / d) + 4.0
    };

    (h * 60.0, s, l)
}

/// Extract all unique hex colors from fill="..." and stroke="..." attributes
/// in an SVG string. Returns a Vec of hex color strings (lowercase, with #).
fn extract_svg_colors(svg: &str) -> Vec<String> {
    let mut colors = Vec::new();
    // Scan for fill="#XXXXXX" and stroke="#XXXXXX" patterns
    for prefix in &["fill=\"", "stroke=\""] {
        let mut pos = 0;
        while let Some(idx) = svg[pos..].find(prefix) {
            let start = pos + idx + prefix.len();
            if let Some(quote_end) = svg[start..].find('"') {
                let color = &svg[start..start + quote_end];
                // Only accept #RGB or #RRGGBB patterns
                if color.starts_with('#')
                    && (color.len() == 4 || color.len() == 7)
                    && color[1..].chars().all(|c| c.is_ascii_hexdigit())
                {
                    let lower = color.to_lowercase();
                    if !colors.contains(&lower) {
                        colors.push(lower);
                    }
                }
                pos = start + quote_end + 1;
            } else {
                break;
            }
        }
    }
    colors
}

/// Count SVG primitive element tags in a string.
/// Looks for <rect, <circle, <path, <text, <line, <ellipse, <polygon, <polyline.
fn count_svg_elements(svg: &str) -> usize {
    let tags = [
        "<rect",
        "<circle",
        "<path",
        "<text",
        "<line",
        "<ellipse",
        "<polygon",
        "<polyline",
    ];
    let mut count = 0;
    for tag in &tags {
        count += svg.matches(tag).count();
    }
    count
}

/// Extract canvas dimensions from an SVG string.
/// Looks for width="..." height="..." attributes, or falls back to
/// parsing viewBox="minX minY w h".
fn extract_canvas_dimensions(svg: &str) -> (f64, f64) {
    // Try width/height attributes first
    let w = svg
        .split("width=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<f64>().ok());
    let h = svg
        .split("height=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<f64>().ok());

    if let (Some(width), Some(height)) = (w, h) {
        return (width, height);
    }

    // Fallback: parse viewBox
    if let Some(vb) = svg
        .split("viewBox=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
    {
        let parts: Vec<&str> = vb.split_whitespace().collect();
        if parts.len() >= 4 {
            if let (Ok(vw), Ok(vh)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                return (vw, vh);
            }
        }
    }

    // Default fallback: 600×400 (most common chart canvas)
    (600.0, 400.0)
}

/// `infographic_qa(svg_string) -> Struct`
///
/// Automatic quality analysis of an SVG diagram output.
///
/// Returns a Struct with:
///   - `passed`: Bool — true if no warnings (advisory, not blocking)
///   - `warnings`: List<String> — list of quality warnings found
///   - `checks_run`: Float — number of checks performed (always 3)
///
/// **Security:** This function reads an SVG string but produces no new
/// markup — it only analyzes existing output. No injection surface
/// is created (same rationale as chart_heatmap in Наряд №79).
///
/// **Philosophy:** `passed: false` means "worth reviewing", not "broken".
/// Low contrast may be intentional for decorative elements; high density
/// may be justified by content. This function advises, it does not gate.
pub(crate) fn builtin_infographic_qa(args: &[Value]) -> Result<Value, String> {
    let svg = expect_string_arg("infographic_qa", args, 0)?;

    if !svg.contains("<svg") {
        return Err(
            "infographic_qa: input does not appear to be an SVG string (no <svg tag found)"
                .to_string(),
        );
    }

    let mut warnings: Vec<String> = Vec::new();

    // ── Check 1: Contrast (Блок 1) ──
    // Scan for DiagramStyle-derivable colors in the SVG.
    // We look for fill on the background <rect> (paper) and text fill (ink).
    // If we can identify both, compute WCAG contrast ratio.
    // Threshold: WCAG AA for normal text = 4.5
    {
        // Find background fill (first <rect fill="..." typically)
        let paper_color = svg
            .split("fill=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .map(|s| s.to_string());

        // Find text fill (first <text ... fill="...")
        let ink_color = svg.find("<text").and_then(|pos| {
            svg[pos..]
                .split("fill=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .map(|s| s.to_string())
        });

        if let (Some(paper), Some(ink)) = (&paper_color, &ink_color) {
            if paper.starts_with('#') && ink.starts_with('#') {
                match contrast_ratio(paper, ink) {
                    Ok(ratio) => {
                        if ratio < 4.5 {
                            warnings.push(format!(
                                "contrast: paper/ink ratio {:.2} below WCAG AA threshold 4.5 (current: paper={}, ink={})",
                                ratio, paper, ink
                            ));
                        }
                    }
                    Err(e) => {
                        warnings.push(format!("contrast: could not compute ratio: {}", e));
                    }
                }
            }
        }
        // If we can't identify paper/ink, we simply skip this check
        // (no warning — absence of evidence is not evidence of absence)
    }

    // ── Check 2: Saturation discipline (Блок 2) ──
    // Count unique colors with saturation > 60%.
    // Design system recommends 1-2 focal accent colors with high saturation.
    // Threshold calibrated by examining color_palette() outputs:
    //   - "calm" light: accent has S=0.65, all others S<0.30 → 1 high-sat
    //   - "energy" light: accent S=0.65, ink S=0.30 → 1 high-sat
    //   - Manual style with accent="#eb6c36": this is S≈0.81 → 1 high-sat
    //   - A style with 3+ high-sat colors is likely visually chaotic.
    // Threshold: >2 unique colors with S > 60% → warning.
    {
        let colors = extract_svg_colors(&svg);
        let saturation_threshold = 0.60;
        let mut high_sat_count = 0usize;
        let mut high_sat_colors = Vec::new();

        for color in &colors {
            if let Ok((r, g, b)) = parse_hex_color(color) {
                let (_h, s, _l) = rgb_to_hsl(r, g, b);
                if s > saturation_threshold {
                    high_sat_count += 1;
                    high_sat_colors.push(color.clone());
                }
            }
        }

        if high_sat_count > 2 {
            warnings.push(format!(
                "saturation: {} unique highly-saturated colors (S>{:.0}%) found: [{}] — design system recommends 1-2 focal accents",
                high_sat_count,
                saturation_threshold * 100.0,
                high_sat_colors.join(", ")
            ));
        }
    }

    // ── Check 3: Element density (Блок 3) ──
    // density = element_count / (width * height / 10000)
    // Calibrated by measuring real chart outputs:
    //   - chart_bar 6 items: 21 elements, 600×400 canvas → density 0.875
    //   - diagram_timeline 10 events: 44 elements, 800×300 → density 1.83
    //   - chart_donut 3 slices: 15 elements, 600×400 → density 0.625
    //   - A manually constructed dense example with 60+ elements in 600×400 → density 2.5+
    // Threshold: density > 2.5 → warning ("likely overloaded")
    //           density < 0.3 → warning ("very sparse, consider smaller canvas or more content")
    {
        let element_count = count_svg_elements(&svg);
        let (width, height) = extract_canvas_dimensions(&svg);

        if width > 0.0 && height > 0.0 {
            let area_units = width * height / 10000.0;
            let density = element_count as f64 / area_units;

            if density > 2.5 {
                warnings.push(format!(
                    "density: {:.2} elements per 10K px² ({} elements in {:.0}×{:.0} canvas) — likely overloaded, consider simplifying or enlarging canvas",
                    density, element_count, width, height
                ));
            } else if density < 0.3 && element_count > 3 {
                // Only warn about sparsity if there are a few elements
                // (1-2 elements could be intentional minimal design)
                warnings.push(format!(
                    "density: {:.2} elements per 10K px² ({} elements in {:.0}×{:.0} canvas) — very sparse, consider smaller canvas or more content",
                    density, element_count, width, height
                ));
            }
        }
    }

    // ── Build result Struct ──
    let passed = warnings.is_empty();
    let checks_run = 3.0;

    let warning_values: Vec<Value> = warnings.into_iter().map(Value::String).collect();

    let mut fields = HashMap::new();
    fields.insert("passed".to_string(), Value::Bool(passed));
    fields.insert("warnings".to_string(), Value::List(warning_values));
    fields.insert("checks_run".to_string(), Value::Float(checks_run));

    Ok(Value::Struct {
        type_name: "InfographicQAResult".to_string(),
        fields,
    })
}
