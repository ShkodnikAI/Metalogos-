// ── Codegen: AST → IR compilation for METALOGOS Phase 1 ──────────────────
// Phase 1: validates via semantic analysis, wraps into ir::Program.
// Phase 4: will translate AST into a flat bytecode instruction buffer.

use crate::ast::Declaration;
use crate::ir::Program;
use crate::semantic;

/// Compile a list of AST declarations into an IR program.
/// Runs semantic analysis first; returns Err if any issues are found.
pub fn compile(decls: Vec<Declaration>) -> Result<Program, String> {
    semantic::analyze(&decls)?;
    Ok(Program { declarations: decls })
}
