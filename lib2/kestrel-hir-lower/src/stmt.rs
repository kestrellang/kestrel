//! Statement lowering: AstStmt → HirStmt.
//!
//! Handles let bindings (simple and destructuring), expression statements,
//! guard-let desugaring, and deinit statements.

use kestrel_ast::ast_body::*;
use kestrel_hir::body::*;
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

impl LowerCtx<'_> {
    /// Lower an AST statement to an HIR statement.
    pub fn lower_stmt(&mut self, body: &AstBody, id: StmtId) -> HirStmtId {
        let stmt = &body.stmts[id];
        match stmt {
            AstStmt::Let {
                is_mut,
                pattern,
                ty,
                value,
                span,
            } => self.lower_let_stmt(body, *is_mut, *pattern, ty.as_ref(), *value, span),

            AstStmt::Expr { expr, span } => {
                let lowered = self.lower_expr(body, *expr);
                self.alloc_stmt(HirStmt::Expr {
                    expr: lowered,
                    span: span.clone(),
                })
            }

            AstStmt::GuardLet {
                conditions,
                else_body,
                span,
            } => self.lower_guard_let(body, conditions, else_body, span),

            AstStmt::Deinit { name, span } => self.alloc_stmt(HirStmt::Deinit {
                name: name.clone(),
                span: span.clone(),
            }),
        }
    }

    /// Lower a let statement.
    /// Simple binding → HirStmt::Let with local.
    /// Complex pattern → temp local + match destructure.
    fn lower_let_stmt(
        &mut self,
        body: &AstBody,
        is_mut: bool,
        pattern: PatId,
        ty: Option<&kestrel_ast::AstType>,
        value: Option<ExprId>,
        span: &Span,
    ) -> HirStmtId {
        let lowered_ty = ty.map(|t| self.lower_type(t));
        let lowered_value = value.map(|v| self.lower_expr(body, v));

        // Check if pattern is a simple binding
        match &body.pats[pattern] {
            AstPat::Binding { name, .. } => {
                let local = self.define_local(name, is_mut, span.clone());
                self.alloc_stmt(HirStmt::Let {
                    local,
                    ty: lowered_ty,
                    value: lowered_value,
                    span: span.clone(),
                })
            }
            _ => {
                // Complex pattern: allocate temp, then destructure via match
                // Emits: { let $let_tmp = value; match $let_tmp { pattern => () } }
                let temp = self.define_local("$let_tmp", is_mut, span.clone());
                let let_stmt = self.alloc_stmt(HirStmt::Let {
                    local: temp,
                    ty: lowered_ty,
                    value: lowered_value,
                    span: span.clone(),
                });

                // Lower the pattern (this allocates locals for bindings within)
                let hir_pat = self.lower_pat(body, pattern);

                // Create a match expression to destructure:
                // match $let_tmp { pattern => () }
                let temp_ref = self.alloc_expr(HirExpr::Local(temp, span.clone()));
                let unit = self.alloc_expr(HirExpr::Tuple {
                    elements: Vec::new(),
                    span: span.clone(),
                });
                let match_expr = self.alloc_expr(HirExpr::Match {
                    scrutinee: temp_ref,
                    arms: vec![HirMatchArm {
                        pattern: hir_pat,
                        guard: None,
                        body: unit,
                    }],
                    span: span.clone(),
                });
                let match_stmt = self.alloc_stmt(HirStmt::Expr {
                    expr: match_expr,
                    span: span.clone(),
                });

                // Wrap both in a block so we return a single statement
                let block_expr = self.alloc_expr(HirExpr::Block {
                    body: HirBlock {
                        stmts: vec![let_stmt, match_stmt],
                        tail_expr: None,
                    },
                    span: span.clone(),
                });
                self.alloc_stmt(HirStmt::Expr {
                    expr: block_expr,
                    span: span.clone(),
                })
            }
        }
    }

    /// Lower a guard-let statement.
    /// Desugars to: if !conditions { else_body } with bindings in outer scope.
    fn lower_guard_let(
        &mut self,
        body: &AstBody,
        conditions: &[IfCondition],
        else_body: &AstBlock,
        span: &Span,
    ) -> HirStmtId {
        // Lower the else body (must diverge: return/break/continue/throw)
        let lowered_else = self.lower_block(body, else_body);

        // Build the condition check
        let condition_expr = self.lower_if_conditions(body, conditions, span);

        // Guard-let: if condition fails (is false / pattern doesn't match), run else
        // The bindings from let-conditions are defined in the current scope (not nested)
        let break_block = HirBlock {
            stmts: lowered_else.stmts,
            tail_expr: lowered_else.tail_expr,
        };

        let guard_expr = self.alloc_expr(HirExpr::If {
            condition: condition_expr,
            then_body: HirBlock {
                stmts: Vec::new(),
                tail_expr: None,
            },
            else_body: Some(break_block),
            span: span.clone(),
        });

        let stmt_id = self.alloc_stmt(HirStmt::Expr {
            expr: guard_expr,
            span: span.clone(),
        });
        // Mark this statement as originating from guard-let so the
        // guard_let_divergence analyzer can check the else block diverges.
        self.guard_let_stmts.push(stmt_id);
        stmt_id
    }
}
