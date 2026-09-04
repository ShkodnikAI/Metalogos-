use metalogos::builtins::Builtins;

/// Наряд №170: SSOT — all handlers are now in BUILTIN_REGISTRY.
/// The old two-list sync (registry.rs + funcs.insert in mod.rs) is gone.
/// This test now checks INTERNAL CONSISTENCY of BUILTIN_REGISTRY:
///
/// - Every spec with `handler: Some(h)` must be registered in Builtins::new()
/// - Every spec with `handler: None` must NOT be in the dispatcher (it's a stub)
/// - There must be no extra entries in the dispatcher
///
/// The old cross-list check (extra funcs.insert, missing funcs.insert) is
/// structurally impossible now — there's only one list.
use metalogos::builtins::{builtin_count, builtin_name_set, builtin_names, BUILTIN_REGISTRY};
use std::collections::HashSet;

/// Internal consistency: builtin_count() / builtin_names() / builtin_name_set()
/// must all agree with BUILTIN_REGISTRY (they are all derived from it).
#[test]
fn registry_names_are_consistent() {
    assert_eq!(
        builtin_count(),
        BUILTIN_REGISTRY.len(),
        "builtin_count() disagrees with BUILTIN_REGISTRY.len()"
    );

    let registry_names: HashSet<String> = BUILTIN_REGISTRY
        .iter()
        .map(|s| s.name.to_string())
        .collect();
    let builtin_names_set: HashSet<String> = builtin_names().into_iter().collect();

    assert_eq!(
        registry_names, builtin_names_set,
        "builtin_names() disagrees with BUILTIN_REGISTRY names"
    );

    let name_set: HashSet<String> = builtin_name_set().into_iter().collect();
    assert_eq!(
        registry_names, name_set,
        "builtin_name_set() disagrees with BUILTIN_REGISTRY"
    );
}

/// THE KEY TEST: verify that Builtins::new() correctly registers handlers
/// from BUILTIN_REGISTRY.
///
/// After Наряд №170, the dispatcher is built by iterating BUILTIN_REGISTRY.
/// This test verifies:
/// 1. Every spec with `handler: Some(h)` is in the dispatcher
/// 2. Every spec with `handler: None` (stub) is NOT in the dispatcher
/// 3. No extra entries exist in the dispatcher
///
/// If this test passes, the SSOT contract holds: the registry is the single
/// source of truth, and Builtins::new() faithfully reflects it.
#[test]
fn dispatcher_matches_registry_handlers() {
    let builtins = Builtins::new();
    let dispatcher_names: HashSet<String> = builtins.dispatcher_names();

    let registry_names: HashSet<String> = BUILTIN_REGISTRY
        .iter()
        .map(|s| s.name.to_string())
        .collect();

    let mut errors: Vec<String> = Vec::new();

    // Direction 1: specs with handler: Some(h) must be in dispatcher
    for spec in BUILTIN_REGISTRY.iter() {
        if spec.handler.is_some() {
            if !dispatcher_names.contains(spec.name) {
                errors.push(format!(
                    "  spec! '{}' has handler: Some but is NOT in dispatcher (Builtins::new() failed to register it)",
                    spec.name
                ));
            }
        } else {
            // handler: None — stub, should NOT be in dispatcher
            if dispatcher_names.contains(spec.name) {
                errors.push(format!(
                    "  spec! '{}' has handler: None (stub) but IS in dispatcher (should not be registered)",
                    spec.name
                ));
            }
        }
    }

    // Direction 2: no extra dispatcher entries (not in registry at all)
    for name in &dispatcher_names {
        if !registry_names.contains(name) {
            errors.push(format!(
                "  dispatcher has '{}' but it's not in BUILTIN_REGISTRY",
                name
            ));
        }
    }

    if !errors.is_empty() {
        errors.sort();
        panic!(
            "BUILTIN_REGISTRY internal consistency check FAILED ({} discrepancies):\n{}",
            errors.len(),
            errors.join("\n")
        );
    }
}

/// Test that specifically verifies the 6 functions from Наряд №55
/// are now in the registry with correct arity.
#[test]
fn naryad_55_fixes_present() {
    use metalogos::builtins::check_builtin_arity;

    // db_execute: 1..2 (ADR-0068)
    assert!(
        check_builtin_arity("db_execute", 1).is_ok(),
        "db_execute(1) should be valid"
    );
    assert!(
        check_builtin_arity("db_execute", 2).is_ok(),
        "db_execute(2) should be valid"
    );
    assert!(
        check_builtin_arity("db_execute", 0).is_err(),
        "db_execute(0) should be invalid"
    );
    assert!(
        check_builtin_arity("db_execute", 3).is_err(),
        "db_execute(3) should be invalid"
    );

    // cron_add: 2
    assert!(
        check_builtin_arity("cron_add", 2).is_ok(),
        "cron_add(2) should be valid"
    );
    assert!(
        check_builtin_arity("cron_add", 1).is_err(),
        "cron_add(1) should be invalid"
    );

    // cron_list: 0 (variadic — takes 0+ args, like todo_list/goals_list/kv_list)
    assert!(
        check_builtin_arity("cron_list", 0).is_ok(),
        "cron_list(0) should be valid"
    );
    assert!(
        check_builtin_arity("cron_list", 1).is_ok(),
        "cron_list(1) should be valid (variadic)"
    );

    // cron_remove: 1
    assert!(
        check_builtin_arity("cron_remove", 1).is_ok(),
        "cron_remove(1) should be valid"
    );

    // cron_run: 1
    assert!(
        check_builtin_arity("cron_run", 1).is_ok(),
        "cron_run(1) should be valid"
    );

    // to_int: 1
    assert!(
        check_builtin_arity("to_int", 1).is_ok(),
        "to_int(1) should be valid"
    );
    assert!(
        check_builtin_arity("to_int", 2).is_err(),
        "to_int(2) should be invalid"
    );
}
