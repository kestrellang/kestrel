//! Expression lowering — stub for Phase 4.

use kestrel_hir::body::HirExprId;
use kestrel_mir_2::{Immediate, Operand};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    /// Lower an HIR expression to an Operand. Stub — returns error for now.
    pub fn lower_expr(&mut self, _expr_id: HirExprId) -> Operand {
        Operand::Const(Immediate::error())
    }
}
