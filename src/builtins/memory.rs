// ── Memory / KV / Session / mtree / graph builtins ───────────────────

use crate::interpreter::Value;
use crate::memory_graph::{graph_search, MemoryGraph, MemoryNode, Relation};
use std::sync::Mutex as StdMutex;

use super::chrono_now_timestamp;
use super::core::expect_string_arg;
use super::http::make_date_struct;

// ── v0.5.0 — KV memory builtins ────────────────────────────
// These use a thread-local KV store (in-memory by default).
// When memory { persist: "..." } is configured, they also persist to SQLite kv_store table.
// Uses a write-through cache: in-memory HashMap is always authoritative;
// SQLite is a persistence backend that mirrors the HashMap.

/// Global KV store — lazy_static pattern using std::sync::OnceLock (Rust 1.70+).
static KV_STORE: std::sync::OnceLock<StdMutex<std::collections::HashMap<String, String>>> =
    std::sync::OnceLock::new();

pub(crate) fn kv_store() -> &'static StdMutex<std::collections::HashMap<String, String>> {
    KV_STORE.get_or_init(|| StdMutex::new(std::collections::HashMap::new()))
}

/// Global SQLite KV persistence backend.
/// Initialized by init_kv_persist() when memory { persist: "..." } is configured.
/// Uses std::sync::Mutex (same thread model as KV_STORE).
static KV_SQLITE: std::sync::OnceLock<StdMutex<Option<rusqlite::Connection>>> =
    std::sync::OnceLock::new();

pub(crate) fn kv_sqlite() -> &'static StdMutex<Option<rusqlite::Connection>> {
    KV_SQLITE.get_or_init(|| StdMutex::new(None))
}

/// Initialize SQLite persistence for the KV store.
/// Called by Interpreter::configure_memory() when persist path is set.
/// Creates kv_store table (key TEXT PRIMARY KEY, value TEXT) in the given database.
/// Loads existing rows into the in-memory HashMap.
pub fn init_kv_persist(db_path: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("[kv_store] Failed to open database '{}': {}", db_path, e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .map_err(|e| format!("[kv_store] Failed to create table: {}", e))?;

    // Load existing KV pairs into in-memory HashMap (write-through cache warmup)
    {
        let mut stmt = conn
            .prepare("SELECT key, value FROM kv_store")
            .map_err(|e| format!("[kv_store] Failed to query: {}", e))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("[kv_store] Failed to iterate: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        // Merge into in-memory store (SQLite is authoritative on init)
        if let Ok(mut store) = kv_store().lock() {
            for (key, value) in rows {
                store.insert(key, value);
            }
        }
    } // stmt is dropped here, releasing borrow on conn

    // Store the connection globally
    let mut sqlite_guard = kv_sqlite()
        .lock()
        .map_err(|e| format!("[kv_store] lock error: {}", e))?;
    *sqlite_guard = Some(conn);
    eprintln!("[kv_store] SQLite persistence enabled: {}", db_path);
    Ok(())
}

/// `kv_set(key, value)` — store a key-value pair.
pub(crate) fn builtin_kv_set(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_set", args, 0)?;
    let value = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("kv_set() requires 2 arguments (key, value)".to_string()),
    };
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("kv_set() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    // Write-through to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    Ok(Value::Unit)
}

/// `kv_get(key)` — retrieve a value by key. Returns empty string if not found.
pub(crate) fn builtin_kv_get(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_get", args, 0)?;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("kv_get() lock error: {}", e))?;
    Ok(Value::String(store.get(&key).cloned().unwrap_or_default()))
}

/// `kv_delete(key)` — remove a key-value pair.
pub(crate) fn builtin_kv_delete(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_delete", args, 0)?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("kv_delete() lock error: {}", e))?;
    store.remove(&key);
    // Write-through delete to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "DELETE FROM kv_store WHERE key = ?1",
                rusqlite::params![key],
            );
        }
    }
    Ok(Value::Unit)
}

/// `kv_exists(key)` — check if a key exists. Returns Bool.
pub(crate) fn builtin_kv_exists(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_exists", args, 0)?;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("kv_exists() lock error: {}", e))?;
    Ok(Value::Bool(store.contains_key(&key)))
}

/// `kv_list()` — list all keys. Returns List of Strings.
pub(crate) fn builtin_kv_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("kv_list() lock error: {}", e))?;
    let keys: Vec<Value> = store.keys().cloned().map(Value::String).collect();
    Ok(Value::List(keys))
}

// ── Наряд №6 — mem_set / mem_get / mem_delete (exact KV, not semantic) ─
// These are user-facing aliases for the KV store with String return types.
// mem_set returns the stored value, mem_get returns value or empty string,
// mem_delete returns the deleted value or empty string.
// They share the same global HashMap + optional SQLite backend as kv_*.

/// `mem_set(key, value)` — exact key-value write. Returns the stored value.
pub(crate) fn builtin_mem_set(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_set", args, 0)?;
    let value = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("mem_set() requires 2 arguments (key, value)".to_string()),
    };
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("mem_set() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    // Write-through to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    Ok(Value::String(value))
}

/// `mem_get(key)` — exact key-value read (not semantic recall).
/// Returns the value or empty string if not found.
pub(crate) fn builtin_mem_get(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_get", args, 0)?;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("mem_get() lock error: {}", e))?;
    Ok(Value::String(store.get(&key).cloned().unwrap_or_default()))
}

/// `mem_delete(key)` — remove a key-value pair. Returns the deleted value or empty string.
pub(crate) fn builtin_mem_delete(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_delete", args, 0)?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("mem_delete() lock error: {}", e))?;
    let removed = store.remove(&key);
    // Write-through delete to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "DELETE FROM kv_store WHERE key = ?1",
                rusqlite::params![key],
            );
        }
    }
    Ok(Value::String(removed.unwrap_or_default()))
}

// ── ADR-0049 — session memory (temporary per-session KV store) ──
// In-memory HashMap<String, HashMap<String, String>> — NOT persistent.
// Resets when mlog serve restarts (by design: session data is ephemeral).
// Unlike mem_set/mem_get (global), session_* is scoped to a specific session_id.
//
// Usage:
//   session_set(session_id, key, value)   -> String (stored value)
//   session_get(session_id, key)             -> String (value or "")
//   session_clear(session_id)                -> Unit

/// Global session store — lazy_static pattern using std::sync::OnceLock.
/// Outer key = session_id, inner key = data key, inner value = data value.
static SESSION_STORE: std::sync::OnceLock<
    StdMutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
> = std::sync::OnceLock::new();

pub(crate) fn session_store(
) -> &'static StdMutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>
{
    SESSION_STORE.get_or_init(|| StdMutex::new(std::collections::HashMap::new()))
}

/// Reset the entire session store. Used by contract tests to verify restart behavior.
pub(crate) fn reset_session_store() {
    if let Ok(mut store) = session_store().lock() {
        store.clear();
    }
}

/// Get the number of sessions in the store. Used by contract tests.
pub(crate) fn session_store_count() -> usize {
    session_store().lock().map(|s| s.len()).unwrap_or(0)
}

/// Get the number of keys in a specific session. Used by contract tests.
pub(crate) fn session_key_count(session_id: &str) -> usize {
    session_store()
        .lock()
        .ok()
        .and_then(|s| s.get(session_id).map(|m| m.len()))
        .unwrap_or(0)
}

/// `session_set(session_id, key, value)` — store a value scoped to a session.
/// Returns the stored value. Creates session bucket if it doesn't exist.
pub(crate) fn builtin_session_set(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_set", args, 0)?;
    let key = expect_string_arg("session_set", args, 1)?;
    let value = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => {
            return Err("session_set() requires 3 arguments (session_id, key, value)".to_string())
        }
    };
    let mut store = session_store()
        .lock()
        .map_err(|e| format!("session_set() lock error: {}", e))?;
    store
        .entry(session_id)
        .or_default()
        .insert(key.clone(), value.clone());
    Ok(Value::String(value))
}

/// `session_get(session_id, key)` — retrieve a value from a session.
/// Returns empty string if session or key not found.
pub(crate) fn builtin_session_get(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_get", args, 0)?;
    let key = expect_string_arg("session_get", args, 1)?;
    let store = session_store()
        .lock()
        .map_err(|e| format!("session_get() lock error: {}", e))?;
    let value = store
        .get(&session_id)
        .and_then(|session| session.get(&key).cloned())
        .unwrap_or_default();
    Ok(Value::String(value))
}

/// `session_clear(session_id)` — remove all keys for a session.
/// Returns "ok". No-op if session doesn't exist.
pub(crate) fn builtin_session_clear(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_clear", args, 0)?;
    let mut store = session_store()
        .lock()
        .map_err(|e| format!("session_clear() lock error: {}", e))?;
    store.remove(&session_id);
    Ok(Value::String("ok".to_string()))
}

// ── Helpers ──

/// Helper: get raw KV value (tries memory store, then SQLite).
pub(crate) fn kv_get_raw(key: &str) -> Option<String> {
    if let Ok(store) = kv_store().lock() {
        if let Some(v) = store.get(key).cloned() {
            return Some(v);
        }
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            if let Ok(v) = conn.query_row(
                "SELECT value FROM kv_store WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            ) {
                return Some(v);
            }
        }
    }
    None
}

// ── Memory Graph (v2) — petgraph-backed knowledge graph ─────────────
// Replaces flat L0/L1/L2 JSON array with a directed graph.
// Stored in KV store under "memory_graph" key as GraphSnapshot JSON.
// Auto-migrates from legacy "mtree_entries" JSON array on first access.

fn get_memory_graph() -> MemoryGraph {
    let store = kv_store().lock().ok();
    let sqlite = kv_sqlite().lock().ok();

    // Try new format first
    let raw = if let Some(ref s) = store {
        s.get("memory_graph").cloned()
    } else if let Some(ref guard) = sqlite {
        guard.as_ref().and_then(|conn| {
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = 'memory_graph'",
                [],
                |row| row.get(0),
            )
            .ok()
        })
    } else {
        None
    };

    if let Some(json) = raw {
        return MemoryGraph::from_json(&json);
    }

    // Migration: try legacy flat array format
    let legacy_raw = if let Some(ref s) = store {
        s.get("mtree_entries").cloned()
    } else if let Some(ref guard) = sqlite {
        guard.as_ref().and_then(|conn| {
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = 'mtree_entries'",
                [],
                |row| row.get(0),
            )
            .ok()
        })
    } else {
        None
    };

    if let Some(legacy_json) = legacy_raw {
        if let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&legacy_json) {
            let mut graph = MemoryGraph::new();
            for e in &entries {
                let node = MemoryNode {
                    id: e["id"].as_str().unwrap_or("").to_string(),
                    text: e["text"].as_str().unwrap_or("").to_string(),
                    level: e["level"].as_str().unwrap_or("L0").to_string(),
                    score: e["score"].as_f64().unwrap_or(0.0),
                    created_at: e["created_at"].as_i64().unwrap_or(0),
                    source: e["source"].as_str().unwrap_or("migrated").to_string(),
                    tags: vec![],
                    last_accessed: 0,
                    access_count: 0,
                };
                if !node.id.is_empty() {
                    graph.add_node(node);
                }
            }
            // Migrated — persist in new format
            save_memory_graph(&graph);
            return graph;
        }
    }

    MemoryGraph::new()
}

fn save_memory_graph(graph: &MemoryGraph) {
    let json = graph.to_json();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("memory_graph".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('memory_graph', ?1)",
                rusqlite::params![json],
            );
        }
    }
}

/// Helper: inline admission gate score (same logic as memory_score builtin).
fn compute_admission_score(text: &str) -> f64 {
    let token_count = (text.len() as f64 / 4.0).ceil() as usize;
    let words: std::collections::HashSet<&str> = text.split_whitespace().collect();
    let unique_words = words.len();
    let entity_density = {
        let caps: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.chars().next().map_or(false, |c| c.is_uppercase()) && w.len() > 1)
            .collect();
        if token_count > 0 {
            caps.len() as f64 / token_count as f64
        } else {
            0.0
        }
    };
    let score = (0.3 * (unique_words as f64 / 50.0).min(1.0))
        + (0.2 * entity_density.min(1.0))
        + (0.2 * 1.0) // recency bonus for fresh entry
        + (0.3 * (token_count as f64 / 200.0).min(1.0));
    score.min(1.0)
}

/// `mtree_store(text, source?)` — store a memory chunk as a graph node.
/// Uses inline admission gate (threshold 0.3).
/// Returns Struct { id, level, score, admitted, reason }.
/// V2: stores as graph node (no edges yet; edges added by mtree_summarize).
pub(crate) fn builtin_mtree_store(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("mtree_store", args, 0)?;
    let source = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => "user".to_string(),
    };

    let score = compute_admission_score(&text);
    let admitted = score >= 0.3;

    if !admitted {
        return Ok(make_date_struct(
            "MTreeStore",
            vec![
                ("id", Value::String("".to_string())),
                ("level", Value::String("L0".to_string())),
                ("score", Value::Float(score)),
                ("admitted", Value::Float(0.0)),
                ("reason", Value::String("below_threshold".to_string())),
            ],
        ));
    }

    let id = format!("mt_{}", chrono_now_timestamp());
    let node = MemoryNode {
        id: id.clone(),
        text: text.clone(),
        level: "L0".to_string(),
        score,
        created_at: chrono_now_timestamp(),
        source,
        tags: vec![],
        last_accessed: 0,
        access_count: 0,
    };

    let mut graph = get_memory_graph();
    graph.add_node(node);
    save_memory_graph(&graph);

    Ok(make_date_struct(
        "MTreeStore",
        vec![
            ("id", Value::String(id)),
            ("level", Value::String("L0".to_string())),
            ("score", Value::Float(score)),
            ("admitted", Value::Float(1.0)),
            ("reason", Value::String("stored".to_string())),
        ],
    ))
}

/// `mtree_retrieve(query, limit?)` — retrieve top-N relevant memories using graph search.
/// V2: uses graph_search (keyword relevance on graph nodes) instead of flat array scan.
/// Default limit: 5. Returns List of Struct { id, text, level, score, relevance }.
pub(crate) fn builtin_mtree_retrieve(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("mtree_retrieve", args, 0)?;
    let limit = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 5,
    };
    let limit = if limit == 0 { 5 } else { limit };

    let graph = get_memory_graph();
    let results = graph_search(&graph, &query, limit, None);

    let mut result = Vec::new();
    for (id, text, level, score, relevance) in &results {
        result.push(make_date_struct(
            "MTreeEntry",
            vec![
                ("id", Value::String(id.clone())),
                ("text", Value::String(text.clone())),
                ("level", Value::String(level.clone())),
                ("score", Value::Float(*score)),
                ("relevance", Value::Float(*relevance)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `mtree_forget(id)` — delete a memory node and its edges from the graph.
/// Returns Struct { id, removed, status }.
pub(crate) fn builtin_mtree_forget(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("mtree_forget", args, 0)?;
    let mut graph = get_memory_graph();
    let existed = graph.get_node(&id).is_some();
    let removed = if existed { 1.0 } else { 0.0 };
    if existed {
        graph.remove_node(&id);
        save_memory_graph(&graph);
    }
    let status = if existed { "removed" } else { "not_found" };
    Ok(make_date_struct(
        "MTreeForget",
        vec![
            ("id", Value::String(id)),
            ("removed", Value::Float(removed)),
            ("status", Value::String(status.to_string())),
        ],
    ))
}

/// `mtree_summarize()` — promote L0 entries to L1 with derived_from edges,
/// then if 3+ L1 entries exist, create L2 global summary with edges to all L1.
/// V2: uses graph edges (DerivedFrom) instead of flat JSON "summary" field.
/// Returns Struct { l0_promoted, l1_count, l2_created, status, edges }.
pub(crate) fn builtin_mtree_summarize(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let mut graph = get_memory_graph();

    // ── Phase 1: L0 → L1 ──
    let l0_nodes: Vec<MemoryNode> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.level == "L0")
        .cloned()
        .collect();

    let mut l0_promoted = 0u32;
    let batch_size = 10;
    for chunk in l0_nodes.chunks(batch_size) {
        let texts: Vec<String> = chunk.iter().map(|n| n.text.clone()).collect();
        let ids: Vec<String> = chunk.iter().map(|n| n.id.clone()).collect();

        let combined: String = texts.join(" | ");
        let summary = if combined.len() > 500 {
            format!("{}...", &combined[..497])
        } else {
            combined
        };

        let l1_id = format!("mt_l1_{}", chrono_now_timestamp());
        let l1_node = MemoryNode {
            id: l1_id.clone(),
            text: summary,
            level: "L1".to_string(),
            score: 0.7,
            created_at: chrono_now_timestamp(),
            source: "mtree_summarize".to_string(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        };
        graph.add_node(l1_node);

        // Add derived_from edges: L1 → each L0 in the batch
        for src_id in &ids {
            let _ = graph.add_edge(&l1_id, src_id, Relation::DerivedFrom, 0.8);
        }
        l0_promoted += 1;
    }

    // ── Phase 2: L1 → L2 global summary ──
    let l1_nodes: Vec<MemoryNode> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.level == "L1")
        .cloned()
        .collect();

    let l1_count = l1_nodes.len();
    let mut l2_created = 0u32;

    if l1_count >= 3 {
        // Remove any existing L2
        let l2_ids: Vec<String> = graph
            .nodes()
            .into_iter()
            .filter(|n| n.level == "L2")
            .map(|n| n.id.clone())
            .collect();
        for id in &l2_ids {
            graph.remove_node(id);
        }

        let l1_texts: Vec<String> = l1_nodes.iter().map(|n| n.text.clone()).collect();
        let l1_ids: Vec<String> = l1_nodes.iter().map(|n| n.id.clone()).collect();

        let combined: String = l1_texts.join("\n---\n");
        let global_summary = if combined.len() > 1000 {
            format!("{}...", &combined[..997])
        } else {
            combined
        };

        let l2_id = format!("mt_l2_{}", chrono_now_timestamp());
        let l2_node = MemoryNode {
            id: l2_id.clone(),
            text: global_summary,
            level: "L2".to_string(),
            score: 0.9,
            created_at: chrono_now_timestamp(),
            source: "mtree_summarize".to_string(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        };
        graph.add_node(l2_node);

        // L2 derives from all L1 entries
        for src_id in &l1_ids {
            let _ = graph.add_edge(&l2_id, src_id, Relation::DerivedFrom, 0.9);
        }
        l2_created = 1;
    }

    save_memory_graph(&graph);

    // Recount L1 after changes
    let l1_final = graph.count_by_level().get("L1").copied().unwrap_or(0);

    let (nodes, edges, components) = graph.stats();
    let status = match (l0_promoted, l2_created) {
        (0, 0) => "no_unsummarized_l0".to_string(),
        (_, 1) => "l0_and_l2_promoted".to_string(),
        (n, 0) if n > 0 => "l0_promoted".to_string(),
        _ => "no_change".to_string(),
    };

    Ok(make_date_struct(
        "MTreeSummarize",
        vec![
            ("l0_promoted", Value::Float(l0_promoted as f64)),
            ("l1_count", Value::Float(l1_final as f64)),
            ("l2_created", Value::Float(l2_created as f64)),
            ("status", Value::String(status)),
            ("graph_nodes", Value::Float(nodes as f64)),
            ("graph_edges", Value::Float(edges as f64)),
            ("components", Value::Float(components as f64)),
        ],
    ))
}

/// `mtree_stats()` — diagnostics: count entries at each level, graph metrics.
/// V2: includes graph nodes, edges, connected components.
/// Returns Struct { l0, l1, l2, total, total_chars, nodes, edges, components }.
pub(crate) fn builtin_mtree_stats(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let graph = get_memory_graph();
    let levels = graph.count_by_level();
    let l0 = *levels.get("L0").unwrap_or(&0);
    let l1 = *levels.get("L1").unwrap_or(&0);
    let l2 = *levels.get("L2").unwrap_or(&0);
    let total_chars = graph.total_chars();
    let (nodes, edges, components) = graph.stats();
    Ok(make_date_struct(
        "MTreeStats",
        vec![
            ("l0", Value::Float(l0 as f64)),
            ("l1", Value::Float(l1 as f64)),
            ("l2", Value::Float(l2 as f64)),
            ("total", Value::Float(nodes as f64)),
            ("total_chars", Value::Float(total_chars as f64)),
            ("graph_edges", Value::Float(edges as f64)),
            ("components", Value::Float(components as f64)),
        ],
    ))
}

// ── Graph Query builtins (v0.8.10) ──────────────────────────────────

/// `graph_query(query, limit?, level?)` — search memory graph by keyword relevance.
/// Same scoring as mtree_retrieve but explicit API. Optional level filter.
/// Returns List of Struct { id, text, level, score, relevance }.
pub(crate) fn builtin_graph_query(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("graph_query", args, 0)?;
    let limit = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 5,
    };
    let limit = if limit == 0 { 5 } else { limit };
    let level_filter = match args.get(2) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.as_str()),
        _ => None,
    };

    let mut graph = get_memory_graph();
    let results = graph_search(&graph, &query, limit, level_filter);

    // Touch accessed nodes to update last_accessed
    let now = chrono_now_timestamp();
    for (id, _, _, _, _) in &results {
        graph.touch(id, now);
    }
    save_memory_graph(&graph);

    let mut result = Vec::new();
    for (id, text, level, score, relevance) in &results {
        result.push(make_date_struct(
            "GraphEntry",
            vec![
                ("id", Value::String(id.clone())),
                ("text", Value::String(text.clone())),
                ("level", Value::String(level.clone())),
                ("score", Value::Float(*score)),
                ("relevance", Value::Float(*relevance)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `graph_path(from_id, to_id)` — find shortest path between two memory nodes.
/// Returns List of Struct { id, text, level } (ordered from source to target),
/// or List with single entry { status: "no_path" } if no path exists.
pub(crate) fn builtin_graph_path(args: &[Value]) -> Result<Value, String> {
    let from_id = expect_string_arg("graph_path", args, 0)?;
    let to_id = expect_string_arg("graph_path", args, 1)?;

    let graph = get_memory_graph();
    match graph.shortest_path(&from_id, &to_id) {
        Some(path) => {
            let mut result = Vec::new();
            for id in &path {
                if let Some(node) = graph.get_node(id) {
                    result.push(make_date_struct(
                        "PathNode",
                        vec![
                            ("id", Value::String(node.id.clone())),
                            ("text", Value::String(node.text.clone())),
                            ("level", Value::String(node.level.clone())),
                        ],
                    ));
                }
            }
            Ok(Value::List(result))
        }
        None => Ok(Value::List(vec![make_date_struct(
            "PathError",
            vec![
                ("status", Value::String("no_path".to_string())),
                ("from", Value::String(from_id)),
                ("to", Value::String(to_id)),
            ],
        )])),
    }
}

/// `graph_neighbors(id, depth?)` — get nodes connected to a given node within depth.
/// Default depth: 1. Bidirectional (follows edges both ways).
/// Returns List of Struct { id, text, level, distance }.
pub(crate) fn builtin_graph_neighbors(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("graph_neighbors", args, 0)?;
    let depth = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 1,
    };
    let depth = if depth == 0 { 1 } else { depth };

    let mut graph = get_memory_graph();
    let nbrs: Vec<(String, String, String, usize)> = graph
        .neighbors(&id, depth)
        .into_iter()
        .map(|(n, d)| (n.id.clone(), n.text.clone(), n.level.clone(), d))
        .collect();

    // Touch the queried node and its neighbors
    let now = chrono_now_timestamp();
    graph.touch(&id, now);
    for (nid, _, _, _) in &nbrs {
        graph.touch(nid, now);
    }
    save_memory_graph(&graph);

    let mut result = Vec::new();
    for (id, text, level, distance) in &nbrs {
        result.push(make_date_struct(
            "Neighbor",
            vec![
                ("id", Value::String(id.clone())),
                ("text", Value::String(text.clone())),
                ("level", Value::String(level.clone())),
                ("distance", Value::Float(*distance as f64)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `memory_decay(lambda?)` — apply exponential decay to all memory node scores.
/// `lambda` controls decay rate (default 0.01 = gentle).
/// Formula: score *= e^(-lambda * hours_since_access).
/// Returns Struct { decayed: <count>, nodes: <total>, edges: <total> }.
pub(crate) fn builtin_memory_decay(args: &[Value]) -> Result<Value, String> {
    let lambda = match args.get(0) {
        Some(Value::Float(f)) => *f,
        _ => 0.01,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    let decayed = graph.decay(lambda, now);
    let (nodes, edges, components) = graph.stats();
    save_memory_graph(&graph);
    Ok(make_date_struct(
        "DecayResult",
        vec![
            ("decayed", Value::Float(decayed as f64)),
            ("nodes", Value::Float(nodes as f64)),
            ("edges", Value::Float(edges as f64)),
            ("components", Value::Float(components as f64)),
        ],
    ))
}

/// `memory_boost(id, amount?)` — boost a memory node's score by amount (default 0.1, capped at 1.0).
/// Updates last_accessed timestamp. Returns Struct { id, new_score, access_count }.
pub(crate) fn builtin_memory_boost(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("memory_boost", args, 0)?;
    let amount = match args.get(1) {
        Some(Value::Float(f)) => *f,
        _ => 0.1,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    let boosted = graph.boost(&id, amount, now);
    if boosted {
        let node = match graph.get_node(&id) {
            Some(n) => n,
            None => return Err("memory_boost: node not found after boost".to_string()),
        };
        save_memory_graph(&graph);
        Ok(make_date_struct(
            "BoostResult",
            vec![
                ("id", Value::String(node.id.clone())),
                ("new_score", Value::Float(node.score)),
                ("access_count", Value::Float(node.access_count as f64)),
            ],
        ))
    } else {
        Err(format!("memory_boost: node '{}' not found", id))
    }
}

/// `memory_prune(threshold?, min_age_hours?)` — remove dead memory nodes.
/// `threshold`: minimum score to keep (default 0.05).
/// `min_age_hours`: minimum age in hours before pruning (default 24, protects fresh entries).
/// Returns Struct { pruned: <count>, remaining: <total> }.
pub(crate) fn builtin_memory_prune(args: &[Value]) -> Result<Value, String> {
    let threshold = match args.get(0) {
        Some(Value::Float(f)) => *f,
        _ => 0.05,
    };
    let min_age_hours = match args.get(1) {
        Some(Value::Float(f)) => *f,
        _ => 24.0,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    let pruned = graph.prune(threshold, min_age_hours, now);
    let (nodes, _, _) = graph.stats();
    save_memory_graph(&graph);
    Ok(make_date_struct(
        "PruneResult",
        vec![
            ("pruned", Value::Float(pruned as f64)),
            ("remaining", Value::Float(nodes as f64)),
        ],
    ))
}

// ── V2: Belief Revision ────────────────────────────────────────────

/// `memory_revise(id, new_text, new_score?)` — update a node and resolve Contradicts.
/// If the node contradicts others, the system keeps the higher-scoring belief
/// and demotes the loser (score *= 0.3, adds Supersedes edge).
/// Returns Struct { action, winner_id, superseded_id? }.
pub(crate) fn builtin_memory_revise(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("memory_revise", args, 0)?;
    let new_text = expect_string_arg("memory_revise", args, 1)?;
    let new_score = match args.get(2) {
        Some(Value::Float(f)) => *f,
        _ => 0.8,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    match graph.revise(&id, &new_text, new_score, now) {
        Some(result) => {
            save_memory_graph(&graph);
            let mut fields = vec![
                ("action", Value::String(result.action)),
                ("winner_id", Value::String(result.winner_id)),
            ];
            if let Some(sid) = result.superseded_id {
                fields.push(("superseded_id", Value::String(sid)));
            }
            Ok(make_date_struct("ReviseResult", fields))
        }
        None => Err(format!("memory_revise: node '{}' not found", id)),
    }
}

// ── V3: Subgraph as First-Class Value ──────────────────────────────

/// `subgraph_extract(id, depth?)` — extract a subgraph around a node as a first-class value.
/// Returns an opaque Subgraph value containing nodes and edges within depth.
/// Pass to subgraph_nodes() or subgraph_json() to inspect.
pub(crate) fn builtin_subgraph_extract(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("subgraph_extract", args, 0)?;
    let depth = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 2,
    };
    let depth = if depth == 0 { 2 } else { depth };

    let mut graph = get_memory_graph();
    let nbrs: Vec<(String, String, String, usize)> = graph
        .neighbors(&id, depth)
        .into_iter()
        .map(|(n, d)| (n.id.clone(), n.text.clone(), n.level.clone(), d))
        .collect();
    let mut ids: Vec<String> = nbrs.iter().map(|(nid, _, _, _)| nid.clone()).collect();
    // Include the center node itself
    if graph.get_node(&id).is_some() && !ids.contains(&id) {
        ids.insert(0, id.clone());
    }

    // Touch accessed nodes
    let now = chrono_now_timestamp();
    graph.touch(&id, now);
    for nid in &ids {
        graph.touch(nid, now);
    }
    save_memory_graph(&graph);

    let snapshot = graph.subgraph(&ids);
    Ok(Value::Subgraph(snapshot))
}

/// `subgraph_nodes(subgraph_value)` — extract node list from a Subgraph value.
/// Returns List of Struct { id, text, level, score }.
pub(crate) fn builtin_subgraph_nodes(args: &[Value]) -> Result<Value, String> {
    let snap = match args.get(0) {
        Some(Value::Subgraph(s)) => s,
        _ => return Err("subgraph_nodes: expected Subgraph value as first argument".to_string()),
    };
    let mut result = Vec::new();
    for node in &snap.nodes {
        result.push(make_date_struct(
            "GraphNode",
            vec![
                ("id", Value::String(node.id.clone())),
                ("text", Value::String(node.text.clone())),
                ("level", Value::String(node.level.clone())),
                ("score", Value::Float(node.score)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `subgraph_json(subgraph_value)` — serialize a Subgraph to JSON string.
pub(crate) fn builtin_subgraph_json(args: &[Value]) -> Result<Value, String> {
    let snap = match args.get(0) {
        Some(Value::Subgraph(s)) => s,
        _ => return Err("subgraph_json: expected Subgraph value as first argument".to_string()),
    };
    match serde_json::to_string(snap) {
        Ok(json) => Ok(Value::String(json)),
        Err(e) => Err(format!("subgraph_json: serialization failed: {}", e)),
    }
}

// ── V4: Execution Tracing ──────────────────────────────────────────

/// `trace_start(name)` — begin a named trace span. Stores start time in session.
/// Returns Unit.
pub(crate) fn builtin_trace_start(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("trace_start", args, 0)?;
    let key = format!("__trace_{}", name);
    let now = format!("{}", chrono_now_timestamp());
    if let Ok(mut store) = kv_store().lock() {
        store.insert(key, now);
    }
    Ok(Value::Unit)
}

/// `trace_end(name)` — end a named trace span. Returns Struct { name, elapsed_ms, elapsed_secs }.
/// Removes the start time from session.
pub(crate) fn builtin_trace_end(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("trace_end", args, 0)?;
    let key = format!("__trace_{}", name);
    let start: i64 = if let Ok(store) = kv_store().lock() {
        match store.get(&key) {
            Some(s) => s.parse().unwrap_or(0),
            None => return Err(format!("trace_end: no active trace '{}'", name)),
        }
    } else {
        return Err("trace_end: cannot access session store".to_string());
    };
    let now = chrono_now_timestamp();
    let elapsed_ms = (now - start).max(0) * 1000; // seconds→ms (timestamp is seconds)
                                                  // Clean up
    if let Ok(mut store) = kv_store().lock() {
        store.remove(&key);
    }
    Ok(make_date_struct(
        "TraceSpan",
        vec![
            ("name", Value::String(name)),
            ("elapsed_ms", Value::Float(elapsed_ms as f64)),
            ("elapsed_secs", Value::Float((now - start).max(0) as f64)),
        ],
    ))
}

// ════════════════════════════════════════════════════════════════════
// sqz-inspired builtins (P1 + P2 + P3)
// Source concept: https://github.com/ojuschugh1/sqz (ELv2 — no code copied)
// ════════════════════════════════════════════════════════════════════

// ── P2: Content-addressed refs ─────────────────────────────────────

/// `ref(content)` — compute SHA-256 hash, store in KV, return hash string. Idempotent.
pub(crate) fn builtin_content_ref(args: &[Value]) -> Result<Value, String> {
    let content = expect_string_arg("ref", args, 0)?;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex::encode(hasher.finalize());
    let key = format!("__ref:{}", hash);
    // Only set if not already present (idempotent)
    let store = kv_store()
        .lock()
        .map_err(|e| format!("ref() lock error: {}", e))?;
    if !store.contains_key(&key) {
        drop(store); // release read lock
        let mut store = kv_store()
            .lock()
            .map_err(|e| format!("ref() lock error: {}", e))?;
        store.insert(key.clone(), content.clone());
        // Write-through to SQLite if available
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO kv_store (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, content],
                );
            }
        }
    }
    Ok(Value::String(hash))
}

/// `deref(hash)` — retrieve content by SHA-256 hash from ref store.
pub(crate) fn builtin_content_deref(args: &[Value]) -> Result<Value, String> {
    let hash = expect_string_arg("deref", args, 0)?;
    if hash.len() != 64 {
        return Err("deref: invalid hash format, expected 64-char hex string".to_string());
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("deref: invalid hash format, expected hex characters only".to_string());
    }
    let key = format!("__ref:{}", hash);
    let store = kv_store()
        .lock()
        .map_err(|e| format!("deref() lock error: {}", e))?;
    match store.get(&key) {
        Some(content) => Ok(Value::String(content.clone())),
        None => Err("deref: hash not found in ref store".to_string()),
    }
}

// ── P3: Token awareness ────────────────────────────────────────────

/// `token_count(text)` — estimate token count. Cyrillic: chars/2, Latin: chars/4.
pub(crate) fn builtin_token_count(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("token_count", args, 0)?;
    if s.is_empty() {
        return Ok(Value::Float(0.0));
    }
    let total_chars = s.chars().count();
    let cyrillic_chars = s
        .chars()
        .filter(|c| matches!(c, '\u{0400}'..='\u{04FF}'))
        .count();
    // If >=50% Cyrillic, use /2 divisor; else /4
    let divisor = if total_chars > 0 && (cyrillic_chars as f64 / total_chars as f64) >= 0.5 {
        2.0
    } else {
        4.0
    };
    let tokens = (total_chars as f64 / divisor).ceil();
    Ok(Value::Float(tokens))
}
