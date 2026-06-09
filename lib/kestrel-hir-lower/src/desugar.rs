//! Desugaring helpers: operators, for-loops, while, try/throw, unwrap,
//! interpolated strings.
//!
//! Each desugaring function takes AST-level inputs and produces HIR nodes.
//! Protocol entities are resolved via ResolveBuiltin (entity IDs, not strings).

use kestrel_ast::ast_body::*;
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_name_res::ResolveBuiltin;
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;

use crate::ctx::LowerCtx;

/// True if `expr` is syntactically shaped like a place expression (an
/// expression that could appear on the LHS of `=`/`+=`). Conservative —
/// rejects literals, calls, blocks, control-flow expressions, etc. and
/// accepts paths (which may resolve to bindings or fields), member access,
/// tuple indexing, and parenthesized place expressions.
///
/// This is a syntactic check used by `desugar_compound_assign`. Mutability
/// of the resolved binding is enforced later by the assignment analyzer.
fn ast_is_place_expr(body: &AstBody, expr: ExprId) -> bool {
    match &body.exprs[expr] {
        AstExpr::Path { .. } | AstExpr::MemberAccess { .. } | AstExpr::TupleIndex { .. } => true,
        AstExpr::Paren { inner, .. } => ast_is_place_expr(body, *inner),
        _ => false,
    }
}

impl LowerCtx<'_> {
    // ===== Binary operators =====

    /// Desugar a binary op where lhs/rhs are already lowered HirExprIds.
    /// Used by Pratt parser which lowers operands before combining them.
    pub(crate) fn desugar_binary_hir(
        &mut self,
        op: BinaryOp,
        lhs: HirExprId,
        rhs: HirExprId,
        span: &Span,
    ) -> HirExprId {
        // Short-circuit ops wrap RHS in a closure
        if let Some((proto, method, label)) = lookup_short_circuit_op(&op) {
            let rhs_closure = self.alloc_expr(HirExpr::Closure {
                params: Vec::new(),
                body: HirBlock {
                    stmts: Vec::new(),
                    tail_expr: Some(rhs),
                },
                span: span.clone(),
            });
            if let Some(protocol) = self.resolve_builtin(proto) {
                return self.alloc_expr(HirExpr::ProtocolCall {
                    receiver: lhs,
                    protocol,
                    method: HirName::name(method),
                    type_args: None,
                    args: vec![HirCallArg {
                        label: label.map(|l| l.to_string()),
                        value: rhs_closure,
                    }],
                    span: span.clone(),
                });
            }
            self.emit_missing_operator_diagnostic(&op, span);
            return self.alloc_expr(HirExpr::Error { span: span.clone() });
        }

        // Regular binary op
        if let Some((proto, method, label)) = lookup_binary_op(&op)
            && let Some(protocol) = self.resolve_builtin(proto)
        {
            return self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lhs,
                protocol,
                method: HirName::name(method),
                type_args: None,
                args: vec![HirCallArg {
                    label: label.map(|l| l.to_string()),
                    value: rhs,
                }],
                span: span.clone(),
            });
        }

        self.emit_missing_operator_diagnostic(&op, span);
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    /// Desugar && to a ProtocolCall (used by if-condition chains).
    pub(crate) fn desugar_logical_and(
        &mut self,
        lhs: HirExprId,
        rhs: HirExprId,
        span: &Span,
    ) -> HirExprId {
        // Wrap RHS in closure for short-circuit
        let rhs_closure = self.alloc_expr(HirExpr::Closure {
            params: Vec::new(),
            body: HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(rhs),
            },
            span: span.clone(),
        });

        if let Some(protocol) = self.resolve_builtin(Builtin::LogicalAndOperatorProtocol) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lhs,
                protocol,
                method: HirName::name("logicalAnd"),
                type_args: None,
                args: vec![HirCallArg {
                    label: None,
                    value: rhs_closure,
                }],
                span: span.clone(),
            })
        } else {
            // Fallback: just return lhs if protocol not found
            lhs
        }
    }

    // ===== Unary operators =====

    /// Desugar a unary operator to a ProtocolCall.
    pub(crate) fn desugar_unary_op(
        &mut self,
        body: &AstBody,
        op: &UnaryOp,
        operand: ExprId,
        span: &Span,
    ) -> HirExprId {
        // +x is identity
        if *op == UnaryOp::Pos {
            return self.lower_expr(body, operand);
        }

        let lowered_operand = self.lower_expr(body, operand);

        if let Some((proto, method)) = lookup_unary_op(op)
            && let Some(protocol) = self.resolve_builtin(proto)
        {
            return self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_operand,
                protocol,
                method: HirName::name(method),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            });
        }

        self.ctx.accumulate(
            Diagnostic::error()
                .with_message(format!(
                    "unsupported unary operator '{}'",
                    unary_op_symbol(op)
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())])
                .with_notes(vec!["is the standard library imported?".to_string()]),
        );
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    // ===== Postfix operators =====

    /// Desugar a postfix operator to a ProtocolCall (e.g. `x..` → `x.rangeFrom()`).
    pub(crate) fn desugar_postfix_op(
        &mut self,
        body: &AstBody,
        op: &PostfixOp,
        operand: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_operand = self.lower_expr(body, operand);

        if let Some((proto, method)) = lookup_postfix_op(op)
            && let Some(protocol) = self.resolve_builtin(proto)
        {
            return self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_operand,
                protocol,
                method: HirName::name(method),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            });
        }

        self.ctx.accumulate(
            Diagnostic::error()
                .with_message(format!(
                    "unsupported postfix operator '{}'",
                    postfix_op_symbol(op)
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())])
                .with_notes(vec!["is the standard library imported?".to_string()]),
        );
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    // ===== Compound assignment =====

    /// Desugar compound assignment (+=, -=, etc.) to a ProtocolCall, wrapped
    /// in a `SugarKind::CompoundAssign` Sugar node.
    ///
    /// Compound-assign requires the LHS to be a place expression. Inference
    /// can't detect this — `5.addAssign(1)` is type-correct on its own — so
    /// we check the AST shape here and emit `HirExpr::Error` for the inner
    /// when the LHS isn't assignable. The Sugar wrapper is emitted in both
    /// the success and failure paths so downstream consumers see a uniform
    /// shape.
    pub(crate) fn desugar_compound_assign(
        &mut self,
        body: &AstBody,
        lhs: ExprId,
        op: &CompoundAssignOp,
        rhs: ExprId,
        span: &Span,
    ) -> HirExprId {
        // AST-level place check: compound-assign requires a settable LHS.
        // Detected shapes are syntactic; deeper validity (mutability of the
        // resolved binding) is checked later by the assignment analyzer once
        // the desugared `addAssign` is treated as a write to its receiver.
        if !ast_is_place_expr(body, lhs) {
            self.ctx.accumulate(
                Diagnostic::error()
                    .with_message("left-hand side of compound assignment is not assignable")
                    .with_labels(vec![Label::primary(span.file_id, span.range())]),
            );
            let err = self.alloc_expr(HirExpr::Error { span: span.clone() });
            return self.alloc_expr(HirExpr::Sugar {
                kind: SugarKind::CompoundAssign,
                inner: err,
                span: span.clone(),
            });
        }

        let lowered_lhs = self.lower_expr(body, lhs);
        let lowered_rhs = self.lower_expr(body, rhs);

        if let Some((proto, method, label)) = lookup_compound_assign_op(op)
            && let Some(protocol) = self.resolve_builtin(proto)
        {
            let pcall = self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_lhs,
                protocol,
                method: HirName::name(method),
                type_args: None,
                args: vec![HirCallArg {
                    label: label.map(|l| l.to_string()),
                    value: lowered_rhs,
                }],
                span: span.clone(),
            });
            return self.alloc_expr(HirExpr::Sugar {
                kind: SugarKind::CompoundAssign,
                inner: pcall,
                span: span.clone(),
            });
        }

        self.ctx.accumulate(
            Diagnostic::error()
                .with_message(format!(
                    "unsupported compound assignment operator '{}'",
                    compound_assign_op_symbol(op)
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())])
                .with_notes(vec!["is the standard library imported?".to_string()]),
        );
        let err = self.alloc_expr(HirExpr::Error { span: span.clone() });
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::CompoundAssign,
            inner: err,
            span: span.clone(),
        })
    }

    // ===== While → Loop =====

    /// Desugar `while condition { body }` → `loop { if condition {} else { break }; body }`
    ///
    /// We use `if cond {} else { break }` instead of `if !cond { break }` to avoid
    /// requiring the condition type to conform to the Not protocol. This matches lib1
    /// where while conditions only need Bool or BooleanConditional, not Not.
    pub(crate) fn desugar_while(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        condition: ExprId,
        while_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        let lowered_cond = self.lower_expr(body, condition);
        self.while_conditions.push(lowered_cond);

        let break_expr = self.alloc_expr(HirExpr::Break {
            label: label.map(|l| l.to_string()),
            span: span.clone(),
        });

        // if condition {} else { break } — condition true = continue, false = break
        let if_break = self.alloc_expr(HirExpr::If {
            condition: lowered_cond,
            then_body: HirBlock {
                stmts: Vec::new(),
                tail_expr: None,
            },
            else_body: Some(HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(break_expr),
            }),
            span: span.clone(),
        });

        let if_break_stmt = self.alloc_stmt(HirStmt::Expr {
            expr: if_break,
            span: span.clone(),
        });

        // Lower while body (push loop label so break/continue can validate)
        self.push_loop(label);
        let lowered_body = self.lower_block(body, while_body);
        self.pop_loop();

        // Build loop body: if_break + body statements
        let mut loop_stmts = vec![if_break_stmt];
        loop_stmts.extend(lowered_body.stmts);

        self.alloc_expr(HirExpr::Loop {
            label: label.map(|l| l.to_string()),
            body: HirBlock {
                stmts: loop_stmts,
                tail_expr: lowered_body.tail_expr,
            },
            span: span.clone(),
        })
    }

    /// Desugar `while let conditions { body }` → `loop { match ... }`.
    ///
    /// For the common single-`let` case, lower to:
    /// ```text
    /// loop {
    ///     match value {
    ///         pattern => { body }
    ///         _       => break
    ///     }
    /// }
    /// ```
    /// This puts the body **inside** the matching arm, so the move-check
    /// dataflow doesn't lose the binding-then-use correlation through a
    /// merge with the unmatched arm. The previous shape
    /// (`loop { if !cond { break }; body }` where `cond` was a
    /// `match v { pat => true, _ => false }`) joined the two arms before
    /// the body, leaving the bound variable `MaybeInit` at the body's
    /// reads — a flood of false-positive E501s in stdlib iterator code
    /// (`Iterator.fold`, `Set.min`, `Array.appendFrom`, etc.).
    ///
    /// For multi-condition while-let (`while let .Some(x) = a, x > 0 { … }`)
    /// the dataflow-friendly shape is harder to express, so we fall back
    /// to the if-break-merge desugaring for those.
    pub(crate) fn desugar_while_let(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        conditions: &[IfCondition],
        while_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        // Single-`let` shape: lower to `loop { match v { pat => body, _ => break } }`
        // — same structure as `desugar_for_loop`. No CFG merge before the body,
        // so the move-check dataflow keeps the binding-then-use correlation.
        if conditions.len() == 1
            && let IfCondition::Let { pattern, value } = &conditions[0]
        {
            return self.desugar_while_let_single(body, label, *pattern, *value, while_body, span);
        }

        self.desugar_while_let_chain(body, label, conditions, while_body, span)
    }

    /// Single-let shape: `while let pat = value { body }` →
    /// `loop { match value { pat => body, _ => break } }`. The body lives
    /// inside the matching arm, so the move-check dataflow sees `pat`'s
    /// bindings as `DefinitelyInit` at the body without a merge.
    fn desugar_while_let_single(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        pattern: PatId,
        value: ExprId,
        while_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        let lowered_value = self.lower_expr(body, value);

        // Pattern bindings are visible inside the loop body but not after.
        self.push_scope();
        let lowered_pat = self.lower_pat(body, pattern);

        // Lower body inside the same scope as the pattern bindings, with
        // the loop label in scope so `break` / `continue` can validate.
        self.push_loop(label);
        let lowered_body = self.lower_block(body, while_body);
        self.pop_loop();
        self.pop_scope();

        let body_expr = self.alloc_expr(HirExpr::Block {
            body: lowered_body,
            span: span.clone(),
        });

        let wildcard_pat = self.alloc_pat(HirPat::Wildcard { span: span.clone() });
        let break_expr = self.alloc_expr(HirExpr::Break {
            label: label.map(|l| l.to_string()),
            span: span.clone(),
        });

        let match_expr = self.alloc_expr(HirExpr::Match {
            scrutinee: lowered_value,
            arms: vec![
                HirMatchArm {
                    pattern: lowered_pat,
                    guard: None,
                    body: body_expr,
                },
                HirMatchArm {
                    pattern: wildcard_pat,
                    guard: None,
                    body: break_expr,
                },
            ],
            source: MatchSource::WhileLet,
            span: span.clone(),
        });

        let match_stmt = self.alloc_stmt(HirStmt::Expr {
            expr: match_expr,
            span: span.clone(),
        });

        self.alloc_expr(HirExpr::Loop {
            label: label.map(|l| l.to_string()),
            body: HirBlock {
                stmts: vec![match_stmt],
                tail_expr: None,
            },
            span: span.clone(),
        })
    }

    /// Multi-condition while-let — `while let .Some(x) = a, let .Some(y) = b
    /// { … }` and friends. Lowers to `loop { match a { .Some(x) => match b {
    /// .Some(y) => { body }, _ => break }, _ => break } }` via the shared
    /// condition chain, so every `let` binding stays in scope in the body under
    /// OSSA (success = one iteration of the body, fail = break out of the loop).
    /// The old `loop { if !cond { break }; body }` shape routed the patterns
    /// through the boolean-AND `lower_if_conditions` path, which dropped the
    /// bindings and tripped OSSA verification (issue #126's while-let twin).
    fn desugar_while_let_chain(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        conditions: &[IfCondition],
        while_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        // Success: run the body for one iteration (inside the pattern scopes so
        // the bindings are visible, with the loop label in scope for
        // break/continue). Fail: break out of the loop.
        let mut on_success = |this: &mut Self| {
            this.push_loop(label);
            let lowered_body = this.lower_block(body, while_body);
            this.pop_loop();
            this.alloc_expr(HirExpr::Block {
                body: lowered_body,
                span: span.clone(),
            })
        };
        let mut on_fail = |this: &mut Self| {
            this.alloc_expr(HirExpr::Break {
                label: label.map(|l| l.to_string()),
                span: span.clone(),
            })
        };
        let match_expr = self.lower_condition_chain(
            body,
            conditions,
            MatchSource::WhileLet,
            span,
            &mut on_success,
            &mut on_fail,
        );

        let match_stmt = self.alloc_stmt(HirStmt::Expr {
            expr: match_expr,
            span: span.clone(),
        });

        self.alloc_expr(HirExpr::Loop {
            label: label.map(|l| l.to_string()),
            body: HirBlock {
                stmts: vec![match_stmt],
                tail_expr: None,
            },
            span: span.clone(),
        })
    }

    // ===== For → Loop + iterator protocol =====

    /// Desugar `for pattern in iterable { body }` to:
    /// ```text
    /// let $iter = iterable.iter()   // via Iterable protocol
    /// loop {
    ///     match $iter.next() {       // via Iterator protocol
    ///         .Some(pattern) => { body }
    ///         .None => break
    ///     }
    /// }
    /// ```
    pub(crate) fn desugar_for_loop(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        pattern: PatId,
        iterable: ExprId,
        for_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        let lowered_iterable = self.lower_expr(body, iterable);

        // $iter = iterable.iter() via Iterable protocol.
        // If the IterableProtocol builtin isn't available (e.g. tests with
        // `stdlib: false`), `for ... in` cannot work — emit the user-language
        // diagnostic mentioning the protocol name and short-circuit to a
        // Sugar-wrapped Error so downstream inference absorbs silently.
        let Some(iter_protocol) = self.resolve_builtin(Builtin::IterableProtocol) else {
            self.ctx.accumulate(
                Diagnostic::error()
                    .with_message("`for` loop requires the `Iterable` protocol")
                    .with_labels(vec![Label::primary(span.file_id, span.range())])
                    .with_notes(vec!["is the standard library imported?".to_string()]),
            );
            let err = self.alloc_expr(HirExpr::Error { span: span.clone() });
            return self.alloc_expr(HirExpr::Sugar {
                kind: SugarKind::ForLoop,
                inner: err,
                span: span.clone(),
            });
        };
        let iterate_call = self.alloc_expr(HirExpr::ProtocolCall {
            receiver: lowered_iterable,
            protocol: iter_protocol,
            method: HirName::name("iter"),
            type_args: None,
            args: Vec::new(),
            span: span.clone(),
        });

        let iter_local = self.define_local("$iter", true, span.clone());
        let iter_let = self.alloc_stmt(HirStmt::Let {
            local: iter_local,
            ty: None,
            value: Some(iterate_call),
            span: span.clone(),
        });

        // $iter.next() via Iterator protocol
        let iter_ref = self.alloc_expr(HirExpr::Local(iter_local, span.clone()));
        let next_call = if let Some(protocol) = self.resolve_builtin(Builtin::IteratorProtocol) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: iter_ref,
                protocol,
                method: HirName::name("next"),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        } else {
            self.alloc_expr(HirExpr::MethodCall {
                receiver: iter_ref,
                method: HirName::name("next"),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        };

        // Pattern for .Some(pattern)
        self.push_scope();
        let lowered_pat = self.lower_pat(body, pattern);
        let some_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name("Some"),
            args: vec![HirPatArg {
                label: None,
                pattern: lowered_pat,
            }],
            span: span.clone(),
        });

        // Lower for body (push loop label so break/continue can validate)
        self.push_loop(label);
        let lowered_for_body = self.lower_block(body, for_body);
        self.pop_loop();
        self.pop_scope();

        // Wrap the for-body as a block expression so all statements are reachable
        let body_expr = self.alloc_expr(HirExpr::Block {
            body: lowered_for_body,
            span: span.clone(),
        });

        // .None => break
        let none_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name("None"),
            args: Vec::new(),
            span: span.clone(),
        });
        let break_expr = self.alloc_expr(HirExpr::Break {
            label: label.map(|l| l.to_string()),
            span: span.clone(),
        });

        // match $iter.next() { .Some(pat) => body, .None => break }
        let match_expr = self.alloc_expr(HirExpr::Match {
            scrutinee: next_call,
            arms: vec![
                HirMatchArm {
                    pattern: some_pat,
                    guard: None,
                    body: body_expr,
                },
                HirMatchArm {
                    pattern: none_pat,
                    guard: None,
                    body: break_expr,
                },
            ],
            source: MatchSource::ForLoop,
            span: span.clone(),
        });

        let match_stmt = self.alloc_stmt(HirStmt::Expr {
            expr: match_expr,
            span: span.clone(),
        });

        // Iterator created once before the loop, match runs each iteration
        let loop_expr = self.alloc_expr(HirExpr::Loop {
            label: label.map(|l| l.to_string()),
            body: HirBlock {
                stmts: vec![match_stmt],
                tail_expr: None,
            },
            span: span.clone(),
        });

        // Wrap in a block: { let $iter = iterable.iter(); loop { ... } }
        let block_expr = self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts: vec![iter_let],
                tail_expr: Some(loop_expr),
            },
            span: span.clone(),
        });

        // Mark the desugared subtree with a Sugar wrapper so post-typing
        // analyzers and inference's primary-constraint emission can recognize
        // the user-facing `for` construct without re-deriving from protocol id.
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::ForLoop,
            inner: block_expr,
            span: span.clone(),
        })
    }

    // ===== Try / Throw / Unwrap =====

    /// Desugar `try operand` to:
    /// ```text
    /// match operand {
    ///     .Ok($v) => $v,
    ///     .Err($e) => return .Err($e)
    /// }
    /// ```
    /// Desugar `try operand` using the Tryable protocol:
    ///
    /// ```text
    /// match operand.tryExtract() {
    ///     .Continue($value) => $value,
    ///     .Break($early) => return .fromResidual($early)
    /// }
    /// ```
    ///
    /// `tryExtract()` is a ProtocolCall on Tryable, returning ControlFlow[Output, Early].
    /// `.fromResidual($early)` is an ImplicitMember resolved against the function's
    /// return type (which must conform to FromResidual[Early]).
    /// Falls back to hardcoded .Ok/.Err if Tryable protocol is not available.
    pub(crate) fn desugar_try(
        &mut self,
        body: &AstBody,
        operand: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_operand = self.lower_expr(body, operand);

        // Try requires the Tryable protocol. Without it (e.g. tests with
        // `stdlib: false`) the desugaring would produce a malformed match
        // whose .Ok/.Err arms cascade into "implicit member not found"
        // errors. Emit the user-language Tryable diagnostic directly and
        // return Sugar wrapping HirExpr::Error so downstream absorbs.
        let Some(try_protocol) = self.resolve_builtin(Builtin::TryableProtocol) else {
            self.ctx.accumulate(
                Diagnostic::error()
                    .with_message("`try` expression requires the `Tryable` protocol")
                    .with_labels(vec![Label::primary(span.file_id, span.range())])
                    .with_notes(vec!["is the standard library imported?".to_string()]),
            );
            let err = self.alloc_expr(HirExpr::Error { span: span.clone() });
            return self.alloc_expr(HirExpr::Sugar {
                kind: SugarKind::Try,
                inner: err,
                span: span.clone(),
            });
        };
        let scrutinee = self.alloc_expr(HirExpr::ProtocolCall {
            receiver: lowered_operand,
            protocol: try_protocol,
            method: HirName::name("tryExtract"),
            type_args: None,
            args: vec![],
            span: span.clone(),
        });

        // Tryable is available — arms unwrap ControlFlow.
        let (success_name, failure_name) = ("Continue", "Break");

        // .Continue($value) => $value  (or .Ok($value) => $value)
        let value_local = self.define_local("$try_value", false, span.clone());
        let value_binding = self.alloc_pat(HirPat::Binding {
            local: value_local,
            span: span.clone(),
        });
        let continue_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name(success_name),
            args: vec![HirPatArg {
                label: None,
                pattern: value_binding,
            }],
            span: span.clone(),
        });
        let continue_body = self.alloc_expr(HirExpr::Local(value_local, span.clone()));

        // .Break($early) => return .fromResidual($early)
        // (or .Err($e) => return .Err($e) for fallback)
        let early_local = self.define_local("$try_early", false, span.clone());
        let early_binding = self.alloc_pat(HirPat::Binding {
            local: early_local,
            span: span.clone(),
        });
        let break_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name(failure_name),
            args: vec![HirPatArg {
                label: None,
                pattern: early_binding,
            }],
            span: span.clone(),
        });
        let early_ref = self.alloc_expr(HirExpr::Local(early_local, span.clone()));

        // .fromResidual($early) — resolved against the function's return type
        let return_value = self.alloc_expr(HirExpr::ImplicitMember {
            name: HirName::name("fromResidual"),
            args: Some(vec![HirCallArg {
                label: Some("residual".to_string()),
                value: early_ref,
            }]),
            span: span.clone(),
        });
        let return_early = self.alloc_expr(HirExpr::Return {
            value: Some(return_value),
            span: span.clone(),
        });

        let match_expr = self.alloc_expr(HirExpr::Match {
            scrutinee,
            arms: vec![
                HirMatchArm {
                    pattern: continue_pat,
                    guard: None,
                    body: continue_body,
                },
                HirMatchArm {
                    pattern: break_pat,
                    guard: None,
                    body: return_early,
                },
            ],
            source: MatchSource::TryOp,
            span: span.clone(),
        });

        // Sugar wrapper marks the user-facing `try` so cascade suppression
        // can fire a single Tryable-conformance error in place of the
        // synthesized `tryExtract` Member + `.Continue`/`.fromResidual`
        // ImplicitMember failures inside `inner`.
        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::Try,
            inner: match_expr,
            span: span.clone(),
        })
    }

    /// Desugar `throw value` → `return .Err(value)`
    ///
    /// If `value` already lowered to `HirExpr::Error` (e.g. parser recovered
    /// from `throw\n}`), short-circuit to `HirExpr::Error` so inference
    /// doesn't cascade — without this the `.Err` member resolution fires
    /// and reports a confusing follow-up like "implicit member '.Err' not
    /// found on Error".
    pub(crate) fn desugar_throw(
        &mut self,
        body: &AstBody,
        value: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_value = self.lower_expr(body, value);

        if matches!(&self.exprs[lowered_value], HirExpr::Error { .. }) {
            return self.alloc_expr(HirExpr::Error { span: span.clone() });
        }

        let err_wrap = self.alloc_expr(HirExpr::ImplicitMember {
            name: HirName::name("Err"),
            args: Some(vec![HirCallArg {
                label: None,
                value: lowered_value,
            }]),
            span: span.clone(),
        });

        self.alloc_expr(HirExpr::Return {
            value: Some(err_wrap),
            span: span.clone(),
        })
    }

    // `operand!` (force-unwrap) desugars to a ProtocolCall via
    // `desugar_postfix_op` + the `POSTFIX_OP_PROTOCOLS` table
    // (`ForceUnwrap.forceUnwrap`), exactly like postfix `..`.

    // ===== Interpolated strings =====

    /// Desugar `"hello \(name)!"` to a DefaultStringInterpolation builder sequence:
    /// ```text
    /// {
    ///     var $dsi = DefaultStringInterpolation(literalCapacity: 6, interpolationCount: 1)
    ///     $dsi.appendLiteral(literal: "hello ")
    ///     $dsi.appendInterpolation(value: name)
    ///     $dsi.appendLiteral(literal: "!")
    ///     $dsi.build()
    /// }
    /// ```
    pub(crate) fn desugar_interpolated_string(
        &mut self,
        body: &AstBody,
        parts: &[StringPart],
        span: &Span,
    ) -> HirExprId {
        // Empty interpolated string → plain empty string literal
        if parts.is_empty() {
            return self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::String {
                    value: String::new(),
                    escape_errors: Vec::new(),
                },
                span: span.clone(),
            });
        }

        // Resolve the DefaultStringInterpolation struct builtin
        let Some(dsi_struct) = self.resolve_builtin(Builtin::DefaultStringInterpolation) else {
            self.ctx.accumulate(
                Diagnostic::error()
                    .with_message("string interpolation requires the standard library")
                    .with_labels(vec![Label::primary(span.file_id, span.range())]),
            );
            let err = self.alloc_expr(HirExpr::Error { span: span.clone() });
            return self.alloc_expr(HirExpr::Sugar {
                kind: SugarKind::StringInterpolation,
                inner: err,
                span: span.clone(),
            });
        };

        // Pre-compute capacity hints
        let mut literal_capacity: i64 = 0;
        let mut interpolation_count: i64 = 0;
        for part in parts {
            match part {
                StringPart::Literal(text) => literal_capacity += text.len() as i64,
                StringPart::Interpolation { .. } => interpolation_count += 1,
            }
        }

        let mut stmts: Vec<HirStmtId> = Vec::new();

        // var $dsi = DefaultStringInterpolation(literalCapacity: N, interpolationCount: M)
        let dsi_local = self.define_local("$dsi", true, span.clone());
        let dsi_type_ref = self.alloc_expr(HirExpr::Def(dsi_struct, Vec::new(), span.clone()));
        let lit_cap = self.alloc_expr(HirExpr::Literal {
            value: HirLiteral::Integer(literal_capacity),
            span: span.clone(),
        });
        let interp_count = self.alloc_expr(HirExpr::Literal {
            value: HirLiteral::Integer(interpolation_count),
            span: span.clone(),
        });
        let init_call = self.alloc_expr(HirExpr::Call {
            callee: dsi_type_ref,
            args: vec![
                HirCallArg {
                    label: Some("literalCapacity".to_string()),
                    value: lit_cap,
                },
                HirCallArg {
                    label: Some("interpolationCount".to_string()),
                    value: interp_count,
                },
            ],
            span: span.clone(),
        });
        stmts.push(self.alloc_stmt(HirStmt::Let {
            local: dsi_local,
            ty: None,
            value: Some(init_call),
            span: span.clone(),
        }));

        // For each part, emit appendLiteral or appendInterpolation
        for part in parts {
            let dsi_ref = self.alloc_expr(HirExpr::Local(dsi_local, span.clone()));
            match part {
                StringPart::Literal(text) => {
                    if text.is_empty() {
                        continue;
                    }
                    let str_lit = self.alloc_expr(HirExpr::Literal {
                        value: HirLiteral::String {
                            value: text.clone(),
                            escape_errors: Vec::new(),
                        },
                        span: span.clone(),
                    });
                    let append = self.alloc_expr(HirExpr::MethodCall {
                        receiver: dsi_ref,
                        method: HirName::name("appendLiteral"),
                        type_args: None,
                        args: vec![HirCallArg {
                            label: None,
                            value: str_lit,
                        }],
                        span: span.clone(),
                    });
                    stmts.push(self.alloc_stmt(HirStmt::Expr {
                        expr: append,
                        span: span.clone(),
                    }));
                },
                StringPart::Interpolation { expr, format } => {
                    let lowered = self.lower_expr(body, *expr);

                    let mut args = vec![HirCallArg {
                        label: None,
                        value: lowered,
                    }];

                    // If format spec is present, parse it and build FormatOptions
                    if let Some(spec_str) = format
                        && let Some(opts_expr) = self.build_format_options_from_spec(spec_str, span)
                    {
                        args.push(HirCallArg {
                            label: None,
                            value: opts_expr,
                        });
                    }

                    let append = self.alloc_expr(HirExpr::MethodCall {
                        receiver: dsi_ref,
                        method: HirName::name("appendInterpolation"),
                        type_args: None,
                        args,
                        span: span.clone(),
                    });
                    stmts.push(self.alloc_stmt(HirStmt::Expr {
                        expr: append,
                        span: span.clone(),
                    }));
                },
            }
        }

        // Tail expression: $dsi.build()
        let dsi_ref = self.alloc_expr(HirExpr::Local(dsi_local, span.clone()));
        let build_call = self.alloc_expr(HirExpr::MethodCall {
            receiver: dsi_ref,
            method: HirName::name("build"),
            type_args: None,
            args: Vec::new(),
            span: span.clone(),
        });

        let block = self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts,
                tail_expr: Some(build_call),
            },
            span: span.clone(),
        });

        self.alloc_expr(HirExpr::Sugar {
            kind: SugarKind::StringInterpolation,
            inner: block,
            span: span.clone(),
        })
    }

    /// Build a FormatOptions expression from a format spec string.
    /// Returns None if the spec can't be parsed or the builtin isn't available.
    ///
    /// Emits a block: `{ var $opts = FormatOptions(); $opts.radix = 16; ... ; $opts }`
    fn build_format_options_from_spec(&mut self, spec: &str, span: &Span) -> Option<HirExprId> {
        use crate::format_spec::{self, Alignment, FormatType, SignMode};

        let parsed = match format_spec::parse_format_spec(spec) {
            Ok(s) => s,
            Err(e) => {
                self.ctx.accumulate(
                    Diagnostic::error()
                        .with_message(format!("invalid format specifier: {e}"))
                        .with_labels(vec![Label::primary(span.file_id, span.range())]),
                );
                return None;
            },
        };

        let fo_entity = self.resolve_builtin(Builtin::FormatOptions)?;

        let mut stmts: Vec<HirStmtId> = Vec::new();

        // var $opts = FormatOptions()
        let opts_local = self.define_local("$opts", true, span.clone());
        let fo_ref = self.alloc_expr(HirExpr::Def(fo_entity, Vec::new(), span.clone()));
        let init_call = self.alloc_expr(HirExpr::Call {
            callee: fo_ref,
            args: Vec::new(),
            span: span.clone(),
        });
        stmts.push(self.alloc_stmt(HirStmt::Let {
            local: opts_local,
            ty: None,
            value: Some(init_call),
            span: span.clone(),
        }));

        // Helper: emit `$opts.field = value`
        let assign_field =
            |this: &mut Self, stmts: &mut Vec<HirStmtId>, name: &str, value: HirExprId| {
                let target_base = this.alloc_expr(HirExpr::Local(opts_local, span.clone()));
                let target = this.alloc_expr(HirExpr::Field {
                    base: target_base,
                    name: HirName::name(name),
                    span: span.clone(),
                });
                let assign = this.alloc_expr(HirExpr::Assign {
                    target,
                    value,
                    span: span.clone(),
                });
                stmts.push(this.alloc_stmt(HirStmt::Expr {
                    expr: assign,
                    span: span.clone(),
                }));
            };

        // width: Int64? — only if specified
        if let Some(w) = parsed.width {
            let int_lit = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Integer(w as i64),
                span: span.clone(),
            });
            let some_val = self.alloc_expr(HirExpr::ImplicitMember {
                name: HirName::name("Some"),
                args: Some(vec![HirCallArg {
                    label: None,
                    value: int_lit,
                }]),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "width", some_val);
        }

        // precision: Int64? — only if specified
        if let Some(p) = parsed.precision {
            let int_lit = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Integer(p as i64),
                span: span.clone(),
            });
            let some_val = self.alloc_expr(HirExpr::ImplicitMember {
                name: HirName::name("Some"),
                args: Some(vec![HirCallArg {
                    label: None,
                    value: int_lit,
                }]),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "precision", some_val);
        }

        // alignment — only if non-default (Left)
        if parsed.alignment != Alignment::Left {
            let variant = match parsed.alignment {
                Alignment::Right => "Right",
                Alignment::Center => "Center",
                Alignment::Left => unreachable!(),
            };
            let val = self.alloc_expr(HirExpr::ImplicitMember {
                name: HirName::name(variant),
                args: None,
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "alignment", val);
        }

        // fill — only if non-default (' ')
        if parsed.fill != ' ' {
            let val = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Char(parsed.fill as u32),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "fill", val);
        }

        // radix — determined by format type
        let radix: i64 = match parsed.format_type {
            FormatType::Binary => 2,
            FormatType::Octal => 8,
            FormatType::Hex | FormatType::HexUpper => 16,
            _ => 10,
        };
        if radix != 10 {
            let val = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Integer(radix),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "radix", val);
        }

        // uppercase
        if matches!(parsed.format_type, FormatType::HexUpper) {
            let val = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Bool(true),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "uppercase", val);
        }

        // sign — only if non-default (Negative)
        if parsed.sign != SignMode::Negative {
            let variant = match parsed.sign {
                SignMode::Always => "Always",
                SignMode::Space => "Space",
                SignMode::Negative => unreachable!(),
            };
            let val = self.alloc_expr(HirExpr::ImplicitMember {
                name: HirName::name(variant),
                args: None,
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "sign", val);
        }

        // alternate
        if parsed.alternate {
            let val = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Bool(true),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "alternate", val);
        }

        // floatStyle — only if non-default (Auto)
        let float_variant = match parsed.format_type {
            FormatType::Fixed => Some("Fixed"),
            FormatType::Scientific => Some("Scientific"),
            FormatType::ScientificUpper => Some("ScientificUpper"),
            FormatType::Percent => Some("Percent"),
            _ => None,
        };
        if let Some(variant) = float_variant {
            let val = self.alloc_expr(HirExpr::ImplicitMember {
                name: HirName::name(variant),
                args: None,
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "floatStyle", val);
        }

        // debug
        if matches!(parsed.format_type, FormatType::Debug) {
            let val = self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Bool(true),
                span: span.clone(),
            });
            assign_field(self, &mut stmts, "debug", val);
        }

        // If no non-default fields were set, skip the block entirely
        if stmts.len() == 1 {
            // Only the Let stmt — return the init directly
            return Some(init_call);
        }

        // Tail: $opts
        let opts_ref = self.alloc_expr(HirExpr::Local(opts_local, span.clone()));

        Some(self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts,
                tail_expr: Some(opts_ref),
            },
            span: span.clone(),
        }))
    }

    // ===== Builtin resolution helper =====

    /// Resolve a builtin type/protocol to its entity ID.
    /// Uses ResolveBuiltin query (cached, resolves from root context).
    fn resolve_builtin(&self, builtin: Builtin) -> Option<kestrel_hecs::Entity> {
        let result = self.ctx.query(ResolveBuiltin {
            builtin,
            root: self.root,
        });
        if result.is_none() {
            kestrel_debug::ktrace!("hir-lower", "builtin not found: {:?}", builtin);
        }
        result
    }

    /// Emit a diagnostic for a binary operator whose protocol couldn't be resolved.
    fn emit_missing_operator_diagnostic(&self, op: &BinaryOp, span: &Span) {
        self.ctx.accumulate(
            Diagnostic::error()
                .with_message(format!(
                    "unsupported binary operator '{}'",
                    binary_op_symbol(op)
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())])
                .with_notes(vec!["is the standard library imported?".to_string()]),
        );
    }
}

// ===== Operator table lookups =====

/// Look up protocol for a binary operator.
fn lookup_binary_op(op: &BinaryOp) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_binary_op(op)
}

/// Look up protocol for a short-circuit operator.
fn lookup_short_circuit_op(op: &BinaryOp) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_short_circuit_op(op)
}

/// Look up protocol for a unary operator.
fn lookup_unary_op(op: &UnaryOp) -> Option<(Builtin, &'static str)> {
    kestrel_hir::body::lookup_unary_op(op)
}

/// Look up protocol for a compound assignment operator.
fn lookup_compound_assign_op(
    op: &CompoundAssignOp,
) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_compound_assign_op(op)
}

/// Human-readable symbol for a binary operator.
fn binary_op_symbol(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Coalesce => "??",
        BinaryOp::RangeInclusive => "...",
        BinaryOp::RangeExclusive => "..<",
    }
}

/// Human-readable symbol for a unary operator.
fn unary_op_symbol(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::BitNot => "!",
        UnaryOp::LogicalNot => "not",
        UnaryOp::Pos => "+",
        UnaryOp::RangeUpTo => "..<",
        UnaryOp::RangeThrough => "..=",
    }
}

fn postfix_op_symbol(op: &PostfixOp) -> &'static str {
    match op {
        PostfixOp::Unwrap => "!",
        PostfixOp::RangeFrom => "..",
    }
}

/// Human-readable symbol for a compound assignment operator.
fn compound_assign_op_symbol(op: &CompoundAssignOp) -> &'static str {
    match op {
        CompoundAssignOp::AddAssign => "+=",
        CompoundAssignOp::SubAssign => "-=",
        CompoundAssignOp::MulAssign => "*=",
        CompoundAssignOp::DivAssign => "/=",
        CompoundAssignOp::RemAssign => "%=",
        CompoundAssignOp::BitAndAssign => "&=",
        CompoundAssignOp::BitOrAssign => "|=",
        CompoundAssignOp::BitXorAssign => "^=",
        CompoundAssignOp::ShlAssign => "<<=",
        CompoundAssignOp::ShrAssign => ">>=",
    }
}
