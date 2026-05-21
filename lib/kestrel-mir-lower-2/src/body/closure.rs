//! Closure lowering — stub for Phase 8.

use kestrel_hir::body::{HirBlock, HirClosureParam, HirExprId};
use kestrel_mir_2::{Immediate, Operand};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    pub fn lower_closure_expr(
        &mut self,
        _expr_id: HirExprId,
        _params: &[HirClosureParam],
        _body: &HirBlock,
    ) -> Operand {
        Operand::Const(Immediate::error())
    }
}
