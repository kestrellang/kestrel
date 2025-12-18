//! Path expression resolution.
//!
//! This module handles resolving path expressions (variable references, function
//! references, qualified names) including local variable lookup and module path resolution.

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::{ResolveTypePath, ResolveValuePath, SymbolFor, TypePathResolution, ValuePathResolution};
use kestrel_semantic_tree::expr::Expression;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::diagnostics::{
    NotGenericError, SelfOutsideInstanceMethodError, TooFewTypeArgumentsError,
    TooManyTypeArgumentsError, TypeArgsOnNonGenericError, UndefinedNameError,
};
use crate::resolution::type_resolver::TypeResolver;
use kestrel_syntax_tree::utils::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::members::resolve_member_chain;
use super::utils::{get_callable_behavior, is_expression_kind, substitute_type};

/// Resolve a path expression (variable reference, function reference, or member access)
pub fn resolve_path_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Check for nested expression inside the path (happens with member access on call expressions)
    // e.g., `obj.method().field` is parsed as ExprPath containing ExprCall
    if let Some(nested_expr) = find_nested_expression(node) {
        let base = resolve_expression(&nested_expr, ctx);
        let trailing_members = extract_trailing_identifiers(node, ctx.source);
        if trailing_members.is_empty() {
            return base;
        }
        return resolve_member_chain(base, &trailing_members, ctx);
    }

    // Extract the path segments with their spans
    let path_with_spans = extract_path_segments_with_spans(node, ctx.source);

    if path_with_spans.is_empty() {
        return Expression::error(span);
    }

    // Extract just the names for lookups
    let path: Vec<String> = path_with_spans
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let first_name = &path[0];
    let first_span = path_with_spans[0].1.clone();

    // First, check if it's a local variable
    if let Some(local_id) = ctx.local_scope.lookup(first_name) {
        // Check for type arguments on the variable itself (first segment only) - not allowed
        // Only check if this is a single-segment path (just `x[T]`), not `x.member[T]`
        if path_with_spans.len() == 1 && has_type_arguments_on_first_segment(node) {
            ctx.diagnostics.add_diagnostic(
                TypeArgsOnNonGenericError {
                    span: span.clone(),
                    callee_description: "a variable".to_string(),
                }
                .into_diagnostic(),
            );
            return Expression::error(span);
        }

        // Get the type and mutability from the local
        let local = ctx.local_scope.function().get_local(local_id);
        let local_ty = local
            .as_ref()
            .map(|l| l.ty().clone())
            .unwrap_or_else(|| Ty::error(span.clone()));
        let is_mutable = local.as_ref().map(|l| l.is_mutable()).unwrap_or(false);

        let base_expr = Expression::local_ref(local_id, local_ty, is_mutable, first_span);

        // If there are more segments, they are member accesses
        if path_with_spans.len() == 1 {
            return base_expr;
        } else {
            return resolve_member_chain(base_expr, &path_with_spans[1..], ctx);
        }
    }

    // Check if this is 'self' being used outside an instance method
    if first_name == "self" {
        // 'self' was not found in local scope, which means we're not in an instance method
        let context = get_function_context(ctx);
        let error = SelfOutsideInstanceMethodError {
            span: first_span.clone(),
            context,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Extract type arguments from the path if present
    let explicit_type_args = extract_type_arguments_from_path(node, ctx);

    // Not a local - resolve as a value path (module path)
    let resolution = ctx.model.query(ResolveValuePath {
        path: path.clone(),
        context: ctx.function_id,
    });
    // Debug: print what we're resolving
    // eprintln!("ResolveValuePath({:?}) = {:?}", path, resolution);
    match resolution {
        ValuePathResolution::Symbol { symbol_id, ty } => {
            // Check if this is a static method accessed via a qualified type path
            // e.g., Box[Int].wrap where wrap is a static method
            if let Some(qualified_ty) = extract_qualified_type_from_path(node, ctx) {
                if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) {
                    if let Some(callable) = get_callable_behavior(&symbol) {
                        if callable.is_static() {
                            // Get struct symbol from qualified type
                            if let Some((struct_sym, _)) = qualified_ty.as_struct_with_subs() {
                                // Create TypeRef receiver with qualified type
                                let type_ref = Expression::type_ref(
                                    struct_sym.metadata().id(),
                                    qualified_ty,
                                    span.clone(),
                                );
                                // Return MethodRef for resolve_method_call to handle
                                let method_name = symbol.metadata().name().value.clone();
                                return Expression::method_ref(
                                    type_ref,
                                    vec![symbol_id],
                                    method_name,
                                    span,
                                );
                            }
                        }
                    }
                }
            }

            // Original handling for non-static-method cases
            // Check if type arguments were provided
            let final_ty = if let Some(ref type_args) = explicit_type_args {
                if !type_args.is_empty() {
                    // Apply type arguments to the function type
                    apply_type_args_to_function(symbol_id, &ty, type_args, &span, ctx).unwrap_or(ty)
                } else {
                    ty
                }
            } else {
                ty
            };

            // For now, module-level symbols (functions) are not mutable lvalues
            Expression::symbol_ref(symbol_id, final_ty, false, span)
        }
        ValuePathResolution::Overloaded { candidates } => {
            Expression::overloaded_ref(candidates, span)
        }
        ValuePathResolution::NotFound { segment, index } => {
            // Report undefined name error
            let error_span = if index < path_with_spans.len() {
                path_with_spans[index].1.clone()
            } else {
                first_span.clone()
            };
            let error = UndefinedNameError {
                span: error_span,
                name: segment,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            Expression::error(span)
        }
        ValuePathResolution::Ambiguous {
            segment,
            index,
            candidates,
        } => {
            // TODO: Report ambiguous name diagnostic
            let _ = (segment, index, candidates);
            Expression::error(span)
        }
        ValuePathResolution::NotAValue { symbol_id } => {
            // Check if this is a field that needs implicit self access
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) {
                if symbol.metadata().kind() == KestrelSymbolKind::Field {
                    // This is a field reference like `x` that should become `self.x`
                    // Look for 'self' in local scope
                    if let Some(self_local_id) = ctx.local_scope.lookup("self") {
                        let self_local = ctx.local_scope.function().get_local(self_local_id);
                        let self_ty = self_local
                            .as_ref()
                            .map(|l| l.ty().clone())
                            .unwrap_or_else(|| Ty::error(span.clone()));
                        let self_mutable = self_local.as_ref().map(|l| l.is_mutable()).unwrap_or(false);

                        // Create self reference
                        let self_expr = Expression::local_ref(self_local_id, self_ty, self_mutable, span.clone());

                        // Get field type and mutability from FieldSymbol
                        use kestrel_semantic_tree::symbol::field::FieldSymbol;
                        let (field_ty, field_mutable) = symbol
                            .downcast_ref::<FieldSymbol>()
                            .map(|f| (f.field_type().clone(), f.is_mutable()))
                            .unwrap_or_else(|| (Ty::error(span.clone()), false));

                        let field_name = symbol.metadata().name().value.clone();
                        return Expression::field_access(self_expr, field_name, field_mutable, field_ty, span);
                    }
                    // If no self, fall through to create type_ref
                }
            }
            // This is a type reference (e.g., struct name) - may be used for initialization
            // For generic types, create struct type with inference variables for each type param
            // This enables proper type inference when the struct is used without explicit type args
            use super::utils::create_struct_type_with_type_args;
            let ty = ctx.model.query(SymbolFor { id: symbol_id })
                .and_then(|symbol| {
                    symbol.clone().downcast_arc::<StructSymbol>().ok().map(|struct_sym| {
                        create_struct_type_with_type_args(
                            &(struct_sym as std::sync::Arc<dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>>),
                            &[],
                            span.clone(),
                            ctx
                        )
                    })
                })
                .unwrap_or_else(|| Ty::infer(span.clone()));
            Expression::type_ref(symbol_id, ty, span)
        }
        ValuePathResolution::TypeParameter { symbol_id } => {
            // This is a type parameter reference (e.g., T in `T()` or `T.create()`)
            // For multi-segment paths like T.create, the db returns TypeParameter
            // for just the first segment, and we need to handle the rest as member accesses

            // Look up the type parameter symbol to create proper type
            let type_param_ty = if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) {
                if let Ok(type_param_arc) = symbol.clone().downcast_arc::<TypeParameterSymbol>() {
                    Ty::type_parameter(type_param_arc, first_span.clone())
                } else {
                    Ty::infer(first_span.clone())
                }
            } else {
                Ty::infer(first_span.clone())
            };

            let base = Expression::type_parameter_ref(symbol_id, type_param_ty, first_span.clone());

            // If there are more segments, resolve them as member accesses
            if path_with_spans.len() > 1 {
                resolve_member_chain(base, &path_with_spans[1..], ctx)
            } else {
                base
            }
        }
    }
}

/// Get a description of the function context for error messages.
///
/// Returns descriptions like "static method", "free function", etc.
fn get_function_context(ctx: &BodyResolutionContext) -> String {
    let Some(function) = ctx.model.query(SymbolFor {
        id: ctx.function_id,
    }) else {
        return "this context".to_string();
    };

    // Check if the function is in a struct or protocol
    let parent = function.metadata().parent();
    match parent.as_ref().map(|p| p.metadata().kind()) {
        Some(KestrelSymbolKind::Struct) | Some(KestrelSymbolKind::Protocol) => {
            // It's a method - check if static
            // We can check by looking for 'self' in local scope, but we already know
            // 'self' wasn't found, so this must be a static method
            "static method".to_string()
        }
        _ => {
            // Not in a struct/protocol, so it's a free function
            "free function".to_string()
        }
    }
}

/// Extract path segments with their spans from a path expression node
fn extract_path_segments_with_spans(node: &SyntaxNode, source: &str) -> Vec<(String, Span)> {
    let mut segments = Vec::new();

    // ExprPath may contain Path or direct PathElements
    if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
        // Path contains PathElements
        for element in path_node.children() {
            if element.kind() == SyntaxKind::PathElement {
                if let Some((name, span)) = extract_path_element_name_with_span(&element, source) {
                    segments.push((name, span));
                }
            }
        }
    } else {
        // Direct identifiers
        for child in node.children() {
            if child.kind() == SyntaxKind::PathElement {
                if let Some((name, span)) = extract_path_element_name_with_span(&child, source) {
                    segments.push((name, span));
                }
            }
        }

        // Fallback: look for Name or Identifier tokens
        if segments.is_empty() {
            for elem in node.children_with_tokens() {
                if let Some(token) = elem.as_token() {
                    if token.kind() == SyntaxKind::Identifier {
                        let span = token.text_range();
                        segments.push((
                            token.text().to_string(),
                            Span::from(span.start().into()..span.end().into()),
                        ));
                    }
                }
            }
        }
    }

    segments
}

/// Extract the name and span from a PathElement node
fn extract_path_element_name_with_span(
    element: &SyntaxNode,
    _source: &str,
) -> Option<(String, Span)> {
    // PathElement contains Name or Identifier
    if let Some(name_node) = element.children().find(|c| c.kind() == SyntaxKind::Name) {
        return name_node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| {
                let range = t.text_range();
                (
                    t.text().to_string(),
                    Span::from(range.start().into()..range.end().into()),
                )
            });
    }

    // Direct Identifier token
    element
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| {
            let range = t.text_range();
            (
                t.text().to_string(),
                Span::from(range.start().into()..range.end().into()),
            )
        })
}

/// Find a nested expression inside a path node
///
/// This handles the case where member access on a call expression is emitted as
/// an ExprPath containing an Expression/ExprCall child.
/// e.g., `obj.method().field` is parsed as ExprPath containing ExprCall
fn find_nested_expression(node: &SyntaxNode) -> Option<SyntaxNode> {
    // We're looking inside an ExprPath node. Normally it contains only identifiers and dots.
    // But when member access is on a call expression, the parser emits the call inside the ExprPath.
    // We need to find such nested call expressions.

    for child in node.children() {
        // Look for Expression wrapper containing a non-path expression
        if child.kind() == SyntaxKind::Expression {
            // Check if this Expression contains an ExprCall or other complex (non-path) expression
            for inner in child.children() {
                // Only return if it's a complex expression type, not just another path
                if inner.kind() == SyntaxKind::ExprCall {
                    return Some(child);
                }
            }
        }
        // Also check for direct ExprCall nodes
        if child.kind() == SyntaxKind::ExprCall {
            return Some(child);
        }
    }
    None
}

/// Extract trailing identifier tokens after a nested expression in a path
///
/// When a path contains a nested expression (e.g., from member access on a call),
/// this extracts the identifiers that appear after the expression.
fn extract_trailing_identifiers(node: &SyntaxNode, _source: &str) -> Vec<(String, Span)> {
    let mut identifiers = Vec::new();
    let mut found_expression = false;

    for elem in node.children_with_tokens() {
        if let Some(child) = elem.as_node() {
            // Mark when we see the nested expression
            if child.kind() == SyntaxKind::Expression || is_expression_kind(child.kind()) {
                found_expression = true;
            }
        } else if let Some(token) = elem.as_token() {
            // Only collect identifiers after the expression
            if found_expression && token.kind() == SyntaxKind::Identifier {
                let range = token.text_range();
                identifiers.push((
                    token.text().to_string(),
                    Span::from(range.start().into()..range.end().into()),
                ));
            }
        }
    }

    identifiers
}

/// Check if a path expression contains type arguments on the first segment
/// This is for detecting `x[T]` where x is a variable
fn has_type_arguments_on_first_segment(node: &SyntaxNode) -> bool {
    // For a path like `x[T]`, we look for TypeArgumentList directly in the first PathElement
    // or directly in the ExprPath if there's no Path wrapper

    // First check if there's a Path child
    if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
        // Get the first PathElement
        if let Some(first_elem) = path_node
            .children()
            .find(|c| c.kind() == SyntaxKind::PathElement)
        {
            return first_elem
                .children()
                .any(|c| c.kind() == SyntaxKind::TypeArgumentList);
        }
    }

    // Also check directly in ExprPath for simpler paths
    for child in node.children() {
        if child.kind() == SyntaxKind::PathElement {
            if child
                .children()
                .any(|c| c.kind() == SyntaxKind::TypeArgumentList)
            {
                return true;
            }
            // Only check the first PathElement
            return false;
        }
        if child.kind() == SyntaxKind::TypeArgumentList {
            return true;
        }
    }

    false
}

/// Extract type arguments from a path expression node.
///
/// Handles paths like `identity[String]` or `module.func[Int, Bool]`.
/// Returns None if no type arguments are present, or Some(vec) with the resolved types.
///
/// IMPORTANT: Only extracts type arguments from the FINAL path segment.
/// For `Box[Int].zero`, the type args `[Int]` belong to `Box`, not to `zero`.
/// Type args on intermediate segments are handled during type resolution of those segments.
///
/// ExprPath structure for "Box[Int].zero":
/// ExprPath
///   Identifier "Box"
///   TypeArgumentList [Int]
///   Dot
///   Identifier "zero"
///
/// We want to only extract type args that come AFTER the last dot (i.e., on the final segment).
fn extract_type_arguments_from_path(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Vec<Ty>> {
    // Look for TypeArgumentList only in the FINAL path segment
    fn find_type_args_on_final_segment(node: &SyntaxNode) -> Option<SyntaxNode> {
        // Find the ExprPath node (either this node or a child)
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

        // Also check direct PathElements for simpler paths
        let path_elements: Vec<_> = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::PathElement)
            .collect();

        // For multi-segment paths, only extract type args from the LAST element
        if path_elements.len() > 1 {
            if let Some(last_element) = path_elements.last() {
                for inner in last_element.children() {
                    if inner.kind() == SyntaxKind::TypeArgumentList {
                        return Some(inner);
                    }
                }
            }
            return None;
        }

        if let Some(only_element) = path_elements.first() {
            for inner in only_element.children() {
                if inner.kind() == SyntaxKind::TypeArgumentList {
                    return Some(inner);
                }
            }
        }

        // Only check direct TypeArgumentList if no path structure found
        if path_elements.is_empty() {
            for child in node.children() {
                if child.kind() == SyntaxKind::TypeArgumentList {
                    return Some(child);
                }
            }
        }

        None
    }

    let type_arg_list = find_type_args_on_final_segment(node)?;

    // Resolve each type in the TypeArgumentList
    let mut type_args = Vec::new();

    for child in type_arg_list.children() {
        if child.kind() == SyntaxKind::Ty {
            let mut resolver =
                TypeResolver::new(ctx.model, ctx.diagnostics, ctx.source, ctx.file_id, ctx.function_id);
            let ty = resolver.resolve(&child);
            type_args.push(ty);
        }
    }

    // Return Some even if empty - the presence of [] means explicit type args were provided
    Some(type_args)
}

/// Apply type arguments to a function type, returning the instantiated type.
///
/// This validates that:
/// - The symbol is a generic function
/// - The number of type arguments matches the number of type parameters
///
/// Returns None if type arguments can't be applied (with diagnostics emitted).
fn apply_type_args_to_function(
    symbol_id: semantic_tree::symbol::SymbolId,
    _original_ty: &Ty,
    type_args: &[Ty],
    span: &Span,
    ctx: &mut BodyResolutionContext,
) -> Option<Ty> {
    // Get the symbol
    let symbol = ctx.model.query(SymbolFor { id: symbol_id })?;

    // Check if it's a function with type parameters
    let func_sym = symbol.as_any().downcast_ref::<FunctionSymbol>()?;
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
        return None;
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
        return None;
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
        return None;
    }

    // Build substitutions from type parameters to provided type arguments
    let mut substitutions = Substitutions::new();
    for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
        substitutions.insert(param.metadata().id(), arg_ty.clone());
    }

    // Get the callable behavior to get the function type
    let callable = get_callable_behavior(&symbol)?;

    // Build the instantiated function type
    let params: Vec<Ty> = callable
        .parameters()
        .iter()
        .map(|p| substitute_type(&p.ty, &substitutions))
        .collect();
    let return_type = substitute_type(callable.return_type(), &substitutions);

    Some(Ty::function(params, return_type, span.clone()))
}

/// Extract the qualified type from an intermediate path segment.
///
/// For `Box[Int].wrap`, this extracts the `Box[Int]` type from the first segment.
/// For `Box.wrap`, this returns `Box` with infer type parameters.
/// For single-segment paths like `wrap`, returns None.
///
/// This is used to capture type arguments on intermediate path segments
/// (before the final segment) so they can be used for type parameter substitution
/// when calling static methods.
fn extract_qualified_type_from_path(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Ty> {
    // Find the ExprPath node
    let expr_path = if node.kind() == SyntaxKind::ExprPath {
        node.clone()
    } else {
        node.children().find(|c| c.kind() == SyntaxKind::ExprPath)?
    };

    let children: Vec<_> = expr_path.children_with_tokens().collect();

    // Find if there's a dot (multi-segment path)
    let first_dot_pos = children.iter().position(|c| {
        c.as_token()
            .map(|t| t.kind() == SyntaxKind::Dot)
            .unwrap_or(false)
    });

    // If no dot, this is a single-segment path - return None
    let first_dot_pos = first_dot_pos?;

    // Get the first identifier (type name)
    let first_ident = children
        .iter()
        .filter_map(|c| c.as_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;
    let type_name = first_ident.text().to_string();

    // Look for TypeArgumentList before the first dot
    let type_arg_list = children[..first_dot_pos]
        .iter()
        .filter_map(|c| c.as_node())
        .find(|n| n.kind() == SyntaxKind::TypeArgumentList);

    // Resolve the type name to get the base type
    let span = get_node_span(node, ctx.file_id);
    let base_ty = match ctx.model.query(ResolveTypePath {
        path: vec![type_name.clone()],
        context: ctx.function_id,
    }) {
        TypePathResolution::Resolved(ty) => ty,
        _ => return None, // Type not found - let normal error handling deal with it
    };

    // Apply type arguments if present
    if let Some(type_arg_list) = type_arg_list {
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
                type_args.push(resolver.resolve(&child));
            }
        }

        if !type_args.is_empty() {
            let mut resolver = TypeResolver::new(
                ctx.model,
                ctx.diagnostics,
                ctx.source,
                ctx.file_id,
                ctx.function_id,
            );
            return Some(resolver.apply_type_arguments(&base_ty, type_args, span));
        }
    }

    // No explicit type arguments - only return Some for generic types that need inference
    // For non-generic types like Point, return None to use the original SymbolRef path
    if let TyKind::Struct { symbol, .. } = base_ty.kind() {
        if !symbol.type_parameters().is_empty() {
            // Generic type without explicit args - return base type for inference
            return Some(base_ty);
        }
    }

    // Non-generic type without type args - return None to use original path
    None
}
