use std::sync::Arc;

use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Spanned;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::diagnostics::{NotAProtocolContext, NotAProtocolError, UnresolvedTypeError};
use crate::database::TypePathResolution;
use crate::resolver::{BindingContext, Resolver};
use crate::resolvers::type_parameter::{add_type_params_as_children, extract_type_parameters};
use crate::syntax::{
    extract_name, extract_path_segments, extract_visibility, find_child, find_visibility_scope,
    get_file_id_for_symbol, get_node_span, get_visibility_span, parse_visibility, resolve_conformance_list,
};

/// Resolver for struct declarations
pub struct StructResolver;

impl Resolver for StructResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Extract name
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, source);

        // Get full span
        let full_span = get_node_span(syntax, source);

        // Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(parse_visibility);

        let visibility_span = get_visibility_span(syntax, source).unwrap_or(name_span.clone());

        // Determine visibility scope
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);

        // Create visibility behavior
        let visibility_behavior =
            VisibilityBehavior::new(visibility_enum, visibility_span, visibility_scope);

        // Create the name object
        let name = Spanned::new(name_str, name_span);

        // Create the struct symbol (GenericsBehavior is added during BIND)
        let struct_symbol = StructSymbol::new(
            name,
            full_span.clone(),
            visibility_behavior,
            parent.cloned(),
        );
        let struct_arc = Arc::new(struct_symbol);

        let struct_type = Ty::r#struct(struct_arc.clone(), full_span.clone());
        let typed_behavior = TypedBehavior::new(struct_type, full_span.clone());

        struct_arc.metadata().add_behavior(typed_behavior);

        let struct_arc_dyn = struct_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Extract type parameters with correct parent (the struct, not the module)
        let type_parameters = extract_type_parameters(syntax, source, Some(struct_arc_dyn.clone()));

        // Add type parameters as children of the struct
        // This ensures type parameters are in scope during type resolution
        add_type_params_as_children(&type_parameters, &struct_arc_dyn);

        // Add to parent if exists
        if let Some(parent) = parent {
            parent.metadata().add_child(&struct_arc_dyn);
        }

        Some(struct_arc)
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process struct symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return;
        }

        let symbol_id = symbol.metadata().id();

        // Get file_id and source for this symbol
        let (file_id, source) = context.get_file_context(symbol);

        // Extract type parameters and resolve where clause bounds
        let generics_behavior = resolve_generics(syntax, &source, symbol_id, context);

        // Add GenericsBehavior
        symbol.metadata().add_behavior(generics_behavior);

        // Resolve conformances from syntax and store them
        resolve_conformance_list(
            syntax,
            &source,
            symbol,
            symbol_id,
            context,
            file_id,
            NotAProtocolContext::Conformance,
        );

        // Note: Protocol method linking happens in the ConformanceValidator
        // during the VALIDATE phase, after all children are bound
    }
}

/// Extract type parameters and resolve where clause bounds, creating a GenericsBehavior.
fn resolve_generics(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> GenericsBehavior {
    // Get type parameters from the symbol's children (they were added during BUILD)
    let symbol = match ctx.db.symbol_by_id(context_id) {
        Some(s) => s,
        None => return GenericsBehavior::empty(),
    };

    let type_parameters: Vec<Arc<TypeParameterSymbol>> = symbol
        .metadata()
        .children()
        .into_iter()
        .filter_map(|child| {
            if child.metadata().kind() == KestrelSymbolKind::TypeParameter {
                child.downcast_arc::<TypeParameterSymbol>().ok()
            } else {
                None
            }
        })
        .collect();

    // Now resolve the where clause with fully resolved protocol types
    let where_clause = resolve_where_clause(syntax, source, context_id, ctx, &type_parameters);

    GenericsBehavior::new(type_parameters, where_clause)
}

/// Resolve where clause from syntax, returning a WhereClause with resolved protocol types.
fn resolve_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let file_id = ctx.db.symbol_by_id(context_id)
        .map(|s| get_file_id_for_symbol(&s, ctx.diagnostics))
        .unwrap_or(0);

    let mut constraints = Vec::new();

    for child in where_clause_node.children() {
        if child.kind() == SyntaxKind::TypeBound {
            if let Some(constraint) = resolve_type_bound(&child, source, context_id, ctx, type_params, file_id) {
                constraints.push(constraint);
            }
        }
    }

    WhereClause::with_constraints(constraints)
}

/// Resolve a single TypeBound, resolving protocol paths to actual types.
fn resolve_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
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
    let param_span: kestrel_span::Span = (text_range.start().into())..(text_range.end().into());

    // Look up the type parameter (may be None if undeclared)
    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == param_name)
        .map(|p| p.metadata().id());

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
                TypePathResolution::Resolved(resolved_ty) => {
                    match resolved_ty.kind() {
                        TyKind::Protocol { .. } => resolved_ty,
                        TyKind::Struct { symbol, .. } => {
                            ctx.diagnostics.throw(NotAProtocolError {
                                span: span.clone(),
                                name: symbol.metadata().name().value.clone(),
                                context: NotAProtocolContext::Bound,
                            }, file_id);
                            Ty::error(span)
                        }
                        TyKind::TypeAlias { symbol, .. } => {
                            ctx.diagnostics.throw(NotAProtocolError {
                                span: span.clone(),
                                name: symbol.metadata().name().value.clone(),
                                context: NotAProtocolContext::Bound,
                            }, file_id);
                            Ty::error(span)
                        }
                        _ => {
                            ctx.diagnostics.throw(NotAProtocolError {
                                span: span.clone(),
                                name: bound_name.clone(),
                                context: NotAProtocolContext::Bound,
                            }, file_id);
                            Ty::error(span)
                        }
                    }
                }
                TypePathResolution::NotFound { .. } => {
                    ctx.diagnostics.throw(UnresolvedTypeError {
                        span: span.clone(),
                        type_name: bound_name.clone(),
                    }, file_id);
                    Ty::error(span)
                }
                TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
                    ctx.diagnostics.throw(NotAProtocolError {
                        span: span.clone(),
                        name: bound_name.clone(),
                        context: NotAProtocolContext::Bound,
                    }, file_id);
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
            None => Some(Constraint::unresolved_type_bound(param_name, param_span, bounds)),
        }
    }
}
