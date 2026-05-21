//! Literal lowering — stub for Phase 7.

use kestrel_hir::body::{HirDictEntry, HirExprId, HirLiteral};
use kestrel_mir_2::{Immediate, MirTy, Operand};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    pub fn lower_literal(&mut self, expr_id: HirExprId, lit: &HirLiteral) -> Operand {
        let ty = self.resolve_expr_type(expr_id);
        match lit {
            HirLiteral::Integer(v) => {
                match self.ctx.module.ty_arena.get(ty) {
                    MirTy::I8 => Operand::Const(Immediate::i8(*v as i128)),
                    MirTy::I16 => Operand::Const(Immediate::i16(*v as i128)),
                    MirTy::I32 => Operand::Const(Immediate::i32(*v as i128)),
                    _ => Operand::Const(Immediate::i64(*v as i128)),
                }
            }
            HirLiteral::Float(v) => {
                match self.ctx.module.ty_arena.get(ty) {
                    MirTy::F32 => Operand::Const(Immediate::f32(*v)),
                    _ => Operand::Const(Immediate::f64(*v)),
                }
            }
            HirLiteral::Bool(v) => Operand::Const(Immediate::bool(*v)),
            HirLiteral::String { .. } => {
                // Full string literal lowering in Phase 7
                Operand::Const(Immediate::error())
            }
            HirLiteral::Char(c) => {
                Operand::Const(Immediate::i32(*c as i128))
            }
            HirLiteral::Null => Operand::Const(Immediate::error()),
        }
    }

    pub fn lower_array_literal(
        &mut self,
        _expr_id: HirExprId,
        _elements: &[HirExprId],
    ) -> Operand {
        Operand::Const(Immediate::error())
    }

    pub fn lower_dict_literal(
        &mut self,
        _expr_id: HirExprId,
        _entries: &[HirDictEntry],
    ) -> Operand {
        Operand::Const(Immediate::error())
    }
}
