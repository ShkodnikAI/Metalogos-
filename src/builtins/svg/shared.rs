// Наряд №110: общие вспомогательные функции и типы для svg/primitives.rs,
// svg/charts.rs, svg/diagrams.rs. Перенесено механически из старого svg.rs
// (git blame сохраняет историю по содержимому строк) без функциональных изменений.
use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;
use std::collections::HashMap;

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


/// Convert polar coordinates (center + angle in radians) to SVG cartesian.
/// SVG Y-axis points down, so we use sin(angle) directly (no negation).
/// angle = -π/2 corresponds to the top of the circle.
pub(super) fn polar_to_xy(cx: f64, cy: f64, r: f64, angle: f64) -> (f64, f64) {
    (cx + r * angle.cos(), cy + r * angle.sin())
}


/// Build a list of N slice colors that stay within the same color family
/// (accent + ink, alternating). For N=1, return [accent]. For N>1, alternate
/// accent and ink so adjacent slices have different colors but the whole
/// chart stays within the same hue family (palette.md V2.1).
pub(super) fn build_slice_colors(accent: &str, ink: &str, n: usize) -> Vec<String> {
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
pub(super) fn radar_series_palette() -> Vec<String> {
    vec![
        hsl_to_hex(0.0, 0.55, 0.55),
        hsl_to_hex(45.0, 0.55, 0.50),
        hsl_to_hex(135.0, 0.40, 0.45),
        hsl_to_hex(205.0, 0.50, 0.55),
        hsl_to_hex(285.0, 0.45, 0.60),
    ]
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
pub(super) fn percentile_r7(sorted: &[f64], p: f64) -> f64 {
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
/// h: 0-360 degrees, s: 0.0-1.0, l: 0.0-1.0.
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


// ── Level 2.5b: svg_icon (Tabler-compatible MIT icon set) ────────────
//
// 10 starter icons, 24x24 viewBox, stroke="currentColor" so they inherit
// color from the parent group. Path data sourced from Tabler Icons (MIT).
// Returns a complete <svg> fragment that can be placed via <use> or
// embedded directly in a <g transform="translate(x,y) scale(s)">.

/// Map icon name to its path data. Returns None for unknown names.
pub(super) fn icon_path_data(name: &str) -> Option<&'static str> {
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
pub(super) fn background_style(intent: &str) -> Result<HashMap<String, Value>, String> {
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
pub(super) fn generate_grid(style: &HashMap<String, Value>, w: f64, h: f64) -> String {
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
pub(super) fn generate_flow(style: &HashMap<String, Value>, w: f64, h: f64, intent: &str) -> String {
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
pub(super) fn generate_noise(style: &HashMap<String, Value>, w: f64, h: f64, intent: &str) -> String {
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
// per-node render to emit a second <text> line when title is present.
pub(super) struct TreeNode {
    pub(super) label: String,
    pub(super) title: Option<String>,
    pub(super) children: Vec<TreeNode>,
}

/// Extract a TreeNode from a Value::Struct. Recurses into `children`.
/// `title` field is optional — None if missing/Unit (diagram_tree case).
/// Enforces the depth + total node count limits at extraction time so
/// the layout function can assume well-bounded input.
pub(super) fn extract_tree_node(
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
pub(super) struct LaidOutNode {
    /// x-center of the box.
    pub(super) cx: f64,
    /// y-top of the box.
    pub(super) y: f64,
    /// Subtree width (for parent centering).
    pub(super) subtree_w: f64,
    /// Node box dimensions (constant across all nodes — kept here for
    /// readability, the layout function uses the constants directly).
    /// Children (laid out recursively).
    pub(super) children: Vec<LaidOutNode>,
    /// Reference to the source tree node (for rendering label/title).
    /// Stored as label + title snapshot to avoid lifetime entanglement.
    pub(super) label: String,
    pub(super) title: Option<String>,
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
pub(super) fn layout_tree(node: &TreeNode, depth: usize) -> LaidOutNode {
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
pub(super) fn render_tree_node(
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


/// Walk a LaidOutNode and add `dx` to every cx (in place). Used to
/// convert relative-to-subtree coords to absolute canvas coords.
pub(super) fn translate_subtree(node: &mut LaidOutNode, dx: f64) {
    node.cx += dx;
    for child in &mut node.children {
        translate_subtree(child, dx);
    }
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
pub(super) struct FlowNode {
    pub(super) id: String,
    pub(super) label: String,
}
pub(super) struct FlowEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) label: Option<String>,
}

/// Extract nodes + edges from the input Struct. Validates that all
/// edge endpoints reference existing node IDs.
pub(super) fn extract_flowchart(data_value: &Value) -> Result<(Vec<FlowNode>, Vec<FlowEdge>), String> {
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
pub(super) fn topological_layers(
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
pub(super) fn bfs_layers_with_cycles(
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


/// Compute the point where the line from (cx,cy) to (tx,ty) intersects
/// the boundary of a box centered at (cx,cy) with width w and height h.
/// Used to make connectors touch the box edge instead of the center.
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
//     This is the initial placement; Н87 anti-overlap engine then
//     resolves any collisions by pushing overlapping boxes apart.
//   - `label` and `description` go on the OPPOSITE side of the dot
//     from `date`, so each event has at most: date (one side) +
//     label/description (other side).
//
// Limits: data.len() ≤ 12.

// ── Наряд №87: Anti-overlap engine ─────────────────────────────────────
//
// Two internal helpers:
//
//   1. `estimate_text_width(text, font_size) → f64`
//      Heuristic: 0.55 × font_size × char_count.
//      Coefficient 0.55 chosen as a reasonable average for proportional
//      fonts (Latin + digits): narrower than monospace (0.60) but wider
//      than pure lowercase (0.50). Matches the mid-range of common
//      sans-serif faces at typical diagram font sizes (10–14 px).
//
//   2. `resolve_overlaps(labels, axis, max_iterations)`
//      Iterative pairwise overlap resolution. On each iteration, scans
//      all label pairs; if two boxes overlap, pushes them apart along
//      the given axis (Vertical or Radial). Terminates when no overlaps
//      remain or max_iterations is exhausted.
//
// Neither is exposed as a public builtin — they are internal machinery
// for diagram_timeline (and future diagram types).

/// Estimate the pixel width of `text` rendered at `font_size`.
///
/// Uses the heuristic: width = 0.55 × font_size × len(text).
/// The coefficient 0.55 is calibrated for proportional sans-serif fonts
/// (e.g. system-ui, Helvetica, Arial) at the font sizes typical in
/// Metalogos diagrams (10–14 px). It sits between monospace (≈0.60)
/// and all-lowercase proportional (≈0.50), providing a safe estimate
/// that is slightly wider than actual rendering — conservative overlap
/// detection is better than missed overlaps.
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
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) w: f64,
    pub(super) h: f64,
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


// ── Block 5/6 shared: extract_graph ─────────────────────────────────
//
// All three of diagram_data_flow / diagram_high_level / diagram_architecture
// share the same `Struct{nodes, edges}` shape. The only difference is
// whether the `icon` field is allowed on nodes (architecture only).
// We parse all three into a common (GraphNode, GraphEdge) representation
// and dispatch to the appropriate layer function at the call site.

pub(super) struct GraphNode {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) icon: Option<String>,
}

pub(super) struct GraphEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) label: Option<String>,
}

/// Parse `Struct{nodes: [{id, label, icon?}], edges: [{from, to, label?}]}`
/// into (Vec<GraphNode>, Vec<GraphEdge>). `allow_icon` controls whether
/// the `icon` field is read on each node — diagram_architecture passes
/// true; data_flow and high_level pass false (the field is silently
/// ignored if present, matching the spec's "icon not used" wording).
pub(super) fn extract_graph(
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
pub(super) fn layout_layered_nodes(
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


// ── Internal: escape XML attribute values ────────────────────────────
//
// For attribute values (inside "..."), we must escape: & < > " '
// We reuse escape_html_chars which already handles all 5.
pub(super) fn escape_attr(s: &str) -> String {
    escape_html_chars(s)
}


// ══════════════════════════════════════════════════════════════════════
// Наряд №89: infographic_qa — automatic quality checks for SVG output
// ══════════════════════════════════════════════════════════════════════
//
// Three mechanically checkable aspects:
//   1. Contrast text/background (WCAG-like formula on DiagramStyle)
//   2. Saturation discipline (scan fill/stroke colors in SVG)
//   3. Element density (primitive tag count / canvas area)
//
// NOT checked (consciously out of scope):
//   - General label overlap/collision — resolve_overlaps exists for
//     diagram_timeline (N87) but generalizing to all 41 diagram types
//     is a separate, larger task.
//   - "Visual harmony" — not formalizable without real vision.
//
// Security: infographic_qa reads an SVG string but produces no new
// markup — it only analyzes existing output. No injection surface
// is created (same rationale as chart_heatmap in N79).

/// Parse a hex color string (#RGB or #RRGGBB) into (R, G, B) in 0.0–1.0.
pub(super) fn parse_hex_color(hex: &str) -> Result<(f64, f64, f64), String> {
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
pub(super) fn relative_luminance(hex: &str) -> Result<f64, String> {
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
pub(super) fn contrast_ratio(hex1: &str, hex2: &str) -> Result<f64, String> {
    let l1 = relative_luminance(hex1)?;
    let l2 = relative_luminance(hex2)?;
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    Ok((lighter + 0.05) / (darker + 0.05))
}


/// Convert RGB (0.0–1.0) to HSL. Returns (h, s, l) where
/// h in 0–360, s in 0–1, l in 0–1.
pub(super) fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
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
pub(super) fn extract_svg_colors(svg: &str) -> Vec<String> {
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
pub(super) fn count_svg_elements(svg: &str) -> usize {
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
pub(super) fn extract_canvas_dimensions(svg: &str) -> (f64, f64) {
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
