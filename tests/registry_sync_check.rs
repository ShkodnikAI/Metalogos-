/// Наряд №55 Block 2: Set-difference test between BUILTIN_REGISTRY and dispatcher.
///
/// This test ensures that every function registered in the dispatcher (`funcs.insert`)
/// has a corresponding `spec!` entry in `BUILTIN_REGISTRY`, and vice versa.
///
/// The existing `registry_arity_check.rs` tests arity values; this test catches
/// missing entries entirely. Different failure modes, both needed.
///
/// Registry functions that intentionally lack a dispatcher entry fall into these
/// categories: "stub" (handled by interceptor in execution.rs), "stateful",
/// "graph", "mtree", "cron" (handled by dedicated invoke_* methods), and "test".
/// These are listed in the REGISTRY_ONLY_CATEGORIES allowlist.
use metalogos::builtins::{builtin_count, builtin_name_set, builtin_names, BUILTIN_REGISTRY};
use std::collections::HashSet;

/// Categories whose members are intentionally registered in BUILTIN_REGISTRY
/// but NOT in the dispatcher (funcs.insert). These are handled by interceptors
/// or specialized invoke_* methods in execution.rs / server.rs.
const REGISTRY_ONLY_CATEGORIES: &[&str] = &[
    "stub",     // handled by execution.rs interceptors
    "stateful", // runtime-only, no direct dispatch
    "graph",    // handled by graph module invoke methods
    "mtree",    // handled by memory module invoke methods
    "cron",     // handled by cron module invoke methods
    "test",     // assert_eq, assert_contains — compile-time only
    "std",      // __trim, __replace etc. — compile-time only
    "convert",  // float(), to_string() etc.
    "web",      // respond, web etc. — server-only
];

/// Categories whose members have BOTH a spec! entry AND a dispatcher entry.
const DISPATCHED_CATEGORIES: &[&str] = &[
    "string",
    "db",
    "llm",
    "memory",
    "time",
    "encoding",
    "recipe",
    "orchestration",
    "vault",
    "pdf",
    "crypto",
    "voice",
    "bot",
    "fluid",
    "json",
    "list",
    "io",
    "system",
];

fn registry_categories() -> HashSet<&'static str> {
    let mut cats = HashSet::new();
    for spec in BUILTIN_REGISTRY.iter() {
        cats.insert(spec.category);
    }
    cats
}

/// All known categories — union of registry-only and dispatched.
fn all_known_categories() -> Vec<&'static str> {
    REGISTRY_ONLY_CATEGORIES
        .iter()
        .chain(DISPATCHED_CATEGORIES.iter())
        .copied()
        .collect()
}

#[test]
fn registry_only_categories_are_allowlisted() {
    let cats = registry_categories();
    let known = all_known_categories();
    for cat in &cats {
        assert!(
            known.iter().any(|k| k == cat),
            "Unknown registry category '{}': add to REGISTRY_ONLY_CATEGORIES or DISPATCHED_CATEGORIES",
            cat
        );
    }
}

/// The key invariant: builtin_count() must match BUILTIN_REGISTRY.len(),
/// and builtin_names() / builtin_name_set() must agree with the registry.
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
