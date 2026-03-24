//! Desugaring helpers: operators, for-loops, while, try/throw, unwrap,
//! interpolated strings.
//!
//! Each desugaring function takes AST-level inputs and produces HIR nodes.
//! Protocol entities are resolved via ResolveBuiltin (entity IDs, not strings).

use kestrel_ast::ast_body::*;
use kestrel_hir::body::*;
use kestrel_hir::Builtin;
use kestrel_name_res::ResolveBuiltin;
use kestrel_reporting2::{Diagnostic, Label};
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

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
                    method: method.to_string(),
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
                    method: method.to_string(),
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
                method: "logicalAnd".to_string(),
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
                method: method.to_string(),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            });
        }

        self.ctx.accumulate(Diagnostic::error()
            .with_message(format!("unsupported unary operator '{}'", unary_op_symbol(op)))
            .with_labels(vec![
                Label::primary(span.file_id, span.range()),
            ])
            .with_notes(vec!["is the standard library imported?".to_string()]));
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    // ===== Compound assignment =====

    /// Desugar compound assignment (+=, -=, etc.) to a ProtocolCall.
    pub(crate) fn desugar_compound_assign(
        &mut self,
        body: &AstBody,
        lhs: ExprId,
        op: &CompoundAssignOp,
        rhs: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_lhs = self.lower_expr(body, lhs);
        let lowered_rhs = self.lower_expr(body, rhs);

        if let Some((proto, method, label)) = lookup_compound_assign_op(op) {
            if let Some(protocol) = self.resolve_builtin(proto) {
                return self.alloc_expr(HirExpr::ProtocolCall {
                    receiver: lowered_lhs,
                    protocol,
                    method: method.to_string(),
                    type_args: None,
                    args: vec![HirCallArg {
                        label: label.map(|l| l.to_string()),
                        value: lowered_rhs,
                    }],
                    span: span.clone(),
                });
            }
        }

        self.ctx.accumulate(Diagnostic::error()
            .with_message(format!("unsupported compound assignment operator '{}'", compound_assign_op_symbol(op)))
            .with_labels(vec![
                Label::primary(span.file_id, span.range()),
            ])
            .with_notes(vec!["is the standard library imported?".to_string()]));
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    // ===== While → Loop =====

    /// Desugar `while condition { body }` → `loop { if !condition { break }; body }`
    pub(crate) fn desugar_while(
        &mut self,
        body: &AstBody,
        label: Option<&str>,
        condition: ExprId,
        while_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        let lowered_cond = self.lower_expr(body, condition);

        // Negate condition: !condition
        let negated = if let Some(protocol) = self.resolve_builtin(Builtin::LogicalNotOperatorProtocol) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_cond,
                protocol,
                method: "logicalNot".to_string(),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        } else {
            lowered_cond
        };

        let break_expr = self.alloc_expr(HirExpr::Break {
            label: label.map(|l| l.to_string()),
            span: span.clone(),
        });

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

        // Lower while body
        let lowered_body = self.lower_block(body, while_body);

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
        // Lower conditions to a boolean expression
        let cond = self.lower_if_conditions(body, conditions, span);

        let break_expr = self.alloc_expr(HirExpr::Break {
            label: label.map(|l| l.to_string()),
            span: span.clone(),
        });

        // Negate: if !cond { break }
        let negated = if let Some(protocol) = self.resolve_builtin(Builtin::LogicalNotOperatorProtocol) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: cond,
                protocol,
                method: "logicalNot".to_string(),
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

        let lowered_body = self.lower_block(body, while_body);

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

        // $iter = iterable.iter() via Iterable protocol
        let iterate_call = if let Some(protocol) = self.resolve_builtin(Builtin::IterableProtocol) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_iterable,
                protocol,
                method: "iter".to_string(),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        } else {
            self.alloc_expr(HirExpr::MethodCall {
                receiver: lowered_iterable,
                method: "iter".to_string(),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        };

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
                method: "next".to_string(),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        } else {
            self.alloc_expr(HirExpr::MethodCall {
                receiver: iter_ref,
                method: "next".to_string(),
                type_args: None,
                args: Vec::new(),
                span: span.clone(),
            })
        };

        // Pattern for .Some(pattern)
        self.push_scope();
        let lowered_pat = self.lower_pat(body, pattern);
        let some_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: "Some".to_string(),
            args: vec![HirPatArg {
                label: None,
                pattern: lowered_pat,
            }],
            span: span.clone(),
        });

        // Lower for body
        let lowered_for_body = self.lower_block(body, for_body);
        self.pop_scope();

        // Wrap the for-body as a block expression so all statements are reachable
        let body_expr = self.alloc_expr(HirExpr::Block {
            body: lowered_for_body,
            span: span.clone(),
        });

        // .None => break
        let none_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: "None".to_string(),
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
            span: span.clone(),
        });

        // Track this match as originating from a for-loop so the
        // for_loop_pattern analyzer can extract and check the user's pattern.
        self.for_loop_matches.push(match_expr);

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
        self.alloc_expr(HirExpr::Block {
            body: HirBlock {
                stmts: vec![iter_let],
                tail_expr: Some(loop_expr),
            },
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
    pub(crate) fn desugar_try(
        &mut self,
        body: &AstBody,
        operand: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_operand = self.lower_expr(body, operand);

        // .Ok($v) => $v
        let ok_local = self.define_local("$try_ok", false, span.clone());
        let ok_binding = self.alloc_pat(HirPat::Binding {
            local: ok_local,
            span: span.clone(),
        });
        let ok_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: "Ok".to_string(),
            args: vec![HirPatArg {
                label: None,
                pattern: ok_binding,
            }],
            span: span.clone(),
        });
        let ok_body = self.alloc_expr(HirExpr::Local(ok_local, span.clone()));

        // .Err($e) => return .Err($e)
        let err_local = self.define_local("$try_err", false, span.clone());
        let err_binding = self.alloc_pat(HirPat::Binding {
            local: err_local,
            span: span.clone(),
        });
        let err_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: "Err".to_string(),
            args: vec![HirPatArg {
                label: None,
                pattern: err_binding,
            }],
            span: span.clone(),
        });
        let err_ref = self.alloc_expr(HirExpr::Local(err_local, span.clone()));
        let err_wrap = self.alloc_expr(HirExpr::ImplicitMember {
            name: "Err".to_string(),
            args: Some(vec![HirCallArg {
                label: None,
                value: err_ref,
            }]),
            span: span.clone(),
        });
        let return_err = self.alloc_expr(HirExpr::Return {
            value: Some(err_wrap),
            span: span.clone(),
        });

        self.alloc_expr(HirExpr::Match {
            scrutinee: lowered_operand,
            arms: vec![
                HirMatchArm {
                    pattern: ok_pat,
                    guard: None,
                    body: ok_body,
                },
                HirMatchArm {
                    pattern: err_pat,
                    guard: None,
                    body: return_err,
                },
            ],
            span: span.clone(),
        })
    }

    /// Desugar `throw value` → `return .Err(value)`
    pub(crate) fn desugar_throw(
        &mut self,
        body: &AstBody,
        value: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_value = self.lower_expr(body, value);

        let err_wrap = self.alloc_expr(HirExpr::ImplicitMember {
            name: "Err".to_string(),
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
            name: "Some".to_string(),
            args: vec![HirPatArg {
                label: None,
                pattern: some_binding,
            }],
            span: span.clone(),
        });
        let some_body = self.alloc_expr(HirExpr::Local(some_local, span.clone()));

        // .None => trap (represented as Error for now — codegen will handle)
        let none_pat = self.alloc_pat(HirPat::ImplicitVariant {
            name: "None".to_string(),
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
                value: HirLiteral::String(String::new()),
                span: span.clone(),
            });
        }

        // Convert each part to an expression
        let mut exprs: Vec<HirExprId> = Vec::new();

        for part in parts {
            match part {
                StringPart::Literal(text) => {
                    if !text.is_empty() {
                        exprs.push(self.alloc_expr(HirExpr::Literal {
                            value: HirLiteral::String(text.clone()),
                            span: span.clone(),
                        }));
                    }
                },
                StringPart::Interpolation { expr, format: _ } => {
                    let lowered = self.lower_expr(body, *expr);
                    // Call .description() on the interpolated expression
                    let formatted = self.alloc_expr(HirExpr::MethodCall {
                        receiver: lowered,
                        method: "description".to_string(),
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
                value: HirLiteral::String(String::new()),
                span: span.clone(),
            });
        }

        let mut result = exprs[0];
        for &next in &exprs[1..] {
            if let Some(protocol) = self.resolve_builtin(Builtin::AddOperatorProtocol) {
                result = self.alloc_expr(HirExpr::ProtocolCall {
                    receiver: result,
                    protocol,
                    method: "add".to_string(),
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
        self.ctx.accumulate(Diagnostic::error()
            .with_message(format!("unsupported binary operator '{}'", binary_op_symbol(op)))
            .with_labels(vec![
                Label::primary(span.file_id, span.range()),
            ])
            .with_notes(vec!["is the standard library imported?".to_string()]));
    }
}

// ===== Operator table lookups =====

/// Look up protocol for a binary operator.
fn lookup_binary_op(op: &BinaryOp) -> Option<(Builtin, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_binary_op(op)
}

/// Look up protocol for a short-circuit operator.
fn lookup_short_circuit_op(
    op: &BinaryOp,
) -> Option<(Builtin, &'static str, Option<&'static str>)> {
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
        UnaryOp::BitNot => "~",
        UnaryOp::LogicalNot => "!",
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
