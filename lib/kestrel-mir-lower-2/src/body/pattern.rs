//! Pattern matching / match lowering — stub for Phase 9.

use kestrel_hir::body::{HirExprId, HirMatchArm};
use kestrel_mir_2::{Immediate, Operand};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    pub fn lower_match(
        &mut self,
        _expr_id: HirExprId,
        _scrutinee: HirExprId,
        _arms: &[HirMatchArm],
    ) -> Operand {
        Operand::Const(Immediate::error())
    }
}
