//! Desugaring helpers: operators, for-loops, while, try/throw, unwrap,
//! interpolated strings.
//!
//! Each desugaring function takes AST-level inputs and produces HIR nodes.
//! Protocol entities are resolved via name resolution.

use kestrel_ast::ast_body::*;
use kestrel_hir::body::*;
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

impl LowerCtx<'_> {
    // ===== Binary operators =====

    /// Desugar a binary expression. Dispatches to regular, short-circuit, or range.
    pub(crate) fn desugar_binary(
        &mut self,
        body: &AstBody,
        lhs: ExprId,
        op: &BinaryOp,
        rhs: ExprId,
        span: &Span,
    ) -> HirExprId {
        // Check short-circuit operators first
        if let Some((proto, method, label)) = lookup_short_circuit_op(op) {
            return self.desugar_short_circuit_op(body, proto, method, label, lhs, rhs, span);
        }

        // Regular binary operator → ProtocolCall
        if let Some((proto, method, label)) = lookup_binary_op(op) {
            return self.desugar_binary_op(body, proto, method, label, lhs, rhs, span);
        }

        // Fallback (shouldn't happen if tables are complete)
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    /// Desugar a regular binary operator to a ProtocolCall.
    fn desugar_binary_op(
        &mut self,
        body: &AstBody,
        protocol_name: &str,
        method_name: &str,
        label: Option<&str>,
        lhs: ExprId,
        rhs: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_lhs = self.lower_expr(body, lhs);
        let lowered_rhs = self.lower_expr(body, rhs);

        if let Some(protocol) = self.resolve_protocol(protocol_name) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_lhs,
                protocol,
                method: method_name.to_string(),
                type_args: None,
                args: vec![HirCallArg {
                    label: label.map(|l| l.to_string()),
                    value: lowered_rhs,
                }],
                span: span.clone(),
            })
        } else {
            self.alloc_expr(HirExpr::Error { span: span.clone() })
        }
    }

    /// Desugar a short-circuit operator (&&, ||, ??).
    /// The RHS is wrapped in a closure to prevent eager evaluation.
    fn desugar_short_circuit_op(
        &mut self,
        body: &AstBody,
        protocol_name: &str,
        method_name: &str,
        label: Option<&str>,
        lhs: ExprId,
        rhs: ExprId,
        span: &Span,
    ) -> HirExprId {
        let lowered_lhs = self.lower_expr(body, lhs);
        let lowered_rhs = self.lower_expr(body, rhs);

        // Wrap RHS in a closure: { () -> T in rhs }
        let rhs_closure = self.alloc_expr(HirExpr::Closure {
            params: Vec::new(),
            body: HirBlock {
                stmts: Vec::new(),
                tail_expr: Some(lowered_rhs),
            },
            span: span.clone(),
        });

        if let Some(protocol) = self.resolve_protocol(protocol_name) {
            self.alloc_expr(HirExpr::ProtocolCall {
                receiver: lowered_lhs,
                protocol,
                method: method_name.to_string(),
                type_args: None,
                args: vec![HirCallArg {
                    label: label.map(|l| l.to_string()),
                    value: rhs_closure,
                }],
                span: span.clone(),
            })
        } else {
            self.alloc_expr(HirExpr::Error { span: span.clone() })
        }
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

        if let Some(protocol) = self.resolve_protocol("And") {
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

        if let Some((proto, method)) = lookup_unary_op(op) {
            if let Some(protocol) = self.resolve_protocol(proto) {
                return self.alloc_expr(HirExpr::ProtocolCall {
                    receiver: lowered_operand,
                    protocol,
                    method: method.to_string(),
                    type_args: None,
                    args: Vec::new(),
                    span: span.clone(),
                });
            }
        }

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
            if let Some(protocol) = self.resolve_protocol(proto) {
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
        let negated = if let Some(protocol) = self.resolve_protocol("Not") {
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
        let negated = if let Some(protocol) = self.resolve_protocol("Not") {
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
    /// let $iter = iterable.iterate()
    /// loop {
    ///     match $iter.next() {
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

        // $iter = iterable.iterate()
        let iterate_call = self.alloc_expr(HirExpr::MethodCall {
            receiver: lowered_iterable,
            method: "iterate".to_string(),
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

        // $iter.next()
        let iter_ref = self.alloc_expr(HirExpr::Local(iter_local, span.clone()));
        let next_call = self.alloc_expr(HirExpr::MethodCall {
            receiver: iter_ref,
            method: "next".to_string(),
            type_args: None,
            args: Vec::new(),
            span: span.clone(),
        });

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

        // Body expression wrapping the block
        let body_expr = if let Some(tail) = lowered_for_body.tail_expr {
            if lowered_for_body.stmts.is_empty() {
                tail
            } else {
                // Wrap in a block-like expression using tuple as unit return
                let unit = self.alloc_expr(HirExpr::Tuple {
                    elements: Vec::new(),
                    span: span.clone(),
                });
                unit
            }
        } else {
            self.alloc_expr(HirExpr::Tuple {
                elements: Vec::new(),
                span: span.clone(),
            })
        };

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

        let match_stmt = self.alloc_stmt(HirStmt::Expr {
            expr: match_expr,
            span: span.clone(),
        });

        // Put the iter_let inside the loop body so we return a single expression
        self.alloc_expr(HirExpr::Loop {
            label: label.map(|l| l.to_string()),
            body: HirBlock {
                stmts: vec![iter_let, match_stmt],
                tail_expr: None,
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
                }
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
                }
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
            if let Some(protocol) = self.resolve_protocol("Addable") {
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

    // ===== Protocol resolution helper =====

    /// Resolve a protocol name to its entity via type path resolution.
    fn resolve_protocol(&self, name: &str) -> Option<kestrel_hecs::Entity> {
        let result = self.ctx.query(ResolveTypePath {
            segments: vec![name.to_string()],
            context: self.owner,
            root: self.root,
        });
        match result {
            TypeResolution::Found(entity) => Some(entity),
            _ => None,
        }
    }
}

// ===== Operator table lookups =====

/// Look up protocol for a binary operator.
fn lookup_binary_op(op: &BinaryOp) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_binary_op(op)
}

/// Look up protocol for a short-circuit operator.
fn lookup_short_circuit_op(
    op: &BinaryOp,
) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_short_circuit_op(op)
}

/// Look up protocol for a unary operator.
fn lookup_unary_op(op: &UnaryOp) -> Option<(&'static str, &'static str)> {
    kestrel_hir::body::lookup_unary_op(op)
}

/// Look up protocol for a compound assignment operator.
fn lookup_compound_assign_op(
    op: &CompoundAssignOp,
) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    kestrel_hir::body::lookup_compound_assign_op(op)
}
