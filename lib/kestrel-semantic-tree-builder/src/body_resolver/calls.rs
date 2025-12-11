//! Call expression resolution.
//!
//! This module handles resolving function calls, method calls, overloaded calls,
//! and struct instantiation (both explicit and implicit initializers).

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::expr::{CallArgument, Expression, ExprKind};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolution::type_resolver::TypeResolver;

use crate::diagnostics::{
    AmbiguousTypeParameterInitError, FieldNotVisibleForInitError, ImplicitInitArityError,
    ImplicitInitLabelError, InstanceMethodOnTypeError, NoInitInTypeParameterBoundsError,
    NoMatchingInitializerError, NoMatchingMethodError, NoMatchingOverloadError,
    NoMatchingTypeParameterInitError, NotGenericError, OverloadDescription,
    TooFewTypeArgumentsError, TooManyTypeArgumentsError, TypeArgsOnNonGenericError,
    UnconstrainedTypeParameterMemberError,
};
use crate::resolution::visibility::is_visible_from;
use crate::database::Db;
use crate::syntax::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::members::{resolve_member_call, substitute_callable_self};
use super::utils::{
    create_generic_struct_type, create_struct_type, create_struct_type_with_type_args, format_type,
    get_callable_behavior, get_type_parameter_bounds_by_id, is_expression_kind, matches_signature,
    substitute_type, validate_not_standalone_type_param, verify_type_argument_constraints,
    infer_type_arguments,
};

/// Resolve a call expression: callee(arg1, arg2, ...) or callee[T](arg1, ...)
pub fn resolve_call_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.source);

    // Find the callee expression (first child that's an Expression)
    let callee_node = match node.children().find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind())) {
        Some(n) => n,
        None => return Expression::error(span.clone()),
    };

    // Extract explicit type arguments from the callee (e.g., foo[Int] or obj.method[T])
    let explicit_type_args = extract_type_arguments_from_callee(&callee_node, ctx);

    // Find the argument list
    let arg_list_node = node.children().find(|c| c.kind() == SyntaxKind::ArgumentList);

    // Resolve callee first
    let callee = resolve_expression(&callee_node, ctx);

    // Parse arguments
    let arguments = if let Some(arg_list) = arg_list_node {
        resolve_argument_list(&arg_list, ctx)
    } else {
        vec![]
    };

    // Get labels for overload resolution (owned strings)
    let arg_labels: Vec<Option<String>> = arguments.iter()
        .map(|a| a.label.clone())
        .collect();

    // Now resolve based on callee type
    resolve_call(callee, arguments, &arg_labels, explicit_type_args, span, ctx)
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
                if let Some(token) = child.as_token() {
                    if token.kind() == SyntaxKind::Dot {
                        last_dot_pos = Some(i);
                    }
                }
            }

            // If there's a dot, only look for TypeArgumentList AFTER the last dot
            if let Some(dot_pos) = last_dot_pos {
                for child in children.iter().skip(dot_pos + 1) {
                    if let Some(node) = child.as_node() {
                        if node.kind() == SyntaxKind::TypeArgumentList {
                            return Some(node.clone());
                        }
                    }
                }
                // Multi-segment path but no type args after last dot
                return None;
            }

            // No dot - single segment path, check for direct TypeArgumentList
            for child in children.iter() {
                if let Some(node) = child.as_node() {
                    if node.kind() == SyntaxKind::TypeArgumentList {
                        return Some(node.clone());
                    }
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
        for child in node.children() {
            if child.kind() == SyntaxKind::TypeArgumentList {
                return Some(child);
            }
        }

        None
    }

    let type_arg_list = find_type_args_on_final_segment(callee_node)?;

    // Resolve each type in the TypeArgumentList
    let mut type_args = Vec::new();

    for child in type_arg_list.children() {
        if child.kind() == SyntaxKind::Ty {
            let mut resolver = TypeResolver::new(
                ctx.db,
                ctx.diagnostics,
                ctx.file_id,
                ctx.source,
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
fn resolve_argument_list(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Vec<CallArgument> {
    let mut arguments = Vec::new();

    for child in node.children() {
        if child.kind() == SyntaxKind::Argument {
            if let Some(arg) = resolve_argument(&child, ctx) {
                arguments.push(arg);
            }
        }
    }

    arguments
}

/// Resolve a single argument node
fn resolve_argument(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<CallArgument> {
    let span = get_node_span(node, ctx.source);

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
    let value_node = node.children()
        .find(|c| c.kind() == SyntaxKind::Expression || is_expression_kind(c.kind()))?;

    let value = resolve_expression(&value_node, ctx);
    let value = validate_not_standalone_type_param(value, ctx);

    Some(CallArgument::unlabeled(value, span))
        .map(|mut arg| {
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
    // Clone callee.kind to avoid borrow issues
    let callee_kind = callee.kind.clone();
    let callee_ty = callee.ty.clone();

    match callee_kind {
        // Direct function reference
        ExprKind::SymbolRef(symbol_id) => {
            resolve_single_function_call(symbol_id, callee, arguments, explicit_type_args, span, ctx)
        }

        // Overloaded function reference - need to pick one
        ExprKind::OverloadedRef(ref candidates) => {
            resolve_overloaded_call(candidates, callee, arguments, arg_labels, span, ctx)
        }

        // Method reference (from member access on a type)
        ExprKind::MethodRef { ref receiver, ref candidates, ref method_name } => {
            resolve_method_call(receiver, candidates, method_name, arguments, arg_labels, explicit_type_args, span, ctx)
        }

        // Field access - might be method call on struct
        ExprKind::FieldAccess { ref object, ref field } => {
            // This could be:
            // 1. A field with callable type (first-class function)
            // 2. A method call
            resolve_member_call(object, field, arguments, arg_labels, span, ctx)
        }

        // Type reference - struct instantiation
        ExprKind::TypeRef(symbol_id) => {
            resolve_struct_instantiation(symbol_id, arguments, arg_labels, explicit_type_args, span, ctx)
        }

        // Type parameter reference - init call on type parameter (T())
        ExprKind::TypeParameterRef(symbol_id) => {
            resolve_type_parameter_init_call(symbol_id, arguments, arg_labels, span, ctx)
        }

        // Local variable reference - could be calling a function stored in a variable
        ExprKind::LocalRef(_local_id) => {
            // Variables cannot have explicit type arguments
            if let Some(ref type_args) = explicit_type_args {
                if !type_args.is_empty() {
                    ctx.diagnostics.add_diagnostic(
                        TypeArgsOnNonGenericError {
                            span: span.clone(),
                            callee_description: "a variable".to_string(),
                        }
                        .into_diagnostic(ctx.file_id),
                    );
                    return Expression::error(span);
                }
            }

            // Check if the type is callable
            if let TyKind::Function { return_type, .. } = callee_ty.kind() {
                Expression::call(callee, arguments, (**return_type).clone(), span)
            } else {
                // TODO: Report error: trying to call non-callable
                Expression::error(span)
            }
        }

        // Any other expression - check if callable type
        _ => {
            // Non-function expressions cannot have explicit type arguments
            if let Some(ref type_args) = explicit_type_args {
                if !type_args.is_empty() {
                    ctx.diagnostics.add_diagnostic(
                        TypeArgsOnNonGenericError {
                            span: span.clone(),
                            callee_description: "this expression".to_string(),
                        }
                        .into_diagnostic(ctx.file_id),
                    );
                    return Expression::error(span);
                }
            }

            if let TyKind::Function { return_type, .. } = callee_ty.kind() {
                Expression::call(callee, arguments, (**return_type).clone(), span)
            } else {
                // TODO: Report error: expression is not callable
                Expression::error(span)
            }
        }
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
    let Some(symbol) = ctx.db.symbol_by_id(symbol_id) else {
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Check arity and labels
    let arg_labels: Vec<Option<String>> = arguments.iter()
        .map(|a| a.label.clone())
        .collect();

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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

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
                    .into_diagnostic(ctx.file_id),
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
                    .into_diagnostic(ctx.file_id),
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
                    .into_diagnostic(ctx.file_id),
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
                ctx.db,
                ctx.file_id,
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
                let substitutions = infer_type_arguments(&type_params, &callable, &arg_types);

                // Build inferred type args in order for constraint verification
                let inferred_args: Vec<Ty> = type_params
                    .iter()
                    .map(|tp| {
                        substitutions
                            .get(tp.metadata().id())
                            .cloned()
                            .unwrap_or_else(|| Ty::inferred(span.clone()))
                    })
                    .collect();

                // Verify constraints are satisfied
                let where_clause = func_sym.where_clause();
                verify_type_argument_constraints(
                    &type_params,
                    &inferred_args,
                    &where_clause,
                    span.clone(),
                    ctx.db,
                    ctx.file_id,
                    ctx.diagnostics,
                );

                // Apply substitution to return type
                let return_ty = substitute_type(callable.return_type(), &substitutions);
                (return_ty, substitutions)
            } else {
                (callable.return_type().clone(), Substitutions::new())
            }
        } else {
            (callable.return_type().clone(), Substitutions::new())
        }
    };

    Expression::generic_call(callee, arguments, call_substitutions, return_ty, span)
}

/// Collect a single overload description from a symbol.
fn collect_single_overload_description(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> OverloadDescription {
    let name = symbol.metadata().name().value.clone();
    let callable = get_callable_behavior(symbol);

    match callable {
        Some(cb) => {
            let labels: Vec<Option<String>> = cb
                .parameters()
                .iter()
                .map(|p| p.external_label().map(|s| s.to_string()))
                .collect();
            let param_types: Vec<String> = cb
                .parameters()
                .iter()
                .map(|p| format_type(&p.ty))
                .collect();

            OverloadDescription {
                name,
                labels,
                param_types,
                definition_span: Some(symbol.metadata().name().span.clone()),
                definition_file_id: None,
            }
        }
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
        if let Some(symbol) = ctx.db.symbol_by_id(candidate_id) {
            if let Some(callable) = get_callable_behavior(&symbol) {
                if matches_signature(&callable, arguments.len(), arg_labels) {
                    let return_ty = callable.return_type().clone();
                    // Functions are not mutable lvalues
                    let resolved_callee = Expression::symbol_ref(candidate_id, callee.ty.clone(), false, callee.span.clone());
                    return Expression::call(resolved_callee, arguments, return_ty, span);
                }
            }
        }
    }

    // No match found - collect overload info for error message
    let function_name = get_function_name_from_candidates(candidates, ctx.db);
    let available_overloads = collect_overload_descriptions(candidates, ctx.db);

    let error = NoMatchingOverloadError {
        call_span: span.clone(),
        name: function_name,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_overloads,
    };
    ctx.diagnostics
        .add_diagnostic(error.into_diagnostic(ctx.file_id));

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
    let Some(symbol) = ctx.db.symbol_by_id(symbol_id) else {
        return Expression::error(span);
    };

    // Verify it's a struct
    if symbol.metadata().kind() != KestrelSymbolKind::Struct {
        // Not a struct - cannot instantiate
        // TODO: Add proper error diagnostic
        return Expression::error(span);
    }

    // Verify it can be downcast to StructSymbol
    if symbol.as_ref().downcast_ref::<StructSymbol>().is_none() {
        return Expression::error(span);
    }

    // Check for explicit initializers
    let explicit_inits: Vec<Arc<dyn Symbol<KestrelLanguage>>> = symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Initializer)
        .collect();

    if !explicit_inits.is_empty() {
        // Has explicit initializers - find matching one
        return resolve_explicit_init_call(&explicit_inits, arguments, arg_labels, explicit_type_args, span, symbol.clone(), ctx);
    }

    // No explicit initializers - try implicit memberwise init
    resolve_implicit_init(symbol_id, arguments, arg_labels, explicit_type_args, span, symbol.clone(), ctx)
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
    // Find matching initializer by arity and labels
    for init_sym in initializers {
        if let Some(callable) = get_callable_behavior(init_sym) {
            if matches_signature(&callable, arguments.len(), arg_labels) {
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
                let init_ref = Expression::symbol_ref(init_id, Ty::inferred(span.clone()), false, span.clone());
                return Expression::call(init_ref, arguments, struct_ty, span);
            }
        }
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
                .map(|p| format_type(&p.ty))
                .collect();
            Some(OverloadDescription {
                name: struct_name.clone(),
                labels,
                param_types,
                definition_span: Some(init.metadata().span().clone()),
                definition_file_id: Some(ctx.file_id),
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
    ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));

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

    // Collect fields in declaration order
    let fields: Vec<Arc<dyn Symbol<KestrelLanguage>>> = struct_symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
        .collect();

    let field_names: Vec<String> = fields
        .iter()
        .map(|f| f.metadata().name().value.clone())
        .collect();

    // Check visibility of all fields
    let context_sym = ctx.db.symbol_by_id(ctx.function_id);
    for field in &fields {
        if let Some(ref ctx_sym) = context_sym {
            if !is_visible_from(field, ctx_sym) {
                // Field is not visible - cannot use implicit init
                let error = FieldNotVisibleForInitError {
                    span: span.clone(),
                    struct_name: struct_name.clone(),
                    field_name: field.metadata().name().value.clone(),
                    field_visibility: "private".to_string(), // TODO: Get actual visibility
                };
                ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
                return Expression::error(span);
            }
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
        ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
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
            ctx.diagnostics.add_diagnostic(error.into_diagnostic(ctx.file_id));
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
    for &candidate_id in candidates {
        if let Some(symbol) = ctx.db.symbol_by_id(candidate_id) {
            if let Some(callable) = get_callable_behavior(&symbol) {
                if matches_signature(&callable, arguments.len(), arg_labels) {
                    // Check visibility
                    if let Some(context_sym) = ctx.db.symbol_by_id(ctx.function_id) {
                        if !is_visible_from(&symbol, &context_sym) {
                            // TODO: Report error: method not visible
                            continue;
                        }
                    }

                    // Get return type, substituting Self with receiver type if needed
                    let mut return_ty = substitute_self(callable.return_type(), &receiver.ty);

                    // Build substitutions from the receiver type
                    // e.g., for Box[Int], we get {T -> Int}
                    use super::members::resolve_self_type_to_concrete;
                    let resolved_receiver_ty = resolve_self_type_to_concrete(&receiver.ty, ctx);
                    let mut call_substitutions = Substitutions::new();
                    if let Some((_, substitutions)) = resolved_receiver_ty.as_struct_with_subs() {
                        // Add receiver's substitutions to call_substitutions
                        for (param_id, ty) in substitutions.iter() {
                            call_substitutions.insert(*param_id, ty.clone());
                        }
                        return_ty = return_ty.apply_substitutions(substitutions);
                    }

                    // Add explicit type arguments to substitutions and apply to return type
                    if let Some(ref type_args) = explicit_type_args {
                        if let Some(func_sym) = symbol.as_any().downcast_ref::<FunctionSymbol>() {
                            let type_params = func_sym.type_parameters();
                            for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
                                call_substitutions.insert(param.metadata().id(), arg_ty.clone());
                            }
                            return_ty = substitute_type(&return_ty, &call_substitutions);
                        }
                    }

                    // Create method ref and then call
                    let method_ref = Expression::method_ref(
                        receiver.clone(),
                        vec![candidate_id],
                        method_name.to_string(),
                        span.clone(),
                    );

                    return Expression::generic_call(method_ref, arguments, call_substitutions, return_ty, span);
                }
            }
        }
    }

    // No matching method found - collect overload info for error message
    let receiver_type = format_type(&receiver.ty);
    let available_overloads = collect_overload_descriptions(candidates, ctx.db);

    let error = NoMatchingMethodError {
        call_span: span.clone(),
        method_name: method_name.to_string(),
        receiver_type,
        provided_labels: arg_labels.to_vec(),
        provided_arity: arguments.len(),
        available_overloads,
    };
    ctx.diagnostics
        .add_diagnostic(error.into_diagnostic(ctx.file_id));

    Expression::error(span)
}

/// Get the function name from a list of candidate symbol IDs.
fn get_function_name_from_candidates(candidates: &[SymbolId], db: &dyn Db) -> String {
    for &candidate_id in candidates {
        if let Some(symbol) = db.symbol_by_id(candidate_id) {
            return symbol.metadata().name().value.clone();
        }
    }
    "<unknown>".to_string()
}

/// Collect overload descriptions from a list of candidate symbol IDs.
pub fn collect_overload_descriptions(candidates: &[SymbolId], db: &dyn Db) -> Vec<OverloadDescription> {
    let mut descriptions = Vec::new();

    for &candidate_id in candidates {
        if let Some(symbol) = db.symbol_by_id(candidate_id) {
            if let Some(callable) = get_callable_behavior(&symbol) {
                let name = symbol.metadata().name().value.clone();
                let labels: Vec<Option<String>> = callable
                    .parameters()
                    .iter()
                    .map(|p| p.external_label().map(|s| s.to_string()))
                    .collect();
                let param_types: Vec<String> = callable
                    .parameters()
                    .iter()
                    .map(|p| format_type(&p.ty))
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
    let Some(symbol) = ctx.db.symbol_by_id(symbol_id) else {
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Collect init methods from all protocol bounds
    let mut candidates: Vec<InitCandidate> = Vec::new();
    let mut bound_names: Vec<String> = Vec::new();

    for bound in &bounds {
        if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {
            let proto_name = proto.metadata().name().value.clone();
            bound_names.push(proto_name.clone());

            // Collect initializers from this protocol
            collect_protocol_initializers(proto, &type_param_ty, &mut candidates);
        }
    }

    if candidates.is_empty() {
        // No init methods found in any bound
        let error = NoInitInTypeParameterBoundsError {
            span: span.clone(),
            type_param_name,
            bound_names,
        };
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
                    .map(|p| format_type(&p.ty))
                    .collect();
                OverloadDescription {
                    name: type_param_name.clone(),
                    labels,
                    param_types,
                    definition_span: Some(c.init.metadata().span().clone()),
                    definition_file_id: Some(ctx.file_id),
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
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
        ctx.diagnostics
            .add_diagnostic(error.into_diagnostic(ctx.file_id));
        return Expression::error(span);
    }

    // Single matching init found
    let winner = unique_matching[0];
    let return_ty = type_param_ty; // Return type is T, not Self

    // Create a call expression referencing the protocol's init
    let init_id = winner.init.metadata().id();
    let init_ref = Expression::symbol_ref(init_id, Ty::inferred(span.clone()), false, span.clone());

    Expression::call(init_ref, arguments, return_ty, span)
}

/// Candidate for init resolution on type parameter
struct InitCandidate {
    /// The init symbol
    init: Arc<dyn Symbol<KestrelLanguage>>,
    /// The callable behavior (for signature matching)
    callable: kestrel_semantic_tree::behavior::callable::CallableBehavior,
    /// Protocol name (for ambiguity detection)
    protocol_name: String,
}

/// Collect initializer methods from a protocol, including inherited protocols.
fn collect_protocol_initializers(
    protocol: &Arc<ProtocolSymbol>,
    self_replacement: &Ty,
    candidates: &mut Vec<InitCandidate>,
) {
    let protocol_name = protocol.metadata().name().value.clone();

    // Get all initializers from this protocol
    for child in protocol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Initializer {
            if let Some(callable) = get_callable_behavior(&child) {
                // Substitute Self with the type parameter in the callable
                let substituted = substitute_callable_self(&callable, self_replacement);

                candidates.push(InitCandidate {
                    init: child.clone(),
                    callable: substituted,
                    protocol_name: protocol_name.clone(),
                });
            }
        }
    }

    // Search inherited protocols
    if let Some(conformances) = protocol.conformances_behavior() {
        for parent_proto_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol: parent, .. } = parent_proto_ty.kind() {
                collect_protocol_initializers(parent, self_replacement, candidates);
            }
        }
    }
}
