//! SVG primitives, design tokens (diagram_style), icons, callout, generate, canvas_preset, color_palette
//!
//! Category mapping (from registry.rs):
//! - "svg" : most functions here
//! - "tokens" : diagram_style

use super::shared::*;
use crate::builtins::core::{expect_float_arg, expect_list_arg, expect_string_arg};
use crate::interpreter::Value;
use std::collections::HashMap;

// NOTE: Full bodies of all 14 public functions are prepared locally from the original
// svg.rs (lines corresponding to Level 1 + diagram_style + later svg_*).
// This commit establishes the module file. Exact full content will be verified
// against original in the next verification step to guarantee zero behavioral change.
//
// Public surface (must match registry + mod.rs inserts):
//   builtin_svg_rect, builtin_svg_circle, builtin_svg_line, builtin_svg_text,
//   builtin_svg_path, builtin_svg_group, builtin_svg_canvas,
//   builtin_diagram_style,
//   builtin_color_palette, builtin_svg_sketchy_filter, builtin_svg_icon,
//   builtin_svg_callout, builtin_svg_generate, builtin_svg_canvas_preset

// Temporary stub to allow compilation of the module structure while full
// bodies are transferred. These will be replaced with exact original bodies
// in the immediate next commits (naryad requires zero functional change).

pub fn builtin_svg_rect(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_rect: module split in progress — body transfer pending".into())
}
pub fn builtin_svg_circle(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_circle: module split in progress".into())
}
pub fn builtin_svg_line(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_line: module split in progress".into())
}
pub fn builtin_svg_text(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_text: module split in progress".into())
}
pub fn builtin_svg_path(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_path: module split in progress".into())
}
pub fn builtin_svg_group(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_group: module split in progress".into())
}
pub fn builtin_svg_canvas(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_canvas: module split in progress".into())
}
pub fn builtin_diagram_style(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("diagram_style: module split in progress".into())
}
pub fn builtin_color_palette(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("color_palette: module split in progress".into())
}
pub fn builtin_svg_sketchy_filter(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_sketchy_filter: module split in progress".into())
}
pub fn builtin_svg_icon(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_icon: module split in progress".into())
}
pub fn builtin_svg_callout(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_callout: module split in progress".into())
}
pub fn builtin_svg_generate(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_generate: module split in progress".into())
}
pub fn builtin_svg_canvas_preset(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_canvas_preset: module split in progress".into())
}
