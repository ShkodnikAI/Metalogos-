// ── Naryad №668 (P1, types; wave 40 — the LOCAL STAGING branch) ─────
//
// Q-2-движение №2 «memory → recognition» — the second §6.3
// sensitive-surface package. The 8 package rows + world_state: the
// REGISTRY-FACT pins for every spelling the package lands (the n565
// pattern — the rows are pinned against the compiled BUILTIN_REGISTRY),
// so a silent spelling change (an element type renamed, a field-meta
// section dropped, the opaque row de-typed) fails here BEFORE the
// CI metric floors do.
//
// The handler-read verification table (the single-arm make_struct /
// make_date_struct sites the spellings were read off) lives in the
// registry comments (the №650 form):
//   human_personas  → office/human.rs:210-220  "PersonaSummary"
//   human_recall    → server.rs:825-840        "Memory"
//   check_reminders → cron.rs:284-297          "DueReminder"
//   list_reminders  → cron.rs:242-256          "Reminder"
//   semantic_search → office/text.rs:738-745   "SearchResult"
//   mtree_retrieve  → memory.rs:535-545        "MTreeEntry"
//   mtree_stats     → memory.rs:716-727        "MTreeStats"
//   mtree_summarize → memory.rs:690-701        "MTreeSummarize"
//   world_state     → embodied.rs:489-497      Value::WorldState (opaque)
//
// The floors themselves (the source baselines + this file's twins in
// type_signature_metric.rs) carry the movement arithmetic.

use metalogos::builtins::sig_types::{parse_field_meta, Type};
use metalogos::builtins::BUILTIN_REGISTRY;

fn spec_of(name: &str) -> &'static metalogos::builtins::BuiltinSpec {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} must be registered"))
}

#[test]
fn n668_the_four_list_rows_carry_their_element_spellings() {
    for (name, elem) in [
        ("human_personas", "PersonaSummary"),
        ("human_recall", "Memory"),
        ("check_reminders", "DueReminder"),
        ("list_reminders", "Reminder"),
    ] {
        let s = spec_of(name);
        assert_eq!(
            s.return_type,
            Type::List,
            "{name}: the List row stays the erased List"
        );
        assert!(
            s.parameterized,
            "{name}: the element spelling List<{elem}> must land parameterized"
        );
    }
}

#[test]
fn n668_semantic_search_and_mtree_retrieve_are_parameterized_lists() {
    for name in ["semantic_search", "mtree_retrieve"] {
        let s = spec_of(name);
        assert_eq!(s.return_type, Type::List, "{name}: List");
        assert!(s.parameterized, "{name}: List<T> parameterized");
    }
}

#[test]
fn n668_the_two_mtree_struct_rows_carry_the_full_field_meta() {
    let stats = spec_of("mtree_stats");
    assert_eq!(stats.return_type, Type::Struct, "mtree_stats: Struct");
    assert!(stats.parameterized, "mtree_stats: Struct<MTreeStats>");
    assert_eq!(
        stats.field_meta,
        "l0:internal,l1:internal,l2:internal,total:internal,total_chars:internal,graph_edges:internal,components:internal",
        "mtree_stats: the 7-entry field-meta, every field the in-tree graph diagnostic → internal"
    );
    let summ = spec_of("mtree_summarize");
    assert_eq!(summ.return_type, Type::Struct, "mtree_summarize: Struct");
    assert!(
        summ.parameterized,
        "mtree_summarize: Struct<MTreeSummarize>"
    );
    assert_eq!(
        summ.field_meta,
        "l0_promoted:internal,l1_count:internal,l2_created:internal,status:internal,graph_nodes:internal,graph_edges:internal,components:internal",
        "mtree_summarize: the 7-entry field-meta, every field the in-tree computation → internal"
    );
}

#[test]
fn n668_world_state_is_the_honest_opaque_embodied_handle() {
    let s = spec_of("world_state");
    assert_eq!(
        s.return_type,
        Type::Opaque(metalogos::builtins::sig_types::OpaqueKind::Embodied),
        "world_state: the OPAQUE WorldState handle spelled Embodied (the audit's own grouping) — NOT a fictitious Struct"
    );
    assert!(!s.parameterized, "world_state: the handle is not a parameterized shape — the sensitive row honestly stays uncovered (the recorded cause in the registry comment)");
    assert!(
        s.field_meta.is_empty(),
        "world_state: no field-meta on an opaque row"
    );
}

#[test]
fn n668_the_spellings_parse_to_the_declared_shapes() {
    // The fill-site parser must agree with every spelling the package
    // landed (a vocabulary miss would compile to Unknown and drop the
    // share — the floors would catch it, this pins the intent).
    assert_eq!(Type::from_path("List<PersonaSummary>"), Type::List);
    assert_eq!(Type::from_path("List<Memory>"), Type::List);
    assert_eq!(Type::from_path("List<DueReminder>"), Type::List);
    assert_eq!(Type::from_path("List<Reminder>"), Type::List);
    assert_eq!(Type::from_path("List<SearchResult>"), Type::List);
    assert_eq!(Type::from_path("List<MTreeEntry>"), Type::List);
    assert_eq!(
        Type::from_path("Struct<MTreeStats>{l0:internal}"),
        Type::Struct
    );
    assert_eq!(
        Type::from_path("Struct<MTreeSummarize>{l0_promoted:internal,l1_count:internal}"),
        Type::Struct
    );
    assert_eq!(
        Type::from_path("Embodied"),
        Type::Opaque(metalogos::builtins::sig_types::OpaqueKind::Embodied)
    );
    // The field-meta side-table parses (the №627 consumer API).
    let pairs = parse_field_meta(spec_of("mtree_stats").field_meta)
        .expect("mtree_stats: the field-meta section must parse");
    assert_eq!(pairs.len(), 7, "mtree_stats: 7 labeled fields");
    assert!(pairs.iter().all(|(_, l)| l.as_str() == "internal"));
}
