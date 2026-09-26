//! №466 (gh#687) — the memory transfer group: the shared live module.
//!
//! The first group of the TW/VM dedup (gate gh#680, decision 4-A, step 3;
//! the CI threshold gate is №462/gh#683, the diff-fuzzer is №465/gh#686).
//! The four memory builtin names — `memorize`, `recall`, `forget`,
//! `recall_top_k` — moved OUT of both backends: `src/vm.rs` and
//! `src/interpreter/execution.rs` keep a single marshaling hook each and
//! the bodies live here. After the move the name literals appear only in
//! this module, so the №462 counter drops 60 → 56.
//!
//! The module is the shared HOME, not a unification: the TW and the VM
//! engines stay deliberately separate (the №442 "honest simple-memory
//! twin" posture — each backend reads its own store). A transfer PR must
//! not change semantics: the crosscheck and the №465 diff-fuzzer pin the
//! behavior, and the divergences the fuzzer found are fixed by separate
//! owner-gated naryads, never silently inside a transfer. The genuinely
//! shared pieces factored here once are the argument-parsing shapes, the
//! timestamp helper, and the JSON result shape of `recall_top_k`; the
//! store algorithms and the exact per-backend error texts stay per
//! backend exactly as they were.

use crate::bytecode::{VmMemoryEntry, VmRelation};
use crate::embeddings::EmbeddingManager;
use crate::interpreter::Value;
use crate::memory_store::{KgStore, MemoryEntry, MemoryStore};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The memory-group names this module owns. Both backends ask this
/// single hook before their own dispatch paths — the names are spelled
/// here and nowhere else outside the registry.
pub fn handles(name: &str) -> bool {
    matches!(name, "memorize" | "recall" | "forget" | "recall_top_k")
}

/// Unix seconds — both backends computed this identically at every
/// memory call site; factored once.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ─────────────────────────────────────────────────────────────────────
// The TW side (the interpreter store lane: FTS5/cosine hybrid engine,
// the typed-lane consent gate, the knowledge-graph walk, embeddings).
// ─────────────────────────────────────────────────────────────────────

/// TW `recall` — moved verbatim from `interpreter::memory::invoke_recall`
/// (the hybrid store lane, the typed-lane fallback, the graph walk, the
/// №442 ledger records; the fail-closed consent gate stays FIRST — a
/// refusal discloses nothing, not even partial results).
pub(crate) fn recall_tw(
    memory: &Mutex<Box<dyn MemoryStore>>,
    kg: &Mutex<Box<dyn KgStore>>,
    embedding_manager: &EmbeddingManager,
    args: &[Value],
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("recall() requires at least 1 argument (query string)".to_string());
    }

    let query = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "recall() expected String argument, got {}",
                other.type_name()
            ))
        }
    };

    let min_confidence = if args.len() > 1 {
        args[1].as_float().unwrap_or(0.3) as f32
    } else {
        0.3
    };

    // ── №442: the typed lane — the fail-closed consent gate comes
    // FIRST (a refusal discloses nothing, not even partial results).
    let lane = crate::memory_typed::recall_lane(&query);
    if let Some((container_id, subject)) = lane.gated_key_matches.first() {
        crate::memory_typed::ledger_recall_denied(&query, container_id);
        return Err(crate::memory_typed::recall_consent_refusal(
            &query,
            container_id,
            subject,
        ));
    }

    // Embed the query for semantic search
    let query_embedding = embedding_manager.embed(&query).unwrap_or_default();

    // ── The store lane, through the hybrid engines (№442). RRF
    // scores are rank-based and not comparable with the 0..1
    // confidence threshold, so the threshold keeps the store's
    // activation semantics (sim × priority × decay — the exact
    // scoring `recall()` has always applied) and is re-checked per
    // candidate; the old full-scan recall() stays as the safety net
    // for candidates the BM25 candidate set missed.
    let store_hit = {
        let mem = crate::interpreter::lock_or_err(memory.lock())?;
        let now = now_secs();
        let hybrid = mem.recall_top_k(&query, &query_embedding, 0.0, 5, "");
        let from_hybrid = hybrid
            .into_iter()
            .map(|(entry, _rrf)| {
                let sim = if !query_embedding.is_empty() && !entry.embedding.is_empty() {
                    crate::embeddings::cosine_similarity(&query_embedding, &entry.embedding)
                } else if entry.value.contains(&query) {
                    1.0
                } else {
                    0.0
                };
                let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
                let decay = (-entry.decay_rate * age_days).exp() as f32;
                let signal = sim * (entry.priority as f32) * decay;
                (entry, signal)
            })
            .find(|(_, signal)| *signal >= min_confidence);
        match from_hybrid {
            Some((entry, _)) => Some(entry),
            None => mem
                .recall(&query, &query_embedding, min_confidence)
                .map(|(entry, _)| entry),
        }
    };

    // ── Merge: the store lane is recall's primary lane (its
    // external contract is regression-pinned); the typed lane is the
    // fallback source whose hits carry the [MEM] provenance suffix.
    let had_store_hit = store_hit.is_some();
    let result = match store_hit {
        Some(entry) => {
            // Walk the knowledge graph for related memories
            let edges = crate::interpreter::lock_or_err(kg.lock())?.edges_for(&entry.value);
            if edges.is_empty() {
                entry.value.clone()
            } else {
                let mut result = entry.value.clone();
                for (relation, other, _weight) in &edges {
                    result.push('\n');
                    result.push_str(&format!("[GRAPH] {} -> {}", relation, other));
                }
                result
            }
        }
        None => match lane.hits.first() {
            Some(hit) => {
                let mut result = hit.text.clone();
                result.push_str(&crate::memory_typed::recall_hit_provenance(hit));
                result
            }
            None => String::new(),
        },
    };

    // ── №442: the ledger record — every recall call is audited
    // {query hash, containers, hits, consent fact} (№393/ADR-0167).
    let disclosed = if had_store_hit { 1 } else { lane.hits.len() };
    crate::memory_typed::ledger_recall(&query, &lane, disclosed);

    Ok(Value::String(result))
}

/// TW `memorize` — moved verbatim from `interpreter::memory::invoke_memorize_fn`
/// (the callable form; the declaration form calls this too). Silent on
/// store errors — no stdout leak in HTTP context (Bug 2.3).
pub(crate) fn memorize_tw(
    memory: &Mutex<Box<dyn MemoryStore>>,
    embedding_manager: &EmbeddingManager,
    args: &[Value],
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("memorize() requires at least 1 argument (text)".to_string());
    }
    let value_str = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "memorize() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let priority = if args.len() > 1 {
        args[1].as_float().unwrap_or(1.0)
    } else {
        1.0
    };
    let mem_type = if args.len() > 2 {
        match &args[2] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        }
    } else {
        String::new()
    };
    let now = now_secs();
    let embedding = embedding_manager.embed(&value_str).unwrap_or_default();
    match crate::interpreter::lock_or_err(memory.lock())?.memorize(MemoryEntry {
        id: None,
        value: value_str.clone(),
        priority,
        timestamp: now,
        decay_rate: 0.01,
        confidence: priority,
        embedding,
        mem_type,
    }) {
        Ok(_id) => { /* Bug 2.3 fix: removed eprintln stdout leak in HTTP context */ }
        Err(_) => { /* silent — don't leak to stdout in HTTP context */ }
    }
    Ok(Value::Unit)
}

/// TW `forget` — moved verbatim from `interpreter::memory::invoke_forget_fn`
/// (the legacy 1..2-argument surface only; 3..4 arguments are the canon
/// §10.3 typed front door and fall through to the registry handler).
pub(crate) fn forget_tw(
    memory: &Mutex<Box<dyn MemoryStore>>,
    args: &[Value],
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("forget() requires at least 1 argument (query)".to_string());
    }
    let query_str = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "forget() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let days = if args.len() > 1 {
        args[1].as_float().unwrap_or(30.0) as i64
    } else {
        30
    };
    let now = now_secs();
    let cutoff = now - (days * 86400);
    crate::interpreter::lock_or_err(memory.lock())?.forget(&query_str, cutoff);
    Ok(Value::Unit)
}

/// TW `recall_top_k` — moved verbatim from
/// `interpreter::memory::invoke_recall_top_k_fn` (hybrid FTS5 BM25 +
/// cosine RRF search over the interpreter store; the JSON result shape
/// is the shared contract with the VM twin).
pub(crate) fn recall_top_k_tw(
    memory: &Mutex<Box<dyn MemoryStore>>,
    embedding_manager: &EmbeddingManager,
    args: &[Value],
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("recall_top_k() requires at least 1 argument (query)".to_string());
    }
    let query = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "recall_top_k() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let k = if args.len() > 1 {
        args[1].as_float().unwrap_or(5.0) as usize
    } else {
        5
    };
    let type_filter = if args.len() > 2 {
        match &args[2] {
            Value::String(s) => s.clone(),
            Value::Unit => String::new(),
            other => format!("{}", other),
        }
    } else {
        String::new()
    };
    let query_embedding = embedding_manager.embed(&query).unwrap_or_default();
    let mem = crate::interpreter::lock_or_err(memory.lock())?;
    let results = mem.recall_top_k(&query, &query_embedding, 0.0, k, &type_filter);
    let json_results: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(entry, score)| {
            serde_json::json!({
                "value": entry.value,
                "score": score,
                "type": entry.mem_type,
                "priority": entry.priority,
            })
        })
        .collect();
    Ok(Value::String(
        serde_json::to_string(&json_results).unwrap_or_default(),
    ))
}

/// The TW marshaling hook: called from BOTH interpreter dispatch routes
/// (the pattern/route step path and the flow-step expression path —
/// they previously carried two identical copies of the four intercepts).
/// `None` means "not handled here" — the caller falls through to the
/// registry handlers (the 3..4-argument `forget` typed front door).
pub fn dispatch_tw(
    name: &str,
    memory: &Mutex<Box<dyn MemoryStore>>,
    kg: &Mutex<Box<dyn KgStore>>,
    embedding_manager: &EmbeddingManager,
    args: &[Value],
) -> Option<Result<Value, String>> {
    match name {
        "recall" => Some(recall_tw(memory, kg, embedding_manager, args)),
        "memorize" => Some(memorize_tw(memory, embedding_manager, args)),
        "recall_top_k" => Some(recall_top_k_tw(memory, embedding_manager, args)),
        "forget" if args.len() <= 2 => Some(forget_tw(memory, args)),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────
// The VM side (the simple in-process store twin: substring + activation
// recall, token-level AND top-k, case-insensitive forget — the №442
// "honest simple-memory twin" posture; the algorithms moved, not
// unified). The VM state is reached through the accessor trait below —
// the `Vm` fields stay private to the vm module.
// ─────────────────────────────────────────────────────────────────────

/// The live contract the VM hands to this module: read/mutate access to
/// the simple-memory store and the VM-side knowledge-graph relations.
/// (The dead `RuntimeContext` stub was deleted by №465 — this trait is
/// the replacing live contract the №466 naryad calls for.)
pub trait VmMemoryAccess {
    /// The store entries (read lane).
    fn mem(&self) -> &[VmMemoryEntry];
    /// The store entries (write lane).
    fn mem_mut(&mut self) -> &mut Vec<VmMemoryEntry>;
    /// The VM-side knowledge-graph relations (the `[GRAPH]` walk).
    fn relations(&self) -> &[VmRelation];
    /// The store-lane recall (substring + activation — the twin of the
    /// TW hybrid lane). A default over the accessors so the engine
    /// below needs no other VM coupling.
    fn recall_lane(&self, query: &str, min_confidence: f64) -> String {
        store_recall_vm(self.mem(), self.relations(), query, min_confidence)
    }
}

/// The VM store-lane recall — moved verbatim from `Vm::recall` (which now
/// delegates here; its other two call sites are unchanged). Substring
/// match + activation (priority × exponential decay), best entry wins;
/// the `[GRAPH]` walk decorates the hit.
pub fn store_recall_vm(
    entries: &[VmMemoryEntry],
    relations: &[VmRelation],
    query: &str,
    min_confidence: f64,
) -> String {
    let now = now_secs();

    let mut best_match: Option<&VmMemoryEntry> = None;
    let mut best_activation: f64 = 0.0;

    for entry in entries {
        if !entry.value.contains(query) {
            continue;
        }
        let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
        let activation = entry.priority * (-entry.decay_rate * age_days).exp();
        if activation > best_activation && activation >= min_confidence {
            best_activation = activation;
            best_match = Some(entry);
        }
    }

    match best_match {
        Some(entry) => {
            let mut result = entry.value.clone();
            // Walk knowledge graph for related memories
            for rel in relations {
                if rel.from == entry.value {
                    result.push('\n');
                    result.push_str(&format!("[GRAPH] {} -> {}", rel.relation, rel.to));
                } else if rel.to == entry.value {
                    result.push('\n');
                    result.push_str(&format!("[GRAPH] {} -> {}", rel.relation, rel.from));
                }
            }
            result
        }
        None => String::new(),
    }
}

/// VM `recall` — moved verbatim from the vm.rs dispatch block. The VM
/// defaults `min_conf` to 0.0 (not the TW's 0.3) and its parse error
/// text is the `{:?}` form — both preserved exactly.
fn recall_vm(vm: &dyn VmMemoryAccess, args: &[Value]) -> Result<Value, String> {
    let query = match args.first() {
        Some(Value::String(s)) => s.clone(),
        other => return Err(format!("recall() expected String, got {:?}", other)),
    };
    let min_conf = if args.len() > 1 {
        args[1].as_float().unwrap_or(0.0)
    } else {
        0.0
    };
    // The typed lane's fail-closed gate comes FIRST.
    let lane = crate::memory_typed::recall_lane(&query);
    if let Some((container_id, subject)) = lane.gated_key_matches.first() {
        crate::memory_typed::ledger_recall_denied(&query, container_id);
        return Err(crate::memory_typed::recall_consent_refusal(
            &query,
            container_id,
            subject,
        ));
    }
    // Store lane (VM-native twin) → typed-lane fallback.
    let store_result = vm.recall_lane(&query, min_conf);
    let had_store_hit = !store_result.is_empty();
    let result = if had_store_hit {
        store_result
    } else {
        match lane.hits.first() {
            Some(hit) => {
                let mut r = hit.text.clone();
                r.push_str(&crate::memory_typed::recall_hit_provenance(hit));
                r
            }
            None => String::new(),
        }
    };
    let disclosed = if had_store_hit { 1 } else { lane.hits.len() };
    crate::memory_typed::ledger_recall(&query, &lane, disclosed);
    Ok(Value::String(result))
}

/// VM `memorize` — moved verbatim from the vm.rs dispatch block (№72
/// parity with the interpreter callable form; no embedding on the VM
/// side — the simple store keeps no vectors).
fn memorize_vm(vm: &mut dyn VmMemoryAccess, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("memorize() requires at least 1 argument (text)".to_string());
    }
    let value_str = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "memorize() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let priority = if args.len() > 1 {
        args[1].as_float().unwrap_or(1.0)
    } else {
        1.0
    };
    let mem_type = if args.len() > 2 {
        match &args[2] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        }
    } else {
        String::new()
    };
    let now = now_secs();
    vm.mem_mut().push(VmMemoryEntry {
        value: value_str,
        priority,
        timestamp: now,
        decay_rate: 0.01,
        mem_type,
    });
    Ok(Value::Unit)
}

/// VM `forget` — moved verbatim from the vm.rs dispatch block (the
/// legacy 1..2-argument surface; the VM lowercases the query — the TW
/// keeps case; the case difference is the pinned posture, not a bug to
/// fix here).
fn forget_vm(vm: &mut dyn VmMemoryAccess, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("forget() requires at least 1 argument (query)".to_string());
    }
    let query_str = match &args[0] {
        Value::String(s) => s.to_lowercase(),
        other => {
            return Err(format!(
                "forget() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let days = if args.len() > 1 {
        args[1].as_float().unwrap_or(30.0) as i64
    } else {
        30
    };
    let now = now_secs();
    let cutoff = now - (days * 86400);
    vm.mem_mut()
        .retain(|m| !(m.value.to_lowercase().contains(&query_str) && m.timestamp < cutoff));
    Ok(Value::Unit)
}

/// VM `recall_top_k` — moved verbatim from the vm.rs dispatch block
/// (Bug #530 parity): token-level AND over the lowercased value with a
/// matched-words score weighted by priority; zero-hit entries STAY with
/// score 0.0 (the contract is "top-k by score", not "only hits").
fn recall_top_k_vm(vm: &dyn VmMemoryAccess, args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("recall_top_k() requires at least 1 argument (query)".to_string());
    }
    let query = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "recall_top_k() expected String as first arg, got {}",
                other.type_name()
            ))
        }
    };
    let k = if args.len() > 1 {
        args[1].as_float().unwrap_or(5.0) as usize
    } else {
        5
    };
    let type_filter = if args.len() > 2 {
        match &args[2] {
            Value::String(s) => s.clone(),
            Value::Unit => String::new(),
            other => format!("{}", other),
        }
    } else {
        String::new()
    };
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let mut scored: Vec<(f64, &VmMemoryEntry)> = Vec::new();
    for entry in vm.mem() {
        if !type_filter.is_empty() && entry.mem_type != type_filter {
            continue;
        }
        // Zero-hit entries STAY (score 0.0) — the TW hybrid returns
        // top-k over the whole store, weak matches included (the
        // contract is "top-k by score", not "only hits").
        let val_lower = entry.value.to_lowercase();
        let hits = query_words
            .iter()
            .filter(|w| val_lower.contains(*w))
            .count() as f64;
        let score = (hits / query_words.len() as f64) * (1.0 + entry.priority);
        scored.push((score, entry));
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let json_results: Vec<serde_json::Value> = scored
        .into_iter()
        .take(k)
        .map(|(score, entry)| {
            serde_json::json!({
                "value": entry.value,
                "score": score,
                "type": entry.mem_type,
                "priority": entry.priority,
            })
        })
        .collect();
    Ok(Value::String(
        serde_json::to_string(&json_results).unwrap_or_default(),
    ))
}

/// The VM marshaling hook: called once from `Vm::call_builtin`. `None`
/// means "not handled here" — the caller falls through (the 3..4-argument
/// `forget` typed front door lives in the registry, as on the TW).
pub fn dispatch_vm(
    name: &str,
    vm: &mut dyn VmMemoryAccess,
    args: &[Value],
) -> Option<Result<Value, String>> {
    match name {
        "recall" => Some(recall_vm(vm, args)),
        "memorize" => Some(memorize_vm(vm, args)),
        "recall_top_k" => Some(recall_top_k_vm(vm, args)),
        "forget" if args.len() <= 2 => Some(forget_vm(vm, args)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::EmbeddingManager;
    use crate::memory_store::{InMemoryKg, InMemoryStore};

    /// A minimal VM memory state — exercises the live contract without a
    /// full `Vm` (the trait is the contract; `Vm` is one implementor).
    struct MockVm {
        memory: Vec<VmMemoryEntry>,
        relations: Vec<VmRelation>,
    }
    impl MockVm {
        fn new() -> Self {
            Self {
                memory: Vec::new(),
                relations: Vec::new(),
            }
        }
    }
    impl VmMemoryAccess for MockVm {
        fn mem(&self) -> &[VmMemoryEntry] {
            &self.memory
        }
        fn mem_mut(&mut self) -> &mut Vec<VmMemoryEntry> {
            &mut self.memory
        }
        fn relations(&self) -> &[VmRelation] {
            &self.relations
        }
    }

    #[test]
    fn handles_exactly_the_memory_group() {
        for name in ["memorize", "recall", "forget", "recall_top_k"] {
            assert!(handles(name), "handles('{name}') must be true");
        }
        for name in ["", "find", "query", "db_insert", "memory_forget", "memoriz"] {
            assert!(!handles(name), "handles('{name}') must be false");
        }
    }

    #[test]
    fn dispatch_falls_through_outside_the_group() {
        let mut vm = MockVm::new();
        let store = Mutex::new(Box::new(InMemoryStore::new()) as Box<dyn MemoryStore>);
        let kg = Mutex::new(Box::new(InMemoryKg::new()) as Box<dyn KgStore>);
        let em = EmbeddingManager::new();

        // A non-group name is not handled by either backend hook.
        assert!(dispatch_vm("find", &mut vm, &[Value::Unit]).is_none());
        assert!(dispatch_tw("find", &store, &kg, &em, &[Value::Unit]).is_none());

        // The 3..4-argument forget falls through to the registry's
        // §10.3 typed front door on both backends.
        let four = [
            Value::String("a".into()),
            Value::Float(1.0),
            Value::String("b".into()),
            Value::String("c".into()),
        ];
        assert!(dispatch_vm("forget", &mut vm, &four).is_none());
        assert!(dispatch_tw("forget", &store, &kg, &em, &four).is_none());
    }

    #[test]
    fn vm_memorize_parse_defaults_and_errors() {
        let mut vm = MockVm::new();

        // Defaults: priority 1.0, empty type, decay 0.01.
        dispatch_vm("memorize", &mut vm, &[Value::String("fact one".into())])
            .unwrap()
            .unwrap();
        assert_eq!(vm.memory.len(), 1);
        assert_eq!(vm.memory[0].value, "fact one");
        assert_eq!(vm.memory[0].priority, 1.0);
        assert_eq!(vm.memory[0].mem_type, "");
        assert_eq!(vm.memory[0].decay_rate, 0.01);

        // Explicit priority and type.
        dispatch_vm(
            "memorize",
            &mut vm,
            &[
                Value::String("fact two".into()),
                Value::Float(0.7),
                Value::String("persona".into()),
            ],
        )
        .unwrap()
        .unwrap();
        assert_eq!(vm.memory[1].priority, 0.7);
        assert_eq!(vm.memory[1].mem_type, "persona");

        // Non-String first arg — the exact error text.
        let err = dispatch_vm("memorize", &mut vm, &[Value::Float(1.0)])
            .unwrap()
            .unwrap_err();
        assert_eq!(err, "memorize() expected String as first arg, got Float");
        // Empty args — the exact error text.
        let err = dispatch_vm("memorize", &mut vm, &[]).unwrap().unwrap_err();
        assert_eq!(err, "memorize() requires at least 1 argument (text)");
    }

    #[test]
    fn vm_recall_lane_and_defaults() {
        let mut vm = MockVm::new();
        dispatch_vm("memorize", &mut vm, &[Value::String("spicy food".into())])
            .unwrap()
            .unwrap();

        // recall via the store lane (min_conf default 0.0 on the VM).
        let out = dispatch_vm("recall", &mut vm, &[Value::String("spicy".into())])
            .unwrap()
            .unwrap();
        match out {
            Value::String(s) => assert!(s.contains("spicy food"), "got: {s}"),
            other => panic!("expected String, got {other:?}"),
        }

        // Parse errors — the exact VM texts (the {:?} form differs from TW;
        // empty args hit the same catch-all with None — the VM has no
        // separate empty-args message on recall).
        let err = dispatch_vm("recall", &mut vm, &[]).unwrap().unwrap_err();
        assert_eq!(err, "recall() expected String, got None");
        let err = dispatch_vm("recall", &mut vm, &[Value::Float(2.0)])
            .unwrap()
            .unwrap_err();
        assert!(
            err.starts_with("recall() expected String, got "),
            "actual: {err}"
        );
    }

    #[test]
    fn vm_forget_is_case_insensitive_and_cutoff_honored() {
        let mut vm = MockVm::new();
        let now = now_secs();
        vm.memory.push(VmMemoryEntry {
            value: "Spicy FOOD".into(),
            priority: 1.0,
            timestamp: now - 40 * 86400, // older than 30 days
            decay_rate: 0.01,
            mem_type: String::new(),
        });
        vm.memory.push(VmMemoryEntry {
            value: "fresh spicy".into(),
            priority: 1.0,
            timestamp: now, // young — stays
            decay_rate: 0.01,
            mem_type: String::new(),
        });

        // The VM lowercases the query: "SPICY" matches "Spicy FOOD".
        dispatch_vm("forget", &mut vm, &[Value::String("SPICY".into())])
            .unwrap()
            .unwrap();
        assert_eq!(vm.memory.len(), 1);
        assert_eq!(vm.memory[0].value, "fresh spicy");
    }

    #[test]
    fn vm_recall_top_k_shape_and_zero_hit_stay() {
        let mut vm = MockVm::new();
        vm.memory.push(VmMemoryEntry {
            value: "apple pie".into(),
            priority: 0.5,
            timestamp: now_secs(),
            decay_rate: 0.0,
            mem_type: "recipe".into(),
        });
        vm.memory.push(VmMemoryEntry {
            value: "unrelated".into(),
            priority: 0.5,
            timestamp: now_secs(),
            decay_rate: 0.0,
            mem_type: "recipe".into(),
        });

        let out = dispatch_vm(
            "recall_top_k",
            &mut vm,
            &[
                Value::String("apple".into()),
                Value::Float(5.0),
                Value::String("recipe".into()),
            ],
        )
        .unwrap()
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(match &out {
            Value::String(s) => s,
            other => panic!("expected String, got {other:?}"),
        })
        .unwrap();
        // Zero-hit entries STAY (the top-k contract).
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["value"], "apple pie");
        assert!(parsed[0]["score"].as_f64().unwrap() > 0.0);
        assert_eq!(parsed[0]["type"], "recipe");
        assert_eq!(parsed[0]["priority"], 0.5);

        // Unit type filter = all types.
        let out = dispatch_vm(
            "recall_top_k",
            &mut vm,
            &[Value::String("apple".into()), Value::Unit, Value::Unit],
        )
        .unwrap()
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(match &out {
            Value::String(s) => s,
            other => panic!("expected String, got {other:?}"),
        })
        .unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn vm_graph_walk_decorates_the_hit() {
        let mut vm = MockVm::new();
        vm.memory.push(VmMemoryEntry {
            value: "alice".into(),
            priority: 1.0,
            timestamp: now_secs(),
            decay_rate: 0.0,
            mem_type: String::new(),
        });
        vm.relations.push(VmRelation {
            from: "alice".into(),
            to: "bob".into(),
            relation: "knows".into(),
        });
        let out = vm.recall_lane("alice", 0.0);
        assert!(out.contains("alice"));
        assert!(out.contains("[GRAPH] knows -> bob"), "got: {out}");
    }

    #[test]
    fn tw_engines_over_in_memory_store() {
        let store = Mutex::new(Box::new(InMemoryStore::new()) as Box<dyn MemoryStore>);
        let kg = Mutex::new(Box::new(InMemoryKg::new()) as Box<dyn KgStore>);
        let em = EmbeddingManager::new();

        // memorize with the TW engine: silent success, Unit.
        let out = memorize_tw(
            &store,
            &em,
            &[
                Value::String("tw fact".into()),
                Value::Float(0.6),
                Value::String("fact".into()),
            ],
        )
        .unwrap();
        assert!(matches!(out, Value::Unit));
        assert_eq!(store.lock().unwrap().count(), 1);

        // recall_top_k with the TW engine: the entry comes back with
        // the shared JSON shape.
        let out = recall_top_k_tw(
            &store,
            &em,
            &[Value::String("tw".into()), Value::Unit, Value::Unit],
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(match &out {
            Value::String(s) => s,
            other => panic!("expected String, got {other:?}"),
        })
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["value"], "tw fact");
        assert_eq!(parsed[0]["type"], "fact");

        // recall with the TW engine: the store lane answers.
        let out = recall_tw(&store, &kg, &em, &[Value::String("tw fact".into())]).unwrap();
        match out {
            Value::String(s) => assert!(s.contains("tw fact"), "got: {s}"),
            other => panic!("expected String, got {other:?}"),
        }

        // forget with the TW engine: the entry is gone. The TW forget
        // removes entries OLDER than the cutoff (timestamp < cutoff), so
        // a negative day-count pushes the cutoff past the fresh entry —
        // the documented cutoff semantics, tested against the fact.
        forget_tw(
            &store,
            &[Value::String("tw fact".into()), Value::Float(-1.0)],
        )
        .unwrap();
        assert_eq!(store.lock().unwrap().count(), 0);

        // TW parse errors keep the exact texts (recall differs from VM).
        let err = recall_tw(&store, &kg, &em, &[Value::Float(1.0)]).unwrap_err();
        assert_eq!(err, "recall() expected String argument, got Float");
        let err = memorize_tw(&store, &em, &[]).unwrap_err();
        assert_eq!(err, "memorize() requires at least 1 argument (text)");
        let err = forget_tw(&store, &[Value::Float(1.0)]).unwrap_err();
        assert_eq!(err, "forget() expected String as first arg, got Float");
        let err = recall_top_k_tw(&store, &em, &[]).unwrap_err();
        assert_eq!(err, "recall_top_k() requires at least 1 argument (query)");
    }
}
