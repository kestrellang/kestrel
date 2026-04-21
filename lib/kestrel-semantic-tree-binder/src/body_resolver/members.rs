//! Member access resolution.
//!
//! This module handles resolving member access expressions (field access, method calls)
//! including visibility checking, member chain resolution, and constraint enforcement
//! for type parameters.

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::{ExtensionsFor, IsVisibleFrom, SymbolFor};
use kestrel_semantic_tree::behavior::ComputedMemberAccessBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression, PrimitiveMethod};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::symbol::protocol::FlattenedProtocolBehavior;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::Substitutions;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_semantic_type_inference::TypeOracle;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{
    AmbiguousConstrainedMethodError, CannotAccessMemberOnTypeError,
    DelegatingInitOutsideInitializerError, MemberNotVisibleError, MethodNotInBoundsError,
    NoSuchMemberError, NoSuchMethodError, UnconstrainedAssociatedTypeMemberError,
    UnconstrainedTypeParameterMemberError,
};

use super::calls::{try_resolve_subscript_call, validate_argument_access_modes};
use super::context::BodyResolutionContext;
use super::utils::{
    get_associated_type_bounds_from_context, get_callable_behavior, get_type_container,
    get_type_parameter_bounds_by_id, get_type_parameter_bounds_from_context, matches_signature,
    resolve_associated_types,
};

/// Resolve a chain of member accesses: obj.field1.field2.field3
pub fn resolve_member_chain(
    base: Expression,
    members: &[(String, Span)],
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let mut current = base;

    for (member_name, member_span) in members {
        current = resolve_member_access(current, member_name, member_span.clone(), ctx);
    }

    current
}

/// Resolve a single member access: base.member
///
/// This function:
/// 1. Checks for primitive methods on primitive types
/// 2. Gets the container type from the base expression
/// 3. Finds a child symbol with the given name
/// 4. Checks visibility
/// 5. Uses MemberAccessBehavior to produce the result expression
pub fn resolve_member_access(
    mut base: Expression,
    member_name: &str,
    member_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Resolve SelfType in the base expression before member access.
    // This ensures that when type inference processes the member access,
    // it has a concrete type to work with instead of SelfType.
    base.ty = resolve_self_type_to_concrete(&base.ty, ctx);

    let base_span = base.span.clone();
    let full_span = Span::new(base_span.file_id, base_span.start..member_span.end);

    // For Grouping expressions like `(x).format()`, we need to look through the
    // grouping to get the actual inner expression's type. Otherwise, the grouping's
    // Infer type prevents us from finding methods (like primitive methods) that
    // are only available on concrete types.
    let base_ty = if let ExprKind::Grouping(inner) = &base.kind {
        &inner.ty
    } else {
        &base.ty
    };

    // 0. Check if base is a TypeParameterRef (for static method access like T.create())
    if let ExprKind::TypeParameterRef(symbol_id) = &base.kind {
        return resolve_type_parameter_static_member(
            *symbol_id,
            member_name,
            member_span,
            full_span,
            ctx,
        );
    }

    // 0.5. Check if base is an AssociatedTypeRef (for chained access like T.Next.Next.method())
    if let ExprKind::AssociatedTypeRef = &base.kind {
        return resolve_associated_type_static_member(
            &base,
            member_name,
            member_span,
            full_span,
            ctx,
        );
    }

    // 1. Check for primitive method (e.g., 5.toString, "hello".length)
    if let Some(primitive_method) = PrimitiveMethod::lookup(base_ty, member_name) {
        return Expression::primitive_method_ref(base, primitive_method, full_span);
    }

    // 2. Propagate error types and Never without cascading diagnostics.
    if matches!(base_ty.kind(), TyKind::Error | TyKind::Never) {
        return Expression::error(full_span);
    }

    // 3. If base type is Infer, defer to type inference.
    if matches!(base_ty.kind(), TyKind::Infer) {
        return Expression::deferred_member_access(
            base,
            member_name.to_string(),
            true, // optimistic: validated post-inference
            Ty::infer(member_span),
            full_span,
        );
    }

    // 4. Handle type parameter instance access — defer to inference.
    // The oracle handles protocol bound resolution during solving.
    if matches!(base_ty.kind(), TyKind::TypeParameter(_)) {
        // Clone the type to break the borrow on `base`
        let mut effective_ty = base_ty.clone();
        // Try to normalize using equality constraints from the where clause
        if let TyKind::TypeParameter(type_param) = effective_ty.kind() {
            if let Some(normalized) =
                normalize_type_param_with_equality(type_param.metadata().id(), ctx.where_clause())
            {
                effective_ty = normalized.clone();
                base.ty = normalized.clone();
            }
        }
        // Check if the member exists in protocol bounds — emit error if definitely not found
        let resolved_base_ty = resolve_self_type_to_concrete(&effective_ty, ctx);
        match ctx
            .model
            .resolve_member(&resolved_base_ty, member_name, false)
        {
            Ok(resolution) => {
                let field_mutable = member_field_mutable(&resolution, ctx);
                return Expression::deferred_member_access(
                    base,
                    member_name.to_string(),
                    field_mutable,
                    resolution.ty,
                    full_span,
                );
            },
            Err(kestrel_semantic_type_inference::MemberError::UnknownType) => {
                return Expression::deferred_member_access(
                    base,
                    member_name.to_string(),
                    true,
                    Ty::infer(member_span),
                    full_span,
                );
            },
            Err(_) => {
                // Member not found in any bound - emit error
                if let TyKind::TypeParameter(type_param) = effective_ty.kind() {
                    let type_param_name = type_param.metadata().name().value.clone();
                    let bounds = get_type_parameter_bounds_from_context(type_param, ctx);
                    let bound_names: Vec<String> = bounds
                        .iter()
                        .filter_map(|b| {
                            if let TyKind::Protocol { symbol: proto, .. } = b.kind() {
                                Some(proto.metadata().name().value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    if bounds.is_empty() {
                        let error = UnconstrainedTypeParameterMemberError {
                            span: full_span.clone(),
                            member_name: member_name.to_string(),
                            type_param_name,
                        };
                        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    } else {
                        let error = MethodNotInBoundsError {
                            call_span: full_span.clone(),
                            method_name: member_name.to_string(),
                            type_param_name,
                            bound_names,
                        };
                        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    }
                }
                return Expression::error(full_span);
            },
        }
    }

    // 4.5. Handle associated type access — defer to inference with error checking.
    if let TyKind::AssociatedType { symbol, .. } = base_ty.kind() {
        let assoc_type_name = symbol.metadata().name().value.clone();
        let container = if let TyKind::AssociatedType { container, .. } = base_ty.kind() {
            container.as_ref().map(|c| c.as_ref())
        } else {
            None
        };
        let bounds = get_associated_type_bounds_from_context(symbol, container, ctx);

        if bounds.is_empty() {
            let error = UnconstrainedAssociatedTypeMemberError {
                span: full_span.clone(),
                member_name: member_name.to_string(),
                assoc_type_name,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(full_span);
        }

        match ctx.model.resolve_member(base_ty, member_name, false) {
            Ok(resolution) => {
                let field_mutable = member_field_mutable(&resolution, ctx);
                return Expression::deferred_member_access(
                    base,
                    member_name.to_string(),
                    field_mutable,
                    resolution.ty,
                    full_span,
                );
            },
            Err(kestrel_semantic_type_inference::MemberError::UnknownType) => {
                return Expression::deferred_member_access(
                    base,
                    member_name.to_string(),
                    true,
                    Ty::infer(member_span),
                    full_span,
                );
            },
            Err(_) => {
                // Try collecting methods directly from bounds as fallback
                let mut found = false;
                for bound in &bounds {
                    if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
                        if let Some(flattened) =
                            proto.metadata().get_behavior::<FlattenedProtocolBehavior>()
                        {
                            if flattened.methods().get(member_name).is_some()
                                || flattened.properties().get(member_name).is_some()
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if found {
                    return Expression::deferred_member_access(
                        base,
                        member_name.to_string(),
                        true,
                        Ty::infer(member_span),
                        full_span,
                    );
                }
                let bound_names: Vec<String> = bounds
                    .iter()
                    .filter_map(|b| {
                        if let TyKind::Protocol { symbol: proto, .. } = b.kind() {
                            Some(proto.metadata().name().value.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let error = MethodNotInBoundsError {
                    call_span: full_span.clone(),
                    method_name: member_name.to_string(),
                    type_param_name: assoc_type_name,
                    bound_names,
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                return Expression::error(full_span);
            },
        }
    }

    // 4.7. Handle SelfType (protocol extensions) — defer to inference.
    // SelfType in protocol extensions is not resolved to a concrete type,
    // so the oracle can't handle it. Try to get a best-effort type from the
    // protocol container to avoid cascading errors.
    if matches!(base_ty.kind(), TyKind::SelfType) {
        let best_effort_ty = get_type_container(base_ty, ctx)
            .and_then(|container| {
                let member = container
                    .metadata()
                    .children()
                    .into_iter()
                    .find(|c| c.metadata().name().value == member_name)?;
                // Try to get the return type from behaviors
                for behavior in member.metadata().behaviors() {
                    if behavior.kind() == KestrelBehaviorKind::MemberAccess {
                        if let Some(access) =
                            behavior.as_ref().downcast_ref::<MemberAccessBehavior>()
                        {
                            return Some(access.member_type().clone());
                        }
                    }
                    if behavior.kind() == KestrelBehaviorKind::ComputedMemberAccess {
                        if let Some(access) = behavior
                            .as_ref()
                            .downcast_ref::<ComputedMemberAccessBehavior>()
                        {
                            return Some(access.member_type().clone());
                        }
                    }
                    if behavior.kind() == KestrelBehaviorKind::Callable {
                        if let Some(callable) = behavior.as_ref().downcast_ref::<CallableBehavior>()
                        {
                            return Some(callable.return_type().clone());
                        }
                    }
                }
                None
            })
            .unwrap_or_else(|| Ty::infer(member_span.clone()));
        return Expression::deferred_member_access(
            base,
            member_name.to_string(),
            true, // optimistic: validated post-inference
            best_effort_ty,
            full_span,
        );
    }

    // 5. For concrete types, try the oracle. If the member exists, defer.
    // If not found, emit a binder-level error for better diagnostics.
    let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);
    let oracle_result = ctx
        .model
        .resolve_member(&resolved_base_ty, member_name, false);
    match oracle_result {
        Ok(resolution) => {
            // Check visibility before deferring — the oracle doesn't filter by visibility
            if !ctx.model.query(IsVisibleFrom {
                target: resolution.symbol_id,
                context: ctx.function_id,
            }) {
                use kestrel_semantic_tree::behavior::visibility::Visibility;

                let member_sym = ctx.model.query(SymbolFor {
                    id: resolution.symbol_id,
                });
                let visibility = member_sym
                    .as_ref()
                    .and_then(|s| s.metadata().get_behavior::<VisibilityBehavior>())
                    .and_then(|v| v.visibility().cloned())
                    .unwrap_or(Visibility::Internal);

                let error = MemberNotVisibleError {
                    member_span,
                    member_name: member_name.to_string(),
                    base_span,
                    base_type: base_ty.to_string(),
                    visibility: visibility.to_string(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                return Expression::error(full_span);
            }

            let field_mutable = member_field_mutable(&resolution, ctx);
            Expression::deferred_member_access(
                base,
                member_name.to_string(),
                field_mutable,
                resolution.ty,
                full_span,
            )
        },
        Err(kestrel_semantic_type_inference::MemberError::UnknownType) => {
            Expression::deferred_member_access(
                base,
                member_name.to_string(),
                true,
                Ty::infer(member_span),
                full_span,
            )
        },
        Err(_) => {
            // Check if the type supports member access at all
            if get_type_container(base_ty, ctx).is_none() {
                let error = CannotAccessMemberOnTypeError {
                    span: full_span.clone(),
                    base_type: base_ty.to_string(),
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                return Expression::error(full_span);
            }

            // Check visibility: member might exist but be private
            let container = get_type_container(base_ty, ctx).unwrap();
            let member = container
                .metadata()
                .children()
                .into_iter()
                .find(|c| c.metadata().name().value == member_name);

            if let Some(member) = member {
                let member_id = member.metadata().id();
                if !ctx.model.query(IsVisibleFrom {
                    target: member_id,
                    context: ctx.function_id,
                }) {
                    use kestrel_semantic_tree::behavior::visibility::Visibility;

                    let visibility = member
                        .metadata()
                        .get_behavior::<VisibilityBehavior>()
                        .and_then(|v| v.visibility().cloned())
                        .unwrap_or(Visibility::Internal);

                    let error = MemberNotVisibleError {
                        member_span,
                        member_name: member_name.to_string(),
                        base_span,
                        base_type: base_ty.to_string(),
                        visibility: visibility.to_string(),
                    };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return Expression::error(full_span);
                }

                // Member exists and is visible — the oracle didn't find it
                // (possibly due to visibility context differences). Defer to inference.
                return Expression::deferred_member_access(
                    base,
                    member_name.to_string(),
                    true,
                    Ty::infer(member_span),
                    full_span,
                );
            }

            // Member definitively not found
            let error = NoSuchMemberError {
                member_span,
                member_name: member_name.to_string(),
                base_span,
                base_type: base_ty.to_string(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            Expression::error(full_span)
        },
    }
}

/// Check if a resolved member is a mutable field.
/// Returns true for non-field members (methods, computed props) since they don't
/// participate in assignment mutability the same way.
fn member_field_mutable(
    resolution: &kestrel_semantic_type_inference::MemberResolution,
    ctx: &BodyResolutionContext,
) -> bool {
    ctx.model
        .query(SymbolFor {
            id: resolution.symbol_id,
        })
        .and_then(|s| s.metadata().get_behavior::<MemberAccessBehavior>())
        .map(|b| b.is_mutable())
        .unwrap_or(true) // non-field members default to true
}

/// Try to resolve a field access without emitting errors.
///
/// Returns `Some(Expression)` if the field exists and is accessible, `None` otherwise.
/// This is used for fallback resolution when trying field+subscript after method resolution fails.
fn try_resolve_field_access(
    base: &Expression,
    field_name: &str,
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Option<Expression> {
    let base_ty = &base.ty;

    // Skip primitive types, type parameters, infer types, and error types
    if matches!(
        base_ty.kind(),
        TyKind::TypeParameter(_) | TyKind::Infer | TyKind::Error
    ) {
        return None;
    }

    // Get container from base type
    let container = get_type_container(base_ty, ctx)?;

    // Find child with that name in direct children
    let member = container
        .metadata()
        .children()
        .into_iter()
        .find(|c| c.metadata().name().value == field_name);

    // If not found in direct children, search extensions
    let member = match member {
        Some(m) => m,
        None => {
            let container_id = container.metadata().id();
            let extensions = ctx.model.query(ExtensionsFor {
                target_id: container_id,
            });
            let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);
            let applicable_extensions =
                filter_applicable_extensions(extensions, &resolved_base_ty, ctx);

            applicable_extensions
                .iter()
                .flat_map(|ext| ext.metadata().children())
                .find(|child| child.metadata().name().value == field_name)?
        },
    };

    // Check visibility (silently fail if not visible)
    let member_id = member.metadata().id();
    if !ctx.model.query(IsVisibleFrom {
        target: member_id,
        context: ctx.function_id,
    }) {
        return None;
    }

    // Check for MemberAccessBehavior (field) or ComputedMemberAccessBehavior (computed property)
    for behavior in member.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::MemberAccess
            && let Some(access) = behavior.as_ref().downcast_ref::<MemberAccessBehavior>()
        {
            let mut result = access.access(base.clone(), span.clone());
            let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);

            if let Some((_, substitutions)) = resolved_base_ty.as_struct_with_subs() {
                result.ty = result.ty.apply_substitutions(substitutions);
            }
            return Some(result);
        }
        if behavior.kind() == KestrelBehaviorKind::ComputedMemberAccess
            && let Some(access) = behavior
                .as_ref()
                .downcast_ref::<ComputedMemberAccessBehavior>()
        {
            let mut result = access.access(base.clone(), span.clone());
            let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);

            if let Some((_, substitutions)) = resolved_base_ty.as_struct_with_subs() {
                result.ty = result.ty.apply_substitutions(substitutions);
            }
            return Some(result);
        }
    }

    // Member exists but doesn't have field access behavior (e.g., it's a function)
    None
}

/// Resolve a member call from a FieldAccess expression: obj.method(args)
///
/// This handles:
/// - Primitive methods (e.g., Int.add, String.length)
/// - Struct/Protocol methods directly
/// - Constrained type parameter methods (via protocol bounds)
pub fn resolve_member_call(
    object: &Expression,
    member_name: &str,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    explicit_type_args: Option<Vec<kestrel_semantic_tree::ty::Ty>>,
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let base_ty = &object.ty;

    // Check for delegating initializer: self.init(...)
    if member_name == "init"
        && let ExprKind::LocalRef(local_id) = &object.kind
    {
        // Check if this is the "self" local (local ID 0 in initializers)
        if *local_id == LocalId(0) {
            // Check if we're inside an initializer
            if let Some(symbol) = ctx.model.query(SymbolFor {
                id: ctx.function_id,
            }) && symbol.metadata().kind() == KestrelSymbolKind::Initializer
            {
                return resolve_delegating_init(&symbol, arguments, arg_labels, span, ctx);
            }

            // Not in an initializer - error
            ctx.diagnostics.add_diagnostic(
                DelegatingInitOutsideInitializerError { span: span.clone() }.into_diagnostic(),
            );
            return Expression::error(span);
        }
    }

    // First check for primitive method
    if let Some(primitive_method) = PrimitiveMethod::lookup(base_ty, member_name) {
        return Expression::primitive_method_call(
            object.clone(),
            primitive_method,
            arguments,
            span,
        );
    }

    // If base type is Error or Never, propagate without cascading diagnostics.
    if matches!(base_ty.kind(), TyKind::Error | TyKind::Never) {
        return Expression::error(span);
    }

    // Try field + subscript fallback (obj.field(index) pattern).
    // The oracle doesn't handle this, so check before deferring.
    if let Some(field_expr) = try_resolve_field_access(object, member_name, span.clone(), ctx) {
        // Try subscript call on the field
        if let Some(subscript_expr) =
            try_resolve_subscript_call(&field_expr, &arguments, arg_labels, &span, ctx)
        {
            return subscript_expr;
        }

        // Check if the field has a callable type (first-class function)
        let field_ty = field_expr.ty.clone();
        match field_ty.kind() {
            TyKind::Function {
                params,
                return_type,
            } => {
                if arguments.len() != params.len() {
                    ctx.diagnostics.add_diagnostic(
                        crate::diagnostics::ClosureArityError {
                            span: span.clone(),
                            expected: params.len(),
                            provided: arguments.len(),
                        }
                        .into_diagnostic(),
                    );
                    return Expression::error(span);
                }
                return Expression::call(field_expr, arguments, (**return_type).clone(), span);
            },
            TyKind::UnresolvedFunction { return_type, .. } => {
                return Expression::call(field_expr, arguments, (**return_type).clone(), span);
            },
            _ => {},
        }
    }

    // Defer method resolution to type inference.
    // Use the TypeOracle to get a best-effort return type so downstream
    // bind-time checks (subscript calls, field access) don't cascade errors.
    let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);
    let arg_labels: Vec<Option<String>> = arguments.iter().map(|a| a.label.clone()).collect();
    let deferred_return_ty = if !matches!(resolved_base_ty.kind(), TyKind::Infer | TyKind::Error) {
        ctx.model
            .resolve_member_with_labels(&resolved_base_ty, member_name, false, &arg_labels)
            .map(|resolution| {
                let mut ty = resolution.ty;
                // Apply receiver substitutions to the return type
                ty = ty.substitute_self(&resolved_base_ty);
                ty = resolve_associated_types(&ty, ctx);
                let expanded = resolved_base_ty.expand_aliases();
                if let TyKind::Struct { substitutions, .. } | TyKind::Enum { substitutions, .. } =
                    expanded.kind()
                {
                    if !substitutions.is_empty() {
                        ty = ty.apply_substitutions(substitutions);
                    }
                }
                ty
            })
            .unwrap_or_else(|_| {
                // Fallback for SelfType in protocol extensions: look up the method
                // directly from the protocol container to get a best-effort return type.
                best_effort_return_type_from_container(base_ty, member_name, ctx)
                    .unwrap_or_else(|| Ty::infer(span.clone()))
            })
    } else {
        Ty::infer(span.clone())
    };
    Expression::deferred_method_call(
        object.clone(),
        member_name.to_string(),
        arguments,
        explicit_type_args,
        deferred_return_ty,
        span,
    )
}

/// Look up a method in the type container to get a best-effort return type.
/// Used when the oracle can't resolve the member (e.g., SelfType in protocol extensions).
fn best_effort_return_type_from_container(
    base_ty: &Ty,
    member_name: &str,
    ctx: &BodyResolutionContext,
) -> Option<Ty> {
    let container = get_type_container(base_ty, ctx)?;
    let member = container
        .metadata()
        .children()
        .into_iter()
        .find(|c| c.metadata().name().value == member_name)?;
    for behavior in member.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::Callable {
            if let Some(callable) = behavior.as_ref().downcast_ref::<CallableBehavior>() {
                return Some(callable.return_type().clone());
            }
        }
    }
    None
}

/// Resolve static member access on a type parameter: `T.staticMethod`.
///
/// This looks up static methods, static properties, and associated types
/// from the type parameter's protocol bounds.
fn resolve_type_parameter_static_member(
    symbol_id: SymbolId,
    member_name: &str,
    _member_span: Span,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Get the type parameter symbol
    let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) else {
        return Expression::error(full_span);
    };

    // Verify it's a type parameter and get Arc<TypeParameterSymbol>
    let type_param_arc = match symbol.clone().downcast_arc::<TypeParameterSymbol>() {
        Ok(arc) => arc,
        Err(_) => return Expression::error(full_span),
    };

    let type_param_name = type_param_arc.metadata().name().value.clone();

    // Get protocol bounds for this type parameter
    let bounds = get_type_parameter_bounds_by_id(symbol_id, ctx);

    if bounds.is_empty() {
        let error = UnconstrainedTypeParameterMemberError {
            span: full_span.clone(),
            member_name: member_name.to_string(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    let type_param_ty = Ty::type_parameter(type_param_arc.clone(), full_span.clone());

    // First, check if member_name is an associated type in any protocol bound.
    // Associated type access is fundamentally different from member access -
    // the oracle has resolve_associated_type() for this, not resolve_member().
    if let Some(assoc_type_expr) =
        find_associated_type_in_bounds(&bounds, member_name, &type_param_ty, full_span.clone(), ctx)
    {
        return assoc_type_expr;
    }

    // Use the oracle for static member resolution (properties and methods)
    match ctx.model.resolve_member(&type_param_ty, member_name, true) {
        Ok(resolution) => {
            // Check if it's a callable (method) vs property
            if let Some(sym) = ctx.model.query(SymbolFor {
                id: resolution.symbol_id,
            }) && sym.metadata().kind() == KestrelSymbolKind::Function
            {
                // Static method - collect overloads and check ambiguity
                let (method_ids, source_protocols) = collect_static_method_candidates_from_bounds(
                    &bounds,
                    member_name,
                    resolution.symbol_id,
                );

                // Check for ambiguity
                if source_protocols.len() > 1 {
                    let protocol_names: Vec<String> = source_protocols.into_iter().collect();
                    let error = AmbiguousConstrainedMethodError {
                        call_span: full_span.clone(),
                        method_name: member_name.to_string(),
                        protocol_names: protocol_names.clone(),
                        definition_spans: protocol_names
                            .iter()
                            .map(|n| (n.clone(), full_span.clone()))
                            .collect(),
                    };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return Expression::error(full_span);
                }

                return Expression::method_ref(
                    Expression::type_parameter_ref(
                        symbol_id,
                        type_param_ty.clone(),
                        Span::new(full_span.file_id, full_span.start..full_span.start),
                    ),
                    method_ids,
                    member_name.to_string(),
                    full_span,
                );
            }
            // Static property - requires protocol_id and has_setter
            if let Some(protocol_id) = resolution.protocol_id
                && let Some(has_setter) = resolution.has_setter
            {
                let receiver = Expression::type_parameter_ref(
                    symbol_id,
                    type_param_ty.clone(),
                    Span::new(full_span.file_id, full_span.start..full_span.start),
                );
                return Expression::protocol_property_access(
                    receiver,
                    resolution.symbol_id,
                    member_name.to_string(),
                    protocol_id,
                    true,
                    has_setter,
                    resolution.ty,
                    full_span,
                );
            }
            // Fallback for non-protocol resolution
            return Expression::method_ref(
                Expression::type_parameter_ref(
                    symbol_id,
                    type_param_ty.clone(),
                    Span::new(full_span.file_id, full_span.start..full_span.start),
                ),
                vec![resolution.symbol_id],
                member_name.to_string(),
                full_span,
            );
        },
        Err(_) => {
            let bound_names: Vec<String> = bounds
                .iter()
                .filter_map(|b| {
                    if let TyKind::Protocol { symbol: proto, .. } = b.kind() {
                        Some(proto.metadata().name().value.clone())
                    } else {
                        None
                    }
                })
                .collect();
            let error = MethodNotInBoundsError {
                call_span: full_span.clone(),
                method_name: member_name.to_string(),
                type_param_name,
                bound_names,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(full_span);
        },
    }
}

/// Find an associated type with the given name in the protocol bounds.
///
/// Returns an `AssociatedTypeRef` expression if found, or `None` if no matching
/// associated type exists in any of the bounds.
fn find_associated_type_in_bounds(
    bounds: &[Ty],
    member_name: &str,
    container_ty: &Ty,
    span: Span,
    ctx: &BodyResolutionContext,
) -> Option<Expression> {
    for bound in bounds {
        if let TyKind::Protocol {
            symbol: protocol, ..
        } = bound.kind()
        {
            // Check direct children of protocol for associated types
            let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == member_name
                {
                    // Found an associated type - create a qualified associated type
                    if let Some(symbol) = ctx.model.query(SymbolFor {
                        id: child.metadata().id(),
                    }) && let Ok(assoc_type_arc) =
                        symbol.into_any_arc().downcast::<AssociatedTypeSymbol>()
                    {
                        let qualified_ty = Ty::qualified_associated_type(
                            assoc_type_arc,
                            container_ty.clone(),
                            span.clone(),
                        );
                        return Some(Expression::associated_type_ref(qualified_ty, span));
                    }
                }
            }

            // Check inherited protocols (via FlattenedProtocolBehavior)
            if let Some(flattened) = protocol
                .metadata()
                .get_behavior::<FlattenedProtocolBehavior>()
                && let Some(flattened_assoc) = flattened.associated_types().get(member_name)
            {
                let qualified_ty = Ty::qualified_associated_type(
                    flattened_assoc.symbol.clone(),
                    container_ty.clone(),
                    span.clone(),
                );
                return Some(Expression::associated_type_ref(qualified_ty, span));
            }
        }
    }
    None
}

/// Resolve static member access on an associated type expression.
///
/// This handles chained associated type access like `T.Next.Next.baseValue()`.
/// When the base is an `AssociatedTypeRef`, we look at the associated type's bounds
/// to find either another associated type or a static method/property.
fn resolve_associated_type_static_member(
    base: &Expression,
    member_name: &str,
    _member_span: Span,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let TyKind::AssociatedType {
        symbol: assoc_type,
        container,
    } = base.ty.kind()
    else {
        return Expression::error(full_span);
    };

    let container_ref = container.as_ref().map(|c| c.as_ref());
    let bounds = get_associated_type_bounds_from_context(assoc_type, container_ref, ctx);

    if bounds.is_empty() {
        let error = UnconstrainedTypeParameterMemberError {
            span: full_span.clone(),
            member_name: member_name.to_string(),
            type_param_name: assoc_type.metadata().name().value.clone(),
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // The container type for nested lookups is the qualified associated type itself
    let container_ty = match container {
        Some(c) => {
            Ty::qualified_associated_type(assoc_type.clone(), (**c).clone(), full_span.clone())
        },
        None => base.ty.clone(),
    };

    // First, check if member_name is an associated type in any protocol bound
    if let Some(assoc_type_expr) =
        find_associated_type_in_bounds(&bounds, member_name, &container_ty, full_span.clone(), ctx)
    {
        return assoc_type_expr;
    }

    // Use the oracle for static member resolution (properties and methods).
    // Fall back to binder-side bound search when the oracle fails (it lacks context).
    match ctx.model.resolve_member(&base.ty, member_name, true) {
        Ok(resolution) => {
            if let Some(protocol_id) = resolution.protocol_id
                && let Some(has_setter) = resolution.has_setter
            {
                if let Some(sym) = ctx.model.query(SymbolFor {
                    id: resolution.symbol_id,
                }) && sym.metadata().kind() == KestrelSymbolKind::Function
                {
                    // Static method - collect overloads and check ambiguity
                    let (method_ids, source_protocols) =
                        collect_static_method_candidates_from_bounds(
                            &bounds,
                            member_name,
                            resolution.symbol_id,
                        );

                    if source_protocols.len() > 1 {
                        let protocol_names: Vec<String> = source_protocols.into_iter().collect();
                        let error = AmbiguousConstrainedMethodError {
                            call_span: full_span.clone(),
                            method_name: member_name.to_string(),
                            protocol_names: protocol_names.clone(),
                            definition_spans: protocol_names
                                .iter()
                                .map(|n| (n.clone(), full_span.clone()))
                                .collect(),
                        };
                        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                        return Expression::error(full_span);
                    }

                    return Expression::method_ref(
                        base.clone(),
                        method_ids,
                        member_name.to_string(),
                        full_span,
                    );
                }
                // Static property
                return Expression::protocol_property_access(
                    base.clone(),
                    resolution.symbol_id,
                    member_name.to_string(),
                    protocol_id,
                    true,
                    has_setter,
                    resolution.ty,
                    full_span,
                );
            }
            return Expression::method_ref(
                base.clone(),
                vec![resolution.symbol_id],
                member_name.to_string(),
                full_span,
            );
        },
        Err(_) => {
            // Oracle failed (likely missing context for where-clause bounds) -
            // fall back to binder-side bound search for static methods/properties
            let mut method_ids: Vec<SymbolId> = Vec::new();
            let mut source_protocols: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut bound_names: Vec<String> = Vec::new();

            for bound in &bounds {
                if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
                    bound_names.push(proto.metadata().name().value.clone());
                    if let Some(flattened) =
                        proto.metadata().get_behavior::<FlattenedProtocolBehavior>()
                    {
                        if let Some(methods) = flattened.methods().get(member_name) {
                            for method in methods {
                                if let Some(callable) = get_callable_behavior(&method.symbol)
                                    && callable.is_static()
                                {
                                    method_ids.push(method.symbol.metadata().id());
                                    source_protocols.insert(method.source_protocol_name.clone());
                                }
                            }
                        }
                        // Check for static properties
                        if let Some(prop) = flattened.properties().get(member_name)
                            && prop.is_static
                        {
                            let prop_ty = prop
                                .symbol
                                .metadata()
                                .get_behavior::<TypedBehavior>()
                                .map(|tb| tb.ty().substitute_self(&container_ty))
                                .unwrap_or_else(|| Ty::error(full_span.clone()));

                            return Expression::protocol_property_access(
                                base.clone(),
                                prop.symbol.metadata().id(),
                                member_name.to_string(),
                                proto.metadata().id(),
                                true,
                                prop.has_setter,
                                prop_ty,
                                full_span,
                            );
                        }
                    }
                }
            }

            if !method_ids.is_empty() {
                if source_protocols.len() > 1 {
                    let protocol_names: Vec<String> = source_protocols.into_iter().collect();
                    let error = AmbiguousConstrainedMethodError {
                        call_span: full_span.clone(),
                        method_name: member_name.to_string(),
                        protocol_names: protocol_names.clone(),
                        definition_spans: protocol_names
                            .iter()
                            .map(|n| (n.clone(), full_span.clone()))
                            .collect(),
                    };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return Expression::error(full_span);
                }
                return Expression::method_ref(
                    base.clone(),
                    method_ids,
                    member_name.to_string(),
                    full_span,
                );
            }

            let error = MethodNotInBoundsError {
                call_span: full_span.clone(),
                method_name: member_name.to_string(),
                type_param_name: assoc_type.metadata().name().value.clone(),
                bound_names,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(full_span);
        },
    }
}

/// Filter extensions to find those applicable to the given type instance.
///
/// Returns extensions sorted by specificity (most specific first).
/// An extension is applicable if:
/// 1. Its type arguments can be unified with the actual type's arguments
/// 2. Any where clause constraints are satisfied
///
/// Collect all static method candidate SymbolIds with the given name from protocol bounds.
///
/// Also tracks which protocols each method comes from for ambiguity detection.
/// Returns (candidates, source_protocol_names).
fn collect_static_method_candidates_from_bounds(
    bounds: &[Ty],
    method_name: &str,
    fallback_id: SymbolId,
) -> (Vec<SymbolId>, std::collections::HashSet<String>) {
    let mut candidates = Vec::new();
    let mut source_protocols = std::collections::HashSet::new();
    for bound in bounds {
        if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
            if let Some(flattened) = proto.metadata().get_behavior::<FlattenedProtocolBehavior>() {
                if let Some(methods) = flattened.methods().get(method_name) {
                    for method in methods {
                        if let Some(callable) = get_callable_behavior(&method.symbol)
                            && callable.is_static()
                        {
                            candidates.push(method.symbol.metadata().id());
                            source_protocols.insert(method.source_protocol_name.clone());
                        }
                    }
                }
            }
        }
    }
    if candidates.is_empty() {
        candidates.push(fallback_id);
    }
    (candidates, source_protocols)
}

/// For example:
/// - `extend Box[T]` applies to `Box[Int]`, `Box[String]`, etc.
/// - `extend Box[Int]` only applies to `Box[Int]`
/// - `extend Pair[T, Int]` applies to `Pair[String, Int]` but not `Pair[String, Bool]`
/// - `extend Box[T] where T: Equatable` only applies to `Box[String]` if String: Equatable
pub(super) fn filter_applicable_extensions(
    extensions: Vec<Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>>,
    actual_ty: &Ty,
    ctx: &BodyResolutionContext,
) -> Vec<Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>> {
    use super::utils::type_satisfies_bound;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;

    // Handle Protocol types specially - return all protocol extensions without filtering
    // This is used when resolving method calls on `self` inside protocol extension bodies
    if matches!(actual_ty.kind(), TyKind::Protocol { .. }) {
        // For protocol extensions, all extensions are applicable (they may have where clauses
        // but those are checked at call site, not when inside the extension body itself)
        return extensions;
    }

    // Handle SelfType inside protocol extensions - also return all extensions
    // This allows calling other extension methods from within a protocol extension
    if matches!(actual_ty.kind(), TyKind::SelfType) {
        return extensions;
    }

    // Get substitutions from actual type (struct or enum)
    let actual_subs = if let Some((_, subs)) = actual_ty.as_struct_with_subs() {
        subs
    } else if let Some((_, subs)) = actual_ty.as_enum_with_subs() {
        subs
    } else {
        // Not a struct or enum type - no extensions apply
        return Vec::new();
    };

    // Filter extensions by applicability
    let mut applicable: Vec<_> = extensions
        .into_iter()
        .filter_map(|ext| {
            // Get the extension's target type
            let behaviors = ext.metadata().behaviors();
            let target_behavior = behaviors
                .iter()
                .find(|b| b.kind() == KestrelBehaviorKind::ExtensionTarget)?;

            let target_behavior = target_behavior
                .as_ref()
                .downcast_ref::<ExtensionTargetBehavior>()?;

            let target_ty = target_behavior.target_type();

            // Extract substitutions from extension's target type (struct or enum)
            let extension_subs = if let Some((_, subs)) = target_ty.as_struct_with_subs() {
                subs
            } else if let Some((_, subs)) = target_ty.as_enum_with_subs() {
                subs
            } else {
                return None;
            };

            // Check if this extension's type arguments are applicable to the actual type
            // by comparing types at each type parameter position
            if !is_extension_applicable(extension_subs, actual_subs) {
                return None;
            }

            // Build substitutions: map extension's type params to actual types
            // by iterating over extension_subs and looking up corresponding actual types
            let mut param_to_actual = std::collections::HashMap::new();
            for (param_id, ext_ty) in extension_subs.iter() {
                if let TyKind::TypeParameter(_) = ext_ty.kind() {
                    // This position has a type parameter - get the actual type
                    if let Some(actual_ty) = actual_subs.get(*param_id) {
                        param_to_actual.insert(*param_id, actual_ty);
                    }
                }
            }

            // Check where clause constraints are satisfied
            let ext_where_clause = target_behavior.where_clause();
            for constraint in ext_where_clause.constraints() {
                if let Some(param_id) = constraint.type_parameter_id() {
                    // Get the actual type for this parameter
                    if let Some(actual_type) = param_to_actual.get(&param_id) {
                        // Check each bound is satisfied
                        for bound in constraint.bounds() {
                            // First check if type actually satisfies bound
                            if type_satisfies_bound(actual_type, bound, ctx.model) {
                                continue;
                            }
                            // If actual_type is a type parameter, check if the bound is
                            // declared in the current context's where clause
                            if let TyKind::TypeParameter(tp_symbol) = actual_type.kind()
                                && type_param_has_bound_in_where_clause(
                                    tp_symbol.metadata().id(),
                                    bound,
                                    ctx.where_clause(),
                                )
                            {
                                continue;
                            }
                            // Constraint not satisfied
                            return None;
                        }
                    }
                    // If param_id not in map, it's a constraint on a type param that's not in scope
                    // This shouldn't happen with valid extensions
                }
            }

            // Count how many concrete (non-type-parameter) arguments the extension has
            // This is the "specificity" - more concrete args = more specific
            let specificity = extension_subs
                .types()
                .filter(|ty| !ty.is_type_parameter())
                .count();
            Some((ext, specificity))
        })
        .collect();

    // Sort by specificity (most specific first)
    applicable.sort_by_key(|(_, specificity)| std::cmp::Reverse(*specificity));

    // Return just the extensions, without specificity scores
    applicable.into_iter().map(|(ext, _)| ext).collect()
}

/// Check if an extension's type arguments are applicable to an actual type's substitutions.
///
/// This performs a simple unification check by comparing types at each type parameter position:
/// - Type parameters in the extension match any concrete type
/// - Concrete types in the extension must match exactly
///
/// IMPORTANT: We compare by type parameter ID (key) rather than by iteration order,
/// because HashMap iteration order is undefined.
fn is_extension_applicable(
    extension_subs: &kestrel_semantic_tree::ty::Substitutions,
    actual_subs: &kestrel_semantic_tree::ty::Substitutions,
) -> bool {
    // Must have same number of type arguments
    if extension_subs.len() != actual_subs.len() {
        return false;
    }

    // If both have no type arguments (e.g., Point with no generics), they match
    if extension_subs.is_empty() && actual_subs.is_empty() {
        return true;
    }

    // Check each type argument by looking up by parameter ID
    for (param_id, ext_ty) in extension_subs.iter() {
        // Get the corresponding actual type for this parameter
        let actual_ty = match actual_subs.get(*param_id) {
            Some(ty) => ty,
            None => return false, // Extension has a param that actual doesn't
        };

        if ext_ty.is_type_parameter() {
            // Extension has a type parameter here - matches anything
            continue;
        } else {
            // Extension has a concrete type - must match exactly
            // Use types_match to avoid infinite recursion from is_assignable_to
            if !types_match_simple(ext_ty, actual_ty) {
                return false;
            }
        }
    }

    true
}

/// Simple type matching that checks structural equality without recursion.
/// This is used to avoid infinite loops during extension applicability checking.
///
/// IMPORTANT: This function ONLY compares types at the top level - it does NOT
/// recursively compare substitutions. For example, Box[Int] matches Box[String]
/// because they're both Box (same struct symbol). The caller must handle
/// type argument comparison separately.
fn types_match_simple(a: &Ty, b: &Ty) -> bool {
    use kestrel_semantic_tree::ty::TyKind;

    match (a.kind(), b.kind()) {
        // Primitives - direct comparison
        (TyKind::Unit, TyKind::Unit) => true,
        (TyKind::Never, TyKind::Never) => true,
        (TyKind::Bool, TyKind::Bool) => true,
        (TyKind::String, TyKind::String) => true,
        (TyKind::Int(a_bits), TyKind::Int(b_bits)) => a_bits == b_bits,
        (TyKind::Float(a_bits), TyKind::Float(b_bits)) => a_bits == b_bits,

        // Structs - compare by symbol ID only, NOT substitutions
        // The caller must check substitutions separately to avoid infinite recursion
        (TyKind::Struct { symbol: a_sym, .. }, TyKind::Struct { symbol: b_sym, .. }) => {
            a_sym.metadata().id() == b_sym.metadata().id()
        },

        // Type parameters - compare by symbol ID
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        },

        // Error types match anything (to suppress cascading errors)
        (TyKind::Error, _) | (_, TyKind::Error) => true,

        // Different kinds don't match
        _ => false,
    }
}

/// Normalize a type parameter using equality constraints from a where clause.
///
/// If the where clause contains a constraint like `V = Array[E]`, this function
/// returns `Some(Array[E])` for type parameter `V`. Returns `None` if no equality
/// constraint exists for the given type parameter.
fn normalize_type_param_with_equality(
    param_id: semantic_tree::symbol::SymbolId,
    where_clause: &kestrel_semantic_tree::ty::WhereClause,
) -> Option<Ty> {
    fn is_concrete_for_member_access(ty: &Ty) -> bool {
        !matches!(
            ty.kind(),
            TyKind::TypeParameter(_)
                | TyKind::AssociatedType { .. }
                | TyKind::Infer
                | TyKind::SelfType
                | TyKind::Error
        )
    }

    for constraint in where_clause.constraints() {
        if let kestrel_semantic_tree::ty::Constraint::TypeEquality { left, right, .. } = constraint
        {
            // Check if left side is our type parameter
            if let TyKind::TypeParameter(tp) = left.kind()
                && tp.metadata().id() == param_id
            {
                // Return the right side if it's concrete enough for member access
                if is_concrete_for_member_access(right) {
                    return Some(right.clone());
                }
            }
            // Also check if right side is our type parameter (constraints are symmetric)
            if let TyKind::TypeParameter(tp) = right.kind()
                && tp.metadata().id() == param_id
            {
                // Return the left side if it's concrete enough for member access
                if is_concrete_for_member_access(left) {
                    return Some(left.clone());
                }
            }
        }
    }
    None
}

/// Check if a type parameter has a specific protocol bound in a where clause.
///
/// This is used when checking extension applicability - if the actual type at a
/// type parameter position is itself a type parameter with a matching bound in
/// the current context's where clause, the extension should be considered applicable.
fn type_param_has_bound_in_where_clause(
    param_id: semantic_tree::symbol::SymbolId,
    bound: &Ty,
    where_clause: &kestrel_semantic_tree::ty::WhereClause,
) -> bool {
    let TyKind::Protocol {
        symbol: required_proto,
        ..
    } = bound.kind()
    else {
        return false;
    };

    for constraint in where_clause.constraints() {
        // Check if this constraint is for our type parameter
        if let Some(constraint_param_id) = constraint.type_parameter_id()
            && constraint_param_id == param_id
        {
            // Check if any of the bounds match
            for constraint_bound in constraint.bounds() {
                if let TyKind::Protocol {
                    symbol: bound_proto,
                    ..
                } = constraint_bound.kind()
                    && bound_proto.metadata().id() == required_proto.metadata().id()
                {
                    return true;
                }
            }
        }
    }

    false
}

/// Resolve SelfType to the concrete type with substitutions.
///
/// When `self` is used in an extension method like `extend Box[Int]`, its type is SelfType,
/// but we need the actual type `Box[Int]` (with substitutions) for member access.
pub fn resolve_self_type_to_concrete(ty: &Ty, ctx: &BodyResolutionContext) -> Ty {
    match ty.kind() {
        TyKind::SelfType => {
            // Get the function symbol, then its parent (struct/protocol/extension)
            if let Some(function) = ctx.model.query(SymbolFor {
                id: ctx.function_id,
            }) && let Some(parent) = function.metadata().parent()
            {
                match parent.metadata().kind() {
                    KestrelSymbolKind::Extension => {
                        // For extension methods, get the target type from ExtensionTargetBehavior
                        // This gives us the type with substitutions (e.g., Box[Int] not Box[T])
                        if let Some(target_beh) =
                            parent.metadata().get_behavior::<ExtensionTargetBehavior>()
                        {
                            // For protocol extensions, keep SelfType abstract so constraint
                            // methods can be resolved (e.g., `extend Proto where Self: Other`)
                            if target_beh.is_protocol_extension() {
                                return ty.clone();
                            }
                            let target_ty = target_beh.target_type();
                            // Make sure target type isn't also SelfType (should never happen, but prevent infinite recursion)
                            if !matches!(target_ty.kind(), TyKind::SelfType) {
                                return target_ty.clone();
                            }
                        }
                    },
                    KestrelSymbolKind::Struct => {
                        // For struct methods, resolve Self to the concrete struct type
                        if let Some(typed) = parent.metadata().get_behavior::<TypedBehavior>() {
                            let struct_ty = typed.ty();
                            if !matches!(struct_ty.kind(), TyKind::SelfType) {
                                return struct_ty.clone();
                            }
                        }
                    },
                    KestrelSymbolKind::Protocol => {
                        // For protocol methods, Self remains abstract
                        // (needed for future default impl support)
                    },
                    _ => {},
                }
            }
            ty.clone()
        },
        _ => ty.clone(),
    }
}

/// Resolve a delegating initializer call: `self.init(...)`
///
/// Called when inside an initializer body and `self.init(...)` is encountered.
/// Finds a matching initializer on the parent struct and creates a DelegatingInit expression.
pub fn resolve_delegating_init(
    current_init: &Arc<dyn Symbol<KestrelLanguage>>,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Get the parent struct
    let Some(parent) = current_init.metadata().parent() else {
        return Expression::error(span);
    };

    if parent.metadata().kind() != KestrelSymbolKind::Struct {
        // Initializer should always be inside a struct
        return Expression::error(span);
    }

    // Find all initializers in the parent struct
    let initializers: Vec<Arc<dyn Symbol<KestrelLanguage>>> = parent
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Initializer)
        .collect();

    // Find matching initializer by arity and labels
    for init_sym in &initializers {
        // Skip self-delegation (can't call the same initializer)
        if init_sym.metadata().id() == current_init.metadata().id() {
            continue;
        }

        if let Some(callable) = get_callable_behavior(init_sym)
            && matches_signature(&callable, arguments.len(), arg_labels)
        {
            // Found matching initializer
            let init_id = init_sym.metadata().id();

            // Validate access modes for arguments
            validate_argument_access_modes(&callable, &arguments, &span, ctx);

            // Build substitutions from the self type
            // For generic structs like Array[T], self has type Array[T] with substitutions {T -> TypeParameter(T)}
            // We need to pass these substitutions to the delegated initializer
            let substitutions = if let Some(self_local_id) = ctx.local_scope.lookup("self") {
                if let Some(self_local) = ctx.local_scope.get_local(self_local_id) {
                    let self_ty = self_local.ty();
                    if let Some((_, subs)) = self_ty.as_struct_with_subs() {
                        subs.clone()
                    } else {
                        Substitutions::new()
                    }
                } else {
                    Substitutions::new()
                }
            } else {
                Substitutions::new()
            };

            return Expression::delegating_init(init_id, arguments, substitutions, span);
        }
    }

    // No matching initializer found - report error
    let struct_name = parent.metadata().name().value.clone();

    // Use the existing NoMatchingMethodError for simplicity
    let error = NoSuchMethodError {
        call_span: span.clone(),
        method_name: "init".to_string(),
        receiver_type: struct_name,
    };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
    Expression::error(span)
}
