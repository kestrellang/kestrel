use std::sync::Arc;

use kestrel_semantic_model::{ResolveTypePath, TypePathResolution};
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::binders::flatten_protocol;
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::diagnostics::{NotAProtocolContext, NotAProtocolError, UnresolvedTypeError};
use crate::syntax::helpers::resolve_conformance_list;
use kestrel_syntax_tree::utils::{extract_path_segments, find_child, get_node_span};

/// Binder for protocol declarations
pub struct ProtocolBinder;

impl DeclarationBinder for ProtocolBinder {
    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process protocol symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return;
        }

        let symbol_id = symbol.metadata().id();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve inherited protocols FIRST, before where clause
        // This is needed so that where clause can reference associated types from inherited protocols
        // e.g., protocol SortedIterator: Iterator where Iterator.Item: Comparable { }
        resolve_conformance_list(
            syntax,
            &source,
            file_id,
            symbol,
            symbol_id,
            context,
            NotAProtocolContext::Inheritance,
        );

        // Extract type parameters and resolve where clause bounds
        // Now inherited protocols are available for associated type path resolution
        let generics_behavior =
            resolve_generics(syntax, &source, file_id, symbol_id, context, symbol);

        // Add GenericsBehavior
        symbol.metadata().add_behavior(generics_behavior);

        // Flatten protocol inheritance hierarchy
        if let Ok(protocol_symbol) = symbol.clone().downcast_arc::<ProtocolSymbol>() {
            if let Some(flattened) = flatten_protocol(&protocol_symbol, context) {
                symbol.metadata().add_behavior(flattened);
            }
        }
    }
}

/// Extract type parameters and resolve where clause bounds, creating a GenericsBehavior.
fn resolve_generics(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> GenericsBehavior {
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
    // Inherited protocols are already resolved (ConformancesBehavior is attached)
    let where_clause =
        resolve_where_clause(syntax, source, file_id, context_id, ctx, &type_parameters, symbol);

    GenericsBehavior::new(type_parameters, where_clause)
}

/// Resolve where clause from syntax, returning a WhereClause with resolved protocol types.
fn resolve_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let mut constraints = Vec::new();

    for child in where_clause_node.children() {
        if child.kind() == SyntaxKind::TypeBound {
            if let Some(constraint) =
                resolve_type_bound(&child, source, file_id, context_id, ctx, type_params, symbol)
            {
                constraints.push(constraint);
            }
        }
    }

    WhereClause::with_constraints(constraints)
}

/// Resolve a single TypeBound, resolving protocol paths to actual types.
///
/// Handles both:
/// - Simple type parameters: `T: Protocol`
/// - Inherited protocol associated types: `Iterator.Item: Comparable`
fn resolve_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<Constraint> {
    // Check if this is an AssociatedTypeTarget (for paths like Iterator.Item)
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        // Associated type path: Iterator.Item
        // For protocols, this could be an inherited protocol's associated type
        let path_node = find_child(&target_node, SyntaxKind::Path)?;
        let segments = extract_path_segments(&path_node);

        if segments.len() >= 2 {
            let protocol_name = &segments[0];
            let assoc_type_name = &segments[1];

            // Check if the first segment refers to an inherited protocol
            if let Some(inherited_protocol) = find_inherited_protocol(symbol, protocol_name) {
                // Validate that the associated type exists in the inherited protocol
                let has_assoc_type = inherited_protocol
                    .metadata()
                    .children()
                    .iter()
                    .any(|child| {
                        child.metadata().kind() == KestrelSymbolKind::AssociatedType
                            && &child.metadata().name().value == assoc_type_name
                    });

                if has_assoc_type {
                    // Resolve the bounds and create an InheritedAssociatedTypeBound constraint
                    let bounds = resolve_bounds(syntax, source, file_id, context_id, ctx);
                    let span = get_node_span(&target_node, file_id);
                    let full_name = segments.join(".");

                    // Create a constraint that represents the inherited associated type bound
                    // This is valid and should NOT be flagged as undeclared
                    return Some(Constraint::inherited_assoc_type_bound(
                        full_name, span, bounds,
                    ));
                }
                // If associated type doesn't exist, fall through to produce an error
            }
        }

        // If we get here, it's an unresolved associated type path
        let full_name = segments.join(".");
        let span = get_node_span(&target_node, file_id);
        let bounds = resolve_bounds(syntax, source, file_id, context_id, ctx);

        if !bounds.is_empty() {
            return Some(Constraint::unresolved_type_bound(full_name, span, bounds));
        }
        return None;
    }

    // Simple type parameter: T
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let span: kestrel_span::Span =
        Span::new(file_id, (text_range.start().into())..(text_range.end().into()));

    // Look up the type parameter (may be None if undeclared)
    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == name)
        .map(|p| p.metadata().id());

    // Resolve the bounds
    let bounds = resolve_bounds(syntax, source, file_id, context_id, ctx);

    if bounds.is_empty() {
        None
    } else {
        match param_id {
            Some(id) => Some(Constraint::type_bound(id, name, span, bounds)),
            None => Some(Constraint::unresolved_type_bound(name, span, bounds)),
        }
    }
}

/// Find an inherited protocol by name from the symbol's ConformancesBehavior
fn find_inherited_protocol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    protocol_name: &str,
) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
    // Get the ConformancesBehavior which contains inherited protocols
    let behaviors = symbol.metadata().behaviors();
    let conformances_behavior = behaviors
        .iter()
        .find(|b| matches!(b.kind(), KestrelBehaviorKind::Conformances))?;

    let conformances = conformances_behavior
        .as_ref()
        .downcast_ref::<ConformancesBehavior>()?;

    // Find the protocol with matching name
    for ty in conformances.conformances() {
        if let TyKind::Protocol {
            symbol: proto_sym, ..
        } = ty.kind()
        {
            if proto_sym.metadata().name().value == protocol_name {
                return Some(proto_sym.clone() as Arc<dyn Symbol<KestrelLanguage>>);
            }
        }
    }
    None
}

/// Resolve bounds from Path children in a TypeBound node
fn resolve_bounds(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Vec<Ty> {
    syntax
        .children()
        .filter(|c| c.kind() == SyntaxKind::Path)
        .map(|path_node| {
            let span = get_node_span(&path_node, file_id);
            let segments = extract_path_segments(&path_node);

            if segments.is_empty() {
                return Ty::error(span);
            }

            let bound_name = segments.join(".");

            // Resolve the path to a type
            match ctx.model.query(ResolveTypePath {
                path: segments,
                context: context_id,
            }) {
                TypePathResolution::Resolved(resolved_ty) => match resolved_ty.kind() {
                    TyKind::Protocol { .. } => resolved_ty,
                    TyKind::Struct { symbol, .. } => {
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: span.clone(),
                            name: symbol.metadata().name().value.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                        Ty::error(span)
                    }
                    TyKind::TypeAlias { symbol, .. } => {
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: span.clone(),
                            name: symbol.metadata().name().value.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                        Ty::error(span)
                    }
                    _ => {
                        ctx.diagnostics.throw(NotAProtocolError {
                            span: span.clone(),
                            name: bound_name.clone(),
                            context: NotAProtocolContext::Bound,
                        });
                        Ty::error(span)
                    }
                },
                TypePathResolution::NotFound { .. } => {
                    ctx.diagnostics.throw(UnresolvedTypeError {
                        span: span.clone(),
                        type_name: bound_name.clone(),
                    });
                    Ty::error(span)
                }
                TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
                    ctx.diagnostics.throw(NotAProtocolError {
                        span: span.clone(),
                        name: bound_name.clone(),
                        context: NotAProtocolContext::Bound,
                    });
                    Ty::error(span)
                }
            }
        })
        .collect()
}
