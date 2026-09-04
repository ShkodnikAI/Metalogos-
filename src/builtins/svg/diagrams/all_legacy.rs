//! Diagram builtins (diagram_*)
//! Category: "diagram" (from registry.rs)
//! diagram_style is in primitives (tokens category)

use super::super::primitives::icon_path_data;
use super::super::shared::*;
use super::flow_seq::{bfs_layers_with_cycles, topological_layers, FLOWCHART_MAX_NODES};
use super::layout::box_edge_point;
use super::tree_org::{DIAGRAM_CANVAS_H, DIAGRAM_CANVAS_W};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;

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
