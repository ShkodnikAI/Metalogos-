//! SVG / Chart / Diagram builtins module
//!
//! Наряд №110: split of the former monolithic `svg.rs` into
//! focused submodules in preparation for future Cargo features.
//!
//! Re-exports everything so that `src/builtins/mod.rs` continues to work
//! unchanged with:
//!   pub(crate) mod svg;
//!   use svg::*;

mod charts;
mod diagrams;
mod primitives;
mod shared;

pub use charts::*;
pub use diagrams::*;
pub use primitives::*;
