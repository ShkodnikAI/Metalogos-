// ── IR (Intermediate Representation) for METALOGOS Phase 1 ─────────────────
// Phase 1: validated AST wrapper. Semantic analysis has already passed.
// Phase 4: will be replaced with bytecode instructions (Vec<IrInstruction>).

use crate::ast::Declaration;

/// A validated, compiled Metalogos program ready for execution.
/// In Phase 1, this wraps the AST declarations after semantic analysis.
/// In Phase 4, this will contain a flat instruction buffer for the bytecode VM.
pub struct Program {
    pub declarations: Vec<Declaration>,
}
