//! Diagram builtins (diagram_*)
//! Category: "diagram" (from registry.rs)
//! diagram_style is in primitives (tokens category)
//!
//! Наряд №169: split of the former monolithic `diagrams.rs` (5875 lines)
//! into functional groups. Same protocol as наряд №110 applied to `svg.rs`:
//! one module = one commit = green `test-lib`. No logic changes.
//!
//! ## Module layout (target — being filled in incrementally)
//!
//! - `layout` — shared helpers (`box_edge_point`, color utilities,
//!   `builtin_infographic_qa`). Used by multiple diagram groups.
//! - `tree_org` — `diagram_tree`, `diagram_org_chart` (+ TreeNode struct)
//! - `flow_seq` — `diagram_flowchart`, `diagram_sequence`, `diagram_swimlane`
//!   (+ topological sort helpers)
//! - `time_gantt` — `diagram_timeline`, `diagram_gantt`, `diagram_process`,
//!   `diagram_loop`
//! - `layered` — `diagram_layers`, `diagram_venn`, `diagram_quadrant`,
//!   `diagram_pyramid`, `diagram_nested`, `diagram_medallion`,
//!   `diagram_er`, `diagram_state`
//! - `dataflow` — `diagram_data_flow`, `diagram_high_level`,
//!   `diagram_architecture` (+ extract_graph, layout_layered_nodes)
//! - `tests` — `#[cfg(test)] mod tests` (no line limit per ADR-0080)
//!
//! ## Status: in-progress split
//!
//! While the split is being performed, the original monolithic file
//! lives at `all_legacy.rs` and is re-exported wholesale. Each commit
//! extracts one group into its own module, removes that group from
//! `all_legacy.rs`, and adds the new module to the re-export list.
//! When `all_legacy.rs` is empty, it gets deleted and the split is done.

#[cfg(test)]
mod tests;

mod all_legacy;
mod flow_seq;
mod layered;
mod layout;
mod time_gantt;
mod tree_org;

pub use all_legacy::*;
pub use flow_seq::*;
pub use layered::*;
pub(crate) use layout::*;
pub use time_gantt::*;
pub use tree_org::*;
