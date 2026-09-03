//! Diagram builtins (diagram_*)
//! Category: "diagram" (from registry.rs)
//! diagram_style is in primitives (tokens category)

use super::primitives::icon_path_data;
use super::shared::*;
use crate::builtins::core::{expect_list_arg, expect_string_arg};
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
const DIAGRAM_CANVAS_W: f64 = 600.0;
const DIAGRAM_CANVAS_H: f64 = 400.0;

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
struct FlowNode {
    id: String,
    label: String,
}
struct FlowEdge {
    from: String,
    to: String,
    label: Option<String>,
}

/// Extract nodes + edges from the input Struct. Validates that all
/// edge endpoints reference existing node IDs.
fn extract_flowchart(data_value: &Value) -> Result<(Vec<FlowNode>, Vec<FlowEdge>), String> {
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
fn topological_layers(
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
fn bfs_layers_with_cycles(
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

/// `diagram_flowchart(data, style) -> String`
///
/// Renders a flowchart with layered topological layout. Nodes in the
/// same layer share a Y coordinate; layers are stacked vertically.
/// Edges use draw_connector; optional edge labels are placed at the
/// midpoint, offset slightly above the line.
///
/// `data` is `Struct { nodes: List<{id, label}>, edges: List<{from, to, label?}> }`.
///
/// Returns Err with "flowchart contains a cycle: ..." if the graph
/// has a cycle (topological sort cannot complete).
pub fn builtin_diagram_flowchart(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_flowchart(&data_value)?;
    // Н84 Block 4: topological_layers now takes plain &[String] + &[(String,String)].
    // Derive the inputs from the parsed FlowNode/FlowEdge structs — behavior
    // is unchanged for flowchart (regression-checked by p84_topological_layers_regression.mlog).
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    let layers = topological_layers(&node_ids, &edge_pairs)?;
    // Layout: each layer is one horizontal row. Within a row, nodes
    // are spread evenly across the canvas width (with side padding).
    let n_layers = layers.len();
    // Y position per layer: distribute vertically across canvas height
    // with top/bottom padding. Layer 0 at top.
    let layer_h = (DIAGRAM_CANVAS_H - 80.0) / (n_layers as f64).max(1.0);
    let mut id_to_pos: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::new();
    let mut id_to_label: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for n in &nodes {
        id_to_label.insert(n.id.clone(), n.label.as_str());
    }
    for (layer_idx, layer_nodes) in layers.iter().enumerate() {
        let count = layer_nodes.len();
        let y_center = 40.0 + (layer_idx as f64 + 0.5) * layer_h;
        // Spread nodes: if 1 node, center; else distribute evenly.
        let total_w = DIAGRAM_CANVAS_W - 80.0; // 40px padding each side
        let step = if count > 1 {
            total_w / (count as f64 - 1.0)
        } else {
            0.0
        };
        let start_x = if count > 1 {
            40.0
        } else {
            DIAGRAM_CANVAS_W / 2.0
        };
        for (i, id) in layer_nodes.iter().enumerate() {
            let x_center = start_x + (i as f64) * step;
            id_to_pos.insert(id.clone(), (x_center, y_center));
        }
    }
    let mut parts: Vec<String> = Vec::new();
    // Background
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(DIAGRAM_CANVAS_W),
        fmt_num(DIAGRAM_CANVAS_H),
        escape_attr(&paper)
    ));
    // Edges first (so node boxes render on top of any line that clips)
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    for edge in &edges {
        // Look up positions — both endpoints must be in id_to_pos
        // (we built the map from layers, which contains every node).
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_flowchart: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_flowchart: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        // Trim endpoints so connectors start/end at the box edges, not centers
        let (sx, sy) = box_edge_point(
            from_x,
            from_y,
            to_x,
            to_y,
            FLOWCHART_NODE_W,
            FLOWCHART_NODE_H,
        );
        let (ex, ey) = box_edge_point(
            to_x,
            to_y,
            from_x,
            from_y,
            FLOWCHART_NODE_W,
            FLOWCHART_NODE_H,
        );
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        // Optional edge label at midpoint, offset above the line
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // Nodes
    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    for (id, (cx, cy)) in &id_to_pos {
        let label = id_to_label.get(id).copied().unwrap_or("");
        let box_x = cx - FLOWCHART_NODE_W / 2.0;
        let box_y = cy - FLOWCHART_NODE_H / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(FLOWCHART_NODE_W),
            fmt_num(FLOWCHART_NODE_H),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(*cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }
    let body = parts.join("\n");
    Ok(Value::String(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
        fmt_num(DIAGRAM_CANVAS_W),
        fmt_num(DIAGRAM_CANVAS_H),
        fmt_num(DIAGRAM_CANVAS_W),
        fmt_num(DIAGRAM_CANVAS_H),
        body
    )))
}

/// Compute the point where the line from (cx,cy) to (tx,ty) intersects
/// the boundary of a box centered at (cx,cy) with width w and height h.
/// Used to make connectors touch the box edge instead of the center.
fn box_edge_point(cx: f64, cy: f64, tx: f64, ty: f64, w: f64, h: f64) -> (f64, f64) {
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

// ── Block 5: diagram_layers ─────────────────────────────────────────
//
// Simplest of the four — no draw_connector, no tree algorithm. Just
// horizontal stripes of equal height stacked top-to-bottom.
//
// Layout:
//   - canvas: 600 × 400
//   - N layers, each height = canvas_h / N
//   - label left-aligned with 16px left padding, vertically centered
//   - optional description right-aligned with 16px right padding,
//     smaller font, muted color
//
// Limits: data.len() ≤ 10 (otherwise stripes become too narrow).

/// `diagram_layers(data, style) -> String`
///
/// `data` is `List<Struct { label, description? }>`. Renders horizontal
/// stripes top-to-bottom.
pub fn builtin_diagram_layers(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_layers", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_layers: data list must not be empty".to_string());
    }
    if data.len() > 10 {
        return Err(format!(
            "diagram_layers: too many layers ({}), maximum is 10",
            data.len()
        ));
    }
    // Extract items
    let mut items: Vec<(String, Option<String>)> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_layers: data[{}] must be Struct {{label, description?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_layers item", f, "label")?;
        let description = struct_opt_string_field(f, "description");
        items.push((label, description));
    }
    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());
    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let layer_h = canvas_h / (items.len() as f64);
    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    for (i, (label, description)) in items.iter().enumerate() {
        let y = (i as f64) * layer_h;
        // Alternating fill: even=index paper, odd=very light rule tint
        // (we don't have a tint primitive, so we use paper for even and
        // a manually lightened version of rule for odd). Simpler: use
        // paper for even layers, rule at 0.15 opacity for odd.
        let is_odd = i % 2 == 1;
        if is_odd {
            parts.push(format!(
                r#"<rect x="0" y="{}" width="{}" height="{}" fill="{}" opacity="0.18" />"#,
                fmt_num(y),
                fmt_num(canvas_w),
                fmt_num(layer_h),
                escape_attr(&rule)
            ));
        }
        // Top border (rule) — separates layers visually
        parts.push(format!(
            r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(y),
            fmt_num(canvas_w),
            fmt_num(y),
            escape_attr(&rule)
        ));
        // Label — left-aligned, vertically centered
        let label_y = y + layer_h / 2.0 + 4.0;
        // Accent left bar (3px wide) — visual anchor on the left edge
        parts.push(format!(
            r#"<rect x="0" y="{}" width="3" height="{}" fill="{}" />"#,
            fmt_num(y),
            fmt_num(layer_h),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="16" y="{}" font-size="14" fill="{}">{}</text>"#,
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
        // Description — right-aligned, smaller, muted
        if let Some(desc) = description {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="end">{}</text>"#,
                fmt_num(canvas_w - 16.0),
                fmt_num(label_y),
                escape_attr(&muted),
                escape_html_chars(desc)
            ));
        }
    }
    // Bottom border (rule)
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_h),
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Наряд №82: Diagrams, part 2 — temporal & process ────────────────
//
// Five additional diagram builtins, all built on top of the geometric
// primitives delivered by Н81 (draw_connector, polar_to_xy) — no new
// geometry is invented here.
//
//   Block 1 — diagram_sequence  (uses draw_connector)
//   Block 2 — diagram_timeline  (uses svg_circle inline + horizontal line)
//   Block 3 — diagram_gantt     (uses svg_rect inline)
//   Block 4 — diagram_process   (uses draw_connector; NOT flowchart — strictly linear)
//   Block 5 — diagram_loop      (uses polar_to_xy + draw_connector; closed cycle)
//
// All five reuse the DIAGRAM_CANVAS_W/H constants (600×400) defined above
// for visual consistency with the Н81 diagram suite.

// ── Block 1: diagram_sequence ──────────────────────────────────────
//
// UML-style sequence diagram: vertical "lifelines" for each actor,
// horizontal arrows between lifelines for each message.
//
// Data shape:
//   Struct {
//     actors:   List<String>,                          // lifeline names
//     messages: List<Struct { from, to, label? }>,     // arrows
//   }
//
// Layout:
//   - N actors → evenly spaced columns across canvas_w
//   - Each actor: vertical dashed line top→bottom + name at top
//   - Each message: horizontal arrow from actor[from] to actor[to]
//     at Y = top_pad + msg_idx × step (top-down chronological order)
//   - Non-adjacent messages (e.g. actor 0 → actor 3) draw a longer
//     diagonal line — this is the spec's "проверить, что диагональные
//     стрелки строятся корректно" requirement.
//
// Limits: actors.len() ≤ 8, messages.len() ≤ 30.

const SEQ_MAX_ACTORS: usize = 8;
const SEQ_MAX_MESSAGES: usize = 30;
const SEQ_TOP_PAD: f64 = 50.0; // space for actor name labels at top
const SEQ_BOTTOM_PAD: f64 = 30.0;
const SEQ_LIFELINE_HALF_H: f64 = 12.0; // half-height of actor head box

/// `diagram_sequence(data, style) -> String`
///
/// Renders a UML-style sequence diagram. `data` is
/// `Struct { actors: List<String>, messages: List<Struct{from, to, label?}> }`.
///
/// Returns Err if:
///   - actors is empty or > 8
///   - messages > 30
///   - a message references an unknown actor name
pub fn builtin_diagram_sequence(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;

    // Extract top-level Struct { actors, messages }
    let data_fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_sequence: data must be Struct {{actors, messages}}, got {}",
                other.type_name()
            ));
        }
    };

    // actors: List<String>
    let actors_value = data_fields
        .get("actors")
        .ok_or_else(|| "diagram_sequence: missing required field 'actors'".to_string())?;
    let actors: Vec<String> = match actors_value {
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, v) in items.iter().enumerate() {
                match v {
                    Value::String(s) => out.push(s.clone()),
                    other => {
                        return Err(format!(
                            "diagram_sequence: actors[{}] must be String, got {}",
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
                "diagram_sequence: 'actors' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };
    if actors.is_empty() {
        return Err("diagram_sequence: actors list must not be empty".to_string());
    }
    if actors.len() > SEQ_MAX_ACTORS {
        return Err(format!(
            "diagram_sequence: too many actors ({}), maximum is {} — lifelines become too narrow",
            actors.len(),
            SEQ_MAX_ACTORS
        ));
    }

    // messages: List<Struct{from, to, label?}>
    let messages_value = data_fields
        .get("messages")
        .ok_or_else(|| "diagram_sequence: missing required field 'messages'".to_string())?;
    let messages_list = match messages_value {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_sequence: 'messages' must be List<Struct>, got {}",
                other.type_name()
            ));
        }
    };
    if messages_list.len() > SEQ_MAX_MESSAGES {
        return Err(format!(
            "diagram_sequence: too many messages ({}), maximum is {}",
            messages_list.len(),
            SEQ_MAX_MESSAGES
        ));
    }
    // Extract messages — validate that from/to reference known actors
    struct SeqMessage {
        from_idx: usize,
        to_idx: usize,
        label: Option<String>,
    }
    let mut messages: Vec<SeqMessage> = Vec::with_capacity(messages_list.len());
    for (i, item) in messages_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_sequence: messages[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_sequence message", f, "from")?;
        let to = struct_string_field("diagram_sequence message", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        let from_idx = actors.iter().position(|a| a == &from).ok_or_else(|| {
            format!(
                "diagram_sequence: messages[{}].from references unknown actor {:?}",
                i, from
            )
        })?;
        let to_idx = actors.iter().position(|a| a == &to).ok_or_else(|| {
            format!(
                "diagram_sequence: messages[{}].to references unknown actor {:?}",
                i, to
            )
        })?;
        messages.push(SeqMessage {
            from_idx,
            to_idx,
            label,
        });
    }

    // Geometry
    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n_actors = actors.len();
    // Evenly space actors across canvas width with side padding
    let pad_x = 60.0_f64;
    let usable_w = canvas_w - 2.0 * pad_x;
    let actor_step = if n_actors > 1 {
        usable_w / (n_actors as f64 - 1.0)
    } else {
        0.0
    };
    let actor_x: Vec<f64> = (0..n_actors)
        .map(|i| {
            if n_actors > 1 {
                pad_x + (i as f64) * actor_step
            } else {
                canvas_w / 2.0
            }
        })
        .collect();

    let lifeline_top = SEQ_TOP_PAD + SEQ_LIFELINE_HALF_H;
    let lifeline_bottom = canvas_h - SEQ_BOTTOM_PAD;
    // Message Y positions: distribute between lifeline_top+10 and lifeline_bottom-10
    let msg_top = lifeline_top + 20.0;
    let msg_bottom = lifeline_bottom - 10.0;
    let msg_step = if messages.is_empty() {
        0.0
    } else if messages.len() > 1 {
        (msg_bottom - msg_top) / (messages.len() as f64 - 1.0)
    } else {
        0.0
    };

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Lifelines + actor labels
    for (i, name) in actors.iter().enumerate() {
        let x = actor_x[i];
        // Actor head box — small rounded rect with name centered
        let head_w = 90.0_f64.min(actor_step.max(80.0) - 12.0).max(60.0);
        let head_x = x - head_w / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
            fmt_num(head_x),
            fmt_num(SEQ_TOP_PAD - SEQ_LIFELINE_HALF_H),
            fmt_num(head_w),
            fmt_num(2.0 * SEQ_LIFELINE_HALF_H),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(SEQ_TOP_PAD + 4.0),
            escape_attr(&ink),
            escape_html_chars(name)
        ));
        // Vertical dashed lifeline below the head box
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" stroke-dasharray="4 4" />"#,
            fmt_num(x),
            fmt_num(lifeline_top),
            fmt_num(x),
            fmt_num(lifeline_bottom),
            escape_attr(&rule)
        ));
    }

    // Messages — horizontal arrows from actor[from] to actor[to]
    for (i, msg) in messages.iter().enumerate() {
        let y = msg_top + (i as f64) * msg_step;
        let x1 = actor_x[msg.from_idx];
        let x2 = actor_x[msg.to_idx];
        // Skip self-messages (from==to) drawn as a small loop — for MVP
        // we still emit a tiny U-shaped arrow. Simpler: skip the line and
        // just place a small note. For correctness of the contract test
        // "messages not only between neighbors", we handle the diagonal
        // case (different actors) here.
        if msg.from_idx == msg.to_idx {
            // Self-message: small loop on the lifeline
            let loop_w = 24.0_f64;
            let loop_h = 14.0_f64;
            // Draw a tiny rectangular loop returning to the same lifeline
            parts.push(format!(
                r#"<path d="M {} {} L {} {} L {} {} L {} {}" fill="none" stroke="{}" stroke-width="1.5" />"#,
                fmt_num(x1),
                fmt_num(y),
                fmt_num(x1 + loop_w),
                fmt_num(y),
                fmt_num(x1 + loop_w),
                fmt_num(y + loop_h),
                fmt_num(x1),
                fmt_num(y + loop_h),
                escape_attr(&rule)
            ));
            // Arrowhead at the end (pointing left into the lifeline)
            parts.push(format!(
                r#"<path d="M {} {} L {} {} L {} {} Z" fill="{}" stroke="none" />"#,
                fmt_num(x1),
                fmt_num(y + loop_h),
                fmt_num(x1 + 7.0),
                fmt_num(y + loop_h - 3.0),
                fmt_num(x1 + 7.0),
                fmt_num(y + loop_h + 3.0),
                escape_attr(&rule)
            ));
        } else {
            // Different actors — connector from (x1, y) to (x2, y)
            parts.push(draw_connector(x1, y, x2, y, &style));
        }
        // Optional label above the arrow midpoint
        if let Some(label) = &msg.label {
            let mid_x = (x1 + x2) / 2.0;
            let label_y = y - 6.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(label_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // accent unused for now — kept for visual parity with other diagrams
    let _ = &accent;

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

// ── Наряд №83: Diagrams, part 3 — sets & comparisons ────────────────
//
// Five additional diagram builtins where area/intersection carries the
// meaning (as opposed to nodes/edges in Н81–82). One of them
// (diagram_medallion) reuses the existing svg_icon builtin's icon path
// table — no new icon geometry is invented here.
//
//   Block 1 — diagram_venn     (2 or 3 semi-transparent circles)
//   Block 2 — diagram_quadrant  (cross axes + scattered points in [-1,1]²)
//   Block 3 — diagram_pyramid   (stacked trapezoids, top = apex)
//   Block 4 — diagram_nested    (concentric circles, outer = first)
//   Block 5 — diagram_medallion (row of round badges, reuses svg_icon)
//
// All five reuse the DIAGRAM_CANVAS_W/H constants (600×400) defined above
// for visual consistency with the Н81–82 diagram suite.

// ── Block 1: diagram_venn ───────────────────────────────────────────
//
// Venn-style overlap diagram. DELIBERATELY restricted to 2 or 3 circles
// — the general N-circle case requires polygon intersection math and is
// explicitly out of scope (see the narazd spec: "Не решать общую задачу
// N-кругового Venn — строго 2 или 3, фиксированные симметричные позиции").
//
// Data shape:
//   Struct {
//     circles:      List<Struct { label: String, value: Float? }>,  // len == 2 or 3
//     overlap_label: String?,                                       // optional center label
//   }
//
// Geometry (fixed symmetric layouts — no overlap area computation):
//   - 2 circles: centers offset horizontally by ±0.3×radius from canvas
//     center; both have the same radius. The visible intersection is a
//     symmetric lens shape.
//   - 3 circles: centers at vertices of an equilateral triangle inscribed
//     in a circle of radius `0.7×R / √3` around the canvas center, where
//     R is the circle radius. Standard 3-set Venn layout — produces a
//     visible central triple-overlap region.
//
// Circles use semi-transparent accent fill (opacity 0.35) so overlap
// regions are visible as darker tones — same approach as chart_area's
// translucent fill. Labels render at a fixed offset from each circle's
// center (the spec explicitly says we don't compute non-overlapping
// label regions — MVP).
//
// Limits: circles.len() must be 2 or 3 (any other count → Err).

const VENN_CIRCLE_R: f64 = 110.0;
const VENN_2_OFFSET: f64 = 66.0; // 0.6 × R / 2 — centers at ±0.3R
const VENN_3_RING_R: f64 = 64.0; // ~ 0.7 × R / √3 — equilateral triangle circumradius

/// `diagram_venn(data, style) -> String`
///
/// Renders a 2- or 3-circle Venn diagram with semi-transparent fills.
/// `data` is `Struct { circles: List<Struct{label, value?}>, overlap_label? }`.
///
/// Returns Err if:
///   - circles.len() is not 2 or 3
///   - any circle is missing the `label` field
pub fn builtin_diagram_venn(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let data = match &data_value {
        Value::Struct { fields, .. } => fields.clone(),
        other => {
            return Err(format!(
                "diagram_venn: data must be Struct {{circles, overlap_label?}}, got {}",
                other.type_name()
            ));
        }
    };
    // Extract circles list
    let circles_value = match data.get("circles") {
        Some(Value::List(l)) => l.clone(),
        Some(other) => {
            return Err(format!(
                "diagram_venn: circles must be List<Struct{{label, value?}}>, got {}",
                other.type_name()
            ));
        }
        None => {
            return Err(
                "diagram_venn: missing required field 'circles' (List<Struct{label, value?}>)"
                    .to_string(),
            );
        }
    };
    // Validate count — explicitly restricted to 2 or 3 (no general N-case)
    if circles_value.len() != 2 && circles_value.len() != 3 {
        return Err(format!(
            "diagram_venn: supports exactly 2 or 3 circles, got {}",
            circles_value.len()
        ));
    }
    // Extract each circle's label and optional value
    struct VennCircle {
        label: String,
        value: Option<f64>,
    }
    let mut circles: Vec<VennCircle> = Vec::with_capacity(circles_value.len());
    for (i, item) in circles_value.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_venn: circles[{}] must be Struct {{label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_venn circle", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        circles.push(VennCircle { label, value });
    }
    // Optional overlap_label — top-level field, not inside the list
    let overlap_label = struct_opt_string_field(&data, "overlap_label");

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;

    // Compute circle centers for the requested layout.
    let centers: Vec<(f64, f64)> = match circles.len() {
        2 => vec![(cx - VENN_2_OFFSET, cy), (cx + VENN_2_OFFSET, cy)],
        3 => {
            // Equilateral triangle: angles 90°, 210°, 330° (measured from
            // +x axis) — but we want one vertex pointing UP, so we use
            // -π/2 (top), -π/2 + 2π/3 (lower-left), -π/2 + 4π/3 (lower-right).
            // Same orientation convention as chart_radar / diagram_loop.
            (0..3)
                .map(|i| {
                    let angle =
                        -std::f64::consts::PI / 2.0 + 2.0 * std::f64::consts::PI * (i as f64) / 3.0;
                    polar_to_xy(cx, cy, VENN_3_RING_R, angle)
                })
                .collect()
        }
        // Unreachable: count validated above, but the compiler doesn't know.
        _ => unreachable!("diagram_venn count validated above"),
    };

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Circles — semi-transparent accent fill so overlaps are visible as
    // darker tones. Stroke with accent at full opacity for crisp edges.
    for (i, (ccx, ccy)) in centers.iter().enumerate() {
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" fill-opacity="0.35" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(*ccx),
            fmt_num(*ccy),
            fmt_num(VENN_CIRCLE_R),
            escape_attr(&accent),
            escape_attr(&accent)
        ));
        // Circle label — placed at a fixed offset from the circle center,
        // AWAY from the canvas center, so the label sits outside the
        // densest overlap area. Direction = (center - canvas_center).
        let dx = *ccx - cx;
        let dy = *ccy - cy;
        let len = (dx * dx + dy * dy).sqrt();
        let (nx, ny) = if len < 0.001 {
            (0.0, -1.0) // fallback for 2-circle case where centers are horizontal
        } else {
            (dx / len, dy / len)
        };
        let label_x = *ccx + nx * (VENN_CIRCLE_R * 0.55);
        let label_y = ccy + ny * (VENN_CIRCLE_R * 0.55) + 4.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(label_x),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&circles[i].label)
        ));
        // Optional value — small muted text just below the label
        if let Some(v) = circles[i].value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(label_x),
                fmt_num(label_y + 14.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Optional overlap_label — at canvas center (the visual centroid of
    // all intersections for both 2- and 3-circle layouts).
    if let Some(ol) = &overlap_label {
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" font-style="italic" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(cy + 4.0),
            escape_attr(&muted),
            escape_html_chars(ol)
        ));
    }

    // Faint border around canvas — visual frame consistent with other diagrams
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 2: diagram_quadrant ───────────────────────────────────────
//
// 2×2 strategic quadrant chart (BCG-matrix style). Cross-shaped axes
// through the canvas center, points scattered in the [-1, 1] × [-1, 1]
// logical space.
//
// Data shape:
//   Struct {
//     x_axis_label: String,
//     y_axis_label: String,
//     items: List<Struct { label: String, x: Float, y: Float }>,
//   }
//
// Geometry:
//   - Horizontal axis: full-width line at canvas vertical center.
//   - Vertical axis: full-height line at canvas horizontal center.
//   - Axis labels: at the right end (x) and top end (y) of each axis.
//   - For each item: pixel_x = cx + x × half_w, pixel_y = cy − y × half_h
//     (y is inverted because SVG y grows downward). Circle marker +
//     label text anchored to the right of the marker.
//
// Limits:
//   - Any item.x or item.y outside [-1.0, 1.0] → Err
//   - items.len() > 20 → Err (points would be visually indistinguishable)

const QUADRANT_MAX_ITEMS: usize = 20;
const QUADRANT_HALF_W: f64 = 250.0; // cx ± half_w → x range [50, 550] on 600 canvas
const QUADRANT_HALF_H: f64 = 160.0; // cy ± half_h → y range [40, 360] on 400 canvas

/// `diagram_quadrant(data, style) -> String`
///
/// Renders a 2×2 quadrant chart with cross axes and scattered points.
/// `data` is `Struct { x_axis_label, y_axis_label, items: List<Struct{label, x, y}> }`.
///
/// Returns Err if:
///   - any item.x or item.y is outside [-1.0, 1.0]
///   - items.len() > 20
///   - missing required fields
pub fn builtin_diagram_quadrant(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let data = match &data_value {
        Value::Struct { fields, .. } => fields.clone(),
        other => {
            return Err(format!(
                "diagram_quadrant: data must be Struct {{x_axis_label, y_axis_label, items}}, got {}",
                other.type_name()
            ));
        }
    };
    // Both axis labels are TOP-LEVEL fields (not inside the items list) —
    // the spec explicitly calls this out as a category that's easy to
    // forget in the security scanner.
    let x_axis_label = struct_string_field("diagram_quadrant", &data, "x_axis_label")?;
    let y_axis_label = struct_string_field("diagram_quadrant", &data, "y_axis_label")?;
    let items_value = match data.get("items") {
        Some(Value::List(l)) => l.clone(),
        Some(other) => {
            return Err(format!(
                "diagram_quadrant: items must be List<Struct{{label, x, y}}>, got {}",
                other.type_name()
            ));
        }
        None => {
            return Err(
                "diagram_quadrant: missing required field 'items' (List<Struct{label, x, y}>)"
                    .to_string(),
            );
        }
    };
    if items_value.is_empty() {
        return Err("diagram_quadrant: items list must not be empty".to_string());
    }
    if items_value.len() > QUADRANT_MAX_ITEMS {
        return Err(format!(
            "diagram_quadrant: too many items ({}), maximum is {} — points would be visually indistinguishable",
            items_value.len(),
            QUADRANT_MAX_ITEMS
        ));
    }

    struct QuadItem {
        label: String,
        x: f64,
        y: f64,
    }
    let mut items: Vec<QuadItem> = Vec::with_capacity(items_value.len());
    for (i, item) in items_value.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_quadrant: items[{}] must be Struct {{label, x, y}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_quadrant item", f, "label")?;
        let x = struct_float_field("diagram_quadrant item", f, "x")?;
        let y = struct_float_field("diagram_quadrant item", f, "y")?;
        // Range check — explicit error per the spec
        if !(-1.0..=1.0).contains(&x) {
            return Err(format!(
                "diagram_quadrant: items[{}].x = {} is out of range [-1.0, 1.0]",
                i, x
            ));
        }
        if !(-1.0..=1.0).contains(&y) {
            return Err(format!(
                "diagram_quadrant: items[{}].y = {} is out of range [-1.0, 1.0]",
                i, y
            ));
        }
        items.push(QuadItem { label, x, y });
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

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Faint quadrant divider tint — very light accent wash to suggest the
    // four regions without obscuring the points. This is purely cosmetic;
    // the axes themselves carry the structural meaning.
    parts.push(format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" fill-opacity="0.05" />"#,
        fmt_num(cx),
        fmt_num(40.0),
        fmt_num(QUADRANT_HALF_W),
        fmt_num(QUADRANT_HALF_H),
        escape_attr(&accent)
    ));
    // Horizontal axis (x-axis) — through vertical center
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" />"#,
        fmt_num(40.0),
        fmt_num(cy),
        fmt_num(canvas_w - 40.0),
        fmt_num(cy),
        escape_attr(&rule)
    ));
    // Vertical axis (y-axis) — through horizontal center
    parts.push(format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1.5" />"#,
        fmt_num(cx),
        fmt_num(40.0),
        fmt_num(cx),
        fmt_num(canvas_h - 40.0),
        escape_attr(&rule)
    ));
    // Axis labels — at the ends of each axis (x: right end, y: top end).
    // These are top-level fields per the spec; they are NOT items[].label.
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}" text-anchor="end">{}</text>"#,
        fmt_num(canvas_w - 40.0),
        fmt_num(cy - 8.0),
        escape_attr(&ink),
        escape_html_chars(&x_axis_label)
    ));
    parts.push(format!(
        r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
        fmt_num(cx),
        fmt_num(32.0),
        escape_attr(&ink),
        escape_html_chars(&y_axis_label)
    ));

    // Items — circle marker + label
    for item in items.iter() {
        let px = cx + item.x * QUADRANT_HALF_W;
        // SVG y grows downward, so positive logical y → smaller pixel y
        let py = cy - item.y * QUADRANT_HALF_H;
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="5" fill="{}" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(px),
            fmt_num(py),
            escape_attr(&accent),
            escape_attr(&paper)
        ));
        // Label slightly offset to the right of the marker — simple MVP
        // placement (the spec doesn't require collision avoidance).
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}">{}</text>"#,
            fmt_num(px + 8.0),
            fmt_num(py + 4.0),
            escape_attr(&muted),
            escape_html_chars(&item.label)
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

// ── Block 3: diagram_pyramid ────────────────────────────────────────
//
// Stacked trapezoidal layers forming a pyramid. CRITICAL ORDERING RULE:
// the FIRST list element is the TOP (apex) — the narrowest layer. The
// LAST element is the BOTTOM (base) — the widest layer. This matches the
// natural top-down description of hierarchies (e.g. Maslow: "self-
// actualization" first = top of pyramid). DO NOT FLIP.
//
// Data shape:
//   List<Struct { label: String, value: Float? }>
//
// Geometry:
//   - Pyramid centered horizontally at cx = canvas_w / 2.
//   - Vertical extent: y ∈ [40, 360] (320 px tall, leaving 40 px margin
//     top/bottom on the 400 px canvas).
//   - Layer i (0-indexed from top):
//       top_y    = 40 + i × layer_h
//       bot_y    = 40 + (i+1) × layer_h
//       top_w    = (i / N) × max_w     ← linearly proportional to position
//       bot_w    = ((i+1) / N) × max_w
//     When i=0 (apex), top_w=0 → the apex is a single point (degenerate
//     trapezoid that's actually a triangle). This is the classic pyramid
//     silhouette.
//   - Trapezoid rendered as <path d="M ... L ... L ... L ... Z"> with
//     4 explicit corner points (NOT a <rect> — the spec forbids that).
//   - Label centered in each layer; optional value rendered as smaller
//     muted text just below the label.
//
// Limits:
//   - data.len() < 2 → Err (a 1-layer pyramid is meaningless)
//   - data.len() > 6 → Err (layers become too thin vertically)

const PYRAMID_MIN_LAYERS: usize = 2;
const PYRAMID_MAX_LAYERS: usize = 6;
const PYRAMID_TOP_Y: f64 = 40.0;
const PYRAMID_BOT_Y: f64 = 360.0;
const PYRAMID_MAX_W: f64 = 480.0; // canvas_w − 2 × 60 margin

/// `diagram_pyramid(data, style) -> String`
///
/// Renders a pyramid of stacked trapezoids. `data` is
/// `List<Struct{label, value?}>`. **The first element is the TOP (apex)
/// of the pyramid** — see the ordering rule in the file-level comment.
///
/// Returns Err if:
///   - data.len() < 2 or > 6
pub fn builtin_diagram_pyramid(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_pyramid", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.len() < PYRAMID_MIN_LAYERS {
        return Err(format!(
            "diagram_pyramid: too few layers ({}), minimum is {} — a single-layer pyramid is meaningless",
            data.len(),
            PYRAMID_MIN_LAYERS
        ));
    }
    if data.len() > PYRAMID_MAX_LAYERS {
        return Err(format!(
            "diagram_pyramid: too many layers ({}), maximum is {} — layers would become too thin vertically",
            data.len(),
            PYRAMID_MAX_LAYERS
        ));
    }
    struct PyramidLayer {
        label: String,
        value: Option<f64>,
    }
    let mut layers: Vec<PyramidLayer> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_pyramid: data[{}] must be Struct {{label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_pyramid item", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        layers.push(PyramidLayer { label, value });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let cx = canvas_w / 2.0;
    let n = layers.len();
    let pyramid_h = PYRAMID_BOT_Y - PYRAMID_TOP_Y;
    let layer_h = pyramid_h / (n as f64);

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Layers — top-down order is critical: layers[0] is the apex (top).
    for (i, layer) in layers.iter().enumerate() {
        let top_y = PYRAMID_TOP_Y + (i as f64) * layer_h;
        let bot_y = PYRAMID_TOP_Y + ((i + 1) as f64) * layer_h;
        // Linearly proportional widths: top_w = (i/N) × max_w
        // For i=0 (apex): top_w = 0 → triangle silhouette
        let top_w = (i as f64 / n as f64) * PYRAMID_MAX_W;
        let bot_w = ((i + 1) as f64 / n as f64) * PYRAMID_MAX_W;
        // 4 corner points of the trapezoid (clockwise from top-left)
        let tl_x = cx - top_w / 2.0;
        let tr_x = cx + top_w / 2.0;
        let br_x = cx + bot_w / 2.0;
        let bl_x = cx - bot_w / 2.0;
        // Alternate fill: even layers get accent at low opacity, odd get
        // muted at low opacity — visual differentiation without heavy
        // color noise. (Same alternating pattern as diagram_layers.)
        let fill_color = if i % 2 == 0 { &accent } else { &muted };
        // Trapezoid as <path> with 4 explicit points + Z (NOT a <rect>).
        parts.push(format!(
            r#"<path d="M {} {} L {} {} L {} {} L {} {} Z" fill="{}" fill-opacity="0.18" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(tl_x),
            fmt_num(top_y),
            fmt_num(tr_x),
            fmt_num(top_y),
            fmt_num(br_x),
            fmt_num(bot_y),
            fmt_num(bl_x),
            fmt_num(bot_y),
            escape_attr(fill_color),
            escape_attr(fill_color)
        ));
        // Label — centered horizontally, vertically at the middle of the
        // layer. For very narrow apex layers (i=0 with top_w=0), the
        // label may overflow horizontally — we accept that as MVP.
        let label_y = (top_y + bot_y) / 2.0 + 4.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&layer.label)
        ));
        // Optional value — small muted text below the label
        if let Some(v) = layer.value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(label_y + 14.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Faint canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 4: diagram_nested ─────────────────────────────────────────
//
// Concentric circles — outermost ring is the FIRST list element,
// innermost is the LAST. Useful for "onion" or "scope" diagrams where
// each layer wraps the ones inside it.
//
// Data shape:
//   List<Struct { label: String, value: Float? }>
//
// Geometry:
//   - All circles centered at canvas center (cx, cy).
//   - Outermost radius = max_r; innermost = max_r / N (linear steps).
//   - Ring i radius: r_i = max_r × (N − i) / N
//     so r_0 = max_r (outermost) and r_{N-1} = max_r / N (innermost).
//   - Light fill so inner circles remain visible — alternating accent
//     and muted at very low opacity (0.08). Stroke at full opacity.
//   - Labels placed at the top of each ring (12 o'clock position),
//     stacked vertically as the rings get smaller. This is the MVP
//     placement — pointer lines to a side legend would be the polished
//     version, deferred per the spec ("решить по месту — MVP").
//
// Limits:
//   - data.len() > 5 → Err (rings become indistinguishably thin)

const NESTED_MAX_RINGS: usize = 5;
const NESTED_MAX_R: f64 = 160.0; // limited by canvas_h/2 − 40 margin

/// `diagram_nested(data, style) -> String`
///
/// Renders concentric circles. `data` is `List<Struct{label, value?}>`.
/// The FIRST element is the OUTERMOST ring; the LAST is the innermost.
///
/// Returns Err if:
///   - data is empty
///   - data.len() > 5
pub fn builtin_diagram_nested(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_nested", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_nested: data list must not be empty".to_string());
    }
    if data.len() > NESTED_MAX_RINGS {
        return Err(format!(
            "diagram_nested: too many rings ({}), maximum is {} — rings would become indistinguishably thin",
            data.len(),
            NESTED_MAX_RINGS
        ));
    }
    struct NestedRing {
        label: String,
        value: Option<f64>,
    }
    let mut rings: Vec<NestedRing> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_nested: data[{}] must be Struct {{label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let label = struct_string_field("diagram_nested item", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        rings.push(NestedRing { label, value });
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
    let n = rings.len();

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    // Rings — draw OUTERMOST first (so inner rings render on top of it).
    // rings[0] = outermost, so iterating in natural order gives the right
    // z-order: outer fill is laid down first, inner strokes overwrite it.
    for (i, ring) in rings.iter().enumerate() {
        let r = NESTED_MAX_R * (n as f64 - i as f64) / (n as f64);
        // Alternating fill — very low opacity so nested rings remain
        // distinguishable without darkening the center excessively.
        let fill_color = if i % 2 == 0 { &accent } else { &muted };
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" fill-opacity="0.08" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(cx),
            fmt_num(cy),
            fmt_num(r),
            escape_attr(fill_color),
            escape_attr(fill_color)
        ));
        // Label at top of each ring (12 o'clock position). Stacked
        // vertically as rings get smaller — simple MVP placement.
        let label_y = cy - r + 14.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&ring.label)
        ));
        // Optional value — small muted text below the label
        if let Some(v) = ring.value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="9" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(label_y + 12.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Faint canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 5: diagram_medallion ──────────────────────────────────────
//
// Row of round "medallion" badges. Each medallion optionally contains
// an icon (reusing the existing svg_icon builtin's path table — see
// icon_path_data) or the first letter of the label as a fallback.
//
// Data shape:
//   List<Struct { icon: String?, label: String, value: Float? }>
//
// `icon` is optional. When present, it MUST be one of the 10 known
// svg_icon names (server, laptop, phone, database, cloud, arrow-right,
// check, warning, user, document). Unknown names produce the SAME ERROR
// TEXT that svg_icon itself produces — we deliberately reuse icon_path_data
// for validation rather than duplicating the name list here (per the
// spec: "не дублировать список имён иконок").
//
// Geometry:
//   - Row of N medallions, centered horizontally on the canvas.
//   - Each medallion: 60 px diameter circle, 24 px gap between centers.
//   - Medallion center Y = 160 (leaves room for label + value below).
//   - If icon specified: 24×24 icon centered inside the medallion.
//   - Else: first character of label, large bold text, centered.
//   - Label below medallion (12 px font, centered).
//   - Optional value: smaller muted text below label.
//
// Limits:
//   - data.len() > 6 → Err (medallions won't fit horizontally)

const MEDALLION_MAX_ITEMS: usize = 6;
const MEDALLION_D: f64 = 60.0; // diameter
const MEDALLION_GAP: f64 = 24.0; // center-to-center gap above diameter
const MEDALLION_CY: f64 = 160.0;
const MEDALLION_ICON_SIZE: f64 = 28.0; // icon fits inside the 60px circle

/// `diagram_medallion(data, style) -> String`
///
/// Renders a row of round medallion badges. `data` is
/// `List<Struct{icon?, label, value?}>`. Icons reuse the svg_icon builtin's
/// validation (no name list duplication).
///
/// Returns Err if:
///   - data is empty
///   - data.len() > 6
///   - icon is present but not one of the 10 known names (error text
///     comes from icon_path_data's validation, identical to svg_icon)
pub fn builtin_diagram_medallion(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("diagram_medallion", args, 0)?;
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    if data.is_empty() {
        return Err("diagram_medallion: data list must not be empty".to_string());
    }
    if data.len() > MEDALLION_MAX_ITEMS {
        return Err(format!(
            "diagram_medallion: too many items ({}), maximum is {} — medallions won't fit horizontally on the canvas",
            data.len(),
            MEDALLION_MAX_ITEMS
        ));
    }
    struct Medallion {
        icon: Option<String>,
        label: String,
        value: Option<f64>,
    }
    let mut medallions: Vec<Medallion> = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_medallion: data[{}] must be Struct {{icon?, label, value?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let icon = struct_opt_string_field(f, "icon");
        let label = struct_string_field("diagram_medallion item", f, "label")?;
        let value = struct_opt_float_field(f, "value");
        // Validate icon name by reusing icon_path_data — this is the
        // spec-mandated "don't duplicate the icon name list" pattern.
        // If icon is Some(name) and name is unknown, we return the same
        // shape of error that builtin_svg_icon would, but with the
        // diagram_medallion: prefix so callers can attribute the failure
        // to the builtin they actually called.
        if let Some(ref name) = icon {
            if icon_path_data(name).is_none() {
                return Err(format!(
                    "diagram_medallion: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                    name
                ));
            }
        }
        medallions.push(Medallion { icon, label, value });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n = medallions.len();
    // total_width = N × D + (N−1) × GAP (center-to-center spacing = D + GAP)
    let total_width = (n as f64) * MEDALLION_D + ((n as f64) - 1.0) * MEDALLION_GAP;
    let start_x = (canvas_w - total_width) / 2.0 + MEDALLION_D / 2.0;

    let mut parts: Vec<String> = Vec::new();
    // Background
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));

    for (i, m) in medallions.iter().enumerate() {
        let cx = start_x + (i as f64) * (MEDALLION_D + MEDALLION_GAP);
        let cy = MEDALLION_CY;
        let r = MEDALLION_D / 2.0;
        // Medallion circle — paper fill, accent stroke (visual emphasis)
        parts.push(format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="2" />"#,
            fmt_num(cx),
            fmt_num(cy),
            fmt_num(r),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        // Content: icon (if specified) OR first letter of label (fallback).
        if let Some(ref icon_name) = m.icon {
            // Reuse icon_path_data with proper error propagation — same
            // pattern as builtin_svg_icon (line 2879). The early validation
            // in the parse loop above already rejects unknown names, so this
            // branch is effectively unreachable for valid inputs; but we
            // still propagate via `?` rather than `unwrap()` because the
            // project denies clippy::unwrap_used unconditionally.
            let path_data = icon_path_data(icon_name).ok_or_else(|| {
                format!(
                    "diagram_medallion: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                    icon_name
                )
            })?;
            let scale = MEDALLION_ICON_SIZE / 24.0;
            let icon_x = cx - MEDALLION_ICON_SIZE / 2.0;
            let icon_y = cy - MEDALLION_ICON_SIZE / 2.0;
            // Inline the same <svg> wrapper that builtin_svg_icon produces,
            // so the icon is positioned correctly inside the medallion.
            // We do NOT call builtin_svg_icon directly because it returns
            // a Value::String (extra unwrap) and we already have the path
            // data from the validation step above.
            parts.push(format!(
                r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 24 24"><g transform="scale({})"><path d="{}" stroke="{}" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></g></svg>"#,
                fmt_num(icon_x),
                fmt_num(icon_y),
                fmt_num(MEDALLION_ICON_SIZE),
                fmt_num(MEDALLION_ICON_SIZE),
                fmt_num(scale),
                path_data,
                escape_attr(&ink)
            ));
        } else {
            // Fallback: first character of the label, large bold text.
            // Char-based slicing is safe here because m.label is a valid
            // UTF-8 String; chars().next() gives us the first grapheme.
            let first_char = m.label.chars().next().unwrap_or('?');
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="24" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(cy + 8.0),
                escape_attr(&accent),
                escape_html_chars(&first_char.to_string())
            ));
        }
        // Label below medallion
        let label_y = cy + r + 18.0;
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(cx),
            fmt_num(label_y),
            escape_attr(&ink),
            escape_html_chars(&m.label)
        ));
        // Optional value — smaller muted text below label
        if let Some(v) = m.value {
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(cx),
                fmt_num(label_y + 14.0),
                escape_attr(&muted),
                escape_html_chars(&fmt_num(v))
            ));
        }
    }

    // Faint canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Наряд №84: Diagrams, part 4 — data & state ───────────────────────
//
// Six new diagram builtins (the largest single narazd in the diagram
// series). Three of them (data_flow, high_level, architecture) share
// the same `Struct{nodes, edges}` graph shape and the generalized
// topological_layers / bfs_layers_with_cycles helpers from Block 4;
// they differ only in (a) which layer function they call and (b) how
// they render each node box (plain rect, large labeled block, or
// block with svg_icon).
//
//   Block 1 — diagram_er         (entity-relation grid; no graph layout)
//   Block 2 — diagram_state      (state machine, BFS layout, cycles OK)
//   Block 3 — diagram_swimlane   (vertical lanes, steps positioned by order)
//   Block 5 — diagram_data_flow  (graph, BFS layout, cycles OK)
//   Block 6 — diagram_high_level + diagram_architecture
//                              (graph, topological, no cycles; arch has icons)
//
// All reuse DIAGRAM_CANVAS_W/H (600×400) for visual consistency with
// the Н81–83 diagram suite.

// ── Block 1: diagram_er ─────────────────────────────────────────────
//
// Entity-Relationship diagram. Each entity is a rectangle split into
// a header (entity name) and a body listing its fields. Relations are
// drawn as connectors between entity box edges, with the optional
// relation label (e.g. "1:N", "1:1") placed at the line midpoint.
//
// Layout is a SIMPLE GRID (3 per row) — the spec is explicit:
// "Не решать общую задачу graph layout для diagram_er — простая сетка,
// не анализ связей для позиционирования." ER diagrams routinely
// contain cycles (bidirectional relationships, many-to-many), so the
// topological sort from Block 4 doesn't apply here.
//
// Limits: entities.len() ≤ 12, fields.len() ≤ 8 per entity.

const ER_MAX_ENTITIES: usize = 12;
const ER_MAX_FIELDS: usize = 8;
const ER_PER_ROW: usize = 3;
const ER_BOX_W: f64 = 160.0;
const ER_BOX_HEADER_H: f64 = 22.0;
const ER_FIELD_H: f64 = 14.0;
const ER_BOX_PADDING: f64 = 14.0;
const ER_GRID_GAP_X: f64 = 30.0;
const ER_GRID_GAP_Y: f64 = 30.0;
const ER_GRID_TOP: f64 = 30.0;

struct ErEntity {
    name: String,
    fields: Vec<String>,
}

struct ErRelation {
    from: String,
    to: String,
    label: Option<String>,
}

/// `diagram_er(data, style) -> String`
///
/// `data` is `Struct { entities: List<Struct{name, fields: List<String>}>,
/// relations: List<Struct{from, to, label?}> }`.
///
/// Returns Err if entities.len() > 12, fields.len() > 8 for any entity,
/// or a relation endpoint doesn't match any entity name.
pub fn builtin_diagram_er(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_er: data must be Struct {{entities, relations}}, got {}",
                other.type_name()
            ));
        }
    };
    let entities_val = fields
        .get("entities")
        .ok_or_else(|| "diagram_er: missing 'entities' field".to_string())?;
    let relations_val = fields
        .get("relations")
        .ok_or_else(|| "diagram_er: missing 'relations' field".to_string())?;
    let entities_list = match entities_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_er: 'entities' must be List, got {}",
                other.type_name()
            ));
        }
    };
    let relations_list = match relations_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_er: 'relations' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if entities_list.is_empty() {
        return Err("diagram_er: entities list must not be empty".to_string());
    }
    if entities_list.len() > ER_MAX_ENTITIES {
        return Err(format!(
            "diagram_er: too many entities ({}), maximum is {} — grid would overflow the canvas",
            entities_list.len(),
            ER_MAX_ENTITIES
        ));
    }
    let mut entities: Vec<ErEntity> = Vec::with_capacity(entities_list.len());
    let mut entity_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in entities_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_er: entities[{}] must be Struct {{name, fields}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let name = struct_string_field("diagram_er entity", f, "name")?;
        if !entity_names.insert(name.clone()) {
            return Err(format!(
                "diagram_er: duplicate entity name {:?} at entities[{}]",
                name, i
            ));
        }
        let fields_list = match f.get("fields") {
            Some(Value::List(items)) => items,
            Some(other) => {
                return Err(format!(
                    "diagram_er: entities[{}].fields must be List<String>, got {}",
                    i,
                    other.type_name()
                ));
            }
            None => {
                return Err(format!(
                    "diagram_er: entities[{}] missing required 'fields' field",
                    i
                ));
            }
        };
        if fields_list.len() > ER_MAX_FIELDS {
            return Err(format!(
                "diagram_er: entities[{}].fields has {} entries, maximum is {} — box would be too tall",
                i,
                fields_list.len(),
                ER_MAX_FIELDS
            ));
        }
        let mut fields_vec: Vec<String> = Vec::with_capacity(fields_list.len());
        for (j, f_item) in fields_list.iter().enumerate() {
            let field_name = match f_item {
                Value::String(s) => s.clone(),
                other => {
                    return Err(format!(
                        "diagram_er: entities[{}].fields[{}] must be String, got {}",
                        i,
                        j,
                        other.type_name()
                    ));
                }
            };
            fields_vec.push(field_name);
        }
        entities.push(ErEntity {
            name,
            fields: fields_vec,
        });
    }
    let mut relations: Vec<ErRelation> = Vec::with_capacity(relations_list.len());
    for (i, item) in relations_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_er: relations[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_er relation", f, "from")?;
        let to = struct_string_field("diagram_er relation", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        if !entity_names.contains(&from) {
            return Err(format!(
                "diagram_er: relations[{}].from references unknown entity {:?}",
                i, from
            ));
        }
        if !entity_names.contains(&to) {
            return Err(format!(
                "diagram_er: relations[{}].to references unknown entity {:?}",
                i, to
            ));
        }
        relations.push(ErRelation { from, to, label });
    }

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    // Simple grid: ER_PER_ROW entities per row, fixed box width.
    let n = entities.len();
    let n_rows = n.div_ceil(ER_PER_ROW).max(1);
    // Compute box heights (depend on field count per entity).
    let box_height =
        |e: &ErEntity| ER_BOX_HEADER_H + (e.fields.len() as f64) * ER_FIELD_H + ER_BOX_PADDING;
    let row_h: Vec<f64> = (0..n_rows)
        .map(|r| {
            (0..ER_PER_ROW)
                .filter_map(|c| {
                    let idx = r * ER_PER_ROW + c;
                    if idx < n {
                        Some(box_height(&entities[idx]))
                    } else {
                        None
                    }
                })
                .fold(0.0_f64, f64::max)
        })
        .collect();
    let total_grid_h: f64 = row_h.iter().sum::<f64>() + (n_rows as f64 - 1.0) * ER_GRID_GAP_Y;
    // Center the grid vertically.
    let grid_top = ((canvas_h - total_grid_h) / 2.0).max(ER_GRID_TOP);
    // Position each entity box on the grid.
    let mut name_to_box: std::collections::HashMap<String, (f64, f64, f64, f64)> =
        std::collections::HashMap::new();
    let mut cursor_y = grid_top;
    let total_row_w = (ER_PER_ROW as f64) * ER_BOX_W + ((ER_PER_ROW as f64) - 1.0) * ER_GRID_GAP_X;
    let grid_left = ((canvas_w - total_row_w) / 2.0).max(ER_GRID_GAP_X);
    for (r, row_max_h) in row_h.iter().enumerate() {
        for c in 0..ER_PER_ROW {
            let idx = r * ER_PER_ROW + c;
            if idx >= n {
                break;
            }
            let x = grid_left + (c as f64) * (ER_BOX_W + ER_GRID_GAP_X);
            let h = box_height(&entities[idx]);
            // Vertically center each box in its row cell.
            let y = cursor_y + ((*row_max_h) - h) / 2.0;
            name_to_box.insert(entities[idx].name.clone(), (x, y, ER_BOX_W, h));
        }
        cursor_y += *row_max_h + ER_GRID_GAP_Y;
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Relations first (so entity boxes render on top of any clipped line).
    for rel in &relations {
        let (fx, fy, fw, fh) = name_to_box.get(&rel.from).cloned().ok_or_else(|| {
            format!(
                "diagram_er: internal error — entity {:?} not in position map",
                rel.from
            )
        })?;
        let (tx, ty, tw, th) = name_to_box.get(&rel.to).cloned().ok_or_else(|| {
            format!(
                "diagram_er: internal error — entity {:?} not in position map",
                rel.to
            )
        })?;
        // Use box centers as connector endpoints; box_edge_point trims
        // the line back to the actual box boundary.
        let from_cx = fx + fw / 2.0;
        let from_cy = fy + fh / 2.0;
        let to_cx = tx + tw / 2.0;
        let to_cy = ty + th / 2.0;
        let (sx, sy) = box_edge_point(from_cx, from_cy, to_cx, to_cy, fw, fh);
        let (ex, ey) = box_edge_point(to_cx, to_cy, from_cx, from_cy, tw, th);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &rel.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 6.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // Entity boxes
    for entity in &entities {
        let (x, y, w, h) = name_to_box.get(&entity.name).cloned().ok_or_else(|| {
            format!(
                "diagram_er: internal error — entity {:?} not in position map (render pass)",
                entity.name
            )
        })?;
        // Box body (paper fill, rule stroke).
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(w),
            fmt_num(h),
            escape_attr(&paper),
            escape_attr(&rule)
        ));
        // Header bar (accent fill, paper text) — visually separates name from fields.
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
            fmt_num(x),
            fmt_num(y),
            fmt_num(w),
            fmt_num(ER_BOX_HEADER_H),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x + w / 2.0),
            fmt_num(y + ER_BOX_HEADER_H - 6.0),
            escape_attr(&paper),
            escape_html_chars(&entity.name)
        ));
        // Fields listed below the header, one per line.
        for (i, field) in entity.fields.iter().enumerate() {
            let fy = y + ER_BOX_HEADER_H + (i as f64 + 1.0) * ER_FIELD_H;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="11" fill="{}">{}</text>"#,
                fmt_num(x + 8.0),
                fmt_num(fy),
                escape_attr(&ink),
                escape_html_chars(field)
            ));
        }
    }
    // Canvas border
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 2: diagram_state ──────────────────────────────────────────
//
// State machine diagram. States are rendered as rounded rectangles
// (radius 14 — visually distinct from flowchart's 4 to signal "this is
// a state, not a step"). Transitions use draw_connector; self-loops
// (A→A) are VALID here (a common state-machine construct: a state that
// transitions to itself on a specific event) — unlike diagram_flowchart
// where a self-loop is rejected as a trivial cycle.
//
// Layout: BFS from the `initial` state (or first state if `initial` not
// specified), treating edges as undirected for layering — this lets
// cyclic state machines lay out sanely. See bfs_layers_with_cycles
// (Block 4) for the algorithm.
//
// If `initial` is specified, we draw a small "entry arrow" — a short
// arrow with no source, terminating at the initial state's left edge.
// This is the classical state-machine notation for "the start state".
//
// Limits: states.len() ≤ 10.

const STATE_MAX_STATES: usize = 10;
const STATE_NODE_W: f64 = 110.0;
const STATE_NODE_H: f64 = 40.0;
const STATE_NODE_RX: f64 = 14.0;

struct StateTransition {
    from: String,
    to: String,
    label: Option<String>,
}

/// `diagram_state(data, style) -> String`
///
/// `data` is `Struct { states: List<String>, transitions: List<Struct{from, to, label?}>, initial: String? }`.
///
/// Cycles and self-loops in transitions are VALID (state machines are
/// inherently cyclic). `initial`, if specified, must be one of `states`.
pub fn builtin_diagram_state(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_state: data must be Struct {{states, transitions, initial?}}, got {}",
                other.type_name()
            ));
        }
    };
    let states_val = fields
        .get("states")
        .ok_or_else(|| "diagram_state: missing 'states' field".to_string())?;
    let transitions_val = fields
        .get("transitions")
        .ok_or_else(|| "diagram_state: missing 'transitions' field".to_string())?;
    let states_list = match states_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_state: 'states' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };
    let transitions_list = match transitions_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_state: 'transitions' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if states_list.is_empty() {
        return Err("diagram_state: states list must not be empty".to_string());
    }
    if states_list.len() > STATE_MAX_STATES {
        return Err(format!(
            "diagram_state: too many states ({}), maximum is {} — diagram would be unreadable",
            states_list.len(),
            STATE_MAX_STATES
        ));
    }
    let mut states: Vec<String> = Vec::with_capacity(states_list.len());
    let mut state_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in states_list.iter().enumerate() {
        let name = match item {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "diagram_state: states[{}] must be String, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if !state_set.insert(name.clone()) {
            return Err(format!(
                "diagram_state: duplicate state name {:?} at states[{}]",
                name, i
            ));
        }
        states.push(name);
    }
    let mut transitions: Vec<StateTransition> = Vec::with_capacity(transitions_list.len());
    for (i, item) in transitions_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_state: transitions[{}] must be Struct {{from, to, label?}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let from = struct_string_field("diagram_state transition", f, "from")?;
        let to = struct_string_field("diagram_state transition", f, "to")?;
        let label = struct_opt_string_field(f, "label");
        if !state_set.contains(&from) {
            return Err(format!(
                "diagram_state: transitions[{}].from references unknown state {:?}",
                i, from
            ));
        }
        if !state_set.contains(&to) {
            return Err(format!(
                "diagram_state: transitions[{}].to references unknown state {:?}",
                i, to
            ));
        }
        transitions.push(StateTransition { from, to, label });
    }
    let initial = struct_opt_string_field(fields, "initial");
    if let Some(ref init) = initial {
        if !state_set.contains(init) {
            return Err(format!(
                "diagram_state: initial {:?} is not in states list",
                init
            ));
        }
    }
    let root = initial
        .clone()
        .unwrap_or_else(|| states.first().cloned().unwrap_or_default());
    // Build edge pairs for the layering function.
    let edge_pairs: Vec<(String, String)> = transitions
        .iter()
        .map(|t| (t.from.clone(), t.to.clone()))
        .collect();
    let layers = bfs_layers_with_cycles(&states, &edge_pairs, &root);

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n_layers = layers.len();
    let layer_h = (canvas_h - 80.0) / (n_layers as f64).max(1.0);
    let mut id_to_pos: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::new();
    for (layer_idx, layer_states) in layers.iter().enumerate() {
        let count = layer_states.len();
        let y_center = 40.0 + (layer_idx as f64 + 0.5) * layer_h;
        let total_w = canvas_w - 80.0;
        let step = if count > 1 {
            total_w / (count as f64 - 1.0)
        } else {
            0.0
        };
        let start_x = if count > 1 { 40.0 } else { canvas_w / 2.0 };
        for (i, id) in layer_states.iter().enumerate() {
            let x_center = start_x + (i as f64) * step;
            id_to_pos.insert(id.clone(), (x_center, y_center));
        }
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Entry arrow for `initial` — short horizontal arrow ending at the
    // state's left edge, with no source (visually "from nowhere").
    if let Some(ref init) = initial {
        if let Some(&(cx, cy)) = id_to_pos.get(init) {
            let ex = cx - STATE_NODE_W / 2.0;
            let sx = ex - 30.0;
            parts.push(draw_connector(sx, cy, ex, cy, &style));
        }
    }
    // Transitions — including self-loops (A→A), which we render as a
    // small curved arrow above the state box. Self-loops are VALID in
    // state machines (unlike flowchart where they're a hard error).
    for t in &transitions {
        if t.from == t.to {
            // Self-loop: small loop above the node.
            if let Some(&(cx, cy)) = id_to_pos.get(&t.from) {
                let top_y = cy - STATE_NODE_H / 2.0;
                let loop_r = 12.0;
                let arc_cx = cx;
                let arc_cy = top_y - loop_r;
                // Half-circle path from left base to right base, drawn
                // ABOVE the node. Arrowhead points down at the right base.
                parts.push(format!(
                    r#"<path d="M {} {} A {} {} 0 0 1 {} {}" fill="none" stroke="{}" stroke-width="1.5" />"#,
                    fmt_num(cx - loop_r),
                    fmt_num(top_y),
                    fmt_num(loop_r),
                    fmt_num(loop_r),
                    fmt_num(cx + loop_r),
                    fmt_num(top_y),
                    escape_attr(&rule)
                ));
                // Arrowhead at the right base, pointing down into the node.
                parts.push(format!(
                    r#"<path d="M {} {} L {} {} L {} {} Z" fill="{}" stroke="none" />"#,
                    fmt_num(cx + loop_r),
                    fmt_num(top_y),
                    fmt_num(cx + loop_r - 4.0),
                    fmt_num(top_y - 6.0),
                    fmt_num(cx + loop_r + 4.0),
                    fmt_num(top_y - 6.0),
                    escape_attr(&rule)
                ));
                if let Some(label) = &t.label {
                    parts.push(format!(
                        r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                        fmt_num(arc_cx),
                        fmt_num(arc_cy - 4.0),
                        escape_attr(&muted),
                        escape_html_chars(label)
                    ));
                }
            }
            continue;
        }
        let (from_x, from_y) = id_to_pos.get(&t.from).cloned().ok_or_else(|| {
            format!(
                "diagram_state: internal error — state {:?} not in position map",
                t.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&t.to).cloned().ok_or_else(|| {
            format!(
                "diagram_state: internal error — state {:?} not in position map",
                t.to
            )
        })?;
        let (sx, sy) = box_edge_point(from_x, from_y, to_x, to_y, STATE_NODE_W, STATE_NODE_H);
        let (ex, ey) = box_edge_point(to_x, to_y, from_x, from_y, STATE_NODE_W, STATE_NODE_H);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &t.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 6.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // State boxes (rounded rects, larger rx than flowchart for visual distinction).
    for (id, (cx, cy)) in &id_to_pos {
        let is_initial = initial.as_deref() == Some(id.as_str());
        let box_x = cx - STATE_NODE_W / 2.0;
        let box_y = cy - STATE_NODE_H / 2.0;
        // Initial state gets an accent border + bolder outline (visual emphasis).
        let stroke = if is_initial { &accent } else { &rule };
        let stroke_w = if is_initial { 2.5 } else { 1.5 };
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(STATE_NODE_W),
            fmt_num(STATE_NODE_H),
            fmt_num(STATE_NODE_RX),
            fmt_num(STATE_NODE_RX),
            escape_attr(&paper),
            escape_attr(stroke),
            fmt_num(stroke_w)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(id)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 3: diagram_swimlane ───────────────────────────────────────
//
// Swimlane diagram. Lanes are horizontal bands stacked vertically
// (similar to diagram_layers from Н81, but here each band contains
// positioned steps rather than a single label). Each lane has a name
// label on the left; steps within a lane are positioned horizontally
// by their `order` field (a Float — NOT their list index), so steps
// in different lanes can be visually aligned by time/order.
//
// `order` values across all steps are normalized to [0, 1] for x
// positioning: min_order → x=left_padding, max_order → x=right_padding.
// Steps in the same lane at the same order would overlap — we don't
// de-duplicate, the caller is responsible for sensible input.
//
// Optional connectors between consecutive-by-order steps in the same
// lane: if two steps share a lane and have consecutive order values,
// we draw a faint dashed arrow between them. This makes the temporal
// flow visible without cluttering cross-lane relationships.
//
// Limits: lanes.len() ≤ 6, steps.len() ≤ 30.

const SWIMLANE_MAX_LANES: usize = 6;
const SWIMLANE_MAX_STEPS: usize = 30;
const SWIMLANE_LABEL_W: f64 = 80.0;
const SWIMLANE_PAD_X: f64 = 16.0;
const SWIMLANE_PAD_Y: f64 = 16.0;

struct SwimlaneStep {
    lane: String,
    label: String,
    order: f64,
}

/// `diagram_swimlane(data, style) -> String`
///
/// `data` is `Struct { lanes: List<String>, steps: List<Struct{lane, label, order}> }`.
/// `order` is a Float — NOT the list index — that determines horizontal
/// position (so steps in different lanes can be aligned by time).
pub fn builtin_diagram_swimlane(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let fields = match &data_value {
        Value::Struct { fields, .. } => fields,
        other => {
            return Err(format!(
                "diagram_swimlane: data must be Struct {{lanes, steps}}, got {}",
                other.type_name()
            ));
        }
    };
    let lanes_val = fields
        .get("lanes")
        .ok_or_else(|| "diagram_swimlane: missing 'lanes' field".to_string())?;
    let steps_val = fields
        .get("steps")
        .ok_or_else(|| "diagram_swimlane: missing 'steps' field".to_string())?;
    let lanes_list = match lanes_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_swimlane: 'lanes' must be List<String>, got {}",
                other.type_name()
            ));
        }
    };
    let steps_list = match steps_val {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "diagram_swimlane: 'steps' must be List, got {}",
                other.type_name()
            ));
        }
    };
    if lanes_list.is_empty() {
        return Err("diagram_swimlane: lanes list must not be empty".to_string());
    }
    if lanes_list.len() > SWIMLANE_MAX_LANES {
        return Err(format!(
            "diagram_swimlane: too many lanes ({}), maximum is {} — lanes would be too narrow",
            lanes_list.len(),
            SWIMLANE_MAX_LANES
        ));
    }
    if steps_list.len() > SWIMLANE_MAX_STEPS {
        return Err(format!(
            "diagram_swimlane: too many steps ({}), maximum is {}",
            steps_list.len(),
            SWIMLANE_MAX_STEPS
        ));
    }
    let mut lanes: Vec<String> = Vec::with_capacity(lanes_list.len());
    let mut lane_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, item) in lanes_list.iter().enumerate() {
        let name = match item {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "diagram_swimlane: lanes[{}] must be String, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if !lane_set.insert(name.clone()) {
            return Err(format!(
                "diagram_swimlane: duplicate lane name {:?} at lanes[{}]",
                name, i
            ));
        }
        lanes.push(name);
    }
    let mut steps: Vec<SwimlaneStep> = Vec::with_capacity(steps_list.len());
    for (i, item) in steps_list.iter().enumerate() {
        let f = match item {
            Value::Struct { fields, .. } => fields,
            other => {
                return Err(format!(
                    "diagram_swimlane: steps[{}] must be Struct {{lane, label, order}}, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        let lane = struct_string_field("diagram_swimlane step", f, "lane")?;
        let label = struct_string_field("diagram_swimlane step", f, "label")?;
        let order = struct_float_field("diagram_swimlane step", f, "order")?;
        if !lane_set.contains(&lane) {
            return Err(format!(
                "diagram_swimlane: steps[{}].lane {:?} is not in lanes list",
                i, lane
            ));
        }
        steps.push(SwimlaneStep { lane, label, order });
    }
    // Compute order range for normalization. If all orders are equal
    // (degenerate case), place everything at the left padding.
    let (min_order, max_order) = steps
        .iter()
        .map(|s| s.order)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), o| {
            (mn.min(o), mx.max(o))
        });
    let order_range = (max_order - min_order).max(1e-9);

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let n_lanes = lanes.len();
    let lane_h = (canvas_h - 2.0 * SWIMLANE_PAD_Y) / (n_lanes as f64);
    let step_area_x = SWIMLANE_LABEL_W + SWIMLANE_PAD_X;
    let step_area_w = canvas_w - step_area_x - SWIMLANE_PAD_X;

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Lane bands with labels (left column).
    for (i, lane_name) in lanes.iter().enumerate() {
        let y = SWIMLANE_PAD_Y + (i as f64) * lane_h;
        // Alternating tint for readability.
        if i % 2 == 1 {
            parts.push(format!(
                r#"<rect x="0" y="{}" width="{}" height="{}" fill="{}" opacity="0.18" />"#,
                fmt_num(y),
                fmt_num(canvas_w),
                fmt_num(lane_h),
                escape_attr(&rule)
            ));
        }
        // Lane separator line (top).
        parts.push(format!(
            r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(y),
            fmt_num(canvas_w),
            fmt_num(y),
            escape_attr(&rule)
        ));
        // Left label column — accent strip + lane name.
        parts.push(format!(
            r#"<rect x="0" y="{}" width="3" height="{}" fill="{}" />"#,
            fmt_num(y),
            fmt_num(lane_h),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" font-weight="bold" fill="{}">{}</text>"#,
            fmt_num(12.0),
            fmt_num(y + lane_h / 2.0 + 4.0),
            escape_attr(&ink),
            escape_html_chars(lane_name)
        ));
        // Vertical separator between label column and step area.
        parts.push(format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
            fmt_num(SWIMLANE_LABEL_W),
            fmt_num(y),
            fmt_num(SWIMLANE_LABEL_W),
            fmt_num(y + lane_h),
            escape_attr(&rule)
        ));
    }
    // Bottom separator.
    let bottom_y = SWIMLANE_PAD_Y + (n_lanes as f64) * lane_h;
    parts.push(format!(
        r#"<line x1="0" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" />"#,
        fmt_num(bottom_y),
        fmt_num(canvas_w),
        fmt_num(bottom_y),
        escape_attr(&rule)
    ));
    // Compute step positions per lane, sorted by order — for connectors.
    let mut lane_to_steps: std::collections::HashMap<String, Vec<(f64, String)>> =
        std::collections::HashMap::new();
    for step in &steps {
        let norm_x = (step.order - min_order) / order_range;
        let x = step_area_x + norm_x * step_area_w;
        let lane_idx = lanes.iter().position(|l| l == &step.lane).ok_or_else(|| {
            format!(
                "diagram_swimlane: internal error — lane {:?} not found",
                step.lane
            )
        })?;
        let y_center = SWIMLANE_PAD_Y + (lane_idx as f64 + 0.5) * lane_h;
        // Step pill — paper fill, accent border, rounded.
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="6" ry="6" fill="{}" stroke="{}" stroke-width="1.2" />"#,
            fmt_num(x - 35.0),
            fmt_num(y_center - 12.0),
            fmt_num(70.0),
            fmt_num(24.0),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(x),
            fmt_num(y_center + 4.0),
            escape_attr(&ink),
            escape_html_chars(&step.label)
        ));
        lane_to_steps
            .entry(step.lane.clone())
            .or_default()
            .push((x, step.label.clone()));
    }
    // Optional: connect consecutive-by-order steps in the same lane with
    // a faint dashed arrow (visualizes temporal flow within a lane).
    for (_lane, mut lane_steps) in lane_to_steps {
        if lane_steps.len() < 2 {
            continue;
        }
        lane_steps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for window in lane_steps.windows(2) {
            let (x1, _) = window[0];
            let (x2, _) = window[1];
            // Faint dashed line — NOT a draw_connector (we want it lighter
            // than the cross-lane relationships).
            parts.push(format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.8" stroke-dasharray="3 2" opacity="0.5" />"#,
                fmt_num(x1 + 35.0),
                fmt_num(0.0), // y set per lane below
                fmt_num(x2 - 35.0),
                fmt_num(0.0),
                escape_attr(&muted)
            ));
        }
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 5/6 shared: extract_graph ─────────────────────────────────
//
// All three of diagram_data_flow / diagram_high_level / diagram_architecture
// share the same `Struct{nodes, edges}` shape. The only difference is
// whether the `icon` field is allowed on nodes (architecture only).
// We parse all three into a common (GraphNode, GraphEdge) representation
// and dispatch to the appropriate layer function at the call site.

struct GraphNode {
    id: String,
    label: String,
    icon: Option<String>,
}

struct GraphEdge {
    from: String,
    to: String,
    label: Option<String>,
}

/// Parse `Struct{nodes: [{id, label, icon?}], edges: [{from, to, label?}]}`
/// into (Vec<GraphNode>, Vec<GraphEdge>). `allow_icon` controls whether
/// the `icon` field is read on each node — diagram_architecture passes
/// true; data_flow and high_level pass false (the field is silently
/// ignored if present, matching the spec's "icon not used" wording).
fn extract_graph(
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
fn layout_layered_nodes(
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

// ── Block 5: diagram_data_flow ──────────────────────────────────────
//
// Same data shape as diagram_flowchart (Struct{nodes, edges}), but:
//   - Cycles are VALID (data flows have feedback loops). Uses
//     bfs_layers_with_cycles instead of topological_layers.
//   - Root for the BFS is the first node in `nodes` (data_flow has no
//     `initial` field, unlike state — pick the first listed node).
//   - Nodes are plain rectangles (no decision-shape semantics, unlike
//     flowchart's diamond conventions).
//
// Limits: same as flowchart (FLOWCHART_MAX_NODES = 25).

const DATAFLOW_NODE_W: f64 = 110.0;
const DATAFLOW_NODE_H: f64 = 44.0;

/// `diagram_data_flow(data, style) -> String`
///
/// `data` is `Struct { nodes: List<{id, label}>, edges: List<{from, to, label?}> }`.
/// Cycles in edges are VALID (data may circulate, feedback loops).
pub fn builtin_diagram_data_flow(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_graph(&data_value, "diagram_data_flow", false)?;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    // Pick the first node as BFS root — data_flow has no `initial` field.
    let root = node_ids
        .first()
        .cloned()
        .ok_or_else(|| "diagram_data_flow: nodes list is empty".to_string())?;
    let layers = bfs_layers_with_cycles(&node_ids, &edge_pairs, &root);

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let id_to_pos = layout_layered_nodes(&layers, canvas_w, canvas_h);
    let mut id_to_label: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for n in &nodes {
        id_to_label.insert(n.id.clone(), n.label.as_str());
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    // Edges first
    for edge in &edges {
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_data_flow: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_data_flow: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        let (sx, sy) = box_edge_point(from_x, from_y, to_x, to_y, DATAFLOW_NODE_W, DATAFLOW_NODE_H);
        let (ex, ey) = box_edge_point(to_x, to_y, from_x, from_y, DATAFLOW_NODE_W, DATAFLOW_NODE_H);
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    // Nodes
    for (id, (cx, cy)) in &id_to_pos {
        let label = id_to_label.get(id).copied().unwrap_or("");
        let box_x = cx - DATAFLOW_NODE_W / 2.0;
        let box_y = cy - DATAFLOW_NODE_H / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="1.5" rx="4" ry="4" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(DATAFLOW_NODE_W),
            fmt_num(DATAFLOW_NODE_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="12" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Block 6: diagram_high_level + diagram_architecture ──────────────
//
// Two SEPARATE public APIs (per the spec: "Реализовать как две отдельные
// функции, не одну с параметром «режим»"). They share the same data
// shape and the same internal extract_graph + topological_layers
// pipeline; they differ in node rendering:
//   - high_level: large labeled blocks (no icons), bolder visual
//   - architecture: same blocks + svg_icon when an `icon` field is
//     specified on a node
//
// Both REJECT cycles (architectural diagrams should be acyclic — a
// cycle here is treated as an input error, same as flowchart). If real-
// world usage shows legitimate bidirectional architecture (e.g. service
// pairs that call each other), we'd revisit this; for now, the spec
// says: "архитектурные схемы обычно ациклические, цикл здесь скорее
// ошибка входных данных, как в diagram_flowchart."

const HIGHLEVEL_NODE_W: f64 = 130.0;
const HIGHLEVEL_NODE_H: f64 = 56.0;
const ARCH_ICON_SIZE: f64 = 20.0;

/// `diagram_high_level(data, style) -> String`
///
/// `data` is `Struct { nodes: List<{id, label}>, edges: List<{from, to, label?}> }`.
/// Cycles → Err (architectural diagrams should be acyclic). Uses
/// topological_layers (same as diagram_flowchart) for layout.
pub fn builtin_diagram_high_level(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_graph(&data_value, "diagram_high_level", false)?;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    // High-level diagrams are acyclic — propagate the cycle error from
    // topological_layers (returns Err with "flowchart contains a cycle: ..."
    // — the message is generic enough; we don't override it).
    let layers = topological_layers(&node_ids, &edge_pairs)?;

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let id_to_pos = layout_layered_nodes(&layers, canvas_w, canvas_h);
    let mut id_to_label: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for n in &nodes {
        id_to_label.insert(n.id.clone(), n.label.as_str());
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    for edge in &edges {
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_high_level: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_high_level: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        let (sx, sy) = box_edge_point(
            from_x,
            from_y,
            to_x,
            to_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        let (ex, ey) = box_edge_point(
            to_x,
            to_y,
            from_x,
            from_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    for (id, (cx, cy)) in &id_to_pos {
        let label = id_to_label.get(id).copied().unwrap_or("");
        let box_x = cx - HIGHLEVEL_NODE_W / 2.0;
        let box_y = cy - HIGHLEVEL_NODE_H / 2.0;
        // Larger, bolder block than flowchart/data_flow — visually signals
        // "high-level architectural component".
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="2" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(HIGHLEVEL_NODE_W),
            fmt_num(HIGHLEVEL_NODE_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(*cx),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(label)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

/// `diagram_architecture(data, style) -> String`
///
/// `data` is `Struct { nodes: List<{id, label, icon?}>, edges: List<{from, to, label?}> }`.
/// Same as diagram_high_level, but each node MAY specify an `icon`
/// (validated against the 10 svg_icon names — same delegation pattern
/// as diagram_medallion). When icon is present, it renders inside the
/// node box to the left of the label; when absent, the node looks
/// identical to a high_level block.
///
/// Cycles → Err (architectural diagrams should be acyclic).
pub fn builtin_diagram_architecture(args: &[Value]) -> Result<Value, String> {
    let data_value = args.first().cloned().unwrap_or(Value::Unit);
    let style_value = args.get(1).cloned().unwrap_or(Value::Unit);
    let style = extract_style(&style_value)?;
    let (nodes, edges) = extract_graph(&data_value, "diagram_architecture", true)?;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let edge_pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    let layers = topological_layers(&node_ids, &edge_pairs)?;

    let ink = style_token(&style, "ink").unwrap_or_else(|_| "#2d3142".to_string());
    let paper = style_token(&style, "paper").unwrap_or_else(|_| "#ffffff".to_string());
    let accent = style_token(&style, "accent").unwrap_or_else(|_| "#eb6c36".to_string());
    let muted = style_token(&style, "muted").unwrap_or_else(|_| "#4f5d75".to_string());
    let rule = style_token(&style, "rule").unwrap_or_else(|_| "#cccccc".to_string());

    let canvas_w = DIAGRAM_CANVAS_W;
    let canvas_h = DIAGRAM_CANVAS_H;
    let id_to_pos = layout_layered_nodes(&layers, canvas_w, canvas_h);
    let mut id_to_node: std::collections::HashMap<String, &GraphNode> =
        std::collections::HashMap::new();
    for n in &nodes {
        id_to_node.insert(n.id.clone(), n);
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
        escape_attr(&paper)
    ));
    for edge in &edges {
        let (from_x, from_y) = id_to_pos.get(&edge.from).cloned().ok_or_else(|| {
            format!(
                "diagram_architecture: internal error — node {:?} not in position map",
                edge.from
            )
        })?;
        let (to_x, to_y) = id_to_pos.get(&edge.to).cloned().ok_or_else(|| {
            format!(
                "diagram_architecture: internal error — node {:?} not in position map",
                edge.to
            )
        })?;
        let (sx, sy) = box_edge_point(
            from_x,
            from_y,
            to_x,
            to_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        let (ex, ey) = box_edge_point(
            to_x,
            to_y,
            from_x,
            from_y,
            HIGHLEVEL_NODE_W,
            HIGHLEVEL_NODE_H,
        );
        parts.push(draw_connector(sx, sy, ex, ey, &style));
        if let Some(label) = &edge.label {
            let mid_x = (sx + ex) / 2.0;
            let mid_y = (sy + ey) / 2.0 - 8.0;
            parts.push(format!(
                r#"<text x="{}" y="{}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
                fmt_num(mid_x),
                fmt_num(mid_y),
                escape_attr(&muted),
                escape_html_chars(label)
            ));
        }
    }
    for (id, (cx, cy)) in &id_to_pos {
        let node = id_to_node.get(id).copied().ok_or_else(|| {
            format!(
                "diagram_architecture: internal error — node {:?} not in node map",
                id
            )
        })?;
        let box_x = cx - HIGHLEVEL_NODE_W / 2.0;
        let box_y = cy - HIGHLEVEL_NODE_H / 2.0;
        parts.push(format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="2" rx="6" ry="6" />"#,
            fmt_num(box_x),
            fmt_num(box_y),
            fmt_num(HIGHLEVEL_NODE_W),
            fmt_num(HIGHLEVEL_NODE_H),
            escape_attr(&paper),
            escape_attr(&accent)
        ));
        // Render icon if specified, then shift label right of the icon.
        let label_x = if let Some(ref icon_name) = node.icon {
            let icon_x = box_x + 8.0;
            let icon_y = cy - ARCH_ICON_SIZE / 2.0;
            // Delegate to icon_path_data with proper error propagation
            // (same pattern as builtin_svg_icon line 2879 + diagram_medallion).
            let path_data = icon_path_data(icon_name).ok_or_else(|| {
                format!(
                    "diagram_architecture: unknown icon name '{}'. Available: server, laptop, phone, database, cloud, arrow-right, check, warning, user, document",
                    icon_name
                )
            })?;
            let scale = ARCH_ICON_SIZE / 24.0;
            parts.push(format!(
                r#"<svg x="{}" y="{}" width="{}" height="{}" viewBox="0 0 24 24"><g transform="scale({})"><path d="{}" stroke="{}" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></g></svg>"#,
                fmt_num(icon_x),
                fmt_num(icon_y),
                fmt_num(ARCH_ICON_SIZE),
                fmt_num(ARCH_ICON_SIZE),
                fmt_num(scale),
                path_data,
                escape_attr(&ink)
            ));
            cx + ARCH_ICON_SIZE / 2.0
        } else {
            *cx
        };
        parts.push(format!(
            r#"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="{}" text-anchor="middle">{}</text>"#,
            fmt_num(label_x),
            fmt_num(cy + 4.0),
            escape_attr(&ink),
            escape_html_chars(&node.label)
        ));
    }
    parts.push(format!(
        r#"<rect x="0" y="0" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1" />"#,
        fmt_num(canvas_w),
        fmt_num(canvas_h),
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

// ── Internal: escape XML attribute values ────────────────────────────
//
// For attribute values (inside "..."), we must escape: & < > " '
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

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::shared::extract_style;
    use super::super::*; // svg module re-exports
    use crate::interpreter::Value;
    use std::collections::HashMap;

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }
    fn f(n: f64) -> Value {
        Value::Float(n)
    }

    #[test]
    fn svg_rect_basic() {
        let out = builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("#eb6c36"), s("none")])
            .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<rect"#));
                assert!(xml.contains(r#"x="10""#));
                assert!(xml.contains(r#"width="100""#));
                assert!(xml.contains(r#"height="50""#));
                assert!(xml.contains(r##"fill="#eb6c36""##));
                assert!(xml.contains(r#"stroke="none""#));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_rect_rejects_zero_dimensions() {
        let r = builtin_svg_rect(&[f(0.0), f(0.0), f(0.0), f(50.0), s("red"), s("none")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_text_escapes_script_tag() {
        let out = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("<script>alert(1)</script>"),
            f(14.0),
            s("#2d3142"),
            s("start"),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                // Critical security invariant: < and > MUST be escaped
                assert!(!xml.contains("<script>"));
                assert!(xml.contains("&lt;script&gt;"));
                assert!(xml.contains("&lt;/script&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_text_escapes_quotes_and_ampersand() {
        let out = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("test \"quoted\" & <tag>"),
            f(14.0),
            s("#000"),
            s("start"),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains("&amp;"));
                assert!(xml.contains("&lt;tag&gt;"));
                assert!(xml.contains("&quot;quoted&quot;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_text_rejects_invalid_anchor() {
        let r = builtin_svg_text(&[
            f(10.0),
            f(20.0),
            s("hello"),
            f(14.0),
            s("#000"),
            s("center"),
        ]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_canvas_returns_valid_xml_skeleton() {
        let child =
            builtin_svg_rect(&[f(10.0), f(10.0), f(100.0), f(50.0), s("red"), s("none")]).unwrap();
        let out = builtin_svg_canvas(&[
            f(200.0),
            f(100.0),
            s("0 0 200 100"),
            Value::List(vec![child]),
        ])
        .unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
                assert!(xml.contains(r#"width="200""#));
                assert!(xml.contains(r#"height="100""#));
                assert!(xml.contains(r#"viewBox="0 0 200 100""#));
                assert!(xml.contains("<rect"));
                assert!(xml.ends_with("</svg>"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_canvas_rejects_invalid_viewbox() {
        let r = builtin_svg_canvas(&[f(200.0), f(100.0), s("0 0 200"), Value::List(vec![])]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_path_rejects_angle_brackets() {
        let r = builtin_svg_path(&[s("M 10 10 <script>"), s("none"), s("black")]);
        assert!(r.is_err());
    }

    #[test]
    fn diagram_style_returns_struct_with_5_tokens() {
        let mut fields = HashMap::new();
        fields.insert("paper".to_string(), s("#f5f5f5"));
        fields.insert("ink".to_string(), s("#2d3142"));
        fields.insert("accent".to_string(), s("#eb6c36"));
        fields.insert("muted".to_string(), s("#4f5d75"));
        fields.insert("rule".to_string(), s("rgba(45,49,66,0.12)"));
        let style_arg = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields,
        };
        let out = builtin_diagram_style(&[style_arg]).unwrap();
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DiagramStyle");
                assert_eq!(fields.len(), 5);
                assert!(fields.contains_key("paper"));
                assert!(fields.contains_key("ink"));
                assert!(fields.contains_key("accent"));
                assert!(fields.contains_key("muted"));
                assert!(fields.contains_key("rule"));
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn diagram_style_rejects_missing_token() {
        let mut fields = HashMap::new();
        fields.insert("paper".to_string(), s("#f5f5f5"));
        fields.insert("ink".to_string(), s("#2d3142"));
        // missing accent, muted, rule
        let style_arg = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields,
        };
        let r = builtin_diagram_style(&[style_arg]);
        assert!(r.is_err());
    }

    #[test]
    fn chart_bar_basic_3_bars() {
        let mut fields1 = HashMap::new();
        fields1.insert("label".to_string(), s("Янв"));
        fields1.insert("value".to_string(), f(40.0));
        let item1 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields1,
        };
        let mut fields2 = HashMap::new();
        fields2.insert("label".to_string(), s("Фев"));
        fields2.insert("value".to_string(), f(65.0));
        let item2 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields2,
        };
        let mut fields3 = HashMap::new();
        fields3.insert("label".to_string(), s("Мар"));
        fields3.insert("value".to_string(), f(30.0));
        let item3 = Value::Struct {
            type_name: "Bar".to_string(),
            fields: fields3,
        };
        let data = Value::List(vec![item1, item2, item3]);

        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#f5f5f5"));
        style_fields.insert("ink".to_string(), s("#2d3142"));
        style_fields.insert("accent".to_string(), s("#eb6c36"));
        style_fields.insert("muted".to_string(), s("#4f5d75"));
        style_fields.insert("rule".to_string(), s("rgba(45,49,66,0.12)"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };

        let out = builtin_chart_bar(&[data, style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with(r#"<svg "#));
                assert!(xml.ends_with("</svg>"));
                // 3 bars (each contains <rect)
                let rect_count = xml.matches("<rect").count();
                assert!(rect_count >= 4); // 3 bars + 1 background = 4
                                          // Labels present and not escaped (Cyrillic is fine in XML UTF-8)
                assert!(xml.contains("Янв"));
                assert!(xml.contains("Фев"));
                assert!(xml.contains("Мар"));
                // The tallest bar (65) should be accent-colored
                assert!(xml.contains("fill=\"#eb6c36\""));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_bar_rejects_empty_data() {
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_bar(&[Value::List(vec![]), style]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_sketchy_filter_default_params() {
        let out = builtin_svg_sketchy_filter(&[s("sketch1")]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<filter id="sketch1">"#));
                assert!(xml.contains("feTurbulence"));
                assert!(xml.contains("feDisplacementMap"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_sketchy_filter_rejects_bad_id() {
        let r = builtin_svg_sketchy_filter(&[s("id with spaces")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_icon_known_name() {
        let out =
            builtin_svg_icon(&[s("server"), f(10.0), f(10.0), f(24.0), s("currentColor")]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.contains(r#"<svg "#));
                assert!(xml.contains(r#"x="10""#));
                assert!(xml.contains(r#"y="10""#));
                assert!(xml.contains(r#"width="24""#));
                assert!(xml.contains(r#"height="24""#));
                assert!(xml.contains(r#"stroke="currentColor""#));
                assert!(xml.contains("<path"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_icon_unknown_name_errors() {
        let r = builtin_svg_icon(&[s("nonexistent"), f(0.0), f(0.0), f(24.0), s("black")]);
        assert!(r.is_err());
    }

    #[test]
    fn svg_callout_default_intent() {
        let out = builtin_svg_callout(&[s("note"), f(10.0), f(10.0), f(100.0), f(50.0)]).unwrap();
        match out {
            Value::String(xml) => {
                // Dashed line (callout invariant)
                assert!(xml.contains(r#"stroke-dasharray="3,3""#));
                // Italic text
                assert!(xml.contains(r#"font-style="italic""#));
                // Anchor dot
                assert!(xml.contains("<circle"));
                // Text content
                assert!(xml.contains("note"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn svg_callout_escapes_text() {
        let out =
            builtin_svg_callout(&[s("<b>bold</b>"), f(10.0), f(10.0), f(100.0), f(50.0)]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(!xml.contains("<b>bold</b>"));
                assert!(xml.contains("&lt;b&gt;bold&lt;/b&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    // ── Наряд №77: color_palette + chart_donut unit tests ──

    #[test]
    fn color_palette_returns_diagram_style_struct_with_5_tokens() {
        let out = builtin_color_palette(&[s("energy"), s("light")]).unwrap();
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "DiagramStyle");
                assert_eq!(fields.len(), 5);
                for k in &["paper", "ink", "accent", "muted", "rule"] {
                    assert!(fields.contains_key(*k), "missing token {}", k);
                }
                // Each token must be a hex string of form #rrggbb
                for k in &["paper", "ink", "accent", "muted", "rule"] {
                    if let Some(Value::String(v)) = fields.get(*k) {
                        assert!(v.starts_with('#'), "{} should start with #", k);
                        assert_eq!(v.len(), 7, "{} should be #rrggbb (7 chars)", k);
                    }
                }
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn color_palette_rejects_unknown_intent() {
        let r = builtin_color_palette(&[s("unknown"), s("light")]);
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(err.contains("intent"), "err: {}", err);
    }

    #[test]
    fn color_palette_rejects_unknown_mode() {
        let r = builtin_color_palette(&[s("calm"), s("neon")]);
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert!(err.contains("mode"), "err: {}", err);
    }

    #[test]
    fn color_palette_light_vs_dark_produce_different_tokens() {
        let light = builtin_color_palette(&[s("authority"), s("light")]).unwrap();
        let dark = builtin_color_palette(&[s("authority"), s("dark")]).unwrap();
        if let (Value::Struct { fields: lf, .. }, Value::Struct { fields: df, .. }) = (light, dark)
        {
            // Light paper should be much lighter than dark paper.
            // Value doesn't impl PartialEq, so extract strings and compare those.
            let lp = match lf.get("paper").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("light paper not String"),
            };
            let dp = match df.get("paper").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("dark paper not String"),
            };
            assert_ne!(lp, dp, "light vs dark paper must differ");
            let li = match lf.get("ink").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("light ink not String"),
            };
            let di = match df.get("ink").unwrap() {
                Value::String(v) => v.clone(),
                _ => panic!("dark ink not String"),
            };
            assert_ne!(li, di, "light vs dark ink must differ");
        }
    }

    #[test]
    fn color_palette_all_6_intents_all_2_modes_produce_valid_hex() {
        // Наряд №162: mono added to the intent set
        for intent in &["calm", "tension", "energy", "authority", "warmth", "mono"] {
            for mode in &["light", "dark"] {
                let out = builtin_color_palette(&[s(intent), s(mode)]).unwrap();
                if let Value::Struct { fields, .. } = out {
                    for k in &["paper", "ink", "accent", "muted", "rule"] {
                        if let Some(Value::String(v)) = fields.get(*k) {
                            assert!(
                                v.starts_with('#') && v.len() == 7,
                                "intent={} mode={} token={} got {:?}",
                                intent,
                                mode,
                                k,
                                v
                            );
                            // Hex digits only after #
                            let hex = &v[1..];
                            assert!(
                                hex.chars().all(|c| c.is_ascii_hexdigit()),
                                "non-hex char in {} for intent={} mode={}",
                                k,
                                intent,
                                mode
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn color_palette_mono_exact_hex_tokens() {
        // Наряд №162: hand-picked values must not drift (not HSL-derived).
        let light = builtin_color_palette(&[s("mono"), s("light")]).unwrap();
        if let Value::Struct { fields, .. } = light {
            let get = |k: &str| match fields.get(k) {
                Some(Value::String(v)) => v.clone(),
                _ => panic!("missing {}", k),
            };
            assert_eq!(get("paper"), "#F0EFEB");
            assert_eq!(get("ink"), "#1C1C1A");
            assert_eq!(get("muted"), "#8F8E88");
            assert_eq!(get("rule"), "#DEDDD6");
            // accent = ink (no chromatic accent in mono aesthetic)
            assert_eq!(get("accent"), "#1C1C1A");
        } else {
            panic!("expected Struct");
        }

        let dark = builtin_color_palette(&[s("mono"), s("dark")]).unwrap();
        if let Value::Struct { fields, .. } = dark {
            let get = |k: &str| match fields.get(k) {
                Some(Value::String(v)) => v.clone(),
                _ => panic!("missing {}", k),
            };
            assert_eq!(get("paper"), "#1C1C1A");
            assert_eq!(get("ink"), "#F0EFEB");
            assert_eq!(get("muted"), "#8F8E88");
            assert_eq!(get("rule"), "#2E2D29");
            assert_eq!(get("accent"), "#F0EFEB");
        } else {
            panic!("expected Struct");
        }
    }

    #[test]
    fn color_palette_mono_rejects_bad_mode() {
        let r = builtin_color_palette(&[s("mono"), s("neon")]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("mode"));
    }

    #[test]
    fn color_palette_result_passes_extract_style() {
        // Critical: color_palette output must be consumable by extract_style
        // (the helper used by chart_bar / chart_donut).
        let out = builtin_color_palette(&[s("warmth"), s("light")]).unwrap();
        let extracted = extract_style(&out);
        assert!(extracted.is_ok(), "extract_style failed: {:?}", extracted);
        let style = extracted.unwrap();
        assert_eq!(style.len(), 5);
        for k in &["paper", "ink", "accent", "muted", "rule"] {
            assert!(style.contains_key(*k));
        }
    }

    #[test]
    fn color_palette_result_works_with_chart_bar() {
        // End-to-end: color_palette → chart_bar (no manual diagram_style needed)
        let palette = builtin_color_palette(&[s("energy"), s("dark")]).unwrap();
        let mut item_fields = HashMap::new();
        item_fields.insert("label".to_string(), s("Q1"));
        item_fields.insert("value".to_string(), f(40.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: item_fields,
        };
        let data = Value::List(vec![item]);
        let out = builtin_chart_bar(&[data, palette]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_basic_3_slices() {
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("Alpha"));
        f1.insert("value".to_string(), f(40.0));
        let item1 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut f2 = HashMap::new();
        f2.insert("label".to_string(), s("Beta"));
        f2.insert("value".to_string(), f(35.0));
        let item2 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f2,
        };
        let mut f3 = HashMap::new();
        f3.insert("label".to_string(), s("Gamma"));
        f3.insert("value".to_string(), f(25.0));
        let item3 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f3,
        };
        let data = Value::List(vec![item1, item2, item3]);

        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#eb6c36"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };

        let out = builtin_chart_donut(&[data, style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
                // 3 slices = 3 <path> elements (each donut slice is one path)
                let path_count = xml.matches("<path").count();
                assert_eq!(path_count, 3, "expected 3 slice paths");
                // Background rect
                assert!(xml.contains("<rect"));
                // Labels present (escaped if needed — Alpha/Beta/Gamma are safe)
                assert!(xml.contains("Alpha"));
                assert!(xml.contains("Beta"));
                assert!(xml.contains("Gamma"));
                // Center total: 40+35+25=100
                assert!(xml.contains(">100<"));
                // Legend swatches: 3 (one per slice)
                let rect_count = xml.matches("<rect").count();
                assert!(
                    rect_count >= 4,
                    "expected 4+ rects (1 bg + 3 legend swatches)"
                );
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_rejects_empty_data() {
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_donut(&[Value::List(vec![]), style]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("empty"));
    }

    #[test]
    fn chart_donut_rejects_negative_value() {
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("A"));
        f1.insert("value".to_string(), f(-10.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let r = builtin_chart_donut(&[Value::List(vec![item]), style]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("non-negative"));
    }

    #[test]
    fn chart_donut_escapes_label_with_script_tag() {
        // Critical security invariant: <script> in label must NOT leak raw
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("<script>alert(1)</script>"));
        f1.insert("value".to_string(), f(40.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#f00"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item]), style]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(
                    !xml.contains("<script>"),
                    "RAW <script> leaked into chart_donut output: {}",
                    xml
                );
                assert!(xml.contains("&lt;script&gt;"));
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_single_slice_uses_accent() {
        // One slice = whole pie = accent color
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("Only"));
        f1.insert("value".to_string(), f(100.0));
        let item = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut style_fields = HashMap::new();
        style_fields.insert("paper".to_string(), s("#fff"));
        style_fields.insert("ink".to_string(), s("#000"));
        style_fields.insert("accent".to_string(), s("#ff8800"));
        style_fields.insert("muted".to_string(), s("#888"));
        style_fields.insert("rule".to_string(), s("#ccc"));
        let style = Value::Struct {
            type_name: "DiagramStyle".to_string(),
            fields: style_fields,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item]), style]).unwrap();
        match out {
            Value::String(xml) => {
                // The single slice should be filled with accent color
                assert!(
                    xml.contains(r##"fill="#ff8800""##),
                    "single slice should be accent-colored, xml: {}",
                    xml
                );
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn chart_donut_works_with_color_palette_output() {
        // End-to-end: color_palette → chart_donut
        let palette = builtin_color_palette(&[s("calm"), s("light")]).unwrap();
        let mut f1 = HashMap::new();
        f1.insert("label".to_string(), s("A"));
        f1.insert("value".to_string(), f(60.0));
        let item1 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f1,
        };
        let mut f2 = HashMap::new();
        f2.insert("label".to_string(), s("B"));
        f2.insert("value".to_string(), f(40.0));
        let item2 = Value::Struct {
            type_name: "Slice".to_string(),
            fields: f2,
        };
        let out = builtin_chart_donut(&[Value::List(vec![item1, item2]), palette]).unwrap();
        match out {
            Value::String(xml) => {
                assert!(xml.starts_with("<svg "));
                assert!(xml.ends_with("</svg>"));
                assert_eq!(xml.matches("<path").count(), 2, "expected 2 slice paths");
            }
            _ => panic!("expected String"),
        }
    }
}
