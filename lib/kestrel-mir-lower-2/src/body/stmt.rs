//! Statement lowering — HirStmt dispatch.

use kestrel_hir::body::{HirStmt, HirStmtId};
use kestrel_mir_2::{Operand, Place, Rvalue, UseMode};

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    pub fn lower_stmt(&mut self, stmt_id: HirStmtId) {
        let stmt = self.hir.stmts[stmt_id].clone();
        let span = stmt_span(&self.hir.stmts[stmt_id]);
        let prev_span = self.current_span.replace(span);

        match &stmt {
            HirStmt::Let { local, value, .. } => {
                let mir_local = self.map_local(*local);
                if let Some(init_expr) = value {
                    let init_ty = self.resolve_expr_type(*init_expr);
                    let init_val = self.lower_expr(*init_expr);
                    self.emit_value_transfer(Place::local(mir_local), init_val, init_ty);
                }
            }
            HirStmt::Expr { expr, .. } => {
                let _ = self.lower_expr(*expr);
            }
            HirStmt::Deinit {
                local: Some(hir_local),
                ..
            } => {
                let mir_local = self.map_local(*hir_local);
                let ty = self.body.local(mir_local).ty;
                let temp = self.fresh_temp(ty);
                self.emit_assign(
                    Place::local(temp),
                    Rvalue::Use(Operand::Place(Place::local(mir_local)), UseMode::Move),
                );
            }
            HirStmt::Deinit { local: None, .. } => {}
        }

        self.current_span = prev_span;
    }
}

fn stmt_span(stmt: &HirStmt) -> kestrel_span::Span {
    match stmt {
        HirStmt::Let { span, .. } | HirStmt::Expr { span, .. } | HirStmt::Deinit { span, .. } => {
            span.clone()
        }
    }
}
