//! Control flow lowering — if, loop, break, continue.

use kestrel_hir::body::{
    HirBlock, HirExpr, HirExprId, HirPat, HirPatId, HirStmt, HirStmtId,
};
use kestrel_mir_2::{Immediate, LocalId, Operand, Place, ScopeId};

use super::{BodyCtx, LoopInfo};

impl BodyCtx<'_, '_> {
    pub fn lower_if(
        &mut self,
        expr_id: HirExprId,
        condition: HirExprId,
        then_body: &HirBlock,
        else_body: Option<&HirBlock>,
    ) -> Operand {
        let cond_val = self.lower_expr(condition);
        let result_ty = self.resolve_expr_type(expr_id);

        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();

        let result_local = self.fresh_temp(result_ty);

        self.emit_branch(cond_val, then_block, else_block);

        // Then branch
        self.switch_to(then_block);
        let then_val = self.lower_hir_block(then_body);
        if !self.is_terminated() {
            self.emit_value_transfer(Place::local(result_local), then_val, result_ty);
            self.emit_jump(merge_block);
        }

        // Else branch
        self.switch_to(else_block);
        if let Some(else_body) = else_body {
            let else_val = self.lower_hir_block(else_body);
            if !self.is_terminated() {
                self.emit_value_transfer(Place::local(result_local), else_val, result_ty);
                self.emit_jump(merge_block);
            }
        } else {
            self.emit_assign_const(Place::local(result_local), Immediate::unit());
            self.emit_jump(merge_block);
        }

        self.switch_to(merge_block);
        Operand::Place(Place::local(result_local))
    }

    pub fn lower_loop(&mut self, body: &HirBlock, label: Option<&str>) -> Operand {
        let header_block = self.new_block();
        let exit_block = self.new_block();

        // Register loop-scoped locals for drop elaboration
        let scoped_locals = self.collect_block_locals(body);
        for &local in &scoped_locals {
            self.body.local_scopes.insert(
                local,
                ScopeId::Loop {
                    header: header_block,
                    exit: exit_block,
                },
            );
        }

        if !self.is_terminated() {
            self.emit_jump(header_block);
        }

        self.loop_stack.push(LoopInfo {
            header_block,
            exit_block,
            label: label.map(|s| s.to_string()),
        });

        // Emit ScopeLive markers at loop header
        self.switch_to(header_block);
        for &local in &scoped_locals {
            self.emit_scope_live(local);
        }

        let _ = self.lower_hir_block(body);

        // Back-edge
        if !self.is_terminated() {
            self.emit_jump(header_block);
        }

        self.loop_stack.pop();
        self.switch_to(exit_block);
        Operand::Const(Immediate::unit())
    }

    pub fn lower_break(&mut self, label: Option<&str>) -> Operand {
        if let Some(exit) = self.find_loop(label).map(|l| l.exit_block) {
            self.emit_jump(exit);
        }
        Operand::Const(Immediate::unit())
    }

    pub fn lower_continue(&mut self, label: Option<&str>) -> Operand {
        if let Some(header) = self.find_loop(label).map(|l| l.header_block) {
            self.emit_jump(header);
        }
        Operand::Const(Immediate::unit())
    }

    fn find_loop(&self, label: Option<&str>) -> Option<&LoopInfo> {
        match label {
            Some(label) => self
                .loop_stack
                .iter()
                .rev()
                .find(|l| l.label.as_deref() == Some(label)),
            None => self.loop_stack.last(),
        }
    }

    // === Scope local collection (for loop drop elaboration) ===

    fn collect_block_locals(&self, block: &HirBlock) -> Vec<LocalId> {
        let mut locals = Vec::new();
        self.collect_block_locals_inner(block, &mut locals);
        locals
    }

    fn collect_block_locals_inner(&self, block: &HirBlock, locals: &mut Vec<LocalId>) {
        for &stmt_id in &block.stmts {
            match &self.hir.stmts[stmt_id] {
                HirStmt::Let { local, value, .. } => {
                    locals.push(self.map_local(*local));
                    if let Some(v) = value {
                        self.collect_expr_pattern_locals(*v, locals);
                    }
                }
                HirStmt::Expr { expr, .. } => {
                    self.collect_expr_pattern_locals(*expr, locals);
                }
                _ => {}
            }
        }
        if let Some(tail) = block.tail_expr {
            self.collect_expr_pattern_locals(tail, locals);
        }
    }

    fn collect_expr_pattern_locals(&self, expr_id: HirExprId, locals: &mut Vec<LocalId>) {
        match &self.hir.exprs[expr_id] {
            HirExpr::Match {
                scrutinee, arms, ..
            } => {
                self.collect_expr_pattern_locals(*scrutinee, locals);
                for arm in arms {
                    self.collect_pat_locals(arm.pattern, locals);
                    if let Some(guard) = arm.guard {
                        self.collect_expr_pattern_locals(guard, locals);
                    }
                    self.collect_expr_pattern_locals(arm.body, locals);
                }
            }
            HirExpr::Sugar { inner, .. } => {
                self.collect_expr_pattern_locals(*inner, locals);
            }
            HirExpr::Block { body, .. } => {
                self.collect_block_locals_inner(body, locals);
            }
            HirExpr::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.collect_expr_pattern_locals(*condition, locals);
                self.collect_block_locals_inner(then_body, locals);
                if let Some(else_b) = else_body {
                    self.collect_block_locals_inner(else_b, locals);
                }
            }
            HirExpr::Call { callee, args, .. } => {
                self.collect_expr_pattern_locals(*callee, locals);
                for arg in args {
                    self.collect_expr_pattern_locals(arg.value, locals);
                }
            }
            HirExpr::MethodCall { receiver, args, .. }
            | HirExpr::ProtocolCall { receiver, args, .. } => {
                self.collect_expr_pattern_locals(*receiver, locals);
                for arg in args {
                    self.collect_expr_pattern_locals(arg.value, locals);
                }
            }
            HirExpr::Assign { target, value, .. } => {
                self.collect_expr_pattern_locals(*target, locals);
                self.collect_expr_pattern_locals(*value, locals);
            }
            HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
                for &e in elements {
                    self.collect_expr_pattern_locals(e, locals);
                }
            }
            HirExpr::Dict { entries, .. } => {
                for entry in entries {
                    self.collect_expr_pattern_locals(entry.key, locals);
                    self.collect_expr_pattern_locals(entry.value, locals);
                }
            }
            HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
                self.collect_expr_pattern_locals(*base, locals);
            }
            HirExpr::Return { value, .. } => {
                if let Some(v) = value {
                    self.collect_expr_pattern_locals(*v, locals);
                }
            }
            HirExpr::ImplicitMember { args, .. } => {
                if let Some(call_args) = args {
                    for arg in call_args {
                        self.collect_expr_pattern_locals(arg.value, locals);
                    }
                }
            }
            HirExpr::Closure { .. } | HirExpr::Loop { .. } => {}
            HirExpr::Literal { .. }
            | HirExpr::Local(..)
            | HirExpr::Def(..)
            | HirExpr::OverloadSet { .. }
            | HirExpr::Break { .. }
            | HirExpr::Continue { .. }
            | HirExpr::Error { .. } => {}
        }
    }

    fn collect_pat_locals(&self, pat_id: HirPatId, locals: &mut Vec<LocalId>) {
        match &self.hir.pats[pat_id] {
            HirPat::Binding { local, .. } => {
                locals.push(self.map_local(*local));
            }
            HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
                for arg in args {
                    self.collect_pat_locals(arg.pattern, locals);
                }
            }
            HirPat::Tuple {
                prefix, suffix, ..
            } => {
                for &p in prefix.iter().chain(suffix.iter()) {
                    self.collect_pat_locals(p, locals);
                }
            }
            HirPat::Struct { fields, .. } => {
                for field in fields {
                    if let Some(pat) = field.pattern {
                        self.collect_pat_locals(pat, locals);
                    }
                }
            }
            HirPat::Array {
                prefix,
                rest,
                suffix,
                ..
            } => {
                for &p in prefix.iter().chain(suffix.iter()) {
                    self.collect_pat_locals(p, locals);
                }
                if let Some(Some(rest_local)) = rest {
                    locals.push(self.map_local(*rest_local));
                }
            }
            HirPat::Or { alternatives, .. } => {
                for &alt in alternatives {
                    self.collect_pat_locals(alt, locals);
                }
            }
            HirPat::At {
                binding,
                subpattern,
                ..
            } => {
                locals.push(self.map_local(*binding));
                self.collect_pat_locals(*subpattern, locals);
            }
            HirPat::Wildcard { .. }
            | HirPat::Literal { .. }
            | HirPat::Range { .. }
            | HirPat::Error { .. } => {}
        }
    }
}
