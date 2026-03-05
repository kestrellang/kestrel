//! Call expression resolution.
//!
//! This module handles resolving function calls, method calls, overloaded calls,
//! and struct instantiation (both explicit and implicit initializers).

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::queries::ExtensionsFor;
use kestrel_semantic_model::{IsVisibleFrom, SymbolFor};
use kestrel_semantic_tree::behavior::callable::ParameterAccessMode;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::subscript::SubscriptBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::subscript::SubscriptSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Substitutions, Ty, TyKind};
use kestrel_semantic_type_inference::TypeOracle;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolution::type_resolver::TypeResolver;

use crate::diagnostics::{
    AmbiguousTypeParameterInitError, CannotMutateThroughImmutableBindingError,
    CannotPassImmutableFieldToMutatingError, CannotPassLetToMutatingError,
    CannotPassTemporaryToMutatingError, ClosureArityError,
    FieldNotVisibleForInitError, ImplicitInitArityError, ImplicitInitLabelError,
    NoInitInTypeParameterBoundsError,
    NoMatchingTypeParameterInitError, NonCallableError, NotGenericError, OverloadDescription,
    PrimitiveMethodArityError, TooFewTypeArgumentsError,
    TypeArgsOnNonGenericError, UnconstrainedTypeParameterMemberError,
};
use kestrel_syntax_tree::utils::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::members::{
    filter_applicable_extensions, resolve_delegating_init,
    resolve_member_call, resolve_self_type_to_concrete,
};
use super::utils::{
    create_generic_struct_type, create_struct_type, create_struct_type_with_type_args,
    get_callable_behavior,
    get_type_container, get_type_parameter_bounds_by_id, infer_type_arguments, is_expression_kind,
    matches_signature, resolve_associated_types,
    validate_not_standalone_type_param,
    verify_type_argument_constraints,
};


/// Resolve a call expression: callee(arg1, arg2, ...) or callee[T](arg1, ...)
pub fn resolve_call_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Find the callee expression (first child that's an Expression)
    let callee_node = match node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))
    {
        Some(n) => n,
        None => return Expression::error(span.clone()),
    };

    // Extract explicit type arguments from the callee (e.g., foo[Int] or obj.method[T])
    let explicit_type_args = extract_type_arguments_from_callee(&callee_node, ctx);

    // Find the argument list
    let arg_list_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ArgumentList);

    // Check for delegating initializer pattern: self.init(...)
    // We must detect this BEFORE resolving the callee, because self.init as a member
    // access will fail (init is a keyword/initializer, not a regular member).
    if is_self_init_call(&callee_node) {
        // Parse arguments first
        let arguments = if let Some(ref arg_list) = arg_list_node {
            resolve_argument_list(arg_list, ctx)
        } else {
            vec![]
        };
        let arg_labels: Vec<Option<String>> = arguments.iter().map(|a| a.label.clone()).collect();

        // Validate we're inside an initializer
        if let Some(init_sym) = ctx.model.query(SymbolFor {
            id: ctx.function_id,
        }) && init_sym.metadata().kind() == KestrelSymbolKind::Initializer
        {
            return resolve_delegating_init(&init_sym, arguments, &arg_labels, span, ctx);
        }

        // Not in an initializer - emit error
        use crate::diagnostics::DelegatingInitOutsideInitializerError;
        ctx.diagnostics.add_diagnostic(
            DelegatingInitOutsideInitializerError { span: span.clone() }.into_diagnostic(),
        );
        return Expression::error(span);
    }

    // Resolve callee first
    let callee = resolve_expression(&callee_node, ctx);

    // Parse arguments
    let arguments = if let Some(arg_list) = arg_list_node {
        resolve_argument_list(&arg_list, ctx)
    } else {
        vec![]
    };

    // Get labels for overload resolution (owned strings)
    let arg_labels: Vec<Option<String>> = arguments.iter().map(|a| a.label.clone()).collect();

    // Now resolve based on callee type
    resolve_call(
        callee,
        arguments,
        &arg_labels,
        explicit_type_args,
        span,
        ctx,
    )
}

/// Extract type arguments from a callee expression node.
/// Handles: foo[T], path.to.func[T, U], obj.method[T]
///
/// IMPORTANT: Only extracts type arguments from the FINAL segment of the path.
/// For `Box[Int].zero()`, the type args `[Int]` belong to `Box` (handled during path
/// resolution), not to `zero` (the actual function being called). So we should NOT
/// extract type args here.
fn extract_type_arguments_from_callee(
    callee_node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Vec<Ty>> {
    // Look for TypeArgumentList only in the FINAL segment of the callee
    // The callee is typically an ExprPath that may contain TypeArgumentList
    //
    // ExprPath structure for "Box[Int].zero":
    // ExprPath
    //   Identifier "Box"
    //   TypeArgumentList [Int]
    //   Dot
    //   Identifier "zero"
    //
    // We want to only extract type args that come AFTER the last dot (i.e., on the final segment).
    fn find_type_args_on_final_segment(node: &SyntaxNode) -> Option<SyntaxNode> {
        // Find the ExprPath node
        let expr_path = if node.kind() == SyntaxKind::ExprPath {
            Some(node.clone())
        } else {
            node.children().find(|c| c.kind() == SyntaxKind::ExprPath)
        };

        if let Some(expr_path) = expr_path {
            // Collect all children to analyze the structure
            let children: Vec<_> = expr_path.children_with_tokens().collect();

            // Find the last Dot token position (if any)
            let mut last_dot_pos = None;
            for (i, child) in children.iter().enumerate() {
                if let Some(token) = child.as_token()
                    && token.kind() == SyntaxKind::Dot
                {
                    last_dot_pos = Some(i);
                }
            }

            // If there's a dot, only look for TypeArgumentList AFTER the last dot
            if let Some(dot_pos) = last_dot_pos {
                for child in children.iter().skip(dot_pos + 1) {
                    if let Some(node) = child.as_node()
                        && node.kind() == SyntaxKind::TypeArgumentList
                    {
                        return Some(node.clone());
                    }
                }
                // Multi-segment path but no type args after last dot
                return None;
            }

            // No dot - single segment path, check for direct TypeArgumentList
            for child in children.iter() {
                if let Some(node) = child.as_node()
                    && node.kind() == SyntaxKind::TypeArgumentList
                {
                    return Some(node.clone());
                }
            }

            return None;
        }

        // For Path nodes (used in type paths), check PathElements
        if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
            let path_elements: Vec<_> = path_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::PathElement)
                .collect();

            // For multi-segment paths, only extract type args from the LAST element
            if path_elements.len() > 1 {
                if let Some(last_element) = path_elements.last() {
                    for child in last_element.children() {
                        if child.kind() == SyntaxKind::TypeArgumentList {
                            return Some(child);
                        }
                    }
                }
                return None;
            }

            // Single element path
            if let Some(only_element) = path_elements.first() {
                for child in only_element.children() {
                    if child.kind() == SyntaxKind::TypeArgumentList {
                        return Some(child);
                    }
                }
            }
            return None;
        }

        // Fallback: direct TypeArgumentList child
        node.children()
            .find(|child| child.kind() == SyntaxKind::TypeArgumentList)
    }

    let type_arg_list = find_type_args_on_final_segment(callee_node)?;

    // Resolve each type in the TypeArgumentList
    let mut type_args = Vec::new();

    for child in type_arg_list.children() {
        if child.kind() == SyntaxKind::Ty {
            let mut resolver = TypeResolver::new(
                ctx.model,
                ctx.diagnostics,
                ctx.source,
                ctx.file_id,
                ctx.function_id,
            );
            let ty = resolver.resolve(&child);
            type_args.push(ty);
        }
    }

    // Return Some even if empty - the presence of [] means explicit type args were provided
    Some(type_args)
}

/// Resolve an argument list node into CallArguments
pub(crate) fn resolve_argument_list(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Vec<CallArgument> {
    let mut arguments = Vec::new();

    for child in node.children() {
        if child.kind() == SyntaxKind::Argument
            && let Some(arg) = resolve_argument(&child, ctx)
        {
            arguments.push(arg);
        }
    }

    arguments
}

/// Resolve a single argument node
fn resolve_argument(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Option<CallArgument> {
    let span = get_node_span(node, ctx.file_id);

    // Check for label (Identifier followed by Colon)
    let mut label = None;
    let mut has_colon = false;

    for elem in node.children_with_tokens() {
        if let Some(token) = elem.as_token() {
            if token.kind() == SyntaxKind::Identifier && !has_colon {
                // This might be a label
                label = Some(token.text().to_string());
            } else if token.kind() == SyntaxKind::Colon {
                has_colon = true;
            }
        }
    }

    // If we found a colon, the identifier was a label; otherwise it wasn't
    if !has_colon {
        label = None;
    }

    // Find the value expression
    // Also validate that it's not a standalone type parameter reference
    let value_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))?;

    let value = resolve_expression(&value_node, ctx);
    let value = validate_not_standalone_type_param(value, ctx);

    Some(CallArgument::unlabeled(value, span)).map(|mut arg| {
        if let Some(l) = label {
            arg.label = Some(l);
        }
        arg
    })
}

/// Resolve a call with the given callee, arguments, and optional explicit type arguments
pub fn resolve_call(
    callee: Expression,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    explicit_type_args: Option<Vec<Ty>>,
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // If the callee is already an error type, don't emit cascading diagnostics.
    // The original error has already been reported where the error type was created.
    if matches!(callee.ty.kind(), TyKind::Error) {
        // Create an error call expression with inferred return type
        return Expression::call(callee, arguments, Ty::infer(span.clone()), span);
    }

    // Clone callee.kind to avoid borrow issues
    let callee_kind = callee.kind.clone();
    let callee_ty = callee.ty.clone();


    match callee_kind {
        // Direct function/field reference
        ExprKind::SymbolRef(symbol_id) => {
            // Check if this is a field symbol - if so, and the field's type has subscripts,
            // try subscript resolution first. This handles cases like Foo.staticField(arg)
            // where staticField is a computed property returning a type with subscripts.
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id })
                && symbol.metadata().kind() == KestrelSymbolKind::Field
            {
                // Try subscript call on the field's type
                if let Some(subscript_expr) =
                    try_resolve_subscript_call(&callee, &arguments, arg_labels, &span, ctx)
                {
                    return subscript_expr;
                }
            }
            // Not a field, or field type has no subscripts - defer function call to type inference
            // Validate access modes and where clause constraints eagerly
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id })
                && let Some(callable) = get_callable_behavior(&symbol)
            {
                validate_argument_access_modes(&callable, &arguments, &span, ctx);

                // Eagerly validate explicit type args count and where clause constraints
                if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
                    let type_params = func_sym.type_parameters();

                    // Validate explicit type arg count
                    if let Some(ref type_args) = explicit_type_args {
                        if type_params.is_empty() {
                            let function_name = symbol.metadata().name().value.clone();
                            ctx.diagnostics.add_diagnostic(
                                NotGenericError {
                                    span: span.clone(),
                                    type_name: function_name,
                                }
                                .into_diagnostic(),
                            );
                        } else if type_args.len() < type_params.len() {
                            let function_name = symbol.metadata().name().value.clone();
                            ctx.diagnostics.add_diagnostic(
                                TooFewTypeArgumentsError {
                                    span: span.clone(),
                                    type_name: function_name,
                                    min_expected: type_params.len(),
                                    got: type_args.len(),
                                }
                                .into_diagnostic(),
                            );
                        }
                    }

                    if !type_params.is_empty() {
                        let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
                        let subs = infer_type_arguments(&type_params, &callable, &arg_types);
                        let inferred_args: Vec<Ty> = type_params
                            .iter()
                            .map(|tp| {
                                let tp_id = tp.metadata().id();
                                subs.get(tp_id).cloned().unwrap_or_else(|| Ty::infer(span.clone()))
                            })
                            .collect();
                        let where_clause = func_sym.where_clause();
                        verify_type_argument_constraints(
                            &type_params,
                            &inferred_args,
                            &where_clause,
                            span.clone(),
                            ctx.model,
                            ctx.diagnostics,
                        );
                    }
                }
            }
            let best_effort_ty = compute_function_best_effort_return_type(
                symbol_id,
                &arguments,
                &explicit_type_args,
                &span,
                ctx,
            );
            Expression::deferred_function_call(
                vec![symbol_id],
                arguments,
                explicit_type_args,
                best_effort_ty,
                span,
            )
        },

        // Overloaded function reference - need to pick one
        ExprKind::OverloadedRef(ref candidates) => {
            // Validate access modes eagerly on the matching candidate
            for &candidate_id in candidates.iter() {
                if let Some(symbol) = ctx.model.query(SymbolFor { id: candidate_id })
                    && let Some(callable) = get_callable_behavior(&symbol)
                    && matches_signature(&callable, arguments.len(), arg_labels)
                {
                    validate_argument_access_modes(&callable, &arguments, &span, ctx);
                    break;
                }
            }
            let best_effort_ty = compute_overloaded_best_effort_return_type(
                candidates,
                &arguments,
                arg_labels,
                &span,
                ctx,
            );
            Expression::deferred_function_call(
                candidates.clone(),
                arguments,
                explicit_type_args,
                best_effort_ty,
                span,
            )
        },

        // Method reference (from member access on a type)
        ExprKind::MethodRef {
            ref receiver,
            ref candidates,
            ref method_name,
        } => {
            // Phase 15: defer ALL method calls to type inference.
            if matches!(
                receiver.kind,
                ExprKind::TypeRef(_) | ExprKind::TypeParameterRef(_) | ExprKind::AssociatedTypeRef
            ) {
                // Static method calls: defer via DeferredStaticCall.
                // For generic types without explicit type args (e.g., `Box.wrap(42)`),
                // fill in Infer types so the solver can infer them from arguments.
                let target_ty = fill_missing_type_params(&receiver.ty, &span);
                let filled_receiver = Expression {
                    ty: target_ty.clone(),
                    ..(**receiver).clone()
                };
                let deferred_return_ty = infer_deferred_method_return_type(
                    &filled_receiver,
                    candidates,
                    &arguments,
                    arg_labels,
                    &explicit_type_args,
                    &span,
                    ctx,
                );
                Expression::deferred_static_call(
                    target_ty,
                    method_name.to_string(),
                    arguments,
                    vec![],
                    explicit_type_args,
                    deferred_return_ty,
                    span,
                )
            } else {
                // Instance method calls: defer via DeferredMethodCall.
                let deferred_return_ty = infer_deferred_method_return_type(
                    receiver,
                    candidates,
                    &arguments,
                    arg_labels,
                    &explicit_type_args,
                    &span,
                    ctx,
                );
                Expression::deferred_method_call(
                    (**receiver).clone(),
                    method_name.to_string(),
                    arguments,
                    explicit_type_args,
                    deferred_return_ty,
                    span,
                )
            }
        },

        // Deferred member access used as callee — the member access was deferred,
        // but now we know it's being called. Route through resolve_member_call which
        // handles field+subscript, callable fields, and method deferral.
        ExprKind::DeferredMemberAccess {
            ref receiver,
            ref member,
        } => {
            if matches!(
                receiver.kind,
                ExprKind::TypeRef(_) | ExprKind::TypeParameterRef(_) | ExprKind::AssociatedTypeRef
            ) {
                let target_ty = fill_missing_type_params(&receiver.ty, &span);
                let deferred_return_ty =
                    match ctx.model.resolve_member(&receiver.ty, member, true) {
                        Ok(resolution) => resolution.ty,
                        _ => Ty::infer(span.clone()),
                    };
                Expression::deferred_static_call(
                    target_ty,
                    member.to_string(),
                    arguments,
                    vec![],
                    explicit_type_args,
                    deferred_return_ty,
                    span,
                )
            } else {
                // Use resolve_member_call which tries field+subscript, callable fields,
                // and defers method calls — exactly what we need.
                resolve_member_call(receiver, member, arguments, arg_labels, explicit_type_args, span, ctx)
            }
        },

        // Field access - might be method call on struct
        ExprKind::FieldAccess {
            ref object,
            ref field,
        } => {
            // For static field access (object is TypeRef), the callee expression represents
            // the field's value. If that value type has subscripts, use subscript resolution.
            // Otherwise, fall through to method call resolution.
            //
            // Example: Foo.myStyle("test") where myStyle is a static computed property
            // returning Style, and Style has subscripts - we want to call the subscript
            // on the Style value, not look for a method named "myStyle" on Foo.
            if matches!(object.kind, ExprKind::TypeRef(_)) {
                // Check if the callee's type (the field's type) has subscripts
                if let Some(subscript_expr) =
                    try_resolve_subscript_call(&callee, &arguments, arg_labels, &span, ctx)
                {
                    return subscript_expr;
                }
            }
            // This could be:
            // 1. A field with callable type (first-class function)
            // 2. A method call
            resolve_member_call(object, field, arguments, arg_labels, explicit_type_args, span, ctx)
        },

        // Primitive method reference - convert to a primitive method call
        ExprKind::PrimitiveMethodRef {
            ref receiver,
            ref method,
        } => {
            // Primitive methods don't support explicit type arguments
            if let Some(ref type_args) = explicit_type_args
                && !type_args.is_empty()
            {
                ctx.diagnostics.add_diagnostic(
                    TypeArgsOnNonGenericError {
                        span: span.clone(),
                        callee_description: format!("primitive method '{}'", method.name()),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }
            // Validate argument count (primitive methods typically take no extra arguments)
            let expected_args = method.arity();
            if arguments.len() != expected_args {
                ctx.diagnostics.add_diagnostic(
                    PrimitiveMethodArityError {
                        call_span: span.clone(),
                        method_name: method.name().to_string(),
                        receiver_type: receiver.ty.to_string(),
                        expected_arity: expected_args,
                        provided_arity: arguments.len(),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }
            Expression::primitive_method_call((**receiver).clone(), *method, arguments, span)
        },

        // Type reference - struct instantiation
        ExprKind::TypeRef(symbol_id) => resolve_struct_instantiation(
            symbol_id,
            arguments,
            arg_labels,
            explicit_type_args,
            span,
            ctx,
        ),

        // Type parameter reference - init call on type parameter (T())
        ExprKind::TypeParameterRef(symbol_id) => {
            resolve_type_parameter_init_call(symbol_id, arguments, arg_labels, span, ctx)
        },

        // Enum case - allow calling with empty parens (Color.Red() is same as Color.Red)
        ExprKind::EnumCase { case_id } => {
            // Only allow empty argument lists for simple enum cases
            if arguments.is_empty() {
                // Return the enum case expression directly (Color.Red() => Color.Red)
                Expression::enum_case(case_id, callee_ty, span)
            } else {
                // Enum case doesn't have associated values but was called with args
                ctx.diagnostics.add_diagnostic(
                    NonCallableError {
                        span: span.clone(),
                        ty: format!("{}", callee_ty),
                    }
                    .into_diagnostic(),
                );
                Expression::error(span)
            }
        },

        // Language intrinsic reference - create an intrinsic call
        ExprKind::LangIntrinsicRef(intrinsic) => {
            // Type arguments for lang intrinsics (like sizeof[T], cast_ptr[T]) are already
            // extracted during path resolution and stored inside the intrinsic itself.
            // We ignore explicit_type_args here since they've already been processed.

            // Validate argument count based on intrinsic
            let expected_arity = intrinsic.arity();
            if arguments.len() != expected_arity {
                ctx.diagnostics.add_diagnostic(
                    ClosureArityError {
                        span: span.clone(),
                        expected: expected_arity,
                        provided: arguments.len(),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }

            // Create the lang intrinsic call expression
            Expression::lang_intrinsic(intrinsic, arguments, span)
        },

        // Local variable reference - could be calling a function stored in a variable
        ExprKind::LocalRef(_local_id) => {
            // Variables cannot have explicit type arguments
            if let Some(ref type_args) = explicit_type_args
                && !type_args.is_empty()
            {
                ctx.diagnostics.add_diagnostic(
                    TypeArgsOnNonGenericError {
                        span: span.clone(),
                        callee_description: "a variable".to_string(),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }

            // Check if the type is callable
            match callee_ty.kind() {
                TyKind::Function {
                    params,
                    return_type,
                } => {
                    // Check argument count matches parameter count
                    if arguments.len() != params.len() {
                        ctx.diagnostics.add_diagnostic(
                            ClosureArityError {
                                span: span.clone(),
                                expected: params.len(),
                                provided: arguments.len(),
                            }
                            .into_diagnostic(),
                        );
                        return Expression::error(span);
                    }
                    Expression::call(callee, arguments, (**return_type).clone(), span)
                },
                TyKind::UnresolvedFunction { return_type, .. } => {
                    // Callable - return type is known, params will be validated by type inference
                    Expression::call(callee, arguments, (**return_type).clone(), span)
                },
                _ => {
                    // Try subscript resolution: the variable's type might have subscripts
                    if let Some(subscript_expr) =
                        try_resolve_subscript_call(&callee, &arguments, arg_labels, &span, ctx)
                    {
                        return subscript_expr;
                    }

                    ctx.diagnostics.add_diagnostic(
                        NonCallableError {
                            span: span.clone(),
                            ty: format!("{}", callee_ty),
                        }
                        .into_diagnostic(),
                    );
                    Expression::error(span)
                },
            }
        },

        // Any other expression - check if callable type
        _ => {
            // Non-function expressions cannot have explicit type arguments
            if let Some(ref type_args) = explicit_type_args
                && !type_args.is_empty()
            {
                ctx.diagnostics.add_diagnostic(
                    TypeArgsOnNonGenericError {
                        span: span.clone(),
                        callee_description: "this expression".to_string(),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }

            match callee_ty.kind() {
                TyKind::Function {
                    params,
                    return_type,
                } => {
                    // Check argument count matches parameter count
                    if arguments.len() != params.len() {
                        ctx.diagnostics.add_diagnostic(
                            ClosureArityError {
                                span: span.clone(),
                                expected: params.len(),
                                provided: arguments.len(),
                            }
                            .into_diagnostic(),
                        );
                        return Expression::error(span);
                    }
                    Expression::call(callee, arguments, (**return_type).clone(), span)
                },
                TyKind::UnresolvedFunction { return_type, .. } => {
                    // Callable - return type is known, params will be validated by type inference
                    Expression::call(callee, arguments, (**return_type).clone(), span)
                },
                _ => {
                    // Try subscript resolution: the expression's type might have subscripts
                    if let Some(subscript_expr) =
                        try_resolve_subscript_call(&callee, &arguments, arg_labels, &span, ctx)
                    {
                        return subscript_expr;
                    }

                    ctx.diagnostics.add_diagnostic(
                        NonCallableError {
                            span: span.clone(),
                            ty: format!("{}", callee_ty),
                        }
                        .into_diagnostic(),
                    );
                    Expression::error(span)
                },
            }
        },
    }
}

// ============================================================================
// Subscript call resolution
// ============================================================================

/// Try to resolve a call as a subscript call.
///
/// When a call is made on a value (not a function), we check if the type has subscripts.
/// For example: `array(0)` where `array` is a value with subscripts.
///
/// Returns `Some(Expression)` if a matching subscript was found, `None` otherwise.
pub fn try_resolve_subscript_call(
    receiver: &Expression,
    arguments: &[CallArgument],
    arg_labels: &[Option<String>],
    span: &Span,
    ctx: &mut BodyResolutionContext,
) -> Option<Expression> {
    let receiver_ty = &receiver.ty;

    // If receiver type is Infer, we can't check for subscripts yet — defer.
    // Return None so the caller can fall through to other resolution paths.
    if matches!(receiver_ty.kind(), TyKind::Infer) {
        return None;
    }

    // Get the container (struct/enum/protocol) from the type
    let container = get_type_container(receiver_ty, ctx)?;

    // Find subscripts on the container
    let subscripts = find_subscripts_on_type(&container, ctx);
    if subscripts.is_empty() {
        return None;
    }

    // Compute best-effort return type from matching subscript
    let best_effort_ty = find_matching_subscript(&subscripts, arguments, arg_labels)
        .and_then(|matching| {
            let behavior = matching.metadata().get_behavior::<SubscriptBehavior>()?;
            let return_type = behavior.return_type().clone();
            let return_type = return_type.substitute_self(receiver_ty);
            let return_type = match receiver_ty.expand_aliases().kind() {
                TyKind::Struct { substitutions, .. }
                | TyKind::Enum { substitutions, .. }
                    if !substitutions.is_empty() =>
                {
                    return_type.apply_substitutions(substitutions)
                },
                _ => return_type,
            };
            Some(return_type)
        })
        .unwrap_or_else(|| Ty::infer(span.clone()));

    // Defer to type inference for full resolution
    Some(Expression::deferred_subscript_call(
        receiver.clone(),
        arguments.to_vec(),
        best_effort_ty,
        span.clone(),
    ))
}

/// Find all subscripts on a type container.
fn find_subscripts_on_type(
    container: &Arc<dyn Symbol<KestrelLanguage>>,
    _ctx: &BodyResolutionContext,
) -> Vec<Arc<SubscriptSymbol>> {
    let mut subscripts = Vec::new();

    // Get direct children that are subscripts
    for child in container.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Subscript
            && let Ok(subscript) = child.downcast_arc::<SubscriptSymbol>()
        {
            subscripts.push(subscript);
        }
    }

    // Check protocol conformances for subscripts
    if let Some(conformances) = container.metadata().get_behavior::<ConformancesBehavior>() {
        for conformance_ty in conformances.conformances() {
            // Extract the protocol symbol from the conformance type
            if let TyKind::Protocol { symbol, .. } = conformance_ty.kind() {
                for child in symbol.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::Subscript
                        && let Ok(subscript) = child.downcast_arc::<SubscriptSymbol>()
                    {
                        subscripts.push(subscript);
                    }
                }
            }
        }
    }

    subscripts
}

/// Find a matching subscript based on argument labels and count.
fn find_matching_subscript(
    subscripts: &[Arc<SubscriptSymbol>],
    arguments: &[CallArgument],
    arg_labels: &[Option<String>],
) -> Option<Arc<SubscriptSymbol>> {
    for subscript in subscripts {
        if matches_subscript_signature(subscript, arguments, arg_labels) {
            return Some(subscript.clone());
        }
    }
    None
}

/// Check if a subscript's parameter signature matches the given arguments.
///
/// For parameters with default values, callers may omit trailing arguments.
fn matches_subscript_signature(
    subscript: &SubscriptSymbol,
    arguments: &[CallArgument],
    arg_labels: &[Option<String>],
) -> bool {
    // Get the subscript behavior which has the parameters
    let Some(behavior) = subscript.metadata().get_behavior::<SubscriptBehavior>() else {
        return false;
    };

    let params = behavior.parameters();

    // Count required parameters (those without defaults)
    let required_count = params.iter().filter(|p| !p.has_default()).count();

    // Check arity: must be at least required_count and at most total params
    if arguments.len() < required_count || arguments.len() > params.len() {
        return false;
    }

    // Check argument labels match using the behavior's built-in method
    let labels_as_str: Vec<Option<&str>> = arg_labels
        .iter()
        .map(|l| l.as_ref().map(|s| s.as_str()))
        .collect();
    behavior.matches_labels(&labels_as_str)
}

/// Returns true if `ty` references any TypeParameter whose ID is in `method_param_ids`.
/// Used to detect method-level type parameters (like `Acc` in `fold[Acc]`) that
/// cannot be resolved at bind time and must remain as `Ty::infer` in best-effort
/// return type estimates. Struct/enum-level type params (like `T` in `Box[T]`) are
/// already substituted by the receiver type and do NOT need this guard.
fn contains_method_type_param(
    ty: &Ty,
    method_param_ids: &std::collections::HashSet<semantic_tree::symbol::SymbolId>,
) -> bool {
    if method_param_ids.is_empty() {
        return false;
    }
    match ty.kind() {
        TyKind::TypeParameter(tp) => {
            method_param_ids.contains(&tp.metadata().id())
        },
        TyKind::Function { params, return_type } => {
            params.iter().any(|p| contains_method_type_param(p, method_param_ids))
                || contains_method_type_param(return_type, method_param_ids)
        },
        TyKind::Tuple(elems) => elems.iter().any(|e| contains_method_type_param(e, method_param_ids)),
        TyKind::Struct { substitutions, .. }
        | TyKind::Enum { substitutions, .. }
        | TyKind::Protocol { substitutions, .. }
        | TyKind::TypeAlias { substitutions, .. } => {
            substitutions.iter().any(|(_, t)| contains_method_type_param(t, method_param_ids))
        },
        TyKind::Pointer(inner) => contains_method_type_param(inner, method_param_ids),
        TyKind::AssociatedType { container, .. } => {
            container.as_ref().is_some_and(|c| contains_method_type_param(c, method_param_ids))
        },
        _ => false,
    }
}

/// Returns true if `ty` contains any `TypeParameter` NOT listed in `method_param_ids`.
/// Used when the receiver is itself a TypeParameter (e.g. `T: SomeProtocol`): we can
/// return concrete types, but must defer to the solver for types that still contain
/// protocol-level type parameters (e.g. `Target` from `Converter[Target]`).
fn contains_non_method_type_parameter(
    ty: &Ty,
    method_param_ids: &std::collections::HashSet<semantic_tree::symbol::SymbolId>,
) -> bool {
    match ty.kind() {
        TyKind::TypeParameter(tp) => !method_param_ids.contains(&tp.metadata().id()),
        TyKind::Function { params, return_type } => {
            params
                .iter()
                .any(|p| contains_non_method_type_parameter(p, method_param_ids))
                || contains_non_method_type_parameter(return_type, method_param_ids)
        },
        TyKind::Tuple(elems) => {
            elems
                .iter()
                .any(|e| contains_non_method_type_parameter(e, method_param_ids))
        },
        TyKind::Struct { substitutions, .. }
        | TyKind::Enum { substitutions, .. }
        | TyKind::Protocol { substitutions, .. }
        | TyKind::TypeAlias { substitutions, .. } => {
            substitutions
                .iter()
                .any(|(_, t)| contains_non_method_type_parameter(t, method_param_ids))
        },
        TyKind::Pointer(inner) => contains_non_method_type_parameter(inner, method_param_ids),
        TyKind::AssociatedType { container, .. } => {
            container
                .as_ref()
                .is_some_and(|c| contains_non_method_type_parameter(c, method_param_ids))
        },
        _ => false,
    }
}

/// Returns true if `ty` contains `SelfType` anywhere in its structure.
/// Used to detect return types that can't be concretized at bind time in protocol extensions.
fn contains_self_type(ty: &Ty) -> bool {
    match ty.kind() {
        TyKind::SelfType => true,
        TyKind::Tuple(elems) => elems.iter().any(contains_self_type),
        TyKind::Function { params, return_type } => {
            params.iter().any(contains_self_type) || contains_self_type(return_type)
        },
        TyKind::Struct { substitutions, .. }
        | TyKind::Enum { substitutions, .. }
        | TyKind::Protocol { substitutions, .. }
        | TyKind::TypeAlias { substitutions, .. } => {
            substitutions.iter().any(|(_, t)| contains_self_type(t))
        },
        TyKind::Pointer(inner) => contains_self_type(inner),
        TyKind::AssociatedType { container, .. } => {
            container.as_ref().is_some_and(|c| contains_self_type(c))
        },
        _ => false,
    }
}

/// Try to resolve an unqualified AssociatedType using the given receiver type.
/// Returns `Some(concrete_type)` if the associated type can be resolved, `None` otherwise.
/// For example: `Item` → resolves to `Int64` when receiver is `ArrayIterator[Int64]`.
fn try_resolve_assoc_type_from_receiver(
    ty: &Ty,
    receiver_ty: &Ty,
    ctx: &BodyResolutionContext,
) -> Option<Ty> {
    match ty.kind() {
        TyKind::AssociatedType { symbol, container: None } => {
            // Unqualified AssociatedType: resolve using receiver as the container.
            ctx.model.resolve_associated_type(receiver_ty, &symbol.metadata().name().value)
        },
        TyKind::AssociatedType { container: Some(_), .. } => {
            // Qualified AssociatedType: try normal resolution.
            let resolved = resolve_associated_types(ty, ctx);
            if matches!(resolved.kind(), TyKind::AssociatedType { .. }) {
                None // Still unresolved
            } else {
                Some(resolved)
            }
        },
        _ => None,
    }
}

/// Recursively match `pattern` against `concrete` to infer substitutions for method-level
/// TypeParameters. Only updates `subs` entries for TypeParameters in `method_type_params`.
fn infer_method_type_params(
    pattern: &Ty,
    concrete: &Ty,
    method_type_params: &[Arc<TypeParameterSymbol>],
    subs: &mut Substitutions,
) {
    match pattern.kind() {
        TyKind::TypeParameter(tp) => {
            let tp_id = tp.metadata().id();
            if method_type_params.iter().any(|p| p.metadata().id() == tp_id)
                && !subs.contains(tp_id)
            {
                subs.insert(tp_id, concrete.clone());
            }
        },
        TyKind::Tuple(pattern_elems) => {
            if let TyKind::Tuple(concrete_elems) = concrete.kind() {
                for (pe, ce) in pattern_elems.iter().zip(concrete_elems.iter()) {
                    infer_method_type_params(pe, ce, method_type_params, subs);
                }
            }
        },
        TyKind::Struct { substitutions: pattern_subs, .. } => {
            if let TyKind::Struct { substitutions: concrete_subs, .. } = concrete.kind() {
                for (id, pattern_sub_ty) in pattern_subs.iter() {
                    if let Some(concrete_sub_ty) = concrete_subs.get(*id) {
                        infer_method_type_params(
                            pattern_sub_ty,
                            concrete_sub_ty,
                            method_type_params,
                            subs,
                        );
                    }
                }
            }
        },
        TyKind::Enum { substitutions: pattern_subs, .. } => {
            if let TyKind::Enum { substitutions: concrete_subs, .. } = concrete.kind() {
                for (id, pattern_sub_ty) in pattern_subs.iter() {
                    if let Some(concrete_sub_ty) = concrete_subs.get(*id) {
                        infer_method_type_params(
                            pattern_sub_ty,
                            concrete_sub_ty,
                            method_type_params,
                            subs,
                        );
                    }
                }
            }
        },
        _ => {},
    }
}

/// For generic struct/enum types without explicit type arguments (e.g., `Box` from `Box.wrap(42)`),
/// fill in inference variables for each type parameter so the solver can infer them from arguments.
/// Returns the type unchanged if it already has substitutions or is non-generic.
fn fill_missing_type_params(ty: &Ty, span: &Span) -> Ty {
    match ty.kind() {
        TyKind::Struct { symbol, substitutions } if substitutions.is_empty() => {
            let type_params = symbol.type_parameters();
            if type_params.is_empty() {
                return ty.clone();
            }
            let mut subs = Substitutions::new();
            for param in type_params {
                subs.insert(param.metadata().id(), Ty::infer(span.clone()));
            }
            Ty::generic_struct(symbol.clone(), subs, span.clone())
        },
        TyKind::Enum { symbol, substitutions } if substitutions.is_empty() => {
            let type_params = symbol.type_parameters();
            if type_params.is_empty() {
                return ty.clone();
            }
            let mut subs = Substitutions::new();
            for param in type_params {
                subs.insert(param.metadata().id(), Ty::infer(span.clone()));
            }
            Ty::generic_enum(symbol.clone(), subs, span.clone())
        },
        _ => ty.clone(),
    }
}

fn infer_deferred_method_return_type(
    receiver: &Expression,
    candidates: &[SymbolId],
    arguments: &[CallArgument],
    arg_labels: &[Option<String>],
    explicit_type_args: &Option<Vec<Ty>>,
    span: &Span,
    ctx: &mut BodyResolutionContext,
) -> Ty {
    // Resolve Self before substituting into candidate signatures.
    let resolved_receiver_ty = resolve_self_type_to_concrete(&receiver.ty, ctx);

    // For Infer receivers, we have no type information at all — can't do anything useful.
    if matches!(resolved_receiver_ty.kind(), TyKind::Infer) {
        return Ty::infer(span.clone());
    }

    let mut inferred: Option<Ty> = None;

    for &candidate_id in candidates {
        let Some(symbol) = ctx.model.query(SymbolFor { id: candidate_id }) else {
            continue;
        };
        let Some(orig_callable) = get_callable_behavior(&symbol) else {
            continue;
        };
        if !matches_signature(&orig_callable, arguments.len(), arg_labels) {
            continue;
        }

        let mut callable = orig_callable.map_types(&mut |t| t.substitute_self(&resolved_receiver_ty));
        callable = callable.map_types(&mut |t| resolve_associated_types(t, ctx));

        let mut candidate_return_ty = callable.return_type().substitute_self(&resolved_receiver_ty);
        candidate_return_ty = resolve_associated_types(&candidate_return_ty, ctx);
        let expanded_receiver_ty = resolved_receiver_ty.expand_aliases();
        if let TyKind::Struct { substitutions, .. } | TyKind::Enum { substitutions, .. } =
            expanded_receiver_ty.kind()
        {
            if !substitutions.is_empty() {
                candidate_return_ty = candidate_return_ty.apply_substitutions(substitutions);
            }
        }

        // If the return type still has SelfType after substitution (e.g. `func clone() -> Self`
        // in a protocol extension), we can't give a concrete type at bind time.
        if contains_self_type(&candidate_return_ty) {
            return Ty::infer(span.clone());
        }

        // If the receiver is a TypeParameter (e.g. `T: Converter[lang.i64]`), we can only
        // resolve the return type if it's fully concrete — or contains only method-level
        // TypeParameters (which the block below will resolve). Protocol-level TypeParameters
        // (like `Target` from `Converter[Target]`) can't be resolved from the constraint
        // at bind time; the inference solver must handle them.
        if matches!(resolved_receiver_ty.kind(), TyKind::TypeParameter(_)) {
            let method_param_ids_for_tp_check: std::collections::HashSet<_> = symbol
                .as_any()
                .downcast_ref::<FunctionSymbol>()
                .map(|f| f.type_parameters().iter().map(|tp| tp.metadata().id()).collect())
                .unwrap_or_default();
            if contains_non_method_type_parameter(
                &candidate_return_ty,
                &method_param_ids_for_tp_check,
            ) {
                return Ty::infer(span.clone());
            }
        }

        // If the return type still has method-level TypeParameters, try to resolve them.
        let method_type_params = symbol
            .as_any()
            .downcast_ref::<FunctionSymbol>()
            .map(|f| f.type_parameters())
            .unwrap_or_default();

        if !method_type_params.is_empty() {
            let method_param_ids: std::collections::HashSet<_> =
                method_type_params.iter().map(|tp| tp.metadata().id()).collect();

            if contains_method_type_param(&candidate_return_ty, &method_param_ids) {
                // Step 0: use explicit type args if provided (handles `cast[Bucket[K, V]]`).
                let mut subs = if let Some(type_args) = explicit_type_args {
                    let mut s = Substitutions::new();
                    for (tp, ty_arg) in method_type_params.iter().zip(type_args.iter()) {
                        s.insert(tp.metadata().id(), ty_arg.clone());
                    }
                    s
                } else {
                    // Step 1: infer TypeParams from argument types (handles `zip[Other]`, `fold[Acc]`).
                    let arg_types: Vec<Ty> =
                        arguments.iter().map(|a| a.value.ty.clone()).collect();
                    infer_type_arguments(&method_type_params, &callable, &arg_types)
                };

                // Step 2: infer remaining TypeParams from where clause TypeEquality constraints
                // (handles `unzip[A, B]() where Item = (A, B)`).
                if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
                    let where_clause = func_sym.where_clause();
                    for constraint in where_clause.constraints() {
                        if let Constraint::TypeEquality { left, right, .. } = constraint {
                            if let Some(resolved) = try_resolve_assoc_type_from_receiver(
                                left,
                                &resolved_receiver_ty,
                                ctx,
                            ) {
                                infer_method_type_params(
                                    right,
                                    &resolved,
                                    &method_type_params,
                                    &mut subs,
                                );
                            }
                            if let Some(resolved) = try_resolve_assoc_type_from_receiver(
                                right,
                                &resolved_receiver_ty,
                                ctx,
                            ) {
                                infer_method_type_params(
                                    left,
                                    &resolved,
                                    &method_type_params,
                                    &mut subs,
                                );
                            }
                        }
                    }
                }
                // Substitute all resolved method TypeParams into the return type.
                // For unresolved params (e.g. `U` from a closure), insert Infer so the
                // return type is partially concrete (e.g. `Dictionary[K, _, H]` instead of
                // bare `_`). This gives downstream bind-time checks enough info to find
                // subscripts, fields, etc. The solver fills in the real type later.
                for tp in &method_type_params {
                    if !subs.contains(tp.metadata().id()) {
                        subs.insert(tp.metadata().id(), Ty::infer(span.clone()));
                    }
                }
                candidate_return_ty = candidate_return_ty.apply_substitutions(&subs);
            }
        }

        match &inferred {
            None => inferred = Some(candidate_return_ty),
            Some(existing) if existing.to_string() == candidate_return_ty.to_string() => {},
            Some(_) => return Ty::infer(span.clone()),
        }
    }

    inferred.unwrap_or_else(|| Ty::infer(span.clone()))
}


/// Compute a best-effort return type for a single function call.
///
/// Extracts return type from the callable behavior without full resolution.
/// Used for deferred function calls to prevent cascading bind-time errors.
fn compute_function_best_effort_return_type(
    symbol_id: SymbolId,
    arguments: &[CallArgument],
    explicit_type_args: &Option<Vec<Ty>>,
    span: &Span,
    ctx: &BodyResolutionContext,
) -> Ty {
    let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) else {
        return Ty::infer(span.clone());
    };

    let Some(callable) = get_callable_behavior(&symbol) else {
        return Ty::infer(span.clone());
    };

    let return_ty = callable.return_type().clone();

    // If explicit type args are provided, apply substitutions
    if let Some(type_args) = explicit_type_args {
        if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
            let type_params = func_sym.type_parameters();
            if !type_params.is_empty() && type_args.len() == type_params.len() {
                let mut subs = Substitutions::new();
                for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
                    subs.insert(param.metadata().id(), arg_ty.clone());
                }
                return return_ty.apply_substitutions(&subs);
            }
        }
    }

    // Try to infer type args from arguments for a best-effort return type
    if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
        let type_params = func_sym.type_parameters();
        if !type_params.is_empty() {
            let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
            let mut subs = infer_type_arguments(&type_params, &callable, &arg_types);
            // Fill remaining with Infer
            for tp in &type_params {
                let tp_id = tp.metadata().id();
                if subs.get(tp_id).is_none() {
                    subs.insert(tp_id, Ty::infer(span.clone()));
                }
            }
            return return_ty.apply_substitutions(&subs);
        }
    }

    // For enum cases: build best-effort return type with Infer vars for parent type params
    if symbol.metadata().kind() == KestrelSymbolKind::EnumCase {
        use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
        if let Some(parent) = symbol.metadata().parent() {
            if let Some(enum_sym) = parent.as_any().downcast_ref::<EnumSymbol>() {
                let type_params = enum_sym.type_parameters();
                if type_params.is_empty() {
                    // Non-generic enum: return as-is
                    return callable.return_type().clone();
                }
                // Create infer vars for each type param, try to infer from arguments
                let mut subs = Substitutions::new();
                let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();
                for tp in &type_params {
                    let tp_id = tp.metadata().id();
                    // Try to infer from argument types
                    let mut inferred = None;
                    for (param, arg_ty) in callable.parameters().iter().zip(arg_types.iter()) {
                        if let TyKind::TypeParameter(param_tp) = param.ty.kind() {
                            if param_tp.metadata().id() == tp_id {
                                inferred = Some(arg_ty.clone());
                                break;
                            }
                        }
                    }
                    subs.insert(tp_id, inferred.unwrap_or_else(|| Ty::infer(span.clone())));
                }
                // Build the enum type with substituted type args
                if let Ok(enum_arc) = parent.clone().downcast_arc::<EnumSymbol>() {
                    return Ty::generic_enum(enum_arc, subs, span.clone());
                }
            }
        }
    }

    return_ty
}

/// Compute a best-effort return type for an overloaded function call.
///
/// Tries each candidate to find one that matches by labels/arity and returns
/// its return type. Falls back to Infer if no match is found.
fn compute_overloaded_best_effort_return_type(
    candidates: &[SymbolId],
    arguments: &[CallArgument],
    arg_labels: &[Option<String>],
    span: &Span,
    ctx: &BodyResolutionContext,
) -> Ty {
    for &candidate_id in candidates {
        if let Some(symbol) = ctx.model.query(SymbolFor { id: candidate_id })
            && let Some(callable) = get_callable_behavior(&symbol)
            && matches_signature(&callable, arguments.len(), arg_labels)
        {
            return callable.return_type().clone();
        }
    }
    Ty::infer(span.clone())
}

/// Resolve struct instantiation: `StructName(x: 1, y: 2)` or `StructName[T, U](x: 1, y: 2)`
///
/// This handles both explicit initializers and implicit memberwise initialization.
/// When explicit type arguments are provided, they are resolved in the current scope
/// before being used as substitutions for the struct's type parameters.
pub fn resolve_struct_instantiation(
    symbol_id: SymbolId,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    explicit_type_args: Option<Vec<Ty>>,
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Get the struct symbol
    let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) else {
        return Expression::error(span);
    };

    // Verify it's a struct
    if symbol.metadata().kind() != KestrelSymbolKind::Struct {
        // Not a struct - check if it's a function.
        // NOTE: With two-pass binding, CallableBehavior should always be available
        // for functions, so they should resolve as SymbolRef not TypeRef. This
        // fallback handles any edge cases where that doesn't happen.
        if symbol.metadata().kind() == KestrelSymbolKind::Function
            && let Some(callable) = get_callable_behavior(&symbol)
        {
            // Validate access modes for arguments
            validate_argument_access_modes(&callable, &arguments, &span, ctx);

            let return_ty = callable.return_type().clone();
            let fn_ty = callable.function_type();
            let callee = Expression::symbol_ref(symbol_id, fn_ty, false, span.clone());
            return Expression::call(callee, arguments, return_ty, span);
        }
        // TODO: Add proper error diagnostic
        return Expression::error(span);
    }

    // Verify it can be downcast to StructSymbol
    if symbol.as_ref().downcast_ref::<StructSymbol>().is_none() {
        return Expression::error(span);
    }

    // Collect initializers from direct struct children
    let mut explicit_inits: Vec<Arc<dyn Symbol<KestrelLanguage>>> = symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Initializer)
        .collect();

    // Also collect initializers from extensions
    let container_id = symbol.metadata().id();
    let extensions = ctx.model.query(ExtensionsFor {
        target_id: container_id,
    });

    // Create struct type for extension filtering
    let struct_ty = create_struct_type(&symbol, span.clone());
    let applicable_extensions = filter_applicable_extensions(extensions, &struct_ty, ctx);

    for extension in applicable_extensions {
        for child in extension.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Initializer {
                explicit_inits.push(child);
            }
        }
    }

    if !explicit_inits.is_empty() {
        // Has explicit initializers - defer to type inference for overload resolution
        let struct_ty = if let Some(ref type_args) = explicit_type_args {
            create_struct_type_with_type_args(&symbol, type_args, span.clone(), ctx)
        } else {
            fill_missing_type_params(&create_struct_type(&symbol, span.clone()), &span)
        };
        return Expression::deferred_init_call(
            struct_ty.clone(),
            arguments,
            explicit_type_args,
            struct_ty, // return type = struct type (best effort)
            span,
        );
    }

    // No explicit initializers - try implicit memberwise init
    resolve_implicit_init(
        symbol_id,
        arguments,
        arg_labels,
        explicit_type_args,
        span,
        symbol.clone(),
        ctx,
    )
}

/// Resolve implicit memberwise initialization
///
/// The struct must not have any explicit initializers and all fields must be visible.
fn resolve_implicit_init(
    _struct_symbol_id: SymbolId,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    explicit_type_args: Option<Vec<Ty>>,
    span: Span,
    struct_symbol: Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let struct_name = struct_symbol.metadata().name().value.clone();

    // Collect stored (non-computed, non-static) fields in declaration order
    // Only stored fields are part of the memberwise initializer
    let fields: Vec<Arc<dyn Symbol<KestrelLanguage>>> = struct_symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
        .filter(|c| {
            // Exclude computed and static fields from memberwise init
            use kestrel_semantic_tree::behavior::{ComputedPropertyMarker, StaticBehavior};
            let is_computed = c.metadata().get_behavior::<ComputedPropertyMarker>().is_some();
            let is_static = c.metadata().get_behavior::<StaticBehavior>().is_some();
            !is_computed && !is_static
        })
        .collect();

    let field_names: Vec<String> = fields
        .iter()
        .map(|f| f.metadata().name().value.clone())
        .collect();

    // Check visibility of all fields
    for field in &fields {
        let field_id = field.metadata().id();
        if !ctx.model.query(IsVisibleFrom {
            target: field_id,
            context: ctx.function_id,
        }) {
            // Field is not visible - cannot use implicit init
            let error = FieldNotVisibleForInitError {
                span: span.clone(),
                struct_name: struct_name.clone(),
                field_name: field.metadata().name().value.clone(),
                field_visibility: "private".to_string(), // TODO: Get actual visibility
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(span);
        }
    }

    // Validate arguments match fields in order
    if arguments.len() != fields.len() {
        let error = ImplicitInitArityError {
            span: span.clone(),
            struct_name,
            expected: fields.len(),
            provided: arguments.len(),
            field_names,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Check that labels match field names
    for (i, (field, _arg)) in fields.iter().zip(arguments.iter()).enumerate() {
        let field_name = field.metadata().name().value.clone();
        let expected_label = Some(field_name.clone());
        let provided_label = arg_labels.get(i).cloned().flatten();

        if arg_labels.get(i) != Some(&expected_label) {
            let error = ImplicitInitLabelError {
                span: span.clone(),
                struct_name: struct_name.clone(),
                arg_index: i,
                provided_label,
                expected_label: field_name,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            return Expression::error(span);
        }
    }

    // All checks passed - create ImplicitStructInit expression
    // Create the actual struct type using the struct symbol
    // If explicit type arguments are provided, use them; otherwise infer from argument types
    let struct_ty = if let Some(ref type_args) = explicit_type_args {
        create_struct_type_with_type_args(&struct_symbol, type_args, span.clone(), ctx)
    } else {
        // For generic structs, infer type arguments from argument types
        let arg_types: Vec<_> = arguments.iter().map(|a| a.value.ty.clone()).collect();
        create_generic_struct_type(&struct_symbol, &fields, &arg_types, span.clone())
    };

    Expression::implicit_struct_init(struct_ty, arguments, span)
}

/// Resolve an init call on a type parameter: `T()` where T is constrained by protocols.
///
/// This looks up init methods from the type parameter's protocol bounds and uses
/// overload resolution to find a matching init.
fn resolve_type_parameter_init_call(
    symbol_id: SymbolId,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Get the type parameter symbol
    let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) else {
        return Expression::error(span);
    };

    // Verify it's a type parameter and get Arc<TypeParameterSymbol>
    let type_param_arc = match symbol.clone().downcast_arc::<TypeParameterSymbol>() {
        Ok(arc) => arc,
        Err(_) => return Expression::error(span),
    };

    let type_param_name = type_param_arc.metadata().name().value.clone();

    // For Self substitution - use the type parameter type so T() returns T, not _
    let type_param_ty = Ty::type_parameter(type_param_arc, span.clone());

    // Get protocol bounds for this type parameter
    let bounds = get_type_parameter_bounds_by_id(symbol_id, ctx);

    if bounds.is_empty() {
        // No bounds - cannot call init on unconstrained type parameter
        let error = UnconstrainedTypeParameterMemberError {
            span: span.clone(),
            member_name: "init".to_string(),
            type_param_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Collect init methods from all protocol bounds
    let mut candidates: Vec<InitCandidate> = Vec::new();
    let mut bound_names: Vec<String> = Vec::new();

    for bound in &bounds {
        if let TyKind::Protocol {
            symbol: proto,
            substitutions,
        } = bound.kind()
        {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Collect initializers from this protocol
            collect_protocol_initializers(proto, &type_param_ty, substitutions, &mut candidates);
        }
    }

    if candidates.is_empty() {
        // No init methods found in any bound
        let error = NoInitInTypeParameterBoundsError {
            span: span.clone(),
            type_param_name,
            bound_names,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Find matching candidates by signature
    let matching: Vec<&InitCandidate> = candidates
        .iter()
        .filter(|c| matches_signature(&c.callable, arguments.len(), arg_labels))
        .collect();

    if matching.is_empty() {
        // No matching signature - collect available init signatures for error message
        let available_inits: Vec<OverloadDescription> = candidates
            .iter()
            .map(|c| {
                let labels: Vec<Option<String>> = c
                    .callable
                    .parameters()
                    .iter()
                    .map(|p| p.external_label().map(|s| s.to_string()))
                    .collect();
                let param_types: Vec<String> = c
                    .callable
                    .parameters()
                    .iter()
                    .map(|p| p.ty.to_string())
                    .collect();
                OverloadDescription {
                    name: type_param_name.clone(),
                    labels,
                    param_types,
                    definition_span: Some(c.init.metadata().span().clone()),
                    definition_file_id: Some(c.init.metadata().span().file_id),
                }
            })
            .collect();

        let error = NoMatchingTypeParameterInitError {
            span: span.clone(),
            type_param_name,
            provided_labels: arg_labels.to_vec(),
            provided_arity: arguments.len(),
            available_inits,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Check for ambiguity - multiple protocols have matching init with same signature
    let mut seen_protocols: std::collections::HashSet<String> = std::collections::HashSet::new();
    let unique_matching: Vec<&InitCandidate> = matching
        .into_iter()
        .filter(|c| seen_protocols.insert(c.protocol_name.clone()))
        .collect();

    if unique_matching.len() > 1 {
        // Ambiguous - multiple protocols have matching init
        let protocol_names: Vec<String> = unique_matching
            .iter()
            .map(|c| c.protocol_name.clone())
            .collect();
        let definition_spans: Vec<(String, Span)> = unique_matching
            .iter()
            .map(|c| (c.protocol_name.clone(), c.init.metadata().span().clone()))
            .collect();

        let error = AmbiguousTypeParameterInitError {
            span: span.clone(),
            type_param_name,
            protocol_names,
            definition_spans,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Single matching init found
    let winner = unique_matching[0];
    let return_ty = type_param_ty; // Return type is T, not Self

    // Create a call expression referencing the protocol's init
    let init_id = winner.init.metadata().id();

    // Build the function type for the initializer
    let param_tys: Vec<Ty> = winner
        .callable
        .parameters()
        .iter()
        .map(|p| p.ty.clone())
        .collect();
    let init_fn_ty = Ty::function(param_tys, return_ty.clone(), span.clone());

    let init_ref = Expression::symbol_ref(init_id, init_fn_ty, false, span.clone());

    // Use generic_call with protocol substitutions so type inference can apply them
    Expression::generic_call(
        init_ref,
        arguments,
        winner.protocol_substitutions.clone(),
        return_ty,
        span,
    )
}

/// Candidate for init resolution on type parameter
struct InitCandidate {
    /// The init symbol
    init: Arc<dyn Symbol<KestrelLanguage>>,
    /// The callable behavior (for signature matching)
    callable: kestrel_semantic_tree::behavior::callable::CallableBehavior,
    /// Protocol name (for ambiguity detection)
    protocol_name: String,
    /// Protocol substitutions to apply (for generic protocol bounds)
    protocol_substitutions: Substitutions,
}

/// Collect initializer methods from a protocol, including inherited protocols.
///
/// The `protocol_substitutions` parameter contains the type arguments for the protocol
/// bound, e.g., for `T: Buildable[lang.i64]`, it maps Buildable's type parameter to `lang.i64`.
fn collect_protocol_initializers(
    protocol: &Arc<ProtocolSymbol>,
    self_replacement: &Ty,
    protocol_substitutions: &Substitutions,
    candidates: &mut Vec<InitCandidate>,
) {
    let protocol_name = protocol.metadata().name().value.clone();

    // Get all initializers from this protocol
    for child in protocol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Initializer
            && let Some(callable) = get_callable_behavior(&child)
        {
            // Substitute Self with the type parameter in the callable
            let substituted = callable.map_types(&mut |t| t.substitute_self(self_replacement));
            // Apply protocol type parameter substitutions
            let substituted =
                substituted.map_types(&mut |t| t.apply_substitutions(protocol_substitutions));

            candidates.push(InitCandidate {
                init: child.clone(),
                callable: substituted,
                protocol_name: protocol_name.clone(),
                protocol_substitutions: protocol_substitutions.clone(),
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
                let composed_subs =
                    parent_subs.map_values(&mut |ty| ty.apply_substitutions(protocol_substitutions));
                collect_protocol_initializers(parent, self_replacement, &composed_subs, candidates);
            }
        }
    }
}

// =============================================================================
// ACCESS MODE VALIDATION
// =============================================================================

/// Result of classifying an expression's mutability for access mode validation.
#[derive(Debug)]
pub(crate) enum MutabilityClassification {
    /// Expression is a mutable lvalue (var binding or mutable field chain)
    Mutable,
    /// Expression is an immutable local binding (let)
    ImmutableLocal { name: String, span: Span },
    /// Expression is an immutable field (let field)
    ImmutableField {
        field_name: String,
        field_span: Option<Span>,
    },
    /// Expression has a mutable field but the root binding is immutable (let)
    ImmutableThroughBinding {
        binding_name: String,
        binding_span: Span,
        field_path: String,
    },
    /// Expression is a temporary value (not an lvalue)
    Temporary,
}

/// Classify an expression's mutability for access mode validation.
///
/// This walks the expression tree to determine:
/// - Whether it's an lvalue (can be assigned to)
/// - If so, whether it's mutable throughout the entire access chain
pub(crate) fn classify_mutability(
    expr: &Expression,
    ctx: &BodyResolutionContext,
) -> MutabilityClassification {
    match &expr.kind {
        // Local variable reference
        ExprKind::LocalRef(local_id) => {
            if let Some(local) = ctx.local_scope.get_local(*local_id) {
                if local.is_mutable() {
                    MutabilityClassification::Mutable
                } else {
                    MutabilityClassification::ImmutableLocal {
                        name: local.name().to_string(),
                        span: local.span().clone(),
                    }
                }
            } else {
                // Local not found - shouldn't happen, treat as temporary
                MutabilityClassification::Temporary
            }
        },

        // Field access: obj.field
        ExprKind::FieldAccess { object, field } => {
            // First check if the expression is already marked as mutable
            if expr.mutable {
                return MutabilityClassification::Mutable;
            }

            // Walk up the chain to find why it's not mutable
            classify_field_chain_mutability(object, field, ctx)
        },

        // Tuple index: tuple.0
        ExprKind::TupleIndex { tuple, .. } => {
            // Tuple elements inherit mutability from the tuple expression
            if expr.mutable {
                MutabilityClassification::Mutable
            } else {
                // Classify based on the tuple expression
                classify_mutability(tuple, ctx)
            }
        },

        // Grouping expression: (expr)
        ExprKind::Grouping(inner) => classify_mutability(inner, ctx),

        // Symbol reference: could be a module-level field
        ExprKind::SymbolRef(symbol_id) => {
            use kestrel_semantic_tree::symbol::field::FieldSymbol;

            // Check if this is a field symbol
            if let Some(symbol) = ctx.model.query(SymbolFor { id: *symbol_id })
                && symbol.metadata().kind() == KestrelSymbolKind::Field
            {
                // Use the expression's mutable flag (set during path resolution)
                if expr.mutable {
                    return MutabilityClassification::Mutable;
                }

                // Not mutable - check if it's an immutable field (let)
                if let Some(field) = symbol.as_ref().downcast_ref::<FieldSymbol>()
                    && !field.is_mutable()
                {
                    return MutabilityClassification::ImmutableField {
                        field_name: symbol.metadata().name().value.clone(),
                        field_span: Some(symbol.metadata().span().clone()),
                    };
                }
            }

            // Not a field, or not found - treat as temporary
            MutabilityClassification::Temporary
        },

        // Protocol property access on type parameters
        ExprKind::ProtocolPropertyAccess {
            property_name,
            has_setter,
            ..
        } => {
            // Protocol property is mutable only if it has a setter
            if *has_setter {
                MutabilityClassification::Mutable
            } else {
                MutabilityClassification::ImmutableField {
                    field_name: property_name.clone(),
                    field_span: None,
                }
            }
        },

        // Deferred member access — check the mutable flag computed at bind time.
        ExprKind::DeferredMemberAccess { receiver, member } => {
            if expr.mutable {
                MutabilityClassification::Mutable
            } else {
                classify_field_chain_mutability(receiver, member, ctx)
            }
        },

        // Everything else is a temporary (call results, literals, etc.)
        _ => MutabilityClassification::Temporary,
    }
}

/// Classify mutability for a field access chain, finding the cause of immutability.
fn classify_field_chain_mutability(
    object: &Expression,
    current_field: &str,
    ctx: &BodyResolutionContext,
) -> MutabilityClassification {
    use kestrel_semantic_tree::symbol::field::FieldSymbol;

    // First, check if the field itself is immutable
    // Look up the field in the object's type (struct or enum)
    if let Some((struct_symbol, _)) = object.ty.as_struct_with_subs() {
        for child in struct_symbol.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Field
                && child.metadata().name().value == current_field
            {
                use kestrel_semantic_tree::behavior::ComputedPropertyMarker;
                let is_computed = child.metadata().get_behavior::<ComputedPropertyMarker>().is_some();
                if let Some(field_sym) = child.as_any().downcast_ref::<FieldSymbol>() {
                    // Computed properties with setters are assignable
                    if is_computed {
                        if field_sym.setter().is_none() {
                            // Read-only computed property (no setter)
                            return MutabilityClassification::ImmutableField {
                                field_name: current_field.to_string(),
                                field_span: Some(child.metadata().name().span.clone()),
                            };
                        }
                        // Has setter - assignment is allowed, continue checking the chain
                    } else if !field_sym.is_mutable() {
                        // Stored field declared with `let`
                        return MutabilityClassification::ImmutableField {
                            field_name: current_field.to_string(),
                            field_span: Some(child.metadata().name().span.clone()),
                        };
                    }
                }
                break;
            }
        }
    } else if let Some(enum_symbol) = object.ty.as_enum() {
        for child in enum_symbol.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Field
                && child.metadata().name().value == current_field
            {
                use kestrel_semantic_tree::behavior::ComputedPropertyMarker;
                let is_computed = child.metadata().get_behavior::<ComputedPropertyMarker>().is_some();
                if let Some(field_sym) = child.as_any().downcast_ref::<FieldSymbol>() {
                    // Computed properties with setters are assignable
                    if is_computed {
                        if field_sym.setter().is_none() {
                            // Read-only computed property (no setter)
                            return MutabilityClassification::ImmutableField {
                                field_name: current_field.to_string(),
                                field_span: Some(child.metadata().name().span.clone()),
                            };
                        }
                        // Has setter - assignment is allowed, continue checking the chain
                    } else if !field_sym.is_mutable() {
                        // Stored field declared with `let`
                        return MutabilityClassification::ImmutableField {
                            field_name: current_field.to_string(),
                            field_span: Some(child.metadata().name().span.clone()),
                        };
                    }
                }
                break;
            }
        }
    }

    // The field is mutable, so check the object expression
    match &object.kind {
        ExprKind::LocalRef(local_id) => {
            if let Some(local) = ctx.local_scope.get_local(*local_id) {
                if local.is_mutable() {
                    MutabilityClassification::Mutable
                } else {
                    // The root is an immutable binding
                    MutabilityClassification::ImmutableThroughBinding {
                        binding_name: local.name().to_string(),
                        binding_span: local.span().clone(),
                        field_path: current_field.to_string(),
                    }
                }
            } else {
                MutabilityClassification::Temporary
            }
        },

        ExprKind::TypeRef(_) => {
            // Static field access (e.g., Foo.staticField)
            // Static fields are always mutable from a classification perspective
            // (their mutability is determined by whether the field is `let` or `var`,
            // which was already checked above)
            MutabilityClassification::Mutable
        },

        ExprKind::FieldAccess {
            object: inner_object,
            field: inner_field,
        } => {
            // Recurse up the chain, tracking the field path
            let inner_result = classify_field_chain_mutability(inner_object, inner_field, ctx);
            match inner_result {
                MutabilityClassification::ImmutableThroughBinding {
                    binding_name,
                    binding_span,
                    field_path,
                } => MutabilityClassification::ImmutableThroughBinding {
                    binding_name,
                    binding_span,
                    field_path: format!("{}.{}", field_path, current_field),
                },
                other => other,
            }
        },

        // For other expression types as object, treat as temporary
        _ => MutabilityClassification::Temporary,
    }
}

/// Validate that arguments satisfy access mode requirements.
///
/// For `mutating` parameters, the argument must be a mutable lvalue:
/// - A `var` binding (not `let`)
/// - A mutable field chain (all fields in the path must be `var`, and root must be `var`)
///
/// Returns true if validation passed, false if errors were emitted.
pub fn validate_argument_access_modes(
    callable: &kestrel_semantic_tree::behavior::callable::CallableBehavior,
    arguments: &[CallArgument],
    call_span: &Span,
    ctx: &mut BodyResolutionContext,
) -> bool {
    let mut valid = true;
    let params = callable.parameters();

    for (i, arg) in arguments.iter().enumerate() {
        let Some(param) = params.get(i) else {
            continue;
        };

        match param.access_mode() {
            ParameterAccessMode::Borrow => {
                // Borrow accepts any expression - no validation needed
            },
            ParameterAccessMode::Consuming => {
                // For consuming parameters with non-copyable types, mark the local as moved
                // so subsequent uses will trigger a use-after-move error
                if let ExprKind::LocalRef(local_id) = &arg.value.kind {
                    // Only mark as moved if the type is non-copyable
                    // Use context-aware check that considers `T: not Copyable` bounds
                    if !arg.value.ty.is_copyable_in_context(ctx.where_clause()) {
                        ctx.move_tracker_mut()
                            .mark_moved(*local_id, arg.value.span.clone());
                    }
                }
            },
            ParameterAccessMode::Mutating => {
                // Mutating requires a mutable lvalue
                let classification = classify_mutability(&arg.value, ctx);
                let param_name = param.internal_name().to_string();

                match classification {
                    MutabilityClassification::Mutable => {
                        // Good - mutable lvalue
                    },
                    MutabilityClassification::ImmutableLocal { name, span } => {
                        ctx.diagnostics.add_diagnostic(
                            CannotPassLetToMutatingError {
                                call_span: call_span.clone(),
                                argument_span: arg.value.span.clone(),
                                binding_name: name,
                                binding_span: span,
                                parameter_name: param_name,
                            }
                            .into_diagnostic(),
                        );
                        valid = false;
                    },
                    MutabilityClassification::ImmutableField {
                        field_name,
                        field_span,
                    } => {
                        ctx.diagnostics.add_diagnostic(
                            CannotPassImmutableFieldToMutatingError {
                                call_span: call_span.clone(),
                                argument_span: arg.value.span.clone(),
                                field_name,
                                field_span,
                                parameter_name: param_name,
                            }
                            .into_diagnostic(),
                        );
                        valid = false;
                    },
                    MutabilityClassification::ImmutableThroughBinding {
                        binding_name,
                        binding_span,
                        field_path,
                    } => {
                        ctx.diagnostics.add_diagnostic(
                            CannotMutateThroughImmutableBindingError {
                                call_span: call_span.clone(),
                                argument_span: arg.value.span.clone(),
                                binding_name,
                                binding_span,
                                field_path,
                                parameter_name: param_name,
                            }
                            .into_diagnostic(),
                        );
                        valid = false;
                    },
                    MutabilityClassification::Temporary => {
                        ctx.diagnostics.add_diagnostic(
                            CannotPassTemporaryToMutatingError {
                                call_span: call_span.clone(),
                                argument_span: arg.value.span.clone(),
                                parameter_name: param_name,
                            }
                            .into_diagnostic(),
                        );
                        valid = false;
                    },
                }
            },
        }
    }

    valid
}

/// Detect if the callee syntax node represents a `self.init` pattern.
///
/// This checks the raw syntax tree BEFORE expression resolution, because
/// `self.init` cannot be resolved as a normal member access (init is a keyword).
fn is_self_init_call(callee_node: &SyntaxNode) -> bool {
    // The callee for `self.init(...)` is an ExprPath containing: self, ., init
    // Structure: ExprPath { Identifier("self"), Dot, Init }

    fn check_expr_path(node: &SyntaxNode) -> bool {
        // Collect tokens in order: should be "self" . "init"
        let tokens: Vec<_> = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| {
                !matches!(
                    t.kind(),
                    SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
                )
            })
            .collect();

        // We expect exactly 3 tokens: Identifier("self"), Dot, Init
        if tokens.len() != 3 {
            return false;
        }

        let first_is_self =
            tokens[0].kind() == SyntaxKind::Identifier && tokens[0].text() == "self";
        let second_is_dot = tokens[1].kind() == SyntaxKind::Dot;
        // Accept either Init keyword or Identifier with text "init" (parser may represent it either way)
        let third_is_init = tokens[2].kind() == SyntaxKind::Init
            || (tokens[2].kind() == SyntaxKind::Identifier && tokens[2].text() == "init");

        first_is_self && second_is_dot && third_is_init
    }

    // Check if this is an ExprPath directly
    if callee_node.kind() == SyntaxKind::ExprPath {
        return check_expr_path(callee_node);
    }

    // Or if it's wrapped in an Expression node
    if callee_node.kind() == SyntaxKind::Expression
        && let Some(expr_path) = callee_node
            .children()
            .find(|c| c.kind() == SyntaxKind::ExprPath)
    {
        return check_expr_path(&expr_path);
    }

    false
}
