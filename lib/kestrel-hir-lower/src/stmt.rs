//! Statement lowering: CST statements → HirStmt.
//!
//! Handles let bindings (simple and destructuring), expression statements,
//! guard desugaring, and deinit statements.

use kestrel_ast::UnaryOp;
use kestrel_hir::body::*;
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ctx::LowerCtx;
use crate::syntax::{
    BlockSyntax, Cond, ExprSrc, PatSrc, StmtSyntax, code_block, first_expr, is_expr_like,
    let_conditions, let_syntax, token, unary_op, unwrap_expr,
};

impl LowerCtx<'_> {
    /// Lower one statement of a block.
    pub(crate) fn lower_stmt(&mut self, stmt: &StmtSyntax) -> HirStmtId {
        match stmt {
            StmtSyntax::Expr { expr, span } => {
                let lowered = self.lower_expr_src(expr);
                self.alloc_stmt(HirStmt::Expr {
                    expr: lowered,
                    span: span.clone(),
                })
            },
            StmtSyntax::Node(node) => {
                let id = self.lower_stmt_node(node);
                self.source_map.record_stmt(node, id);
                id
            },
        }
    }

    fn lower_stmt_node(&mut self, node: &SyntaxNode) -> HirStmtId {
        let span = self.span(node);
        match node.kind() {
            SyntaxKind::VariableDeclaration => self.lower_let_stmt(node, &span),
            SyntaxKind::GuardStatement => self.lower_guard(node, &span),
            SyntaxKind::DeinitStatement => self.lower_deinit_stmt(node, &span),
            SyntaxKind::ExpressionStatement => {
                let expr = ExprSrc::or_error(first_expr(node), &span);
                let lowered = self.lower_expr_src(&expr);
                self.alloc_stmt(HirStmt::Expr {
                    expr: lowered,
                    span,
                })
            },
            kind => {
                let expr = if is_expr_like(kind) {
                    ExprSrc::Node(node.clone())
                } else {
                    ExprSrc::Error(span.clone())
                };
                let lowered = self.lower_expr_src(&expr);
                self.alloc_stmt(HirStmt::Expr {
                    expr: lowered,
                    span,
                })
            },
        }
    }

    /// `deinit name;`
    fn lower_deinit_stmt(&mut self, node: &SyntaxNode, span: &Span) -> HirStmtId {
        // An absent name was reported by the parser; there is nothing to
        // look up.
        let Some(name) = token(node, SyntaxKind::Identifier) else {
            return self.alloc_stmt(HirStmt::Deinit {
                name: HirName::Missing,
                local: None,
                span: span.clone(),
            });
        };
        let name = name.text().to_string();
        let local = self.lookup_local(&name);
        if local.is_none() {
            self.ctx.accumulate(
                Diagnostic::error()
                    .with_code("E137")
                    .with_message(format!("undeclared variable '{name}'"))
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range())
                            .with_message("no local with this name in scope"),
                    ]),
            );
        }
        self.alloc_stmt(HirStmt::Deinit {
            name: HirName::Name(name),
            local,
            span: span.clone(),
        })
    }

    /// Lower a let statement.
    /// Simple binding → HirStmt::Let with local.
    /// Complex pattern → temp local + match destructure.
    fn lower_let_stmt(&mut self, node: &SyntaxNode, span: &Span) -> HirStmtId {
        let syn = let_syntax(node, span, self.file_id);
        let is_mut = syn.is_mut;
        let lowered_ty = syn
            .ty
            .as_ref()
            .map(|t| self.lower_type_in(t, crate::ty::RefPosition::Binding));
        let pat = syn.pat.resolve(self.file_id);
        // A simple binding names the local directly; anything else (including
        // a binding whose name the parser could not find) destructures.
        let binding = match &pat {
            PatSrc::Node(n)
                if matches!(
                    n.kind(),
                    SyntaxKind::BindingPattern | SyntaxKind::RefBindingPattern
                ) =>
            {
                token(n, SyntaxKind::Identifier).map(|name| (n.clone(), name))
            },
            _ => None,
        };

        // Named ref binding carve (stage 1.5 item 2): a `&expr` /
        // `&mutating expr` initializer is legal exactly here — a simple
        // immutable `let` — and lowers to `HirExpr::Borrow` instead of
        // hitting the unary-op rejection (E488). `var` (no rebinding to
        // confuse with store-through) and destructuring patterns (the
        // desugared temp would dangle) reject with E209 but still lower
        // as a Borrow so downstream diagnostics stay typed.
        let borrow_init = syn.value.as_ref().and_then(|v| {
            let unary = unwrap_expr(v);
            if unary.kind() != SyntaxKind::ExprUnary {
                return None;
            }
            let mutating = match unary_op(&unary)? {
                UnaryOp::Borrow => false,
                UnaryOp::BorrowMutating => true,
                _ => return None,
            };
            let uspan = self.span(&unary);
            let operand = ExprSrc::or_error(first_expr(&unary), &uspan);
            Some((unary, operand, mutating, uspan))
        });
        let lowered_value = match borrow_init {
            Some((unary, operand, mutating, uspan)) => {
                if is_mut || binding.is_none() {
                    self.ctx.accumulate(
                        Diagnostic::error()
                            .with_code("E209")
                            .with_message("a ref binding must be a simple `let`")
                            .with_labels(vec![
                                Label::primary(uspan.file_id, uspan.range())
                                    .with_message("borrow initializer"),
                            ])
                            .with_notes(vec![
                                "`&` bindings cannot be reassigned or destructured; \
                                 write `let r = &…;`"
                                    .to_string(),
                            ]),
                    );
                    // Recovery: drop the `&` — a var/destructured local must
                    // stay value-typed (a Ref-typed `var` slot has no MIR
                    // representation).
                    Some(self.lower_expr_src(&operand))
                } else {
                    let inner = self.lower_expr_src(&operand);
                    let borrow = self.alloc_expr(HirExpr::Borrow {
                        inner,
                        mutating,
                        span: uspan,
                    });
                    self.source_map.record_expr(&unary, borrow);
                    Some(borrow)
                }
            },
            None => syn.value.as_ref().map(|v| self.lower_expr(v)),
        };

        if let Some((binding_node, name)) = binding {
            let local = self.define_named_local(&binding_node, &name, is_mut, span.clone());
            return self.alloc_stmt(HirStmt::Let {
                local,
                ty: lowered_ty,
                value: lowered_value,
                span: span.clone(),
            });
        }

        // Complex pattern: allocate temp, then destructure via match
        // Emits: { let $let_tmp = value; match $let_tmp { pattern => () } }
        let temp = self.define_local("$let_tmp", is_mut, span.clone());
        let let_stmt = self.alloc_stmt(HirStmt::Let {
            local: temp,
            ty: lowered_ty,
            value: lowered_value,
            span: span.clone(),
        });

        // Lower the pattern (this allocates locals for bindings within).
        // Pass `is_mut` so an outer `var (a, b) = …` propagates mutability
        // into each sub-pattern binding (a, b), not just the temp.
        let hir_pat = self.lower_pat_forcing_mut(&pat, is_mut);

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
            source: MatchSource::LetDestructure,
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

    /// A guard statement's conditions and else block.
    pub(crate) fn guard_parts(&self, node: &SyntaxNode) -> (Vec<Cond>, BlockSyntax) {
        let conditions = let_conditions(node, SyntaxKind::GuardCondition, self.file_id);
        let else_body = BlockSyntax::of(code_block(node), self.file_id);
        (conditions, else_body)
    }

    /// Lower a guard statement.
    /// Desugars to: if !conditions { else_body } with bindings in outer scope.
    fn lower_guard(&mut self, node: &SyntaxNode, span: &Span) -> HirStmtId {
        let (conditions, else_body) = self.guard_parts(node);

        // Lower the else body (must diverge: return/break/continue/throw)
        let lowered_else = self.lower_block(&else_body);

        // Build the condition check
        let condition_expr = self.lower_if_conditions(&conditions, MatchSource::Guard, span);

        // Guard: if condition fails (is false / pattern doesn't match), run else
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
        // Mark this statement as originating from guard so the
        // guard divergence analyzer can check the else block diverges.
        self.guard_stmts.push(stmt_id);
        stmt_id
    }
}
