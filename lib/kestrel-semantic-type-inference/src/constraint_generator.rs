//! Constraint generation from code blocks.
//!
//! This module walks a `CodeBlock` and generates type inference constraints
//! for the expressions and statements within it.

use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::context::InferenceContext;

/// Generate type inference constraints for a code block.
///
/// This walks all statements and expressions in the block, generating
/// constraints that the solver will use to infer types.
///
/// # Arguments
/// * `ctx` - The inference context to add constraints to
/// * `block` - The code block to generate constraints for
/// * `return_type` - The expected return type (for yield expression)
pub fn generate_constraints(
    ctx: &mut InferenceContext<'_>,
    block: &CodeBlock,
    return_type: Option<&Ty>,
) {
    // Store return type in context for use by nested return statements
    ctx.set_return_type(return_type.cloned());

    // Process all statements
    for stmt in &block.statements {
        generate_statement_constraints(ctx, stmt);
    }

    // Process yield expression if present
    if let Some(yield_expr) = block.yield_expr() {
        generate_expression_constraints(ctx, yield_expr);

        // Equate yield expression type with return type
        // ret_ty is expected, yield_expr.ty is found
        if let Some(ret_ty) = return_type {
            ctx.register_type(ret_ty);
            ctx.register_type(&yield_expr.ty);
            ctx.equate(ret_ty.id(), yield_expr.ty.id(), yield_expr.span.clone());
        }
    }
}

/// Generate constraints for a statement.
fn generate_statement_constraints(ctx: &mut InferenceContext<'_>, stmt: &Statement) {
    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            // Generate constraints for the pattern
            generate_pattern_constraints(ctx, pattern);

            // If there's an initializer, equate its type with the pattern type
            if let Some(init) = value {
                generate_expression_constraints(ctx, init);
                ctx.register_type(&init.ty);
                ctx.equate(pattern.ty.id(), init.ty.id(), stmt.span.clone());
            }
        }
        StatementKind::Expr(expr) => {
            generate_expression_constraints(ctx, expr);
        }
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Generate constraints for each condition in the chain
            for condition in conditions {
                generate_if_condition_constraints(ctx, condition);
            }

            // Generate constraints for the else block statements
            for else_stmt in &else_block.statements {
                generate_statement_constraints(ctx, else_stmt);
            }
            // The else block's yield expression (if any)
            if let Some(yield_expr) = else_block.yield_expr() {
                generate_expression_constraints(ctx, yield_expr);
            }
        }
        StatementKind::Deinit { .. } => {
            // Deinit statement doesn't generate any type constraints
            // The move tracking is already handled during body resolution
        }
    }
}

/// Generate constraints for a pattern.
///
/// This registers the pattern's type and generates constraints based on the pattern kind:
/// - For tuples: creates tuple type constraints from elements
/// - For literals: constrains to the literal's type
/// - For enum variants: the case resolution happens later during type application
///
/// # Arguments
/// * `ctx` - The inference context to add constraints to
/// * `pattern` - The pattern to generate constraints for
pub fn generate_pattern_constraints(ctx: &mut InferenceContext<'_>, pattern: &Pattern) {
    // Register the pattern's type
    ctx.register_type(&pattern.ty);

    match &pattern.kind {
        PatternKind::Local { .. } => {
            // Local bindings just register their type - nothing more needed
        }

        PatternKind::Wildcard => {
            // Wildcard patterns match anything - type is already registered
        }

        PatternKind::Tuple {
            prefix,
            has_rest,
            suffix,
        } => {
            // For tuple patterns, generate constraints for each element
            // and ensure the tuple type matches
            if let TyKind::Tuple(elem_tys) = pattern.ty.kind() {
                // Handle prefix elements (matched from the start)
                for (i, elem) in prefix.iter().enumerate() {
                    generate_pattern_constraints(ctx, elem);
                    if let Some(elem_ty) = elem_tys.get(i) {
                        ctx.register_type(elem_ty);
                        ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                    }
                }

                // Handle suffix elements (matched from the end)
                let suffix_start = elem_tys.len().saturating_sub(suffix.len());
                for (i, elem) in suffix.iter().enumerate() {
                    generate_pattern_constraints(ctx, elem);
                    if let Some(elem_ty) = elem_tys.get(suffix_start + i) {
                        ctx.register_type(elem_ty);
                        ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                    }
                }
            } else {
                // Pattern type is not a tuple - still process all elements
                for elem in prefix.iter().chain(suffix.iter()) {
                    generate_pattern_constraints(ctx, elem);
                }
            }
        }

        PatternKind::Literal { .. } => {
            // Literal patterns have concrete types - nothing more needed
            // The type is set during pattern creation
        }

        PatternKind::EnumVariant {
            case_name,
            bindings,
            ..
        } => {
            // For enum patterns, generate constraints for each binding
            for binding in bindings {
                generate_pattern_constraints(ctx, &binding.pattern);
            }

            // Generate a constraint to validate the enum case exists and connect
            // binding types to the enum case's parameter types.
            let binding_tys: Vec<(Option<String>, _)> = bindings
                .iter()
                .map(|b| {
                    ctx.register_type(&b.pattern.ty);
                    (b.label.clone(), b.pattern.ty.id())
                })
                .collect();

            ctx.enum_pattern_binding(
                pattern.ty.id(),
                case_name.clone(),
                binding_tys,
                pattern.span.clone(),
            );
        }

        PatternKind::Range { .. } => {
            // Range patterns have concrete types (Int or Char) - type is already set
        }

        PatternKind::Struct {
            struct_name,
            fields,
            has_rest,
            ..
        } => {
            // For struct patterns, generate constraints for each field pattern
            for field in fields {
                generate_pattern_constraints(ctx, &field.pattern);
            }

            // Generate struct pattern binding constraint to connect field types
            // to the struct's field types
            let field_bindings: Vec<(String, _)> = fields
                .iter()
                .map(|f| {
                    ctx.register_type(&f.pattern.ty);
                    (f.field_name.clone(), f.pattern.ty.id())
                })
                .collect();

            ctx.struct_pattern_binding(
                pattern.ty.id(),
                struct_name.clone(),
                field_bindings,
                *has_rest,
                pattern.span.clone(),
            );
        }

        PatternKind::Array { prefix, suffix, .. } => {
            // For array patterns, generate constraints for prefix and suffix patterns
            for elem in prefix {
                generate_pattern_constraints(ctx, elem);
            }
            for elem in suffix {
                generate_pattern_constraints(ctx, elem);
            }
            // The rest pattern (.. or ..name) is just a marker/binding - no pattern constraints needed
        }

        PatternKind::Or { alternatives } => {
            // For or-patterns, generate constraints for each alternative
            // and equate their types with the or-pattern's type
            for alt in alternatives {
                generate_pattern_constraints(ctx, alt);
                // Each alternative's type must equal the or-pattern's type
                ctx.equate(pattern.ty.id(), alt.ty.id(), alt.span.clone());
            }
        }

        PatternKind::At { subpattern, .. } => {
            // For at-patterns, generate constraints for the subpattern
            generate_pattern_constraints(ctx, subpattern);
            // The @ pattern's type must equal the subpattern's type
            ctx.equate(pattern.ty.id(), subpattern.ty.id(), pattern.span.clone());
        }

        PatternKind::Rest => {
            // Rest patterns are just markers - no additional constraints needed
        }

        PatternKind::Error => {
            // Error patterns are poison values - don't generate constraints
        }
    }
}

/// Generate constraints for an if condition (used by if-let, while-let, guard-let chains).
fn generate_if_condition_constraints(
    ctx: &mut InferenceContext<'_>,
    condition: &kestrel_semantic_tree::expr::IfCondition,
) {
    use kestrel_semantic_tree::expr::IfCondition;
    match condition {
        IfCondition::Expr(expr) => {
            generate_expression_constraints(ctx, expr);
            // Boolean condition must be Bool
            let bool_ty = Ty::bool(expr.span.clone());
            ctx.register_type(&bool_ty);
            ctx.equate(expr.ty.id(), bool_ty.id(), expr.span.clone());
        }
        IfCondition::Let { pattern, value, .. } => {
            // Generate constraints for the scrutinee expression
            generate_expression_constraints(ctx, value);
            // Generate constraints for the pattern (pattern type == scrutinee type)
            generate_pattern_constraints(ctx, pattern);
            ctx.equate(pattern.ty.id(), value.ty.id(), value.span.clone());
        }
    }
}

/// Generate constraints for an expression.
fn generate_expression_constraints(ctx: &mut InferenceContext<'_>, expr: &Expression) {
    // Register this expression's type
    ctx.register_type(&expr.ty);

    match &expr.kind {
        // Literals have concrete types - no constraints needed
        ExprKind::Literal(_) => {}

        // Arrays: all elements must have the same type
        ExprKind::Array(elements) => {
            if let TyKind::Array(elem_ty) = expr.ty.kind() {
                ctx.register_type(elem_ty);
                for elem in elements {
                    generate_expression_constraints(ctx, elem);
                    ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                }
            }
        }

        // Tuples: each element has its corresponding type
        ExprKind::Tuple(elements) => {
            if let TyKind::Tuple(elem_tys) = expr.ty.kind() {
                for (elem, elem_ty) in elements.iter().zip(elem_tys.iter()) {
                    generate_expression_constraints(ctx, elem);
                    ctx.register_type(elem_ty);
                    ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                }
            }
        }

        // Grouping: just process the inner expression
        ExprKind::Grouping(inner) => {
            generate_expression_constraints(ctx, inner);
            ctx.equate(inner.ty.id(), expr.ty.id(), expr.span.clone());
        }

        // References: type is already set during binding
        ExprKind::LocalRef(_) | ExprKind::SymbolRef(_) | ExprKind::TypeRef(_) => {}
        ExprKind::OverloadedRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef => {}

        // Field access: type is the field type
        ExprKind::FieldAccess { object, field } => {
            generate_expression_constraints(ctx, object);
            // If the expression type is Infer, generate a member access constraint
            // so the solver can resolve the field type once the receiver type is known
            if matches!(expr.ty.kind(), TyKind::Infer) {
                ctx.register_type(&object.ty);
                ctx.register_type(&expr.ty);
                ctx.member_access(
                    object.ty.id(),
                    field.clone(),
                    false, // instance access
                    expr.ty.id(),
                    expr.id,
                    expr.span.clone(),
                );
            }
        }

        // Tuple index: type is the element type
        ExprKind::TupleIndex { tuple, .. } => {
            generate_expression_constraints(ctx, tuple);
        }

        // Method reference: process receiver
        ExprKind::MethodRef { receiver, .. } => {
            generate_expression_constraints(ctx, receiver);
        }

        // Calls
        ExprKind::Call {
            callee, arguments, ..
        } => {
            generate_expression_constraints(ctx, callee);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }

            // Generate constraints between argument types and parameter types
            // This enables bidirectional type inference for closures passed as arguments
            match callee.ty.kind() {
                TyKind::Function { params, .. } => {
                    for (arg, param_ty) in arguments.iter().zip(params.iter()) {
                        ctx.register_type(&arg.value.ty);
                        ctx.register_type(param_ty);
                        ctx.equate(arg.value.ty.id(), param_ty.id(), arg.span.clone());
                    }
                }
                TyKind::UnresolvedFunction { .. } => {
                    // Unresolved function - can't generate param constraints yet
                    // The closure will be resolved through other constraints
                }
                _ => {
                    // Callee type is not a function - might be an error or inference needed
                }
            }
        }

        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            generate_expression_constraints(ctx, receiver);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
        }

        ExprKind::DeferredMethodCall {
            receiver,
            method_name,
            arguments,
        } => {
            generate_expression_constraints(ctx, receiver);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
            // Generate a member access constraint to resolve the method once receiver type is known
            ctx.register_type(&receiver.ty);
            ctx.register_type(&expr.ty);
            ctx.member_access(
                receiver.ty.id(),
                method_name.clone(),
                false, // instance method call
                expr.ty.id(),
                expr.id,
                expr.span.clone(),
            );
        }

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }

            // Generate constraints between argument types and field types
            // This enables bidirectional type inference for closures in struct fields
            if let Some((struct_sym, substitutions)) = expr.ty.as_struct_with_subs() {
                // Get field symbols from struct children
                let fields: Vec<_> = struct_sym
                    .metadata()
                    .children()
                    .into_iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                    .filter_map(|c| c.downcast_arc::<FieldSymbol>().ok())
                    .collect();

                // Equate each argument type with its corresponding field type
                for (arg, field) in arguments.iter().zip(fields.iter()) {
                    // Get field type from TypedBehavior (resolved type) or fallback to field_type
                    let raw_field_ty = field
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|typed| typed.ty().clone())
                        .unwrap_or_else(|| field.field_type().clone());
                    let field_ty = raw_field_ty.apply_substitutions(substitutions);
                    ctx.register_type(&arg.value.ty);
                    ctx.register_type(&field_ty);
                    ctx.equate(arg.value.ty.id(), field_ty.id(), arg.span.clone());
                }
            }
        }

        // Assignment
        ExprKind::Assignment { target, value } => {
            generate_expression_constraints(ctx, target);
            generate_expression_constraints(ctx, value);
            ctx.equate(target.ty.id(), value.ty.id(), expr.span.clone());
        }

        // Control flow
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Process each condition
            for condition in conditions {
                generate_if_condition_constraints(ctx, condition);
            }

            // Process then branch
            for stmt in then_branch {
                generate_statement_constraints(ctx, stmt);
            }
            if let Some(then_val) = then_value {
                generate_expression_constraints(ctx, then_val);
                // Only equate then value type with expression type when there's an else branch.
                // Without else, the if expression type is () and the then value is discarded.
                if else_branch.is_some() {
                    ctx.equate(expr.ty.id(), then_val.ty.id(), then_val.span.clone());
                }
            }

            // Process else branch
            if let Some(else_br) = else_branch {
                match else_br {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            generate_statement_constraints(ctx, stmt);
                        }
                        if let Some(else_val) = value {
                            generate_expression_constraints(ctx, else_val);
                            // Else branch value type equals expression type
                            ctx.equate(expr.ty.id(), else_val.ty.id(), else_val.span.clone());
                        }
                    }
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(else_if) => {
                        generate_expression_constraints(ctx, else_if);
                        // Else-if expression type equals this expression type
                        ctx.equate(expr.ty.id(), else_if.ty.id(), else_if.span.clone());
                    }
                }
            }
        }

        ExprKind::While {
            condition, body, ..
        } => {
            generate_expression_constraints(ctx, condition);
            // Condition must be Bool
            let bool_ty = Ty::bool(condition.span.clone());
            ctx.register_type(&bool_ty);
            ctx.equate(condition.ty.id(), bool_ty.id(), condition.span.clone());

            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        }

        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            // Generate constraints for each condition in the chain
            for condition in conditions {
                generate_if_condition_constraints(ctx, condition);
            }

            // Generate constraints for body statements
            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        }

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        }

        ExprKind::Break { .. } | ExprKind::Continue { .. } => {}

        ExprKind::Return { value } => {
            if let Some(val) = value {
                generate_expression_constraints(ctx, val);

                // Equate return value type with function return type
                if let Some(ret_ty) = ctx.return_type().cloned() {
                    ctx.register_type(&ret_ty);
                    ctx.register_type(&val.ty);
                    ctx.equate(ret_ty.id(), val.ty.id(), val.span.clone());
                }
            } else {
                // Return with no value - equate Unit with return type
                if let Some(ret_ty) = ctx.return_type().cloned() {
                    let unit_ty = Ty::unit(expr.span.clone());
                    ctx.register_type(&unit_ty);
                    ctx.register_type(&ret_ty);
                    ctx.equate(unit_ty.id(), ret_ty.id(), expr.span.clone());
                }
            }
        }

        ExprKind::Closure {
            body,
            tail_expr,
            params,
            uses_it,
            implicit_param,
            ..
        } => {
            // Register the closure type
            ctx.register_type(&expr.ty);

            // Handle based on the closure's type
            match expr.ty.kind() {
                // Explicit params - closure has a concrete function type
                TyKind::Function {
                    params: closure_param_tys,
                    return_type: closure_return_ty,
                } => {
                    ctx.register_type(closure_return_ty);

                    // Record closure metadata for better error messages
                    let param_count = params.as_ref().map(|p| p.len()).unwrap_or(0);
                    let has_explicit_params = params.is_some();
                    ctx.register_closure_metadata(crate::context::ClosureMetadata {
                        expr_id: expr.id,
                        param_count,
                        uses_it: *uses_it,
                        has_explicit_params,
                        span: expr.span.clone(),
                        ty_id: expr.ty.id(),
                    });

                    // Register parameter types and create constraints
                    if let Some(param_list) = params {
                        for (param, param_ty) in param_list.iter().zip(closure_param_tys.iter()) {
                            ctx.register_type(&param.ty);
                            ctx.register_type(param_ty);
                            // Equate the parameter's declared/inferred type with the function type parameter
                            ctx.equate(param.ty.id(), param_ty.id(), param.span.clone());
                        }
                    }

                    // Handle implicit `it` parameter constraints
                    if let Some((_, it_ty, it_span)) = implicit_param {
                        if let Some(first_param_ty) = closure_param_tys.first() {
                            ctx.register_type(it_ty);
                            ctx.register_type(first_param_ty);
                            // Equate `it` type with the first function parameter type
                            ctx.equate(it_ty.id(), first_param_ty.id(), it_span.clone());
                        }
                    }

                    // Generate constraints for body statements
                    for stmt in body {
                        generate_statement_constraints(ctx, stmt);
                    }

                    // Generate constraints for tail expression and return type
                    if let Some(tail) = tail_expr {
                        generate_expression_constraints(ctx, tail);
                        // Equate tail expression type with closure return type
                        ctx.equate(tail.ty.id(), closure_return_ty.id(), tail.span.clone());
                    } else {
                        // No tail expression means return type should be Unit
                        let unit_ty = Ty::unit(expr.span.clone());
                        ctx.register_type(&unit_ty);
                        ctx.equate(unit_ty.id(), closure_return_ty.id(), expr.span.clone());
                    }
                }

                // UnresolvedFunction - closure without explicit params
                TyKind::UnresolvedFunction {
                    param_info,
                    return_type,
                } => {
                    use kestrel_semantic_tree::ty::ParamInfo;

                    // Register nested types
                    ctx.register_type(return_type);

                    // Register and handle param info
                    match param_info {
                        ParamInfo::ImplicitIt { it_type } => {
                            ctx.register_type(it_type);
                            // Record closure metadata for better error messages
                            ctx.register_closure_metadata(crate::context::ClosureMetadata {
                                expr_id: expr.id,
                                param_count: 1, // ImplicitIt means exactly 1 param
                                uses_it: true,
                                has_explicit_params: false,
                                span: expr.span.clone(),
                                ty_id: expr.ty.id(),
                            });
                        }
                        ParamInfo::Unconstrained => {
                            // Record closure metadata for better error messages
                            ctx.register_closure_metadata(crate::context::ClosureMetadata {
                                expr_id: expr.id,
                                param_count: 0, // Will be determined by context
                                uses_it: false,
                                has_explicit_params: false,
                                span: expr.span.clone(),
                                ty_id: expr.ty.id(),
                            });
                        }
                        ParamInfo::Explicit { param_types } => {
                            for pt in param_types {
                                ctx.register_type(pt);
                            }
                            // Record closure metadata for better error messages
                            ctx.register_closure_metadata(crate::context::ClosureMetadata {
                                expr_id: expr.id,
                                param_count: param_types.len(),
                                uses_it: false,
                                has_explicit_params: true,
                                span: expr.span.clone(),
                                ty_id: expr.ty.id(),
                            });
                        }
                    }

                    // Generate constraints for body statements
                    for stmt in body {
                        generate_statement_constraints(ctx, stmt);
                    }

                    // Equate tail expression with return type
                    if let Some(tail) = tail_expr {
                        generate_expression_constraints(ctx, tail);
                        ctx.equate(tail.ty.id(), return_type.id(), tail.span.clone());
                    } else {
                        // No tail expression means return type should be Unit
                        let unit_ty = Ty::unit(expr.span.clone());
                        ctx.register_type(&unit_ty);
                        ctx.equate(unit_ty.id(), return_type.id(), expr.span.clone());
                    }
                }

                // Fallback - shouldn't happen with well-formed trees
                _ => {
                    for stmt in body {
                        generate_statement_constraints(ctx, stmt);
                    }
                    if let Some(tail) = tail_expr {
                        generate_expression_constraints(ctx, tail);
                    }
                }
            }
        }

        // Enum case reference: type is already set during binding
        ExprKind::EnumCase { .. } => {}

        // Implicit member access: will be resolved during type inference
        ExprKind::ImplicitMemberAccess {
            member_name,
            arguments,
        } => {
            // Register the expression type
            ctx.register_type(&expr.ty);

            // Process argument expressions and collect their type IDs
            let argument_tys: Vec<(Option<String>, _)> = arguments
                .as_ref()
                .map(|args| {
                    args.iter()
                        .map(|arg| {
                            generate_expression_constraints(ctx, &arg.value);
                            ctx.register_type(&arg.value.ty);
                            (arg.label.clone(), arg.value.ty.id())
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Generate the ImplicitMember constraint
            ctx.implicit_member(
                expr.ty.id(),
                member_name.clone(),
                argument_tys,
                expr.id,
                expr.span.clone(),
            );
        }

        ExprKind::Match { scrutinee, arms } => {
            // Generate constraints for the scrutinee
            generate_expression_constraints(ctx, scrutinee);
            ctx.register_type(&scrutinee.ty);

            // Generate constraints for each arm
            for arm in arms {
                // Generate constraints for the pattern
                generate_pattern_constraints(ctx, &arm.pattern);
                ctx.register_type(&arm.pattern.ty);

                // Pattern type must match scrutinee type
                ctx.equate(
                    arm.pattern.ty.id(),
                    scrutinee.ty.id(),
                    arm.pattern.span.clone(),
                );

                // Generate constraints for the guard if present
                if let Some(guard) = &arm.guard {
                    generate_expression_constraints(ctx, guard);
                    ctx.register_type(&guard.ty);
                    // Guard must be Bool
                    let bool_ty = Ty::bool(guard.span.clone());
                    ctx.register_type(&bool_ty);
                    ctx.equate(guard.ty.id(), bool_ty.id(), guard.span.clone());
                }

                // Generate constraints for the body
                generate_expression_constraints(ctx, &arm.body);
                ctx.register_type(&arm.body.ty);

                // Body type contributes to match expression type
                // All arms should have compatible types
                ctx.equate(expr.ty.id(), arm.body.ty.id(), arm.body.span.clone());
            }
        }

        ExprKind::Block { statements, value } => {
            // Generate constraints for statements
            for stmt in statements {
                generate_statement_constraints(ctx, stmt);
            }
            // Generate constraints for the value expression if present
            if let Some(val) = value {
                generate_expression_constraints(ctx, val);
                ctx.register_type(&val.ty);
                // Block type equals value type
                ctx.equate(expr.ty.id(), val.ty.id(), val.span.clone());
            }
            // If no value, block type should be unit (already set in AST)
        }

        ExprKind::Error => {}
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require setting up a full semantic model
}
