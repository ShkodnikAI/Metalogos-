// ── Naryad №669 (P2, types; wave 40 — the LOCAL STAGING branch) ─────
//
// The typing movement 0.31 №1 — the forecast/graph untyped package
// (the B1 line, §13.3; the №543 лекало; the №637 package precedent).
// The registry-fact pins for every row the package processed — the 4
// typed spellings AND the 2 honest refusals (the №650 form): a silent
// fictitious spelling on a refusal row (a Forecast opaque invented, a
// List<PathNode> lying about the no-path arm) fails HERE before any
// metric floor does.
//
// The handler-read verification table (the single-arm construction
// sites the spellings were read off) lives in the registry comments:
//   forecast_state   → src/forecast.rs:777-802  Value::Struct "ForecastState" (10 fields)
//   forecast_points  → src/forecast.rs:869-880  Value::Struct "ForecastPoints" (4 fields)
//   graph_neighbors  → memory.rs:837-845        make_date_struct "Neighbor" (single arm)
//   graph_query      → memory.rs:759-769        make_date_struct "GraphEntry" (single arm)
//   forecast_next    → src/forecast.rs:736-743  Value::ForecastHandle (the OPAQUE refusal)
//   graph_path       → memory.rs:776-806        List<PathNode> | List<PathError> (the HETEROGENEOUS refusal)

use metalogos::builtins::sig_types::{parse_field_meta, Type};
use metalogos::builtins::BUILTIN_REGISTRY;

fn spec_of(name: &str) -> &'static metalogos::builtins::BuiltinSpec {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} must be registered"))
}

#[test]
fn n669_the_four_typed_rows_carry_their_spellings() {
    for name in [
        "forecast_state",
        "forecast_points",
        "graph_neighbors",
        "graph_query",
    ] {
        let s = spec_of(name);
        assert!(
            s.return_type != Type::Unknown,
            "{name}: the package typed this row"
        );
        assert!(
            s.parameterized,
            "{name}: List<T>/Struct<Name> parameterized"
        );
    }
}

#[test]
fn n669_forecast_state_field_meta_is_the_prov_block() {
    let s = spec_of("forecast_state");
    assert_eq!(s.return_type, Type::Struct);
    assert_eq!(
        s.field_meta,
        "id:internal,series_id:internal,horizon:untrusted,rung:internal,pin:internal,degraded:internal,window_hash:internal,label:untrusted,rung_note:internal,skipped:internal",
        "forecast_state: the 10-entry prov-block field-meta — the in-tree run audit → internal, the caller echoes (horizon, label) → untrusted"
    );
    let pairs = parse_field_meta(s.field_meta).expect("the section must parse");
    assert_eq!(pairs.len(), 10);
}

#[test]
fn n669_forecast_points_field_meta_is_all_internal() {
    let s = spec_of("forecast_points");
    assert_eq!(s.return_type, Type::Struct);
    assert_eq!(
        s.field_meta, "points:internal,p10:internal,p50:internal,p90:internal",
        "forecast_points: the four quantile vectors — the in-tree ladder computation → internal"
    );
    let pairs = parse_field_meta(s.field_meta).expect("the section must parse");
    assert_eq!(pairs.len(), 4);
    assert!(pairs.iter().all(|(_, l)| l.as_str() == "internal"));
}

#[test]
fn n669_graph_rows_are_the_parameterized_lists() {
    let n = spec_of("graph_neighbors");
    assert_eq!(n.return_type, Type::List);
    assert!(n.parameterized, "graph_neighbors: List<Neighbor>");
    assert!(
        n.field_meta.is_empty(),
        "the List rows keep the labels stage 2"
    );
    let q = spec_of("graph_query");
    assert_eq!(q.return_type, Type::List);
    assert!(q.parameterized, "graph_query: List<GraphEntry>");
    assert!(
        q.field_meta.is_empty(),
        "the List rows keep the labels stage 2"
    );
}

#[test]
fn n669_forecast_next_stays_the_honest_opaque_refusal() {
    let s = spec_of("forecast_next");
    assert_eq!(
        s.return_type,
        Type::Unknown,
        "forecast_next: the value IS the opaque ForecastHandle and the stage-0 OpaqueKind set has NO Forecast kind — Unknown is the honest answer; a fictitious spelling is forbidden"
    );
    assert!(!s.parameterized);
}

#[test]
fn n669_graph_path_stays_the_honest_heterogeneous_refusal() {
    let s = spec_of("graph_path");
    assert_eq!(
        s.return_type,
        Type::Unknown,
        "graph_path: the found arm carries PathNode{{id,text,level}}, the no-path arm carries the PathError{{status,from,to}} singleton (memory.rs) — no single element spelling exists; a fictitious List<PathNode> would lie about the no-path arm"
    );
    assert!(!s.parameterized);
}
