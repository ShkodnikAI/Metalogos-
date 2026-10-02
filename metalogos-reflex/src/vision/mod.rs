//! The vision diffusion chain — the generative contour's image path
//! (naryad №545 (в)): the six stop-list modules moved 1:1 from the
//! language crate together with the shared coverage check. The
//! language crate re-exports them (the shell) so `crate::vision::*`
//! consumer paths are preserved. Gating mirrors the language crate:
//! everything here is behind the `vision` feature (which implies
//! `candle` — ADR-0122/№211/№212 lineage).

#[cfg(feature = "vision")]
pub mod coverage;
#[cfg(feature = "vision")]
pub mod dit;
#[cfg(feature = "vision")]
pub mod lora;
#[cfg(feature = "vision")]
pub mod sampler;
#[cfg(feature = "vision")]
pub mod text_encoder;
#[cfg(feature = "vision")]
pub mod tokenizer;
#[cfg(feature = "vision")]
pub mod vae;
