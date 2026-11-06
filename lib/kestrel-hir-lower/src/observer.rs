//! The synthesized setter of an observed stored property
//! (`var x: T = v { willSet { … } didSet { … } }`, see
//! `kestrel_ast_builder::ObserverSetter`). Its body is built here from the
//! observer clauses instead of from a written block:
//!
//! ```text
//! { willSet body }           // sees `newValue`
//! let oldValue = self.x      // only when `didSet` reads `oldValue`
//! self.x = newValue          // a direct store: MIR lowering never routes
//!                            // this setter's own writes back through it
//! { didSet body }            // sees `oldValue`
//! ```

use kestrel_ast_builder::Name;
use kestrel_hir::body::{HirBlock, HirExpr, HirName, HirStmt, HirStmtId};
use kestrel_syntax_tree::SyntaxNode;
use kestrel_syntax_tree::ast::{self, AstNode};

use crate::ctx::LowerCtx;
use crate::syntax::block_syntax;

impl LowerCtx<'_> {
    /// Lower the observer setter body. `accessors` is the field's
    /// `PropertyAccessors` node; the receiver and `newValue` are already
    /// defined as params (`self` first, `newValue` second).
    pub(crate) fn lower_observer_body(&mut self, accessors: &SyntaxNode) -> HirBlock {
        let mut stmts = Vec::new();
        let Some(acc) = ast::PropertyAccessors::cast(accessors.clone()) else {
            return HirBlock {
                stmts,
                tail_expr: None,
            };
        };
        let field_name = self
            .ctx
            .parent_of(self.owner)
            .and_then(|f| self.ctx.get::<Name>(f).map(|n| n.0.clone()));
        let (Some(field_name), [self_local, new_value]) = (field_name, self.params.as_slice())
        else {
            // No receiver (a static/global observer, rejected by E638):
            // lower nothing rather than guess at the storage.
            return HirBlock {
                stmts,
                tail_expr: None,
            };
        };
        let (self_local, new_value) = (*self_local, *new_value);
        let span = self.span(accessors);
        let field = |this: &mut Self| {
            let base = this.alloc_expr(HirExpr::Local(self_local, span.clone()));
            this.alloc_expr(HirExpr::Field {
                base,
                name: HirName::name(field_name.clone()),
                span: span.clone(),
            })
        };

        if let Some(body) = acc.will_set().and_then(|c| c.code_block()) {
            stmts.push(self.observer_block_stmt(body.syntax()));
        }

        // `oldValue` is in scope only for `didSet`. The block is lowered
        // first so name resolution decides whether it reads `oldValue`:
        // only then is the old value copied out before the store (it costs
        // a copy, and a non-Copyable value can't be copied).
        let old_value = self.define_local("oldValue", false, span.clone());
        let did_set = acc
            .did_set()
            .and_then(|c| c.code_block())
            .map(|body| self.observer_block_stmt(body.syntax()));
        if self.reads_local(old_value) {
            let value = field(self);
            stmts.push(self.alloc_stmt(HirStmt::Let {
                local: old_value,
                ty: None,
                value: Some(value),
                span: span.clone(),
            }));
        }

        let target = field(self);
        let value = self.alloc_expr(HirExpr::Local(new_value, span.clone()));
        let assign = self.alloc_expr(HirExpr::Assign {
            target,
            value,
            span: span.clone(),
        });
        stmts.push(self.alloc_stmt(HirStmt::Expr {
            expr: assign,
            span: span.clone(),
        }));
        stmts.extend(did_set);
        HirBlock {
            stmts,
            tail_expr: None,
        }
    }

    /// Does any lowered expression read `local`?
    fn reads_local(&self, local: kestrel_hir::LocalId) -> bool {
        self.exprs
            .iter()
            .any(|(_, e)| matches!(e, HirExpr::Local(l, _) if *l == local))
    }

    /// One observer clause's block, as a statement in its own scope.
    fn observer_block_stmt(&mut self, block: &SyntaxNode) -> HirStmtId {
        let span = self.span(block);
        let body = self.lower_block(&block_syntax(block, self.file_id));
        let expr = self.alloc_expr(HirExpr::Block {
            body,
            span: span.clone(),
        });
        self.alloc_stmt(HirStmt::Expr { expr, span })
    }
}
