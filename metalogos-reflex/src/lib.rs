//! metalogos-reflex — the reflex domain crate STUB.
//!
//! №545 (Wave 24, issue #883; the №472 roadmap, decision 2-B;
//! docs/refactoring-split-plan.md): this crate is the crate-stub
//! ("крейт-заготовка") of sub-step (а) — the workspace member exists,
//! the version is lockstep ([workspace.package]), the candle gate
//! mirrors the language crate's off-by-default posture.
//!
//! **This stub ships NO moved code yet, deliberately.** The physical
//! module moves (the NN module, the generative-model machinery, the
//! candle/tokenizers surfaces — ADR-0178 §3) are BLOCKED on the
//! dependency-direction fork the roadmap explicitly reserves for the
//! owner's gate ("depends on core for the value/handle types, or on a
//! tiny `metalogos-values` crate if the dependency direction demands
//! it — the split PR decides with the owner's gate"):
//!
//! 1. THE COUPLING FACT (the inversion inventory, №545): the core
//!    currently depends on the domain in BOTH directions' worth of
//!    edges — `Value::Reflex(crate::nn::ReflexId)` and
//!    `Value::BpeVocab(crate::nn::bpe::BpeVocabId)` live in the CORE
//!    value enum (src/interpreter/values.rs), and the core files
//!    import `crate::nn::` directly: src/vm.rs, src/distill_hub.rs,
//!    src/builtins/reflex.rs, src/interpreter/{mod,learnable,
//!    execution}.rs, src/builtins/template.rs. Moving the modules
//!    before these edges are inverted would create a CYCLIC crate
//!    dependency (metalogos → reflex → metalogos), which Rust
//!    forbids.
//! 2. THE FORK: (i) core-first inversion — the core drops its
//!    `crate::nn::` imports through trait seams (the №484 DbAccess
//!    precedent), the moved code keeps `Value` via
//!    `metalogos::interpreter::Value`; or (ii) the tiny
//!    `metalogos-values` crate both depend on. The fork decides the
//!    shape of every follow-up sub-PR.
//! 3. THE FOLLOW-UP SUB-PRs (per the naryad's "под-шаги = под-PR"):
//!    the inversion sub-PRs per the inventory above, THEN the
//!    physical move of the stop-list files (16 files, the 9430 LOC
//!    baseline moves 1:1 — the manifest follows the files, the
//!    baseline does not grow, ADR-0178 §4 needs no patch), THEN the
//!    re-export shell (`metalogos::reflex` → the new crate) for the
//!    no-API-change contract.
//!
//! Until the fork is decided, this crate compiles EMPTY on purpose —
//! an empty lib is an honest stub, not a silent placeholder (§16.0-D
//! reads on the naryad's contract, not on an architecture decision
//! this crate is not authorized to make).
//!
//! The stop-list (№463) still counts the files at their CURRENT paths;
//! the move sub-PRs will relocate the manifest entries in the SAME PR
//! as the files (the gate reads the manifest, not the tree).
