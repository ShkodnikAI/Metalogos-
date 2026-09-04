//! Tree and org-chart diagram builtins.
//!
//! Наряд №169: extracted from former diagrams.rs. No logic changes.
//!
//! Includes:
//! - `diagram_tree(data, style)` — recursive tree from Struct{label, title?, children}
//! - `diagram_org_chart(data, style)` — same shape, title field is rendered
//! - `TreeNode` struct, `LaidOutNode` layout result struct
//! - `extract_tree_node`, `layout_tree`, `render_tree_node`, `translate_subtree`
//!   helpers
//! - `box_edge_point` is in `super::layout` (shared with other groups)
//! - canvas constants `DIAGRAM_CANVAS_W/H` are tree-org specific (used here
//!   for canvas sizing; other groups have their own canvas constants)

use super::super::shared::*;
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;
use std::collections::HashMap;

struct TreeNode {
    label: String,
    title: Option<String>,
    children: Vec<TreeNode>,
}

/// Extract a TreeNode from a Value::Struct. Recurses into `children`.
/// `title` field is optional — None if missing/Unit (diagram_tree case).
/// Enforces the depth + total node count limits at extraction time so
/// the layout function can assume well-bounded input.
fn extract_tree_node(
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
struct LaidOutNode {
    /// x-center of the box.
    cx: f64,
    /// y-top of the box.
    y: f64,
    /// Subtree width (for parent centering).
    subtree_w: f64,
    /// Node box dimensions (constant across all nodes — kept here for
    /// readability, the layout function uses the constants directly).
    /// Children (laid out recursively).
    children: Vec<LaidOutNode>,
    /// Reference to the source tree node (for rendering label/title).
    /// Stored as label + title snapshot to avoid lifetime entanglement.
    label: String,
    title: Option<String>,
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
fn layout_tree(node: &TreeNode, depth: usize) -> LaidOutNode {
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
fn render_tree_node(
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

/// Diagram canvas: 600 × 400 (matches chart_bar). Tree layout may exceed
/// this horizontally for wide trees — we scale the viewBox to fit the
/// actual laid-out tree width, so wide trees are rendered fully (no
/// clipping). Height is fixed (depth ≤ 6 → max ~6 * 90px = 540px).
pub(super) const DIAGRAM_CANVAS_W: f64 = 600.0;
pub(super) const DIAGRAM_CANVAS_H: f64 = 400.0;

/// `diagram_tree(data, style) -> String`
///
/// Renders a recursive tree. `data` is `Struct { label, children }` —
/// children is the same shape recursively. Empty children list = leaf.
///
/// Limits: depth ≤ 6, total nodes ≤ 40 (returns Err otherwise).
pub fn builtin_diagram_tree(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let mut node_count = 0usize;
    let root = extract_tree_node(&data_value, "root", false, 0, &mut node_count)?;
    let laid = layout_tree(&root, 0);
    // Compute total width: subtree_w of root. Center the tree horizontally
    // in the canvas with at least 20px left padding.
    let tree_w = laid.subtree_w.max(TREE_NODE_W);
    let canvas_w = DIAGRAM_CANVAS_W.max(tree_w + 40.0);
    // x-offset to center tree in canvas
    let x_offset = (canvas_w - tree_w) / 2.0;
    // Translate root cx (which is relative to subtree left) to absolute
    let mut parts: Vec<String> = Vec::new();
    // Render into a translate group so all relative coords become absolute
    let mut absolute_node = laid;
    absolute_node.cx += x_offset;
    translate_subtree(&mut absolute_node, x_offset);
    // Background
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        escape_attr(&paper)
    ));
    render_tree_node(&absolute_node, &style, false, &mut parts);
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        body
    )))
}

/// Walk a LaidOutNode and add `dx` to every cx (in place). Used to
/// convert relative-to-subtree coords to absolute canvas coords.
fn translate_subtree(node: &mut LaidOutNode, dx: f64) {
    node.cx += dx;
    for child in &mut node.children {
        translate_subtree(child, dx);
    }
}

/// `diagram_org_chart(data, style) -> String`
///
/// Thin wrapper over diagram_tree's layout algorithm. The ONLY
/// difference is the per-node render: when a `title` field is present,
/// the node box is taller and a second <text> line is emitted. The
/// layout algorithm (subtree width, parent centering, depth spacing)
/// is identical — we call the same `extract_tree_node` with
/// `allow_title=true`, the same `layout_tree`, and the same
/// `render_tree_node` with `is_org_chart=true`.
///
/// `data` is `Struct { label, title?, children }`.
pub fn builtin_diagram_org_chart(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let mut node_count = 0usize;
    let root = extract_tree_node(&data_value, "root", true, 0, &mut node_count)?;
    let laid = layout_tree(&root, 0);
    let tree_w = laid.subtree_w.max(TREE_NODE_W);
    let canvas_w = DIAGRAM_CANVAS_W.max(tree_w + 40.0);
    let x_offset = (canvas_w - tree_w) / 2.0;
    let mut absolute_node = laid;
    absolute_node.cx += x_offset;
    translate_subtree(&mut absolute_node, x_offset);
    let mut parts: Vec<String> = Vec::new();
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        escape_attr(&paper)
    ));
    render_tree_node(&absolute_node, &style, true, &mut parts);
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        fmt_num(canvas_w),
        fmt_num(DIAGRAM_CANVAS_H),
        body
    )))
}

// ── Block 4: diagram_flowchart ─────────────────────────────────────
