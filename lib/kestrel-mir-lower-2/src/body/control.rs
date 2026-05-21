//! Control flow lowering — stub for Phase 5.

use kestrel_hir::body::{HirBlock, HirExprId};
use kestrel_mir_2::{Immediate, Operand};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    pub fn lower_if(
        &mut self,
        _expr_id: HirExprId,
        _condition: HirExprId,
        _then_body: &HirBlock,
        _else_body: Option<&HirBlock>,
    ) -> Operand {
        Operand::Const(Immediate::error())
    }

    pub fn lower_loop(&mut self, _body: &HirBlock, _label: Option<&str>) -> Operand {
        Operand::Const(Immediate::unit())
    }

    pub fn lower_break(&mut self, _label: Option<&str>) -> Operand {
        Operand::Const(Immediate::unit())
    }

    pub fn lower_continue(&mut self, _label: Option<&str>) -> Operand {
        Operand::Const(Immediate::unit())
    }
}
