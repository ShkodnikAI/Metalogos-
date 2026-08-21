//! SVG / Chart / Diagram builtins module
//!
//! Наряд №110: split of the former monolithic `svg.rs`.
//! Наряд №111: Cargo features — `svg` / `chart` / `diagram`.

#[cfg(any(feature = "svg", feature = "chart", feature = "diagram"))]
mod shared;

#[cfg(feature = "svg")]
mod primitives;
#[cfg(feature = "chart")]
mod charts;
#[cfg(feature = "diagram")]
mod diagrams;

#[cfg(feature = "svg")]
pub use primitives::*;
#[cfg(feature = "chart")]
pub use charts::*;
#[cfg(feature = "diagram")]
pub use diagrams::*;
