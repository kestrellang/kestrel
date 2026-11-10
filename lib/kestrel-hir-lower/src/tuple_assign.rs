//! Tuple assignment: `(a, b) = (b, a + b)`.
//!
//! The right side is evaluated in full before any target is written, as in
//! Rust. It lowers to one irrefutable match whose pattern mirrors the target
//! tuple with hidden bindings, and whose arm writes each target in order:
//!
//! ```text
//! match (b, a + b) { ($t0, $t1) => { a = $t0; b = $t1; } }
//! ```
//!
//! Nested tuples nest the pattern (`((a, b), c) = …`) and a `_` target is a
//! wildcard that drops its part. Each write is an ordinary `Assign`, so the
//! assignment analyzer checks every target as it would a plain `x = v`.

use kestrel_hir::body::*;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ctx::LowerCtx;
use crate::syntax::{ExprSrc, expr_children, first_expr, unwrap_expr};

/// The target as a tuple expression, looking through `Expression` wrappers
/// and parentheses: `(a, b)` and `((a, b))` both qualify.
pub(crate) fn tuple_target(lhs: &ExprSrc) -> Option<SyntaxNode> {
    let mut node = unwrap_expr(lhs.inner().as_ref()?);
    while node.kind() == SyntaxKind::ExprGrouping {
        node = unwrap_expr(&first_expr(&node)?);
    }
    (node.kind() == SyntaxKind::ExprTuple).then_some(node)
}

impl LowerCtx<'_> {
    /// Lower `tuple = rhs` (see the module docs).
    pub(crate) fn lower_tuple_assign(
        &mut self,
        target: &SyntaxNode,
        rhs: &ExprSrc,
        span: &Span,
    ) -> HirExprId {
        let value = self.lower_expr_src(rhs);
        self.push_scope();
        let mut writes = Vec::new();
        let pattern = self.tuple_assign_pat(target, &mut writes);
        self.pop_scope();
        let stmts = writes
            .into_iter()
            .map(|(target, local, tspan)| {
                let value = self.alloc_expr(HirExpr::Local(local, tspan.clone()));
                let assign = self.alloc_expr(HirExpr::Assign {
                    target,
                    value,
                    span: tspan.clone(),
                });
                self.alloc_stmt(HirStmt::Expr {
                    expr: assign,
                    span: tspan,
                })
            })
            .collect();
        let body = self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts,
                tail_expr: None,
            },
            span: span.clone(),
        });
        self.alloc_expr(HirExpr::Match {
            scrutinee: value,
            arms: vec![HirMatchArm {
                pattern,
                guard: None,
                body,
            }],
            source: MatchSource::LetDestructure,
            span: span.clone(),
        })
    }

    /// The pattern for one target: a nested tuple recurses, `_` is a
    /// wildcard, anything else binds a hidden local and queues the write
    /// `target = local`.
    fn tuple_assign_pat(
        &mut self,
        node: &SyntaxNode,
        writes: &mut Vec<(HirExprId, kestrel_hir::LocalId, Span)>,
    ) -> HirPatId {
        let span = self.span(node);
        if let Some(inner) = tuple_target(&ExprSrc::Node(node.clone())) {
            let prefix = expr_children(&inner)
                .map(|e| self.tuple_assign_pat(&e, writes))
                .collect();
            return self.alloc_pat(HirPat::Tuple {
                prefix,
                has_rest: false,
                suffix: Vec::new(),
                span,
            });
        }
        let leaf = unwrap_expr(node);
        if matches!(
            leaf.kind(),
            SyntaxKind::ExprPlaceholder | SyntaxKind::ExprPlaceholderUse
        ) {
            return self.alloc_pat(HirPat::Wildcard { span });
        }
        let target = self.lower_expr(node);
        let local = self.define_local(&format!("$t{}", writes.len()), false, span.clone());
        writes.push((target, local, span.clone()));
        self.alloc_pat(HirPat::Binding {
            local,
            by_ref: None,
            span,
        })
    }
}
