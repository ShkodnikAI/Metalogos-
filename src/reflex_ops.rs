//! №466 (gh#687) — the reflex transfer group: the shared live module.
//!
//! The fourth group of the TW/VM dedup (gate gh#680, decision 4-A, step 3;
//! the CI threshold gate is №462/gh#683, the diff-fuzzer is №465/gh#686).
//! Seven builtin names — `reflex_train`, `reflex_predict`, `reflex_save`,
//! `reflex_load`, `reflex_metrics`, `reflex_list`, `reflex_generate` —
//! moved OUT of both backends: `src/vm.rs` and `src/interpreter/` keep
//! their exact per-site marshaling hooks (const-name checks in the SAME
//! dispatch order as before) and the name strings are spelled here and
//! nowhere else outside the registry. After the move the №462 counter
//! drops 42 → 35.
//!
//! Unlike the conv/db/memory groups, no function bodies moved HERE:
//! the reflex bodies were already shared since their birth — №179b
//! (train/predict), №180 (save/load), №187 (metrics/list) and №193
//! (generate) put the whole dispatch logic in `src/builtins/reflex.rs`
//! (`reflex_*_dispatch`) and both backends delegate to it. The duplication
//! the №462 counter saw on this group was the name literals plus the
//! per-site dispatch chains — exactly the consent-pair shape of group 3 —
//! so the transfer is the name constants below plus the `handles()` hook,
//! while the dispatch chains keep their sites and order.
//!
//! The module is the shared HOME, not a unification. The per-backend
//! marshaling forms stay deliberately separate and byte-identical to
//! their inline originals:
//! - the TW flow path (`Interpreter::invoke`) takes the registry with
//!   `get_mut()` and the text "reflex registry poisoned: {}", and clones
//!   the persist path BEFORE taking the registry (the borrow-checker
//!   order the №180 site documents);
//! - the TW expression path (`invoke_reflex_*` in
//!   `src/interpreter/reflex_builtin.rs`) locks the Mutex with per-name
//!   texts "reflex_X: registry lock poisoned: {}" and fetches the persist
//!   path AFTER the lock;
//! - the VM (`Vm::call_reflex_builtin`) touches its registry field
//!   directly with no lock — train/load take `&mut`, the rest `&`;
//! - the bare-Ident first-argument guards (train/predict/save/metrics/
//!   generate resolve a bare model Ident through `reflex_names` into
//!   `Value::Reflex`, with the per-name "not declared" error texts) are
//!   interpreter-side marshaling and stay at their sites.
//!
//! The live-contract requirement of the naryad (the owner's
//! "revive-or-delete" strengthening for the dead `RuntimeContext`)
//! continues the group 1/2 posture: the dead stub is already gone
//! (№465) and every backend passes its own registry, names map and
//! persist path explicitly to the shared dispatch functions — no hidden
//! global, no revived dead struct, nothing for this module to own beyond
//! the names. The №465 fuzzer pinned no reflex divergence classes; the
//! class set stays identical across this transfer (the report is checked
//! in before and after).

/// The reflex-group names this module owns. The backends compare their
/// dispatch names against the constants below — the name strings are
/// spelled here and nowhere else outside the registry.
pub const NAME_REFLEX_TRAIN: &str = "reflex_train";
pub const NAME_REFLEX_PREDICT: &str = "reflex_predict";
pub const NAME_REFLEX_SAVE: &str = "reflex_save";
pub const NAME_REFLEX_LOAD: &str = "reflex_load";
pub const NAME_REFLEX_METRICS: &str = "reflex_metrics";
pub const NAME_REFLEX_LIST: &str = "reflex_list";
pub const NAME_REFLEX_GENERATE: &str = "reflex_generate";

/// The seven reflex-group names this module owns, as a single hook.
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        NAME_REFLEX_TRAIN
            | NAME_REFLEX_PREDICT
            | NAME_REFLEX_SAVE
            | NAME_REFLEX_LOAD
            | NAME_REFLEX_METRICS
            | NAME_REFLEX_LIST
            | NAME_REFLEX_GENERATE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── the name contract ───────────────────────────────────────────────

    #[test]
    fn handles_covers_exactly_the_seven_names() {
        for name in [
            NAME_REFLEX_TRAIN,
            NAME_REFLEX_PREDICT,
            NAME_REFLEX_SAVE,
            NAME_REFLEX_LOAD,
            NAME_REFLEX_METRICS,
            NAME_REFLEX_LIST,
            NAME_REFLEX_GENERATE,
        ] {
            assert!(handles(name), "{} must be covered", name);
        }
        assert!(!handles("reflex"));
        assert!(!handles("reflex_train_now"));
        assert!(!handles("memory_train"));
        assert!(!handles(""));
    }

    #[test]
    fn name_constants_match_the_registry_spelling() {
        // The constants are the single spelling of the group outside the
        // registry; the values must stay byte-identical to the historical
        // inline literals (a rename is an owner-gated change, not a
        // transfer).
        assert_eq!(NAME_REFLEX_TRAIN, "reflex_train");
        assert_eq!(NAME_REFLEX_PREDICT, "reflex_predict");
        assert_eq!(NAME_REFLEX_SAVE, "reflex_save");
        assert_eq!(NAME_REFLEX_LOAD, "reflex_load");
        assert_eq!(NAME_REFLEX_METRICS, "reflex_metrics");
        assert_eq!(NAME_REFLEX_LIST, "reflex_list");
        assert_eq!(NAME_REFLEX_GENERATE, "reflex_generate");
    }

    #[test]
    fn the_seven_constants_are_distinct() {
        let names = [
            NAME_REFLEX_TRAIN,
            NAME_REFLEX_PREDICT,
            NAME_REFLEX_SAVE,
            NAME_REFLEX_LOAD,
            NAME_REFLEX_METRICS,
            NAME_REFLEX_LIST,
            NAME_REFLEX_GENERATE,
        ];
        let mut seen = std::collections::BTreeSet::new();
        for name in names {
            assert!(seen.insert(name), "duplicate constant value: {}", name);
        }
        assert_eq!(seen.len(), 7);
    }
}
