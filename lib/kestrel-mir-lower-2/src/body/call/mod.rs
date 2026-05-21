//! Call dispatch — stub for Phase 6.

use kestrel_hir::body::{HirCallArg, HirExprId};
use kestrel_hir::ty::HirTy;
use kestrel_mir_2::{Immediate, Operand};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    pub fn lower_call_expr(
        &mut self,
        _expr_id: HirExprId,
        _callee_expr: HirExprId,
        _args: &[HirCallArg],
    ) -> Operand {
        Operand::Const(Immediate::error())
    }

    pub fn lower_method_call_expr(
        &mut self,
        _expr_id: HirExprId,
        _receiver: HirExprId,
        _method: &str,
        _type_args: Option<&[HirTy]>,
        _args: &[HirCallArg],
    ) -> Operand {
        Operand::Const(Immediate::error())
    }

    pub fn lower_protocol_call_expr(
        &mut self,
        _expr_id: HirExprId,
        _receiver: HirExprId,
        _protocol: kestrel_hecs::Entity,
        _method: &str,
        _args: &[HirCallArg],
    ) -> Operand {
        Operand::Const(Immediate::error())
    }
}
