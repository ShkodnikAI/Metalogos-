//! Timeline, gantt, process, and loop diagram builtins.
//!
//! Наряд №169: extracted from former diagrams.rs (all_legacy.rs).
//! No logic changes — pure move.
//!
//! Includes:
//! - `diagram_timeline(data, style)` — horizontal timeline with event dots
//! - `diagram_gantt(data, style)` — gantt chart with task bars
//! - `diagram_process(data, style)` — process flow with numbered steps
//! - `diagram_loop(data, style)` — circular loop diagram
//! - Constants: DATE_*, LABEL_*, DESC_*, GANTT_*, PROCESS_*, LOOP_*

use super::super::shared::*;
use super::layout::box_edge_point;
use super::tree_org::{DIAGRAM_CANVAS_H, DIAGRAM_CANVAS_W};
use crate::builtins::core::expect_list_arg;
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;

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

// Estimate the pixel width of `text` rendered at `font_size`. This
// is not a builtin — internal machinery for diagram_timeline.
//
// Uses the heuristic: width = 0.55 × font_size × len(text).
// The coefficient 0.55 is calibrated for proportional sans-serif fonts
// (e.g. system-ui, Helvetica, Arial) at the font sizes typical in
// Metalogos diagrams (10-14 px). It sits between monospace (~0.60)
// and all-lowercase proportional (~0.50), providing a safe estimate
// that is slightly wider than actual rendering — conservative overlap
// detection is better than missed overlaps.
pub fn builtin_diagram_timeline(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_timeline", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_timeline: data list must not be empty".to_string());
    }
    if data.len() > TIMELINE_MAX_EVENTS {
        return Err(format!(
            "diagram_timeline: too many events ({}), maximum is {}",
            data.len(),
            TIMELINE_MAX_EVENTS
        ));
    }
    // Extract items
    struct TlEvent {
        date: String,
        label: String,
        description: Option<String>,
    }
    let mut items: Vec<TlEvent> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_timeline: data[{}] must be Struct {{date, label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let date = struct_string_field("diagram_timeline item", f, "date")?;
        let label = struct_string_field("diagram_timeline item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push(TlEvent {
            date,
            label,
            description,
        });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let chart_x = 60.0_f64;
    let chart_w = canvas_w - 2.0 * chart_x;
    let n = items.len();
    let step = if n > 1 {
        chart_w / (n as f64 - 1.0)
    } else {
        0.0
    };

    // ── Н87: Build label bounding boxes for anti-overlap ─────────────
    //
    // For each event we create up to 3 label boxes: date (font 11),
    // label (font 12), description (font 10). Initial placement uses
    // parity alternation (same as pre-Н87), then resolve_overlaps
    // pushes apart any overlapping boxes vertically.
    //
    // We track which box belongs to which event and which text role
    // (date / label / description) so we can read back resolved y.

    const DATE_FONT_SIZE: f64 = 11.0;
    const LABEL_FONT_SIZE: f64 = 12.0;
    const DESC_FONT_SIZE: f64 = 10.0;
    const DATE_LINE_H: f64 = 14.0; // approximate line height for font 11
    const LABEL_LINE_H: f64 = 16.0; // approximate line height for font 12
    const DESC_LINE_H: f64 = 13.0; // approximate line height for font 10

    /// Which text role a LabelBox represents within a timeline event.
    #[derive(Clone, Copy)]
    enum TextRole {
        Date,
        Label,
        Description,
    }

    /// Index into items[] + role, so we can map resolved boxes back.
    struct BoxMeta {
        event_idx: usize,
        role: TextRole,
    }

    let mut label_boxes: Vec<LabelBox> = Vec::new();
    let mut box_metas: Vec<BoxMeta> = Vec::new();

    // First pass: compute x positions and initial y for all labels
    let event_xs: Vec<f64> = items
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if n > 1 {
                chart_x + (i as f64) * step
            } else {
                canvas_w / 2.0
            }
        })
        .collect();

    for (i, ev) in items.iter().enumerate() {
        let x = event_xs[i];
        let y = TIMELINE_AXIS_Y;
        // Parity alternation for initial placement (same as pre-Н87)
        let date_above = i % 2 == 0;
        let date_y = if date_above {
            y - TIMELINE_LABEL_OFFSET
        } else {
            y + TIMELINE_LABEL_OFFSET + 4.0
        };
        let label_y = if date_above {
            y + TIMELINE_LABEL_OFFSET + 4.0
        } else {
            y - TIMELINE_LABEL_OFFSET
        };

        // Date label box (text-anchor=middle → x is center)
        let date_w = estimate_text_width(&ev.date, DATE_FONT_SIZE);
        label_boxes.push(LabelBox {
            x: x - date_w / 2.0,
            y: date_y - DATE_LINE_H + 3.0, // top of text line
            w: date_w,
            h: DATE_LINE_H,
        });
        box_metas.push(BoxMeta {
            event_idx: i,
            role: TextRole::Date,
        });

        // Event label box
        let lbl_w = estimate_text_width(&ev.label, LABEL_FONT_SIZE);
        label_boxes.push(LabelBox {
            x: x - lbl_w / 2.0,
            y: label_y - LABEL_LINE_H + 3.0,
            w: lbl_w,
            h: LABEL_LINE_H,
        });
        box_metas.push(BoxMeta {
            event_idx: i,
            role: TextRole::Label,
        });

        // Optional description box
        if let Some(desc) = &ev.description {
            let desc_y = if date_above {
                label_y + 14.0
            } else {
                label_y - 14.0
            };
            let desc_w = estimate_text_width(desc, DESC_FONT_SIZE);
            label_boxes.push(LabelBox {
                x: x - desc_w / 2.0,
                y: desc_y - DESC_LINE_H + 3.0,
                w: desc_w,
                h: DESC_LINE_H,
            });
            box_metas.push(BoxMeta {
                event_idx: i,
                role: TextRole::Description,
            });
        }
    }

    // Run anti-overlap engine (Н87)
    let _iterations = resolve_overlaps(&mut label_boxes, Axis::Vertical, 20);

    // Read back resolved positions into per-event arrays
    // Each event has: date_y, label_y, desc_y (Option)
    let mut resolved_date_ys: Vec<f64> = vec![0.0; n];
    let mut resolved_label_ys: Vec<f64> = vec![0.0; n];
    let mut resolved_desc_ys: Vec<Option<f64>> = vec![None; n];

    for (bi, meta) in box_metas.iter().enumerate() {
        // Convert box top back to text baseline (box.y = baseline - line_h + 3)
        match meta.role {
            TextRole::Date => {
                resolved_date_ys[meta.event_idx] = label_boxes[bi].y + DATE_LINE_H - 3.0;
            }
            TextRole::Label => {
                resolved_label_ys[meta.event_idx] = label_boxes[bi].y + LABEL_LINE_H - 3.0;
            }
            TextRole::Description => {
                resolved_desc_ys[meta.event_idx] = Some(label_boxes[bi].y + DESC_LINE_H - 3.0);
            }
        }
    }

    // ── Render SVG ────────────────────────────────────────────────────
    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Horizontal axis line
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" />"#,
        fmt_num(chart_x - 10.0),
        fmt_num(TIMELINE_AXIS_Y),
        fmt_num(canvas_w - chart_x + 10.0),
        fmt_num(TIMELINE_AXIS_Y),
        escape_attr(&rule)
    ));
    // End caps (small ticks)
    for cap_x in &[chart_x - 10.0, canvas_w - chart_x + 10.0] {
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="2" />"#,
            fmt_num(*cap_x),
            fmt_num(TIMELINE_AXIS_Y - 6.0),
            fmt_num(*cap_x),
            fmt_num(TIMELINE_AXIS_Y + 6.0),
            escape_attr(&rule)
        ));
    }

    // Events — render at resolved positions
    for (i, ev) in items.iter().enumerate() {
        let x = event_xs[i];
        let y = TIMELINE_AXIS_Y;
        let date_y = resolved_date_ys[i];
        let label_y = resolved_label_ys[i];

        // Event dot — accent fill
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(TIMELINE_DOT_R),
            escape_attr(&accent),
            escape_attr(&paper)
        ));

        // Tick connecting dot to date label
        let date_above = date_y < y;
        let tick_y1 = if date_above {
            y - TIMELINE_DOT_R
        } else {
            y + TIMELINE_DOT_R
        };
        let tick_y2 = if date_above {
            date_y + 4.0
        } else {
            date_y - 8.0
        };
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(x),
            fmt_num(tick_y1),
            fmt_num(x),
            fmt_num(tick_y2),
            escape_attr(&rule)
        ));

        // Date label (accent color)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(date_y),
            escape_attr(&accent),
            escape_html_chars(&ev.date)
        ));
        // Event label (ink, primary)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&ev.label)
        ));
        // Optional description (muted, smaller)
        if let Some(desc_y) = resolved_desc_ys[i] {
            if let Some(desc) = &ev.description {
                parts.push(format!(
                    r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                    fmt_num(x),
                    fmt_num(desc_y),
                    escape_attr(&muted),
                    escape_html_chars(desc)
                ));
            }
        }
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

// ── Block 3: diagram_gantt ─────────────────────────────────────────
//
// Gantt chart: one horizontal bar per task, scaled to fit canvas.
// `start` and `duration` are abstract numeric units (days/weeks/etc —
// MVP does not bind to a calendar).
//
// Data shape:
//   List<Struct { task: String, start: Float, duration: Float }>
//
// Layout:
//   - canvas 600×400, chart area x=[140, 580] (left 140px reserved for
//     task labels), y=[40, 360]
//   - row_h = chart_h / N (each task gets equal vertical space)
//   - bar_y = chart_y_top + i × row_h + row_h × 0.25
//     (top + i×row + 25% inset so bars don't touch)
//   - bar_h = row_h × 0.5 (half the row height — leaves breathing room)
//   - bar_x = chart_x + (start / max_end) × chart_w
//   - bar_w = (duration / max_end) × chart_w
//     where max_end = max(start + duration) across all tasks
//   - Task label left-aligned to the right of the left margin
//     (i.e. at x = chart_x - 8, right-anchored)
//
// Limits: data.len() ≤ 15. duration ≤ 0 → Err (invalid input, not a
// silent zero-width bar).

const GANTT_MAX_TASKS: usize = 15;
const GANTT_CHART_X: f64 = 140.0; // left margin for task labels
const GANTT_CHART_W: f64 = 440.0; // 580 - 140
const GANTT_CHART_Y_TOP: f64 = 40.0;
const GANTT_CHART_H: f64 = 320.0; // 360 - 40

/// `diagram_gantt(data, style) -> String`
///
/// `data` is `List<Struct{task, start, duration}>`. Renders a Gantt chart
/// with one horizontal bar per task. The horizontal scale is derived from
/// `max(start + duration)` across all tasks.
pub fn builtin_diagram_gantt(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_gantt", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_gantt: data list must not be empty".to_string());
    }
    if data.len() > GANTT_MAX_TASKS {
        return Err(format!(
            "diagram_gantt: too many tasks ({}), maximum is {}",
            data.len(),
            GANTT_MAX_TASKS
        ));
    }
    // Extract items — validate duration > 0
    struct GanttTask {
        task: String,
        start: f64,
        duration: f64,
    }
    let mut items: Vec<GanttTask> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_gantt: data[{}] must be Struct {{task, start, duration}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let task = struct_string_field("diagram_gantt item", f, "task")?;
        let start = struct_float_field("diagram_gantt item", f, "start")?;
        let duration = struct_float_field("diagram_gantt item", f, "duration")?;
        if duration <= 0.0 {
            return Err(format!(
                "diagram_gantt: data[{}].duration must be positive (got {}) — invalid input, not a zero-width bar",
                i, duration
            ));
        }
        if start < 0.0 {
            return Err(format!(
                "diagram_gantt: data[{}].start must be non-negative (got {})",
                i, start
            ));
        }
        items.push(GanttTask {
            task,
            start,
            duration,
        });
    }
    // Scale: max(start + duration) across all tasks
    let max_end = items
        .iter()
        .map(|t| t.start + t.duration)
        .fold(f64::NEG_INFINITY, f64::max);
    if max_end <= 0.0 {
        return Err(format!(
            "diagram_gantt: max(start + duration) must be positive (got {})",
            max_end
        ));
    }
    let scale = GANTT_CHART_W / max_end;

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n = items.len();
    let row_h = GANTT_CHART_H / (n as f64);
    let bar_h = (row_h * 0.5).clamp(8.0, 28.0);

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Header rule (above first bar) — separates "title row" from bars
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(GANTT_CHART_Y_TOP - 8.0),
        fmt_num(canvas_w),
        fmt_num(GANTT_CHART_Y_TOP - 8.0),
        escape_attr(&rule)
    ));
    // Vertical separator between task labels and bar area
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(GANTT_CHART_X - 8.0),
        fmt_num(GANTT_CHART_Y_TOP - 8.0),
        fmt_num(GANTT_CHART_X - 8.0),
        fmt_num(GANTT_CHART_Y_TOP + GANTT_CHART_H + 8.0),
        escape_attr(&rule)
    ));
    // Bottom rule
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(GANTT_CHART_Y_TOP + GANTT_CHART_H + 8.0),
        fmt_num(canvas_w),
        fmt_num(GANTT_CHART_Y_TOP + GANTT_CHART_H + 8.0),
        escape_attr(&rule)
    ));

    // Bars + labels
    for (i, t) in items.iter().enumerate() {
        let row_y = GANTT_CHART_Y_TOP + (i as f64) * row_h;
        let bar_y = row_y + (row_h - bar_h) / 2.0;
        let bar_x = GANTT_CHART_X + t.start * scale;
        let bar_w = t.duration * scale;
        // Alternating row tint for readability
        if i % 2 == 1 {
            parts.push(format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" opacity="0.12" />"#,
                fmt_num(GANTT_CHART_X),
                fmt_num(row_y),
                fmt_num(GANTT_CHART_W),
                fmt_num(row_h),
                escape_attr(&rule)
            ));
        }
        // Task label (right-aligned, ink)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="end">{}</text>"#,
            fmt_num(GANTT_CHART_X - 12.0),
            fmt_num(bar_y + bar_h / 2.0 + 4.0),
            escape_attr(&ink),
            escape_html_chars(&t.task)
        ));
        // Bar — accent fill, paper stroke (subtle outline)
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1" rx="2" ry="2" />"#,
            fmt_num(bar_x),
            fmt_num(bar_y),
            fmt_num(bar_w),
            fmt_num(bar_h),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Duration label inside bar (if bar is wide enough)
        if bar_w > 40.0 {
            let dur_label = format!("{:.1}", t.duration);
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(bar_x + bar_w / 2.0),
                fmt_num(bar_y + bar_h / 2.0 + 3.0),
                escape_attr(&paper),
                escape_html_chars(&dur_label)
            ));
        }
    }
    // muted is used for nothing here but kept for style-token parity
    let _ = &muted;

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

// ── Block 4: diagram_process ───────────────────────────────────────
//
// Strictly LINEAR chain of numbered steps — NOT a flowchart.
//
// Difference from diagram_flowchart (Н81 Block 4):
//   - flowchart: arbitrary graph with branches/merges, topological sort
//     into layers, nodes/edges data shape.
//   - process: linear chain only, no branches, List<Struct{label, ...}>
//     data shape (NOT nodes/edges). Each step has a numbered badge
//     (1, 2, 3, ...) and is connected to the next via draw_connector.
//
// Data shape:
//   List<Struct { label: String, description?: String }>
//
// Layout:
//   - Horizontal chain of boxes left→right
//   - N steps → evenly spaced across chart_w
//   - Each box: 80w × 50h (or 60w × 60h if N is large) with rounded
//     corners, paper fill, rule border
//   - Numbered badge: small circle (r=10) in the top-left corner of
//     each box, filled with accent, containing the step number (1-indexed)
//   - Connectors between consecutive boxes via draw_connector
//
// Limits: data.len() ≤ 8.

const PROCESS_MAX_STEPS: usize = 8;
const PROCESS_BOX_W: f64 = 90.0;
const PROCESS_BOX_H: f64 = 56.0;
const PROCESS_BADGE_R: f64 = 10.0;

/// `diagram_process(data, style) -> String`
///
/// `data` is `List<Struct{label, description?}>`. Renders a strictly
/// linear chain of numbered steps connected by arrows. This is NOT the
/// same as diagram_flowchart — process has no branches/merges and a
/// different data shape (List<Struct>, not Struct{nodes, edges}).
pub fn builtin_diagram_process(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_process", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_process: data list must not be empty".to_string());
    }
    if data.len() > PROCESS_MAX_STEPS {
        return Err(format!(
            "diagram_process: too many steps ({}), maximum is {} — linear chain longer than 8 doesn't fit a reasonable canvas",
            data.len(),
            PROCESS_MAX_STEPS
        ));
    }
    // Extract items
    struct ProcStep {
        label: String,
        description: Option<String>,
    }
    let mut items: Vec<ProcStep> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_process: data[{}] must be Struct {{label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_process item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push(ProcStep { label, description });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n = items.len();
    // Center the chain horizontally — each box center at:
    //   x_i = pad + (i + 0.5) × (canvas_w - 2×pad) / n
    // where pad reserves space for half a box on each side.
    let pad = PROCESS_BOX_W / 2.0 + 12.0;
    let usable_w = canvas_w - 2.0 * pad;
    let step_w = usable_w / (n as f64);
    let box_cy = canvas_h / 2.0;
    let box_y = box_cy - PROCESS_BOX_H / 2.0;

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Connectors FIRST (so boxes render on top of any line tips)
    for i in 1..n {
        let prev_cx = pad + ((i - 1) as f64 + 0.5) * step_w;
        let curr_cx = pad + (i as f64 + 0.5) * step_w;
        let start_x = prev_cx + PROCESS_BOX_W / 2.0;
        let end_x = curr_cx - PROCESS_BOX_W / 2.0;
        // Horizontal connector at box vertical midpoint
        parts.push(draw_connector(start_x, box_cy, end_x, box_cy, &style));
    }

    // Boxes + badges + labels
    for (i, step) in items.iter().enumerate() {
        let cx = pad + (i as f64 + 0.5) * step_w;
        let box_x = cx - PROCESS_BOX_W / 2.0;
        // Box — paper fill, rule border, rounded
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(PROCESS_BOX_W),
            fmt_num(PROCESS_BOX_H),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        // Numbered badge — top-left corner, accent fill, paper text
        let badge_cx = box_x + 4.0 + PROCESS_BADGE_R;
        let badge_cy = box_y + 4.0 + PROCESS_BADGE_R;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy),
            fmt_num(PROCESS_BADGE_R),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Step number (1-indexed)
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy + 4.0),
            escape_attr(&paper),
            i + 1
        ));
        // Label — centered horizontally, slightly below badge
        let label_y = box_y + PROCESS_BOX_H / 2.0 + 4.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&step.label)
        ));
        // Optional description below the box (small, muted)
        if let Some(desc) = &step.description {
            let desc_y = box_y + PROCESS_BOX_H + 14.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(desc_y),
                escape_attr(&muted),
                escape_html_chars(desc)
            ));
        }
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

// ── Block 5: diagram_loop ──────────────────────────────────────────
//
// Closed-loop (flywheel) diagram — steps arranged on a circle, last
// step connects back to the first to close the cycle.
//
// Data shape:
//   List<Struct { label: String, description?: String }>
//
// Layout:
//   - N steps placed on a circle using polar_to_xy
//   - angle_i = 2π × i / N − π/2  (start at top, same orientation as
//     chart_radar from Н79 — preserved for visual consistency between
//     circular functions in the graphics suite)
//   - Each step rendered as a small box at polar_to_xy(cx, cy, r, angle_i)
//   - Connectors via draw_connector from step i to step i+1
//   - Last step (i = N-1) connects back to step 0 (closed loop)
//
// Limits: 3 ≤ N ≤ 8 (N < 3 → visually meaningless cycle; N > 8 → labels
// overlap on the circle's circumference).

const LOOP_MIN_STEPS: usize = 3;
const LOOP_MAX_STEPS: usize = 8;
const LOOP_BOX_W: f64 = 90.0;
const LOOP_BOX_H: f64 = 44.0;

/// `diagram_loop(data, style) -> String`
///
/// `data` is `List<Struct{label, description?}>`. Renders a closed-loop
/// (flywheel) diagram with steps arranged on a circle. The last step
/// connects back to the first to close the cycle.
pub fn builtin_diagram_loop(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_loop", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_loop: data list must not be empty".to_string());
    }
    if data.len() < LOOP_MIN_STEPS {
        return Err(format!(
            "diagram_loop: too few steps ({}), minimum is {} — a cycle with <3 steps is visually meaningless",
            data.len(),
            LOOP_MIN_STEPS
        ));
    }
    if data.len() > LOOP_MAX_STEPS {
        return Err(format!(
            "diagram_loop: too many steps ({}), maximum is {} — labels would overlap on the circle",
            data.len(),
            LOOP_MAX_STEPS
        ));
    }
    // Extract items
    struct LoopStep {
        label: String,
        description: Option<String>,
    }
    let mut items: Vec<LoopStep> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_loop: data[{}] must be Struct {{label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_loop item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push(LoopStep { label, description });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;
    // Reserve room for labels OUTSIDE the boxes — reduce radius so boxes
    // fit comfortably inside the canvas (box half-diagonal ≈ 50px).
    let r = (canvas_h / 2.0 - 70.0).min(canvas_w / 2.0 - 80.0);
    let n = items.len();

    // Compute box centers via polar_to_xy
    let centers: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            // angle_i = 2π × i / N − π/2  (start at top)
            let angle =
                2.0 * std::f64::consts::PI * (i as f64) / (n as f64) - std::f64::consts::PI / 2.0;
            polar_to_xy(cx, cy, r, angle)
        })
        .collect();

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Faint reference circle (visual anchor — shows the conceptual loop)
    parts.push(format!(
        r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="1" stroke-opacity="0.25" stroke-dasharray="3 3" />"#,
        fmt_num(cx),
        fmt_num(cy),
        fmt_num(r),
        escape_attr(&rule)
    ));

    // Connectors FIRST (so boxes render on top)
    // For each step i, draw connector from centers[i] to centers[(i+1) % n].
    // The modular +1 ensures the last step connects back to the first —
    // this is the closed-loop invariant from the narazd spec.
    for i in 0..n {
        let (x1, y1) = centers[i];
        let (x2, y2) = centers[(i + 1) % n];
        // Trim endpoints so connectors touch box edges, not centers
        let (sx, sy) = box_edge_point(x1, y1, x2, y2, LOOP_BOX_W, LOOP_BOX_H);
        let (ex, ey) = box_edge_point(x2, y2, x1, y1, LOOP_BOX_W, LOOP_BOX_H);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
    }

    // Boxes + labels
    for (i, step) in items.iter().enumerate() {
        let (bx, by) = centers[i];
        let box_x = bx - LOOP_BOX_W / 2.0;
        let box_y = by - LOOP_BOX_H / 2.0;
        // Box — paper fill, accent border (loop is "the protagonist" so
        // its border is accent rather than rule — visual emphasis)
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(LOOP_BOX_W),
            fmt_num(LOOP_BOX_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        // Step number (small badge in top-left, similar to diagram_process)
        let badge_cx = box_x + 12.0;
        let badge_cy = box_y + 12.0;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="8" fill="{}" />"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="10" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(badge_cx),
            fmt_num(badge_cy + 3.0),
            escape_attr(&paper),
            i + 1
        ));
        // Label — centered horizontally, vertically just below center
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(bx),
            fmt_num(by + 4.0),
            escape_attr(&ink),
            escape_html_chars(&step.label)
        ));
        // Optional description below the box, anchored to the side facing
        // away from the center (so it doesn't overlap the loop interior).
        if let Some(desc) = &step.description {
            // Direction from center to box → outward normal
            let dx = bx - cx;
            let dy = by - cy;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let nx = dx / len;
            let ny = dy / len;
            let desc_x = bx + nx * (LOOP_BOX_W / 2.0 + 14.0);
            let desc_y = by + ny * (LOOP_BOX_H / 2.0 + 14.0) + 4.0;
            // Anchor depends on which side of the circle we're on
            let anchor = if nx > 0.3 {
                "start"
            } else if nx < -0.3 {
                "end"
            } else {
                "middle"
            };
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="{}">{}</text>"#,
                fmt_num(desc_x),
                fmt_num(desc_y),
                escape_attr(&muted),
                anchor,
                escape_html_chars(desc)
            ));
        }
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
