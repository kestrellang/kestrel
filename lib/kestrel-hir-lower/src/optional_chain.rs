//! Optional chaining: `a?.b.c()`, `f?.(x)`, `a?.n = 1`, `a?.n += 1`.
//!
//! The parser wraps the maximal postfix chain holding a `?.` in
//! `ExprOptionalChain` and each `a?` in `ExprOptionalBind`. Lowering is
//! bottom-up: lowering a chain pushes a frame; each bind lowers its base,
//! records `(ProtocolCall(base, OptionalChain, …), $o)` in the frame and
//! stands for `Local($o)`; when the tail is done, the binds become nested
//! matches, last bind innermost:
//!
//! ```text
//! Sugar(OptionalChain,
//!   match s₁.chainBorrow() {
//!     .Some($o₁) => match s₂[$o₁].chainBorrow() { .Some($o₂) => tail, .None => .None },
//!     .None => .None })
//! ```
//!
//! Everything after a bind (later bases, call arguments, an assignment's
//! right side) is inside its `.Some` arm, so it runs only on success.
//!
//! The method is provisional: a place base gets `chainBorrow`, which
//! inference upgrades to `chainMutate` when the chain mutates the payload
//! (`TypedBody::chain_modes`); an rvalue base gets `chainTake` — a temporary
//! is owned, so handing its payload over is never observable.
//! See `docs/plans/optional-chaining/optional-chaining-design.md`.

use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;

use crate::ctx::LowerCtx;
use crate::desugar::is_place_syntax;
use crate::syntax::{ExprSrc, first_expr};

/// One `?.` bind of the chain being lowered.
pub(crate) struct ChainBind {
    /// `ProtocolCall(base, OptionalChain, chainBorrow | chainTake)`.
    pub scrutinee: HirExprId,
    /// `$o`: the payload the rest of the chain continues on.
    pub binder: LocalId,
}

impl LowerCtx<'_> {
    /// An `ExprOptionalChain`. `tail` lowers the chain's expression (the
    /// node's child) into the innermost arm body: the expression itself, or
    /// an assignment through it.
    pub(crate) fn lower_optional_chain(
        &mut self,
        chain: &SyntaxNode,
        span: &Span,
        tail: impl FnOnce(&mut Self, &ExprSrc) -> HirExprId,
    ) -> HirExprId {
        let inner = ExprSrc::or_error(first_expr(chain), span);
        self.chain_frames.push(Vec::new());
        self.push_scope();
        let body = tail(self, &inner);
        self.pop_scope();
        let binds = self.chain_frames.pop().unwrap_or_default();
        self.build_chain(binds, body, span)
    }

    /// An `ExprOptionalBind` (`a?`): records the bind and stands for its
    /// payload.
    pub(crate) fn lower_optional_bind(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let base_src = ExprSrc::or_error(first_expr(node), span);
        let base = self.lower_expr_src(&base_src);
        // The parser wraps every bind in a chain; without one, lower the
        // operand alone rather than invent a match.
        if self.chain_frames.is_empty() {
            return base;
        }
        let Some(protocol) = self.resolve_builtin(Builtin::OptionalChainProtocol) else {
            self.ctx.accumulate(
                Diagnostic::error()
                    .with_code("E157")
                    .with_message("`?.` requires the `OptionalChain` protocol")
                    .with_labels(vec![Label::primary(span.file_id, span.range())])
                    .with_notes(vec!["is the standard library imported?".to_string()]),
            );
            return self.alloc_expr(HirExpr::Error { span: span.clone() });
        };
        let take = !base_src.inner().is_some_and(|n| is_place_syntax(&n));
        let method = if take { "chainTake" } else { "chainBorrow" };
        let scrutinee = self.alloc_expr(HirExpr::ProtocolCall {
            from_operator: false,
            receiver: base,
            protocol,
            method: HirName::name(method),
            type_args: None,
            args: Vec::new(),
            span: span.clone(),
        });
        // A taken payload is an owned temporary; the chain may mutate it.
        let binder = self.define_local("$o", take, span.clone());
        if let Some(frame) = self.chain_frames.last_mut() {
            frame.push(ChainBind { scrutinee, binder });
        }
        self.alloc_expr(HirExpr::Local(binder, span.clone()))
    }

    /// Nest `body` in one match per bind, the last bind innermost.
    fn build_chain(&mut self, binds: Vec<ChainBind>, body: HirExprId, span: &Span) -> HirExprId {
        if binds.is_empty() {
            return body;
        }
        let mut body = body;
        for bind in binds.into_iter().rev() {
            let binding = self.alloc_pat(HirPat::Binding {
                local: bind.binder,
                by_ref: None,
                span: span.clone(),
            });
            let some = self.alloc_pat(HirPat::ImplicitVariant {
                name: HirName::name("Some"),
                args: vec![HirPatArg {
                    label: None,
                    pattern: binding,
                }],
                span: span.clone(),
            });
            let none = self.alloc_pat(HirPat::ImplicitVariant {
                name: HirName::name("None"),
                args: Vec::new(),
                span: span.clone(),
            });
            let none_value = self.alloc_expr(HirExpr::ImplicitMember {
                owner: None,
                name: HirName::name("None"),
                args: None,
                span: span.clone(),
            });
            body = self.alloc_expr(HirExpr::Match {
                scrutinee: bind.scrutinee,
                arms: vec![
                    HirMatchArm {
                        pattern: some,
                        guard: None,
                        body,
                    },
                    HirMatchArm {
                        pattern: none,
                        guard: None,
                        body: none_value,
                    },
                ],
                source: MatchSource::OptionalChain,
                span: span.clone(),
            });
        }
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::OptionalChain,
            inner: body,
            span: span.clone(),
        })
    }
}
