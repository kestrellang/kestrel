//! Extension declaration resolver
//!
//! Extensions add methods and protocol conformances to existing types.
//! Unlike structs, extensions don't have a name - they're identified by their target type.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::database::TypePathResolution;
use crate::diagnostics::{NotAProtocolContext, NotAProtocolError, UnresolvedTypeError};
use crate::resolution::TypeResolver;
use crate::resolver::{BindingContext, Resolver};
use crate::syntax::{extract_path_segments, find_child, get_node_span, resolve_conformance_list};

/// Resolver for extension declarations
pub struct ExtensionResolver;

impl Resolver for ExtensionResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        _root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Get full span
        let full_span = get_node_span(syntax, source);

        // Create the extension symbol (target resolution happens during BIND)
        let extension_symbol = ExtensionSymbol::new(full_span.clone(), parent.cloned());
        let extension_arc = Arc::new(extension_symbol);
        let extension_arc_dyn = extension_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Add to parent if exists
        if let Some(parent) = parent {
            parent.metadata().add_child(&extension_arc_dyn);
        }

        Some(extension_arc)
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process extension symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Extension {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let (file_id, source) = context.get_file_context(symbol);

        // Resolve the target type from the Ty node
        let target_result =
            resolve_extension_target(syntax, &source, symbol_id, context, file_id);

        if let Some((target_ty, target_struct, type_arguments, referenced_params)) = target_result {
            // Get the target struct's where clause constraints (inherited)
            let inherited_where_clause = target_struct.where_clause();

            // Resolve extension's own where clause
            let extension_where_clause = resolve_extension_where_clause(
                syntax,
                &source,
                symbol_id,
                context,
                &referenced_params,
                file_id,
            );

            // Combine inherited and extension constraints
            let combined_where_clause = combine_where_clauses(inherited_where_clause, extension_where_clause);

            // Create and add ExtensionTargetBehavior
            let target_behavior = ExtensionTargetBehavior::new(
                target_ty.clone(),
                type_arguments,
                referenced_params,
                combined_where_clause,
            );
            symbol.metadata().add_behavior(target_behavior);

            // Register extension in the ExtensionRegistry
            let target_id = target_struct.metadata().id();
            if let Ok(extension_symbol) = symbol.clone().downcast_arc::<ExtensionSymbol>() {
                context.db.register_extension(target_id, extension_symbol);
            }
        }

        // Resolve conformances from syntax
        resolve_conformance_list(
            syntax,
            &source,
            symbol,
            symbol_id,
            context,
            file_id,
            NotAProtocolContext::Conformance,
        );
    }
}

/// Resolve the extension's target type from syntax.
///
/// Returns (target_type, target_struct_symbol, type_arguments, referenced_type_params)
///
/// For generic extensions like `extend Pair[T, U]`, the type arguments T and U are
/// resolved as references to the struct's type parameters, not looked up in scope.
fn resolve_extension_target(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Option<(Ty, Arc<StructSymbol>, Vec<Ty>, Vec<Arc<TypeParameterSymbol>>)> {
    // Find the Ty node (target type expression)
    let ty_node = find_child(syntax, SyntaxKind::Ty)?;
    let ty_span = get_node_span(&ty_node, source);

    // Find TyPath within Ty
    let ty_path_node = ty_node
        .children()
        .find(|c| c.kind() == SyntaxKind::TyPath)?;

    // Find Path within TyPath to get the base type name
    let path_node = ty_path_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Path)?;

    let segments = extract_path_segments(&path_node);
    if segments.is_empty() {
        return None;
    }

    // First, resolve just the base type (without type arguments) to get the struct
    let base_resolution = ctx.db.resolve_type_path(segments.clone(), context_id);
    let struct_symbol = match base_resolution {
        TypePathResolution::Resolved(ty) => match ty.kind() {
            TyKind::Struct { symbol, .. } => symbol.clone(),
            _ => {
                ctx.diagnostics.throw(
                    CannotExtendTypeError {
                        span: ty_span.clone(),
                        type_name: format_type_kind(ty.kind()),
                    });
                return None;
            }
        },
        TypePathResolution::NotFound { segment, .. } => {
            ctx.diagnostics.throw(
                UnresolvedTypeError {
                    span: ty_span.clone(),
                    type_name: segment,
                });
            return None;
        }
        _ => return None,
    };

    // Get the struct's type parameters
    let struct_type_params = struct_symbol.type_parameters();

    // Now resolve the type arguments, using the struct's type params as available references
    let type_args = resolve_extension_type_arguments(
        &ty_path_node,
        source,
        &struct_type_params,
        context_id,
        ctx,
        file_id,
    )?;

    // Collect referenced type parameters from the type arguments
    let referenced_params = collect_referenced_type_params(&type_args);

    // Build the final type with substitutions
    // For generic extensions like `extend Pair[T, U]`, validate that type parameters
    // appear in their declared positions (T must be in position 0, U in position 1)
    let mut substitutions = kestrel_semantic_tree::ty::Substitutions::new();
    for (index, (param, arg)) in struct_type_params.iter().zip(type_args.iter()).enumerate() {
        let param_id = param.metadata().id();

        // Check if arg is a type parameter reference
        if let kestrel_semantic_tree::ty::TyKind::TypeParameter(arg_param) = arg.kind() {
            let arg_param_id = arg_param.metadata().id();

            // Check if it's a different type parameter in this position
            // (not self-referential, e.g., U in T's position)
            if arg_param_id != param_id {
                // Check if it's actually one of the struct's type parameters but in wrong position
                let is_struct_param_wrong_position = struct_type_params.iter().enumerate().any(|(other_index, other_param)| {
                    other_param.metadata().id() == arg_param_id && other_index != index
                });

                if is_struct_param_wrong_position {
                    // Error: type parameter in wrong position (e.g., extend Pair[U, T])
                    let param_name = param.metadata().name().value.clone();
                    let arg_name = arg_param.metadata().name().value.clone();
                    ctx.diagnostics.throw(
                        crate::diagnostics::TypeParameterWrongPositionError {
                            span: ty_span.clone(),
                            message: format!(
                                "type parameter '{}' used in wrong position (expected '{}' in position {})",
                                arg_name, param_name, index
                            ),
                        });
                    return None;
                }
            }
        }

        // Add all substitutions, including self-referential ones (T -> T)
        // The cycle detection in Substitutions::apply will handle self-referential cases gracefully
        substitutions.insert(param_id, arg.clone());
    }
    let resolved_ty = Ty::generic_struct(struct_symbol.clone(), substitutions, ty_span);

    Some((resolved_ty, struct_symbol, type_args, referenced_params))
}

/// Resolve type arguments for an extension target.
///
/// This handles the special case where type arguments like T and U in `extend Pair[T, U]`
/// should resolve to the struct's type parameters, not be looked up in scope.
///
/// Returns None if there's a type parameter count mismatch, indicating an error was emitted.
fn resolve_extension_type_arguments(
    ty_path_node: &SyntaxNode,
    source: &str,
    struct_type_params: &[Arc<TypeParameterSymbol>],
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    file_id: usize,
) -> Option<Vec<Ty>> {
    let arg_list = match ty_path_node
        .children()
        .find(|c| c.kind() == SyntaxKind::TypeArgumentList)
    {
        Some(list) => list,
        None => return Some(Vec::new()),
    };

    // Count the type arguments in the syntax
    let ty_nodes: Vec<_> = arg_list.children().filter(|c| c.kind() == SyntaxKind::Ty).collect();
    let arg_count = ty_nodes.len();
    let expected_count = struct_type_params.len();

    // Check for type parameter count mismatch
    if arg_count != expected_count {
        let arg_list_span = get_node_span(&arg_list, source);
        ctx.diagnostics.throw(
            WrongTypeParameterCountError {
                span: arg_list_span,
                expected: expected_count,
                actual: arg_count,
            });
        return None;
    }

    let mut type_args = Vec::new();

    for ty_node in ty_nodes {
        let ty_span = get_node_span(&ty_node, source);

        // Try to extract a simple identifier from this type argument
        let simple_name = extract_simple_type_name(&ty_node);

        if let Some(name) = simple_name {
            // Check if this name matches any of the struct's type parameters
            if let Some(type_param) = struct_type_params.iter().find(|p| p.metadata().name().value == name) {
                // This is a reference to the struct's type parameter
                type_args.push(Ty::type_parameter(type_param.clone(), ty_span));
                continue;
            }
        }

        // Not a type parameter reference - resolve as a normal type
        let mut type_resolver = TypeResolver::new(ctx.db, ctx.diagnostics, file_id, source, context_id);
        type_args.push(type_resolver.resolve(&ty_node));
    }

    Some(type_args)
}

/// Extract a simple type name from a Ty node, if it's just a simple identifier.
/// Returns None if the type is complex (has type arguments, is a function type, etc.)
fn extract_simple_type_name(ty_node: &SyntaxNode) -> Option<String> {
    // Find TyPath
    let ty_path = ty_node.children().find(|c| c.kind() == SyntaxKind::TyPath)?;

    // Check it doesn't have type arguments (which would make it not a simple name)
    if ty_path.children().any(|c| c.kind() == SyntaxKind::TypeArgumentList) {
        return None;
    }

    // Find Path
    let path = ty_path.children().find(|c| c.kind() == SyntaxKind::Path)?;

    // Extract segments
    let segments = extract_path_segments(&path);

    // Must be exactly one segment for a simple type name
    if segments.len() == 1 {
        Some(segments[0].clone())
    } else {
        None
    }
}

/// Collect type parameter symbols referenced in type arguments
fn collect_referenced_type_params(type_args: &[Ty]) -> Vec<Arc<TypeParameterSymbol>> {
    let mut params = Vec::new();

    for ty in type_args {
        if let TyKind::TypeParameter(symbol) = ty.kind() {
            params.push(symbol.clone());
        }
        // Note: We could recursively search nested types if needed
    }

    params
}

/// Format a TyKind for error messages
fn format_type_kind(kind: &TyKind) -> String {
    match kind {
        TyKind::Int { .. } => "Int".to_string(),
        TyKind::Float { .. } => "Float".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeParameter(symbol) => symbol.metadata().name().value.clone(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::Error => "<error>".to_string(),
        _ => "<unknown>".to_string(),
    }
}

/// Error for trying to extend a non-struct type
#[derive(Debug, Clone)]
pub struct CannotExtendTypeError {
    pub span: kestrel_span::Span,
    pub type_name: String,
}

impl kestrel_reporting::IntoDiagnostic for CannotExtendTypeError {
    fn into_diagnostic(&self) -> kestrel_reporting::Diagnostic<usize> {
        kestrel_reporting::Diagnostic::error()
            .with_message(format!(
                "cannot extend type '{}' - only structs can be extended",
                self.type_name
            ))
            .with_labels(vec![kestrel_reporting::Label::primary(
                self.span.file_id,
                self.span.range(),
            )
            .with_message("not a struct type")])
            .with_notes(vec![
                "Extensions can only be applied to struct types".to_string(),
            ])
    }
}

/// Error for wrong number of type parameters in extension target
#[derive(Debug, Clone)]
pub struct WrongTypeParameterCountError {
    pub span: kestrel_span::Span,
    pub expected: usize,
    pub actual: usize,
}

impl kestrel_reporting::IntoDiagnostic for WrongTypeParameterCountError {
    fn into_diagnostic(&self) -> kestrel_reporting::Diagnostic<usize> {
        let message = if self.actual > self.expected {
            format!(
                "too many type parameters: expected {}, found {}",
                self.expected, self.actual
            )
        } else {
            format!(
                "too few type parameters: expected {}, found {}",
                self.expected, self.actual
            )
        };
        kestrel_reporting::Diagnostic::error()
            .with_message(message)
            .with_labels(vec![kestrel_reporting::Label::primary(
                self.span.file_id,
                self.span.range(),
            )
            .with_message(format!("expected {} type parameter(s)", self.expected))])
    }
}

/// Resolve where clause from extension syntax.
///
/// Extensions can only reference type parameters that appear in their target type.
fn resolve_extension_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    referenced_params: &[Arc<TypeParameterSymbol>],
    file_id: usize,
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let mut constraints = Vec::new();

    for child in where_clause_node.children() {
        if child.kind() == SyntaxKind::TypeBound {
            if let Some(constraint) = resolve_extension_type_bound(
                &child,
                source,
                context_id,
                ctx,
                referenced_params,
                file_id,
            ) {
                constraints.push(constraint);
            }
        }
    }

    WhereClause::with_constraints(constraints)
}

/// Resolve a single TypeBound in an extension's where clause.
fn resolve_extension_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    referenced_params: &[Arc<TypeParameterSymbol>],
    file_id: usize,
) -> Option<Constraint> {
    // Find the Name node and extract the type parameter name and span
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let param_name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let param_span: kestrel_span::Span = Span::from((text_range.start().into())..(text_range.end().into()));

    // Look up the type parameter in the referenced params (not all struct params)
    let param = referenced_params
        .iter()
        .find(|p| p.metadata().name().value == param_name);

    let param_id = param.map(|p| p.metadata().id());

    // Resolve each Path to a protocol type
    let bounds: Vec<Ty> = syntax
        .children()
        .filter(|c| c.kind() == SyntaxKind::Path)
        .map(|path_node| {
            let span = get_node_span(&path_node, source);
            let segments = extract_path_segments(&path_node);

            if segments.is_empty() {
                return Ty::error(span);
            }

            let bound_name = segments.join(".");

            // Resolve the path to a type
            match ctx.db.resolve_type_path(segments, context_id) {
                TypePathResolution::Resolved(resolved_ty) => match resolved_ty.kind() {
                    TyKind::Protocol { .. } => resolved_ty,
                    TyKind::Struct { symbol, .. } => {
                        ctx.diagnostics.throw(
                            NotAProtocolError {
                                span: span.clone(),
                                name: symbol.metadata().name().value.clone(),
                                context: NotAProtocolContext::Bound,
                            });
                        Ty::error(span)
                    }
                    TyKind::TypeAlias { symbol, .. } => {
                        ctx.diagnostics.throw(
                            NotAProtocolError {
                                span: span.clone(),
                                name: symbol.metadata().name().value.clone(),
                                context: NotAProtocolContext::Bound,
                            });
                        Ty::error(span)
                    }
                    _ => {
                        ctx.diagnostics.throw(
                            NotAProtocolError {
                                span: span.clone(),
                                name: bound_name.clone(),
                                context: NotAProtocolContext::Bound,
                            });
                        Ty::error(span)
                    }
                },
                TypePathResolution::NotFound { .. } => {
                    ctx.diagnostics.throw(
                        UnresolvedTypeError {
                            span: span.clone(),
                            type_name: bound_name.clone(),
                        });
                    Ty::error(span)
                }
                TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
                    ctx.diagnostics.throw(
                        NotAProtocolError {
                            span: span.clone(),
                            name: bound_name.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                    Ty::error(span)
                }
            }
        })
        .collect();

    if bounds.is_empty() {
        None
    } else {
        match param_id {
            Some(id) => Some(Constraint::type_bound(id, param_name, param_span, bounds)),
            None => Some(Constraint::unresolved_type_bound(
                param_name, param_span, bounds,
            )),
        }
    }
}

/// Combine inherited where clause from struct with extension's own where clause.
fn combine_where_clauses(inherited: WhereClause, extension: WhereClause) -> WhereClause {
    let mut combined_constraints = inherited.constraints.to_vec();
    combined_constraints.extend(extension.constraints.iter().cloned());
    WhereClause::with_constraints(combined_constraints)
}
