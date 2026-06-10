//! Statement lowering — HirStmt dispatch for OSSA.
//!
//! Three statement kinds:
//! - `Let` — lower init expr, bind result to HIR local in local_map, track if @owned
//! - `Expr` — lower expression for side effects (result unused)
//! - `Deinit` — explicit deinit: emit DestroyValue for the local's current value

use kestrel_hir::body::{HirStmt, HirStmtId};
use kestrel_span::Span;

use super::OssaBodyCtx;

impl OssaBodyCtx<'_, '_> {
    pub fn lower_stmt(&mut self, stmt_id: HirStmtId) {
        let stmt = self.hir.stmts[stmt_id].clone();
        let span = stmt_span(&self.hir.stmts[stmt_id]);
        let prev_span = self.current_span.replace(span);

        match &stmt {
            HirStmt::Let { local, value, .. } => {
                let is_var = self.hir.locals[*local].is_mut;
                let name = self.hir.locals[*local].name.clone();
                if let Some(init_expr) = value {
                    let init_val = self.lower_expr(*init_expr);
                    // Stage-1 binding decay: a ref-typed initializer
                    // (ret_borrow call result) is COPIED out — the binding
                    // owns a value, never the place — and the copy is the
                    // ref's single use, so its borrow ends here.
                    let init_val = if self.ref_results.contains(&init_val) {
                        let owned = self.emit_copy_value(init_val);
                        self.emit_end_borrow(init_val);
                        owned
                    } else {
                        init_val
                    };
                    if is_var {
                        let ty = self.resolve_local_type(*local);
                        let addr = self.emit_uninit(ty);
                        self.emit_store_init(addr, init_val);
                        self.body.value_names.insert(addr, name);
                        self.local_map
                            .insert(*local, super::LocalBinding::Var(addr));
                        let flag = self.maybe_alloc_var_flag(ty);
                        self.track_var(addr, ty, Some(*local), flag);
                    } else {
                        // Diagnostics-only: escape errors name the binding the
                        // returned borrow roots at ("borrows local `x`").
                        self.body.value_names.insert(init_val, name);
                        self.local_map
                            .insert(*local, super::LocalBinding::Ssa(init_val));
                    }
                } else if is_var {
                    let ty = self.resolve_local_type(*local);
                    let addr = self.emit_uninit(ty);
                    self.body.value_names.insert(addr, name);
                    self.local_map
                        .insert(*local, super::LocalBinding::Var(addr));
                    let flag = self.maybe_alloc_var_flag(ty);
                    self.track_var(addr, ty, Some(*local), flag);
                }
            },
            HirStmt::Expr { expr, .. } => {
                self.lower_expr(*expr);
            },
            HirStmt::Deinit {
                local: Some(hir_local),
                ..
            } => {
                if self.is_var_local(hir_local) {
                    let addr = self.map_local(*hir_local);
                    let ty = self.resolve_local_type(*hir_local);
                    self.push_inst(kestrel_mir::inst::InstKind::DestroyAddr { address: addr, ty });
                } else {
                    let val = self.map_local(*hir_local);
                    self.emit_destroy_value(val);
                }
            },
            HirStmt::Deinit { local: None, .. } => {
                // Unresolved deinit — nothing to do
            },
        }

        self.current_span = prev_span;
    }
}

fn stmt_span(stmt: &HirStmt) -> Span {
    match stmt {
        HirStmt::Let { span, .. } | HirStmt::Expr { span, .. } | HirStmt::Deinit { span, .. } => {
            span.clone()
        },
    }
}
