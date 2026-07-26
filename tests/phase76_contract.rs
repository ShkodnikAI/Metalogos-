// ── Phase 7.6 Contract Tests: Memory Persistence ──────────────────────
// Contracts:
//   C1: SQLite memorize + recall roundtrip
//   C2: Persistence across restart (close + reopen DB)
//   C3: In-memory default (no persist → backward compatible)
//   C4: Decay formula: activation = priority * exp(-rate * age)
//   C5: Forget removes matching entries
//   C6: KG persist + walk across restart
//   C7: Embedding roundtrip via BLOB
//   C8: No persist → data lost on restart

use metalogos::memory_store::*;

// ── C1: SQLite memorize + recall ──────────────────────────────────────

#[test]
fn test_76_sqlite_memorize_and_recall() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_memory.db");
    let mut store = SqliteStore::open(&path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    store
        .memorize(MemoryEntry {
            id: None,
            value: "the cat sat on the mat".to_string(),
            priority: 1.0,
            timestamp: now,
            decay_rate: 0.01,
            confidence: 1.0,
            embedding: Vec::new(),
        })
        .unwrap();

    assert_eq!(store.count(), 1, "C1: count should be 1 after memorize");

    let result = store.recall("cat sat", &[], 0.3);
    assert!(result.is_some(), "C1: should recall stored fact");
    let (entry, score) = result.unwrap();
    assert_eq!(
        entry.value, "the cat sat on the mat",
        "C1: exact value match"
    );
    assert!(
        score >= 0.3,
        "C1: score should exceed min_confidence, got {}",
        score
    );
}

// ── C2: Persistence across restart ───────────────────────────────────

#[test]
fn test_76_persistence_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("persist_test.db");

    let now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    };

    // Session 1: memorize
    {
        let mut store = SqliteStore::open(&path).unwrap();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "persistent fact about the universe".to_string(),
                priority: 0.9,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 0.9,
                embedding: Vec::new(),
            })
            .unwrap();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "another fact that should survive".to_string(),
                priority: 0.8,
                timestamp: now(),
                decay_rate: 0.01,
                confidence: 0.8,
                embedding: Vec::new(),
            })
            .unwrap();
        assert_eq!(store.count(), 2);
    } // store dropped — connection closed

    // Session 2: reopen and recall (simulates process restart)
    {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.count(), 2, "C2: both memories should survive restart");

        let result = store.recall("persistent fact", &[], 0.3);
        assert!(result.is_some(), "C2: should recall persisted fact");
        let (entry, _) = result.unwrap();
        assert_eq!(entry.value, "persistent fact about the universe");

        let result2 = store.recall("another fact", &[], 0.3);
        assert!(result2.is_some(), "C2: should recall second persisted fact");
        let (entry2, _) = result2.unwrap();
        assert_eq!(entry2.value, "another fact that should survive");
    }
}

// ── C3: In-memory default (backward compatible) ───────────────────────

#[test]
fn test_76_inmemory_default_no_persist() {
    let mut store = InMemoryStore::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    store
        .memorize(MemoryEntry {
            id: None,
            value: "in-memory fact".to_string(),
            priority: 1.0,
            timestamp: now,
            decay_rate: 0.01,
            confidence: 1.0,
            embedding: Vec::new(),
        })
        .unwrap();

    assert_eq!(store.count(), 1, "C3: in-memory count should be 1");

    let result = store.recall("in-memory", &[], 0.3);
    assert!(result.is_some(), "C3: in-memory recall should work");
    let (entry, _) = result.unwrap();
    assert_eq!(entry.value, "in-memory fact");
}

// ── C4: Decay formula correctness ─────────────────────────────────────

#[test]
fn test_76_decay_formula() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("decay_test.db");
    let mut store = SqliteStore::open(&path).unwrap();

    // Memorize with timestamp 2 days ago, decay_rate = 0.1
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    store
        .memorize(MemoryEntry {
            id: None,
            value: "decaying entry".to_string(),
            priority: 1.0,
            timestamp: now - 2 * 86400, // 2 days ago
            decay_rate: 0.1,
            confidence: 1.0,
            embedding: Vec::new(),
        })
        .unwrap();

    let count = store.decay();
    assert_eq!(count, 1, "C4: exactly one entry should be decayed");

    let entries = store.all_entries();
    assert_eq!(entries.len(), 1);

    // Expected: priority = 1.0 * exp(-0.1 * 2) = exp(-0.2) ≈ 0.8187
    let expected = 1.0 * (-0.1 * 2.0_f64).exp();
    let actual = entries[0].priority;
    assert!(
        (actual - expected).abs() < 0.01,
        "C4: decayed priority should be ~exp(-0.2) ≈ {:.4}, got {:.4}",
        expected,
        actual
    );
}

// ── C5: Forget removes matching entries ──────────────────────────────

#[test]
fn test_76_forget_removes_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forget_test.db");
    let mut store = SqliteStore::open(&path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Old entry (should be forgotten)
    store
        .memorize(MemoryEntry {
            id: None,
            value: "old fact to forget".to_string(),
            priority: 1.0,
            timestamp: now - 200000,
            decay_rate: 0.01,
            confidence: 1.0,
            embedding: Vec::new(),
        })
        .unwrap();

    // New entry (should NOT be forgotten)
    store
        .memorize(MemoryEntry {
            id: None,
            value: "new fact to forget".to_string(),
            priority: 1.0,
            timestamp: now,
            decay_rate: 0.01,
            confidence: 1.0,
            embedding: Vec::new(),
        })
        .unwrap();

    assert_eq!(store.count(), 2);
    store.forget("to forget", now); // cutoff = now, old entry has timestamp < now
    assert_eq!(store.count(), 1, "C5: only old entry should be removed");

    let entries = store.all_entries();
    assert_eq!(
        entries[0].value, "new fact to forget",
        "C5: new entry survives"
    );
}

// ── C6: KG persist + walk across restart ──────────────────────────────

#[test]
fn test_76_kg_persistence_and_walk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kg_test.db");

    // Session 1: create relations
    {
        let mut kg = SqliteKg::open(&path).unwrap();
        kg.relate("alice", "bob", "coworker", 1.0).unwrap();
        kg.relate("bob", "charlie", "friend", 0.8).unwrap();
        kg.relate("charlie", "diana", "mentor", 0.9).unwrap();
        assert_eq!(kg.edge_count(), 3);
    }

    // Session 2: walk should reach transitive nodes
    {
        let kg = SqliteKg::open(&path).unwrap();
        assert_eq!(
            kg.edge_count(),
            3,
            "C6: edges should persist across restart"
        );

        let edges = kg.edges_for("bob");
        assert_eq!(
            edges.len(),
            2,
            "C6: bob should have 2 connections (alice + charlie)"
        );

        let walk = kg.walk("alice", 5);
        assert!(
            walk.iter().any(|(_, v, _)| v == "bob"),
            "C6: walk reaches bob"
        );
        assert!(
            walk.iter().any(|(_, v, _)| v == "charlie"),
            "C6: walk reaches charlie"
        );
        assert!(
            walk.iter().any(|(_, v, _)| v == "diana"),
            "C6: walk reaches diana"
        );
    }
}

// ── C7: Embedding roundtrip via BLOB ──────────────────────────────────

#[test]
fn test_76_embedding_blob_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("embed_test.db");
    let mut store = SqliteStore::open(&path).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let embedding = vec![0.15_f32, -0.23, 0.87, 0.01, -0.55, 0.42];

    store
        .memorize(MemoryEntry {
            id: None,
            value: "embedded fact".to_string(),
            priority: 1.0,
            timestamp: now,
            decay_rate: 0.01,
            confidence: 1.0,
            embedding: embedding.clone(),
        })
        .unwrap();

    // Reopen to verify BLOB persistence
    let store = SqliteStore::open(&path).unwrap();
    let entries = store.all_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].embedding, embedding,
        "C7: embedding should roundtrip through BLOB"
    );
    assert_eq!(entries[0].embedding.len(), 6);
}

// ── C8: No persist → data lost on restart ─────────────────────────────

#[test]
fn test_76_no_persist_data_lost() {
    // InMemoryStore has no persistence — data is lost when store is dropped
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Session 1
    {
        let mut store = InMemoryStore::new();
        store
            .memorize(MemoryEntry {
                id: None,
                value: "ephemeral data".to_string(),
                priority: 1.0,
                timestamp: now,
                decay_rate: 0.01,
                confidence: 1.0,
                embedding: Vec::new(),
            })
            .unwrap();
        assert_eq!(store.count(), 1);
    } // store dropped — data gone

    // Session 2: new store has no data
    let store = InMemoryStore::new();
    assert_eq!(
        store.count(),
        0,
        "C8: in-memory store should be empty after 'restart'"
    );
}
