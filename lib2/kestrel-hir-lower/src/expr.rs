//! Expression lowering: AstExpr → HirExpr.
//!
//! The core of HIR lowering. Resolves paths to entities/locals,
//! dispatches operator desugaring, and handles control flow.

use kestrel_ast::ast_body::*;
use kestrel_ast_builder::{DeclSpan, Name};
use kestrel_hir::body::*;
use kestrel_name_res::{ResolveValuePath, ValueResolution};
use kestrel_reporting2::{Diagnostic, Label};
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

impl LowerCtx<'_> {
    /// Lower an AST expression to an HIR expression.
    pub fn lower_expr(&mut self, body: &AstBody, id: ExprId) -> HirExprId {
        let expr = body.exprs[id].clone();
        match expr {
            AstExpr::Literal { kind, span } => self.lower_literal(&kind, &span),
            AstExpr::InterpolatedString { parts, span } => {
                self.desugar_interpolated_string(body, &parts, &span)
            },
            AstExpr::Array { elements, span } => {
                let lowered: Vec<HirExprId> =
                    elements.iter().map(|&e| self.lower_expr(body, e)).collect();
                self.alloc_expr(HirExpr::Array {
                    elements: lowered,
                    span,
                })
            },
            AstExpr::Dictionary { entries, span } => {
                let lowered: Vec<HirDictEntry> = entries
                    .iter()
                    .map(|e| HirDictEntry {
                        key: self.lower_expr(body, e.key),
                        value: self.lower_expr(body, e.value),
                    })
                    .collect();
                self.alloc_expr(HirExpr::Dict {
                    entries: lowered,
                    span,
                })
            },
            AstExpr::Tuple { elements, span } => {
                let lowered: Vec<HirExprId> =
                    elements.iter().map(|&e| self.lower_expr(body, e)).collect();
                self.alloc_expr(HirExpr::Tuple {
                    elements: lowered,
                    span,
                })
            },
            AstExpr::Path { segments, span } => self.lower_path(body, &segments, &span),
            AstExpr::MemberAccess {
                base,
                member,
                type_args: _,
                span,
            } => {
                // Standalone member access (not callee of Call) → Field
                let lowered_base = self.lower_expr(body, base);
                self.alloc_expr(HirExpr::Field {
                    base: lowered_base,
                    name: member,
                    span,
                })
            },
            AstExpr::TupleIndex { base, index, span } => {
                let lowered_base = self.lower_expr(body, base);
                self.alloc_expr(HirExpr::TupleIndex {
                    base: lowered_base,
                    index,
                    span,
                })
            },
            AstExpr::ImplicitMember {
                member,
                arguments,
                span,
            } => {
                let lowered_args = arguments.map(|args| self.lower_call_args(body, &args));
                self.alloc_expr(HirExpr::ImplicitMember {
                    name: member,
                    args: lowered_args,
                    span,
                })
            },
            AstExpr::Unary { op, operand, span } => {
                self.desugar_unary_op(body, &op, operand, &span)
            },
            AstExpr::Postfix { operand, op, span } => match op {
                PostfixOp::Unwrap => self.desugar_unwrap(body, operand, &span),
            },
            AstExpr::Binary { .. } => {
                self.lower_binary_with_precedence(body, id)
            },
            AstExpr::Assignment { lhs, rhs, span } => {
                let target = self.lower_expr(body, lhs);
                let value = self.lower_expr(body, rhs);
                self.alloc_expr(HirExpr::Assign {
                    target,
                    value,
                    span,
                })
            },
            AstExpr::CompoundAssignment { lhs, op, rhs, span } => {
                self.desugar_compound_assign(body, lhs, &op, rhs, &span)
            },
            AstExpr::Call {
                callee,
                arguments,
                span,
            } => self.lower_call(body, callee, &arguments, &span),
            AstExpr::If {
                conditions,
                then_body,
                else_body,
                span,
            } => self.lower_if(body, &conditions, &then_body, else_body.as_ref(), &span),
            AstExpr::While {
                label,
                condition,
                body: while_body,
                span,
            } => self.desugar_while(body, label.as_deref(), condition, &while_body, &span),
            AstExpr::WhileLet {
                label,
                conditions,
                body: while_body,
                span,
            } => self.desugar_while_let(body, label.as_deref(), &conditions, &while_body, &span),
            AstExpr::Loop {
                label,
                body: loop_body,
                span,
            } => {
                self.push_loop(label.as_deref());
                let lowered = self.lower_block(body, &loop_body);
                self.pop_loop();
                self.alloc_expr(HirExpr::Loop {
                    label,
                    body: lowered,
                    span,
                })
            },
            AstExpr::For {
                label,
                pattern,
                iterable,
                body: for_body,
                span,
            } => self.desugar_for_loop(body, label.as_deref(), pattern, iterable, &for_body, &span),
            AstExpr::Break { label, span } => {
                self.validate_break_continue("break", &label, &span);
                self.alloc_expr(HirExpr::Break { label, span })
            },
            AstExpr::Continue { label, span } => {
                self.validate_break_continue("continue", &label, &span);
                self.alloc_expr(HirExpr::Continue { label, span })
            },
            AstExpr::Return { value, span } => {
                let lowered = value.map(|v| self.lower_expr(body, v));
                self.alloc_expr(HirExpr::Return {
                    value: lowered,
                    span,
                })
            },
            AstExpr::Throw { value, span } => self.desugar_throw(body, value, &span),
            AstExpr::Try { operand, span } => self.desugar_try(body, operand, &span),
            AstExpr::Closure {
                params,
                body: closure_body,
                span,
            } => self.lower_closure(body, &params, &closure_body, &span),
            AstExpr::Match {
                scrutinee,
                arms,
                span,
            } => self.lower_match(body, scrutinee, &arms, &span),
            AstExpr::Block { body: block, span } => {
                let lowered = self.lower_block(body, &block);
                self.alloc_expr(HirExpr::Block {
                    body: lowered,
                    span: span.clone(),
                })
            }
            AstExpr::Error { span } => self.alloc_expr(HirExpr::Error { span }),
        }
    }

    /// Lower a literal expression.
    fn lower_literal(&mut self, kind: &AstLiteral, span: &Span) -> HirExprId {
        let value = match kind {
            AstLiteral::Integer(s) => HirLiteral::Integer(crate::pat::parse_int(s)),
            AstLiteral::Float(s) => HirLiteral::Float(crate::pat::parse_float(s)),
            AstLiteral::String(s) | AstLiteral::RawString(s) => HirLiteral::String(s.clone()),
            AstLiteral::Char(s) => HirLiteral::Char(crate::pat::parse_char_validated(s, span, self.ctx)),
            AstLiteral::Bool(b) => HirLiteral::Bool(*b),
            AstLiteral::Null => HirLiteral::Null,
            AstLiteral::Unit => {
                return self.alloc_expr(HirExpr::Tuple {
                    elements: Vec::new(),
                    span: span.clone(),
                });
            },
        };
        self.alloc_expr(HirExpr::Literal {
            value,
            span: span.clone(),
        })
    }

    /// Lower a path expression. Check locals first, then name resolution.
    fn lower_path(
        &mut self,
        _body: &AstBody,
        segments: &[ExprPathSegment],
        span: &Span,
    ) -> HirExprId {
        if segments.is_empty() {
            return self.alloc_expr(HirExpr::Error { span: span.clone() });
        }

        let first = &segments[0];

        // Check if first segment is a local (covers self, params, let/var bindings).
        // Remaining segments become field accesses — type inference resolves them later.
        if first.type_args.is_none() {
            if let Some(local_id) = self.lookup_local(&first.name) {
                let mut current = self.alloc_expr(HirExpr::Local(local_id, first.span.clone()));
                for seg in &segments[1..] {
                    current = self.alloc_expr(HirExpr::Field {
                        base: current,
                        name: seg.name.clone(),
                        span: seg.span.clone(),
                    });
                }
                return current;
            }
        } else if self.lookup_local(&first.name).is_some() {
            // Local variable with type args (e.g., `x[Int]`) — variables don't accept type args
            self.ctx.accumulate(
                kestrel_reporting2::Diagnostic::error()
                    .with_message(format!("variable '{}' does not accept type arguments", first.name))
                    .with_labels(vec![
                        kestrel_reporting2::Label::primary(first.span.file_id, first.span.range())
                            .with_message("type arguments not allowed on variables"),
                    ])
            );
            return self.alloc_expr(HirExpr::Error { span: span.clone() });
        }

        // For multi-segment paths, check if the first segment is a type parameter.
        // Type parameters can't be resolved as multi-segment paths (T.create),
        // so emit Def(T) + Field/MethodCall chain for the solver to resolve via bounds.
        if segments.len() > 1 {
            let first_result = self.ctx.query(ResolveValuePath {
                segments: vec![segments[0].name.clone()],
                context: self.owner,
                root: self.root,
            });
            if let ValueResolution::TypeParameter(entity) = first_result {
                let first_type_args: Vec<kestrel_hir::ty::HirTy> = segments[0]
                    .type_args.iter().flatten()
                    .map(|t| self.lower_type(t))
                    .collect();
                let mut current = self.alloc_expr(HirExpr::Def(entity, first_type_args, segments[0].span.clone()));
                for seg in &segments[1..] {
                    current = self.alloc_expr(HirExpr::Field {
                        base: current,
                        name: seg.name.clone(),
                        span: seg.span.clone(),
                    });
                }
                return current;
            }
        }

        // Fall back to name resolution
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let result = self.ctx.query(ResolveValuePath {
            segments: seg_names,
            context: self.owner,
            root: self.root,
        });

        // Check for empty type argument brackets (e.g., `identity[]`)
        for seg in segments {
            if let Some(args) = &seg.type_args {
                if args.is_empty() {
                    self.ctx.accumulate(
                        kestrel_reporting2::Diagnostic::error()
                            .with_message("empty type argument list")
                            .with_labels(vec![
                                kestrel_reporting2::Label::primary(seg.span.file_id, seg.span.range())
                                    .with_message("expected at least one type argument"),
                            ])
                    );
                    return self.alloc_expr(HirExpr::Error { span: span.clone() });
                }
            }
        }

        // Collect explicit type args from all path segments (e.g., Pointer[UInt8])
        let explicit_type_args: Vec<kestrel_hir::ty::HirTy> = segments
            .iter()
            .flat_map(|s| s.type_args.iter().flatten())
            .map(|t| self.lower_type(t))
            .collect();

        match result {
            ValueResolution::Def(entity) | ValueResolution::TypeParameter(entity) => {
                self.alloc_expr(HirExpr::Def(entity, explicit_type_args.clone(), span.clone()))
            },
            ValueResolution::Overloaded(entities) => {
                // Preserve full overload set — type inference disambiguates at call site
                self.alloc_expr(HirExpr::OverloadSet {
                    candidates: entities,
                    type_args: explicit_type_args.clone(),
                    span: span.clone(),
                })
            },
            ValueResolution::EnumCaseValue { entity, .. } => {
                self.alloc_expr(HirExpr::Def(entity, explicit_type_args.clone(), span.clone()))
            },
            ValueResolution::FieldValue { entity, .. } => {
                self.alloc_expr(HirExpr::Def(entity, vec![], span.clone()))
            },
            ValueResolution::AssociatedType { entity, .. } => {
                self.alloc_expr(HirExpr::Def(entity, vec![], span.clone()))
            },
            ValueResolution::AssociatedTypeStaticMember { entity: _, assoc_type } => {
                // Emit Field { base: Def(assoc_type), name: member } so the solver
                // can do Self-substitution (e.g., Item.zero → Member(Item, "zero"))
                let member_name = segments.last().map(|s| s.name.clone()).unwrap_or_default();
                let base = self.alloc_expr(HirExpr::Def(assoc_type, vec![], span.clone()));
                self.alloc_expr(HirExpr::Field {
                    base,
                    name: member_name,
                    span: span.clone(),
                })
            },
            ValueResolution::Ambiguous(entities) => {
                let path_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
                // Primary label on the use site
                let mut labels = vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("{} symbols with this name in scope", entities.len())),
                ];
                // Secondary labels on each candidate's declaration
                for &entity in &entities {
                    if let Some(decl) = self.ctx.get::<DeclSpan>(entity) {
                        let name = self.ctx.get::<Name>(entity)
                            .map(|n| format!("declared here as '{}'", n.0))
                            .unwrap_or_else(|| "declared here".to_string());
                        labels.push(
                            Label::secondary(decl.0.file_id, decl.0.range()).with_message(name),
                        );
                    }
                }
                let diag = Diagnostic::error()
                    .with_message(format!("ambiguous name '{path_name}'"))
                    .with_labels(labels)
                    .with_notes(vec!["use a fully qualified path to disambiguate".to_string()]);
                self.ctx.accumulate(diag);
                self.alloc_expr(HirExpr::Error { span: span.clone() })
            },
            ValueResolution::NotFound(ref seg) => {
                let path_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
                self.ctx.accumulate(Diagnostic::error()
                    .with_message(format!("undefined name '{path_name}'"))
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range())
                            .with_message(format!("not found (failed at '{seg}')")),
                    ]));
                self.alloc_expr(HirExpr::Error { span: span.clone() })
            },
        }
    }

    /// Lower a call expression. Detect method calls vs direct calls.
    ///
    /// Method calls come from two AST shapes:
    /// 1. `AstExpr::MemberAccess { base, member }` — computed-base access like `expr.method()`
    /// 2. `AstExpr::Path { segments: [local, method] }` — when parser emits `local.method()`
    ///    as a path (first segment is a local variable, last segment is the method)
    fn lower_call(
        &mut self,
        body: &AstBody,
        callee: ExprId,
        arguments: &[CallArg],
        span: &Span,
    ) -> HirExprId {
        let lowered_args = self.lower_call_args(body, arguments);

        // Check if callee is a member access → method call
        match &body.exprs[callee] {
            AstExpr::MemberAccess {
                base,
                member,
                type_args,
                ..
            } => {
                let base = *base;
                let member = member.clone();
                let type_args = type_args.clone();

                // Check if this is a static method call on a type: Type[Args].staticMethod()
                // Resolve directly as Call(Def) instead of MethodCall so type inference
                // doesn't filter out the static method during member resolution.
                if let Some((static_entity, base_type_args)) =
                    self.try_resolve_static_call(body, base, &member)
                {
                    // Include both struct type_args and method's own type_args
                    let mut all_type_args = base_type_args;
                    if let Some(ref method_args) = type_args {
                        all_type_args.extend(method_args.iter().map(|t| self.lower_type(t)));
                    }
                    let callee = self.alloc_expr(
                        HirExpr::Def(static_entity, all_type_args, span.clone()),
                    );
                    return self.alloc_expr(HirExpr::Call {
                        callee,
                        args: lowered_args,
                        span: span.clone(),
                    });
                }

                // Instance method call
                let lowered_base = self.lower_expr(body, base);
                let lowered_type_args =
                    type_args.map(|args| args.iter().map(|t| self.lower_type(t)).collect());

                self.alloc_expr(HirExpr::MethodCall {
                    receiver: lowered_base,
                    method: member,
                    type_args: lowered_type_args,
                    args: lowered_args,
                    span: span.clone(),
                })
            },

            // Path where first segment is a local: `local.method(args)` → MethodCall
            // The parser emits this as Path when the base is a simple name.
            AstExpr::Path { segments, .. } if segments.len() >= 2 => {
                let first = &segments[0];
                if first.type_args.is_none() {
                    if let Some(_) = self.lookup_local(&first.name) {
                        // Lower all segments except the last as nested Field accesses
                        let last = &segments[segments.len() - 1];
                        let method = last.name.clone();
                        let type_args = last.type_args.clone();

                        // Build receiver from first N-1 segments
                        let current = self.lower_path_prefix(segments);

                        let lowered_type_args = type_args
                            .map(|args| args.iter().map(|t| self.lower_type(t)).collect());

                        return self.alloc_expr(HirExpr::MethodCall {
                            receiver: current,
                            method,
                            type_args: lowered_type_args,
                            args: lowered_args,
                            span: span.clone(),
                        });
                    }
                }

                // Not a local-based path — check for static method call.
                // For Type[Args].staticMethod() or mod.Type[Args].staticMethod(),
                // resolve the static method directly so type inference doesn't need
                // to handle it as a member constraint.
                {
                    let last = &segments[segments.len() - 1];
                    if let Some((static_entity, base_type_args)) =
                        self.try_resolve_static_call_from_segments(segments, &last.name)
                    {
                        let callee = self.alloc_expr(
                            HirExpr::Def(static_entity, base_type_args, span.clone()),
                        );
                        return self.alloc_expr(HirExpr::Call {
                            callee,
                            args: lowered_args,
                            span: span.clone(),
                        });
                    }
                }

                // Type parameter static call: T.method(args) → MethodCall
                // The solver resolves the method via protocol bounds.
                {
                    let first_result = self.ctx.query(ResolveValuePath {
                        segments: vec![segments[0].name.clone()],
                        context: self.owner,
                        root: self.root,
                    });
                    if let ValueResolution::TypeParameter(entity) = first_result {
                        let first_type_args: Vec<kestrel_hir::ty::HirTy> = segments[0]
                            .type_args.iter().flatten()
                            .map(|t| self.lower_type(t))
                            .collect();
                        let receiver = self.alloc_expr(
                            HirExpr::Def(entity, first_type_args, segments[0].span.clone()),
                        );
                        let last = &segments[segments.len() - 1];
                        let lowered_type_args = last.type_args.as_ref()
                            .map(|args| args.iter().map(|t| self.lower_type(t)).collect());
                        return self.alloc_expr(HirExpr::MethodCall {
                            receiver,
                            method: last.name.clone(),
                            type_args: lowered_type_args,
                            args: lowered_args,
                            span: span.clone(),
                        });
                    }
                }

                // Regular direct call (lowered as-is)
                let lowered_callee = self.lower_expr(body, callee);
                self.alloc_expr(HirExpr::Call {
                    callee: lowered_callee,
                    args: lowered_args,
                    span: span.clone(),
                })
            },

            _ => {
                // Direct call
                let lowered_callee = self.lower_expr(body, callee);
                self.alloc_expr(HirExpr::Call {
                    callee: lowered_callee,
                    args: lowered_args,
                    span: span.clone(),
                })
            },
        }
    }

    /// Check if a multi-segment path ending in `member` is a static method call.
    /// Resolves all segments except the last as a type, then searches for a static
    /// method named `member` on that type.
    fn try_resolve_static_call_from_segments(
        &mut self,
        segments: &[ExprPathSegment],
        member: &str,
    ) -> Option<(kestrel_hecs::Entity, Vec<kestrel_hir::ty::HirTy>)> {
        use kestrel_ast_builder::{Name, NodeKind, Static};

        if segments.len() < 2 {
            return None;
        }

        // Resolve all segments except the last as a type path
        let type_segments: Vec<String> = segments[..segments.len() - 1]
            .iter()
            .map(|s| s.name.clone())
            .collect();

        let result = self.ctx.query(ResolveValuePath {
            segments: type_segments,
            context: self.owner,
            root: self.root,
        });

        let type_entity = match result {
            ValueResolution::Def(entity) => entity,
            _ => return None,
        };

        // Must be a struct or enum
        let kind = self.ctx.get::<NodeKind>(type_entity)?;
        if !matches!(kind, NodeKind::Struct | NodeKind::Enum) {
            return None;
        }

        // Search children for a static function matching the member name
        let children: Vec<_> = self.ctx.children_of(type_entity).to_vec();
        for &child in &children {
            if self.ctx.get::<NodeKind>(child) != Some(&NodeKind::Function) {
                continue;
            }
            if self.ctx.get::<Static>(child).is_none() {
                continue;
            }
            let Some(child_name) = self.ctx.get::<Name>(child) else { continue };
            if child_name.0 == member {
                // Collect struct type_args from base segments + method type_args from last segment
                let mut type_args: Vec<kestrel_hir::ty::HirTy> = segments[..segments.len() - 1]
                    .iter()
                    .flat_map(|s| s.type_args.iter().flatten())
                    .map(|t| self.lower_type(t))
                    .collect();
                // Append the method's own type_args (e.g., [T] in Layout.array[T])
                let last = &segments[segments.len() - 1];
                if let Some(ref method_args) = last.type_args {
                    type_args.extend(method_args.iter().map(|t| self.lower_type(t)));
                }
                return Some((child, type_args));
            }
        }

        None
    }

    /// Check if `base_expr.member` is a static method call on a type.
    /// Returns `Some((static_method_entity, type_args))` if the base resolves to
    /// a struct/enum and the member is a static method on it.
    fn try_resolve_static_call(
        &mut self,
        body: &AstBody,
        base_expr: ExprId,
        member: &str,
    ) -> Option<(kestrel_hecs::Entity, Vec<kestrel_hir::ty::HirTy>)> {
        use kestrel_ast_builder::{Name, NodeKind, Static};

        // Base must be a Path expression (type reference)
        let AstExpr::Path { segments, .. } = &body.exprs[base_expr] else {
            return None;
        };

        // Resolve the base path to an entity
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let result = self.ctx.query(ResolveValuePath {
            segments: seg_names,
            context: self.owner,
            root: self.root,
        });

        let base_entity = match result {
            ValueResolution::Def(entity) => entity,
            _ => return None,
        };

        // Must be a struct or enum
        let kind = self.ctx.get::<NodeKind>(base_entity)?;
        if !matches!(kind, NodeKind::Struct | NodeKind::Enum) {
            return None;
        }

        // Search children for a static function matching the member name
        for &child in self.ctx.children_of(base_entity) {
            if self.ctx.get::<NodeKind>(child) != Some(&NodeKind::Function) {
                continue;
            }
            if self.ctx.get::<Static>(child).is_none() {
                continue;
            }
            let child_name = self.ctx.get::<Name>(child)?;
            if child_name.0 == member {
                // Collect type args from the base path segments
                let type_args: Vec<kestrel_hir::ty::HirTy> = segments
                    .iter()
                    .flat_map(|s| s.type_args.iter().flatten())
                    .map(|t| self.lower_type(t))
                    .collect();
                return Some((child, type_args));
            }
        }

        None
    }

    /// Lower Path segments except the last one as receiver.
    /// For `[a, b, c]` returns `Field { base: Field { base: Local(a), name: "b" }, name: ... }`
    /// but stops before the last segment.
    fn lower_path_prefix(
        &mut self,
        segments: &[ExprPathSegment],
    ) -> HirExprId {
        let first = &segments[0];
        let local_id = self.lookup_local(&first.name).unwrap();
        let mut current = self.alloc_expr(HirExpr::Local(local_id, first.span.clone()));
        // Build Field chain for all segments except first and last
        for seg in &segments[1..segments.len() - 1] {
            current = self.alloc_expr(HirExpr::Field {
                base: current,
                name: seg.name.clone(),
                span: seg.span.clone(),
            });
        }
        current
    }

    // ===== Binary expression Pratt parsing =====

    /// Flatten a nested Binary chain into operands + operators, then Pratt parse
    /// to produce correct precedence.
    fn lower_binary_with_precedence(&mut self, body: &AstBody, expr_id: ExprId) -> HirExprId {
        let mut operands: Vec<ExprId> = Vec::new();
        let mut operators: Vec<(BinaryOp, Span)> = Vec::new();

        Self::flatten_binary(body, expr_id, &mut operands, &mut operators);

        if operands.len() == 1 {
            return self.lower_expr(body, operands[0]);
        }

        self.pratt_parse(
            body,
            &mut operands.into_iter().peekable(),
            &mut operators.into_iter().peekable(),
            0,
        )
    }

    /// Recursively flatten nested Binary exprs into flat operand/operator lists.
    fn flatten_binary(
        body: &AstBody,
        expr_id: ExprId,
        operands: &mut Vec<ExprId>,
        operators: &mut Vec<(BinaryOp, Span)>,
    ) {
        match &body.exprs[expr_id] {
            AstExpr::Binary { lhs, op, rhs, span } => {
                let (lhs, op, rhs, span) = (*lhs, op.clone(), *rhs, span.clone());
                Self::flatten_binary(body, lhs, operands, operators);
                operators.push((op, span));
                Self::flatten_binary(body, rhs, operands, operators);
            }
            _ => {
                operands.push(expr_id);
            }
        }
    }

    /// Pratt parser: precedence climbing over flat operand/operator lists.
    fn pratt_parse<I, J>(
        &mut self,
        body: &AstBody,
        operands: &mut std::iter::Peekable<I>,
        operators: &mut std::iter::Peekable<J>,
        min_bp: u8,
    ) -> HirExprId
    where
        I: Iterator<Item = ExprId>,
        J: Iterator<Item = (BinaryOp, Span)>,
    {
        let first = operands.next().expect("pratt_parse: no operand");
        let mut lhs = self.lower_expr(body, first);

        loop {
            let Some(&(ref op, _)) = operators.peek() else {
                break;
            };
            let prec = op.precedence();
            if prec < min_bp {
                break;
            }

            let (op, span) = operators.next().unwrap();
            let next_min = if op.is_right_assoc() { prec } else { prec + 1 };
            let rhs = self.pratt_parse(body, operands, operators, next_min);
            lhs = self.desugar_binary_hir(op, lhs, rhs, &span);
        }

        lhs
    }

    /// Lower call arguments.
    fn lower_call_args(&mut self, body: &AstBody, args: &[CallArg]) -> Vec<HirCallArg> {
        args.iter()
            .map(|arg| HirCallArg {
                label: arg.label.clone(),
                value: self.lower_expr(body, arg.value),
            })
            .collect()
    }

    /// Lower an if expression.
    fn lower_if(
        &mut self,
        body: &AstBody,
        conditions: &[IfCondition],
        then_body: &AstBlock,
        else_body: Option<&ElseBody>,
        span: &Span,
    ) -> HirExprId {
        let condition = self.lower_if_conditions(body, conditions, span);
        let then_block = self.lower_block(body, then_body);
        let else_block = else_body.map(|eb| match eb {
            ElseBody::Block(block) => self.lower_block(body, block),
            ElseBody::ElseIf(expr_id) => {
                // Else-if: the expr is another If expression
                let lowered = self.lower_expr(body, *expr_id);
                HirBlock {
                    stmts: Vec::new(),
                    tail_expr: Some(lowered),
                }
            },
        });

        self.alloc_expr(HirExpr::If {
            condition,
            then_body: then_block,
            else_body: else_block,
            span: span.clone(),
        })
    }

    /// Lower if-condition chains into a single boolean expression.
    /// Multiple conditions are ANDed together.
    /// Let-conditions create bindings in the current scope.
    pub(crate) fn lower_if_conditions(
        &mut self,
        body: &AstBody,
        conditions: &[IfCondition],
        span: &Span,
    ) -> HirExprId {
        if conditions.is_empty() {
            return self.alloc_expr(HirExpr::Literal {
                value: HirLiteral::Bool(true),
                span: span.clone(),
            });
        }

        if conditions.len() == 1 {
            return match &conditions[0] {
                IfCondition::Expr(expr_id) => self.lower_expr(body, *expr_id),
                IfCondition::Let { pattern, value } => {
                    // if let pattern = value → desugar to match with bool result
                    let lowered_value = self.lower_expr(body, *value);
                    let lowered_pat = self.lower_pat(body, *pattern);

                    // match value { pattern => true, _ => false }
                    let true_lit = self.alloc_expr(HirExpr::Literal {
                        value: HirLiteral::Bool(true),
                        span: span.clone(),
                    });
                    let false_lit = self.alloc_expr(HirExpr::Literal {
                        value: HirLiteral::Bool(false),
                        span: span.clone(),
                    });
                    let wildcard = self.alloc_pat(HirPat::Wildcard { span: span.clone() });

                    self.alloc_expr(HirExpr::Match {
                        scrutinee: lowered_value,
                        arms: vec![
                            HirMatchArm {
                                pattern: lowered_pat,
                                guard: None,
                                body: true_lit,
                            },
                            HirMatchArm {
                                pattern: wildcard,
                                guard: None,
                                body: false_lit,
                            },
                        ],
                        span: span.clone(),
                    })
                },
            };
        }

        // Multiple conditions: lower first, AND with rest
        let first = self.lower_if_conditions(body, &conditions[..1], span);
        let rest = self.lower_if_conditions(body, &conditions[1..], span);

        // first && rest — desugar to protocol call
        self.desugar_logical_and(first, rest, span)
    }

    /// Lower a closure expression.
    fn lower_closure(
        &mut self,
        body: &AstBody,
        params: &[ClosureParam],
        closure_body: &AstBlock,
        span: &Span,
    ) -> HirExprId {
        self.push_scope();

        // For complex patterns (tuple, struct), create a synthetic local
        // and prepend a match-based destructure to the closure body.
        let mut desugar_stmts = Vec::new();
        let mut param_counter = 0u32;

        let hir_params: Vec<HirClosureParam> = params
            .iter()
            .map(|p| {
                let pat = &body.pats[p.pattern];
                let (name, is_mut, needs_desugar) = match pat {
                    AstPat::Binding { name, is_mut, .. } => (name.clone(), *is_mut, false),
                    AstPat::Wildcard { .. } => ("_".to_string(), false, false),
                    _ => {
                        // Complex pattern — use synthetic name, desugar later
                        let name = format!("_cparam_{}", param_counter);
                        param_counter += 1;
                        (name, false, true)
                    }
                };
                let local = self.define_local(&name, is_mut, span.clone());
                let ty = p.ty.as_ref().map(|t| self.lower_type(t));

                if needs_desugar {
                    // Lower the pattern (creates locals for bindings)
                    let hir_pat = self.lower_pat(body, p.pattern);
                    let param_ref = self.alloc_expr(HirExpr::Local(local, span.clone()));
                    let unit = self.alloc_expr(HirExpr::Tuple {
                        elements: Vec::new(),
                        span: span.clone(),
                    });
                    let match_expr = self.alloc_expr(HirExpr::Match {
                        scrutinee: param_ref,
                        arms: vec![HirMatchArm {
                            pattern: hir_pat,
                            guard: None,
                            body: unit,
                        }],
                        span: span.clone(),
                    });
                    let stmt = self.alloc_stmt(HirStmt::Expr {
                        expr: match_expr,
                        span: span.clone(),
                    });
                    desugar_stmts.push(stmt);
                }

                HirClosureParam { local, ty }
            })
            .collect();

        let mut lowered_body = self.lower_block(body, closure_body);

        // Prepend destructure statements to closure body
        if !desugar_stmts.is_empty() {
            desugar_stmts.extend(lowered_body.stmts);
            lowered_body.stmts = desugar_stmts;
        }

        self.pop_scope();

        self.alloc_expr(HirExpr::Closure {
            params: hir_params,
            body: lowered_body,
            span: span.clone(),
        })
    }

    /// Lower a match expression.
    fn lower_match(
        &mut self,
        body: &AstBody,
        scrutinee: ExprId,
        arms: &[MatchArm],
        span: &Span,
    ) -> HirExprId {
        let lowered_scrutinee = self.lower_expr(body, scrutinee);

        let lowered_arms: Vec<HirMatchArm> = arms
            .iter()
            .map(|arm| {
                self.push_scope();
                let pattern = self.lower_pat(body, arm.pattern);
                let guard = arm.guard.map(|g| self.lower_expr(body, g));
                let arm_body = self.lower_expr(body, arm.body);
                self.pop_scope();

                HirMatchArm {
                    pattern,
                    guard,
                    body: arm_body,
                }
            })
            .collect();

        self.alloc_expr(HirExpr::Match {
            scrutinee: lowered_scrutinee,
            arms: lowered_arms,
            span: span.clone(),
        })
    }

    /// Lower an AST block to an HIR block.
    pub(crate) fn lower_block(&mut self, body: &AstBody, block: &AstBlock) -> HirBlock {
        self.push_scope();

        let stmts: Vec<HirStmtId> = block
            .stmts
            .iter()
            .map(|&id| self.lower_stmt(body, id))
            .collect();

        let tail_expr = block.tail_expr.map(|id| self.lower_expr(body, id));

        self.pop_scope();

        HirBlock { stmts, tail_expr }
    }

    /// Validate break/continue: must be inside a loop, and label (if any) must be in scope.
    fn validate_break_continue(&self, keyword: &str, label: &Option<String>, span: &Span) {
        if !self.in_loop() {
            self.ctx.accumulate(
                kestrel_reporting2::Diagnostic::error()
                    .with_message(format!("'{}' outside of loop", keyword))
                    .with_labels(vec![
                        kestrel_reporting2::Label::primary(span.file_id, span.range())
                            .with_message(format!("'{}' can only be used inside a loop", keyword)),
                    ])
            );
            return;
        }
        if let Some(lbl) = label {
            if !self.has_loop_label(lbl) {
                self.ctx.accumulate(
                    kestrel_reporting2::Diagnostic::error()
                        .with_message(format!("undeclared label '{}'", lbl))
                        .with_labels(vec![
                            kestrel_reporting2::Label::primary(span.file_id, span.range())
                                .with_message(format!("label '{}' not found in enclosing loops", lbl)),
                        ])
                );
            }
        }
    }
}
