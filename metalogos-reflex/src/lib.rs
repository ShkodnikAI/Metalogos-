//! metalogos-reflex — the reflex domain crate: the generative
//! contour's home (the №472 roadmap, ADR-0178 made physical).
//!
//! №545 (Wave 24, issue #883): stage (а) landed the crate-stub, stage
//! (б) inverted the core's edges through the handle contracts
//! (`metalogos::nn::handles` — the DbAccess №484 precedent, the
//! consumer-owned contract that STAYS in the language crate), stage
//! (в) — THIS change — moved the machinery 1:1:
//!
//! - `nn` — the ten stop-list modules (attention, bpe, transformer
//!   blocks, rmsnorm, swiglu, seq/gen models, the SequenceLayer
//!   trait) plus the accuracy SSOT;
//! - `vision` — the six diffusion-chain modules (dit, vae, lora,
//!   sampler, text_encoder, tokenizer) plus the shared coverage
//!   check.
//!
//! The dependency direction is CORE → REFLEX (this crate depends on
//! NOTHING from the language crate — it is Value-free by design: the
//! Value marshaling stays with the language builtins, the machinery
//! speaks primitives and tensors). The `mlog` binary links the
//! machinery through the language crate's dependency on this crate;
//! the re-export shell (`metalogos::nn::*`, `metalogos::vision::*`)
//! preserves every public path — the no-API-change contract.
//!
//! The stop-list (№463) manifest entries moved WITH the files in the
//! same PR; the baseline does not grow (the split is a move, not a
//! growth — ADR-0178 §4).

pub mod nn;
pub mod vision;
