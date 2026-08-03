// ── Memory Store for METALOGOS (Phase 7.6) ──────────────────────────
// Provides persistent memory storage via SQLite and in-memory fallback.
//
// Architecture:
//   MemoryStore trait  — abstracts memorize/recall/forget/decay operations
//   InMemoryStore      — Vec-backed (backward compatible, no persistence)
//   SqliteStore        — SQLite-backed (persists across restarts)
//
//   KgStore trait      — abstracts knowledge graph (relate operations)
//   InMemoryKg         — Vec-backed (backward compatible)
//   SqliteKg           — SQLite-backed (persists across restarts)
//
// Configuration via `memory { persist: "./data/memory.db" }` in .mlog.
// Without persist → in-memory stores (old behavior, all tests pass unchanged).

use std::path::Path;

// ── Memory Entry (shared between all store implementations) ──────────

/// A single memory entry: stored fact with priority, timestamp, decay, and embedding.
///
/// `mem_type` classifies the entry for type-aware recall and differentiated decay.
/// Types: "" (legacy), "persona", "episodic", "instruction", "fact".
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// Database row ID (None for in-memory entries).
    pub id: Option<i64>,
    /// The stored value (string content of the fact).
    pub value: String,
    /// Priority/confidence at time of memorization (0.0..1.0).
    pub priority: f64,
    /// Unix timestamp (seconds) when memorized.
    pub timestamp: i64,
    /// Decay rate per day (0.0 = no decay, 0.01 = slow, 0.1 = fast).
    pub decay_rate: f64,
    /// Confidence score (computed or stored).
    pub confidence: f64,
    /// Embedding vector for semantic recall (Phase 7.2).
    /// Serialized as BLOB in SQLite, stored as Vec<f32> in memory.
    pub embedding: Vec<f32>,
    /// Memory type for type-aware recall: "", "persona", "episodic", "instruction", "fact".
    pub mem_type: String,
}

// ── Knowledge Graph Entry ──────────────────────────────────────────

/// A knowledge graph node.
#[derive(Debug, Clone)]
pub struct KgNode {
    pub id: i64,
    pub value: String,
    pub node_type: String,
}

/// A knowledge graph edge.
#[derive(Debug, Clone)]
pub struct KgEdge {
    pub from_id: i64,
    pub to_id: i64,
    pub relation: String,
    pub weight: f64,
}

// ── MemoryStore Trait ────────────────────────────────────────────────

/// Trait for memory storage backends.
/// Implementations: InMemoryStore (default), SqliteStore (persistent).
pub trait MemoryStore: Send + Sync {
    /// Insert a new memory entry. Returns the assigned ID.
    fn memorize(&mut self, entry: MemoryEntry) -> Result<i64, String>;

    /// Recall: find best matching entry using semantic similarity or substring fallback.
    /// Returns (entry, score) for the best match above min_confidence, or None.
    fn recall(
        &self,
        query: &str,
        query_embedding: &[f32],
        min_confidence: f32,
    ) -> Option<(MemoryEntry, f32)>;

    /// Recall top-K entries with optional type filter (ADR-0073: Phase 3 hybrid search).
    /// Returns entries sorted by score descending, up to `limit` results.
    /// `type_filter`: if non-empty, only return entries matching this mem_type.
    /// Default implementation: falls back to scanning all_entries (InMemoryStore).
    /// SqliteStore overrides with FTS5 BM25 + cosine hybrid.
    fn recall_top_k(
        &self,
        query: &str,
        query_embedding: &[f32],
        min_confidence: f32,
        limit: usize,
        type_filter: &str,
    ) -> Vec<(MemoryEntry, f32)> {
        // Default: delegate to all_entries + in-Rust scoring (InMemoryStore path).
        use crate::embeddings::cosine_similarity;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut scored: Vec<(MemoryEntry, f32)> = self.all_entries()
            .into_iter()
            .filter(|e| type_filter.is_empty() || e.mem_type == type_filter)
            .filter_map(|e| {
                let sim = if !query_embedding.is_empty() && !e.embedding.is_empty() {
                    cosine_similarity(query_embedding, &e.embedding)
                } else if e.value.contains(query) {
                    1.0
                } else {
                    return None;
                };
                let age_days = ((now - e.timestamp).max(0) as f64) / 86400.0;
                let decay = (-e.decay_rate * age_days).exp() as f32;
                let score = sim * (e.priority as f32) * decay;
                if score >= min_confidence {
                    Some((e, score))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    /// Delete entries matching query string that are older than cutoff timestamp.
    fn forget(&mut self, query: &str, cutoff: i64);

    /// Apply exponential decay to all entries: priority *= exp(-decay_rate * age_days).
    /// Returns the number of entries updated.
    fn decay(&mut self) -> usize;

    /// Return all entries (for graph walk + export).
    fn all_entries(&self) -> Vec<MemoryEntry>;

    /// Count of stored memories.
    fn count(&self) -> usize;
}

// ── KgStore Trait ──────────────────────────────────────────────────

/// Trait for knowledge graph storage backends.
pub trait KgStore: Send + Sync {
    /// Insert a relation edge between two values.
    fn relate(&mut self, from: &str, to: &str, relation: &str, weight: f64) -> Result<(), String>;

    /// Find all edges connected to a value. Returns (relation, other_value, weight) tuples.
    fn edges_for(&self, value: &str) -> Vec<(String, String, f64)>;

    /// Graph walk: find all nodes reachable from a starting value via edges.
    /// Returns (relation, other_value, weight) tuples including transitive connections.
    fn walk(&self, value: &str, max_depth: usize) -> Vec<(String, String, f64)>;

    /// Count of stored edges.
    fn edge_count(&self) -> usize;

    /// Return all edges (for export).
    fn all_edges(&self) -> Vec<(String, String, String, f64)>;
}

// ── InMemoryStore ─────────────────────────────────────────────────

/// In-memory memory store (default, backward compatible).
/// Uses Vec<MemoryEntry> — identical to pre-7.6 behavior.
pub struct InMemoryStore {
    entries: Vec<MemoryEntry>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        InMemoryStore {
            entries: Vec::new(),
        }
    }
}

impl MemoryStore for InMemoryStore {
    fn memorize(&mut self, entry: MemoryEntry) -> Result<i64, String> {
        let id = self.entries.len() as i64;
        self.entries.push(entry);
        Ok(id)
    }

    fn recall(
        &self,
        query: &str,
        query_embedding: &[f32],
        min_confidence: f32,
    ) -> Option<(MemoryEntry, f32)> {
        use crate::embeddings::cosine_similarity;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut best_match: Option<&MemoryEntry> = None;
        let mut best_score: f32 = 0.0;

        for entry in &self.entries {
            let semantic_sim = if !query_embedding.is_empty() && !entry.embedding.is_empty() {
                cosine_similarity(query_embedding, &entry.embedding)
            } else {
                // Fallback: substring match
                if entry.value.contains(query) {
                    1.0
                } else {
                    continue;
                }
            };

            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let decay = (-entry.decay_rate * age_days).exp() as f32;
            let score = semantic_sim * (entry.priority as f32) * decay;

            if score > best_score && score >= min_confidence {
                best_score = score;
                best_match = Some(entry);
            }
        }

        best_match.map(|e| (e.clone(), best_score))
    }

    fn forget(&mut self, query: &str, cutoff: i64) {
        self.entries
            .retain(|e| !(e.value.contains(query) && e.timestamp < cutoff));
    }

    fn decay(&mut self) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut count = 0;
        for entry in &mut self.entries {
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let new_priority = entry.priority * (-entry.decay_rate * age_days).exp();
            if (new_priority - entry.priority).abs() > 1e-10 {
                entry.priority = new_priority;
                count += 1;
            }
        }
        count
    }

    fn all_entries(&self) -> Vec<MemoryEntry> {
        self.entries.clone()
    }

    fn count(&self) -> usize {
        self.entries.len()
    }
}

// ── InMemoryKg ─────────────────────────────────────────────────────

/// In-memory knowledge graph store (default, backward compatible).
pub struct InMemoryKg {
    edges: Vec<(String, String, String, f64)>, // (from, to, relation, weight)
}

impl Default for InMemoryKg {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryKg {
    pub fn new() -> Self {
        InMemoryKg { edges: Vec::new() }
    }
}

impl KgStore for InMemoryKg {
    fn relate(&mut self, from: &str, to: &str, relation: &str, weight: f64) -> Result<(), String> {
        self.edges.push((
            from.to_string(),
            to.to_string(),
            relation.to_string(),
            weight,
        ));
        Ok(())
    }

    fn edges_for(&self, value: &str) -> Vec<(String, String, f64)> {
        let mut result = Vec::new();
        for (from, to, relation, weight) in &self.edges {
            if from == value {
                result.push((relation.clone(), to.clone(), *weight));
            } else if to == value {
                result.push((relation.clone(), from.clone(), *weight));
            }
        }
        result
    }

    fn walk(&self, value: &str, max_depth: usize) -> Vec<(String, String, f64)> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(value.to_string());
        self.walk_recursive(value, max_depth, 0, &mut visited, &mut result);
        result
    }

    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn all_edges(&self) -> Vec<(String, String, String, f64)> {
        self.edges.clone()
    }
}

impl InMemoryKg {
    fn walk_recursive(
        &self,
        current: &str,
        max_depth: usize,
        depth: usize,
        visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<(String, String, f64)>,
    ) {
        if depth >= max_depth {
            return;
        }
        for (from, to, relation, weight) in &self.edges {
            let (neighbor, _direction) = if from == current {
                (to.as_str(), "outgoing")
            } else if to == current {
                (from.as_str(), "incoming")
            } else {
                continue;
            };
            if !visited.contains(neighbor) {
                visited.insert(neighbor.to_string());
                result.push((relation.clone(), neighbor.to_string(), *weight));
                self.walk_recursive(neighbor, max_depth, depth + 1, visited, result);
            }
        }
    }
}

// ── SqliteStore ─────────────────────────────────────────────────────

/// SQLite-backed memory store for persistence across restarts.
pub struct SqliteStore {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SqliteStore {
    /// Load all entries with their row IDs (for decay update).
    fn load_all(conn: &rusqlite::Connection) -> Result<Vec<MemoryEntry>, String> {
        let mut stmt = conn.prepare(
            "SELECT id, value, priority, confidence, decay_rate, created_at, embedding, mem_type FROM memories"
        ).map_err(|e| format!("Load all failed: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(MemoryEntry {
                    id: Some(row.get(0)?),
                    value: row.get(1)?,
                    priority: row.get(2)?,
                    confidence: row.get(3)?,
                    decay_rate: row.get(4)?,
                    timestamp: row.get(5)?,
                    embedding: Self::blob_to_embedding(
                        &row.get::<_, Vec<u8>>(6).unwrap_or_default(),
                    ),
                    mem_type: row.get::<_, String>(7).unwrap_or_default(),
                })
            })
            .map_err(|e| format!("Load all query failed: {}", e))?;

        let mut entries = Vec::new();
        for row in rows {
            match row {
                Ok(e) => entries.push(e),
                Err(_) => continue,
            }
        }
        Ok(entries)
    }

    /// Open a SQLite memory store from a file path. Creates tables if needed.
    pub fn open(path: &Path) -> Result<Self, String> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create memory db directory: {}", e))?;
        }

        let conn = rusqlite::Connection::open(path)
            .map_err(|e| format!("Failed to open memory database '{}': {}", path.display(), e))?;

        // Initialize schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT,
                value TEXT NOT NULL,
                priority REAL NOT NULL DEFAULT 1.0,
                confidence REAL NOT NULL DEFAULT 1.0,
                decay_rate REAL NOT NULL DEFAULT 0.01,
                created_at INTEGER NOT NULL,
                embedding BLOB,
                mem_type TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_memories_value ON memories(value);
            CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
            CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(mem_type);",
        )
        .map_err(|e| format!("Failed to create memories table: {}", e))?;

        // FTS5 full-text index for BM25 keyword search (Phase 2: hybrid recall).
        // Content-synced via triggers so INSERT/UPDATE/DELETE on memories
        // automatically propagate to the FTS table.
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                value,
                mem_type UNINDEXED,
                content=memories,
                content_rowid=id
            );
            CREATE TRIGGER IF NOT EXISTS memories_fts_insert AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, value, mem_type)
                VALUES (new.id, new.value, new.mem_type);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_fts_delete AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, value, mem_type)
                VALUES ('delete', old.id, old.value, old.mem_type);
            END;
            CREATE TRIGGER IF NOT EXISTS memories_fts_update AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, value, mem_type)
                VALUES ('delete', old.id, old.value, old.mem_type);
                INSERT INTO memories_fts(rowid, value, mem_type)
                VALUES (new.id, new.value, new.mem_type);
            END;",
        )
        .map_err(|e| format!("Failed to create FTS5 table: {}", e))?;

        Ok(SqliteStore {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Convert embedding Vec<f32> to BLOB bytes.
    fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &val in embedding {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// Convert BLOB bytes back to Vec<f32>.
    fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
        let mut result = Vec::with_capacity(blob.len() / 4);
        for chunk in blob.chunks_exact(4) {
            let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            result.push(val);
        }
        result
    }

    /// Query FTS5 BM25 index for keyword matches. Returns rowid → positive score map.
    /// Empty query or FTS5 errors return empty map.
    fn bm25_search(
        conn: &rusqlite::Connection,
        query: &str,
        limit: usize,
        type_filter: &str,
    ) -> std::collections::HashMap<i64, f32> {
        if query.is_empty() {
            return std::collections::HashMap::new();
        }
        let rows_result = if type_filter.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT rowid, rank FROM memories_fts WHERE memories_fts MATCH ?1 ORDER BY rank LIMIT ?2"
            ) {
                Ok(s) => s,
                Err(_) => return std::collections::HashMap::new(),
            };
            stmt.query_map(rusqlite::params![query, limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })
        } else {
            let mut stmt = match conn.prepare(
                "SELECT rowid, rank FROM memories_fts WHERE memories_fts MATCH ?1 AND mem_type = ?3 ORDER BY rank LIMIT ?2"
            ) {
                Ok(s) => s,
                Err(_) => return std::collections::HashMap::new(),
            };
            stmt.query_map(rusqlite::params![query, limit as i64, type_filter], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })
        };
        match rows_result {
            Ok(rows) => rows
                .filter_map(|r| r.ok())
                .map(|(rowid, rank)| (rowid, -rank as f32)) // FTS5 rank is negative
                .collect(),
            Err(_) => std::collections::HashMap::new(),
        }
    }
}

impl MemoryStore for SqliteStore {
    fn memorize(&mut self, entry: MemoryEntry) -> Result<i64, String> {
        let blob = Self::embedding_to_blob(&entry.embedding);
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT INTO memories (value, priority, confidence, decay_rate, created_at, embedding, mem_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![entry.value, entry.priority, entry.confidence, entry.decay_rate, entry.timestamp, blob, entry.mem_type],
        ).map_err(|e| format!("Failed to memorize: {}", e))?;

        Ok(conn.last_insert_rowid())
    }

    fn recall(
        &self,
        query: &str,
        query_embedding: &[f32],
        min_confidence: f32,
    ) -> Option<(MemoryEntry, f32)> {
        use crate::embeddings::cosine_similarity;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return None,
        };

        let mut stmt = match conn.prepare(
            "SELECT id, value, priority, confidence, decay_rate, created_at, embedding, mem_type FROM memories"
        ) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let rows = match stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Vec<u8>>(6).unwrap_or_default(),
                row.get::<_, String>(7).unwrap_or_default(),
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return None,
        };

        let mut best_match: Option<MemoryEntry> = None;
        let mut best_score: f32 = 0.0;

        for row in rows {
            let (_id, value, priority, confidence, decay_rate, timestamp, blob, mem_type) = match row {
                Ok(r) => r,
                Err(_) => continue,
            };

            let entry_embedding = Self::blob_to_embedding(&blob);

            let semantic_sim = if !query_embedding.is_empty() && !entry_embedding.is_empty() {
                cosine_similarity(query_embedding, &entry_embedding)
            } else {
                if value.contains(query) {
                    1.0
                } else {
                    continue;
                }
            };

            let age_days = ((now - timestamp).max(0) as f64) / 86400.0;
            let decay = (-decay_rate * age_days).exp() as f32;
            let score = semantic_sim * (priority as f32) * decay;

            if score > best_score && score >= min_confidence {
                best_score = score;
                best_match = Some(MemoryEntry {
                    id: None,
                    value,
                    priority,
                    timestamp,
                    decay_rate,
                    confidence,
                    embedding: entry_embedding,
                    mem_type,
                });
            }
        }

        best_match.map(|e| (e, best_score))
    }

    fn forget(&mut self, query: &str, cutoff: i64) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.execute(
            "DELETE FROM memories WHERE value LIKE '%' || ?1 || '%' AND created_at < ?2",
            rusqlite::params![query, cutoff],
        );
    }

    fn decay(&mut self) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };

        // Load all entries, compute decay in Rust, UPDATE individually.
        // We avoid relying on SQLite's exp() which may not be compiled in.
        let entries = match Self::load_all(&conn) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let mut count = 0;
        for mut entry in entries {
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let new_priority = entry.priority * (-entry.decay_rate * age_days).exp();
            if (new_priority - entry.priority).abs() > 1e-10 {
                entry.priority = new_priority;
                let _ = conn.execute(
                    "UPDATE memories SET priority = ?1 WHERE id = ?2",
                    rusqlite::params![new_priority, entry.id],
                );
                count += 1;
            }
        }
        count
    }

    fn all_entries(&self) -> Vec<MemoryEntry> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut stmt = match conn.prepare(
            "SELECT value, priority, confidence, decay_rate, created_at, embedding, mem_type FROM memories",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut entries = Vec::new();
        if let Ok(mapped_rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5).unwrap_or_default(),
                row.get::<_, String>(6).unwrap_or_default(),
            ))
        }) {
            Ok(mapped_rows) => {
                for row_result in mapped_rows {
                    match row_result {
                        Ok((value, priority, confidence, decay_rate, timestamp, blob, mem_type)) => {
                            entries.push(MemoryEntry {
                                id: None,
                                value,
                                priority,
                                timestamp,
                                decay_rate,
                                confidence,
                                embedding: Self::blob_to_embedding(&blob),
                                mem_type,
                            });
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        entries
    }

    fn count(&self) -> usize {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .unwrap_or(0)
    }

    /// Hybrid recall using FTS5 BM25 + cosine similarity with weighted blend (ADR-0073).
    /// Returns top-K entries sorted by combined score.
    /// `type_filter`: if non-empty, restricts results to entries with matching mem_type.
    fn recall_top_k(
        &self,
        query: &str,
        query_embedding: &[f32],
        min_confidence: f32,
        limit: usize,
        type_filter: &str,
    ) -> Vec<(MemoryEntry, f32)> {
        use crate::embeddings::cosine_similarity;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        // Step 1: BM25 candidates from FTS5 (keyword matching).
        let bm25_scores = SqliteStore::bm25_search(&*conn, query, limit, type_filter);

        // Step 2: Fetch candidate entries (BM25 matches + type filter).
        let candidate_ids: Vec<i64> = if !bm25_scores.is_empty() {
            bm25_scores.keys().copied().collect()
        } else if type_filter.is_empty() {
            // No BM25 hits — fall back to cosine/substring scan.
            match conn.prepare("SELECT id FROM memories LIMIT ?1") {
                Ok(mut stmt) => {
                    match stmt.query_map(rusqlite::params![limit as i64 * 10], |row| {
                        row.get::<_, i64>(0)
                    }) {
                        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                        Err(_) => return Vec::new(),
                    }
                }
                Err(_) => return Vec::new(),
            }
        } else {
            match conn.prepare("SELECT id FROM memories WHERE mem_type = ?2 LIMIT ?1") {
                Ok(mut stmt) => {
                    match stmt.query_map(rusqlite::params![limit as i64 * 10, type_filter], |row| {
                        row.get::<_, i64>(0)
                    }) {
                        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                        Err(_) => return Vec::new(),
                    }
                }
                Err(_) => return Vec::new(),
            }
        };

        // Step 3: Load full entries for candidates and compute cosine scores.
        let mut scored: Vec<(MemoryEntry, f32)> = Vec::new();
        for id in candidate_ids {
            let entry = match conn.query_row(
                "SELECT id, value, priority, confidence, decay_rate, created_at, embedding, mem_type FROM memories WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(MemoryEntry {
                        id: Some(row.get(0)?),
                        value: row.get(1)?,
                        priority: row.get(2)?,
                        confidence: row.get(3)?,
                        decay_rate: row.get(4)?,
                        timestamp: row.get(5)?,
                        embedding: Self::blob_to_embedding(&row.get::<_, Vec<u8>>(6).unwrap_or_default()),
                        mem_type: row.get::<_, String>(7).unwrap_or_default(),
                    })
                },
            ) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Compute cosine similarity score
            let cosine_score = if !query_embedding.is_empty() && !entry.embedding.is_empty() {
                cosine_similarity(query_embedding, &entry.embedding)
            } else if entry.value.contains(query) {
                1.0
            } else {
                continue; // no match from either signal
            };

            // Apply temporal decay
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let decay = (-entry.decay_rate * age_days).exp() as f32;

            // Weighted blend: BM25 + cosine. When no BM25 hit, cosine only.
            let bm25_score = bm25_scores.get(&id).copied().unwrap_or(0.0);
            let combined = if bm25_score > 0.0 {
                let bm25_norm = bm25_score.max(0.0).min(1.0);
                // 40% BM25 + 60% cosine*decay*priority
                0.4 * bm25_norm + 0.6 * cosine_score * decay * (entry.priority as f32)
            } else {
                cosine_score * (entry.priority as f32) * decay
            };

            if combined >= min_confidence {
                scored.push((entry, combined));
            }
        }

        // Step 4: Sort by combined score descending, take top-K.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}

// ── SqliteKg ───────────────────────────────────────────────────────

/// SQLite-backed knowledge graph store for persistence across restarts.
pub struct SqliteKg {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SqliteKg {
    /// Open KG tables in a SQLite database file. Creates tables if needed.
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| format!("Failed to open KG database '{}': {}", path.display(), e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kg_nodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value TEXT NOT NULL UNIQUE,
                type TEXT NOT NULL DEFAULT 'fact'
            );
            CREATE TABLE IF NOT EXISTS kg_edges (
                from_id INTEGER NOT NULL REFERENCES kg_nodes(id),
                to_id INTEGER NOT NULL REFERENCES kg_nodes(id),
                relation TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0
            );
            CREATE INDEX IF NOT EXISTS idx_kg_nodes_value ON kg_nodes(value);
            CREATE INDEX IF NOT EXISTS idx_kg_edges_from ON kg_edges(from_id);
            CREATE INDEX IF NOT EXISTS idx_kg_edges_to ON kg_edges(to_id);
            CREATE INDEX IF NOT EXISTS idx_kg_edges_rel ON kg_edges(relation);",
        )
        .map_err(|e| format!("Failed to create KG tables: {}", e))?;

        Ok(SqliteKg {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Get or create a node ID for a given value.
    fn get_or_create_node(&self, value: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Try to find existing node
        if let Ok(id) = conn.query_row(
            "SELECT id FROM kg_nodes WHERE value = ?1",
            rusqlite::params![value],
            |row| row.get(0),
        ) {
            return Ok(id);
        }

        // Insert new node
        conn.execute(
            "INSERT INTO kg_nodes (value, type) VALUES (?1, 'fact')",
            rusqlite::params![value],
        )
        .map_err(|e| format!("Failed to create KG node: {}", e))?;

        Ok(conn.last_insert_rowid())
    }
}

impl KgStore for SqliteKg {
    fn relate(&mut self, from: &str, to: &str, relation: &str, weight: f64) -> Result<(), String> {
        let from_id = self.get_or_create_node(from)?;
        let to_id = self.get_or_create_node(to)?;

        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "INSERT INTO kg_edges (from_id, to_id, relation, weight) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![from_id, to_id, relation, weight],
        )
        .map_err(|e| format!("Failed to create KG edge: {}", e))?;

        Ok(())
    }

    fn edges_for(&self, value: &str) -> Vec<(String, String, f64)> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let node_id = match conn.query_row(
            "SELECT id FROM kg_nodes WHERE value = ?1",
            rusqlite::params![value],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(id) => id,
            Err(_) => return Vec::new(),
        };

        let mut result = Vec::new();

        // Outgoing edges
        let mut stmt = match conn.prepare(
            "SELECT relation, kg_nodes.value, weight FROM kg_edges
             JOIN kg_nodes ON kg_edges.to_id = kg_nodes.id
             WHERE kg_edges.from_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        if let Ok(mapped_rows) = stmt.query_map(rusqlite::params![node_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        }) {
            for (rel, val, w) in mapped_rows.flatten() {
                result.push((rel, val, w));
            }
        }

        // Incoming edges
        let mut stmt = match conn.prepare(
            "SELECT relation, kg_nodes.value, weight FROM kg_edges
             JOIN kg_nodes ON kg_edges.from_id = kg_nodes.id
             WHERE kg_edges.to_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        if let Ok(mapped_rows) = stmt.query_map(rusqlite::params![node_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        }) {
            for (rel, val, w) in mapped_rows.flatten() {
                result.push((rel, val, w));
            }
        }

        result
    }

    fn walk(&self, value: &str, max_depth: usize) -> Vec<(String, String, f64)> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(value.to_string());
        self.walk_recursive(value, max_depth, 0, &mut visited, &mut result);
        result
    }

    fn edge_count(&self) -> usize {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COUNT(*) FROM kg_edges", [], |row| row.get(0))
            .unwrap_or(0)
    }

    fn all_edges(&self) -> Vec<(String, String, String, f64)> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut stmt = match conn.prepare(
            "SELECT n1.value, n2.value, e.relation, e.weight
             FROM kg_edges e
             JOIN kg_nodes n1 ON e.from_id = n1.id
             JOIN kg_nodes n2 ON e.to_id = n2.id",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut entries = Vec::new();
        if let Ok(mapped_rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        }) {
            entries.extend(mapped_rows.flatten());
        }
        entries
    }
}

impl SqliteKg {
    fn walk_recursive(
        &self,
        current: &str,
        max_depth: usize,
        depth: usize,
        visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<(String, String, f64)>,
    ) {
        if depth >= max_depth {
            return;
        }

        // Collect neighbors in a scoped block so the Mutex lock is released before recursing.
        // std::sync::Mutex is not reentrant — holding the lock while recursing causes deadlock.
        let to_visit: Vec<(String, String, f64)> = {
            let conn = match self.conn.lock() {
                Ok(c) => c,
                Err(_) => return,
            };

            let node_id = match conn.query_row(
                "SELECT id FROM kg_nodes WHERE value = ?1",
                rusqlite::params![current],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(id) => id,
                Err(_) => return,
            };

            let mut neighbors: Vec<(String, f64, String)> = Vec::new();

            // Outgoing edges
            {
                let mut stmt = match conn.prepare(
                    "SELECT e.relation, e.weight, n.value FROM kg_edges e
                     JOIN kg_nodes n ON e.to_id = n.id WHERE e.from_id = ?1",
                ) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let _ = stmt
                    .query_map(rusqlite::params![node_id], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, f64>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })
                    .map(|rows| {
                        neighbors.extend(rows.flatten());
                    });
            } // stmt dropped here

            // Incoming edges
            {
                let mut stmt = match conn.prepare(
                    "SELECT e.relation, e.weight, n.value FROM kg_edges e
                     JOIN kg_nodes n ON e.from_id = n.id WHERE e.to_id = ?1",
                ) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let _ = stmt
                    .query_map(rusqlite::params![node_id], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, f64>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })
                    .map(|rows| {
                        neighbors.extend(rows.flatten());
                    });
            } // stmt dropped here

            // conn (MutexGuard) dropped here when this block ends

            let mut to_visit = Vec::new();
            for (relation, weight, neighbor) in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    result.push((relation.clone(), neighbor.clone(), weight));
                    to_visit.push((relation, neighbor, weight));
                }
            }
            to_visit
        }; // conn lock released here

        // Recurse after lock is released
        for (_, neighbor, _) in to_visit {
            self.walk_recursive(&neighbor, max_depth, depth + 1, visited, result);
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    // ── InMemoryStore ───────────────────────────────────────────────

    #[test]
    fn test_inmemory_memorize_and_recall() {
        let mut store = InMemoryStore::new();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "the cat sat on the mat".to_string(),
                priority: 1.0,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();

        let result = store.recall("the cat sat", &[], 0.3);
        assert!(result.is_some());
        let (entry, score) = result.unwrap();
        assert!(
            entry.value.contains("cat sat"),
            "Should find cat sat, got: {}",
            entry.value
        );
        assert!(score >= 0.3, "Score should exceed threshold");
    }

    #[test]
    fn test_inmemory_recall_empty() {
        let store = InMemoryStore::new();
        let result = store.recall("anything", &[], 0.3);
        assert!(result.is_none());
    }

    #[test]
    fn test_inmemory_forget() {
        let mut store = InMemoryStore::new();
        let old_ts = now() - 100000; // old entry
        store
            .memorize(MemoryEntry {
                id: None,
                value: "old fact".to_string(),
                priority: 1.0,
                timestamp: old_ts,
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();

        assert_eq!(store.count(), 1);
        store.forget("old fact", now()); // cutoff = now, old_ts < now
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_inmemory_decay() {
        let mut store = InMemoryStore::new();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "test".to_string(),
                priority: 1.0,
                timestamp: now() - 86400, // 1 day ago
                decay_rate: 0.1,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();

        let count = store.decay();
        assert!(count > 0, "At least one entry should be decayed");

        let entry = &store.all_entries()[0];
        assert!(
            entry.priority < 1.0,
            "Priority should decrease after decay, got {}",
            entry.priority
        );
    }

    #[test]
    fn test_inmemory_count() {
        let mut store = InMemoryStore::new();
        assert_eq!(store.count(), 0);
        store
            .memorize(MemoryEntry {
                id: None,
                value: "a".to_string(),
                priority: 1.0,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "b".to_string(),
                priority: 1.0,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();
        assert_eq!(store.count(), 2);
    }

    // ── InMemoryKg ──────────────────────────────────────────────────

    #[test]
    fn test_inmemory_kg_relate_and_edges() {
        let mut kg = InMemoryKg::new();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();

        let edges = kg.edges_for("alice");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, "coworker");
        assert_eq!(edges[0].1, "bob");
    }

    #[test]
    fn test_inmemory_kg_bidirectional() {
        let mut kg = InMemoryKg::new();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();

        let edges_bob = kg.edges_for("bob");
        assert_eq!(edges_bob.len(), 1);
        assert_eq!(edges_bob[0].1, "alice");
    }

    #[test]
    fn test_inmemory_kg_walk() {
        let mut kg = InMemoryKg::new();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();
        kg.relate("bob", "charlie", "friend", 0.8).unwrap();

        let walk = kg.walk("alice", 3);
        assert!(walk.iter().any(|(_, v, _)| v == "bob"), "Should reach bob");
        assert!(
            walk.iter().any(|(_, v, _)| v == "charlie"),
            "Should reach charlie"
        );
    }

    #[test]
    fn test_inmemory_kg_walk_depth_limit() {
        let mut kg = InMemoryKg::new();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();
        kg.relate("bob", "charlie", "friend", 0.8).unwrap();

        let walk = kg.walk("alice", 1); // depth 1 — only direct neighbors
        assert!(walk.iter().any(|(_, v, _)| v == "bob"));
        assert!(
            !walk.iter().any(|(_, v, _)| v == "charlie"),
            "Should NOT reach charlie at depth 1"
        );
    }

    #[test]
    fn test_inmemory_kg_edge_count() {
        let mut kg = InMemoryKg::new();
        assert_eq!(kg.edge_count(), 0);
        kg.relate("a", "b", "rel", 1.0).unwrap();
        kg.relate("b", "c", "rel2", 1.0).unwrap();
        assert_eq!(kg.edge_count(), 2);
    }

    // ── SqliteStore ─────────────────────────────────────────────────

    #[test]
    fn test_sqlite_memorize_and_recall() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        let mut store = SqliteStore::open(&path).unwrap();

        store
            .memorize(MemoryEntry {
                id: None,
                value: "hello world from sqlite".to_string(),
                priority: 1.0,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();

        assert_eq!(store.count(), 1);

        let result = store.recall("hello world", &[], 0.3);
        assert!(result.is_some());
        let (entry, _score) = result.unwrap();
        assert_eq!(entry.value, "hello world from sqlite");
    }

    #[test]
    fn test_sqlite_recall_empty() {
        let tmp = NamedTempFile::new().unwrap();
        let store = SqliteStore::open(tmp.path()).unwrap();
        let result = store.recall("anything", &[], 0.3);
        assert!(result.is_none());
    }

    #[test]
    fn test_sqlite_forget() {
        let tmp = NamedTempFile::new().unwrap();
        let mut store = SqliteStore::open(tmp.path()).unwrap();

        let old_ts = now() - 100000;
        store
            .memorize(MemoryEntry {
                id: None,
                value: "old fact".to_string(),
                priority: 1.0,
                timestamp: old_ts,
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "new fact".to_string(),
                priority: 1.0,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();

        assert_eq!(store.count(), 2);
        store.forget("old fact", now());
        assert_eq!(store.count(), 1);

        let entries = store.all_entries();
        assert_eq!(entries[0].value, "new fact");
    }

    #[test]
    fn test_sqlite_decay() {
        let tmp = NamedTempFile::new().unwrap();
        let mut store = SqliteStore::open(tmp.path()).unwrap();

        store
            .memorize(MemoryEntry {
                id: None,
                value: "decaying fact".to_string(),
                priority: 1.0,
                timestamp: now() - 86400,
                decay_rate: 0.1,
                confidence: 1.0,
                embedding: Vec::new(),
                mem_type: String::new(),
            })
            .unwrap();

        let count = store.decay();
        assert!(count > 0);

        let entries = store.all_entries();
        assert!(entries[0].priority < 1.0);
    }

    #[test]
    fn test_sqlite_embedding_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let mut store = SqliteStore::open(tmp.path()).unwrap();

        let embedding = vec![0.1_f32, 0.2, 0.3, 0.4, 0.5];
        store
            .memorize(MemoryEntry {
                id: None,
                value: "embedded fact".to_string(),
                priority: 1.0,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: embedding.clone(),
                mem_type: String::new(),
            })
            .unwrap();

        let entries = store.all_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].embedding, embedding);
    }

    #[test]
    fn test_sqlite_persistence_across_reopen() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // First session: memorize
        {
            let mut store = SqliteStore::open(&path).unwrap();
            store
                .memorize(MemoryEntry {
                    id: None,
                    value: "persistent fact".to_string(),
                    priority: 0.9,
                    timestamp: now(),
                    decay_rate: 0.01,
                    confidence: 0.9,
                    embedding: Vec::new(),
                    mem_type: String::new(),
                })
                .unwrap();
            assert_eq!(store.count(), 1);
        }

        // Second session (simulates restart): recall
        {
            let store = SqliteStore::open(&path).unwrap();
            assert_eq!(store.count(), 1, "Memory should persist across reopen");
            let result = store.recall("persistent fact", &[], 0.3);
            assert!(
                result.is_some(),
                "Should recall persisted fact after reopen"
            );
            let (entry, _) = result.unwrap();
            assert_eq!(entry.value, "persistent fact");
        }
    }

    // ── SqliteKg ────────────────────────────────────────────────────

    #[test]
    fn test_sqlite_kg_relate_and_recall() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let mut kg = SqliteKg::open(&path).unwrap();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();

        assert_eq!(kg.edge_count(), 1);
        let edges = kg.edges_for("alice");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].1, "bob");
    }

    #[test]
    fn test_sqlite_kg_persistence_across_reopen() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // First session: relate
        {
            let mut kg = SqliteKg::open(&path).unwrap();
            kg.relate("node_a", "node_b", "related", 0.9).unwrap();
            assert_eq!(kg.edge_count(), 1);
        }

        // Second session: recall relation
        {
            let kg = SqliteKg::open(&path).unwrap();
            assert_eq!(kg.edge_count(), 1, "KG should persist across reopen");
            let edges = kg.edges_for("node_a");
            assert_eq!(edges.len(), 1);
            assert_eq!(edges[0].0, "related");
            assert_eq!(edges[0].1, "node_b");
        }
    }

    #[test]
    fn test_sqlite_kg_walk() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let mut kg = SqliteKg::open(&path).unwrap();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();
        kg.relate("bob", "charlie", "friend", 0.8).unwrap();

        let walk = kg.walk("alice", 3);
        assert!(walk.iter().any(|(_, v, _)| v == "bob"));
        assert!(walk.iter().any(|(_, v, _)| v == "charlie"));
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}
