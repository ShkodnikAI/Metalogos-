//! Diagram builtins (diagram_*)
//! Category: "diagram" (from registry.rs)
//! diagram_style is in primitives (tokens category)
//!
//! Наряд №169: split of the former monolithic `diagrams.rs` (5875 lines)
//! into 6 functional modules. All groups extracted, `all_legacy.rs` deleted.
//!
//! ## Final module layout
//!
//! - `layout` — shared helpers (`box_edge_point`, color utilities,
//!   `builtin_infographic_qa`). ~380 lines.
//! - `tree_org` — `diagram_tree`, `diagram_org_chart` (+ TreeNode struct). ~376 lines.
//! - `flow_seq` — `diagram_flowchart`, `diagram_sequence`, `diagram_swimlane`
//!   (+ topological sort helpers). ~1042 lines.
//! - `time_gantt` — `diagram_timeline`, `diagram_gantt`, `diagram_process`,
//!   `diagram_loop`. ~952 lines.
//! - `layered` — `diagram_layers`, `diagram_venn`, `diagram_quadrant`,
//!   `diagram_pyramid`, `diagram_nested`, `diagram_medallion`,
//!   `diagram_er`, `diagram_state`. ~1853 lines.
//! - `dataflow` — `diagram_data_flow`, `diagram_high_level`,
//!   `diagram_architecture` (+ extract_graph, layout_layered_nodes). ~651 lines.
//! - `tests` — `#[cfg(test)] mod tests` (no line limit per ADR-0080). ~701 lines.

#[cfg(test)]
mod tests;

mod dataflow;
mod flow_seq;
mod layered;
mod layout;
mod time_gantt;
mod tree_org;

pub use dataflow::*;
pub use flow_seq::*;
pub use layered::*;
pub(crate) use layout::*;
pub use time_gantt::*;
pub use tree_org::*;
