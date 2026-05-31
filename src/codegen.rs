// ── Codegen: AST → IR compilation for METALOGOS Phase 2 ──────────────────
// Phase 2: validates via semantic analysis (with error recovery + type inference),
// wraps into ir::Program. Propagates warnings.
// Phase 4: will translate AST into a flat bytecode instruction buffer.

use crate::ast::Declaration;
use crate::ir::Program;
use crate::semantic;

/// Compile a list of AST declarations into an IR program.
/// Runs semantic analysis first; returns Err with all errors if any issues found.
/// Returns Ok with (Program, warnings) on success.
pub fn compile(decls: Vec<Declaration>) -> Result<(Program, Vec<String>), String> {
    let result = semantic::analyze(&decls);
    if result.has_errors() {
        return Err(result.format_errors());
    }
    Ok((Program { declarations: decls }, result.warnings))
}
