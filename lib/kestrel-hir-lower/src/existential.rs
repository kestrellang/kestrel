//! `is` patterns and the expressions built on them (existentials design §8,
//! §8b).
//!
//! The `is` pattern is the one primitive: `HirPat::TypeTest`. The expression
//! forms desugar onto it, so typing, borrow tracking and exhaustiveness are
//! the ordinary match machinery:
//!
//! - `x is T` → `match x { is T => true, _ => false }` (swapped for `is not`);
//!   the arms are bool literals, so the result types like `true`.
//! - `x as? T` → `{ let $cast: Optional[&T] = match x { $hit is T =>
//!   .Some($hit), _ => .None }; $cast }`.
//! - `consuming x as? T` → `{ let $boxed = x; let $result: Result[T, _] =
//!   if $boxed is T { .Ok(TakePayload($boxed)) } else { .Err($boxed) };
//!   $result }`.

use kestrel_ast::AstType;
use kestrel_ast_builder::ast_type::ast_type_from_cst;
use kestrel_hir::body::*;
use kestrel_hir::ty::HirTy;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ctx::LowerCtx;
use crate::syntax::{ExprSrc, first_expr, has_token, token};

impl LowerCtx<'_> {
    /// The target type written after `is` / `as?`, as written and lowered.
    fn type_test_target(&mut self, node: &SyntaxNode, span: &Span) -> (AstType, HirTy) {
        let ast = node
            .children()
            .find(|c| c.kind().is_type())
            .and_then(|c| ast_type_from_cst(&c, self.file_id))
            .unwrap_or_else(|| AstType::Inferred(span.clone()));
        let hir = match &ast {
            AstType::Inferred(_) => HirTy::Error(span.clone()),
            written => self.lower_type(written),
        };
        (ast, hir)
    }

    /// `[var] [c] is T [{ fields }]`.
    pub(crate) fn lower_is_pattern(&mut self, node: &SyntaxNode, span: Span) -> HirPatId {
        // E153 is syntactic: an `is` binding borrows the payload.
        if has_token(node, SyntaxKind::Var) {
            self.ctx.accumulate(
                kestrel_reporting::Diagnostic::error()
                    .with_code("E153")
                    .with_message(
                        "an 'is' pattern binds a borrow of the payload, so it cannot be 'var'",
                    )
                    .with_labels(vec![kestrel_reporting::Label::primary(
                        span.file_id,
                        span.range(),
                    )]),
            );
        }
        let (_, target) = self.type_test_target(node, &span);
        let binding = token(node, SyntaxKind::Identifier)
            .map(|name| self.define_named_local(node, &name, false, span.clone()));
        let sub = if has_token(node, SyntaxKind::LBrace) {
            let entity = match &target {
                HirTy::Struct { entity, .. } => Some(*entity),
                _ => None,
            };
            let (fields, has_rest) = self.struct_pattern_members(node);
            self.lower_struct_pat_fields(entity, &fields, has_rest, &span, false)
        } else {
            self.alloc_pat(HirPat::Wildcard { span: span.clone() })
        };
        self.alloc_pat(HirPat::TypeTest {
            binding,
            target,
            sub,
            span,
        })
    }

    /// `match scrutinee { is T => hit, _ => miss }` over bool literals.
    fn type_test_match(
        &mut self,
        scrutinee: HirExprId,
        target: HirTy,
        hit: bool,
        span: &Span,
    ) -> HirExprId {
        let wildcard = self.alloc_pat(HirPat::Wildcard { span: span.clone() });
        let test = self.alloc_pat(HirPat::TypeTest {
            binding: None,
            target,
            sub: wildcard,
            span: span.clone(),
        });
        let otherwise = self.alloc_pat(HirPat::Wildcard { span: span.clone() });
        let hit_expr = self.alloc_expr(HirExpr::Literal {
            value: HirLiteral::Bool(hit),
            span: span.clone(),
        });
        let miss_expr = self.alloc_expr(HirExpr::Literal {
            value: HirLiteral::Bool(!hit),
            span: span.clone(),
        });
        self.alloc_expr(HirExpr::Match {
            scrutinee,
            arms: vec![
                HirMatchArm {
                    pattern: test,
                    guard: None,
                    body: hit_expr,
                },
                HirMatchArm {
                    pattern: otherwise,
                    guard: None,
                    body: miss_expr,
                },
            ],
            source: MatchSource::TypeTest,
            span: span.clone(),
        })
    }

    /// `x is T` / `x is not T`.
    pub(crate) fn desugar_type_test(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let operand = ExprSrc::or_error(first_expr(node), span);
        let scrutinee = self.lower_expr_src(&operand);
        let (_, target) = self.type_test_target(node, span);
        let negated = has_token(node, SyntaxKind::Not);
        let inner = self.type_test_match(scrutinee, target, !negated, span);
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::TypeTest,
            inner,
            span: span.clone(),
        })
    }

    /// `x as? T` and `consuming x as? T`.
    pub(crate) fn desugar_cast(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let operand = ExprSrc::or_error(first_expr(node), span);
        let scrutinee = self.lower_expr_src(&operand);
        let (written, target) = self.type_test_target(node, span);
        let (kind, inner) = if has_token(node, SyntaxKind::Consuming) {
            let result = AstType::Result {
                ok: Box::new(written),
                err: Box::new(AstType::Inferred(span.clone())),
                span: span.clone(),
            };
            (
                SugarKind::ConsumingCast,
                self.consuming_cast(scrutinee, target, result, span),
            )
        } else {
            let optional = AstType::Optional(
                Box::new(AstType::Ref {
                    inner: Box::new(written),
                    mutating: false,
                    span: span.clone(),
                }),
                span.clone(),
            );
            (
                SugarKind::Cast,
                self.borrowing_cast(scrutinee, target, optional, span),
            )
        };
        self.alloc_expr(HirExpr::Sugar {
            kind,
            inner,
            span: span.clone(),
        })
    }

    /// `.name(arg)` / `.name`.
    fn implicit_member(&mut self, name: &str, arg: Option<HirExprId>, span: &Span) -> HirExprId {
        self.alloc_expr(HirExpr::ImplicitMember {
            name: HirName::name(name),
            args: arg.map(|value| vec![HirCallArg { label: None, value }]),
            span: span.clone(),
        })
    }

    /// `{ let $name: ty = value; $name }`, so the implicit members inside
    /// `value` have a type to resolve against.
    fn ascribed_block(
        &mut self,
        name: &str,
        ty: Option<HirTy>,
        prefix: Vec<HirStmtId>,
        value: HirExprId,
        span: &Span,
    ) -> HirExprId {
        let local = self.define_local(name, false, span.clone());
        let mut stmts = prefix;
        stmts.push(self.alloc_stmt(HirStmt::Let {
            local,
            ty,
            value: Some(value),
            span: span.clone(),
        }));
        let tail = self.alloc_expr(HirExpr::Local(local, span.clone()));
        self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts,
                tail_expr: Some(tail),
            },
            span: span.clone(),
        })
    }

    /// `annotation` is `Optional[&T]`, lowered like the user's `&T?` would be.
    fn borrowing_cast(
        &mut self,
        scrutinee: HirExprId,
        target: HirTy,
        annotation: AstType,
        span: &Span,
    ) -> HirExprId {
        let hit_local = self.define_local("$cast_hit", false, span.clone());
        let wildcard = self.alloc_pat(HirPat::Wildcard { span: span.clone() });
        let test = self.alloc_pat(HirPat::TypeTest {
            binding: Some(hit_local),
            target: target.clone(),
            sub: wildcard,
            span: span.clone(),
        });
        let otherwise = self.alloc_pat(HirPat::Wildcard { span: span.clone() });
        let hit_ref = self.alloc_expr(HirExpr::Local(hit_local, span.clone()));
        let some = self.implicit_member("Some", Some(hit_ref), span);
        let none = self.implicit_member("None", None, span);
        let matched = self.alloc_expr(HirExpr::Match {
            scrutinee,
            arms: vec![
                HirMatchArm {
                    pattern: test,
                    guard: None,
                    body: some,
                },
                HirMatchArm {
                    pattern: otherwise,
                    guard: None,
                    body: none,
                },
            ],
            source: MatchSource::TypeTest,
            span: span.clone(),
        });
        let ty = self.lower_type(&annotation);
        self.ascribed_block("$cast", Some(ty), Vec::new(), matched, span)
    }

    /// `annotation` is `T throws _`: the error type is the box, inferred from
    /// `$boxed`.
    fn consuming_cast(
        &mut self,
        scrutinee: HirExprId,
        target: HirTy,
        annotation: AstType,
        span: &Span,
    ) -> HirExprId {
        let boxed = self.define_local("$boxed", false, span.clone());
        let bind = self.alloc_stmt(HirStmt::Let {
            local: boxed,
            ty: None,
            value: Some(scrutinee),
            span: span.clone(),
        });
        let test_operand = self.alloc_expr(HirExpr::Local(boxed, span.clone()));
        let condition = self.type_test_match(test_operand, target.clone(), true, span);
        let take_operand = self.alloc_expr(HirExpr::Local(boxed, span.clone()));
        let payload = self.alloc_expr(HirExpr::TakePayload {
            expr: take_operand,
            target: target.clone(),
            span: span.clone(),
        });
        let ok = self.implicit_member("Ok", Some(payload), span);
        let back = self.alloc_expr(HirExpr::Local(boxed, span.clone()));
        let err = self.implicit_member("Err", Some(back), span);
        let branch = self.alloc_expr(HirExpr::If {
            condition,
            then_body: HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(ok),
            },
            else_body: Some(HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(err),
            }),
            span: span.clone(),
        });
        let ty = self.lower_type(&annotation);
        self.ascribed_block("$result", Some(ty), vec![bind], branch, span)
    }
}
