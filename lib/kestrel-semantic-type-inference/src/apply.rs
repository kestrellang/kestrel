//! Apply type inference solutions to code blocks.
//!
//! This module transforms a `CodeBlock` by replacing all `TyKind::Infer`
//! placeholders with their resolved types from the solution.

use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::expr::{
    CallArgument, ElseBranch, ExprKind, Expression, InterpolationPart, PrimitiveMethod,
};
use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::local::LocalContainer;
use kestrel_semantic_tree::ty::{ParamInfo, Substitutions, Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::solution::{PromotionInfo, Solution};

/// Apply a solution to a code block, resolving all inference placeholders.
///
/// Returns a new `CodeBlock` where all `TyKind::Infer` types have been
/// replaced with their resolved concrete types.
pub fn apply_solution(block: &CodeBlock, solution: &Solution) -> CodeBlock {
    let statements = block
        .statements
        .iter()
        .map(|stmt| apply_to_statement(stmt, solution))
        .collect();

    let yield_expr = block
        .yield_expr
        .as_ref()
        .map(|expr| Box::new(apply_to_expression(expr, solution)));

    CodeBlock {
        statements,
        yield_expr,
    }
}

/// Apply a solution to a vector of patterns, resolving all inference placeholders.
///
/// Used for parameter patterns in function declarations.
pub fn apply_solution_to_patterns(patterns: &[Pattern], solution: &Solution) -> Vec<Pattern> {
    patterns
        .iter()
        .map(|p| apply_to_pattern(p, solution))
        .collect()
}

/// Apply a solution to default value expressions, resolving all inference placeholders.
///
/// Returns a new Vec where each Some contains a resolved expression.
pub fn apply_solution_to_defaults(
    defaults: &[Option<Expression>],
    solution: &Solution,
) -> Vec<Option<Expression>> {
    defaults
        .iter()
        .map(|opt| opt.as_ref().map(|expr| apply_to_expression(expr, solution)))
        .collect()
}

/// Apply solution to a statement.
fn apply_to_statement(stmt: &Statement, solution: &Solution) -> Statement {
    let kind = match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            let resolved_pattern = apply_to_pattern(pattern, solution);
            let resolved_value = value.as_ref().map(|v| apply_to_expression(v, solution));
            StatementKind::Binding {
                pattern: resolved_pattern,
                value: resolved_value,
            }
        },
        StatementKind::Expr(expr) => StatementKind::Expr(apply_to_expression(expr, solution)),
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            let resolved_conditions = conditions
                .iter()
                .map(|cond| apply_to_if_condition(cond, solution))
                .collect();
            let resolved_else_block = apply_solution(else_block, solution);
            StatementKind::GuardLet {
                conditions: resolved_conditions,
                else_block: resolved_else_block,
            }
        },
        StatementKind::Deinit { local_id, name } => {
            // Deinit statement doesn't contain types that need resolution
            StatementKind::Deinit {
                local_id: *local_id,
                name: name.clone(),
            }
        },
    };

    Statement::new(kind, stmt.span.clone())
}

/// Apply solution to an expression.
fn apply_to_expression(expr: &Expression, solution: &Solution) -> Expression {
    let resolved_ty = resolve_type(&expr.ty, solution);

    let kind = match &expr.kind {
        // Simple cases - just clone the kind
        ExprKind::Literal(lit) => ExprKind::Literal(lit.clone()),
        ExprKind::InterpolatedString { parts } => {
            // Apply solution to interpolation expressions
            let resolved_parts = parts
                .iter()
                .map(|part| match part {
                    InterpolationPart::Literal { text, span } => InterpolationPart::Literal {
                        text: text.clone(),
                        span: span.clone(),
                    },
                    InterpolationPart::Interpolation {
                        expr,
                        format_spec,
                        span,
                    } => InterpolationPart::Interpolation {
                        expr: Box::new(apply_to_expression(expr, solution)),
                        format_spec: format_spec.clone(),
                        span: span.clone(),
                    },
                })
                .collect();
            ExprKind::InterpolatedString {
                parts: resolved_parts,
            }
        },
        ExprKind::LocalRef(id) => ExprKind::LocalRef(*id),
        ExprKind::SymbolRef(id) => ExprKind::SymbolRef(*id),
        ExprKind::OverloadedRef(ids) => ExprKind::OverloadedRef(ids.clone()),
        ExprKind::TypeRef(id) => ExprKind::TypeRef(*id),
        ExprKind::TypeParameterRef(id) => ExprKind::TypeParameterRef(*id),
        ExprKind::AssociatedTypeRef => ExprKind::AssociatedTypeRef,
        ExprKind::EnumCase { case_id } => ExprKind::EnumCase { case_id: *case_id },
        ExprKind::ImplicitMemberAccess {
            member_name,
            arguments,
        } => {
            let resolved_arguments: Option<Vec<CallArgument>> = arguments.as_ref().map(|args| {
                args.iter()
                    .map(|arg| apply_to_argument(arg, solution))
                    .collect()
            });

            // Check if we have a resolved case for this expression
            if let Some(value_resolution) = solution.get_value(expr.id) {
                // Transform into EnumCase (with arguments if present)
                if let Some(args) = resolved_arguments {
                    // Case with associated values - create a Call to the enum case
                    let case_expr = Expression::enum_case(
                        value_resolution.symbol_id,
                        resolved_ty.clone(),
                        expr.span.clone(),
                    );
                    ExprKind::Call {
                        callee: Box::new(case_expr),
                        arguments: args,
                        substitutions: value_resolution.substitutions.clone(),
                    }
                } else {
                    // Simple case - just create EnumCase
                    ExprKind::EnumCase {
                        case_id: value_resolution.symbol_id,
                    }
                }
            } else {
                // No resolution found - keep as implicit (will error during lowering)
                ExprKind::ImplicitMemberAccess {
                    member_name: member_name.clone(),
                    arguments: resolved_arguments,
                }
            }
        },
        ExprKind::Error => ExprKind::Error,
        ExprKind::Break { loop_id, label } => ExprKind::Break {
            loop_id: *loop_id,
            label: label.clone(),
        },
        ExprKind::Continue { loop_id, label } => ExprKind::Continue {
            loop_id: *loop_id,
            label: label.clone(),
        },

        // Compound expressions - recurse
        ExprKind::Array(elements) => ExprKind::Array(
            elements
                .iter()
                .map(|e| apply_to_expression(e, solution))
                .collect(),
        ),

        ExprKind::Dictionary(pairs) => ExprKind::Dictionary(
            pairs
                .iter()
                .map(|(k, v)| {
                    (
                        apply_to_expression(k, solution),
                        apply_to_expression(v, solution),
                    )
                })
                .collect(),
        ),

        ExprKind::Tuple(elements) => ExprKind::Tuple(
            elements
                .iter()
                .map(|e| apply_to_expression(e, solution))
                .collect(),
        ),

        ExprKind::Grouping(inner) => {
            ExprKind::Grouping(Box::new(apply_to_expression(inner, solution)))
        },

        ExprKind::FieldAccess { object, field } => ExprKind::FieldAccess {
            object: Box::new(apply_to_expression(object, solution)),
            field: field.clone(),
        },

        ExprKind::TupleIndex { tuple, index } => ExprKind::TupleIndex {
            tuple: Box::new(apply_to_expression(tuple, solution)),
            index: *index,
        },

        ExprKind::MethodRef {
            receiver,
            candidates,
            method_name,
        } => ExprKind::MethodRef {
            receiver: Box::new(apply_to_expression(receiver, solution)),
            candidates: candidates.clone(),
            method_name: method_name.clone(),
        },

        ExprKind::PrimitiveMethodRef { receiver, method } => ExprKind::PrimitiveMethodRef {
            receiver: Box::new(apply_to_expression(receiver, solution)),
            method: *method,
        },

        ExprKind::Call {
            callee,
            arguments,
            substitutions,
        } => {
            let resolved_callee = apply_to_expression(callee, solution);
            let resolved_arguments: Vec<CallArgument> = arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect();

            // Check if callee is a MethodRef and we have a value resolution for this Call.
            // This handles the MethodRef pattern for protocol method calls (used by
            // desugared for-loops for iter()/next()).
            if let ExprKind::MethodRef {
                receiver,
                method_name,
                ..
            } = &resolved_callee.kind
            {
                if let Some(value_resolution) = solution.get_value(expr.id) {
                    // Create a new MethodRef with the resolved method symbol
                    let method_ref = Expression::method_ref(
                        (**receiver).clone(),
                        vec![value_resolution.symbol_id],
                        method_name.clone(),
                        resolved_callee.span.clone(),
                    );
                    ExprKind::Call {
                        callee: Box::new(method_ref),
                        arguments: resolved_arguments,
                        substitutions: value_resolution.substitutions.clone(),
                    }
                } else {
                    // No resolution found - pass through with original callee
                    ExprKind::Call {
                        callee: Box::new(resolved_callee),
                        arguments: resolved_arguments,
                        substitutions: resolve_substitutions(substitutions, solution),
                    }
                }
            } else {
                // Default: just pass through with resolved components
                ExprKind::Call {
                    callee: Box::new(resolved_callee),
                    arguments: resolved_arguments,
                    substitutions: resolve_substitutions(substitutions, solution),
                }
            }
        },

        ExprKind::PrimitiveMethodCall {
            receiver,
            method,
            arguments,
        } => {
            let resolved_receiver = apply_to_expression(receiver, solution);
            let resolved_arguments: Vec<CallArgument> = arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect();

            // If the receiver type was inferred, we may need to correct the method.
            // For example, if a placeholder IntGt was used but the receiver is Float,
            // we need to resolve it to FloatGt to generate the correct comparison.
            let resolved_method =
                PrimitiveMethod::lookup(&resolved_receiver.ty, method.name()).unwrap_or(*method);

            ExprKind::PrimitiveMethodCall {
                receiver: Box::new(resolved_receiver),
                method: resolved_method,
                arguments: resolved_arguments,
            }
        },

        ExprKind::DeferredMethodCall {
            receiver,
            method_name,
            arguments,
        } => {
            let resolved_receiver = apply_to_expression(receiver, solution);
            let resolved_arguments: Vec<CallArgument> = arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect();

            // Check if we have a resolved symbol for this expression
            if let Some(value_resolution) = solution.get_value(expr.id) {
                // Create a MethodRef with the resolved method symbol
                let method_ref = Expression::method_ref(
                    resolved_receiver.clone(),
                    vec![value_resolution.symbol_id],
                    method_name.clone(),
                    expr.span.clone(),
                );
                // Create a Call expression with the method ref as callee
                ExprKind::Call {
                    callee: Box::new(method_ref),
                    arguments: resolved_arguments,
                    substitutions: value_resolution.substitutions.clone(),
                }
            } else {
                // No resolution found - keep as deferred (will error during lowering)
                ExprKind::DeferredMethodCall {
                    receiver: Box::new(resolved_receiver),
                    method_name: method_name.clone(),
                    arguments: resolved_arguments,
                }
            }
        },

        ExprKind::DeferredStaticCall {
            target_ty,
            method_name,
            arguments,
            protocol_candidates,
        } => {
            let resolved_target_ty = resolve_type(target_ty, solution);
            let resolved_arguments: Vec<CallArgument> = arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect();

            // Check if we have a resolved symbol for this expression
            if let Some(value_resolution) = solution.get_value(expr.id) {
                // Get the type symbol for creating a TypeRef
                let type_symbol_id_opt = match resolved_target_ty.kind() {
                    TyKind::Struct { symbol, .. } => Some(symbol.metadata().id()),
                    TyKind::Enum { symbol, .. } => Some(symbol.metadata().id()),
                    _ => None,
                };

                if let Some(type_symbol_id) = type_symbol_id_opt {
                    // Create a TypeRef expression for the target type
                    let type_ref = Expression::type_ref(
                        type_symbol_id,
                        resolved_target_ty.clone(),
                        expr.span.clone(),
                    );

                    // Create a MethodRef with the resolved static method symbol
                    let method_ref = Expression::method_ref(
                        type_ref,
                        vec![value_resolution.symbol_id],
                        method_name.clone(),
                        expr.span.clone(),
                    );

                    // Create a Call expression with the method ref as callee
                    ExprKind::Call {
                        callee: Box::new(method_ref),
                        arguments: resolved_arguments,
                        substitutions: value_resolution.substitutions.clone(),
                    }
                } else {
                    // Cannot resolve static call on non-struct/enum type
                    // Keep as deferred (will error during lowering)
                    ExprKind::DeferredStaticCall {
                        target_ty: resolved_target_ty,
                        method_name: method_name.clone(),
                        arguments: resolved_arguments,
                        protocol_candidates: protocol_candidates.clone(),
                    }
                }
            } else {
                // No resolution found - keep as deferred (will error during lowering)
                ExprKind::DeferredStaticCall {
                    target_ty: resolved_target_ty,
                    method_name: method_name.clone(),
                    arguments: resolved_arguments,
                    protocol_candidates: protocol_candidates.clone(),
                }
            }
        },

        ExprKind::ImplicitStructInit {
            struct_type,
            arguments,
        } => ExprKind::ImplicitStructInit {
            struct_type: resolve_type(struct_type, solution),
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
        },

        ExprKind::DelegatingInit {
            initializer,
            arguments,
            substitutions,
        } => ExprKind::DelegatingInit {
            initializer: *initializer,
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
            substitutions: substitutions.clone(),
        },

        ExprKind::Assignment { target, value } => ExprKind::Assignment {
            target: Box::new(apply_to_expression(target, solution)),
            value: Box::new(apply_to_expression(value, solution)),
        },

        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => ExprKind::If {
            conditions: conditions
                .iter()
                .map(|c| apply_to_if_condition(c, solution))
                .collect(),
            then_branch: then_branch
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
            then_value: then_value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution))),
            else_branch: else_branch
                .as_ref()
                .map(|eb| apply_to_else_branch(eb, solution)),
        },

        ExprKind::While {
            loop_id,
            label,
            condition,
            body,
        } => ExprKind::While {
            loop_id: *loop_id,
            label: label.clone(),
            condition: Box::new(apply_to_expression(condition, solution)),
            body: body
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
        },

        ExprKind::WhileLet {
            loop_id,
            label,
            conditions,
            body,
            from_for_loop,
        } => ExprKind::WhileLet {
            loop_id: *loop_id,
            label: label.clone(),
            conditions: conditions
                .iter()
                .map(|c| apply_to_if_condition(c, solution))
                .collect(),
            body: body
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
            from_for_loop: *from_for_loop,
        },

        ExprKind::Loop {
            loop_id,
            label,
            body,
        } => ExprKind::Loop {
            loop_id: *loop_id,
            label: label.clone(),
            body: body
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
        },

        ExprKind::Return { value } => ExprKind::Return {
            value: value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution))),
        },

        ExprKind::Throw { value } => ExprKind::Throw {
            value: Box::new(apply_to_expression(value, solution)),
        },

        ExprKind::Closure {
            params,
            body,
            tail_expr,
            captures,
            uses_it,
            implicit_param,
        } => {
            // Apply solution to closure parameters
            let resolved_params = params.as_ref().map(|ps| {
                ps.iter()
                    .map(|p| kestrel_semantic_tree::expr::ClosureParam {
                        pattern: apply_to_pattern(&p.pattern, solution),
                        ty: resolve_type(&p.ty, solution),
                        is_type_annotated: p.is_type_annotated,
                        span: p.span.clone(),
                    })
                    .collect()
            });

            // Apply solution to body statements
            let resolved_body = body
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect();

            // Apply solution to tail expression
            let resolved_tail = tail_expr
                .as_ref()
                .map(|e| Box::new(apply_to_expression(e, solution)));

            // Apply solution to captures
            let resolved_captures = captures
                .iter()
                .map(|c| kestrel_semantic_tree::expr::Capture {
                    local_id: c.local_id,
                    name: c.name.clone(),
                    ty: resolve_type(&c.ty, solution),
                    kind: c.kind,
                    span: c.span.clone(),
                })
                .collect();

            // Apply solution to implicit_param
            let resolved_implicit_param = implicit_param
                .as_ref()
                .map(|(id, ty, span)| (*id, resolve_type(ty, solution), span.clone()));

            ExprKind::Closure {
                params: resolved_params,
                body: resolved_body,
                tail_expr: resolved_tail,
                captures: resolved_captures,
                uses_it: *uses_it,
                implicit_param: resolved_implicit_param,
            }
        },

        ExprKind::Match { scrutinee, arms } => {
            let resolved_scrutinee = Box::new(apply_to_expression(scrutinee, solution));
            let resolved_arms = arms
                .iter()
                .map(|arm| apply_to_match_arm(arm, solution))
                .collect();
            ExprKind::Match {
                scrutinee: resolved_scrutinee,
                arms: resolved_arms,
            }
        },

        ExprKind::Block { statements, value } => {
            let resolved_statements = statements
                .iter()
                .map(|stmt| apply_to_statement(stmt, solution))
                .collect();
            let resolved_value = value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution)));
            ExprKind::Block {
                statements: resolved_statements,
                value: resolved_value,
            }
        },

        // Language intrinsics - apply solution to arguments and resolve intrinsic types
        ExprKind::LangIntrinsic {
            intrinsic,
            arguments,
        } => ExprKind::LangIntrinsic {
            intrinsic: resolve_intrinsic(intrinsic, solution, &resolved_ty),
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
        },

        // Language intrinsic reference - no changes needed
        ExprKind::LangIntrinsicRef(intrinsic) => ExprKind::LangIntrinsicRef(intrinsic.clone()),

        // Subscript call - apply solution to receiver and arguments
        ExprKind::SubscriptCall {
            receiver,
            getter,
            arguments,
        } => ExprKind::SubscriptCall {
            receiver: Box::new(apply_to_expression(receiver, solution)),
            getter: *getter,
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
        },
        ExprKind::ProtocolPropertyAccess {
            receiver,
            field_id,
            property_name,
            protocol_id,
            is_static,
            has_setter,
        } => ExprKind::ProtocolPropertyAccess {
            receiver: Box::new(apply_to_expression(receiver, solution)),
            field_id: *field_id,
            property_name: property_name.clone(),
            protocol_id: *protocol_id,
            is_static: *is_static,
            has_setter: *has_setter,
        },
    };

    let processed = Expression::new(kind, resolved_ty.clone(), expr.span.clone(), expr.mutable);

    // Check if this expression needs promotion wrapping (FromValue.from())
    if let Some(promo) = solution.get_promotion(expr.id) {
        wrap_with_promotion(processed, promo)
    } else {
        processed
    }
}

/// Apply solution to a call argument.
fn apply_to_argument(arg: &CallArgument, solution: &Solution) -> CallArgument {
    CallArgument {
        label: arg.label.clone(),
        value: apply_to_expression(&arg.value, solution),
        span: arg.span.clone(),
    }
}

/// Apply solution to an else branch.
fn apply_to_else_branch(branch: &ElseBranch, solution: &Solution) -> ElseBranch {
    match branch {
        ElseBranch::Block { statements, value } => ElseBranch::Block {
            statements: statements
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
            value: value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution))),
        },
        ElseBranch::ElseIf(if_expr) => {
            ElseBranch::ElseIf(Box::new(apply_to_expression(if_expr, solution)))
        },
    }
}

/// Apply solution to an if condition.
fn apply_to_if_condition(
    condition: &kestrel_semantic_tree::expr::IfCondition,
    solution: &Solution,
) -> kestrel_semantic_tree::expr::IfCondition {
    use kestrel_semantic_tree::expr::IfCondition;

    match condition {
        IfCondition::Expr(expr) => IfCondition::Expr(apply_to_expression(expr, solution)),
        IfCondition::Let {
            pattern,
            value,
            span,
        } => IfCondition::Let {
            pattern: apply_to_pattern(pattern, solution),
            value: apply_to_expression(value, solution),
            span: span.clone(),
        },
    }
}

/// Apply solution to a match arm.
fn apply_to_match_arm(
    arm: &kestrel_semantic_tree::expr::MatchArm,
    solution: &Solution,
) -> kestrel_semantic_tree::expr::MatchArm {
    let resolved_pattern = apply_to_pattern(&arm.pattern, solution);
    let resolved_guard = arm.guard.as_ref().map(|g| apply_to_expression(g, solution));
    let resolved_body = apply_to_expression(&arm.body, solution);

    kestrel_semantic_tree::expr::MatchArm {
        pattern: resolved_pattern,
        guard: resolved_guard,
        body: resolved_body,
        span: arm.span.clone(),
    }
}

/// Apply solution to a pattern.
fn apply_to_pattern(pattern: &Pattern, solution: &Solution) -> Pattern {
    use kestrel_semantic_tree::pattern::PatternKind;

    let resolved_ty = resolve_type(&pattern.ty, solution);

    let kind = match &pattern.kind {
        // Simple patterns - just clone
        PatternKind::Local {
            local_id,
            mutability,
            name,
        } => PatternKind::Local {
            local_id: *local_id,
            mutability: *mutability,
            name: name.clone(),
        },
        PatternKind::Wildcard => PatternKind::Wildcard,
        PatternKind::Literal { value } => PatternKind::Literal {
            value: value.clone(),
        },
        PatternKind::Rest => PatternKind::Rest,
        PatternKind::Error => PatternKind::Error,

        // Compound patterns - recurse
        PatternKind::Tuple {
            prefix,
            has_rest,
            suffix,
        } => PatternKind::Tuple {
            prefix: prefix
                .iter()
                .map(|p| apply_to_pattern(p, solution))
                .collect(),
            has_rest: *has_rest,
            suffix: suffix
                .iter()
                .map(|p| apply_to_pattern(p, solution))
                .collect(),
        },
        PatternKind::EnumVariant {
            case_id,
            case_name,
            bindings,
        } => PatternKind::EnumVariant {
            case_id: *case_id,
            case_name: case_name.clone(),
            bindings: bindings
                .iter()
                .map(|b| kestrel_semantic_tree::pattern::EnumPatternBinding {
                    label: b.label.clone(),
                    pattern: Box::new(apply_to_pattern(&b.pattern, solution)),
                    span: b.span.clone(),
                })
                .collect(),
        },
        PatternKind::Range {
            start,
            end,
            inclusive,
        } => PatternKind::Range {
            start: start.clone(),
            end: end.clone(),
            inclusive: *inclusive,
        },
        PatternKind::Struct {
            struct_id,
            struct_name,
            fields,
            has_rest,
        } => PatternKind::Struct {
            struct_id: *struct_id,
            struct_name: struct_name.clone(),
            fields: fields
                .iter()
                .map(|f| kestrel_semantic_tree::pattern::StructPatternField {
                    field_name: f.field_name.clone(),
                    pattern: apply_to_pattern(&f.pattern, solution),
                    span: f.span.clone(),
                })
                .collect(),
            has_rest: *has_rest,
        },
        PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => PatternKind::Array {
            prefix: prefix
                .iter()
                .map(|p| apply_to_pattern(p, solution))
                .collect(),
            rest: rest.clone(),
            suffix: suffix
                .iter()
                .map(|p| apply_to_pattern(p, solution))
                .collect(),
        },
        PatternKind::Or { alternatives } => PatternKind::Or {
            alternatives: alternatives
                .iter()
                .map(|p| apply_to_pattern(p, solution))
                .collect(),
        },
        PatternKind::At {
            name,
            local_id,
            mutability,
            subpattern,
        } => PatternKind::At {
            name: name.clone(),
            local_id: *local_id,
            mutability: *mutability,
            subpattern: Box::new(apply_to_pattern(subpattern, solution)),
        },
    };

    Pattern::new(kind, resolved_ty, pattern.span.clone())
}

/// Resolve a type using the solution.
/// If the type is an inference placeholder or unresolved function, look it up in the solution.
/// Recursively resolves nested types.
fn resolve_type(ty: &Ty, solution: &Solution) -> Ty {
    // Check if this type has a substitution (for Infer and UnresolvedFunction types)
    if let Some(resolved) = solution.get_type(ty.id()) {
        return resolve_type(resolved, solution);
    }

    // For compound types, recursively resolve components
    match ty.kind() {
        TyKind::Tuple(elements) => {
            let resolved_elements: Vec<_> =
                elements.iter().map(|e| resolve_type(e, solution)).collect();
            Ty::tuple(resolved_elements, ty.span().clone())
        },
        // Note: Array[T] struct types are handled by the Struct case above
        TyKind::Function {
            params,
            return_type,
        } => {
            let resolved_params: Vec<_> =
                params.iter().map(|p| resolve_type(p, solution)).collect();
            let resolved_return = resolve_type(return_type, solution);
            Ty::function(resolved_params, resolved_return, ty.span().clone())
        },
        TyKind::UnresolvedFunction {
            param_info,
            return_type,
        } => {
            let resolved_return = resolve_type(return_type, solution);
            let resolved_param_info = match param_info {
                ParamInfo::Unconstrained => ParamInfo::Unconstrained,
                ParamInfo::ImplicitIt { it_type } => ParamInfo::ImplicitIt {
                    it_type: Box::new(resolve_type(it_type, solution)),
                },
                ParamInfo::Explicit { param_types } => ParamInfo::Explicit {
                    param_types: param_types
                        .iter()
                        .map(|p| resolve_type(p, solution))
                        .collect(),
                },
            };
            Ty::unresolved_function(resolved_param_info, resolved_return, ty.span().clone())
        },
        // Struct types with substitutions - resolve any inference placeholders in substitutions
        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            let resolved_subs = resolve_substitutions(substitutions, solution);
            Ty::generic_struct(symbol.clone(), resolved_subs, ty.span().clone())
        },
        // Enum types with substitutions - resolve any inference placeholders in substitutions
        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            let resolved_subs = resolve_substitutions(substitutions, solution);
            Ty::generic_enum(symbol.clone(), resolved_subs, ty.span().clone())
        },
        // Pointer types - resolve pointee
        TyKind::Pointer(pointee) => Ty::pointer(resolve_type(pointee, solution), ty.span().clone()),
        // Other types - no inference placeholders to resolve
        _ => ty.clone(),
    }
}

/// Resolve inference placeholders within substitutions.
fn resolve_substitutions(subs: &Substitutions, solution: &Solution) -> Substitutions {
    let mut resolved = Substitutions::new();
    for (id, ty) in subs.iter() {
        resolved.insert(*id, resolve_type(ty, solution));
    }
    resolved
}

/// Resolve embedded types within a LangIntrinsic, using the expression's resolved type.
///
/// Many lang intrinsics carry type information that may start as inference placeholders
/// (e.g., `PtrNull { pointee_ty: Ty::infer() }`). After type inference, we need to
/// resolve these placeholders to their concrete types. For intrinsics that return pointer
/// types, we can extract the pointee type from the expression's resolved return type.
fn resolve_intrinsic(
    intrinsic: &kestrel_semantic_tree::expr::LangIntrinsic,
    solution: &Solution,
    expr_ty: &Ty,
) -> kestrel_semantic_tree::expr::LangIntrinsic {
    use kestrel_semantic_tree::expr::LangIntrinsic;

    match intrinsic {
        // Intrinsics that return Pointer[T] - extract T from the expression's type
        LangIntrinsic::PtrNull { pointee_ty } => {
            let resolved_pointee = if let TyKind::Pointer(ptr_pointee) = expr_ty.kind() {
                // Extract pointee type from the expression's resolved pointer type
                resolve_type(ptr_pointee, solution)
            } else {
                // Fallback: just resolve the embedded type
                resolve_type(pointee_ty, solution)
            };
            LangIntrinsic::PtrNull {
                pointee_ty: resolved_pointee,
            }
        },
        LangIntrinsic::PtrFromAddress { pointee_ty } => {
            let resolved_pointee = if let TyKind::Pointer(ptr_pointee) = expr_ty.kind() {
                resolve_type(ptr_pointee, solution)
            } else {
                resolve_type(pointee_ty, solution)
            };
            LangIntrinsic::PtrFromAddress {
                pointee_ty: resolved_pointee,
            }
        },
        LangIntrinsic::PtrTo { pointee_ty } => {
            let resolved_pointee = if let TyKind::Pointer(ptr_pointee) = expr_ty.kind() {
                resolve_type(ptr_pointee, solution)
            } else {
                resolve_type(pointee_ty, solution)
            };
            LangIntrinsic::PtrTo {
                pointee_ty: resolved_pointee,
            }
        },
        LangIntrinsic::PtrRead { pointee_ty } => LangIntrinsic::PtrRead {
            pointee_ty: resolve_type(pointee_ty, solution),
        },
        LangIntrinsic::PtrWrite { pointee_ty } => LangIntrinsic::PtrWrite {
            pointee_ty: resolve_type(pointee_ty, solution),
        },
        LangIntrinsic::CastPtr { target_ty } => {
            let resolved_target = if let TyKind::Pointer(ptr_pointee) = expr_ty.kind() {
                resolve_type(ptr_pointee, solution)
            } else {
                resolve_type(target_ty, solution)
            };
            LangIntrinsic::CastPtr {
                target_ty: resolved_target,
            }
        },
        LangIntrinsic::SizeOf { ty } => LangIntrinsic::SizeOf {
            ty: resolve_type(ty, solution),
        },
        LangIntrinsic::AlignOf { ty } => LangIntrinsic::AlignOf {
            ty: resolve_type(ty, solution),
        },

        // Intrinsics without embedded types - just clone
        LangIntrinsic::PanicUnwind => LangIntrinsic::PanicUnwind,
        LangIntrinsic::Cast { from, to } => LangIntrinsic::Cast {
            from: *from,
            to: *to,
        },
        LangIntrinsic::IntBinary { primitive, op } => LangIntrinsic::IntBinary {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::IntBinarySigned { primitive, op } => LangIntrinsic::IntBinarySigned {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::IntBinaryUnsigned { primitive, op } => LangIntrinsic::IntBinaryUnsigned {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::IntUnary { primitive, op } => LangIntrinsic::IntUnary {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::FloatBinary { primitive, op } => LangIntrinsic::FloatBinary {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::FloatUnary { primitive, op } => LangIntrinsic::FloatUnary {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::FloatConst {
            primitive,
            constant,
        } => LangIntrinsic::FloatConst {
            primitive: *primitive,
            constant: *constant,
        },
        LangIntrinsic::FloatPred { primitive, pred } => LangIntrinsic::FloatPred {
            primitive: *primitive,
            pred: *pred,
        },
        LangIntrinsic::FloatMath { primitive, op } => LangIntrinsic::FloatMath {
            primitive: *primitive,
            op: *op,
        },
        LangIntrinsic::FloatFma { primitive } => LangIntrinsic::FloatFma {
            primitive: *primitive,
        },
        LangIntrinsic::FloatCopysign { primitive } => LangIntrinsic::FloatCopysign {
            primitive: *primitive,
        },
        LangIntrinsic::PtrToAddress => LangIntrinsic::PtrToAddress,
        LangIntrinsic::PtrOffset => LangIntrinsic::PtrOffset,
        LangIntrinsic::PtrIsNull => LangIntrinsic::PtrIsNull,
        LangIntrinsic::I1Eq => LangIntrinsic::I1Eq,
        LangIntrinsic::I1And => LangIntrinsic::I1And,
        LangIntrinsic::I1Or => LangIntrinsic::I1Or,
        LangIntrinsic::I1Not => LangIntrinsic::I1Not,
        LangIntrinsic::AtomicAdd => LangIntrinsic::AtomicAdd,
        LangIntrinsic::AtomicSub => LangIntrinsic::AtomicSub,
    }
}

/// Wrap an expression with a `FromValue.from()` call for promotion.
///
/// This is used when an expression needs to be implicitly promoted from `T` to
/// `Optional[T]` or `Result[T, E]`. The expression is wrapped in a static method
/// call: `TargetType.from(inner)`.
fn wrap_with_promotion(inner: Expression, promo: &PromotionInfo) -> Expression {
    let span = inner.span.clone();

    // Get the type symbol ID for the target type (for creating a TypeRef)
    let type_symbol_id = match promo.target_ty.kind() {
        TyKind::Struct { symbol, .. } => symbol.metadata().id(),
        TyKind::Enum { symbol, .. } => symbol.metadata().id(),
        _ => {
            // Shouldn't happen for Optional/Result, but fall back to unchanged
            return inner;
        },
    };

    // Create a TypeRef expression for the target type
    let type_ref = Expression::type_ref(type_symbol_id, promo.target_ty.clone(), span.clone());

    // Create a MethodRef with the from() method symbol
    let method_ref = Expression::method_ref(
        type_ref,
        vec![promo.from_method],
        "from".to_string(),
        span.clone(),
    );

    // Create the argument for from(value:)
    let arg = CallArgument::labeled("value".to_string(), inner, span.clone());

    // Create a Call expression: TargetType.from(inner)
    Expression::generic_call(
        method_ref,
        vec![arg],
        promo.substitutions.clone(),
        promo.target_ty.clone(),
        span,
    )
}

/// Update all locals in the container with their resolved types from the solution.
///
/// This is necessary because pattern-bound variables are initially created with
/// `Ty::infer()` placeholder types. After type inference solves the constraints,
/// the actual types are known but the `Local` entries in the container still have
/// the old placeholder types. When subsequent code references these locals via
/// `LocalRef`, it reads the type from the container, so we must update it.
///
/// Additionally, compound types (like `Struct[Infer]`) may contain inference
/// placeholders in their substitutions that need to be resolved.
///
/// The `self` local is created with `SelfType` which needs to be resolved to
/// the concrete type (struct, enum, or extension target type).
pub fn apply_solution_to_locals(
    container: &dyn LocalContainer,
    solution: &Solution,
    concrete_self_type: Option<&Ty>,
) {
    for local in container.locals() {
        let ty = local.ty();

        // Handle SelfType -> concrete type (must check first before resolve_type)
        if matches!(ty.kind(), TyKind::SelfType) {
            if let Some(concrete) = concrete_self_type {
                container.update_local_type(local.id(), concrete.clone());
            }
            continue;
        }

        // Resolve ALL types, not just TyKind::Infer at the top level.
        // This handles cases like `Wrapper[Infer]` where the struct's substitutions
        // contain inference placeholders that need to be resolved.
        let resolved = resolve_type(ty, solution);
        if resolved.id() != ty.id() {
            // Only update if the type actually changed
            container.update_local_type(local.id(), resolved);
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require setting up expressions and solutions
}
