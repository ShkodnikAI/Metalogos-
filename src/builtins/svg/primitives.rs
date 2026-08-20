//! SVG primitives, design tokens (diagram_style), icons, callout, generate, canvas_preset, color_palette
//!
//! Category mapping (from registry.rs):
//! - "svg" : most functions here
//! - "tokens" : diagram_style

use super::shared::*;
use crate::interpreter::Value;
use std::collections::HashMap;

// Placeholder — full body will be pushed in next commits (size limit).
// This commit establishes the file so the module tree compiles once charts/diagrams exist.

pub fn builtin_svg_rect(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_rect: module split in progress".into())
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

pub(crate) fn extract_style(value: &Value) -> Result<HashMap<String, Value>, String> {
    let _ = value;
    Err("extract_style: module split in progress".into())
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

pub(crate) fn canvas_preset(name: &str) -> Option<(f64, f64)> {
    let _ = name;
    None
}

pub fn builtin_svg_canvas_preset(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    Err("svg_canvas_preset: module split in progress".into())
}
