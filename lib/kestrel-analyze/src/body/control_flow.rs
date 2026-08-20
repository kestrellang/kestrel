//! # Shared Control-Flow Predicates
//!
//! Pure, diagnostic-free control-flow facts about a `HirBody` that more than
//! one analyzer needs. Nothing in here emits, reads `TypedBody`, or touches
//! `BodyContext` — `&HirBody` in, plain data out.
//!
//! It exists because five analyzers (`dead_code`, `exhaustive_return`,
//! `definite_assignment`, `move_tracking`, `guard`) each had their own private
//! copy of "does this loop body contain a break that exits it?", and the copies
//! drifted: all four ignored `break`'s label, none recursed into nested loops,
//! and only `dead_code`'s copy ever grew the `Sugar` arm. See
//! `docs/fragility/G8-G9-G10/`.

use kestrel_hir::{HirBlock, HirBody, HirExpr, HirExprId, HirStmt, HirStmtId, label_selects_loop};

/// Does `block` — the body of a loop labeled `target` — contain a `break` that
/// exits *that* loop?
///
/// Answers the question every loop-divergence check actually wants: "can this
/// loop fall through to its successor?" A loop with no such break is infinite
/// (or only leaves via `return`), so its successor is unreachable.
///
/// Label resolution follows `kestrel_hir::label_selects_loop`, matching MIR
/// lowering's `find_loop`:
/// - A `break` directly in `block` exits it whether labeled `target` or bare.
/// - A `break` inside a **nested** loop exits `block`'s loop only if it names
///   `target` explicitly — a bare `break` there belongs to the inner loop.
/// - A nested loop that reuses `target` as its own label *shadows* it, so
///   nothing below can name the outer loop any more.
/// - `continue` never exits a loop; closures are their own break scope.
pub(crate) fn block_contains_break_for(
    hir: &HirBody,
    block: &HirBlock,
    target: Option<&str>,
) -> bool {
    contains_break_in_block(hir, block, target, false)
}

/// `crossed` = "we have descended into a nested loop", i.e. we are no longer
/// directly inside the loop `target` names. It is not expressible through
/// `target` alone: an unlabeled `target` (`None`) still has to distinguish a
/// bare `break` at its own level (exits it) from one inside a nested loop
/// (exits the *inner* loop).
fn contains_break_in_block(
    hir: &HirBody,
    block: &HirBlock,
    target: Option<&str>,
    crossed: bool,
) -> bool {
    block
        .stmts
        .iter()
        .any(|&s| contains_break_in_stmt(hir, s, target, crossed))
        || block
            .tail_expr
            .is_some_and(|t| contains_break_in_expr(hir, t, target, crossed))
}

fn contains_break_in_stmt(
    hir: &HirBody,
    id: HirStmtId,
    target: Option<&str>,
    crossed: bool,
) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => contains_break_in_expr(hir, *expr, target, crossed),
        HirStmt::Let { value: Some(v), .. } => contains_break_in_expr(hir, *v, target, crossed),
        _ => false,
    }
}

fn contains_break_in_expr(
    hir: &HirBody,
    id: HirExprId,
    target: Option<&str>,
    crossed: bool,
) -> bool {
    match &hir.exprs[id] {
        HirExpr::Break { label, .. } => {
            if crossed {
                // Inside a nested loop: only an explicit label naming the
                // target reaches back out. A bare break belongs to the inner
                // loop, and an unlabeled target can never be named.
                target.is_some() && label.as_deref() == target
            } else {
                label_selects_loop(label.as_deref(), target)
            }
        },

        // `continue` restarts a loop, it never exits one.
        HirExpr::Continue { .. } => false,

        HirExpr::Loop {
            label: inner, body, ..
        } => {
            // An inner loop reusing the exact same label shadows the target:
            // no `break target` below this point can reach the outer loop.
            if target.is_some() && inner.is_some() && inner.as_deref() == target {
                return false;
            }
            contains_break_in_block(hir, body, target, true)
        },

        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            contains_break_in_block(hir, then_body, target, crossed)
                || else_body
                    .as_ref()
                    .is_some_and(|e| contains_break_in_block(hir, e, target, crossed))
        },
        HirExpr::Match { arms, .. } => arms
            .iter()
            .any(|arm| contains_break_in_expr(hir, arm.body, target, crossed)),
        HirExpr::Block { body, .. } => contains_break_in_block(hir, body, target, crossed),
        // Transparent wrapper — the desugared subtree is the real control flow.
        HirExpr::Sugar { inner, .. } => contains_break_in_expr(hir, *inner, target, crossed),

        // A closure body is a separate break scope; a `break` inside it cannot
        // exit a loop in the enclosing function.
        HirExpr::Closure { .. } => false,

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_hir::{HirLiteral, SugarKind};
    use kestrel_span::Span;

    /// Minimal HIR builder — the crate has no shared fixture helper and these
    /// shapes only need `Loop`/`Break`/`If`/`Block`/`Sugar`/`Closure` nodes.
    struct B {
        hir: HirBody,
    }

    impl B {
        fn new() -> Self {
            Self {
                hir: HirBody::empty(),
            }
        }

        fn sp() -> Span {
            Span::synthetic(0)
        }

        fn expr(&mut self, e: HirExpr) -> HirExprId {
            self.hir.exprs.alloc(e)
        }

        /// Wrap an expr as a single-statement block.
        fn block(&mut self, e: HirExprId) -> HirBlock {
            let s = self.hir.stmts.alloc(HirStmt::Expr {
                expr: e,
                span: Self::sp(),
            });
            HirBlock {
                stmts: vec![s],
                tail_expr: None,
            }
        }

        fn brk(&mut self, label: Option<&str>) -> HirExprId {
            self.expr(HirExpr::Break {
                label: label.map(str::to_string),
                span: Self::sp(),
            })
        }

        fn loop_(&mut self, label: Option<&str>, body: HirBlock) -> HirExprId {
            self.expr(HirExpr::Loop {
                label: label.map(str::to_string),
                body,
                span: Self::sp(),
            })
        }
    }

    #[test]
    fn unlabeled_break_directly_in_body() {
        let mut b = B::new();
        let brk = b.brk(None);
        let body = b.block(brk);
        assert!(block_contains_break_for(&b.hir, &body, None));
    }

    #[test]
    fn unlabeled_break_inside_nested_loop_does_not_escape() {
        // loop { loop { break; } }  — the break belongs to the inner loop.
        let mut b = B::new();
        let brk = b.brk(None);
        let inner_body = b.block(brk);
        let inner = b.loop_(None, inner_body);
        let outer_body = b.block(inner);
        assert!(!block_contains_break_for(&b.hir, &outer_body, None));
    }

    #[test]
    fn labeled_break_escapes_one_level() {
        // outer: loop { loop { break outer; } }
        let mut b = B::new();
        let brk = b.brk(Some("outer"));
        let inner_body = b.block(brk);
        let inner = b.loop_(None, inner_body);
        let outer_body = b.block(inner);
        assert!(block_contains_break_for(&b.hir, &outer_body, Some("outer")));
    }

    #[test]
    fn labeled_break_escapes_two_levels() {
        // The G9 shape: outer: loop { loop { loop { break outer; } } }
        let mut b = B::new();
        let brk = b.brk(Some("outer"));
        let l3_body = b.block(brk);
        let l3 = b.loop_(None, l3_body);
        let l2_body = b.block(l3);
        let l2 = b.loop_(None, l2_body);
        let outer_body = b.block(l2);
        assert!(block_contains_break_for(&b.hir, &outer_body, Some("outer")));
    }

    #[test]
    fn inner_loop_reusing_the_label_shadows_it() {
        // outer: loop { outer: loop { break outer; } }
        // The break names the *inner* loop, so the outer one has no exit.
        let mut b = B::new();
        let brk = b.brk(Some("outer"));
        let inner_body = b.block(brk);
        let inner = b.loop_(Some("outer"), inner_body);
        let outer_body = b.block(inner);
        assert!(!block_contains_break_for(
            &b.hir,
            &outer_body,
            Some("outer")
        ));
    }

    #[test]
    fn labeled_break_inside_closure_does_not_escape() {
        let mut b = B::new();
        let brk = b.brk(Some("outer"));
        let closure_body = b.block(brk);
        let closure = b.expr(HirExpr::Closure {
            params: Vec::new(),
            body: closure_body,
            span: B::sp(),
        });
        let outer_body = b.block(closure);
        assert!(!block_contains_break_for(
            &b.hir,
            &outer_body,
            Some("outer")
        ));
    }

    #[test]
    fn break_behind_a_sugar_wrapper_is_seen() {
        let mut b = B::new();
        let brk = b.brk(None);
        let sugar = b.expr(HirExpr::Sugar {
            kind: SugarKind::Try,
            inner: brk,
            span: B::sp(),
        });
        let body = b.block(sugar);
        assert!(block_contains_break_for(&b.hir, &body, None));
    }

    #[test]
    fn continue_is_not_an_exit() {
        let mut b = B::new();
        let cont = b.expr(HirExpr::Continue {
            label: None,
            span: B::sp(),
        });
        let body = b.block(cont);
        assert!(!block_contains_break_for(&b.hir, &body, None));
    }

    #[test]
    fn unlabeled_target_ignores_a_labeled_break_from_a_nested_loop() {
        // loop { loop { break outer; } } analyzed as the *unlabeled* outer
        // loop: `break outer` names some further-out loop, not this one.
        let mut b = B::new();
        let brk = b.brk(Some("outer"));
        let inner_body = b.block(brk);
        let inner = b.loop_(None, inner_body);
        let outer_body = b.block(inner);
        assert!(!block_contains_break_for(&b.hir, &outer_body, None));
    }

    #[test]
    fn break_inside_an_if_still_counts() {
        let mut b = B::new();
        let brk = b.brk(None);
        let then_body = b.block(brk);
        let cond = b.expr(HirExpr::Literal {
            value: HirLiteral::Integer(1),
            span: B::sp(),
        });
        let if_ = b.expr(HirExpr::If {
            condition: cond,
            then_body,
            else_body: None,
            span: B::sp(),
        });
        let body = b.block(if_);
        assert!(block_contains_break_for(&b.hir, &body, None));
    }
}
