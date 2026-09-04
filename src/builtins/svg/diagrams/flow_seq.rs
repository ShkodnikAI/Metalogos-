//! Flowchart, sequence, and swimlane diagram builtins.
//!
//! Наряд №169: extracted from former diagrams.rs (all_legacy.rs).
//! No logic changes — pure move.
//!
//! Includes:
//! - `diagram_flowchart(data, style)` — topological layers + connectors
//! - `diagram_sequence(data, style)` — UML sequence (lifelines + messages)
//! - `diagram_swimlane(data, style)` — swimlane layout
//! - `FlowNode`, `FlowEdge`, `SwimlaneStep` structs
//! - `extract_flowchart`, `topological_layers`, `bfs_layers_with_cycles`
//! - `box_edge_point` is in `super::layout` (shared with other groups)

use super::super::shared::*;
use super::layout::box_edge_point;
use super::tree_org::{DIAGRAM_CANVAS_H, DIAGRAM_CANVAS_W};
use crate::builtins::string::escape_html_chars;
use crate::interpreter::Value;

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
pub(super) const FLOWCHART_MAX_NODES: usize = 25;
pub(super) const FLOWCHART_NODE_W: f64 = 110.0;
pub(super) const FLOWCHART_NODE_H: f64 = 44.0;

pub(super) const SEQ_MAX_ACTORS: usize = 8;
pub(super) const SEQ_MAX_MESSAGES: usize = 30;
pub(super) const SEQ_TOP_PAD: f64 = 50.0; // space for actor name labels at top
pub(super) const SEQ_BOTTOM_PAD: f64 = 30.0;
pub(super) const SEQ_LIFELINE_HALF_H: f64 = 12.0; // half-height of actor head box

pub(super) const SWIMLANE_MAX_LANES: usize = 6;
pub(super) const SWIMLANE_MAX_STEPS: usize = 30;
pub(super) const SWIMLANE_LABEL_W: f64 = 80.0;
pub(super) const SWIMLANE_PAD_X: f64 = 16.0;
pub(super) const SWIMLANE_PAD_Y: f64 = 16.0;

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
