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
                if let Some(init_expr) = value {
                    let init_val = self.lower_expr(*init_expr);
                    if is_var {
                        // var locals are stack-allocated: Uninit + StoreInit.
                        // local_map holds the address; reads go through Load,
                        // field assignments use FieldAddr directly.
                        let ty = self.resolve_local_type(*local);
                        let addr = self.emit_uninit(ty);
                        self.emit_store_init(addr, init_val);
                        self.local_map.insert(*local, addr);
                        self.var_locals.insert(*local);
                        self.track_var(addr, ty);
                    } else {
                        self.local_map.insert(*local, init_val);
                    }
                } else if is_var {
                    // var with no initializer: allocate stack slot, leave uninit.
                    let ty = self.resolve_local_type(*local);
                    let addr = self.emit_uninit(ty);
                    self.local_map.insert(*local, addr);
                    self.var_locals.insert(*local);
                    self.track_var(addr, ty);
                }
            }
            HirStmt::Expr { expr, .. } => {
                let val = self.lower_expr(*expr);
                // If the expression produced an @owned value that nobody
                // consumes, destroy it immediately to avoid leaks.
                let ownership = self.body.value(val).ownership;
                if ownership == kestrel_mir_3::value::Ownership::Owned {
                    // The value is already tracked in scope by the expr lowering.
                    // Nothing extra to do — scope cleanup will handle it.
                }
            }
            HirStmt::Deinit {
                local: Some(hir_local),
                ..
            } => {
                if self.var_locals.contains(hir_local) {
                    // var local: destroy the value at the address, not the address itself
                    let addr = self.map_local(*hir_local);
                    let ty = self.resolve_local_type(*hir_local);
                    self.push_inst(kestrel_mir_3::inst::InstKind::DestroyAddr { address: addr, ty });
                } else {
                    let val = self.map_local(*hir_local);
                    self.emit_destroy_value(val);
                }
            }
            HirStmt::Deinit { local: None, .. } => {
                // Unresolved deinit — nothing to do
            }
        }

        self.current_span = prev_span;
    }
}

fn stmt_span(stmt: &HirStmt) -> Span {
    match stmt {
        HirStmt::Let { span, .. } | HirStmt::Expr { span, .. } | HirStmt::Deinit { span, .. } => {
            span.clone()
        }
    }
}
