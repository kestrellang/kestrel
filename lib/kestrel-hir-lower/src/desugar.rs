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
                captures: Vec::new(),
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
        if let Some((proto, method, label)) = lookup_binary_op(&op) {
            if let Some(protocol) = self.resolve_builtin(proto) {
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
            captures: Vec::new(),
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

        if let Some((proto, method, label)) = lookup_compound_assign_op(op) {
            if let Some(protocol) = self.resolve_builtin(proto) {
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

    /// Desugar `while let conditions { body }` → `loop { match ... }`
    pub(crate) fn desugar_while_let(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        conditions: &[IfCondition],
        while_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        // Scope enclosing the condition + loop body so `while let` pattern
        // bindings are visible inside the body but not after the loop.
        self.push_scope();
        // Lower conditions to a boolean expression
        let cond = self.lower_if_conditions(body, conditions, MatchSource::WhileLet, span);

        let break_expr = self.alloc_expr(HirExpr::Break {
            label: label.map(|l| l.to_string()),
            span: span.clone(),
        });

        // Negate: if !cond { break }
        let negated =
            if let Some(protocol) = self.resolve_builtin(Builtin::LogicalNotOperatorProtocol) {
                self.alloc_expr(HirExpr::ProtocolCall {
                    receiver: cond,
                    protocol,
                    method: HirName::name("logicalNot"),
                    type_args: None,
                    args: Vec::new(),
                    span: span.clone(),
                })
            } else {
                cond
            };

        let if_break = self.alloc_expr(HirExpr::If {
            condition: negated,
            then_body: HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(break_expr),
            },
            else_body: None,
            span: span.clone(),
        });

        let if_break_stmt = self.alloc_stmt(HirStmt::Expr {
            expr: if_break,
            span: span.clone(),
        });

        self.push_loop(label);
        let lowered_body = self.lower_block(body, while_body);
        self.pop_loop();
        self.pop_scope();

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

    /// Desugar `operand!` (unwrap) to:
    /// ```text
    /// match operand {
    ///     .Some($v) => $v,
    ///     .None => <unreachable/trap>
    /// }
    /// ```
    pub(crate) fn desugar_unwrap(
        &mut self,
        body: &AstBody,
        operand: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_operand = self.lower_expr(body, operand);

        // .Some($v) => $v
        let some_local = self.define_local("$unwrap", false, span.clone());
        let some_binding = self.alloc_pat(HirPat::Binding {
            local: some_local,
            span: span.clone(),
        });
        let some_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name("Some"),
            args: vec![HirPatArg {
                label: None,
                pattern: some_binding,
            }],
            span: span.clone(),
        });
        let some_body = self.alloc_expr(HirExpr::Local(some_local, span.clone()));

        // .None => trap (represented as Error for now — codegen will handle)
        let none_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: HirName::name("None"),
            args: Vec::new(),
            span: span.clone(),
        });
        let trap = self.alloc_expr(HirExpr::Error { span: span.clone() });

        self.alloc_expr(HirExpr::Match {
            scrutinee: lowered_operand,
            arms: vec![
                HirMatchArm {
                    pattern: some_pat,
                    guard: None,
                    body: some_body,
                },
                HirMatchArm {
                    pattern: none_pat,
                    guard: None,
                    body: trap,
                },
            ],
            source: MatchSource::UnwrapOp,
            span: span.clone(),
        })
    }

    // ===== Interpolated strings =====

    /// Desugar interpolated string to String concatenation.
    /// `"hello \(name)!"` → `"hello " + name.format() + "!"`
    ///
    /// For simplicity, we desugar to a chain of add operations.
    pub(crate) fn desugar_interpolated_string(
        &mut self,
        body: &AstBody,
        parts: &[StringPart],
        span: &Span,
    ) -> HirExprId {
        if parts.is_empty() {
            return self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::String {
                    value: String::new(),
                    escape_errors: Vec::new(),
                },
                span: span.clone(),
            });
        }

        // Convert each part to an expression
        let mut exprs: Vec<HirExprId> = Vec::new();

        for part in parts {
            match part {
                StringPart::Literal(text) => {
                    if !text.is_empty() {
                        // The parser currently emits the full interpolated string
                        // as a single token, so this `text` may contain unparsed
                        // `\(...)` interpolation syntax + surrounding quotes (see
                        // ast-builder/lower.rs `lower_interpolated_string` fallback).
                        // Skip escape decoding here — flagging `\(` as invalid
                        // would be wrong, and decoding `\n` etc. inside an opaque
                        // unparsed blob isn't meaningful. When the parser is taught
                        // to split interpolations, decode the structured parts.
                        exprs.push(self.alloc_expr(HirExpr::Literal {
                            value: HirLiteral::String {
                                value: text.clone(),
                                escape_errors: Vec::new(),
                            },
                            span: span.clone(),
                        }));
                    }
                },
                StringPart::Interpolation { expr, format: _ } => {
                    let lowered = self.lower_expr(body, *expr);
                    // Call .description() on the interpolated expression
                    let formatted = self.alloc_expr(HirExpr::MethodCall {
                        receiver: lowered,
                        method: HirName::name("description"),
                        type_args: None,
                        args: Vec::new(),
                        span: span.clone(),
                    });
                    exprs.push(formatted);
                },
            }
        }

        // Chain with + operator (Addable protocol)
        if exprs.is_empty() {
            return self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::String {
                    value: String::new(),
                    escape_errors: Vec::new(),
                },
                span: span.clone(),
            });
        }

        let mut result = exprs[0];
        for &next in &exprs[1..] {
            if let Some(protocol) = self.resolve_builtin(Builtin::AddOperatorProtocol) {
                result = self.alloc_expr(HirExpr::ProtocolCall {
                    receiver: result,
                    protocol,
                    method: HirName::name("add"),
                    type_args: None,
                    args: vec![HirCallArg {
                        label: None,
                        value: next,
                    }],
                    span: span.clone(),
                });
            }
        }

        result
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
