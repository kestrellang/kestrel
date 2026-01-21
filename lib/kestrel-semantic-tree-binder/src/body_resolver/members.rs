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
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression, PrimitiveMethod};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::symbol::protocol::FlattenedProtocolBehavior;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::Substitutions;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{
    AmbiguousConstrainedMethodError, CannotAccessMemberOnTypeError,
    DelegatingInitOutsideInitializerError, MemberNotAccessibleError, MemberNotVisibleError,
    MethodNotInBoundsError, NoMatchingMethodError, NoSuchMemberError, NoSuchMethodError,
    UnconstrainedTypeParameterMemberError,
};

use super::calls::{
    collect_overload_descriptions, try_resolve_subscript_call, validate_argument_access_modes,
};
use super::context::BodyResolutionContext;
use super::utils::{
    find_type_directed_match, format_symbol_kind, get_callable_behavior, get_type_container,
    get_type_parameter_bounds_by_id, get_type_parameter_bounds_from_context, infer_type_arguments,
    matches_signature, substitute_self, substitute_type, type_satisfies_bound,
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
    let base_ty = &base.ty;
    let full_span = Span::new(base_span.file_id, base_span.start..member_span.end);

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
        return resolve_associated_type_member_access(
            &base,
            member_name,
            member_span,
            full_span,
            ctx,
        );
    }

    // 1. Check for primitive method (e.g., 5.toString, "hello".length)
    // Primitive methods can only be called, not used as first-class values.
    // Return a PrimitiveMethodRef that call resolution can convert to a call.
    // If this expression is NOT called, call resolution will emit an error.
    if let Some(primitive_method) = PrimitiveMethod::lookup(base_ty, member_name) {
        return Expression::primitive_method_ref(base, primitive_method, full_span);
    }

    // 2. Handle type parameter specially - we can't access fields, only methods
    // For type parameters, create a MethodRef that will be resolved when called
    if let TyKind::TypeParameter(type_param) = base_ty.kind() {
        let type_param = type_param.clone();
        return resolve_constrained_member_access(
            base,
            &type_param,
            member_name,
            member_span,
            full_span.clone(),
            ctx,
        );
    }

    // 3. If base type is Infer, don't emit error yet - type inference will resolve it.
    // Create a field access with inferred type and let type inference resolve the actual field type.
    if matches!(base_ty.kind(), TyKind::Infer) {
        return Expression::field_access(
            base,
            member_name.to_string(),
            false, // field_mutable - conservative default, type inference may correct this
            Ty::infer(member_span.clone()),
            full_span,
        );
    }

    // 3.5. If base type is Error, propagate error without cascading diagnostics.
    // The original error has already been reported where the error type was created.
    if matches!(base_ty.kind(), TyKind::Error) {
        return Expression::error(full_span);
    }

    // 4. Get container from base type
    let container = match get_type_container(base_ty, ctx) {
        Some(c) => c,
        None => {
            // Type doesn't support member access (e.g., Int, Bool, etc.)
            let error = CannotAccessMemberOnTypeError {
                span: full_span.clone(),
                base_type: base_ty.to_string(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(full_span.clone());
        },
    };

    // 2. Find child with that name - first in direct children, then in extensions
    let member = container
        .metadata()
        .children()
        .into_iter()
        .find(|c| c.metadata().name().value == member_name);

    // Get applicable extensions once for reuse
    let container_id = container.metadata().id();
    let extensions = ctx.model.query(ExtensionsFor {
        target_id: container_id,
    });
    // Resolve Self to concrete type for extension filtering (Self doesn't have substitutions)
    let resolved_base_ty_for_extensions = resolve_self_type_to_concrete(base_ty, ctx);
    // Filter to only applicable extensions (now with cycle detection in substitutions)
    let applicable_extensions =
        filter_applicable_extensions(extensions, &resolved_base_ty_for_extensions, ctx);

    // If not found in direct children, search type extensions, then protocol extensions
    let member = match member {
        Some(m) => m,
        None => {
            // Try to find in applicable type extensions
            let extension_member = applicable_extensions
                .iter()
                .flat_map(|ext| ext.metadata().children())
                .find(|child| child.metadata().name().value == member_name);

            match extension_member {
                Some(m) => m,
                None => {
                    // Try to find in protocol extensions
                    let protocol_ext_methods =
                        find_methods_in_protocol_extensions(base_ty, member_name, ctx);

                    if !protocol_ext_methods.is_empty() {
                        // Found method(s) in protocol extensions - create MethodRef
                        return Expression::method_ref(
                            base,
                            protocol_ext_methods,
                            member_name.to_string(),
                            full_span.clone(),
                        );
                    }

                    // Try Self constraint protocols (for protocol extensions with where clauses)
                    if matches!(base_ty.kind(), TyKind::SelfType) {
                        let constraint_methods =
                            get_methods_from_self_constraints(member_name, ctx);
                        if !constraint_methods.is_empty() {
                            let method_ids: Vec<_> = constraint_methods
                                .iter()
                                .map(|m| m.metadata().id())
                                .collect();
                            return Expression::method_ref(
                                base,
                                method_ids,
                                member_name.to_string(),
                                full_span.clone(),
                            );
                        }
                    }

                    let error = NoSuchMemberError {
                        member_span,
                        member_name: member_name.to_string(),
                        base_span,
                        base_type: base_ty.to_string(),
                    };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return Expression::error(full_span.clone());
                },
            }
        },
    };

    // 3. Check visibility
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
        return Expression::error(full_span.clone());
    }

    // 4. Get MemberAccessBehavior or ComputedMemberAccessBehavior and produce expression
    for behavior in member.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::MemberAccess
            && let Some(access) = behavior.as_ref().downcast_ref::<MemberAccessBehavior>()
        {
            let mut result = access.access(base.clone(), full_span.clone());
            // Apply substitutions from the parent's type to the member type
            // e.g., for Box[T].value, substitute Box's T with the instantiated type arg

            // First, resolve SelfType to the actual type if needed
            let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);

            if let Some((_, substitutions)) = resolved_base_ty.as_struct_with_subs() {
                result.ty = result.ty.apply_substitutions(substitutions);
            }
            return result;
        }
        // Handle computed properties (getter-only access for now)
        if behavior.kind() == KestrelBehaviorKind::ComputedMemberAccess
            && let Some(access) = behavior
                .as_ref()
                .downcast_ref::<ComputedMemberAccessBehavior>()
        {
            let mut result = access.access(base.clone(), full_span.clone());
            // Apply substitutions from the parent's type to the member type
            let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);

            if let Some((_, substitutions)) = resolved_base_ty.as_struct_with_subs() {
                result.ty = result.ty.apply_substitutions(substitutions);
            }
            return result;
        }
    }

    // 5. If it's a function, create a MethodRef (for method calls like obj.method())
    if member.metadata().kind() == KestrelSymbolKind::Function {
        // Find all methods with this name (for overloads) from direct children
        let mut candidates: Vec<SymbolId> = container
            .metadata()
            .children()
            .into_iter()
            .filter(|c| {
                c.metadata().kind() == KestrelSymbolKind::Function
                    && c.metadata().name().value == member_name
            })
            .map(|c| c.metadata().id())
            .collect();

        // Also collect methods from applicable type extensions
        for extension in &applicable_extensions {
            for child in extension.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::Function
                    && child.metadata().name().value == member_name
                {
                    candidates.push(child.metadata().id());
                }
            }
        }

        // Also collect methods from protocol extensions (lowest priority)
        let protocol_ext_methods = find_methods_in_protocol_extensions(base_ty, member_name, ctx);
        candidates.extend(protocol_ext_methods);

        return Expression::method_ref(
            base,
            candidates,
            member_name.to_string(),
            full_span.clone(),
        );
    }

    // Member exists but doesn't have MemberAccessBehavior (e.g., type alias, nested type)
    let error = MemberNotAccessibleError {
        member_span,
        member_name: member_name.to_string(),
        base_span,
        base_type: base_ty.to_string(),
        member_kind: format_symbol_kind(member.metadata().kind()),
    };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
    Expression::error(full_span.clone())
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

/// Tracks a method found in a protocol bound, with its source protocol.
struct ProtocolMethodCandidate {
    method_id: SymbolId,
    protocol_name: String,
    definition_span: Span,
}

/// Resolve a member access on a constrained type parameter: a.member where a: T
///
/// Type parameters can only have method members (accessed from protocol bounds).
/// This creates a MethodRef that will be resolved when the call happens.
fn resolve_constrained_member_access(
    base: Expression,
    type_param: &Arc<kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol>,
    member_name: &str,
    _member_span: Span,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let type_param_name = type_param.metadata().name().value.clone();

    // Get all protocol bounds for this type parameter
    let bounds = get_type_parameter_bounds_from_context(type_param, ctx);

    // If no bounds, report error
    if bounds.is_empty() {
        let error = UnconstrainedTypeParameterMemberError {
            span: full_span.clone(),
            member_name: member_name.to_string(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Collect all method candidates from protocol bounds, tracking their source
    let mut candidates: Vec<ProtocolMethodCandidate> = Vec::new();
    let mut bound_names: Vec<String> = Vec::new();

    for bound in &bounds {
        if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Collect method IDs from this protocol (including inherited)
            collect_protocol_method_candidates(proto, member_name, &mut candidates, ctx);
        }
    }

    if candidates.is_empty() {
        let error = MethodNotInBoundsError {
            call_span: full_span.clone(),
            method_name: member_name.to_string(),
            type_param_name,
            bound_names,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Check for ambiguity - multiple distinct protocols have a method with this name
    let mut unique_protocols: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for c in &candidates {
        unique_protocols.insert(&c.protocol_name);
    }

    if unique_protocols.len() > 1 {
        // Ambiguous - method found in multiple different protocols
        let protocol_names: Vec<String> =
            candidates.iter().map(|c| c.protocol_name.clone()).collect();
        let definition_spans: Vec<(String, Span)> = candidates
            .iter()
            .map(|c| (c.protocol_name.clone(), c.definition_span.clone()))
            .collect();

        let error = AmbiguousConstrainedMethodError {
            call_span: full_span.clone(),
            method_name: member_name.to_string(),
            protocol_names,
            definition_spans,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Single protocol source - create MethodRef
    let method_ids: Vec<SymbolId> = candidates.iter().map(|c| c.method_id).collect();
    Expression::method_ref(base, method_ids, member_name.to_string(), full_span)
}

/// Collect method candidates from a protocol, including inherited protocols.
/// Each candidate tracks which protocol it came from for ambiguity detection.
fn collect_protocol_method_candidates(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    candidates: &mut Vec<ProtocolMethodCandidate>,
    _ctx: &BodyResolutionContext,
) {
    // Use flattened behavior if available (normal case after BIND phase)
    if let Some(flattened) = protocol
        .metadata()
        .get_behavior::<FlattenedProtocolBehavior>()
    {
        if let Some(methods) = flattened.methods().get(method_name) {
            for method in methods {
                candidates.push(ProtocolMethodCandidate {
                    method_id: method.symbol.metadata().id(),
                    protocol_name: method.source_protocol_name.clone(),
                    definition_span: method.definition_span.clone(),
                });
            }
        }
        return;
    }

    // FALLBACK: Recursive traversal (for BUILD phase or if flattening failed)
    let proto_name = protocol.metadata().name().value.clone();

    // Search direct methods
    for child in protocol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Function
            && child.metadata().name().value == method_name
        {
            candidates.push(ProtocolMethodCandidate {
                method_id: child.metadata().id(),
                protocol_name: proto_name.clone(),
                definition_span: child.metadata().name().span.clone(),
            });
        }
    }

    // Search inherited protocols
    if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
        for parent_proto_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol: parent, .. } = parent_proto_ty.kind() {
                collect_protocol_method_candidates(parent, method_name, candidates, _ctx);
            }
        }
    }
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

    // Check if base type is a type parameter - needs special handling
    if let TyKind::TypeParameter(type_param) = base_ty.kind() {
        return resolve_constrained_member_call(
            object,
            type_param,
            member_name,
            arguments,
            arg_labels,
            span,
            ctx,
        );
    }

    // If base type is Infer, don't emit error yet - type inference will resolve it.
    // Create a deferred method call with inferred return type.
    if matches!(base_ty.kind(), TyKind::Infer) {
        return Expression::deferred_method_call(
            object.clone(),
            member_name.to_string(),
            arguments,
            Ty::infer(span.clone()),
            span,
        );
    }

    // If base type is Error, propagate error without cascading diagnostics.
    if matches!(base_ty.kind(), TyKind::Error) {
        return Expression::error(span);
    }

    // Get container from type (for Struct, Protocol, Self types)
    let container = match get_type_container(base_ty, ctx) {
        Some(c) => c,
        None => {
            // Report error: cannot call method on this type
            let error = NoSuchMethodError {
                call_span: span.clone(),
                method_name: member_name.to_string(),
                receiver_type: base_ty.to_string(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(span);
        },
    };

    // Find method(s) with this name - first in direct children
    let mut methods: Vec<Arc<dyn Symbol<KestrelLanguage>>> = container
        .metadata()
        .children()
        .into_iter()
        .filter(|c| {
            c.metadata().kind() == KestrelSymbolKind::Function
                && c.metadata().name().value == member_name
        })
        .collect();

    // If not found in direct children, search extensions
    if methods.is_empty() {
        let container_id = container.metadata().id();
        let extensions = ctx.model.query(ExtensionsFor {
            target_id: container_id,
        });

        // Resolve Self to concrete type for extension filtering (Self doesn't have substitutions)
        let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);
        // Filter to applicable extensions, sorted by specificity (now with cycle detection)
        let applicable_extensions =
            filter_applicable_extensions(extensions, &resolved_base_ty, ctx);

        for extension in applicable_extensions {
            for child in extension.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::Function
                    && child.metadata().name().value == member_name
                {
                    methods.push(child);
                }
            }
        }
    }

    // If still not found and base type is SelfType, check Self constraint protocols
    // This allows `self.constraintMethod()` inside `extend Proto where Self: OtherProto { ... }`
    if methods.is_empty() && matches!(base_ty.kind(), TyKind::SelfType) {
        methods = get_methods_from_self_constraints(member_name, ctx);
    }

    if methods.is_empty() {
        // No method found - try field + subscript as fallback
        // This handles: obj.field(index) where field has subscripts
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

        // Report error: no such method
        let error = NoSuchMethodError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            receiver_type: base_ty.to_string(),
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Find matching overload - collect all candidates first for type-directed selection
    let mut invisible_matches = Vec::new();
    let mut candidates: Vec<(usize, &Arc<dyn Symbol<KestrelLanguage>>, CallableBehavior)> =
        Vec::new();

    for (idx, method) in methods.iter().enumerate() {
        if let Some(callable) = get_callable_behavior(method)
            && matches_signature(&callable, arguments.len(), arg_labels)
        {
            // Check visibility
            let method_id = method.metadata().id();
            if !ctx.model.query(IsVisibleFrom {
                target: method_id,
                context: ctx.function_id,
            }) {
                invisible_matches.push(method.clone());
                continue;
            }
            candidates.push((idx, method, callable));
        }
    }

    // Select the best candidate using type-directed conformance if multiple
    let selected_idx = if candidates.len() > 1 {
        let arg_types: Vec<Ty> = arguments.iter().map(|arg| arg.value.ty.clone()).collect();
        // Use methods[0] as dummy struct_symbol (not used in the function anyway)
        find_type_directed_match(&candidates, &arg_types, &methods[0]).unwrap_or(0)
    } else {
        0
    };

    // Use the selected candidate
    if let Some((_, method, callable)) = candidates.get(selected_idx) {
        let mut return_ty = callable.return_type().clone();
        let method_id = method.metadata().id();

        // Apply substitutions from the base type to the return type
        // e.g., for Box[Int].get() where get returns T, substitute T with Int
        // or for Option[Int].Some where Some returns Option[T], substitute T with Int
        let resolved_base_ty = resolve_self_type_to_concrete(base_ty, ctx);
        if let Some((_, substitutions)) = resolved_base_ty.as_struct_with_subs() {
            return_ty = return_ty.apply_substitutions(substitutions);
        } else if let Some((enum_sym, substitutions)) = resolved_base_ty.as_enum_with_subs() {
            // Check if base type already has concrete substitutions
            let has_concrete_subs = !substitutions.is_empty()
                && substitutions
                    .iter()
                    .all(|(_, ty)| !matches!(ty.kind(), TyKind::TypeParameter(_)));

            if has_concrete_subs {
                // Base type has concrete type args (e.g., Option[Int].Some)
                return_ty = return_ty.apply_substitutions(substitutions);
            } else {
                // Base type has no concrete type args - infer from arguments
                // e.g., Option.Some(value: 42) should infer T = Int from the argument
                let type_params = enum_sym.type_parameters();
                if !type_params.is_empty() {
                    let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
                    let inferred_subs = infer_type_arguments(&type_params, callable, &arg_types);
                    return_ty = substitute_type(&return_ty, &inferred_subs);
                }
            }
        }

        // Validate access modes for arguments
        validate_argument_access_modes(callable, &arguments, &span, ctx);

        // Build combined substitutions for the Call expression:
        // 1. Base type substitutions (e.g., T from Optional[T])
        // 2. Method's own type parameter substitutions (e.g., U from map[U])
        let mut call_subs = Substitutions::new();

        // Add base type substitutions
        if let Some((_, base_subs)) = resolved_base_ty.as_struct_with_subs() {
            for (key, ty) in base_subs.iter() {
                call_subs.insert(*key, ty.clone());
            }
        } else if let Some((_, base_subs)) = resolved_base_ty.as_enum_with_subs() {
            for (key, ty) in base_subs.iter() {
                call_subs.insert(*key, ty.clone());
            }
        }

        // Infer method's own type parameters from argument types
        // e.g., for map[U](transform: (T) -> U), infer U from the closure's return type
        if let Some(generics) = method.metadata().get_behavior::<GenericsBehavior>() {
            let method_type_params = generics.type_parameters();
            if !method_type_params.is_empty() {
                let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
                let method_subs = infer_type_arguments(method_type_params, callable, &arg_types);
                for (key, ty) in method_subs.iter() {
                    call_subs.insert(*key, ty.clone());
                }
                // Also apply method substitutions to return type
                return_ty = return_ty.apply_substitutions(&method_subs);
            }
        }

        // Create method ref and then call
        let method_ref = Expression::method_ref(
            object.clone(),
            vec![method_id],
            member_name.to_string(),
            span.clone(),
        );

        // Use generic_call to store substitutions for type checking
        return Expression::generic_call(method_ref, arguments, call_subs, return_ty, span);
    }

    // No matching visible method found
    if !invisible_matches.is_empty() {
        let first_invisible = &invisible_matches[0];
        let visibility = first_invisible
            .metadata()
            .get_behavior::<VisibilityBehavior>()
            .and_then(|v| v.visibility().map(|vis| vis.to_string()))
            .unwrap_or_else(|| "internal".to_string());

        let error = MemberNotVisibleError {
            member_span: span.clone(),
            member_name: member_name.to_string(),
            base_span: object.span.clone(),
            base_type: base_ty.to_string(),
            visibility,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // No matching method found - collect overload info for error message
    let receiver_type = base_ty.to_string();
    let method_ids: Vec<SymbolId> = methods.iter().map(|m| m.metadata().id()).collect();
    let available_overloads = collect_overload_descriptions(&method_ids, ctx.model);

    let error = NoMatchingMethodError {
        call_span: span.clone(),
        method_name: member_name.to_string(),
        receiver_type,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_overloads,
    };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());

    Expression::error(span)
}

// =============================================================================
// Constrained Type Parameter Method Resolution
// =============================================================================

/// A method candidate found in a protocol bound.
struct ConstrainedMethodCandidate {
    /// The method symbol
    method: Arc<dyn Symbol<KestrelLanguage>>,
    /// The callable behavior with Self substituted
    callable: CallableBehavior,
    /// The protocol this method comes from
    protocol_name: String,
    /// Span of the method definition
    definition_span: Span,
}

/// Resolve a method call on a constrained type parameter.
///
/// When calling `a.method(b)` where `a: T` and `T: Protocol`:
/// 1. Look up the protocol bounds for T
/// 2. Search for the method in each protocol (including inherited protocols)
/// 3. Substitute Self with T in the method signature
/// 4. Check for ambiguous methods (same signature in multiple protocols)
/// 5. Return the resolved call expression
fn resolve_constrained_member_call(
    object: &Expression,
    type_param: &Arc<kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol>,
    member_name: &str,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let type_param_name = type_param.metadata().name().value.clone();
    let receiver_ty = &object.ty;

    // Get all protocol bounds for this type parameter
    let bounds = get_type_parameter_bounds_from_context(type_param, ctx);

    // If no bounds, report error
    if bounds.is_empty() {
        let error = UnconstrainedTypeParameterMemberError {
            span: span.clone(),
            member_name: member_name.to_string(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Collect all matching methods from all protocol bounds
    let mut candidates: Vec<ConstrainedMethodCandidate> = Vec::new();
    let mut bound_names: Vec<String> = Vec::new();

    for bound in &bounds {
        if let TyKind::Protocol {
            symbol: proto,
            substitutions,
        } = bound.kind()
        {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Collect methods from this protocol (including inherited)
            collect_protocol_methods(
                proto,
                member_name,
                receiver_ty,
                substitutions,
                &mut candidates,
                ctx,
            );
        }
    }

    if candidates.is_empty() {
        let error = MethodNotInBoundsError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            type_param_name,
            bound_names,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Find matching candidates by signature
    let matching: Vec<&ConstrainedMethodCandidate> = candidates
        .iter()
        .filter(|c| matches_signature(&c.callable, arguments.len(), arg_labels))
        .collect();

    if matching.is_empty() {
        // No matching signature - report error with available overloads
        let method_ids: Vec<SymbolId> = candidates
            .iter()
            .map(|c| c.method.metadata().id())
            .collect();
        let available_overloads = collect_overload_descriptions(&method_ids, ctx.model);

        let error = NoMatchingMethodError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            receiver_type: type_param_name,
            provided_labels: arg_labels.to_vec(),
            provided_arity: arguments.len(),
            available_overloads,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Check for ambiguity - multiple protocols have matching method with same signature
    // Deduplicate by protocol name (same protocol can appear multiple times)
    let mut seen_protocols: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unique_matching: Vec<&ConstrainedMethodCandidate> = matching
        .into_iter()
        .filter(|c| seen_protocols.insert(c.protocol_name.clone()))
        .collect();

    if unique_matching.len() > 1 {
        // Multiple different protocols have matching method with same signature
        let protocol_names: Vec<String> = unique_matching
            .iter()
            .map(|c| c.protocol_name.clone())
            .collect();
        let definition_spans: Vec<(String, Span)> = unique_matching
            .iter()
            .map(|c| (c.protocol_name.clone(), c.definition_span.clone()))
            .collect();

        let error = AmbiguousConstrainedMethodError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            protocol_names,
            definition_spans,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    let matching = unique_matching;

    // Single matching method found
    let winner = matching[0];
    let mut return_ty = winner.callable.return_type().clone();
    let method_id = winner.method.metadata().id();

    // Validate access modes for arguments
    validate_argument_access_modes(&winner.callable, &arguments, &span, ctx);

    // Build substitutions for the Call expression
    let mut call_subs = Substitutions::new();

    // Infer method's own type parameters from argument types
    if let Some(generics) = winner.method.metadata().get_behavior::<GenericsBehavior>() {
        let method_type_params = generics.type_parameters();
        if !method_type_params.is_empty() {
            let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
            let method_subs =
                infer_type_arguments(method_type_params, &winner.callable, &arg_types);
            for (key, ty) in method_subs.iter() {
                call_subs.insert(*key, ty.clone());
            }
            // Also apply method substitutions to return type
            return_ty = return_ty.apply_substitutions(&method_subs);
        }
    }

    // Create method ref and call
    let method_ref = Expression::method_ref(
        object.clone(),
        vec![method_id],
        member_name.to_string(),
        span.clone(),
    );

    // Use generic_call to store substitutions for type checking
    Expression::generic_call(method_ref, arguments, call_subs, return_ty, span)
}

/// Collect methods from a protocol, including inherited protocols.
///
/// The `protocol_substitutions` parameter contains the type arguments for the protocol
/// bound, e.g., for `T: Converter[lang.i64]`, it maps the Converter's type parameter
/// to `lang.i64`. These substitutions are applied to method signatures.
#[allow(clippy::only_used_in_recursion)]
fn collect_protocol_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    receiver_ty: &Ty,
    protocol_substitutions: &Substitutions,
    candidates: &mut Vec<ConstrainedMethodCandidate>,
    ctx: &BodyResolutionContext,
) {
    // Use flattened behavior if available (normal case after BIND phase)
    if let Some(flattened) = protocol
        .metadata()
        .get_behavior::<FlattenedProtocolBehavior>()
    {
        if let Some(methods) = flattened.methods().get(method_name) {
            for method in methods {
                if let Some(callable) = get_callable_behavior(&method.symbol) {
                    // Substitute Self with the receiver type
                    let substituted_callable = substitute_callable_self(&callable, receiver_ty);
                    // Apply protocol type parameter substitutions
                    let substituted_callable =
                        substitute_callable(&substituted_callable, protocol_substitutions);

                    candidates.push(ConstrainedMethodCandidate {
                        method: method.symbol.clone(),
                        callable: substituted_callable,
                        protocol_name: method.source_protocol_name.clone(),
                        definition_span: method.definition_span.clone(),
                    });
                }
            }
        }
        return;
    }

    // FALLBACK: Recursive traversal (for BUILD phase or if flattening failed)
    let proto_name = protocol.metadata().name().value.clone();

    // Search direct methods
    for child in protocol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Function
            && child.metadata().name().value == method_name
            && let Some(callable) = get_callable_behavior(&child)
        {
            // Substitute Self with the receiver type
            let substituted_callable = substitute_callable_self(&callable, receiver_ty);
            // Apply protocol type parameter substitutions
            let substituted_callable =
                substitute_callable(&substituted_callable, protocol_substitutions);

            candidates.push(ConstrainedMethodCandidate {
                method: child.clone(),
                callable: substituted_callable,
                protocol_name: proto_name.clone(),
                definition_span: child.metadata().name().span.clone(),
            });
        }
    }

    // Search inherited protocols
    if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
        for parent_proto_ty in conformances.conformances() {
            if let TyKind::Protocol {
                symbol: parent,
                substitutions: parent_subs,
            } = parent_proto_ty.kind()
            {
                // Compose substitutions: apply our substitutions to the parent's type arguments
                let composed_subs = compose_substitutions(protocol_substitutions, parent_subs);
                collect_protocol_methods(
                    parent,
                    method_name,
                    receiver_ty,
                    &composed_subs,
                    candidates,
                    ctx,
                );
            }
        }
    }
}

/// Substitute Self with the receiver type in a CallableBehavior.
pub fn substitute_callable_self(callable: &CallableBehavior, receiver_ty: &Ty) -> CallableBehavior {
    use kestrel_semantic_tree::behavior::callable::CallableParameter;

    let new_params: Vec<CallableParameter> = callable
        .parameters()
        .iter()
        .map(|p| {
            let new_ty = substitute_self(&p.ty, receiver_ty);
            CallableParameter {
                access_mode: p.access_mode,
                ty: new_ty,
                label: p.label.clone(),
                bind_name: p.bind_name.clone(),
            }
        })
        .collect();

    let new_return = substitute_self(callable.return_type(), receiver_ty);

    // Preserve receiver kind if present
    match callable.receiver() {
        Some(receiver_kind) => CallableBehavior::with_receiver(
            new_params,
            new_return,
            receiver_kind,
            callable.span().clone(),
        ),
        None => CallableBehavior::new(new_params, new_return, callable.span().clone()),
    }
}

/// Substitute type parameters in a CallableBehavior using protocol substitutions.
///
/// This is used when a protocol bound has type arguments, e.g., `T: Converter[lang.i64]`.
/// The protocol's type parameters need to be substituted with the concrete types.
fn substitute_callable(
    callable: &CallableBehavior,
    substitutions: &Substitutions,
) -> CallableBehavior {
    use kestrel_semantic_tree::behavior::callable::CallableParameter;

    // Skip if no substitutions to apply
    if substitutions.is_empty() {
        return callable.clone();
    }

    let new_params: Vec<CallableParameter> = callable
        .parameters()
        .iter()
        .map(|p| CallableParameter {
            access_mode: p.access_mode,
            ty: substitute_type(&p.ty, substitutions),
            label: p.label.clone(),
            bind_name: p.bind_name.clone(),
        })
        .collect();

    let new_return = substitute_type(callable.return_type(), substitutions);

    // Preserve receiver kind if present
    match callable.receiver() {
        Some(receiver_kind) => CallableBehavior::with_receiver(
            new_params,
            new_return,
            receiver_kind,
            callable.span().clone(),
        ),
        None => CallableBehavior::new(new_params, new_return, callable.span().clone()),
    }
}

/// Compose substitutions: apply outer substitutions to inner substitution values.
///
/// When a generic protocol inherits from another generic protocol, we need to
/// compose the substitutions. For example, if `SpecialConverter[T]: Converter[T]`
/// and we have a bound `X: SpecialConverter[lang.i64]`, then when looking at
/// Converter's methods, T should be substituted with lang.i64.
fn compose_substitutions(outer: &Substitutions, inner: &Substitutions) -> Substitutions {
    let mut result = Substitutions::new();
    for (id, ty) in inner.iter() {
        result.insert(*id, substitute_type(ty, outer));
    }
    result
}

/// Resolve static member access on a type parameter: `T.staticMethod`.
///
/// This looks up static methods from the type parameter's protocol bounds.
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
        // No bounds - cannot access static methods on unconstrained type parameter
        let error = UnconstrainedTypeParameterMemberError {
            span: full_span.clone(),
            member_name: member_name.to_string(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // For Self substitution - use the type parameter type so T.create() returns T, not _
    let type_param_ty = Ty::type_parameter(type_param_arc.clone(), full_span.clone());

    // First, check if member_name is an associated type in any protocol bound
    // This enables chained associated type access like T.Next.Next.baseValue()
    if let Some(assoc_type_expr) =
        find_associated_type_in_bounds(&bounds, member_name, &type_param_ty, full_span.clone(), ctx)
    {
        return assoc_type_expr;
    }

    // Collect static methods from all protocol bounds
    let mut candidates: Vec<StaticMethodCandidate> = Vec::new();
    let mut bound_names: Vec<String> = Vec::new();

    for bound in &bounds {
        if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Collect static methods from this protocol
            collect_protocol_static_methods(proto, member_name, &type_param_ty, &mut candidates);
        }
    }

    if candidates.is_empty() {
        // No static method found with that name in any bound
        let error = MethodNotInBoundsError {
            call_span: full_span.clone(),
            method_name: member_name.to_string(),
            type_param_name,
            bound_names,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Check for ambiguity - same method in multiple protocols
    let mut seen_protocols: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unique_candidates: Vec<&StaticMethodCandidate> = candidates
        .iter()
        .filter(|c| seen_protocols.insert(c.protocol_name.clone()))
        .collect();

    if unique_candidates.len() > 1 {
        // Multiple protocols have the same static method
        let protocol_names: Vec<String> = unique_candidates
            .iter()
            .map(|c| c.protocol_name.clone())
            .collect();
        let definition_spans: Vec<(String, Span)> = unique_candidates
            .iter()
            .map(|c| (c.protocol_name.clone(), c.definition_span.clone()))
            .collect();

        let error = AmbiguousConstrainedMethodError {
            call_span: full_span.clone(),
            method_name: member_name.to_string(),
            protocol_names,
            definition_spans,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Single method found - create method reference
    // Note: If there are multiple overloads in the same protocol, we return all of them
    let method_ids: Vec<SymbolId> = candidates.iter().map(|c| c.method_id).collect();

    // Use the type parameter type for the receiver so Self substitution works correctly
    Expression::method_ref(
        Expression::type_parameter_ref(
            symbol_id,
            type_param_ty.clone(),
            Span::new(full_span.file_id, full_span.start..full_span.start),
        ),
        method_ids,
        member_name.to_string(),
        full_span,
    )
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

/// Resolve member access on an associated type expression.
///
/// This handles chained associated type access like `T.Next.Next.baseValue()`.
/// When the base is an `AssociatedTypeRef`, we look at the associated type's bounds
/// to find either another associated type or a static method.
fn resolve_associated_type_member_access(
    base: &Expression,
    member_name: &str,
    _member_span: Span,
    full_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Extract the associated type symbol and container from base.ty
    let TyKind::AssociatedType {
        symbol: assoc_type,
        container,
    } = base.ty.kind()
    else {
        // Should not happen - AssociatedTypeRef should always have AssociatedType ty
        return Expression::error(full_span);
    };

    // Get the bounds of the associated type (e.g., `type Next: Level2` has bounds [Level2])
    let Some(bounds) = assoc_type.bounds() else {
        // No bounds - cannot access members on unconstrained associated type
        // This is similar to an unconstrained type parameter
        let error = UnconstrainedTypeParameterMemberError {
            span: full_span.clone(),
            member_name: member_name.to_string(),
            type_param_name: assoc_type.metadata().name().value.clone(),
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    };

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
    // e.g., for T.Next.Next, the container for the second Next is T.Next
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

    // Collect static methods from all protocol bounds
    let mut candidates: Vec<StaticMethodCandidate> = Vec::new();
    let mut bound_names: Vec<String> = Vec::new();

    for bound in &bounds {
        if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Collect static methods from this protocol
            collect_protocol_static_methods(proto, member_name, &container_ty, &mut candidates);
        }
    }

    if candidates.is_empty() {
        // No static method found with that name in any bound
        let error = MethodNotInBoundsError {
            call_span: full_span.clone(),
            method_name: member_name.to_string(),
            type_param_name: assoc_type.metadata().name().value.clone(),
            bound_names,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Check for ambiguity - same method in multiple protocols
    let mut seen_protocols: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unique_candidates: Vec<&StaticMethodCandidate> = candidates
        .iter()
        .filter(|c| seen_protocols.insert(c.protocol_name.clone()))
        .collect();

    if unique_candidates.len() > 1 {
        // Multiple protocols have the same static method
        let protocol_names: Vec<String> = unique_candidates
            .iter()
            .map(|c| c.protocol_name.clone())
            .collect();
        let definition_spans: Vec<(String, Span)> = unique_candidates
            .iter()
            .map(|c| (c.protocol_name.clone(), c.definition_span.clone()))
            .collect();

        let error = AmbiguousConstrainedMethodError {
            call_span: full_span.clone(),
            method_name: member_name.to_string(),
            protocol_names,
            definition_spans,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
    }

    // Single method found - create method reference
    let method_ids: Vec<SymbolId> = candidates.iter().map(|c| c.method_id).collect();

    // Clone the base expression for the receiver
    Expression::method_ref(base.clone(), method_ids, member_name.to_string(), full_span)
}

/// Candidate for static method resolution on type parameter
struct StaticMethodCandidate {
    method_id: SymbolId,
    protocol_name: String,
    definition_span: Span,
}

/// Collect static methods from a protocol, including inherited protocols.
fn collect_protocol_static_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    _self_replacement: &Ty,
    candidates: &mut Vec<StaticMethodCandidate>,
) {
    // Use flattened behavior if available (normal case after BIND phase)
    if let Some(flattened) = protocol
        .metadata()
        .get_behavior::<FlattenedProtocolBehavior>()
    {
        if let Some(methods) = flattened.methods().get(method_name) {
            for method in methods {
                if let Some(callable) = get_callable_behavior(&method.symbol) {
                    // Check if it's a static method (no receiver)
                    if callable.is_static() {
                        candidates.push(StaticMethodCandidate {
                            method_id: method.symbol.metadata().id(),
                            protocol_name: method.source_protocol_name.clone(),
                            definition_span: method.definition_span.clone(),
                        });
                    }
                }
            }
        }
        return;
    }

    // FALLBACK: Recursive traversal (for BUILD phase or if flattening failed)
    let protocol_name = protocol.metadata().name().value.clone();

    // Get all static methods with the given name from this protocol
    for child in protocol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Function
            && child.metadata().name().value == method_name
            && let Some(callable) = get_callable_behavior(&child)
        {
            // Check if it's a static method (no receiver)
            if callable.is_static() {
                candidates.push(StaticMethodCandidate {
                    method_id: child.metadata().id(),
                    protocol_name: protocol_name.clone(),
                    definition_span: child.metadata().span().clone(),
                });
            }
        }
    }

    // Search inherited protocols
    if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
        for parent_proto_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol: parent, .. } = parent_proto_ty.kind() {
                collect_protocol_static_methods(parent, method_name, _self_replacement, candidates);
            }
        }
    }
}

/// Filter extensions to find those applicable to the given type instance.
///
/// Returns extensions sorted by specificity (most specific first).
/// An extension is applicable if:
/// 1. Its type arguments can be unified with the actual type's arguments
/// 2. Any where clause constraints are satisfied
///
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
            let where_clause = target_behavior.where_clause();
            for constraint in where_clause.constraints() {
                if let Some(param_id) = constraint.type_parameter_id() {
                    // Get the actual type for this parameter
                    if let Some(actual_type) = param_to_actual.get(&param_id) {
                        // Check each bound is satisfied
                        for bound in constraint.bounds() {
                            if !type_satisfies_bound(actual_type, bound, ctx.model) {
                                return None; // Constraint not satisfied
                            }
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

/// Get all protocols that a concrete type conforms to (including through extensions).
///
/// Returns a list of (protocol_symbol, protocol_type) pairs.
fn get_type_conformances(ty: &Ty, ctx: &BodyResolutionContext) -> Vec<(Arc<ProtocolSymbol>, Ty)> {
    let mut conformances = Vec::new();

    match ty.kind() {
        TyKind::Struct { symbol, .. } => {
            // Direct conformances on the struct
            if let Some(conf_behavior) = symbol.metadata().get_behavior::<ConformancesBehavior>() {
                for conf_ty in conf_behavior.conformances() {
                    if let TyKind::Protocol { symbol: proto, .. } = conf_ty.kind() {
                        conformances.push((proto.clone(), conf_ty.clone()));
                    }
                }
            }

            // Conformances added via type extensions
            let struct_id = symbol.metadata().id();
            let extensions = ctx.model.query(ExtensionsFor {
                target_id: struct_id,
            });
            for extension in extensions {
                // Skip protocol extensions when collecting type conformances
                if let Some(target) = extension
                    .metadata()
                    .get_behavior::<ExtensionTargetBehavior>()
                    && target.is_protocol_extension()
                {
                    continue;
                }
                if let Some(conf_behavior) =
                    extension.metadata().get_behavior::<ConformancesBehavior>()
                {
                    for conf_ty in conf_behavior.conformances() {
                        if let TyKind::Protocol { symbol: proto, .. } = conf_ty.kind() {
                            conformances.push((proto.clone(), conf_ty.clone()));
                        }
                    }
                }
            }
        },
        TyKind::Enum { symbol, .. } => {
            // Direct conformances on the enum
            if let Some(conf_behavior) = symbol.metadata().get_behavior::<ConformancesBehavior>() {
                for conf_ty in conf_behavior.conformances() {
                    if let TyKind::Protocol { symbol: proto, .. } = conf_ty.kind() {
                        conformances.push((proto.clone(), conf_ty.clone()));
                    }
                }
            }

            // Conformances added via type extensions
            let enum_id = symbol.metadata().id();
            let extensions = ctx.model.query(ExtensionsFor { target_id: enum_id });
            for extension in extensions {
                if let Some(target) = extension
                    .metadata()
                    .get_behavior::<ExtensionTargetBehavior>()
                    && target.is_protocol_extension()
                {
                    continue;
                }
                if let Some(conf_behavior) =
                    extension.metadata().get_behavior::<ConformancesBehavior>()
                {
                    for conf_ty in conf_behavior.conformances() {
                        if let TyKind::Protocol { symbol: proto, .. } = conf_ty.kind() {
                            conformances.push((proto.clone(), conf_ty.clone()));
                        }
                    }
                }
            }
        },
        _ => {},
    }

    conformances
}

/// Get all applicable protocol extensions for a concrete type.
///
/// This finds all protocol extensions where:
/// 1. The concrete type conforms to the target protocol
/// 2. All SelfBound constraints are satisfied
fn get_applicable_protocol_extensions(
    concrete_ty: &Ty,
    ctx: &BodyResolutionContext,
) -> Vec<(
    Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>,
    usize,
)> {
    let conformances = get_type_conformances(concrete_ty, ctx);
    let mut applicable = Vec::new();

    for (protocol, _protocol_ty) in conformances {
        // Get all extensions for this protocol
        let protocol_id = protocol.metadata().id();
        let extensions = ctx.model.query(ExtensionsFor {
            target_id: protocol_id,
        });

        for extension in extensions {
            // Check if this is actually a protocol extension
            let target_behavior = match extension
                .metadata()
                .get_behavior::<ExtensionTargetBehavior>()
            {
                Some(b) => b,
                None => continue,
            };

            if !target_behavior.is_protocol_extension() {
                continue;
            }

            // Check SelfBound constraints
            if is_protocol_extension_applicable(&extension, concrete_ty, ctx) {
                let specificity = target_behavior.protocol_extension_specificity();
                applicable.push((extension, specificity));
            }
        }
    }

    // Sort by specificity (most specific first)
    applicable.sort_by_key(|(_, specificity)| std::cmp::Reverse(*specificity));

    applicable
}

/// Check if a protocol extension is applicable to a concrete type.
///
/// This checks that all SelfBound constraints in the extension's where clause are satisfied.
fn is_protocol_extension_applicable(
    extension: &Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>,
    concrete_ty: &Ty,
    ctx: &BodyResolutionContext,
) -> bool {
    use kestrel_semantic_tree::ty::Constraint;

    let target_behavior = match extension
        .metadata()
        .get_behavior::<ExtensionTargetBehavior>()
    {
        Some(b) => b,
        None => return false,
    };

    let where_clause = target_behavior.where_clause();

    for constraint in where_clause.constraints() {
        if let Constraint::SelfBound {
            associated_type_path,
            bounds,
            ..
        } = constraint
            && associated_type_path.is_empty()
        {
            // Self: Protocol - check if concrete type conforms to all bounds
            for bound in bounds {
                if !type_satisfies_bound(concrete_ty, bound, ctx.model) {
                    return false;
                }
            }
        }
        // Self.Item: Protocol - resolve associated type and check bounds
        // For now, we don't fully support this - requires associated type resolution
        // TODO: Implement Self.AssociatedType constraint checking
        // For now, skip these constraints (they'll be checked at call site)
        // Other constraint types shouldn't appear in protocol extensions
    }

    true
}

/// Find a method in protocol extensions for a given concrete type.
///
/// Returns a list of method SymbolIds from applicable protocol extensions.
/// Only methods from the most specific (highest specificity) extensions are returned.
/// If multiple extensions at the same specificity provide the method, all are returned
/// (ambiguity is detected at call resolution time based on signature matching).
fn find_methods_in_protocol_extensions(
    concrete_ty: &Ty,
    method_name: &str,
    ctx: &BodyResolutionContext,
) -> Vec<SymbolId> {
    let applicable_extensions = get_applicable_protocol_extensions(concrete_ty, ctx);

    if applicable_extensions.is_empty() {
        return Vec::new();
    }

    // Extensions are sorted by specificity (highest first)
    // Determine the highest specificity
    let highest_specificity = applicable_extensions[0].1;

    let mut methods = Vec::new();

    // Only collect methods from extensions at the highest specificity level
    for (extension, specificity) in applicable_extensions {
        if specificity < highest_specificity {
            // We've passed all extensions at the highest specificity level
            break;
        }

        for child in extension.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Function
                && child.metadata().name().value == method_name
            {
                methods.push(child.metadata().id());
            }
        }
    }

    methods
}

/// Get methods from Self constraint protocols when inside a protocol extension.
///
/// When a protocol extension has `where Self: OtherProtocol`, methods from
/// `OtherProtocol` should be accessible on `self` within the extension body.
///
/// Returns a list of method symbols from constraint protocols.
fn get_methods_from_self_constraints(
    method_name: &str,
    ctx: &BodyResolutionContext,
) -> Vec<Arc<dyn Symbol<KestrelLanguage>>> {
    use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
    use kestrel_semantic_tree::ty::Constraint;

    let mut methods = Vec::new();

    // Get the current function
    let Some(function) = ctx.model.query(SymbolFor {
        id: ctx.function_id,
    }) else {
        return methods;
    };

    // Get the parent (should be an extension for protocol extensions)
    let Some(parent) = function.metadata().parent() else {
        return methods;
    };

    // Check if we're in a protocol extension
    if parent.metadata().kind() != KestrelSymbolKind::Extension {
        return methods;
    }

    // Get the ExtensionTargetBehavior to check if this is a protocol extension
    let Some(target_beh) = parent.metadata().get_behavior::<ExtensionTargetBehavior>() else {
        return methods;
    };

    if !target_beh.is_protocol_extension() {
        return methods;
    }

    // Get the where clause from the extension's target behavior
    let where_clause = target_beh.where_clause();

    // Look for SelfBound constraints (Self: Protocol)
    for constraint in where_clause.constraints() {
        if let Constraint::SelfBound { bounds, .. } = constraint {
            // Each bound should be a protocol
            for bound in bounds {
                if let TyKind::Protocol { symbol, .. } = bound.kind() {
                    // Search this protocol for the method
                    for child in symbol.metadata().children() {
                        if child.metadata().kind() == KestrelSymbolKind::Function
                            && child.metadata().name().value == method_name
                        {
                            methods.push(child);
                        }
                    }
                }
            }
        }
    }

    methods
}
