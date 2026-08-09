// ── DAG / graph builtins ─────────────────────────────────────────────
// dag_phases, topo_sort

use super::super::core::*;
use super::super::json::*;
use crate::interpreter::Value;

/// `dag_phases(dag)` — extract parallel execution phases from a DAG.
///
/// The DAG is a list of nodes, each a struct with:
///   - "id": String (node identifier)
///   - "depends_on": List of String (node IDs this node depends on)
///
/// Returns a list of phases (lists of node IDs), where each phase contains
/// nodes that can be executed in parallel (all dependencies satisfied).
pub(crate) fn builtin_dag_phases(args: &[Value]) -> Result<Value, String> {
    let nodes = expect_list_arg("dag_phases", args, 0)?;
    if nodes.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Extract node IDs and build adjacency info
    let mut node_ids: Vec<String> = Vec::new();
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &nodes {
        let node_json = mlog_value_to_json(node);
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Err("dag_phases: each node must have an 'id' field (String)".into());
        }

        let deps: Vec<String> = node_json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        in_degree.insert(id.clone(), deps.len());
        deps_map.insert(id.clone(), deps);
        node_ids.push(id);
    }

    // Validate: all dependency references exist
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for (node, deps) in &deps_map {
        for dep in deps {
            if !node_set.contains(dep.as_str()) {
                return Err(format!(
                    "dag_phases: node '{}' depends on unknown node '{}'",
                    node, dep
                ));
            }
        }
    }

    // Kahn's algorithm — extract phases
    let mut remaining_in: std::collections::HashMap<String, usize> = in_degree.clone();
    let mut phases: Vec<Value> = Vec::new();
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        // Find all nodes with in-degree 0 (not yet processed)
        let phase_nodes: Vec<String> = node_ids
            .iter()
            .filter(|id| {
                !processed.contains(*id) && remaining_in.get(*id).copied().unwrap_or(0) == 0
            })
            .cloned()
            .collect();

        if phase_nodes.is_empty() {
            break;
        }

        // Add phase as a list of node IDs
        let phase_value = Value::List(
            phase_nodes
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect(),
        );
        phases.push(phase_value);

        // "Remove" phase nodes: decrease in-degree of dependents
        for id in &phase_nodes {
            processed.insert(id.clone());
            for (node, deps) in &deps_map {
                if deps.contains(id) {
                    if let Some(deg) = remaining_in.get_mut(node) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }
    }

    // Cycle detection
    if processed.len() != node_ids.len() {
        let unprocessed: Vec<&str> = node_ids
            .iter()
            .filter(|id| !processed.contains(*id))
            .map(|s| s.as_str())
            .collect();
        return Err(format!(
            "dag_phases: cycle detected among nodes: {}",
            unprocessed.join(", ")
        ));
    }

    Ok(Value::List(phases))
}

/// `topo_sort(dag)` — topological sort of a DAG.
///
/// Same input format as dag_phases. Returns a flat list of node IDs
/// in topological order (Kahn's algorithm).
pub(crate) fn builtin_topo_sort(args: &[Value]) -> Result<Value, String> {
    let nodes = expect_list_arg("topo_sort", args, 0)?;
    if nodes.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let mut node_ids: Vec<String> = Vec::new();
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &nodes {
        let node_json = mlog_value_to_json(node);
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Err("topo_sort: each node must have an 'id' field (String)".into());
        }

        let deps: Vec<String> = node_json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        in_degree.insert(id.clone(), deps.len());
        deps_map.insert(id.clone(), deps);
        node_ids.push(id);
    }

    // Validate dependency references
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for (node, deps) in &deps_map {
        for dep in deps {
            if !node_set.contains(dep.as_str()) {
                return Err(format!(
                    "topo_sort: node '{}' depends on unknown node '{}'",
                    node, dep
                ));
            }
        }
    }

    // Kahn's algorithm
    let mut remaining_in = in_degree.clone();
    let mut queue: std::collections::VecDeque<String> = node_ids
        .iter()
        .filter(|id| remaining_in.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut result: Vec<String> = Vec::new();

    while let Some(id) = queue.pop_front() {
        result.push(id.clone());
        for (node, deps) in &deps_map {
            if deps.contains(&id) {
                if let Some(deg) = remaining_in.get_mut(node) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(node.clone());
                    }
                }
            }
        }
    }

    // Cycle detection
    if result.len() != node_ids.len() {
        return Err("topo_sort: cycle detected in DAG".into());
    }

    Ok(Value::List(result.into_iter().map(Value::String).collect()))
}
