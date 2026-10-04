//! Expression lowering: CST expressions → HirExpr.
//!
//! The core of HIR lowering. Resolves paths to entities/locals,
//! dispatches operator desugaring, and handles control flow.

use kestrel_ast_builder::{DeclSpan, Name};
use kestrel_hir::body::*;
use kestrel_name_res::{ResolveValuePath, ValueResolution};
use kestrel_reporting::{Diagnostic, Label};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ctx::{LowerCtx, hir_name};
use crate::syntax::{
    ArgSyntax, BlockSyntax, Cond, ExprSrc, PatSrc, PathBase, PathSeg, PathSyntax, StmtSyntax,
    arguments, binary_op, closure_body_syntax, closure_params, code_block, compound_assign_op,
    expr_children, first_expr, first_pat, first_token, if_conditions, implicit_it_reference,
    jump_label, let_conditions, loop_label, postfix_op, token, unary_op, unwrap_expr,
};

/// How a `Type.instanceMethod` path was written. Both forms are the same
/// mistake — an instance method has no receiver when named through its type —
/// and share one diagnostic; only the wording differs.
#[derive(Clone, Copy)]
enum MethodOnTypeUse {
    /// `Box.doubled(b, 7)`
    Call,
    /// `apply(Box.doubled, 7)`
    Value,
}

/// The `else` of an `if`.
enum ElseSyntax {
    Block(BlockSyntax),
    /// `else if …` (or any expression after `else`).
    ElseIf(SyntaxNode),
}

/// A match arm's body.
enum ArmBody {
    Expr(ExprSrc),
    /// `pat => { stmts; value }` — a header-less closure in arm position is a
    /// block, not a closure (no implicit `it`).
    Block(SyntaxNode),
}

/// The type path a member call's base names, when the base is a plain
/// path: `Type[Args]` in `Type[Args].staticMethod()`.
fn static_call_base(path: &PathSyntax, file_id: usize) -> Option<Vec<PathSeg>> {
    if path.members.len() != 1 {
        return None;
    }
    match &path.base {
        PathBase::Segments(segs) => Some(segs.clone()),
        PathBase::Expr(base) => {
            let inner = unwrap_expr(base);
            (inner.kind() == SyntaxKind::ExprPath)
                .then(|| PathSyntax::of(&inner, file_id))
                .and_then(|p| p.as_segments().map(<[PathSeg]>::to_vec))
        },
    }
}

impl LowerCtx<'_> {
    /// Lower an expression, or the error expression standing for a missing
    /// one.
    pub(crate) fn lower_expr_src(&mut self, src: &ExprSrc) -> HirExprId {
        match src {
            ExprSrc::Node(node) => self.lower_expr(node),
            ExprSrc::Error(span) => self.alloc_expr(HirExpr::Error { span: span.clone() }),
        }
    }

    /// Lower an expression node (an `Expression` wrapper or an `Expr*`
    /// node), recording it in the source map.
    pub(crate) fn lower_expr(&mut self, node: &SyntaxNode) -> HirExprId {
        let node = unwrap_expr(node);
        let id = self.lower_expr_node(&node);
        self.source_map.record_expr(&node, id);
        id
    }

    fn lower_expr_node(&mut self, node: &SyntaxNode) -> HirExprId {
        let span = self.span(node);
        match node.kind() {
            SyntaxKind::ExprInteger => {
                let value = HirLiteral::Integer(crate::pat::parse_int(&literal_text(node)));
                self.alloc_literal(value, span)
            },
            SyntaxKind::ExprFloat => {
                let value = HirLiteral::Float(crate::pat::parse_float(&literal_text(node)));
                self.alloc_literal(value, span)
            },
            SyntaxKind::ExprString | SyntaxKind::ExprRawString => {
                let (value, escape_errors) = crate::literal::decode_string_literal_token(
                    &literal_text(node),
                    span.file_id,
                    span.start,
                );
                let value = HirLiteral::String {
                    value,
                    escape_errors,
                };
                self.alloc_literal(value, span)
            },
            SyntaxKind::ExprChar => {
                let (value, escape_errors) =
                    crate::pat::parse_char_validated(&literal_text(node), &span, self.ctx);
                let value = HirLiteral::Char {
                    value,
                    escape_errors,
                };
                self.alloc_literal(value, span)
            },
            SyntaxKind::ExprBool => {
                let value = HirLiteral::Bool(literal_text(node) == "true");
                self.alloc_literal(value, span)
            },
            SyntaxKind::ExprNull => self.alloc_literal(HirLiteral::Null, span),
            SyntaxKind::ExprUnit => self.alloc_expr(HirExpr::Tuple {
                elements: Vec::new(),
                span,
            }),
            SyntaxKind::ExprInterpolatedString => self.desugar_interpolated_string(node, &span),
            SyntaxKind::ExprArray => {
                let elements = expr_children(node).map(|e| self.lower_expr(&e)).collect();
                self.alloc_expr(HirExpr::Array { elements, span })
            },
            SyntaxKind::ExprDictionary => {
                let entries = node
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::DictionaryEntry)
                    .filter_map(|entry| {
                        let mut exprs = expr_children(&entry);
                        Some((exprs.next()?, exprs.next()?))
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(key, value)| HirDictEntry {
                        key: self.lower_expr(&key),
                        value: self.lower_expr(&value),
                    })
                    .collect();
                self.alloc_expr(HirExpr::Dict { entries, span })
            },
            SyntaxKind::ExprTuple => {
                let elements = expr_children(node).map(|e| self.lower_expr(&e)).collect();
                self.alloc_expr(HirExpr::Tuple { elements, span })
            },
            // Grouping is transparent: `(e)` is `e` (the parser already
            // applied precedence, so nothing regroups across it).
            SyntaxKind::ExprGrouping => match first_expr(node) {
                Some(inner) => self.lower_expr(&inner),
                None => self.alloc_expr(HirExpr::Error { span }),
            },
            SyntaxKind::ExprPath => {
                let path = PathSyntax::of(node, self.file_id);
                self.lower_path_chain(&path, path.members.len(), &span)
            },
            SyntaxKind::ExprTupleIndex => {
                let base = ExprSrc::or_error(first_expr(node), &span);
                let index = token(node, SyntaxKind::Integer)
                    .and_then(|t| t.text().parse::<u32>().ok())
                    .unwrap_or(0);
                let base = self.lower_expr_src(&base);
                self.alloc_expr(HirExpr::TupleIndex { base, index, span })
            },
            SyntaxKind::ExprImplicitMemberAccess => {
                let name = node
                    .children()
                    .find(|c| c.kind() == SyntaxKind::Name)
                    .and_then(|n| token(&n, SyntaxKind::Identifier))
                    .or_else(|| token(node, SyntaxKind::Identifier))
                    .map(|t| t.text().to_string());
                let args = node
                    .children()
                    .find(|c| c.kind() == SyntaxKind::ArgumentList)
                    .map(|list| {
                        let args = arguments(&list, self.file_id);
                        self.lower_call_args(&args)
                    });
                self.alloc_expr(HirExpr::ImplicitMember {
                    name: hir_name(name),
                    args,
                    span,
                })
            },
            SyntaxKind::ExprUnary => {
                // An operator the parser does not produce means a malformed
                // tree: lower to Error rather than guessing — a fallback here
                // silently rewrites the program's meaning.
                let Some(op) = unary_op(node) else {
                    return self.alloc_expr(HirExpr::Error { span });
                };
                let operand = ExprSrc::or_error(first_expr(node), &span);
                self.desugar_unary_op(&op, &operand, &span)
            },
            SyntaxKind::ExprPostfix => {
                let Some(op) = postfix_op(node) else {
                    return self.alloc_expr(HirExpr::Error { span });
                };
                let operand = ExprSrc::or_error(first_expr(node), &span);
                // Both `!` (Unwrap) and `..` (RangeFrom) desugar to a
                // ProtocolCall via the operator → protocol table.
                self.desugar_postfix_op(&op, &operand, &span)
            },
            SyntaxKind::ExprBinary => {
                // The parser already applied precedence and associativity
                // (`kestrel-parser` `binary_binding_power`), so the nesting is
                // final and `span` covers exactly this operator's operands.
                let Some(op) = binary_op(node) else {
                    return self.alloc_expr(HirExpr::Error { span });
                };
                let (lhs, rhs) = self.operands(node, &span);
                let lhs = self.lower_expr_src(&lhs);
                let rhs = self.lower_expr_src(&rhs);
                self.desugar_binary_hir(op, lhs, rhs, &span)
            },
            SyntaxKind::ExprAssignment => {
                let (lhs, rhs) = self.operands(node, &span);
                let target = self.lower_expr_src(&lhs);
                let value = self.lower_expr_src(&rhs);
                self.alloc_expr(HirExpr::Assign {
                    target,
                    value,
                    span,
                })
            },
            SyntaxKind::ExprCompoundAssignment => {
                let Some(op) = compound_assign_op(node) else {
                    return self.alloc_expr(HirExpr::Error { span });
                };
                let (lhs, rhs) = self.operands(node, &span);
                self.desugar_compound_assign(&lhs, &op, &rhs, &span)
            },
            SyntaxKind::ExprCall => self.lower_call(node, &span),
            SyntaxKind::ExprIf => self.lower_if(node, &span),
            SyntaxKind::ExprWhile => {
                let label = loop_label(node);
                let body = BlockSyntax::of(code_block(node), self.file_id);
                if node
                    .children()
                    .any(|c| c.kind() == SyntaxKind::WhileLetCondition)
                {
                    let conditions =
                        let_conditions(node, SyntaxKind::WhileLetCondition, self.file_id);
                    self.desugar_while_let(label.as_deref(), &conditions, &body, &span)
                } else {
                    // A plain `while` reads its first condition expression.
                    let condition = ExprSrc::or_error(first_expr(node), &span);
                    self.desugar_while(label.as_deref(), &condition, &body, &span)
                }
            },
            SyntaxKind::ExprLoop => {
                let label = loop_label(node);
                let body = BlockSyntax::of(code_block(node), self.file_id);
                self.push_loop(label.as_deref());
                let lowered = self.lower_block(&body);
                self.pop_loop();
                self.alloc_expr(HirExpr::Loop {
                    label,
                    body: lowered,
                    span,
                })
            },
            SyntaxKind::ExprFor => {
                let label = loop_label(node);
                let pattern = PatSrc::or_error(
                    node.children()
                        .find(|c| c.kind() == SyntaxKind::ForPattern)
                        .and_then(|p| first_pat(&p)),
                    &span,
                );
                let iterable = ExprSrc::or_error(
                    node.children()
                        .find(|c| c.kind() == SyntaxKind::ForIterable)
                        .and_then(|i| first_expr(&i)),
                    &span,
                );
                let body = BlockSyntax::of(code_block(node), self.file_id);
                self.desugar_for_loop(label.as_deref(), &pattern, &iterable, &body, &span)
            },
            SyntaxKind::ExprBreak => {
                let label = jump_label(node);
                self.validate_break_continue("break", &label, &span);
                self.alloc_expr(HirExpr::Break { label, span })
            },
            SyntaxKind::ExprContinue => {
                let label = jump_label(node);
                self.validate_break_continue("continue", &label, &span);
                self.alloc_expr(HirExpr::Continue { label, span })
            },
            SyntaxKind::ExprReturn => {
                let lowered = first_expr(node).map(|v| self.lower_expr(&v));
                // Bare return in effectful init: wrap () in .Some(())/.Ok(())
                let wrapped = if lowered.is_none() {
                    self.wrap_init_success_value(span.clone())
                } else {
                    lowered
                };
                self.alloc_expr(HirExpr::Return {
                    value: wrapped,
                    span,
                })
            },
            SyntaxKind::ExprThrow => {
                let value = ExprSrc::or_error(first_expr(node), &span);
                self.desugar_throw(&value, &span)
            },
            SyntaxKind::ExprTry => {
                let operand = ExprSrc::or_error(first_expr(node), &span);
                self.desugar_try(&operand, &span)
            },
            SyntaxKind::ExprClosure => self.lower_closure(node, &span),
            SyntaxKind::ExprMatch => self.lower_match(node, &span),
            _ => self.alloc_expr(HirExpr::Error { span }),
        }
    }

    fn alloc_literal(&mut self, value: HirLiteral, span: Span) -> HirExprId {
        self.alloc_expr(HirExpr::Literal { value, span })
    }

    /// The two operands of a binary-shaped node, each an error at `span`
    /// when absent.
    fn operands(&self, node: &SyntaxNode, span: &Span) -> (ExprSrc, ExprSrc) {
        let mut exprs = expr_children(node);
        let lhs = ExprSrc::or_error(exprs.next(), span);
        let rhs = ExprSrc::or_error(exprs.next(), span);
        (lhs, rhs)
    }

    /// Allocate an expression that one path segment names (a `Local`, a
    /// type parameter's `Def`, a member `Field`), recording the segment's
    /// identifier in the source map.
    fn alloc_seg(&mut self, expr: HirExpr, seg: &PathSeg) -> HirExprId {
        let id = self.alloc_expr(expr);
        let range =
            rowan::TextRange::new((seg.span.start as u32).into(), (seg.span.end as u32).into());
        self.source_map.record_name_ref(range, id);
        id
    }

    /// An `ExprPath` up to (not including) member `upto`: its base — a path
    /// resolved by scope, or a computed expression — then `.member` accesses,
    /// each a `Field` the solver resolves from the base's type.
    fn lower_path_chain(&mut self, path: &PathSyntax, upto: usize, span: &Span) -> HirExprId {
        let mut current = match &path.base {
            PathBase::Segments(segments) => self.lower_path(segments, span),
            PathBase::Expr(base) => self.lower_expr(base),
        };
        for member in &path.members[..upto] {
            current = self.alloc_expr(HirExpr::Field {
                base: current,
                name: hir_name(member.name.clone()),
                span: span.clone(),
            });
            if let Some(range) = member.name_range {
                self.source_map.record_name_ref(range, current);
            }
        }
        current
    }

    /// Lower a path expression. Check locals first, then name resolution.
    /// Chain trailing path segments as `Field` accesses on an already-resolved
    /// base value. Used when name resolution stops at a VALUE partway through a
    /// path (an enum case or a field/getter used as an intermediate value) and
    /// the remaining segments are member accesses on that value — the
    /// inference solver resolves each `Field` from the base's type. The
    /// `Local`- and `TypeParameter`-leading paths build the same chain inline.
    fn lower_trailing_member_segments(&mut self, base: HirExprId, rest: &[PathSeg]) -> HirExprId {
        let mut current = base;
        for seg in rest {
            let field = HirExpr::Field {
                base: current,
                name: HirName::Name(seg.name.clone()),
                span: seg.span.clone(),
            };
            current = self.alloc_seg(field, seg);
        }
        current
    }

    fn lower_path(&mut self, segments: &[PathSeg], span: &Span) -> HirExprId {
        // Consume the callee-position marker up front so the nested
        // `lower_path` calls this function makes (and any path lowered inside
        // a callee expression) are checked as ordinary values.
        let in_callee_position = std::mem::take(&mut self.in_callee_position);

        if segments.is_empty() {
            return self.alloc_expr(HirExpr::Error { span: span.clone() });
        }

        let first = &segments[0];

        // Check if first segment is a local (covers self, params, let/var bindings).
        // Remaining segments become field accesses — type inference resolves them later.
        if first.type_args.is_none() {
            if let Some(local_id) = self.lookup_local(&first.name) {
                let local = self.alloc_seg(HirExpr::Local(local_id, first.span.clone()), first);
                return self.lower_trailing_member_segments(local, &segments[1..]);
            }

            // Specific diagnostic for `self` used where no receiver is in scope.
            // Distinguish static methods (owner is inside a type) from free functions.
            if first.name == "self" {
                return self.emit_self_out_of_scope(&first.span, span);
            }
        } else if self.lookup_local(&first.name).is_some() {
            // Local variable with type args (e.g., `x[Int]`) — variables don't accept type args
            self.ctx.accumulate(
                kestrel_reporting::Diagnostic::error()
                    .with_code("E130")
                    .with_message(format!(
                        "variable '{}' does not accept type arguments",
                        first.name
                    ))
                    .with_labels(vec![
                        kestrel_reporting::Label::primary(first.span.file_id, first.span.range())
                            .with_message("type arguments not allowed on variables"),
                    ]),
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
                    .type_args
                    .iter()
                    .flatten()
                    .map(|t| self.lower_type(t))
                    .collect();
                let def = HirExpr::Def(entity, first_type_args, segments[0].span.clone());
                let def = self.alloc_seg(def, &segments[0]);
                return self.lower_trailing_member_segments(def, &segments[1..]);
            }
        }

        // Fall back to name resolution
        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let result = self.ctx.query(ResolveValuePath {
            segments: seg_names,
            context: self.owner,
            root: self.root,
        });

        // `Box.doubled` — an instance method named through its *type*, used as
        // a value. Same rule as the call form `Box.doubled(b, 7)` rejected in
        // `lower_call`, so it shares that emitter; only the wording differs.
        //
        // This must be caught here because name resolution's direct-children
        // walk (`resolve_value.rs::walk_path_from`) returns the method entity
        // with no receiver bound — the `is_static_method` filter guards only
        // the *extension* fallback, never direct members. Left through,
        // inference types it as the method's signature minus `self` and MIR
        // emits an `apply_partial` over a two-parameter thunk behind a
        // one-parameter thick type: a silent miscompile (fragility audit
        // G3 §2b). Static methods keep working — they have no receiver.
        if !in_callee_position
            && segments.len() >= 2
            && let ValueResolution::Def(entity) = result
            && self.is_instance_method(entity)
        {
            let last = &segments[segments.len() - 1];
            return self.emit_instance_method_on_type(&last.name, span, MethodOnTypeUse::Value);
        }

        // Check for empty type argument brackets (e.g., `identity[]`)
        for seg in segments {
            if let Some(args) = &seg.type_args
                && args.is_empty()
            {
                self.ctx.accumulate(
                    kestrel_reporting::Diagnostic::error()
                        .with_code("E131")
                        .with_message("empty type argument list")
                        .with_labels(vec![
                            kestrel_reporting::Label::primary(seg.span.file_id, seg.span.range())
                                .with_message("expected at least one type argument"),
                        ]),
                );
                return self.alloc_expr(HirExpr::Error { span: span.clone() });
            }
        }

        // Collect explicit type args from all path segments (e.g., Pointer[UInt8])
        let explicit_type_args: Vec<kestrel_hir::ty::HirTy> = segments
            .iter()
            .flat_map(|s| s.type_args.iter().flatten())
            .map(|t| self.lower_type(t))
            .collect();

        match result {
            ValueResolution::Def(entity)
            | ValueResolution::TypeParameter(entity)
            | ValueResolution::SelfValue(entity) => self.alloc_expr(HirExpr::Def(
                entity,
                explicit_type_args.clone(),
                span.clone(),
            )),
            ValueResolution::Overloaded(entities) => {
                // Preserve full overload set — type inference disambiguates at call site
                self.alloc_expr(HirExpr::OverloadSet {
                    candidates: entities,
                    type_args: explicit_type_args.clone(),
                    span: span.clone(),
                })
            },
            ValueResolution::EnumCaseValue {
                entity,
                resolved_index,
            } => {
                let base = self.alloc_expr(HirExpr::Def(
                    entity,
                    explicit_type_args.clone(),
                    span.clone(),
                ));
                self.lower_trailing_member_segments(base, &segments[resolved_index + 1..])
            },
            ValueResolution::FieldValue {
                entity,
                resolved_index,
            } => {
                // The path resolved to a VALUE (a field/getter — e.g. the static
                // computed var in `Money.seven.cents`) at `resolved_index`; the
                // segments after it are member accesses on that value, not part
                // of the resolved name. Emitting them as a `Field` chain is what
                // keeps the `.cents` projection — without it the whole
                // expression collapsed to `Money.seven` (#214).
                let base = self.alloc_expr(HirExpr::Def(entity, vec![], span.clone()));
                self.lower_trailing_member_segments(base, &segments[resolved_index + 1..])
            },
            // A qualified path ending in an associated type (`U.Item`,
            // `Item.Sub`) is a projection: lower it as the *type* so it keeps
            // its base, exactly as the static-call receiver does (G26). A
            // `Def` of the alias entity would keep `Sub` and drop `Item`.
            ValueResolution::AssociatedType {
                container: Some(_), ..
            } => self.lower_type_receiver_path(segments),
            // Bare `Item` inside its own protocol: the base is the implicit
            // `Self`, so the alias entity alone is the whole answer.
            ValueResolution::AssociatedType {
                entity,
                container: None,
            } => self.alloc_expr(HirExpr::Def(entity, vec![], span.clone())),
            ValueResolution::AssociatedTypeStaticMember { .. } => {
                // `Item.zero` / `Item.Sub.zero`: a static member off an
                // associated type. The prefix is lowered as the *type* (a
                // `TypeRef` projection, base = the implicit Self or the named
                // base) rather than a `Def` of the alias entity, so a
                // two-level prefix keeps its middle segment (G26).
                let member_name = segments.last().map(|s| s.name.clone()).unwrap_or_default();
                let base = self.lower_type_receiver_path(&segments[..segments.len() - 1]);
                self.alloc_expr(HirExpr::Field {
                    base,
                    name: HirName::Name(member_name),
                    span: span.clone(),
                })
            },
            ValueResolution::Ambiguous(entities) => {
                let path_name = segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                // Primary label on the use site
                let mut labels =
                    vec![
                        Label::primary(span.file_id, span.range()).with_message(format!(
                            "{} symbols with this name in scope",
                            entities.len()
                        )),
                    ];
                // Secondary labels on each candidate's declaration
                for &entity in &entities {
                    if let Some(decl) = self.ctx.get::<DeclSpan>(entity) {
                        let name = self
                            .ctx
                            .get::<Name>(entity)
                            .map(|n| format!("declared here as '{}'", n.0))
                            .unwrap_or_else(|| "declared here".to_string());
                        labels.push(
                            Label::secondary(decl.0.file_id, decl.0.range()).with_message(name),
                        );
                    }
                }
                let diag = Diagnostic::error()
                    .with_code("E133")
                    .with_message(format!("ambiguous name '{path_name}'"))
                    .with_labels(labels)
                    .with_notes(vec![
                        "use a fully qualified path to disambiguate".to_string(),
                    ]);
                self.ctx.accumulate(diag);
                self.alloc_expr(HirExpr::Error { span: span.clone() })
            },
            ValueResolution::SelfNotInScope => {
                self.ctx.accumulate(
                    Diagnostic::error()
                        .with_code("E134")
                        .with_message(
                            "'Self' is only valid inside a type, extension, or protocol body",
                        )
                        .with_labels(vec![
                            Label::primary(span.file_id, span.range())
                                .with_message("'Self' used outside of a type body"),
                        ]),
                );
                self.alloc_expr(HirExpr::Error { span: span.clone() })
            },
            ValueResolution::NotFound(ref seg) => {
                let path_name = segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                self.ctx.accumulate(
                    Diagnostic::error()
                        .with_code("E132")
                        .with_message(format!("undefined name '{path_name}'"))
                        .with_labels(vec![
                            Label::primary(span.file_id, span.range())
                                .with_message(format!("not found (failed at '{seg}')")),
                        ]),
                );
                self.alloc_expr(HirExpr::Error { span: span.clone() })
            },
        }
    }

    /// `self.init(...)` is a delegating-init call and is only legal inside
    /// another initializer body. In any other body (func, getter, setter,
    /// deinit, ...) the call has no valid resolution — return `HirExpr::Error`
    /// so downstream passes don't cascade into argument-label / mutability
    /// errors that mislead the user.
    fn is_self_init_call(&self, segments: &[PathSeg]) -> bool {
        use kestrel_ast_builder::NodeKind;
        if segments.len() < 2 {
            return false;
        }
        if segments[0].name != "self" || segments[segments.len() - 1].name != "init" {
            return false;
        }
        !matches!(
            self.ctx.get::<NodeKind>(self.owner),
            Some(NodeKind::Initializer)
        )
    }

    fn emit_init_outside_initializer(&mut self, span: &Span) -> HirExprId {
        self.ctx.accumulate(
            Diagnostic::error()
                .with_code("E136")
                .with_message("cannot call 'init' outside of an initializer".to_string())
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("'self.init' is only valid inside another initializer"),
                ]),
        );
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    /// Emit a diagnostic for `self` used where no receiver is in scope.
    /// Distinguishes static methods (owner's parent is a type decl) from free functions.
    fn emit_self_out_of_scope(&mut self, self_span: &Span, full_span: &Span) -> HirExprId {
        use kestrel_ast_builder::NodeKind;

        let parent_kind = self
            .ctx
            .parent_of(self.owner)
            .and_then(|p| self.ctx.get::<NodeKind>(p).cloned());
        let in_type = parent_kind.as_ref().is_some_and(NodeKind::is_type_scope);
        let message = if in_type {
            "cannot use 'self' in static method"
        } else {
            "cannot use 'self' in free function"
        };
        self.ctx.accumulate(
            Diagnostic::error()
                .with_code("E135")
                .with_message(message.to_string())
                .with_labels(vec![
                    Label::primary(self_span.file_id, self_span.range())
                        .with_message("'self' is only available in instance methods"),
                ]),
        );
        self.alloc_expr(HirExpr::Error {
            span: full_span.clone(),
        })
    }

    /// Lower a call expression. Detect method calls vs direct calls.
    ///
    /// Method calls come from two callee shapes:
    /// 1. a member access on a computed base — `expr.method()` (an `ExprPath`
    ///    with members), lowered by [`Self::lower_member_call`];
    /// 2. a multi-segment path — `local.method()`, `Type.staticMethod()`,
    ///    `T.method()` — whose meaning depends on what its prefix names in
    ///    scope, decided by [`Self::lower_path_call`].
    fn lower_call(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let callee = ExprSrc::or_error(first_expr(node), span);
        let args = node
            .children()
            .find(|c| c.kind() == SyntaxKind::ArgumentList)
            .map(|list| arguments(&list, self.file_id))
            .unwrap_or_default();
        let lowered_args = self.lower_call_args(&args);

        let callee_path = callee
            .inner()
            .filter(|n| n.kind() == SyntaxKind::ExprPath)
            .map(|n| PathSyntax::of(&n, self.file_id));
        if let Some(path) = &callee_path
            && !path.members.is_empty()
        {
            return self.lower_member_call(path, lowered_args, span);
        }
        if let Some(segments) = callee_path.as_ref().and_then(PathSyntax::as_segments)
            && segments.len() >= 2
        {
            return self.lower_path_call(segments, &callee, lowered_args, span);
        }

        // Direct call
        let lowered_callee = self.lower_callee(&callee);
        self.alloc_expr(HirExpr::Call {
            callee: lowered_callee,
            args: lowered_args,
            span: span.clone(),
        })
    }

    /// `base.member(args)`: a static method named through a type
    /// (`Type[Args].staticMethod()`) or an instance method call.
    fn lower_member_call(
        &mut self,
        path: &PathSyntax,
        lowered_args: Vec<HirCallArg>,
        span: &Span,
    ) -> HirExprId {
        let last = path.members.len() - 1;
        let member = &path.members[last];

        // Check if this is a static method call on a type: Type[Args].staticMethod()
        // Resolve directly as Call(Def) instead of MethodCall so type inference
        // doesn't filter out the static method during member resolution.
        // Multiple overloads become an OverloadSet the solver disambiguates.
        let static_call = match (&member.name, static_call_base(path, self.file_id)) {
            (Some(name), Some(base)) => self.try_resolve_static_call(&base, name),
            _ => None,
        };
        if let Some((static_candidates, base_type_args)) = static_call {
            let mut all_type_args = base_type_args;
            if let Some(ref method_args) = member.type_args {
                all_type_args.extend(method_args.iter().map(|t| self.lower_type(t)));
            }
            let callee = if static_candidates.len() == 1 {
                self.alloc_expr(HirExpr::Def(
                    static_candidates[0],
                    all_type_args,
                    span.clone(),
                ))
            } else {
                self.alloc_expr(HirExpr::OverloadSet {
                    candidates: static_candidates,
                    type_args: all_type_args,
                    span: span.clone(),
                })
            };
            return self.alloc_expr(HirExpr::Call {
                callee,
                args: lowered_args,
                span: span.clone(),
            });
        }

        // Instance method call
        let lowered_base = self.lower_path_chain(path, last, span);
        let lowered_type_args = member
            .type_args
            .as_ref()
            .map(|args| args.iter().map(|t| self.lower_type(t)).collect());

        self.alloc_expr(HirExpr::MethodCall {
            receiver: lowered_base,
            method: hir_name(member.name.clone()),
            type_args: lowered_type_args,
            args: lowered_args,
            span: span.clone(),
        })
    }

    /// `a.b.c(args)` with every segment an identifier. Which prefix is a
    /// value and which segment the method is depends on scope: a local, a
    /// type with a static method, a type parameter, a value-typed static…
    fn lower_path_call(
        &mut self,
        segments: &[PathSeg],
        callee: &ExprSrc,
        lowered_args: Vec<HirCallArg>,
        span: &Span,
    ) -> HirExprId {
        // `self.init(...)` is only legal inside another initializer;
        // reject early so downstream passes don't cascade.
        if self.is_self_init_call(segments) {
            return self.emit_init_outside_initializer(span);
        }
        let first = &segments[0];
        if first.type_args.is_none() && self.lookup_local(&first.name).is_some() {
            // Lower all segments except the last as nested Field accesses
            let last = &segments[segments.len() - 1];
            let method = last.name.clone();
            let type_args = last.type_args.clone();

            // Build receiver from first N-1 segments
            let current = self.lower_path_prefix(segments);

            let lowered_type_args =
                type_args.map(|args| args.iter().map(|t| self.lower_type(t)).collect());

            return self.alloc_expr(HirExpr::MethodCall {
                receiver: current,
                method: HirName::Name(method),
                type_args: lowered_type_args,
                args: lowered_args,
                span: span.clone(),
            });
        }

        // Not a local-based path — check for static method call.
        // For Type[Args].staticMethod() or mod.Type[Args].staticMethod(),
        // resolve the static method directly so type inference doesn't need
        // to handle it as a member constraint. Multiple overloads become
        // an OverloadSet the solver disambiguates.
        {
            let last = &segments[segments.len() - 1];
            if let Some((static_candidates, base_type_args)) =
                self.try_resolve_static_call_from_segments(segments, &last.name)
            {
                let callee = if static_candidates.len() == 1 {
                    self.alloc_expr(HirExpr::Def(
                        static_candidates[0],
                        base_type_args,
                        span.clone(),
                    ))
                } else {
                    self.alloc_expr(HirExpr::OverloadSet {
                        candidates: static_candidates,
                        type_args: base_type_args,
                        span: span.clone(),
                    })
                };
                return self.alloc_expr(HirExpr::Call {
                    callee,
                    args: lowered_args,
                    span: span.clone(),
                });
            }

            // No static method matched. If the prefix names a type and
            // there's an instance method by that name, it's a misuse
            // (`Counter.getValue()` on a non-static method). Emit an
            // error and return Error so downstream phases short-circuit.
            if self.is_instance_method_on_type(segments, &last.name) {
                let method = last.name.clone();
                return self.emit_instance_method_on_type(&method, span, MethodOnTypeUse::Call);
            }
        }

        // Type-level static call → MethodCall: `T.method(args)`,
        // `T.Item.method(args)`, or — with no type parameter in the
        // path — an associated type named from inside its protocol's
        // extension (`Item.seed()`, `Item.Sub.seed()`, whose base is
        // the implicit Self). The solver resolves the method via
        // protocol bounds on the receiver.
        'type_level: {
            if segments.len() < 2 {
                break 'type_level;
            }
            let first_result = self.ctx.query(ResolveValuePath {
                segments: vec![segments[0].name.clone()],
                context: self.owner,
                root: self.root,
            });
            let first_is_param = matches!(first_result, ValueResolution::TypeParameter(_));
            let prefix_segments: Vec<String> = segments[..segments.len() - 1]
                .iter()
                .map(|s| s.name.clone())
                .collect();
            let prefix_result = self.ctx.query(ResolveValuePath {
                segments: prefix_segments,
                context: self.owner,
                root: self.root,
            });
            // Build receiver from the prefix resolution. An
            // associated-type prefix (`B.Item`, `Item.Sub`) is a
            // projection: lowering it as a `Def` of the alias entity
            // would keep `Item` and drop `B`, so a bound on `A.Item`
            // would be handed to `B.Item` (G17 S5, G26).
            let prefix = &segments[..segments.len() - 1];
            let receiver = match prefix_result {
                ValueResolution::TypeParameter(entity) | ValueResolution::Def(entity)
                    if first_is_param =>
                {
                    Some(self.lower_type_receiver_def(entity, &segments[0]))
                },
                ValueResolution::AssociatedType { .. } => {
                    Some(self.lower_type_receiver_path(prefix))
                },
                _ => None,
            };
            let Some(receiver) = receiver else {
                break 'type_level;
            };
            let last = &segments[segments.len() - 1];
            let lowered_type_args = last
                .type_args
                .as_ref()
                .map(|args| args.iter().map(|t| self.lower_type(t)).collect());
            return self.alloc_expr(HirExpr::MethodCall {
                receiver,
                method: HirName::Name(last.name.clone()),
                type_args: lowered_type_args,
                args: lowered_args,
                span: span.clone(),
            });
        }

        // Value-prefix method call: `Type.staticProp.instanceMethod(args)`.
        // If the first N-1 segments resolve to a value (Gettable field,
        // enum-case value, field-through-field chain), the last segment
        // is an instance method on that value. Emit MethodCall so type
        // inference sees the correct receiver + method shape instead of
        // treating the path as a namespace lookup that falls off a field.
        {
            use kestrel_ast_builder::{Gettable, NodeKind};
            let prefix_names: Vec<String> = segments[..segments.len() - 1]
                .iter()
                .map(|s| s.name.clone())
                .collect();
            let prefix_result = self.ctx.query(ResolveValuePath {
                segments: prefix_names,
                context: self.owner,
                root: self.root,
            });
            let is_value_prefix = match &prefix_result {
                ValueResolution::Def(entity) => {
                    matches!(
                        self.ctx.get::<NodeKind>(*entity),
                        Some(NodeKind::Field) | Some(NodeKind::EnumCase)
                    ) || self.ctx.has::<Gettable>(*entity)
                },
                ValueResolution::FieldValue { .. } | ValueResolution::EnumCaseValue { .. } => true,
                _ => false,
            };
            if is_value_prefix {
                let prefix_slice = &segments[..segments.len() - 1];
                let prefix_span = Span::new(
                    segments[0].span.file_id,
                    segments[0].span.start..prefix_slice.last().unwrap().span.end,
                );
                let receiver = self.lower_path(prefix_slice, &prefix_span);
                let last = &segments[segments.len() - 1];
                let lowered_type_args = last
                    .type_args
                    .as_ref()
                    .map(|args| args.iter().map(|t| self.lower_type(t)).collect());
                return self.alloc_expr(HirExpr::MethodCall {
                    receiver,
                    method: HirName::Name(last.name.clone()),
                    type_args: lowered_type_args,
                    args: lowered_args,
                    span: span.clone(),
                });
            }
        }

        // Type-prefix protocol-extension static method call:
        // `A.helper()` where `helper` lives in `extend SomeProto` and
        // `A: SomeProto`. The full-path collapse to `Call(Def(helper))`
        // loses the receiver `A`, so MIR can't compute the witness
        // self_type. Emit `MethodCall(Def(A), "helper")` instead, so
        // `lower_method_call` uses A's type as self_type.
        {
            use kestrel_ast_builder::NodeKind;
            if segments.len() >= 2 {
                let prefix_names: Vec<String> = segments[..segments.len() - 1]
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                let prefix_result = self.ctx.query(ResolveValuePath {
                    segments: prefix_names,
                    context: self.owner,
                    root: self.root,
                });
                let full_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
                let full_result = self.ctx.query(ResolveValuePath {
                    segments: full_names,
                    context: self.owner,
                    root: self.root,
                });
                let prefix_is_type = matches!(
                    &prefix_result,
                    ValueResolution::Def(e) if matches!(
                        self.ctx.get::<NodeKind>(*e),
                        Some(&NodeKind::Struct | &NodeKind::Enum)
                    )
                );
                let method_via_proto_ext = matches!(
                    &full_result,
                    ValueResolution::Def(method)
                        if self.ctx.get::<NodeKind>(*method) == Some(&NodeKind::Function)
                        && self.ctx.parent_of(*method).is_some_and(|p|
                            self.ctx.get::<NodeKind>(p) == Some(&NodeKind::Extension)
                            && self.ctx.query(kestrel_name_res::ExtensionTargetEntity {
                                extension: p,
                                root: self.root,
                            }).is_some_and(|target|
                                self.ctx.get::<NodeKind>(target) == Some(&NodeKind::Protocol)
                            )
                        )
                );
                if prefix_is_type && method_via_proto_ext {
                    let prefix_slice = &segments[..segments.len() - 1];
                    let prefix_span = Span::new(
                        segments[0].span.file_id,
                        segments[0].span.start..prefix_slice.last().unwrap().span.end,
                    );
                    let receiver = self.lower_path(prefix_slice, &prefix_span);
                    let last = &segments[segments.len() - 1];
                    let lowered_type_args = last
                        .type_args
                        .as_ref()
                        .map(|args| args.iter().map(|t| self.lower_type(t)).collect());
                    return self.alloc_expr(HirExpr::MethodCall {
                        receiver,
                        method: HirName::Name(last.name.clone()),
                        type_args: lowered_type_args,
                        args: lowered_args,
                        span: span.clone(),
                    });
                }
            }
        }

        // Regular direct call (lowered as-is)
        let lowered_callee = self.lower_callee(callee);
        self.alloc_expr(HirExpr::Call {
            callee: lowered_callee,
            args: lowered_args,
            span: span.clone(),
        })
    }

    /// Lower a call's callee. Identical to `lower_expr` except that a path
    /// here is in *callee* position: it names something being invoked, so it
    /// is exempt from `lower_path`'s "instance method used as a value" check.
    fn lower_callee(&mut self, callee: &ExprSrc) -> HirExprId {
        self.in_callee_position = true;
        let lowered = self.lower_expr_src(callee);
        // Cleared by the `lower_path` that consumed it; reset for callees that
        // are not paths at all (`(f)(x)`, `make()(x)`).
        self.in_callee_position = false;
        lowered
    }

    /// Is `entity` an *instance* method — a Function with a receiver and no
    /// `Static` marker? The single definition of the predicate; both misuses
    /// of `Type.instanceMethod` (called, and used as a value) test it.
    fn is_instance_method(&self, entity: kestrel_hecs::Entity) -> bool {
        use kestrel_ast_builder::{Callable, NodeKind, Static};

        self.ctx.get::<NodeKind>(entity) == Some(&NodeKind::Function)
            && !self.ctx.has::<Static>(entity)
            && self
                .ctx
                .get::<Callable>(entity)
                .is_some_and(|c| c.receiver.is_some())
    }

    /// One rule, one message, one place: an instance method named through its
    /// *type* (`Box.doubled`) has no receiver to bind. Reported for both
    /// misuses — the call `Box.doubled(b, 7)` and the value `apply(Box.doubled, 7)`
    /// — so the two can never drift apart. Returns the poison node callers
    /// substitute for the bad expression.
    fn emit_instance_method_on_type(
        &mut self,
        method: &str,
        span: &Span,
        use_kind: MethodOnTypeUse,
    ) -> HirExprId {
        let (message, label, notes) = match use_kind {
            MethodOnTypeUse::Call => (
                format!("instance method '{method}' cannot be called on a type"),
                "call this on an instance, not the type",
                Vec::new(),
            ),
            MethodOnTypeUse::Value => (
                format!("instance method '{method}' cannot be used as a value"),
                "an unbound method reference is not a value",
                vec![format!(
                    "methods cannot be used as first-class values; \
                     call it on an instance instead: 'instance.{method}()'"
                )],
            ),
        };
        self.ctx.accumulate(
            Diagnostic::error()
                .with_code("E100")
                .with_message(message)
                .with_labels(vec![
                    Label::primary(span.file_id, span.range()).with_message(label),
                ])
                .with_notes(notes),
        );
        self.alloc_expr(HirExpr::Error { span: span.clone() })
    }

    /// Whether the path `Type.member` (`segments[..-1]` resolving to a struct
    /// or enum) names an *instance* method on that type. Used to catch misuses
    /// like `Counter.getValue()` where `getValue` requires a `self`.
    fn is_instance_method_on_type(&mut self, segments: &[PathSeg], member: &str) -> bool {
        use kestrel_ast_builder::{Name, NodeKind};

        if segments.len() < 2 {
            return false;
        }
        let type_segments: Vec<String> = segments[..segments.len() - 1]
            .iter()
            .map(|s| s.name.clone())
            .collect();
        let result = self.ctx.query(ResolveValuePath {
            segments: type_segments,
            context: self.owner,
            root: self.root,
        });
        let Some(type_entity) = (match result {
            ValueResolution::Def(e) => Some(e),
            _ => None,
        }) else {
            return false;
        };
        if !matches!(
            self.ctx.get::<NodeKind>(type_entity),
            Some(&NodeKind::Struct) | Some(&NodeKind::Enum)
        ) {
            return false;
        }
        self.ctx.children_of(type_entity).iter().any(|&child| {
            self.is_instance_method(child)
                && self.ctx.get::<Name>(child).is_some_and(|n| n.0 == member)
        })
    }

    /// Check if a multi-segment path ending in `member` is a static method call.
    /// Resolves all segments except the last as a type, then collects ALL static
    /// methods named `member` on that type (one entity per overload).
    fn try_resolve_static_call_from_segments(
        &mut self,
        segments: &[PathSeg],
        member: &str,
    ) -> Option<(Vec<kestrel_hecs::Entity>, Vec<kestrel_hir::ty::HirTy>)> {
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

        // Collect every static-function child matching `member` — multiple
        // entities mean overloads, which the solver disambiguates by labels/arity.
        let mut matches: Vec<kestrel_hecs::Entity> = Vec::new();
        for &child in self.ctx.children_of(type_entity) {
            if self.ctx.get::<NodeKind>(child) != Some(&NodeKind::Function) {
                continue;
            }
            if self.ctx.get::<Static>(child).is_none() {
                continue;
            }
            let Some(child_name) = self.ctx.get::<Name>(child) else {
                continue;
            };
            if child_name.0 == member {
                matches.push(child);
            }
        }

        if matches.is_empty() {
            return None;
        }

        // Collect struct type_args from base segments + method type_args from last segment
        let mut type_args: Vec<kestrel_hir::ty::HirTy> = segments[..segments.len() - 1]
            .iter()
            .flat_map(|s| s.type_args.iter().flatten())
            .map(|t| self.lower_type(t))
            .collect();
        let last = &segments[segments.len() - 1];
        if let Some(ref method_args) = last.type_args {
            type_args.extend(method_args.iter().map(|t| self.lower_type(t)));
        }
        Some((matches, type_args))
    }

    /// Check if `segments.member` (a path naming a type) is a static method call.
    /// Returns `Some((candidates, type_args))` where `candidates` collects every
    /// static overload named `member` — the solver disambiguates by labels/arity.
    fn try_resolve_static_call(
        &mut self,
        segments: &[PathSeg],
        member: &str,
    ) -> Option<(Vec<kestrel_hecs::Entity>, Vec<kestrel_hir::ty::HirTy>)> {
        use kestrel_ast_builder::{Name, NodeKind, Static};

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

        // Collect every static-function child matching `member`
        let mut matches: Vec<kestrel_hecs::Entity> = Vec::new();
        for &child in self.ctx.children_of(base_entity) {
            if self.ctx.get::<NodeKind>(child) != Some(&NodeKind::Function) {
                continue;
            }
            if self.ctx.get::<Static>(child).is_none() {
                continue;
            }
            let Some(child_name) = self.ctx.get::<Name>(child) else {
                continue;
            };
            if child_name.0 == member {
                matches.push(child);
            }
        }

        if matches.is_empty() {
            return None;
        }

        let type_args: Vec<kestrel_hir::ty::HirTy> = segments
            .iter()
            .flat_map(|s| s.type_args.iter().flatten())
            .map(|t| self.lower_type(t))
            .collect();
        Some((matches, type_args))
    }

    /// Receiver of a type-level static call whose prefix names a single
    /// entity (`T.zero()`): a `Def`, carrying the first segment's type args.
    fn lower_type_receiver_def(
        &mut self,
        entity: kestrel_hecs::Entity,
        first: &PathSeg,
    ) -> HirExprId {
        let type_args: Vec<kestrel_hir::ty::HirTy> = first
            .type_args
            .iter()
            .flatten()
            .map(|t| self.lower_type(t))
            .collect();
        self.alloc_seg(HirExpr::Def(entity, type_args, first.span.clone()), first)
    }

    /// An associated-type projection in expression position, lowered as the
    /// *type* through the same path as a type annotation, so it arrives as
    /// `HirTy::AssocProjection { base: Param(B), .. }` and keeps its base.
    /// The one lowering for both callers: the receiver of a type-level static
    /// call (`B.Item.zero()`, G17 S5) and a value path ending in an
    /// associated type (`Item.Sub`, G26).
    fn lower_type_receiver_path(&mut self, prefix: &[PathSeg]) -> HirExprId {
        let (first, last) = (&prefix[0], &prefix[prefix.len() - 1]);
        let span = Span {
            file_id: first.span.file_id,
            start: first.span.start,
            end: last.span.end,
        };
        let segments = prefix
            .iter()
            .map(|s| kestrel_ast::PathSegment {
                name: s.name.clone(),
                type_args: s.type_args.clone().unwrap_or_default(),
                span: s.span.clone(),
            })
            .collect();
        let ty = self.lower_type(&kestrel_ast::AstType::Named {
            segments,
            span: span.clone(),
        });
        self.alloc_expr(HirExpr::TypeRef { ty, span })
    }

    /// Lower Path segments except the last one as receiver.
    /// For `[a, b, c]` returns `Field { base: Field { base: Local(a), name: "b" }, name: ... }`
    /// but stops before the last segment.
    fn lower_path_prefix(&mut self, segments: &[PathSeg]) -> HirExprId {
        let first = &segments[0];
        let local_id = self.lookup_local(&first.name).unwrap();
        let local = self.alloc_seg(HirExpr::Local(local_id, first.span.clone()), first);
        // Build Field chain for all segments except first and last
        self.lower_trailing_member_segments(local, &segments[1..segments.len() - 1])
    }

    /// Lower call arguments.
    fn lower_call_args(&mut self, args: &[ArgSyntax]) -> Vec<HirCallArg> {
        args.iter()
            .map(|arg| HirCallArg {
                label: arg.label.clone(),
                value: self.lower_expr_src(&arg.value),
            })
            .collect()
    }

    /// Lower an if expression.
    fn lower_if(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let conditions = if_conditions(node, self.file_id);
        let then_body = BlockSyntax::of(code_block(node), self.file_id);
        let else_body = node
            .children()
            .find(|c| c.kind() == SyntaxKind::ElseClause)
            .map(|clause| {
                // ElseClause contains either a CodeBlock or a nested `if`
                if let Some(block) = code_block(&clause) {
                    ElseSyntax::Block(BlockSyntax::of(Some(block), self.file_id))
                } else if let Some(expr) = clause
                    .children()
                    .find(|c| matches!(c.kind(), SyntaxKind::ExprIf | SyntaxKind::Expression))
                {
                    ElseSyntax::ElseIf(expr)
                } else {
                    ElseSyntax::Block(BlockSyntax::empty())
                }
            });

        // If any condition is an `if let`, desugar through the shared condition
        // chain so pattern bindings stay in scope in the then-body (success =
        // then, fail = else). A single binding nests to one Match; chained
        // conditions nest deeper. The bool-returning match + if-branch strategy
        // breaks OSSA dominance because bindings created inside the match don't
        // dominate the if's then-block (issue #126's if-let twin), so binding
        // conditions must never go through `lower_if_conditions`.
        if conditions.iter().any(Cond::is_let) {
            let mut on_success = |this: &mut Self| {
                let then_block = this.lower_block(&then_body);
                this.hir_block_to_expr(then_block, span)
            };
            let mut on_fail = |this: &mut Self| match &else_body {
                None => this.alloc_expr(HirExpr::Tuple {
                    elements: Vec::new(),
                    span: span.clone(),
                }),
                Some(ElseSyntax::Block(block)) => {
                    let eb = this.lower_block(block);
                    this.hir_block_to_expr(eb, span)
                },
                Some(ElseSyntax::ElseIf(expr)) => this.lower_expr(expr),
            };
            return self.lower_condition_chain(
                &conditions,
                MatchSource::IfLet,
                span,
                &mut on_success,
                &mut on_fail,
            );
        }

        // Regular if expression (no pattern binding)
        self.push_scope();
        let condition = self.lower_if_conditions(&conditions, MatchSource::IfLet, span);
        let then_block = self.lower_block(&then_body);
        self.pop_scope();
        let else_block = else_body.map(|eb| match eb {
            ElseSyntax::Block(block) => self.lower_block(&block),
            ElseSyntax::ElseIf(expr) => {
                // Else-if: the expr is another If expression
                let lowered = self.lower_expr(&expr);
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
    /// `source` tags any desugared let-condition matches so the right
    /// diagnostic fires (IfLet → E302, WhileLet → E308, Guard → E309).
    pub(crate) fn lower_if_conditions(
        &mut self,
        conditions: &[Cond],
        source: MatchSource,
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
                Cond::Expr(expr) => self.lower_expr_src(expr),
                Cond::Let { pat, value } => {
                    // if let pattern = value → desugar to match with bool result
                    let lowered_value = self.lower_expr_src(value);
                    let lowered_pat = self.lower_pat(pat);

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
                        source,
                        span: span.clone(),
                    })
                },
            };
        }

        // Multiple conditions: lower first, AND with rest
        let first = self.lower_if_conditions(&conditions[..1], source, span);
        let rest = self.lower_if_conditions(&conditions[1..], source, span);

        // first && rest — through the same table-driven path as a written `&&`,
        // so a missing `LogicalAndOperator` conformance reports instead of
        // silently discarding `rest`.
        self.desugar_binary_hir(kestrel_ast::BinaryOp::And, first, rest, span)
    }

    /// Lower a closure expression.
    /// Warn when a closure's implicit `it` (first referenced at `first_ref`)
    /// hides another `it` in scope. The implicit parameter always wins — it
    /// belongs to the innermost headerless closure and never captures an
    /// outer binding — so either way the user may have meant the other one.
    /// Call before the implicit parameter is defined.
    fn warn_implicit_it_shadowing(&self, first_ref: &Span) {
        let Some(outer) = self.lookup_local("it") else {
            return;
        };
        let (code, message, outer_label) = match self.implicit_it_locals.get(&outer) {
            Some(_) => (
                "E142",
                "implicit parameter 'it' shadows the 'it' of an enclosing closure",
                "the enclosing closure's 'it'",
            ),
            None => (
                "E143",
                "implicit parameter 'it' shadows the outer binding 'it'",
                "outer 'it' declared here",
            ),
        };
        // Point at the outer `it` itself: an enclosing closure's first `it`,
        // else the outer binding's name (its `Local::span` covers the whole
        // declaration — a `let` statement, or nothing for a parameter).
        let outer_span = self
            .implicit_it_locals
            .get(&outer)
            .cloned()
            .or_else(|| {
                let name = self.source_map.local_source(outer)?.name;
                Some(Span::new(self.file_id, name.into()))
            })
            .unwrap_or_else(|| self.locals[outer].span.clone());
        self.ctx.accumulate(
            kestrel_reporting::Diagnostic::warning()
                .with_code(code)
                .with_message(message)
                .with_labels(vec![
                    kestrel_reporting::Label::primary(first_ref.file_id, first_ref.range())
                        .with_message("this is the innermost closure's own parameter"),
                    kestrel_reporting::Label::secondary(outer_span.file_id, outer_span.range())
                        .with_message(outer_label),
                ])
                .with_notes(vec![
                    "name the closure's parameter to make it explicit, e.g. `{ (x) in ... }`"
                        .to_string(),
                ]),
        );
    }

    /// Lower a closure expression.
    fn lower_closure(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        // Implicit `it` parameter: a closure without a parameter header whose
        // body refers to the name `it` gets `it` as its parameter — `{ it + 1 }`
        // is `{ (it) in it + 1 }`. It belongs to the innermost header-less
        // closure and always wins over an outer binding named `it` (both
        // shadowings warn: E142/E143).
        let header = closure_params(node, self.file_id);
        let implicit_it = match header {
            None => implicit_it_reference(node).map(|first_ref| self.span(&first_ref)),
            Some(_) => None,
        };
        let params = header.unwrap_or_default();
        let closure_body = closure_body_syntax(node, self.file_id);

        self.push_scope();

        // For complex patterns (tuple, struct), create a synthetic local
        // and prepend a match-based destructure to the closure body.
        let mut desugar_stmts = Vec::new();
        let mut param_counter = 0u32;
        let mut hir_params: Vec<HirClosureParam> = Vec::with_capacity(params.len() + 1);

        if let Some(first_ref) = implicit_it {
            self.warn_implicit_it_shadowing(&first_ref);
            let local = self.define_local("it", false, span.clone());
            self.implicit_it_locals.insert(local, first_ref);
            hir_params.push(HirClosureParam {
                local,
                ty: None,
                pattern: None,
                is_mut: false,
            });
        }

        for p in &params {
            let pat = p.pat.clone().resolve(self.file_id);
            // A plain binding names the parameter; `_` is anonymous; anything
            // else gets a synthetic name and destructures in the body.
            let binding = match &pat {
                PatSrc::Node(n)
                    if matches!(
                        n.kind(),
                        SyntaxKind::BindingPattern | SyntaxKind::RefBindingPattern
                    ) =>
                {
                    token(n, SyntaxKind::Identifier).map(|name| {
                        let is_mut = n.kind() == SyntaxKind::BindingPattern
                            && crate::syntax::has_token(n, SyntaxKind::Var);
                        (n.clone(), name, is_mut)
                    })
                },
                _ => None,
            };
            let is_wildcard = pat.kind() == Some(SyntaxKind::WildcardPattern);
            // A `mutating` closure param (`p.is_mut`) makes the binding
            // mutable so `x`/`x.field` assignment is allowed (no E604/E201)
            // and the body lowers the param as a by-reference place.
            let (local, param_is_mut, needs_desugar) = match &binding {
                Some((binding_node, name, is_mut)) => {
                    let param_is_mut = *is_mut || p.is_mut;
                    let local =
                        self.define_named_local(binding_node, name, param_is_mut, span.clone());
                    (local, param_is_mut, false)
                },
                None if is_wildcard => (
                    self.define_local("_", p.is_mut, span.clone()),
                    p.is_mut,
                    false,
                ),
                None => {
                    let name = format!("_cparam_{}", param_counter);
                    param_counter += 1;
                    (
                        self.define_local(&name, p.is_mut, span.clone()),
                        p.is_mut,
                        true,
                    )
                },
            };
            let ty =
                p.ty.as_ref()
                    .map(|t| self.lower_type_in(t, crate::ty::RefPosition::Param));

            let pattern = if needs_desugar {
                // Lower the pattern (creates locals for bindings)
                let hir_pat = self.lower_pat(&pat);
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
                    source: MatchSource::ParamDestructure,
                    span: span.clone(),
                });
                let stmt = self.alloc_stmt(HirStmt::Expr {
                    expr: match_expr,
                    span: span.clone(),
                });
                desugar_stmts.push(stmt);
                Some(hir_pat)
            } else {
                None
            };

            hir_params.push(HirClosureParam {
                local,
                ty,
                pattern,
                is_mut: param_is_mut,
            });
        }

        // A closure body is a separate function body: `break`/`continue` in it
        // cannot target a loop in the *enclosing* body. Without this the
        // enclosing loop stack stays visible, `validate_break_continue` passes,
        // and MIR's `lower_break` then finds no loop and emits a unit literal —
        // the `break` becomes a silent no-op.
        let saved_loops = std::mem::take(&mut self.loop_labels);
        let mut lowered_body = self.lower_block(&closure_body);
        self.loop_labels = saved_loops;

        // Prepend destructure statements to closure body
        if !desugar_stmts.is_empty() {
            desugar_stmts.extend(lowered_body.stmts);
            lowered_body.stmts = desugar_stmts;
        }

        self.pop_scope();

        // Captures are computed post-inference by the `ClosureCaptures` query
        // (kestrel-type-infer), the single source of truth — not recorded here.
        self.alloc_expr(HirExpr::Closure {
            params: hir_params,
            body: lowered_body,
            span: span.clone(),
        })
    }

    /// Lower a match expression.
    fn lower_match(&mut self, node: &SyntaxNode, span: &Span) -> HirExprId {
        let scrutinee = ExprSrc::or_error(first_expr(node), span);
        let lowered_scrutinee = self.lower_expr_src(&scrutinee);

        let arms: Vec<SyntaxNode> = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::MatchArm)
            .collect();
        let lowered_arms: Vec<HirMatchArm> = arms
            .iter()
            .map(|arm| {
                let arm_span = self.span(arm);
                let pat = PatSrc::or_error(first_pat(arm), &arm_span);
                let guard = arm
                    .children()
                    .find(|c| c.kind() == SyntaxKind::MatchArmGuard)
                    .map(|g| {
                        let guard_span = self.span(&g);
                        ExprSrc::or_error(first_expr(&g), &guard_span)
                    });
                let body = match expr_children(arm).last() {
                    Some(expr) => {
                        let inner = unwrap_expr(&expr);
                        let headerless_closure = inner.kind() == SyntaxKind::ExprClosure
                            && !inner
                                .children()
                                .any(|c| c.kind() == SyntaxKind::ClosureParams);
                        if headerless_closure {
                            ArmBody::Block(inner)
                        } else {
                            ArmBody::Expr(ExprSrc::Node(expr))
                        }
                    },
                    None => ArmBody::Expr(ExprSrc::Error(arm_span)),
                };

                self.push_scope();
                // `&` binder patterns are legal exactly here — user-match
                // arm patterns (the place-mode lowering's domain).
                let prev = std::mem::replace(&mut self.ref_patterns_allowed, true);
                let pattern = self.lower_pat(&pat);
                self.ref_patterns_allowed = prev;
                let guard = guard.map(|g| self.lower_expr_src(&g));
                let arm_body = match &body {
                    ArmBody::Expr(expr) => self.lower_expr_src(expr),
                    ArmBody::Block(closure) => {
                        let block = closure_body_syntax(closure, self.file_id);
                        let lowered = self.lower_block(&block);
                        let block_span = self.span(closure);
                        let id = self.alloc_expr(HirExpr::Block {
                            body: lowered,
                            span: block_span,
                        });
                        self.source_map.record_expr(closure, id);
                        id
                    },
                };
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
            source: MatchSource::UserMatch,
            span: span.clone(),
        })
    }

    /// Lower a block to an HIR block, in its own scope.
    pub(crate) fn lower_block(&mut self, block: &BlockSyntax) -> HirBlock {
        self.push_scope();
        let result = self.lower_block_stmts(&block.stmts, block.tail.as_ref());
        self.pop_scope();
        result
    }

    /// Lower a sequence of statements + optional tail expression.
    /// Detects `guard let` and CPS-transforms: the remaining block becomes the
    /// match arm's body so pattern bindings stay in scope under OSSA.
    /// Chained `guard let`s nest naturally via recursion.
    pub(crate) fn lower_block_stmts(
        &mut self,
        stmts: &[StmtSyntax],
        tail_expr: Option<&ExprSrc>,
    ) -> HirBlock {
        for (i, stmt) in stmts.iter().enumerate() {
            // Any guard binding at least one `let` must CPS-transform so the
            // bindings flow into the continuation. A guard with several
            // comma-chained conditions (e.g. `guard let a = .., let b = ..`)
            // nests one match/if per condition — the boolean-AND fallback in
            // `lower_if_conditions` would evaluate each pattern only for its
            // truth value and drop the bindings, breaking OSSA.
            let StmtSyntax::Node(node) = stmt else {
                continue;
            };
            if node.kind() != SyntaxKind::GuardStatement {
                continue;
            }
            let (conditions, else_body) = self.guard_parts(node);
            if !conditions.iter().any(Cond::is_let) {
                continue;
            }
            let span = self.span(node);

            // Lower preceding statements normally
            let prev: Vec<HirStmtId> = stmts[..i].iter().map(|s| self.lower_stmt(s)).collect();

            // CPS: wrap remaining stmts + tail as the innermost continuation,
            // nesting one condition per level.
            let match_expr =
                self.lower_guard_cps(&conditions, &else_body, &stmts[i + 1..], tail_expr, &span);

            return HirBlock {
                stmts: prev,
                tail_expr: Some(match_expr),
            };
        }

        // No guard-let found — lower everything normally
        let lowered: Vec<HirStmtId> = stmts.iter().map(|s| self.lower_stmt(s)).collect();
        let tail = tail_expr.map(|e| self.lower_expr_src(e));
        HirBlock {
            stmts: lowered,
            tail_expr: tail,
        }
    }

    /// Lower a chain of if-conditions into nested matches/ifs that thread every
    /// `let` binding into the success continuation under OSSA. This is the
    /// single source of truth shared by `guard`, `if let`, and `while let`.
    ///
    /// Each `let p = v` becomes a two-arm match (`p` → deeper chain, `_` →
    /// fail); each boolean `e` becomes `if e { deeper } else { fail }`. Chained
    /// `let`s nest, so a later condition's value expression and the success
    /// continuation both see the earlier bindings:
    ///
    /// ```text
    /// let p = v, e   ⇒   match v { p => if e { <success> } else { <fail> }
    ///                                _ => <fail> }
    /// ```
    ///
    /// `on_success` is materialized at the innermost level, inside all the
    /// pattern scopes, so it sees every binding. The fail continuation is
    /// lowered **once**, here, *outside* every scope (bindings are undefined
    /// there) and the resulting id is referenced from each level's fail arm.
    /// `source` tags the generated `let` matches (GuardLet/IfLet/WhileLet) for
    /// the divergence + exhaustiveness analyzers.
    ///
    /// Lowering fail once is load-bearing, not just tidy. It used to be rebuilt
    /// per level, which duplicated every diagnostic the else body produces (one
    /// copy per condition) and, because an `else if` is itself lowered through
    /// this function, lowered the tail of an `else if` chain 2^depth times —
    /// multiplying the findings of every HIR-walking analyzer with it
    /// (fragility audit F31). Duplicating was only ever *semantically* safe:
    /// guard/while fail branches diverge, and if-let's else runs on exactly one
    /// path, so the arms referencing one shared id are mutually exclusive.
    ///
    /// Do NOT route binding conditions through the boolean-AND
    /// `lower_if_conditions` path: it lowers each pattern to a throwaway
    /// `match { p => true, _ => false }` and drops the binding, leaving the
    /// continuation referencing locals that were never threaded in (OSSA
    /// "used but never defined" — issue #126 and its if-let/while-let twins).
    pub(crate) fn lower_condition_chain(
        &mut self,
        conditions: &[Cond],
        source: MatchSource,
        span: &Span,
        on_success: &mut dyn FnMut(&mut Self) -> HirExprId,
        on_fail: &mut dyn FnMut(&mut Self) -> HirExprId,
    ) -> HirExprId {
        // Nothing to branch on: no fail arm is reachable, so don't lower one.
        if conditions.is_empty() {
            return on_success(self);
        }
        let fail = on_fail(self);
        self.lower_condition_chain_with_fail(conditions, source, span, on_success, fail)
    }

    /// The recursive half of [`Self::lower_condition_chain`], with the fail
    /// continuation already lowered to a single id shared by every level.
    fn lower_condition_chain_with_fail(
        &mut self,
        conditions: &[Cond],
        source: MatchSource,
        span: &Span,
        on_success: &mut dyn FnMut(&mut Self) -> HirExprId,
        fail: HirExprId,
    ) -> HirExprId {
        // No conditions left: materialize the success continuation. It runs
        // inside the scopes opened by the enclosing `let` conditions, so their
        // bindings are visible.
        let Some((first, rest)) = conditions.split_first() else {
            return on_success(self);
        };

        match first {
            Cond::Let { pat, value } => {
                let scrutinee = self.lower_expr_src(value);

                // Bindings are visible in the success continuation (deeper
                // conditions + the body), not in the fail branch. Scope the
                // pattern + continuation so the bindings don't leak into fail.
                self.push_scope();
                let pat = self.lower_pat(pat);
                let success =
                    self.lower_condition_chain_with_fail(rest, source, span, on_success, fail);
                self.pop_scope();

                let wildcard = self.alloc_pat(HirPat::Wildcard { span: span.clone() });

                self.alloc_expr(HirExpr::Match {
                    scrutinee,
                    arms: vec![
                        HirMatchArm {
                            pattern: pat,
                            guard: None,
                            body: success,
                        },
                        HirMatchArm {
                            pattern: wildcard,
                            guard: None,
                            body: fail,
                        },
                    ],
                    source,
                    span: span.clone(),
                })
            },
            Cond::Expr(expr) => {
                let condition = self.lower_expr_src(expr);
                let success =
                    self.lower_condition_chain_with_fail(rest, source, span, on_success, fail);

                self.alloc_expr(HirExpr::If {
                    condition,
                    then_body: HirBlock {
                        stmts: Vec::new(),
                        tail_expr: Some(success),
                    },
                    else_body: Some(HirBlock {
                        stmts: Vec::new(),
                        tail_expr: Some(fail),
                    }),
                    span: span.clone(),
                })
            },
        }
    }

    /// CPS-desugar a `guard <c0>, <c1>, … else { <else_body> }` that binds at
    /// least one `let`: success = the remaining statements + tail (so bindings
    /// stay in scope under OSSA), fail = the diverging else body. Divergence is
    /// checked on every `GuardLet` match's else arm by the analyzer.
    fn lower_guard_cps(
        &mut self,
        conditions: &[Cond],
        else_body: &BlockSyntax,
        remaining_stmts: &[StmtSyntax],
        tail_expr: Option<&ExprSrc>,
        span: &Span,
    ) -> HirExprId {
        let mut on_success = |this: &mut Self| {
            let cont = this.lower_block_stmts(remaining_stmts, tail_expr);
            this.hir_block_to_expr(cont, span)
        };
        let mut on_fail = |this: &mut Self| {
            let else_block = this.lower_block(else_body);
            this.hir_block_to_expr(else_block, span)
        };
        self.lower_condition_chain(
            conditions,
            MatchSource::GuardLet,
            span,
            &mut on_success,
            &mut on_fail,
        )
    }

    /// Convert an HirBlock into a single HirExprId.
    fn hir_block_to_expr(&mut self, block: HirBlock, span: &Span) -> HirExprId {
        if block.stmts.is_empty() {
            block.tail_expr.unwrap_or_else(|| {
                self.alloc_expr(HirExpr::Tuple {
                    elements: Vec::new(),
                    span: span.clone(),
                })
            })
        } else {
            self.alloc_expr(HirExpr::Block {
                body: block,
                span: span.clone(),
            })
        }
    }

    /// Validate break/continue: must be inside a loop, and label (if any) must be in scope.
    fn validate_break_continue(&self, keyword: &str, label: &Option<String>, span: &Span) {
        if !self.in_loop() {
            self.ctx.accumulate(
                kestrel_reporting::Diagnostic::error()
                    .with_code("E010")
                    .with_message(format!("'{}' outside of loop", keyword))
                    .with_labels(vec![
                        kestrel_reporting::Label::primary(span.file_id, span.range())
                            .with_message(format!("'{}' can only be used inside a loop", keyword)),
                    ]),
            );
            return;
        }
        if let Some(lbl) = label
            && !self.has_loop_label(lbl)
        {
            self.ctx.accumulate(
                kestrel_reporting::Diagnostic::error()
                    .with_code("E011")
                    .with_message(format!("undeclared label '{}'", lbl))
                    .with_labels(vec![
                        kestrel_reporting::Label::primary(span.file_id, span.range())
                            .with_message(format!("label '{}' not found in enclosing loops", lbl)),
                    ]),
            );
        }
    }
}

/// The text of a literal's token (empty when the parser recovered without
/// one).
fn literal_text(node: &SyntaxNode) -> String {
    first_token(node)
        .map(|t| t.text().to_string())
        .unwrap_or_default()
}
