//! The synthesized setter of an observed stored property
//! (`var x: T = v { willSet { … } didSet { … } }`, see
//! `kestrel_ast_builder::ObserverSetter`). Its body is built here from the
//! observer clauses instead of from a written block:
//!
//! ```text
//! { willSet body }           // sees `newValue`
//! let oldValue = self.x      // only when `didSet` names `oldValue`
//! self.x = newValue          // a direct store: MIR lowering never routes
//!                            // this setter's own writes back through it
//! { didSet body }            // sees `oldValue`
//! ```

use kestrel_ast_builder::Name;
use kestrel_hir::body::{HirBlock, HirExpr, HirName, HirStmt, HirStmtId};
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

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

        // Bound after `willSet`, so only `didSet` sees it.
        let did_set = acc.did_set().and_then(|c| c.code_block());
        if did_set
            .as_ref()
            .is_some_and(|b| names_old_value(b.syntax()))
        {
            let value = field(self);
            let local = self.define_local("oldValue", false, span.clone());
            stmts.push(self.alloc_stmt(HirStmt::Let {
                local,
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

        if let Some(body) = did_set {
            stmts.push(self.observer_block_stmt(body.syntax()));
        }
        HirBlock {
            stmts,
            tail_expr: None,
        }
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

/// Does a `didSet` body name `oldValue`? Only then is the old value copied
/// out before the store (it costs a copy, and a non-Copyable value can't be).
fn names_old_value(block: &SyntaxNode) -> bool {
    block
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .any(|t| t.kind() == SyntaxKind::Identifier && t.text() == "oldValue")
}
