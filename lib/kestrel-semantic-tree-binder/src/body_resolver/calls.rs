//! Call expression resolution.
//!
//! This module handles resolving function calls, method calls, overloaded calls,
//! and struct instantiation (both explicit and implicit initializers).

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::queries::ExtensionsFor;
use kestrel_semantic_model::{IsVisibleFrom, SemanticModel, SymbolFor};
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ParameterAccessMode};
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::subscript::SubscriptBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprId, ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_case::EnumCaseSymbol;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::subscript::SubscriptSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolution::type_resolver::TypeResolver;

use crate::diagnostics::{
    AmbiguousTypeParameterInitError, CannotMutateThroughImmutableBindingError,
    CannotPassImmutableFieldToMutatingError, CannotPassLetToMutatingError,
    CannotPassTemporaryToMutatingError, ClosureArityError, FieldNotVisibleForInitError,
    ImplicitInitArityError, ImplicitInitLabelError, InstanceMethodOnTypeError,
    MemberNotVisibleError, NoInitInTypeParameterBoundsError, NoMatchingInitializerError,
    NoMatchingMethodError, NoMatchingOverloadError, NoMatchingTypeParameterInitError,
    NonCallableError, NotGenericError, OverloadDescription, PrimitiveMethodArityError,
    TooFewTypeArgumentsError, TooManyTypeArgumentsError, TypeArgsOnNonGenericError,
    UnconstrainedTypeParameterMemberError,
};
use kestrel_syntax_tree::utils::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::members::{
    filter_applicable_extensions, resolve_delegating_init, resolve_member_call,
    substitute_callable_self,
};
use super::utils::{
    create_generic_struct_type, create_struct_type, create_struct_type_with_type_args,
    find_type_directed_match, get_callable_behavior, get_type_container,
    get_type_parameter_bounds_by_id, infer_type_arguments, is_expression_kind, matches_signature,
    replace_type_params_except, substitute_self, substitute_type,
    validate_not_standalone_type_param, verify_type_argument_constraints,
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
        // Direct function reference
        ExprKind::SymbolRef(symbol_id) => resolve_single_function_call(
            symbol_id,
            callee,
            arguments,
            explicit_type_args,
            span,
            ctx,
        ),

        // Overloaded function reference - need to pick one
        ExprKind::OverloadedRef(ref candidates) => {
            resolve_overloaded_call(candidates, callee, arguments, arg_labels, span, ctx)
        },

        // Method reference (from member access on a type)
        ExprKind::MethodRef {
            ref receiver,
            ref candidates,
            ref method_name,
        } => resolve_method_call(
            receiver,
            candidates,
            method_name,
            arguments,
            arg_labels,
            explicit_type_args,
            span,
            ctx,
        ),

        // Field access - might be method call on struct
        ExprKind::FieldAccess {
            ref object,
            ref field,
        } => {
            // This could be:
            // 1. A field with callable type (first-class function)
            // 2. A method call
            resolve_member_call(object, field, arguments, arg_labels, span, ctx)
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

    // Get the container (struct/enum/protocol) from the type
    let container = get_type_container(receiver_ty, ctx)?;

    // Find subscripts on the container
    let subscripts = find_subscripts_on_type(&container, ctx);
    if subscripts.is_empty() {
        return None;
    }

    // Try to find a matching subscript based on argument labels
    let matching = find_matching_subscript(&subscripts, arguments, arg_labels)?;

    // Get the subscript behavior for parameter/return type info
    let behavior = matching.metadata().get_behavior::<SubscriptBehavior>()?;

    // Get the subscript's getter ID
    let getter_id = matching.getter_id()?;

    // Get return type and substitute Self with the receiver type
    let return_type = behavior.return_type().clone();
    let return_type = substitute_self(&return_type, receiver_ty);

    // Substitute type arguments from the receiver type
    let return_type = substitute_receiver_type_args(&return_type, receiver_ty);

    // Create a call to the getter with the receiver as the first implicit argument
    // The getter signature is: get(self, <params>) -> T
    Some(Expression::subscript_call(
        receiver.clone(),
        getter_id,
        arguments.to_vec(),
        return_type,
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

    // Check argument count
    if arguments.len() != params.len() {
        return false;
    }

    // Check argument labels match using the behavior's built-in method
    let labels_as_str: Vec<Option<&str>> = arg_labels
        .iter()
        .map(|l| l.as_ref().map(|s| s.as_str()))
        .collect();
    behavior.matches_labels(&labels_as_str)
}

/// Substitute type arguments from the receiver type into a type.
fn substitute_receiver_type_args(ty: &Ty, receiver_ty: &Ty) -> Ty {
    match receiver_ty.kind() {
        TyKind::Struct { substitutions, .. } | TyKind::Enum { substitutions, .. } => {
            if substitutions.is_empty() {
                ty.clone()
            } else {
                // Substitute the type arguments
                substitute_type(ty, substitutions)
            }
        },
        _ => ty.clone(),
    }
}

/// Resolve a call to a single function (not overloaded)
fn resolve_single_function_call(
    symbol_id: SymbolId,
    callee: Expression,
    arguments: Vec<CallArgument>,
    explicit_type_args: Option<Vec<Ty>>,
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Get the function symbol
    let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) else {
        return Expression::error(span);
    };

    // Get the callable behavior
    let Some(callable) = get_callable_behavior(&symbol) else {
        return Expression::error(span);
    };

    // Check if this is an instance method being called without an instance
    // This happens when we have Type.instanceMethod() instead of instance.instanceMethod()
    if callable.is_instance_method() {
        // Get the parent type name for the error message
        let type_name = symbol
            .metadata()
            .parent()
            .map(|p| p.metadata().name().value.clone())
            .unwrap_or_else(|| "<unknown>".to_string());
        let method_name = symbol.metadata().name().value.clone();

        let error = InstanceMethodOnTypeError {
            span: span.clone(),
            type_name,
            method_name,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Check arity and labels
    let arg_labels: Vec<Option<String>> = arguments.iter().map(|a| a.label.clone()).collect();

    if !matches_signature(&callable, arguments.len(), &arg_labels) {
        // Report error - wrong arity or labels
        let function_name = symbol.metadata().name().value.clone();
        let available_overloads = vec![collect_single_overload_description(&symbol)];

        let error = NoMatchingOverloadError {
            call_span: span.clone(),
            name: function_name,
            provided_labels: arg_labels,
            provided_arity: arguments.len(),
            available_overloads,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Validate access modes for arguments (mutating parameters require mutable lvalues)
    validate_argument_access_modes(&callable, &arguments, &span, ctx);

    // For static methods, get the parent type for Self substitution
    use super::utils::substitute_self;
    use kestrel_semantic_tree::behavior::typed::TypedBehavior;
    let self_replacement: Option<Ty> = symbol.metadata().parent().and_then(|parent| {
        parent
            .metadata()
            .get_behavior::<TypedBehavior>()
            .map(|typed| typed.ty().clone())
    });

    // Get return type and substitutions, applying explicit type arguments if provided
    // or inferring them from argument types
    let (return_ty, call_substitutions) = if let Some(ref type_args) = explicit_type_args {
        // Try to downcast to FunctionSymbol to access type parameters
        if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
            let type_params = func_sym.type_parameters();
            let function_name = symbol.metadata().name().value.clone();

            // Validate: function must be generic if type args are provided
            if type_params.is_empty() {
                ctx.diagnostics.add_diagnostic(
                    NotGenericError {
                        span: span.clone(),
                        type_name: function_name,
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }

            // Validate: type arg count must match type param count
            if type_args.len() < type_params.len() {
                ctx.diagnostics.add_diagnostic(
                    TooFewTypeArgumentsError {
                        span: span.clone(),
                        type_name: function_name,
                        min_expected: type_params.len(),
                        got: type_args.len(),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }

            if type_args.len() > type_params.len() {
                ctx.diagnostics.add_diagnostic(
                    TooManyTypeArgumentsError {
                        span: span.clone(),
                        type_name: function_name,
                        max_expected: type_params.len(),
                        got: type_args.len(),
                    }
                    .into_diagnostic(),
                );
                return Expression::error(span);
            }

            // Build substitutions from type parameters to provided type arguments
            let mut substitutions = Substitutions::new();
            for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
                substitutions.insert(param.metadata().id(), arg_ty.clone());
            }

            // Verify constraints are satisfied
            let where_clause = func_sym.where_clause();
            verify_type_argument_constraints(
                &type_params,
                type_args,
                &where_clause,
                span.clone(),
                ctx.model,
                ctx.diagnostics,
            );

            // Apply substitution to return type
            let return_ty = substitute_type(callable.return_type(), &substitutions);
            (return_ty, substitutions)
        } else {
            (callable.return_type().clone(), Substitutions::new())
        }
    } else {
        // No explicit type arguments - try to infer from argument types
        if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
            let type_params = func_sym.type_parameters();

            if !type_params.is_empty() {
                // Collect argument types
                let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();

                // Infer type arguments from argument types
                let mut substitutions = infer_type_arguments(&type_params, &callable, &arg_types);

                // Build inferred type args, using Infer for parameters that couldn't be determined
                // Also ensure substitutions contains all type parameters (even those mapped to Infer)
                let inferred_args: Vec<Ty> = type_params
                    .iter()
                    .map(|tp| {
                        let tp_id = tp.metadata().id();
                        if let Some(inferred_ty) = substitutions.get(tp_id) {
                            inferred_ty.clone()
                        } else {
                            // Create fresh inference variable for this type parameter
                            let infer_ty = Ty::infer(span.clone());
                            substitutions.insert(tp_id, infer_ty.clone());
                            infer_ty
                        }
                    })
                    .collect();

                // Verify constraints are satisfied
                let where_clause = func_sym.where_clause();
                verify_type_argument_constraints(
                    &type_params,
                    &inferred_args,
                    &where_clause,
                    span.clone(),
                    ctx.model,
                    ctx.diagnostics,
                );

                // Apply substitution to return type
                let return_ty = substitute_type(callable.return_type(), &substitutions);
                (return_ty, substitutions)
            } else {
                (callable.return_type().clone(), Substitutions::new())
            }
        } else if symbol.as_any().downcast_ref::<EnumCaseSymbol>().is_some() {
            // Enum case - check if callee already has concrete types from qualified path
            // e.g., Result[Int, Bool].Ok already has substitutions applied
            let from_qualified_path = if let TyKind::Function { return_type, .. } = callee.ty.kind()
            {
                // Check if the return type is a concrete enum (not just type parameters)
                if let TyKind::Enum { substitutions, .. } = return_type.kind() {
                    // Check if substitutions contain concrete types (not just type parameters)
                    let has_concrete_subs = substitutions
                        .iter()
                        .any(|(_, ty)| !matches!(ty.kind(), TyKind::TypeParameter(_)));
                    if has_concrete_subs {
                        // Use the already-substituted return type from the callee
                        Some((return_type.as_ref().clone(), substitutions.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((ret_ty, subs)) = from_qualified_path {
                (ret_ty, subs)
            } else if let Some(parent) = symbol.metadata().parent() {
                // Fallback: infer type parameters from arguments
                if let Some(enum_sym) = parent.as_any().downcast_ref::<EnumSymbol>() {
                    let type_params = enum_sym.type_parameters();

                    if !type_params.is_empty() {
                        // Collect argument types
                        let arg_types: Vec<Ty> =
                            arguments.iter().map(|a| a.value.ty.clone()).collect();

                        // Infer type arguments from argument types
                        let mut substitutions =
                            infer_type_arguments(&type_params, &callable, &arg_types);

                        // Ensure all type parameters are in substitutions (use Infer for unknown)
                        for tp in type_params {
                            let tp_id = tp.metadata().id();
                            if !substitutions.contains(tp_id) {
                                substitutions.insert(tp_id, Ty::infer(span.clone()));
                            }
                        }

                        // Apply substitution to return type
                        let return_ty = substitute_type(callable.return_type(), &substitutions);
                        (return_ty, substitutions)
                    } else {
                        (callable.return_type().clone(), Substitutions::new())
                    }
                } else {
                    (callable.return_type().clone(), Substitutions::new())
                }
            } else {
                (callable.return_type().clone(), Substitutions::new())
            }
        } else {
            (callable.return_type().clone(), Substitutions::new())
        }
    };

    // Substitute Self with the parent type if this is a static method
    let return_ty = if let Some(ref replacement) = self_replacement {
        substitute_self(&return_ty, replacement)
    } else {
        return_ty
    };

    // Instantiate callee type with substitutions.
    // This is the key fix: the callee's function type needs to have type parameters
    // replaced with their concrete types so that constraint generation can properly
    // unify closure types with the expected parameter types.
    let instantiated_callee = if !call_substitutions.is_empty() {
        let instantiated_ty = callee.ty.apply_substitutions(&call_substitutions);
        Expression {
            ty: instantiated_ty,
            ..callee
        }
    } else {
        callee
    };

    Expression::generic_call(
        instantiated_callee,
        arguments,
        call_substitutions,
        return_ty,
        span,
    )
}

/// Collect a single overload description from a symbol.
fn collect_single_overload_description(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> OverloadDescription {
    let name = symbol.metadata().name().value.clone();
    let callable = get_callable_behavior(symbol);

    match callable {
        Some(cb) => {
            let labels: Vec<Option<String>> = cb
                .parameters()
                .iter()
                .map(|p| p.external_label().map(|s| s.to_string()))
                .collect();
            let param_types: Vec<String> =
                cb.parameters().iter().map(|p| p.ty.to_string()).collect();

            OverloadDescription {
                name,
                labels,
                param_types,
                definition_span: Some(symbol.metadata().name().span.clone()),
                definition_file_id: None,
            }
        },
        None => OverloadDescription {
            name,
            labels: vec![],
            param_types: vec![],
            definition_span: Some(symbol.metadata().name().span.clone()),
            definition_file_id: None,
        },
    }
}

/// Resolve an overloaded function call by matching arity + labels
fn resolve_overloaded_call(
    candidates: &[SymbolId],
    callee: Expression,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Find the matching overload
    for &candidate_id in candidates {
        if let Some(symbol) = ctx.model.query(SymbolFor { id: candidate_id })
            && let Some(callable) = get_callable_behavior(&symbol)
            && matches_signature(&callable, arguments.len(), arg_labels)
        {
            // Validate access modes for arguments
            validate_argument_access_modes(&callable, &arguments, &span, ctx);

            let return_ty = callable.return_type().clone();
            // Functions are not mutable lvalues
            let resolved_callee =
                Expression::symbol_ref(candidate_id, callee.ty.clone(), false, callee.span.clone());
            return Expression::call(resolved_callee, arguments, return_ty, span);
        }
    }

    // No match found - collect overload info for error message
    let function_name = get_function_name_from_candidates(candidates, ctx.model);
    let available_overloads = collect_overload_descriptions(candidates, ctx.model);

    let error = NoMatchingOverloadError {
        call_span: span.clone(),
        name: function_name,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_overloads,
    };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());

    Expression::error(span)
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
        // Has explicit initializers - find matching one
        return resolve_explicit_init_call(
            &explicit_inits,
            arguments,
            arg_labels,
            explicit_type_args,
            span,
            symbol.clone(),
            ctx,
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

/// Resolve a call to an explicit initializer
fn resolve_explicit_init_call(
    initializers: &[Arc<dyn Symbol<KestrelLanguage>>],
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    explicit_type_args: Option<Vec<Ty>>,
    span: Span,
    struct_symbol: Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    // Collect all matching initializers by arity and labels
    let mut candidates: Vec<(usize, &Arc<dyn Symbol<KestrelLanguage>>, CallableBehavior)> =
        Vec::new();
    for (idx, init_sym) in initializers.iter().enumerate() {
        if let Some(callable) = get_callable_behavior(init_sym)
            && matches_signature(&callable, arguments.len(), arg_labels)
        {
            candidates.push((idx, init_sym, callable));
        }
    }

    // If multiple candidates, try type-directed conformance selection
    let selected_idx = if candidates.len() > 1 {
        // Extract argument types for type-directed selection
        let arg_types: Vec<Ty> = arguments.iter().map(|arg| arg.value.ty.clone()).collect();

        // Try to find a match based on conformance type arguments
        find_type_directed_match(&candidates, &arg_types, &struct_symbol).unwrap_or(0) // Fall back to first match if no type-directed match
    } else {
        0
    };

    // Select the initializer
    if let Some((_, init_sym, callable)) = candidates.get(selected_idx) {
        // Found matching initializer
        // The return type is the actual struct type
        // Create a struct type from the struct symbol
        // If explicit type arguments are provided, use them; otherwise infer
        let struct_ty = if let Some(ref type_args) = explicit_type_args {
            create_struct_type_with_type_args(&struct_symbol, type_args, span.clone(), ctx)
        } else {
            create_struct_type(&struct_symbol, span.clone())
        };

        // For explicit init, create a Call expression
        // Initializers are not mutable lvalues
        let init_id = init_sym.metadata().id();

        // Get substitutions from the struct type to apply to initializer parameters.
        // This maps the struct's type parameters (e.g., Slice's T) to the instantiation
        // type arguments (e.g., inference placeholders or explicit type args).
        let struct_subs = match struct_ty.kind() {
            TyKind::Struct { substitutions, .. } => substitutions.clone(),
            _ => Substitutions::new(),
        };

        // Check if the initializer has its own type parameters (e.g., init[From](from: From))
        // If so, we need to infer them from argument types and validate constraints.
        let init_subs =
            if let Some(init_symbol) = init_sym.as_any().downcast_ref::<InitializerSymbol>() {
                let init_type_params = init_symbol.type_parameters();

                if !init_type_params.is_empty() {
                    // Collect argument types for inference
                    let arg_types: Vec<Ty> = arguments.iter().map(|a| a.value.ty.clone()).collect();

                    // Infer type arguments from argument types
                    let mut init_substitutions =
                        infer_type_arguments(&init_type_params, callable, &arg_types);

                    // Build inferred type args, using Infer for parameters that couldn't be determined
                    let inferred_args: Vec<Ty> = init_type_params
                        .iter()
                        .map(|tp| {
                            let tp_id = tp.metadata().id();
                            if let Some(inferred_ty) = init_substitutions.get(tp_id) {
                                inferred_ty.clone()
                            } else {
                                // Create fresh inference variable for this type parameter
                                let infer_ty = Ty::infer(span.clone());
                                init_substitutions.insert(tp_id, infer_ty.clone());
                                infer_ty
                            }
                        })
                        .collect();

                    // Verify where clause constraints are satisfied
                    let where_clause = init_symbol.where_clause();
                    verify_type_argument_constraints(
                        &init_type_params,
                        &inferred_args,
                        &where_clause,
                        span.clone(),
                        ctx.model,
                        ctx.diagnostics,
                    );

                    init_substitutions
                } else {
                    Substitutions::new()
                }
            } else {
                Substitutions::new()
            };

        // Combine struct substitutions and initializer substitutions
        let mut combined_subs = struct_subs.clone();
        for (key, ty) in init_subs.iter() {
            combined_subs.insert(*key, ty.clone());
        }

        // Build the function type for the initializer, applying combined substitutions
        // to parameter types so that:
        // - Slice.init(pointer: Pointer[T], ...) becomes Slice.init(pointer: Pointer[Infer], ...)
        // - init[From](from: From) becomes init(from: ConcreteType)
        //
        // We also need to replace any type parameters that aren't in the substitution
        // map with inference placeholders - this handles cases where the callable's
        // parameter types use different TypeParameter symbols than the struct's.
        //
        // However, if the substitution maps to a TypeParameter (e.g., Pointer[T] where T
        // is from the caller's scope), we should NOT replace that with Infer - it's a
        // valid type parameter that should be preserved.
        //
        // Collect the IDs of type parameters that are substitution values - these should
        // be preserved (not replaced with Infer) after substitution.
        let preserved_type_params: std::collections::HashSet<SymbolId> = combined_subs
            .iter()
            .filter_map(|(_, ty)| {
                if let TyKind::TypeParameter(tp) = ty.kind() {
                    Some(tp.metadata().id())
                } else {
                    None
                }
            })
            .collect();

        let param_tys: Vec<Ty> = callable
            .parameters()
            .iter()
            .map(|p| {
                let ty = p.ty.apply_substitutions(&combined_subs);
                // Replace unsubstituted type params with Infer, but preserve type params
                // that came from substitution values (they're valid in the caller's scope)
                replace_type_params_except(&ty, &preserved_type_params, &span)
            })
            .collect();
        let init_fn_ty = Ty::function(param_tys, struct_ty.clone(), span.clone());

        let init_ref = Expression::symbol_ref(init_id, init_fn_ty, false, span.clone());
        // Use generic_call to store the combined substitutions so that CallableParamTypesForCall
        // can apply them when type-checking arguments
        return Expression::generic_call(init_ref, arguments, combined_subs, struct_ty, span);
    }

    // No matching initializer found - report error
    let struct_name = struct_symbol.metadata().name().value.clone();

    // Build list of available initializers for the error message
    let available_initializers: Vec<OverloadDescription> = initializers
        .iter()
        .filter_map(|init| {
            let callable = get_callable_behavior(init)?;
            let labels: Vec<Option<String>> = callable
                .parameters()
                .iter()
                .map(|p| p.label.as_ref().map(|l| l.value.clone()))
                .collect();
            let param_types: Vec<String> = callable
                .parameters()
                .iter()
                .map(|p| p.ty.to_string())
                .collect();
            Some(OverloadDescription {
                name: struct_name.clone(),
                labels,
                param_types,
                definition_span: Some(init.metadata().span().clone()),
                definition_file_id: Some(init.metadata().span().file_id),
            })
        })
        .collect();

    let error = NoMatchingInitializerError {
        span: span.clone(),
        struct_name,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_initializers,
    };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());

    Expression::error(span)
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
            if let Some(field) = c.as_ref().downcast_ref::<FieldSymbol>() {
                !field.is_computed() && !field.is_static()
            } else {
                true // Include if we can't downcast (shouldn't happen)
            }
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

/// Resolve a method call from a MethodRef expression
pub fn resolve_method_call(
    receiver: &Expression,
    candidates: &[SymbolId],
    method_name: &str,
    arguments: Vec<CallArgument>,
    arg_labels: &[Option<String>],
    explicit_type_args: Option<Vec<Ty>>,
    span: Span,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    use super::utils::substitute_self;
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    // Find matching overload
    let mut invisible_matches = Vec::new();

    for &candidate_id in candidates {
        if let Some(symbol) = ctx.model.query(SymbolFor { id: candidate_id })
            && let Some(callable) = get_callable_behavior(&symbol)
            && matches_signature(&callable, arguments.len(), arg_labels)
        {
            // Check visibility
            if !ctx.model.query(IsVisibleFrom {
                target: candidate_id,
                context: ctx.function_id,
            }) {
                invisible_matches.push(symbol);
                continue;
            }

            // Build substitutions from the receiver type
            // e.g., for Box[Int], we get {T -> Int}
            // For static methods (TypeRef receiver), get the struct type from the symbol
            // For instance methods, resolve Self to concrete type
            use super::members::resolve_self_type_to_concrete;
            use kestrel_semantic_tree::behavior::typed::TypedBehavior;
            let resolved_receiver_ty = match &receiver.kind {
                ExprKind::TypeRef(type_symbol_id) => {
                    // For TypeRef, check if receiver.ty has explicit type arguments
                    // (from qualified path like Box[lang.i64].wrap or Pointer[T](...))
                    // ANY non-empty substitutions means type args were explicitly written,
                    // even if those types are type parameters (like T in a generic context)
                    let has_explicit_type_args = match receiver.ty.kind() {
                        TyKind::Struct { substitutions, .. }
                        | TyKind::Enum { substitutions, .. } => {
                            // Non-empty substitutions = explicit type args were provided
                            // This includes both concrete types (lang.i64) and type params (T)
                            !substitutions.is_empty()
                        },
                        _ => false,
                    };

                    if has_explicit_type_args {
                        // Use receiver.ty directly - it has the qualified type with explicit type args
                        receiver.ty.clone()
                    } else {
                        // Get the base type from the symbol (for initializers, unspecialized generics, etc.)
                        if let Some(type_sym) = ctx.model.query(SymbolFor {
                            id: *type_symbol_id,
                        }) {
                            if let Some(typed) = type_sym.metadata().get_behavior::<TypedBehavior>()
                            {
                                typed.ty().clone()
                            } else {
                                receiver.ty.clone()
                            }
                        } else {
                            receiver.ty.clone()
                        }
                    }
                },
                _ => resolve_self_type_to_concrete(&receiver.ty, ctx), // Instance method
            };
            // Expand type aliases to get the underlying type with substitutions
            // e.g., OptionalTypeOperator[Int] -> Optional[Int]
            let resolved_receiver_ty = resolved_receiver_ty.expand_aliases();

            // Get return type, substituting Self with the resolved receiver type
            let mut return_ty = substitute_self(callable.return_type(), &resolved_receiver_ty);
            let mut call_substitutions = Substitutions::new();
            let mut resolved_receiver_ty = resolved_receiver_ty;

            // Helper: collect type parameters from the callable's parameter types
            fn collect_callable_type_params(
                callable: &CallableBehavior,
            ) -> Vec<Arc<TypeParameterSymbol>> {
                let mut type_params = Vec::new();
                for param in callable.parameters() {
                    collect_type_params_from_ty(&param.ty, &mut type_params);
                }
                type_params
            }

            fn collect_type_params_from_ty(ty: &Ty, params: &mut Vec<Arc<TypeParameterSymbol>>) {
                match ty.kind() {
                    TyKind::TypeParameter(tp) => {
                        if !params
                            .iter()
                            .any(|p| p.metadata().id() == tp.metadata().id())
                        {
                            params.push(tp.clone());
                        }
                    },
                    TyKind::Struct { substitutions, .. } | TyKind::Enum { substitutions, .. } => {
                        for (_, sub_ty) in substitutions.iter() {
                            collect_type_params_from_ty(sub_ty, params);
                        }
                    },
                    TyKind::Array(elem) => collect_type_params_from_ty(elem, params),
                    TyKind::Tuple(elems) => {
                        for elem in elems {
                            collect_type_params_from_ty(elem, params);
                        }
                    },
                    TyKind::Function {
                        params: fn_params,
                        return_type,
                    } => {
                        for p in fn_params {
                            collect_type_params_from_ty(p, params);
                        }
                        collect_type_params_from_ty(return_type, params);
                    },
                    _ => {},
                }
            }

            // Check if this is a static method call (TypeRef receiver)
            let is_static_method = matches!(&receiver.kind, ExprKind::TypeRef(_));

            // Handle TypeParameterRef receiver (static method on type parameter)
            // e.g., T.create() where T: Factory[lang.i64]
            // We need to find the protocol bound that contains this method and apply its substitutions
            if let ExprKind::TypeParameterRef(type_param_id) = &receiver.kind {
                // Look up protocol bounds for this type parameter
                let bounds = get_type_parameter_bounds_by_id(*type_param_id, ctx);

                // Find the protocol that contains this method and get composed substitutions
                for bound in &bounds {
                    if let TyKind::Protocol {
                        symbol: proto,
                        substitutions: proto_subs,
                    } = bound.kind()
                    {
                        // Get composed substitutions tracing through inheritance
                        if let Some(composed_subs) =
                            get_method_protocol_substitutions(&symbol, proto, proto_subs)
                        {
                            // Apply composed substitutions to return type and callable
                            if !composed_subs.is_empty() {
                                return_ty = substitute_type(&return_ty, &composed_subs);
                                for (param_id, ty) in composed_subs.iter() {
                                    call_substitutions.insert(*param_id, ty.clone());
                                }
                            }
                            break;
                        }
                    }
                }
            }
            // Also handle instance method calls where receiver's TYPE is a TypeParameter
            // e.g., val.convert() where val: T and T: Converter[lang.i64]
            else if let TyKind::TypeParameter(type_param) = receiver.ty.kind() {
                // Look up protocol bounds for this type parameter
                let bounds = get_type_parameter_bounds_by_id(type_param.metadata().id(), ctx);

                // Find the protocol that contains this method and get composed substitutions
                for bound in &bounds {
                    if let TyKind::Protocol {
                        symbol: proto,
                        substitutions: proto_subs,
                    } = bound.kind()
                    {
                        // Get composed substitutions tracing through inheritance
                        if let Some(composed_subs) =
                            get_method_protocol_substitutions(&symbol, proto, proto_subs)
                        {
                            // Apply composed substitutions to return type and callable
                            if !composed_subs.is_empty() {
                                return_ty = substitute_type(&return_ty, &composed_subs);
                                for (param_id, ty) in composed_subs.iter() {
                                    call_substitutions.insert(*param_id, ty.clone());
                                }
                            }
                            break;
                        }
                    }
                }
            }

            // Handle struct receiver types (e.g., Box[Int])
            if let Some((struct_sym, substitutions)) = resolved_receiver_ty.as_struct_with_subs() {
                // Check if we need type inference (only for static methods):
                // Only when struct is generic but has EMPTY substitutions (Box.wrap case)
                // If substitutions are non-empty (even with type params like T), we use them directly
                let needs_inference = is_static_method
                    && substitutions.is_empty()
                    && !struct_sym.type_parameters().is_empty();

                if needs_inference {
                    // Get type parameters from the callable's parameter types
                    // These are the extension's type params, not the struct's
                    let callable_type_params = collect_callable_type_params(&callable);

                    if !callable_type_params.is_empty() {
                        // Infer type params from method arguments
                        let arg_types: Vec<Ty> =
                            arguments.iter().map(|a| a.value.ty.clone()).collect();
                        let inferred =
                            infer_type_arguments(&callable_type_params, &callable, &arg_types);

                        // Build new substitutions with inferred types
                        for (param_id, ty) in inferred.iter() {
                            call_substitutions.insert(*param_id, ty.clone());
                        }

                        if !substitutions.is_empty() {
                            // Map struct type params through the substitution chain
                            // substitutions maps: struct's T -> extension's T
                            // inferred maps: extension's T -> concrete type
                            // We need: struct's T -> concrete type
                            for (struct_param_id, ext_ty) in substitutions.iter() {
                                let resolved_ty = ext_ty.apply_substitutions(&call_substitutions);
                                call_substitutions.insert(*struct_param_id, resolved_ty);
                            }
                        } else {
                            // Empty substitutions (generic struct without explicit type args)
                            // Map callable type params to struct type params positionally
                            let struct_type_params = struct_sym.type_parameters();
                            for (callable_tp, struct_tp) in
                                callable_type_params.iter().zip(struct_type_params.iter())
                            {
                                if let Some(inferred_ty) = inferred.get(callable_tp.metadata().id())
                                {
                                    call_substitutions
                                        .insert(struct_tp.metadata().id(), inferred_ty.clone());
                                }
                            }
                        }

                        // Update resolved_receiver_ty with inferred types
                        resolved_receiver_ty =
                            resolved_receiver_ty.apply_substitutions(&call_substitutions);
                        return_ty = callable
                            .return_type()
                            .apply_substitutions(&call_substitutions);
                    }
                } else {
                    // Add receiver's substitutions to call_substitutions
                    for (param_id, ty) in substitutions.iter() {
                        call_substitutions.insert(*param_id, ty.clone());
                    }
                    return_ty = return_ty.apply_substitutions(substitutions);
                }
            }
            // Handle enum receiver types (e.g., Optional[Int])
            else if let Some((enum_sym, substitutions)) = resolved_receiver_ty.as_enum_with_subs()
            {
                // Check if we need type inference (only for static methods):
                // Only when enum is generic but has EMPTY substitutions
                let needs_inference = is_static_method
                    && substitutions.is_empty()
                    && !enum_sym.type_parameters().is_empty();

                if needs_inference {
                    let callable_type_params = collect_callable_type_params(&callable);

                    if !callable_type_params.is_empty() {
                        let arg_types: Vec<Ty> =
                            arguments.iter().map(|a| a.value.ty.clone()).collect();
                        let inferred =
                            infer_type_arguments(&callable_type_params, &callable, &arg_types);

                        for (param_id, ty) in inferred.iter() {
                            call_substitutions.insert(*param_id, ty.clone());
                        }

                        if !substitutions.is_empty() {
                            for (enum_param_id, ext_ty) in substitutions.iter() {
                                let resolved_ty = ext_ty.apply_substitutions(&call_substitutions);
                                call_substitutions.insert(*enum_param_id, resolved_ty);
                            }
                        } else {
                            // Empty substitutions - map positionally
                            let enum_type_params = enum_sym.type_parameters();
                            for (callable_tp, enum_tp) in
                                callable_type_params.iter().zip(enum_type_params.iter())
                            {
                                if let Some(inferred_ty) = inferred.get(callable_tp.metadata().id())
                                {
                                    call_substitutions
                                        .insert(enum_tp.metadata().id(), inferred_ty.clone());
                                }
                            }
                        }

                        resolved_receiver_ty =
                            resolved_receiver_ty.apply_substitutions(&call_substitutions);
                        return_ty = callable
                            .return_type()
                            .apply_substitutions(&call_substitutions);
                    }
                } else {
                    for (param_id, ty) in substitutions.iter() {
                        call_substitutions.insert(*param_id, ty.clone());
                    }
                    return_ty = return_ty.apply_substitutions(substitutions);
                }
            }

            // Infer type parameters from argument types if method has its own type parameters
            // This handles cases like Optional.map[U](transform: (T) -> U) where U needs
            // to be inferred from the closure's return type
            if explicit_type_args.is_none() {
                use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
                if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>()
                    && let Some(generics) = func_sym.metadata().get_behavior::<GenericsBehavior>()
                {
                    let method_type_params = generics.type_parameters();
                    if !method_type_params.is_empty() {
                        let arg_types: Vec<Ty> =
                            arguments.iter().map(|a| a.value.ty.clone()).collect();
                        // Use infer_type_arguments which handles nested types like (T) -> U
                        let method_subs =
                            infer_type_arguments(method_type_params, &callable, &arg_types);
                        for (param_id, ty) in method_subs.iter() {
                            call_substitutions.insert(*param_id, ty.clone());
                        }
                        // Reapply substitutions to return type with inferred types
                        if !method_subs.is_empty() {
                            return_ty = callable.return_type().clone();
                            return_ty = return_ty.apply_substitutions(&call_substitutions);
                        }
                    }
                }
            }

            // Add explicit type arguments to substitutions and apply to return type
            if let Some(ref type_args) = explicit_type_args
                && let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>()
            {
                let type_params = func_sym.type_parameters();
                for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
                    call_substitutions.insert(param.metadata().id(), arg_ty.clone());
                }
                return_ty = substitute_type(&return_ty, &call_substitutions);
            }

            // Validate access modes for arguments
            validate_argument_access_modes(&callable, &arguments, &span, ctx);

            // Compute the function type with substitutions applied
            // Must substitute both type parameters AND Self
            let method_fn_ty = {
                let param_tys: Vec<Ty> = callable
                    .parameters()
                    .iter()
                    .map(|p| {
                        let ty = substitute_type(&p.ty, &call_substitutions);
                        substitute_self(&ty, &resolved_receiver_ty)
                    })
                    .collect();
                let ret_ty = substitute_type(&return_ty, &call_substitutions);
                Ty::function(param_tys, ret_ty, span.clone())
            };

            // Create method ref with the correct function type
            let method_ref = Expression {
                id: ExprId::new(),
                kind: ExprKind::MethodRef {
                    receiver: Box::new(receiver.clone()),
                    candidates: vec![candidate_id],
                    method_name: method_name.to_string(),
                },
                ty: method_fn_ty,
                span: span.clone(),
                mutable: false,
            };

            return Expression::generic_call(
                method_ref,
                arguments,
                call_substitutions,
                return_ty,
                span,
            );
        }
    }

    // No matching visible method found
    if !invisible_matches.is_empty() {
        let first_invisible = &invisible_matches[0];
        let visibility = first_invisible
            .metadata()
            .get_behavior::<kestrel_semantic_tree::behavior::visibility::VisibilityBehavior>()
            .and_then(|v| v.visibility().map(|vis| vis.to_string()))
            .unwrap_or_else(|| "internal".to_string());

        let error = MemberNotVisibleError {
            member_span: span.clone(), // Could be more precise if we had the member name span
            member_name: method_name.to_string(),
            base_span: receiver.span.clone(),
            base_type: receiver.ty.to_string(),
            visibility,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // No matching method found at all - collect overload info for error message
    let receiver_type = receiver.ty.to_string();
    let available_overloads = collect_overload_descriptions(candidates, ctx.model);

    let error = NoMatchingMethodError {
        call_span: span.clone(),
        method_name: method_name.to_string(),
        receiver_type,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_overloads,
    };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());

    Expression::error(span)
}

/// Get the function name from a list of candidate symbol IDs.
fn get_function_name_from_candidates(candidates: &[SymbolId], model: &SemanticModel) -> String {
    for &candidate_id in candidates {
        if let Some(symbol) = model.query(SymbolFor { id: candidate_id }) {
            return symbol.metadata().name().value.clone();
        }
    }
    "<unknown>".to_string()
}

/// Collect overload descriptions from a list of candidate symbol IDs.
pub fn collect_overload_descriptions(
    candidates: &[SymbolId],
    model: &SemanticModel,
) -> Vec<OverloadDescription> {
    let mut descriptions = Vec::new();

    for &candidate_id in candidates {
        if let Some(symbol) = model.query(SymbolFor { id: candidate_id })
            && let Some(callable) = get_callable_behavior(&symbol)
        {
            let name = symbol.metadata().name().value.clone();
            let labels: Vec<Option<String>> = callable
                .parameters()
                .iter()
                .map(|p| p.external_label().map(|s| s.to_string()))
                .collect();
            let param_types: Vec<String> = callable
                .parameters()
                .iter()
                .map(|p| p.ty.to_string())
                .collect();

            descriptions.push(OverloadDescription {
                name,
                labels,
                param_types,
                definition_span: Some(symbol.metadata().name().span.clone()),
                definition_file_id: None, // TODO: Get file ID from symbol
            });
        }
    }

    descriptions
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

/// Find the substitutions needed to use a method from a protocol, tracing through inheritance.
///
/// Returns Some(substitutions) if the method is from the protocol or its ancestors,
/// None if the method is not found in this protocol's hierarchy.
fn get_method_protocol_substitutions(
    method: &Arc<dyn Symbol<KestrelLanguage>>,
    protocol: &Arc<ProtocolSymbol>,
    base_substitutions: &Substitutions,
) -> Option<Substitutions> {
    // Get the method's parent protocol
    let method_parent = method.metadata().parent()?;
    if method_parent.metadata().kind() != KestrelSymbolKind::Protocol {
        return None;
    }
    let method_parent_id = method_parent.metadata().id();
    let protocol_id = protocol.metadata().id();

    // Direct parent check - method is directly from this protocol
    if method_parent_id == protocol_id {
        return Some(base_substitutions.clone());
    }

    // Check if method comes from an inherited protocol
    // Trace through the inheritance to find the path and compose substitutions
    if let Some(conformances) = protocol.metadata().get_behavior::<ConformancesBehavior>() {
        for parent_ty in conformances.conformances() {
            if let TyKind::Protocol {
                symbol: parent,
                substitutions: parent_subs,
            } = parent_ty.kind()
            {
                // Compose substitutions: apply our base_substitutions to the parent's type args
                let composed = compose_substitutions(base_substitutions, parent_subs);

                if parent.metadata().id() == method_parent_id {
                    // Found the method's protocol directly
                    return Some(composed);
                }

                // Recursively check
                if let Some(result) = get_method_protocol_substitutions(method, parent, &composed) {
                    return Some(result);
                }
            }
        }
    }

    None
}

/// Check if a method symbol belongs to a protocol (directly or through inheritance).
#[allow(dead_code)]
fn method_is_from_protocol(
    method: &Arc<dyn Symbol<KestrelLanguage>>,
    protocol: &Arc<ProtocolSymbol>,
) -> bool {
    get_method_protocol_substitutions(method, protocol, &Substitutions::new()).is_some()
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
            let substituted = substitute_callable_self(&callable, self_replacement);
            // Apply protocol type parameter substitutions
            let substituted =
                substitute_callable_with_substitutions(&substituted, protocol_substitutions);

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
                let composed_subs = compose_substitutions(protocol_substitutions, parent_subs);
                collect_protocol_initializers(parent, self_replacement, &composed_subs, candidates);
            }
        }
    }
}

/// Substitute type parameters in a CallableBehavior using protocol substitutions.
fn substitute_callable_with_substitutions(
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
fn compose_substitutions(outer: &Substitutions, inner: &Substitutions) -> Substitutions {
    let mut result = Substitutions::new();
    for (id, ty) in inner.iter() {
        result.insert(*id, substitute_type(ty, outer));
    }
    result
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
pub(crate) fn classify_mutability(expr: &Expression, ctx: &BodyResolutionContext) -> MutabilityClassification {
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
    // Look up the field in the object's type
    if let Some((struct_symbol, _)) = object.ty.as_struct_with_subs() {
        for child in struct_symbol.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Field
                && child.metadata().name().value == current_field
            {
                if let Some(field_sym) = child.as_any().downcast_ref::<FieldSymbol>()
                    && !field_sym.is_mutable()
                {
                    // The field itself is immutable (let field)
                    return MutabilityClassification::ImmutableField {
                        field_name: current_field.to_string(),
                        field_span: Some(child.metadata().name().span.clone()),
                    };
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
