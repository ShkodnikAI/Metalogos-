// ── Memory Graph v2 — petgraph-backed knowledge graph for Metalogos ──
// Replaces flat L0/L1/L2 JSON array with a proper directed graph.
// Borrowed: petgraph (MIT) from crates.io for all graph algorithms.

use petgraph::algo::dijkstra;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Relation types between memory nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Relation {
    /// A summary was derived from source entries.
    DerivedFrom,
    /// Two entries contradict each other.
    Contradicts,
    /// Entries are topically related.
    RelatedTo,
    /// Newer entry supersedes older one.
    Supersedes,
    /// Entry references another (e.g., response refers to prompt).
    RefersTo,
}

impl Relation {
    fn as_str(&self) -> &'static str {
        match self {
            Relation::DerivedFrom => "derived_from",
            Relation::Contradicts => "contradicts",
            Relation::RelatedTo => "related_to",
            Relation::Supersedes => "supersedes",
            Relation::RefersTo => "refers_to",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "derived_from" => Some(Relation::DerivedFrom),
            "contradicts" => Some(Relation::Contradicts),
            "related_to" => Some(Relation::RelatedTo),
            "supersedes" => Some(Relation::Supersedes),
            "refers_to" => Some(Relation::RefersTo),
            _ => None,
        }
    }
}

/// A node in the memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub text: String,
    pub level: String, // "L0" (raw), "L1" (chunk summary), "L2" (global summary)
    pub score: f64,
    pub created_at: i64,
    pub source: String,
    pub tags: Vec<String>,
    /// Unix timestamp of last access (read/query/touch). 0 = never accessed.
    #[serde(default)]
    pub last_accessed: i64,
    /// How many times this node has been accessed.
    #[serde(default)]
    pub access_count: u32,
}

/// An edge in the memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub relation: Relation,
    pub weight: f64, // 0.0..1.0
}

/// The memory graph: a directed graph of MemoryNode with weighted typed edges.
pub struct MemoryGraph {
    graph: StableDiGraph<MemoryNode, MemoryEdge>,
    /// Fast lookup from node id string to petgraph NodeIndex.
    id_index: HashMap<String, NodeIndex>,
}

/// Serializable snapshot of the graph for KV storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<MemoryNode>,
    pub edges: Vec<GraphEdgeRecord>,
}

/// Serializable edge record (stores source/target node ids, not indices).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeRecord {
    pub source_id: String,
    pub target_id: String,
    pub relation: String,
    pub weight: f64,
}

/// Result of a belief revision operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviseResult {
    /// What happened: "updated", "superseded_rival", or "demoted"
    pub action: String,
    /// ID of the node that was superseded (if any).
    pub superseded_id: Option<String>,
    /// ID of the winning node.
    pub winner_id: String,
}

impl Default for MemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryGraph {
    pub fn new() -> Self {
        MemoryGraph {
            graph: StableDiGraph::new(),
            id_index: HashMap::new(),
        }
    }

    /// Add a node. Returns the node id.
    /// If a node with the same id already exists, returns existing id without duplicating.
    pub fn add_node(&mut self, node: MemoryNode) -> String {
        if let Some(&idx) = self.id_index.get(&node.id) {
            return self.graph[idx].id.clone();
        }
        let idx = self.graph.add_node(node);
        let id = self.graph[idx].id.clone();
        self.id_index.insert(id.clone(), idx);
        id
    }

    /// Add a directed edge between two nodes by id.
    /// Returns Ok(()) or Err if source/target not found.
    pub fn add_edge(
        &mut self,
        source_id: &str,
        target_id: &str,
        relation: Relation,
        weight: f64,
    ) -> Result<(), String> {
        let src = self
            .id_index
            .get(source_id)
            .ok_or_else(|| format!("node not found: {}", source_id))?;
        let tgt = self
            .id_index
            .get(target_id)
            .ok_or_else(|| format!("node not found: {}", target_id))?;
        self.graph.add_edge(
            *src,
            *tgt,
            MemoryEdge {
                relation,
                weight: weight.clamp(0.0, 1.0),
            },
        );
        Ok(())
    }

    /// Get a node by id. Returns None if not found.
    pub fn get_node(&self, id: &str) -> Option<&MemoryNode> {
        self.id_index.get(id).map(|&idx| &self.graph[idx])
    }

    /// Get all nodes.
    pub fn nodes(&self) -> Vec<&MemoryNode> {
        self.graph.node_indices().map(|i| &self.graph[i]).collect()
    }

    /// Get all edges as (source_node, target_node, edge) triples.
    pub fn edges(&self) -> Vec<(&MemoryNode, &MemoryNode, &MemoryEdge)> {
        self.graph
            .edge_indices()
            .filter_map(|ei| {
                let (src, tgt) = self.graph.edge_endpoints(ei)?;
                Some((&self.graph[src], &self.graph[tgt], &self.graph[ei]))
            })
            .collect()
    }

    /// Get neighbors of a node within a given depth (BFS, bidirectional).
    /// Returns (node, distance) tuples.
    pub fn neighbors(&self, id: &str, depth: usize) -> Vec<(&MemoryNode, usize)> {
        use std::collections::VecDeque;
        let start = match self.id_index.get(id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };
        let mut result = Vec::new();
        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(start, 0usize);
        queue.push_back((start, 0usize));
        while let Some((node, d)) = queue.pop_front() {
            if d > 0 && d <= depth {
                result.push((&self.graph[node], d));
            }
            if d >= depth {
                continue;
            }
            for neighbor in self.graph.neighbors(node) {
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor) {
                    e.insert(d + 1);
                    queue.push_back((neighbor, d + 1));
                }
            }
            for neighbor in self
                .graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
            {
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor) {
                    e.insert(d + 1);
                    queue.push_back((neighbor, d + 1));
                }
            }
        }
        result
    }

    /// Find shortest path between two nodes (unweighted).
    /// Returns list of node ids on the path, or None if no path exists.
    pub fn shortest_path(&self, from_id: &str, to_id: &str) -> Option<Vec<String>> {
        let from = *self.id_index.get(from_id)?;
        let to = *self.id_index.get(to_id)?;
        let scores = dijkstra(&self.graph, from, Some(to), |_| 1.0f64);
        // Reconstruct path
        if !scores.contains_key(&to) {
            return None;
        }
        // Simple BFS reconstruction from scores
        let mut path = Vec::new();
        let mut current = to;
        path.push(self.graph[current].id.clone());
        while current != from {
            let current_score = scores[&current];
            let mut found = false;
            for edge in self
                .graph
                .edges_directed(current, petgraph::Direction::Incoming)
            {
                let parent = edge.source();
                if let Some(&ps) = scores.get(&parent) {
                    if (ps + 1.0 - current_score).abs() < 0.001 {
                        current = parent;
                        path.push(self.graph[current].id.clone());
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                return None;
            }
        }
        path.reverse();
        Some(path)
    }

    /// Remove a node and all its edges.
    pub fn remove_node(&mut self, id: &str) -> bool {
        if let Some(idx) = self.id_index.remove(id) {
            self.graph.remove_node(idx);
            true
        } else {
            false
        }
    }

    /// Count nodes at each level.
    pub fn count_by_level(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for node in self.graph.node_indices() {
            let level = &self.graph[node].level;
            *counts.entry(level.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Total character count across all nodes.
    pub fn total_chars(&self) -> usize {
        self.graph
            .node_indices()
            .map(|i| self.graph[i].text.len())
            .sum()
    }

    /// Number of nodes, edges, connected components.
    pub fn stats(&self) -> (usize, usize, usize) {
        let components = self.weakly_connected_components();
        (self.graph.node_count(), self.graph.edge_count(), components)
    }

    fn weakly_connected_components(&self) -> usize {
        use petgraph::visit::NodeIndexable;
        if self.graph.node_count() == 0 {
            return 0;
        }
        let mut visited = vec![false; self.graph.node_bound()];
        let mut components = 0usize;
        for start in self.graph.node_indices() {
            if visited[start.index()] {
                continue;
            }
            components += 1;
            let mut stack = vec![start];
            while let Some(node) = stack.pop() {
                if visited[node.index()] {
                    continue;
                }
                visited[node.index()] = true;
                for neighbor in self.graph.neighbors(node) {
                    if !visited[neighbor.index()] {
                        stack.push(neighbor);
                    }
                }
                for neighbor in self
                    .graph
                    .neighbors_directed(node, petgraph::Direction::Incoming)
                {
                    if !visited[neighbor.index()] {
                        stack.push(neighbor);
                    }
                }
            }
        }
        components
    }

    /// Apply exponential decay to all node scores based on time since last access.
    /// `lambda` controls decay rate (e.g. 0.01 = gentle, 0.1 = aggressive).
    /// `now` is the current Unix timestamp.
    /// Formula: score *= e^(-lambda * delta_hours)
    /// Nodes with last_accessed == 0 use created_at instead.
    /// Returns the number of nodes whose score changed.
    pub fn decay(&mut self, lambda: f64, now: i64) -> usize {
        let mut changed = 0usize;
        let indices: Vec<_> = self.graph.node_indices().collect();
        for idx in indices {
            let node = &mut self.graph[idx];
            let reference = if node.last_accessed > 0 {
                node.last_accessed
            } else {
                node.created_at
            };
            let delta_secs = (now - reference).max(0) as f64;
            let delta_hours = delta_secs / 3600.0;
            if delta_hours > 0.0 {
                let factor = (-lambda * delta_hours).exp();
                if factor < 0.999 {
                    node.score *= factor;
                    node.score = node.score.max(0.0);
                    changed += 1;
                }
            }
        }
        changed
    }

    /// Boost a node's score on access. Increases score by `boost_amount`
    /// (capped at 1.0) and updates last_accessed timestamp and access_count.
    /// Returns true if the node was found and boosted.
    pub fn boost(&mut self, id: &str, boost_amount: f64, now: i64) -> bool {
        if let Some(&idx) = self.id_index.get(id) {
            let node = &mut self.graph[idx];
            node.score = (node.score + boost_amount).min(1.0);
            node.last_accessed = now;
            node.access_count = node.access_count.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Touch a node — update last_accessed and access_count without changing score.
    /// Called when a node is read/queried but not explicitly boosted.
    /// Returns true if found.
    pub fn touch(&mut self, id: &str, now: i64) -> bool {
        if let Some(&idx) = self.id_index.get(id) {
            let node = &mut self.graph[idx];
            node.last_accessed = now;
            node.access_count = node.access_count.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Prune nodes whose score is below `threshold`.
    /// Also removes isolated nodes (no edges, low score, older than `min_age_hours`).
    /// Returns the number of nodes removed.
    pub fn prune(&mut self, threshold: f64, min_age_hours: f64, now: i64) -> usize {
        let min_age_secs = (min_age_hours * 3600.0) as i64;
        // Collect ids to remove (can't modify while iterating)
        let to_remove: Vec<String> = self
            .graph
            .node_indices()
            .filter(|&idx| {
                let node = &self.graph[idx];
                if node.score < threshold {
                    // Only prune if old enough (don't kill fresh low-score nodes)
                    let age = now - node.created_at;
                    if age > min_age_secs {
                        return true;
                    }
                }
                false
            })
            .map(|idx| self.graph[idx].id.clone())
            .collect();

        let count = to_remove.len();
        for id in &to_remove {
            self.remove_node(id);
        }
        count
    }

    /// Belief revision: update a node's text/score, then resolve Contradicts edges.
    /// If the updated node has Contradicts edges, the system keeps the belief with
    /// higher score (or newer timestamp if scores are equal) and demotes the loser.
    /// Returns a struct describing what happened: { action, superseded_id?, winner_id }.
    pub fn revise(
        &mut self,
        id: &str,
        new_text: &str,
        new_score: f64,
        now: i64,
    ) -> Option<ReviseResult> {
        let idx = match self.id_index.get(id) {
            Some(&i) => i,
            None => return None,
        };

        // Update the node
        let node = &mut self.graph[idx];
        node.text = new_text.to_string();
        node.score = new_score.min(1.0);
        node.last_accessed = now;
        node.access_count = node.access_count.saturating_add(1);

        // Find Contradicts edges FROM and TO this node
        let contradicted_ids: Vec<String> = self
            .graph
            .edges(idx)
            .filter(|e| matches!(e.weight().relation, Relation::Contradicts))
            .map(|e| self.graph[e.target()].id.clone())
            .collect();
        let contradicted_by: Vec<String> = self
            .graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .filter(|e| matches!(e.weight().relation, Relation::Contradicts))
            .map(|e| self.graph[e.source()].id.clone())
            .collect();

        let all_contradictions: Vec<String> = contradicted_ids
            .into_iter()
            .chain(contradicted_by)
            .filter(|cid| cid != id)
            .collect();

        if all_contradictions.is_empty() {
            return Some(ReviseResult {
                action: "updated".to_string(),
                superseded_id: None,
                winner_id: id.to_string(),
            });
        }

        // Resolve each contradiction
        let mut superseded = None;
        for cid in &all_contradictions {
            if let Some(&other_idx) = self.id_index.get(cid) {
                let other_score = self.graph[other_idx].score;
                let other_time = self.graph[other_idx]
                    .last_accessed
                    .max(self.graph[other_idx].created_at);

                if new_score > other_score || (new_score == other_score && now > other_time) {
                    // We win — demote the other
                    self.graph[other_idx].score *= 0.3;
                    // Add Supersedes edge from us to them
                    let _ = self.add_edge(id, cid, Relation::Supersedes, 0.9);
                    superseded = Some(cid.clone());
                } else {
                    // They win — demote us
                    self.graph[idx].score *= 0.3;
                    let _ = self.add_edge(cid, id, Relation::Supersedes, 0.9);
                    return Some(ReviseResult {
                        action: "demoted".to_string(),
                        superseded_id: Some(id.to_string()),
                        winner_id: cid.clone(),
                    });
                }
            }
        }

        Some(ReviseResult {
            action: if superseded.is_some() {
                "superseded_rival".to_string()
            } else {
                "updated".to_string()
            },
            superseded_id: superseded,
            winner_id: id.to_string(),
        })
    }

    /// Extract a subgraph containing the given node ids and all edges between them.
    /// Returns a GraphSnapshot (serializable).
    pub fn subgraph(&self, ids: &[String]) -> GraphSnapshot {
        let id_set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        let nodes: Vec<MemoryNode> = self
            .graph
            .node_indices()
            .filter(|i| id_set.contains(self.graph[*i].id.as_str()))
            .map(|i| self.graph[i].clone())
            .collect();

        let edge_ids: std::collections::HashSet<String> =
            id_set.iter().map(|s| s.to_string()).collect();
        let edges: Vec<GraphEdgeRecord> = self
            .graph
            .edge_indices()
            .filter_map(|ei| {
                let (src, tgt) = self.graph.edge_endpoints(ei)?;
                let src_id = &self.graph[src].id;
                let tgt_id = &self.graph[tgt].id;
                if edge_ids.contains(src_id) && edge_ids.contains(tgt_id) {
                    let edge = &self.graph[ei];
                    Some(GraphEdgeRecord {
                        source_id: src_id.clone(),
                        target_id: tgt_id.clone(),
                        relation: edge.relation.as_str().to_string(),
                        weight: edge.weight,
                    })
                } else {
                    None
                }
            })
            .collect();

        GraphSnapshot { nodes, edges }
    }

    /// Serialize to JSON for KV storage.
    pub fn to_json(&self) -> String {
        let snapshot = GraphSnapshot {
            nodes: self
                .graph
                .node_indices()
                .map(|i| self.graph[i].clone())
                .collect(),
            edges: self
                .graph
                .edge_indices()
                .filter_map(|ei| {
                    let (src, tgt) = self.graph.edge_endpoints(ei)?;
                    let edge = &self.graph[ei];
                    Some(GraphEdgeRecord {
                        source_id: self.graph[src].id.clone(),
                        target_id: self.graph[tgt].id.clone(),
                        relation: edge.relation.as_str().to_string(),
                        weight: edge.weight,
                    })
                })
                .collect(),
        };
        serde_json::to_string(&snapshot).unwrap_or_else(|_| "[]".to_string())
    }

    /// Deserialize from JSON (from KV storage).
    pub fn from_json(json: &str) -> Self {
        let snapshot: GraphSnapshot = serde_json::from_str(json).unwrap_or(GraphSnapshot {
            nodes: Vec::new(),
            edges: Vec::new(),
        });
        let mut graph = MemoryGraph::new();
        for node in snapshot.nodes {
            graph.add_node(node);
        }
        for edge in snapshot.edges {
            if let Some(relation) = Relation::from_str(&edge.relation) {
                let _ = graph.add_edge(&edge.source_id, &edge.target_id, relation, edge.weight);
            }
        }
        graph
    }
}

/// Simple keyword relevance scoring (shared with builtins).
/// Returns 0.0..1.0 based on overlapping words.
pub fn keyword_relevance(text: &str, query: &str) -> f64 {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let query_words: std::collections::HashSet<&str> = query_lower.split_whitespace().collect();
    if query_words.is_empty() {
        return 0.0;
    }
    let text_words: std::collections::HashSet<&str> = text_lower.split_whitespace().collect();
    let overlap: usize = query_words.intersection(&text_words).count();
    overlap as f64 / query_words.len() as f64
}

/// Search nodes by keyword relevance, optionally filtered by level.
/// Returns (node_id, text, level, score, relevance) sorted by composite score.
pub fn graph_search(
    graph: &MemoryGraph,
    query: &str,
    limit: usize,
    filter_level: Option<&str>,
) -> Vec<(String, String, String, f64, f64)> {
    let mut scored: Vec<(f64, &MemoryNode)> = graph
        .nodes()
        .into_iter()
        .filter(|n| {
            if let Some(level) = filter_level {
                n.level == level
            } else {
                n.level == "L0" || n.level == "L1"
            }
        })
        .map(|n| {
            let rel = keyword_relevance(&n.text, query);
            let composite = if n.level == "L1" {
                rel * 0.8 + 0.2
            } else {
                rel * 0.6 + n.score * 0.4
            };
            (composite, n)
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    scored
        .into_iter()
        .map(|(relevance, n)| {
            (
                n.id.clone(),
                n.text.clone(),
                n.level.clone(),
                n.score,
                relevance,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_node() {
        let mut g = MemoryGraph::new();
        let id = g.add_node(MemoryNode {
            id: "test_1".to_string(),
            text: "hello world".to_string(),
            level: "L0".to_string(),
            score: 0.8,
            created_at: 1000,
            source: "test".to_string(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        assert_eq!(id, "test_1");
        let node = g.get_node("test_1").unwrap();
        assert_eq!(node.text, "hello world");
    }

    #[test]
    fn test_add_edge_and_neighbors() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "a".into(),
            text: "alpha".into(),
            level: "L0".into(),
            score: 0.5,
            created_at: 1,
            source: "t".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "b".into(),
            text: "beta".into(),
            level: "L0".into(),
            score: 0.5,
            created_at: 2,
            source: "t".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "c".into(),
            text: "gamma".into(),
            level: "L0".into(),
            score: 0.5,
            created_at: 3,
            source: "t".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_edge("a", "b", Relation::DerivedFrom, 0.9).unwrap();
        g.add_edge("b", "c", Relation::RelatedTo, 0.5).unwrap();

        let nbrs = g.neighbors("a", 1);
        assert_eq!(nbrs.len(), 1); // only b at depth 1

        let nbrs = g.neighbors("a", 2);
        assert_eq!(nbrs.len(), 2); // b at depth 1, c at depth 2
    }

    #[test]
    fn test_shortest_path() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "x".into(),
            text: "".into(),
            level: "L0".into(),
            score: 0.0,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "y".into(),
            text: "".into(),
            level: "L0".into(),
            score: 0.0,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "z".into(),
            text: "".into(),
            level: "L0".into(),
            score: 0.0,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_edge("x", "y", Relation::RelatedTo, 1.0).unwrap();
        g.add_edge("y", "z", Relation::RelatedTo, 1.0).unwrap();

        let path = g.shortest_path("x", "z").unwrap();
        assert_eq!(path, vec!["x", "y", "z"]);

        assert!(g.shortest_path("x", "nonexistent").is_none());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "n1".into(),
            text: "text1".into(),
            level: "L0".into(),
            score: 0.7,
            created_at: 100,
            source: "s".into(),
            tags: vec!["tag1".into()],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "n2".into(),
            text: "text2".into(),
            level: "L1".into(),
            score: 0.9,
            created_at: 200,
            source: "s".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_edge("n1", "n2", Relation::DerivedFrom, 0.8).unwrap();

        let json = g.to_json();
        let g2 = MemoryGraph::from_json(&json);
        assert_eq!(g2.nodes().len(), 2);
        assert_eq!(g2.edges().len(), 1);
        assert_eq!(g2.get_node("n1").unwrap().text, "text1");
    }

    #[test]
    fn test_remove_node_cleans_edges() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "a".into(),
            text: "".into(),
            level: "L0".into(),
            score: 0.0,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "b".into(),
            text: "".into(),
            level: "L0".into(),
            score: 0.0,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_edge("a", "b", Relation::RelatedTo, 0.5).unwrap();
        assert_eq!(g.edges().len(), 1);
        g.remove_node("a");
        assert_eq!(g.edges().len(), 0);
        assert_eq!(g.nodes().len(), 1);
    }

    #[test]
    fn test_keyword_relevance() {
        assert!(keyword_relevance("hello world foo", "hello foo") > 0.5);
        assert_eq!(keyword_relevance("abc", "xyz"), 0.0);
        assert_eq!(keyword_relevance("test", ""), 0.0);
    }

    #[test]
    fn test_decay_reduces_old_scores() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "old".into(),
            text: "old data".into(),
            level: "L0".into(),
            score: 0.8,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "fresh".into(),
            text: "new data".into(),
            level: "L0".into(),
            score: 0.8,
            created_at: 360000,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        // Simulate 100 hours passed for old node, 0 for fresh (same timestamp as now)
        let changed = g.decay(0.1, 360000); // 100 hours
        assert_eq!(changed, 1); // only "old" should decay (fresh has created_at == now)
        assert!(g.get_node("old").unwrap().score < 0.8);
    }

    #[test]
    fn test_boost_increases_score_and_touches() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "x".into(),
            text: "data".into(),
            level: "L0".into(),
            score: 0.5,
            created_at: 1000,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        assert!(g.boost("x", 0.3, 2000));
        let node = g.get_node("x").unwrap();
        assert_eq!(node.score, 0.8);
        assert_eq!(node.last_accessed, 2000);
        assert_eq!(node.access_count, 1);
        // Boost again — should cap at 1.0
        g.boost("x", 0.5, 3000);
        assert_eq!(g.get_node("x").unwrap().score, 1.0);
        assert_eq!(g.get_node("x").unwrap().access_count, 2);
    }

    #[test]
    fn test_prune_removes_low_score_old_nodes() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "dead".into(),
            text: "junk".into(),
            level: "L0".into(),
            score: 0.01,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        g.add_node(MemoryNode {
            id: "alive".into(),
            text: "important".into(),
            level: "L0".into(),
            score: 0.8,
            created_at: 0,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        // Prune: threshold 0.05, min_age 1 hour, now = 7200 (2 hours)
        let removed = g.prune(0.05, 1.0, 7200);
        assert_eq!(removed, 1);
        assert_eq!(g.nodes().len(), 1);
        assert!(g.get_node("alive").is_some());
    }

    #[test]
    fn test_touch_updates_metadata() {
        let mut g = MemoryGraph::new();
        g.add_node(MemoryNode {
            id: "t".into(),
            text: "test".into(),
            level: "L0".into(),
            score: 0.5,
            created_at: 100,
            source: "".into(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        });
        assert!(g.touch("t", 500));
        let node = g.get_node("t").unwrap();
        assert_eq!(node.last_accessed, 500);
        assert_eq!(node.access_count, 1);
        assert_eq!(node.score, 0.5); // score unchanged by touch
        assert!(!g.touch("nonexistent", 500));
    }
}
