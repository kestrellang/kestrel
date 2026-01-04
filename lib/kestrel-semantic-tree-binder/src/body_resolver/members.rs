//! Member access resolution.
//!
//! This module handles resolving member access expressions (field access, method calls)
//! including visibility checking, member chain resolution, and constraint enforcement
//! for type parameters.

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::{ExtensionsFor, IsVisibleFrom, SymbolFor};
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression, PrimitiveMethod};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::FlattenedProtocolBehavior;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{
    AmbiguousConstrainedMethodError, CannotAccessMemberOnTypeError, MemberNotAccessibleError,
    MemberNotVisibleError, MethodNotInBoundsError, NoMatchingMethodError, NoSuchMemberError,
    NoSuchMethodError, PrimitiveMethodNotCallableError, UnconstrainedTypeParameterMemberError,
    UnsupportedGenericProtocolBoundError,
};

use super::calls::{collect_overload_descriptions, validate_argument_access_modes};
use super::context::BodyResolutionContext;
use super::utils::{
    format_symbol_kind, get_callable_behavior, get_type_container,
    get_type_parameter_bounds_by_id, get_type_parameter_bounds_from_context, infer_type_arguments,
    matches_signature, substitute_self, substitute_type,
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
    base: Expression,
    member_name: &str,
    member_span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let base_span = base.span.clone();
    let base_ty = &base.ty;
    let full_span = Span::from(base_span.start..member_span.end);

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
    // Primitive methods can only be called, not used as first-class values
    if let Some(primitive_method) = PrimitiveMethod::lookup(base_ty, member_name) {
        // Primitive methods cannot be used as first-class values.
        // Report an error - they must be called directly.
        let error = PrimitiveMethodNotCallableError {
            span: full_span.clone(),
            method_name: primitive_method.name().to_string(),
            receiver_type: base_ty.to_string(),
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(full_span);
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

    // 3. Get container from base type
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
        }
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

    // If not found in direct children, search extensions
    let member = match member {
        Some(m) => m,
        None => {
            // Try to find in applicable extensions
            let extension_member = applicable_extensions
                .iter()
                .flat_map(|ext| ext.metadata().children())
                .find(|child| child.metadata().name().value == member_name);

            match extension_member {
                Some(m) => m,
                None => {
                    let error = NoSuchMemberError {
                        member_span,
                        member_name: member_name.to_string(),
                        base_span,
                        base_type: base_ty.to_string(),
                    };
                    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                    return Expression::error(full_span.clone());
                }
            }
        }
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

    // 4. Get MemberAccessBehavior and produce expression
    for behavior in member.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::MemberAccess {
            if let Some(access) = behavior.as_ref().downcast_ref::<MemberAccessBehavior>() {
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

        // Also collect methods from applicable extensions
        for extension in &applicable_extensions {
            for child in extension.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::Function
                    && child.metadata().name().value == member_name
                {
                    candidates.push(child.metadata().id());
                }
            }
        }

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

            // Check for generic protocol (not yet supported)
            if !proto.type_parameters().is_empty() {
                let error = UnsupportedGenericProtocolBoundError {
                    span: bound.span().clone(),
                    protocol_name: proto_name,
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                return Expression::error(full_span);
            }

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
        }
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

    if methods.is_empty() {
        // Report error: no such method
        let error = NoSuchMethodError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            receiver_type: base_ty.to_string(),
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Find matching overload
    let mut invisible_matches = Vec::new();

    for method in &methods {
        if let Some(callable) = get_callable_behavior(method) {
            if matches_signature(&callable, arguments.len(), arg_labels) {
                // Check visibility
                let method_id = method.metadata().id();
                if !ctx.model.query(IsVisibleFrom {
                    target: method_id,
                    context: ctx.function_id,
                }) {
                    invisible_matches.push(method.clone());
                    continue;
                }

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
                        && substitutions.iter().all(|(_, ty)| !matches!(ty.kind(), TyKind::TypeParameter(_)));
                    
                    if has_concrete_subs {
                        // Base type has concrete type args (e.g., Option[Int].Some)
                        return_ty = return_ty.apply_substitutions(substitutions);
                    } else {
                        // Base type has no concrete type args - infer from arguments
                        // e.g., Option.Some(value: 42) should infer T = Int from the argument
                        let type_params = enum_sym.type_parameters();
                        if !type_params.is_empty() {
                            let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
                            let inferred_subs = infer_type_arguments(&type_params, &callable, &arg_types);
                            return_ty = substitute_type(&return_ty, &inferred_subs);
                        }
                    }
                }

                // Validate access modes for arguments
                validate_argument_access_modes(&callable, &arguments, &span, ctx);

                // Create method ref and then call
                let method_ref = Expression::method_ref(
                    object.clone(),
                    vec![method_id],
                    member_name.to_string(),
                    span.clone(),
                );

                return Expression::call(method_ref, arguments, return_ty, span);
            }
        }
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
        if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Check for generic protocol (not yet supported)
            if !proto.type_parameters().is_empty() {
                let error = UnsupportedGenericProtocolBoundError {
                    span: bound.span().clone(),
                    protocol_name: proto_name,
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic());
                return Expression::error(span);
            }

            // Collect methods from this protocol (including inherited)
            collect_protocol_methods(proto, member_name, receiver_ty, &mut candidates, ctx);
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
    let return_ty = winner.callable.return_type().clone();
    let method_id = winner.method.metadata().id();

    // Validate access modes for arguments
    validate_argument_access_modes(&winner.callable, &arguments, &span, ctx);

    // Create method ref and call
    let method_ref = Expression::method_ref(
        object.clone(),
        vec![method_id],
        member_name.to_string(),
        span.clone(),
    );

    Expression::call(method_ref, arguments, return_ty, span)
}

/// Collect methods from a protocol, including inherited protocols.
fn collect_protocol_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    receiver_ty: &Ty,
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
        {
            if let Some(callable) = get_callable_behavior(&child) {
                // Substitute Self with the receiver type
                let substituted_callable = substitute_callable_self(&callable, receiver_ty);

                candidates.push(ConstrainedMethodCandidate {
                    method: child.clone(),
                    callable: substituted_callable,
                    protocol_name: proto_name.clone(),
                    definition_span: child.metadata().name().span.clone(),
                });
            }
        }
    }

    // Search inherited protocols
    if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
        for parent_proto_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol: parent, .. } = parent_proto_ty.kind() {
                collect_protocol_methods(parent, method_name, receiver_ty, candidates, ctx);
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
    if let Some(assoc_type_expr) = find_associated_type_in_bounds(
        &bounds,
        member_name,
        &type_param_ty,
        full_span.clone(),
        ctx,
    ) {
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
            Span::from(full_span.start..full_span.start),
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
        if let TyKind::Protocol { symbol: protocol, .. } = bound.kind() {
            // Check direct children of protocol for associated types
            let protocol_dyn = protocol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == member_name
                {
                    // Found an associated type - create a qualified associated type
                    if let Some(symbol) = ctx.model.query(SymbolFor {
                        id: child.metadata().id(),
                    }) {
                        if let Ok(assoc_type_arc) =
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
            }

            // Check inherited protocols (via FlattenedProtocolBehavior)
            if let Some(flattened) = protocol.metadata().get_behavior::<FlattenedProtocolBehavior>() {
                if let Some(flattened_assoc) = flattened.associated_types().get(member_name) {
                    let qualified_ty = Ty::qualified_associated_type(
                        flattened_assoc.symbol.clone(),
                        container_ty.clone(),
                        span.clone(),
                    );
                    return Some(Expression::associated_type_ref(qualified_ty, span));
                }
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
    let TyKind::AssociatedType { symbol: assoc_type, container } = base.ty.kind() else {
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
        Some(c) => Ty::qualified_associated_type(assoc_type.clone(), (**c).clone(), full_span.clone()),
        None => base.ty.clone(),
    };

    // First, check if member_name is an associated type in any protocol bound
    if let Some(assoc_type_expr) = find_associated_type_in_bounds(
        &bounds,
        member_name,
        &container_ty,
        full_span.clone(),
        ctx,
    ) {
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
    Expression::method_ref(
        base.clone(),
        method_ids,
        member_name.to_string(),
        full_span,
    )
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
        {
            if let Some(callable) = get_callable_behavior(&child) {
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
fn filter_applicable_extensions(
    extensions: Vec<Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>>,
    actual_ty: &Ty,
    ctx: &BodyResolutionContext,
) -> Vec<Arc<kestrel_semantic_tree::symbol::extension::ExtensionSymbol>> {
    use super::utils::type_satisfies_bound;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;

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
        }

        // Type parameters - compare by symbol ID
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        }

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
            }) {
                if let Some(parent) = function.metadata().parent() {
                    match parent.metadata().kind() {
                        KestrelSymbolKind::Extension => {
                            // For extension methods, get the target type from ExtensionTargetBehavior
                            // This gives us the type with substitutions (e.g., Box[Int] not Box[T])
                            if let Some(target_beh) =
                                parent.metadata().get_behavior::<ExtensionTargetBehavior>()
                            {
                                let target_ty = target_beh.target_type();
                                // Make sure target type isn't also SelfType (should never happen, but prevent infinite recursion)
                                if !matches!(target_ty.kind(), TyKind::SelfType) {
                                    return target_ty.clone();
                                }
                            }
                        }
                        KestrelSymbolKind::Struct => {
                            // For struct methods, resolve Self to the concrete struct type
                            if let Some(typed) = parent.metadata().get_behavior::<TypedBehavior>() {
                                let struct_ty = typed.ty();
                                if !matches!(struct_ty.kind(), TyKind::SelfType) {
                                    return struct_ty.clone();
                                }
                            }
                        }
                        KestrelSymbolKind::Protocol => {
                            // For protocol methods, Self remains abstract
                            // (needed for future default impl support)
                        }
                        _ => {}
                    }
                }
            }
            ty.clone()
        }
        _ => ty.clone(),
    }
}
