use std::sync::Arc;

use kestrel_semantic_model::{ResolveTypePath, SymbolFor, TypePathResolution};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::FlattenedProtocolBehavior;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::declaration_binder::BindingContext;
use crate::diagnostics::{
    NotAProtocolContext, NotAProtocolError, UnresolvedTypeError, WhereClauseAssociatedTypeNotFoundError,
};
use crate::resolution::type_resolver::{resolve_type_from_ty_node, TypeSyntaxContext};
use kestrel_syntax_tree::utils::{extract_path_segments, find_child, get_node_span};

pub(crate) fn resolve_generics(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> GenericsBehavior {
    let symbol = match ctx.model.query(SymbolFor { id: context_id }) {
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

    let where_clause = resolve_where_clause(syntax, source, file_id, context_id, ctx, &type_parameters);
    GenericsBehavior::new(type_parameters, where_clause)
}

pub(crate) fn resolve_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let mut constraints = Vec::new();

    // Pass 1: TypeBound constraints (needed for T.Item references)
    for child in where_clause_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::TypeBound)
    {
        if let Some(constraint) =
            resolve_type_bound(&child, source, file_id, context_id, ctx, type_params, &constraints)
        {
            constraints.push(constraint);
        }
    }

    // Pass 2: TypeEquality constraints
    for child in where_clause_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::TypeEquality)
    {
        if let Some(constraint) =
            resolve_type_equality(&child, source, file_id, context_id, ctx, type_params, &constraints)
        {
            constraints.push(constraint);
        }
    }

    WhereClause::with_constraints(constraints)
}

fn resolve_type_bound(
    syntax: &SyntaxNode,
    _source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    already_collected: &[Constraint],
) -> Option<Constraint> {
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        let path_segments = extract_path_from_node(&target_node);
        let target_span = get_node_span(&target_node, file_id);

        if path_segments.len() >= 2 {
            // If the first segment is a type parameter, validate that the associated type exists
            // on at least one protocol bound (when possible).
            let type_param_name = &path_segments[0];
            let assoc_type_name = &path_segments[1];
            validate_type_param_associated_type_target(
                type_param_name,
                assoc_type_name,
                &target_span,
                type_params,
                already_collected,
                ctx,
            );
        }

        let bounds = resolve_protocol_bounds_from_type_bound(syntax, file_id, context_id, ctx);
        if bounds.is_empty() {
            return None;
        }

        let full_path = path_segments.join(".");
        return Some(Constraint::inherited_assoc_type_bound(
            full_path,
            target_span,
            bounds,
        ));
    }

    // Simple bound: T: Protocol
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let param_name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let param_span: Span = Span::new(file_id, (text_range.start().into())..(text_range.end().into()));

    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == param_name)
        .map(|p| p.metadata().id());

    let bounds = resolve_protocol_bounds_from_type_bound(syntax, file_id, context_id, ctx);
    if bounds.is_empty() {
        return None;
    }

    Some(match param_id {
        Some(id) => Constraint::type_bound(id, param_name, param_span, bounds),
        None => Constraint::unresolved_type_bound(param_name, param_span, bounds),
    })
}

fn resolve_type_equality(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    already_collected: &[Constraint],
) -> Option<Constraint> {
    let span = get_node_span(syntax, file_id);

    let (left_path, left_span) = if let Some(left_target) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        (extract_path_from_node(&left_target), get_node_span(&left_target, file_id))
    } else if let Some(name_node) = find_child(syntax, SyntaxKind::Name) {
        let name_token = name_node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)?;
        let text_range = name_token.text_range();
        let name_span: Span =
            Span::new(file_id, (text_range.start().into())..(text_range.end().into()));
        (vec![name_token.text().to_string()], name_span)
    } else {
        return None;
    };

    let left_ty = resolve_path_in_where_clause(
        &left_path,
        &left_span,
        context_id,
        type_params,
        already_collected,
        ctx,
    );

    let ty_node = find_child(syntax, SyntaxKind::Ty)?;

    let right_ty = if let Some(ty_path_node) = ty_node.children().find(|c| c.kind() == SyntaxKind::TyPath) {
        if let Some(path_node) = ty_path_node.children().find(|c| c.kind() == SyntaxKind::Path) {
            let right_path = extract_path_segments(&path_node);
            let right_span = get_node_span(&ty_node, file_id);
            let resolved = resolve_path_in_where_clause(
                &right_path,
                &right_span,
                context_id,
                type_params,
                already_collected,
                ctx,
            );
            if !resolved.is_error() {
                resolved
            } else {
                let mut type_ctx =
                    TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
                resolve_type_from_ty_node(&ty_node, &mut type_ctx)
            }
        } else {
            let mut type_ctx =
                TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
            resolve_type_from_ty_node(&ty_node, &mut type_ctx)
        }
    } else {
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        resolve_type_from_ty_node(&ty_node, &mut type_ctx)
    };

    Some(Constraint::type_equality(left_ty, right_ty, span))
}

fn resolve_path_in_where_clause(
    path: &[String],
    span: &Span,
    context_id: SymbolId,
    type_params: &[Arc<TypeParameterSymbol>],
    already_collected: &[Constraint],
    ctx: &BindingContext,
) -> Ty {
    if path.is_empty() {
        return Ty::error(span.clone());
    }

    // Prefer type parameter paths (T, T.Item) so we can resolve using already-collected bounds.
    if let Some(type_param) = type_params.iter().find(|p| p.metadata().name().value == path[0]) {
        if path.len() == 1 {
            return Ty::type_parameter(type_param.clone(), span.clone());
        }

        // Associated type path: T.Item
        let param_id = type_param.metadata().id();
        let bounds: Vec<&Ty> = already_collected
            .iter()
            .filter_map(|c| {
                if c.param_id() == Some(param_id) {
                    match c {
                        Constraint::TypeBound { bounds, .. } => {
                            Some(bounds.iter().collect::<Vec<_>>())
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        let assoc_type_name = &path[1];
        for bound in bounds {
            if let TyKind::Protocol { symbol, .. } = bound.kind() {
                for child in symbol.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == *assoc_type_name
                    {
                        if let Ok(assoc_sym) = child
                            .clone()
                            .into_any_arc()
                            .downcast::<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>()
                        {
                            let container = Ty::type_parameter(type_param.clone(), span.clone());
                            return Ty::qualified_associated_type(assoc_sym, container, span.clone());
                        }
                    }
                }

                if let Some(flattened) = symbol.metadata().get_behavior::<FlattenedProtocolBehavior>() {
                    if let Some(flattened_assoc) = flattened.associated_types().get(assoc_type_name) {
                        let container = Ty::type_parameter(type_param.clone(), span.clone());
                        return Ty::qualified_associated_type(
                            flattened_assoc.symbol.clone(),
                            container,
                            span.clone(),
                        );
                    }
                }
            }
        }

        return Ty::error(span.clone());
    }

    // Fall back to model type-path resolution for non-type-parameter paths.
    match ctx.model.query(ResolveTypePath {
        path: path.to_vec(),
        context: context_id,
    }) {
        TypePathResolution::Resolved(ty) => ty,
        _ => Ty::error(span.clone()),
    }
}

fn validate_type_param_associated_type_target(
    type_param_name: &str,
    assoc_type_name: &str,
    span: &Span,
    type_params: &[Arc<TypeParameterSymbol>],
    already_collected: &[Constraint],
    ctx: &mut BindingContext,
) {
    let Some(type_param) = type_params
        .iter()
        .find(|p| p.metadata().name().value == type_param_name)
    else {
        return;
    };

    let param_id = type_param.metadata().id();
    let bounds: Vec<&Ty> = already_collected
        .iter()
        .filter_map(|c| {
            if c.param_id() == Some(param_id) {
                match c {
                    Constraint::TypeBound { bounds, .. } => Some(bounds.iter().collect::<Vec<_>>()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .flatten()
        .collect();

    if bounds.is_empty() {
        return;
    }

    let mut protocol_name = String::new();
    for bound in &bounds {
        if let TyKind::Protocol { symbol, .. } = bound.kind() {
            protocol_name = symbol.metadata().name().value.clone();
            let has_type = symbol.metadata().children().iter().any(|child| {
                child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == assoc_type_name
            });
            if has_type {
                return;
            }
        }
    }

    ctx.diagnostics
        .throw(WhereClauseAssociatedTypeNotFoundError {
            span: span.clone(),
            type_param: type_param_name.to_string(),
            assoc_type_name: assoc_type_name.to_string(),
            protocol_name,
        });
}

fn resolve_protocol_bounds_from_type_bound(
    syntax: &SyntaxNode,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> Vec<Ty> {
    let children: Vec<_> = syntax.children().collect();
    let mut bounds: Vec<Ty> = Vec::new();
    let mut i = 0;

    while i < children.len() {
        let child = &children[i];
        if child.kind() != SyntaxKind::Path {
            i += 1;
            continue;
        }

        let span = get_node_span(child, file_id);
        let segments = extract_path_segments(child);

        if segments.is_empty() {
            bounds.push(Ty::error(span));
            i += 1;
            continue;
        }

        // If the next sibling is TypeArgumentList, accept syntax but emit the existing diagnostic
        // and treat the bound as error for now.
        if i + 1 < children.len() && children[i + 1].kind() == SyntaxKind::TypeArgumentList {
            use crate::diagnostics::UnsupportedGenericProtocolBoundError;
            let protocol_name = segments.join("::");
            ctx.diagnostics.throw(UnsupportedGenericProtocolBoundError {
                span: span.clone(),
                protocol_name,
            });
            bounds.push(Ty::error(span));
            i += 2;
            continue;
        }

        bounds.push(resolve_protocol_bound_path(
            &segments,
            span,
            context_id,
            ctx,
        ));
        i += 1;
    }

    bounds
}

fn resolve_protocol_bound_path(
    segments: &[String],
    span: Span,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    if segments.is_empty() {
        return Ty::error(span);
    }

    let bound_name = segments.join(".");
    match ctx.model.query(ResolveTypePath {
        path: segments.to_vec(),
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
                    name: bound_name,
                    context: NotAProtocolContext::Bound,
                });
                Ty::error(span)
            }
        },
        TypePathResolution::NotFound { .. } => {
            ctx.diagnostics.throw(UnresolvedTypeError {
                span: span.clone(),
                type_name: bound_name,
            });
            Ty::error(span)
        }
        TypePathResolution::Ambiguous { .. } | TypePathResolution::NotAType { .. } => {
            ctx.diagnostics.throw(NotAProtocolError {
                span: span.clone(),
                name: bound_name,
                context: NotAProtocolContext::Bound,
            });
            Ty::error(span)
        }
    }
}

fn extract_path_from_node(node: &SyntaxNode) -> Vec<String> {
    let mut segments = Vec::new();

    if let Some(path_node) = find_child(node, SyntaxKind::Path) {
        for child in path_node.children() {
            if child.kind() == SyntaxKind::PathElement {
                for elem in child.children_with_tokens() {
                    if let Some(token) = elem.into_token() {
                        if token.kind() == SyntaxKind::Identifier {
                            segments.push(token.text().to_string());
                        }
                    }
                }
            }
        }
    }

    segments
}
