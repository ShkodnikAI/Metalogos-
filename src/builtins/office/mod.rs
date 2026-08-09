// ── Office & productivity builtins (module) ─────────────────────────
// Split from office.rs (naryad-61) — 1724 lines → 6 focused modules.
// Original: human_respond, goals, todos, entity extraction, memory scoring,
// recipes, DAG, semantic search, preferences, vault, compression.

mod config;
mod goals;
mod graph;
mod human;
mod recipes;
mod text;

pub(crate) use config::*;
pub(crate) use goals::*;
pub(crate) use graph::*;
pub(crate) use human::*;
pub(crate) use recipes::*;
pub(crate) use text::*;
