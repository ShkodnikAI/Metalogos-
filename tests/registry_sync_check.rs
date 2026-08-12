use metalogos::builtins::Builtins;
/// Наряд №55 Block 2: Set-difference test between BUILTIN_REGISTRY and dispatcher.
///
/// This test ensures that every function registered in the dispatcher (`funcs.insert`)
/// has a corresponding `spec!` entry in `BUILTIN_REGISTRY`, and vice versa.
///
/// The existing `registry_arity_check.rs` tests arity values; this test catches
/// missing entries entirely. Different failure modes, both needed.
///
/// HOW IT WORKS:
/// - Creates a Builtins instance to get the real dispatcher names (funcs.insert keys)
/// - Compares them against BUILTIN_REGISTRY names
/// - Extra dispatcher names (no spec!) = FAIL (caught insertion without registration)
/// - Missing dispatcher names (spec! but no funcs.insert) = FAIL only for dispatched categories
///   (registry-only categories like "stub", "cron" are allowlisted)
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
    "math",
    "email",    // Наряд MLG-4: SMTP + IMAP
    "calendar", // Наряд MLG-5: CalDAV + iCal
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

/// THE KEY TEST: compare dispatcher (funcs.insert) with registry (spec!).
///
/// Catches two failure modes:
/// 1. funcs.insert without spec! → "dispatcher has extra: {name}"
/// 2. spec! without funcs.insert for a dispatched category → "registry has extra: {name}"
///
/// If this test NEVER fails, it means the dispatcher and registry are in sync.
/// If it fails, someone added a function to one but not the other.
#[test]
fn dispatcher_vs_registry_set_difference() {
    // Get real dispatcher names by instantiating Builtins
    let builtins = Builtins::new();
    let dispatcher_names: HashSet<String> = builtins.dispatcher_names();

    // Get registry names
    let registry_names: HashSet<String> = BUILTIN_REGISTRY
        .iter()
        .map(|s| s.name.to_string())
        .collect();

    // Collect ALL discrepancies (both directions) before asserting,
    // so the test reports everything in one panic, not just the first hit.
    let mut errors: Vec<String> = Vec::new();

    // Direction 1: registry-only (spec! but no funcs.insert) in dispatched categories
    for name in &registry_names {
        if dispatcher_names.contains(name) {
            continue;
        }
        let category = BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == *name)
            .map(|s| s.category)
            .unwrap_or("unknown");
        if !REGISTRY_ONLY_CATEGORIES.contains(&category) {
            errors.push(format!(
                "  REGISTRY -> no dispatcher: '{}' (category '{}')",
                name, category
            ));
        }
    }

    // Direction 2: dispatcher-only (funcs.insert but no spec!)
    for name in &dispatcher_names {
        if registry_names.contains(name) {
            continue;
        }
        errors.push(format!("  DISPATCHER -> no spec!: '{}'", name));
    }

    if !errors.is_empty() {
        errors.sort();
        panic!(
            "Registry-dispatcher sync check FAILED ({} discrepancies):\n{}",
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
