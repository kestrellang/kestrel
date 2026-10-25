//! Placeholder arguments (`f(_, 1)`, `map(_.name)`) and the pipe operator
//! (`x |> f`, `x |> f(_, 1)`).
//!
//! Both are pure syntax rewrites into constructs that already exist:
//!
//! - a **hole** (`_` as a whole argument) makes the nearest call a closure
//!   with one parameter per hole: `f(_, 1)` is `{ ($p0) in f($p0, 1) }`;
//! - an **argument placeholder** (`_` inside a larger argument) makes that
//!   argument a one-parameter closure, every `_` in it the same parameter:
//!   `map(_.a + _.b)` is `map({ ($p0) in $p0.a + $p0.b })`;
//! - a **pipe** fills every `_` in its right-hand call's top-level arguments
//!   with the left value and builds no closure (`x |> f(_, 1)` is `f(x, 1)`);
//!   any other right side is called with the left value (`x |> f` is `f(x)`).
//!
//! The boundary of a `_` is syntactic: every argument list opens a new one
//! and closure literals close them all (the parser reports a `_` outside any
//! argument as E811). See `docs/plans/placeholder-arguments/` and
//! `docs/plans/pipe-operator/`.

use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ctx::LowerCtx;
use crate::desugar::is_place_syntax;
use crate::syntax::{ArgSyntax, ExprSrc, arguments};

/// What a `_` lowers to.
#[derive(Clone)]
pub(crate) enum PlaceholderFill {
    /// A synthesized closure parameter (`$p0`) or a pipe's `$pipe` temporary.
    Local(LocalId),
    /// A piped place (`buf`, `self.items`), lowered again at each use so its
    /// borrows and access modes are exactly those of the hand-written call.
    Place(SyntaxNode),
}

/// How a call's arguments treat `_`.
#[derive(Clone, Copy)]
pub(crate) enum ArgFill<'a> {
    /// Holes are filled by these closure parameters, in order. An
    /// argument placeholder becomes its own closure.
    Holes(&'a [LocalId]),
    /// No holes; an argument placeholder becomes its own closure.
    Closures,
    /// A pipe's right-hand call: every top-level `_` is the piped value.
    Pipe(&'a PlaceholderFill),
}

/// `_` as the whole argument.
pub(crate) fn is_hole(value: &ExprSrc) -> bool {
    value
        .inner()
        .is_some_and(|n| n.kind() == SyntaxKind::ExprPlaceholder)
}

/// Does `node` hold a `_` that belongs to the argument it is in? Nested
/// argument lists and closure literals are boundaries: their `_`s belong to
/// them.
fn contains_placeholder_use(node: &SyntaxNode) -> bool {
    if node.kind() == SyntaxKind::ExprPlaceholderUse {
        return true;
    }
    node.children()
        .filter(|c| !matches!(c.kind(), SyntaxKind::ArgumentList | SyntaxKind::ExprClosure))
        .any(|c| contains_placeholder_use(&c))
}

fn has_placeholder_use(value: &ExprSrc) -> bool {
    value.node().is_some_and(contains_placeholder_use)
}

impl LowerCtx<'_> {
    /// Lower call arguments, treating `_` according to `fill`.
    pub(crate) fn lower_call_args_filling(
        &mut self,
        args: &[ArgSyntax],
        fill: ArgFill<'_>,
    ) -> Vec<HirCallArg> {
        let mut holes = 0;
        args.iter()
            .map(|arg| {
                let value = if is_hole(&arg.value) {
                    let hole = holes;
                    holes += 1;
                    self.lower_hole(&arg.value, fill, hole)
                } else if has_placeholder_use(&arg.value) {
                    match fill {
                        ArgFill::Pipe(piped) => self.with_placeholder(piped.clone(), |this| {
                            this.lower_expr_src(&arg.value)
                        }),
                        ArgFill::Holes(_) | ArgFill::Closures => {
                            self.lower_argument_placeholder(&arg.value)
                        },
                    }
                } else {
                    self.lower_expr_src(&arg.value)
                };
                HirCallArg {
                    label: arg.label.clone(),
                    value,
                }
            })
            .collect()
    }

    /// The `index`th hole of a call.
    fn lower_hole(&mut self, value: &ExprSrc, fill: ArgFill<'_>, index: usize) -> HirExprId {
        let Some(node) = value.inner() else {
            return self.lower_expr_src(value);
        };
        let span = self.span(&node);
        let id = match fill {
            ArgFill::Holes(params) => self.alloc_expr(HirExpr::Local(params[index], span)),
            ArgFill::Pipe(piped) => self.lower_fill(piped, &span),
            // Every caller with holes passes `Holes` or `Pipe`.
            ArgFill::Closures => self.alloc_expr(HirExpr::Error { span }),
        };
        self.source_map.record_expr(&node, id);
        id
    }

    /// `_` inside an argument: the innermost frame's value. Without a frame
    /// the parser has already reported E811.
    pub(crate) fn lower_placeholder_use(&mut self, span: &Span) -> HirExprId {
        match self.placeholder_frames.last().cloned() {
            Some(fill) => self.lower_fill(&fill, span),
            None => self.alloc_expr(HirExpr::Error { span: span.clone() }),
        }
    }

    fn lower_fill(&mut self, fill: &PlaceholderFill, span: &Span) -> HirExprId {
        match fill {
            PlaceholderFill::Local(local) => self.alloc_expr(HirExpr::Local(*local, span.clone())),
            PlaceholderFill::Place(node) => self.lower_expr(node),
        }
    }

    /// Run `f` with `_` standing for `fill`.
    fn with_placeholder(
        &mut self,
        fill: PlaceholderFill,
        f: impl FnOnce(&mut Self) -> HirExprId,
    ) -> HirExprId {
        self.placeholder_frames.push(fill);
        let id = f(self);
        self.placeholder_frames.pop();
        id
    }

    /// An argument with `_` inside it: `{ ($p0) in <arg with _ ↦ $p0> }`.
    fn lower_argument_placeholder(&mut self, value: &ExprSrc) -> HirExprId {
        let span = match value.inner() {
            Some(node) => self.span(&node),
            None => return self.lower_expr_src(value),
        };
        self.lower_placeholder_closure(1, &span, |this, params| {
            this.with_placeholder(PlaceholderFill::Local(params[0]), |this| {
                this.lower_expr_src(value)
            })
        })
    }

    /// `{ ($p0, …, $p(count-1)) in body }`, wrapped as placeholder sugar.
    /// `body` is lowered in the closure's scope, with the parameters.
    pub(crate) fn lower_placeholder_closure(
        &mut self,
        count: usize,
        span: &Span,
        body: impl FnOnce(&mut Self, &[LocalId]) -> HirExprId,
    ) -> HirExprId {
        self.push_scope();
        let params: Vec<LocalId> = (0..count)
            .map(|i| self.define_local(&format!("$p{i}"), false, span.clone()))
            .collect();
        // Like a closure literal, the body is its own function: `break` in
        // it cannot target an enclosing loop.
        let saved_loops = std::mem::take(&mut self.loop_labels);
        let tail = body(self, &params);
        self.loop_labels = saved_loops;
        self.pop_scope();

        let closure = self.alloc_expr(HirExpr::Closure {
            params: params
                .into_iter()
                .map(|local| HirClosureParam {
                    local,
                    ty: None,
                    pattern: None,
                    is_mut: false,
                })
                .collect(),
            body: HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(tail),
            },
            span: span.clone(),
        });
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::Placeholder,
            inner: closure,
            span: span.clone(),
        })
    }

    /// `lhs |> rhs`.
    ///
    /// A place `lhs` is substituted at each use (borrowed as if written by
    /// hand); any other `lhs` is evaluated first, into `$pipe`. Then:
    /// - `rhs` a call with `_` in its top-level arguments: each `_` is `lhs`;
    /// - `rhs` a path (`f`, `Type.parse`, `obj.method`): called with `lhs`,
    ///   exactly like `rhs(lhs)` written out;
    /// - anything else (`makeAdder(1)`, a closure literal): evaluated, then
    ///   called with `lhs`.
    pub(crate) fn lower_pipe(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let (lhs, rhs) = self.operands(node, span);
        self.push_scope();
        let place = lhs
            .inner()
            .filter(|n| is_place_syntax(n) && !contains_placeholder_use(n));
        let mut stmts = Vec::new();
        let fill = match place {
            Some(_) => PlaceholderFill::Place(lhs.node().cloned().expect("place has a node")),
            None => {
                let value = self.lower_expr_src(&lhs);
                let local = self.define_local("$pipe", false, span.clone());
                stmts.push(self.alloc_stmt(HirStmt::Let {
                    local,
                    ty: None,
                    value: Some(value),
                    span: span.clone(),
                }));
                PlaceholderFill::Local(local)
            },
        };

        let rhs_node = rhs.inner();
        let call = match rhs_node.as_ref().map(|n| (n.kind(), n)) {
            Some((SyntaxKind::ExprCall, call))
                if call_has_top_level_placeholder(call, self.file_id) =>
            {
                self.lower_call(call, span, Some(&fill))
            },
            Some((SyntaxKind::ExprPath, _)) => {
                let arg = self.lower_fill(&fill, span);
                let args = vec![HirCallArg {
                    label: None,
                    value: arg,
                }];
                self.lower_call_with_args(&rhs, args, span)
            },
            _ => {
                let arg = self.lower_fill(&fill, span);
                let callee = self.lower_callee(&rhs);
                self.alloc_expr(HirExpr::Call {
                    callee,
                    args: vec![HirCallArg {
                        label: None,
                        value: arg,
                    }],
                    span: span.clone(),
                })
            },
        };
        self.pop_scope();

        let inner = if stmts.is_empty() {
            call
        } else {
            self.alloc_expr(HirExpr::Block {
                body: HirBlock {
                    stmts,
                    tail_expr: Some(call),
                },
                span: span.clone(),
            })
        };
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::Pipe,
            inner,
            span: span.clone(),
        })
    }
}

/// A call with a hole or a placeholder use among its own arguments.
fn call_has_top_level_placeholder(call: &SyntaxNode, file_id: usize) -> bool {
    call.children()
        .filter(|c| c.kind() == SyntaxKind::ArgumentList)
        .flat_map(|list| arguments(&list, file_id))
        .any(|arg| is_hole(&arg.value) || has_placeholder_use(&arg.value))
}
