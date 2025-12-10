//! Member access resolution.
//!
//! This module handles resolving member access expressions (field access, method calls)
//! including visibility checking, member chain resolution, and constraint enforcement
//! for type parameters.

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression, PrimitiveMethod};
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::diagnostics::{
    AmbiguousConstrainedMethodError, CannotAccessMemberOnTypeError, MemberNotAccessibleError,
    MemberNotVisibleError, MethodNotInBoundsError, NoMatchingMethodError, NoSuchMemberError,
    NoSuchMethodError, PrimitiveMethodNotCallableError, UnconstrainedTypeParameterMemberError,
    UnsupportedGenericProtocolBoundError,
};
use crate::resolution::visibility::is_visible_from;

use super::calls::collect_overload_descriptions;
use super::context::BodyResolutionContext;
use super::utils::{
    format_symbol_kind, format_type, get_callable_behavior, get_type_container,
    get_type_parameter_bounds_by_id, get_type_parameter_bounds_from_context,
    matches_signature, substitute_self,
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
    let full_span = base_span.start..member_span.end;

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

    // 1. Check for primitive method (e.g., 5.toString, "hello".length)
    // Primitive methods can only be called, not used as first-class values
    if let Some(primitive_method) = PrimitiveMethod::lookup(base_ty, member_name) {
        // Primitive methods cannot be used as first-class values.
        // Report an error - they must be called directly.
        let error = PrimitiveMethodNotCallableError {
            span: full_span.clone(),
            method_name: primitive_method.name().to_string(),
            receiver_type: format_type(base_ty),
        };
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
            full_span,
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
                base_type: format_type(base_ty),
            };
            ctx.diagnostics
                .add_diagnostic(error.into_diagnostic(ctx.file_id));
            return Expression::error(full_span);
        }
    };

    // 2. Find child with that name
    let member = container
        .metadata()
        .children()
        .into_iter()
        .find(|c| c.metadata().name().value == member_name);

    let member = match member {
        Some(m) => m,
        None => {
            let error = NoSuchMemberError {
                member_span,
                member_name: member_name.to_string(),
                base_span,
                base_type: format_type(base_ty),
            };
            ctx.diagnostics
                .add_diagnostic(error.into_diagnostic(ctx.file_id));
            return Expression::error(full_span);
        }
    };

    // 3. Check visibility
    let context_symbol = ctx.db.symbol_by_id(ctx.function_id);
    if let Some(ref context_sym) = context_symbol {
        if !is_visible_from(&member, context_sym) {
            use kestrel_semantic_tree::behavior::visibility::Visibility;
            use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;

            let visibility = member
                .visibility_behavior()
                .and_then(|v| v.visibility().cloned())
                .unwrap_or(Visibility::Internal);

            let error = MemberNotVisibleError {
                member_span,
                member_name: member_name.to_string(),
                base_span,
                base_type: format_type(base_ty),
                visibility: visibility.to_string(),
            };
            ctx.diagnostics
                .add_diagnostic(error.into_diagnostic(ctx.file_id));
            return Expression::error(full_span);
        }
    }

    // 4. Get MemberAccessBehavior and produce expression
    for behavior in member.metadata().behaviors() {
        if behavior.kind() == KestrelBehaviorKind::MemberAccess {
            if let Some(access) = behavior.as_ref().downcast_ref::<MemberAccessBehavior>() {
                return access.access(base, full_span);
            }
        }
    }

    // 5. If it's a function, create a MethodRef (for method calls like obj.method())
    if member.metadata().kind() == KestrelSymbolKind::Function {
        // Find all methods with this name (for overloads)
        let candidates: Vec<SymbolId> = container
            .metadata()
            .children()
            .into_iter()
            .filter(|c| {
                c.metadata().kind() == KestrelSymbolKind::Function
                    && c.metadata().name().value == member_name
            })
            .map(|c| c.metadata().id())
            .collect();

        return Expression::method_ref(base, candidates, member_name.to_string(), full_span);
    }

    // Member exists but doesn't have MemberAccessBehavior (e.g., type alias, nested type)
    let error = MemberNotAccessibleError {
        member_span,
        member_name: member_name.to_string(),
        base_span,
        base_type: format_type(base_ty),
        member_kind: format_symbol_kind(member.metadata().kind()),
    };
    ctx.diagnostics
        .add_diagnostic(error.into_diagnostic(ctx.file_id));
    Expression::error(full_span)
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
                ctx.diagnostics
                    .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(full_span);
    }

    // Check for ambiguity - multiple distinct protocols have a method with this name
    let mut unique_protocols: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for c in &candidates {
        unique_protocols.insert(&c.protocol_name);
    }

    if unique_protocols.len() > 1 {
        // Ambiguous - method found in multiple different protocols
        let protocol_names: Vec<String> = candidates.iter().map(|c| c.protocol_name.clone()).collect();
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
    if let Some(flattened) = protocol.flattened_protocol_behavior() {
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
    if let Some(conformances) = protocol.conformances_behavior() {
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
                receiver_type: format_type(base_ty),
            };
            ctx.diagnostics
                .add_diagnostic(error.into_diagnostic(ctx.file_id));
            return Expression::error(span);
        }
    };

    // Find method(s) with this name
    let methods: Vec<Arc<dyn Symbol<KestrelLanguage>>> = container
        .metadata()
        .children()
        .into_iter()
        .filter(|c| {
            c.metadata().kind() == KestrelSymbolKind::Function
                && c.metadata().name().value == member_name
        })
        .collect();

    if methods.is_empty() {
        // Report error: no such method
        let error = NoSuchMethodError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            receiver_type: format_type(base_ty),
        };
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Find matching overload
    for method in &methods {
        if let Some(callable) = get_callable_behavior(method) {
            if matches_signature(&callable, arguments.len(), arg_labels) {
                // Check visibility
                if let Some(context_sym) = ctx.db.symbol_by_id(ctx.function_id) {
                    if !is_visible_from(method, &context_sym) {
                        // TODO: Report error: method not visible
                        continue;
                    }
                }

                let return_ty = callable.return_type().clone();
                let method_id = method.metadata().id();

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

    // No matching method found - collect overload info for error message
    let receiver_type = format_type(base_ty);
    let method_ids: Vec<SymbolId> = methods.iter().map(|m| m.metadata().id()).collect();
    let available_overloads = collect_overload_descriptions(&method_ids, ctx.db);

    let error = NoMatchingMethodError {
        call_span: span.clone(),
        method_name: member_name.to_string(),
        receiver_type,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_overloads,
    };
    ctx.diagnostics
        .add_diagnostic(error.into_diagnostic(ctx.file_id));

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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
                ctx.diagnostics
                    .add_diagnostic(error.into_diagnostic(ctx.file_id));
                return Expression::error(span);
            }

            // Collect methods from this protocol (including inherited)
            collect_protocol_methods(
                proto,
                member_name,
                receiver_ty,
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Find matching candidates by signature
    let matching: Vec<&ConstrainedMethodCandidate> = candidates
        .iter()
        .filter(|c| matches_signature(&c.callable, arguments.len(), arg_labels))
        .collect();

    if matching.is_empty() {
        // No matching signature - report error with available overloads
        let method_ids: Vec<SymbolId> = candidates.iter().map(|c| c.method.metadata().id()).collect();
        let available_overloads = collect_overload_descriptions(&method_ids, ctx.db);

        let error = NoMatchingMethodError {
            call_span: span.clone(),
            method_name: member_name.to_string(),
            receiver_type: type_param_name,
            provided_labels: arg_labels.to_vec(),
            provided_arity: arguments.len(),
            available_overloads,
        };
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
        let protocol_names: Vec<String> = unique_matching.iter().map(|c| c.protocol_name.clone()).collect();
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    let matching = unique_matching;

    // Single matching method found
    let winner = matching[0];
    let return_ty = winner.callable.return_type().clone();
    let method_id = winner.method.metadata().id();

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
    if let Some(flattened) = protocol.flattened_protocol_behavior() {
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
    if let Some(conformances) = protocol.conformances_behavior() {
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
    let Some(symbol) = ctx.db.symbol_by_id(symbol_id) else {
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(full_span);
    }

    // For Self substitution - use the type parameter type so T.create() returns T, not _
    let type_param_ty = Ty::type_parameter(type_param_arc, full_span.clone());

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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(full_span);
    }

    // Single method found - create method reference
    // Note: If there are multiple overloads in the same protocol, we return all of them
    let method_ids: Vec<SymbolId> = candidates.iter().map(|c| c.method_id).collect();

    // Use the type parameter type for the receiver so Self substitution works correctly
    Expression::method_ref(
        Expression::type_parameter_ref(symbol_id, type_param_ty.clone(), full_span.start..full_span.start),
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

/// Collect static methods from a protocol.
fn collect_protocol_static_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    _self_replacement: &Ty,
    candidates: &mut Vec<StaticMethodCandidate>,
) {
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

    // TODO: Also collect from inherited protocols
}
