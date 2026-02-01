use std::sync::Arc;

use kestrel_semantic_model::{ResolveTypePath, SymbolFor, TypePathResolution};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::FlattenedProtocolBehavior;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind, WhereClause};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::binders::utils::type_paths::resolve_protocol_bound_path;
use crate::declaration_binder::BindingContext;
use crate::diagnostics::{
    DuplicateTypeParameterError, ShadowedTypeParameterError, WhereClauseAssociatedTypeNotFoundError,
};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};
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

    // Check for duplicate and shadowed type parameter names
    check_duplicate_type_parameters(&type_parameters, &symbol, ctx);

    // Collect outer type parameters for where clause resolution.
    // This allows method-level where clauses to reference parent type parameters.
    // E.g., `func clone() -> Set[T, A] where T: Cloneable` can reference struct's T
    let outer_type_params = collect_outer_type_parameters(&symbol);
    let all_type_params: Vec<_> = type_parameters
        .iter()
        .chain(outer_type_params.iter())
        .cloned()
        .collect();

    let where_clause =
        resolve_where_clause(syntax, source, file_id, context_id, ctx, &all_type_params);
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
        if let Some(constraint) = resolve_type_bound(
            &child,
            source,
            file_id,
            context_id,
            ctx,
            type_params,
            &constraints,
        ) {
            constraints.push(constraint);
        }
    }

    // Pass 2: TypeEquality constraints
    for child in where_clause_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::TypeEquality)
    {
        if let Some(constraint) = resolve_type_equality(
            &child,
            source,
            file_id,
            context_id,
            ctx,
            type_params,
            &constraints,
        ) {
            constraints.push(constraint);
        }
    }

    WhereClause::with_constraints(constraints)
}

fn resolve_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
    type_params: &[Arc<TypeParameterSymbol>],
    already_collected: &[Constraint],
) -> Option<Constraint> {
    if let Some(target_node) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
        let path_segments = extract_path_from_node(&target_node);
        let target_span = get_node_span(&target_node, file_id);

        // Check if this is a Self.Item: Protocol constraint (for protocol extensions)
        if !path_segments.is_empty() && path_segments[0] == "Self" {
            let bounds =
                resolve_protocol_bounds_from_type_bound(syntax, source, file_id, context_id, ctx);
            if bounds.is_empty() {
                return None;
            }

            // Create SelfBound with the associated type path (everything after "Self")
            let associated_type_path: Vec<String> = path_segments[1..].to_vec();
            return Some(Constraint::self_bound(
                associated_type_path,
                target_span,
                bounds,
            ));
        }

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

        let bounds =
            resolve_protocol_bounds_from_type_bound(syntax, source, file_id, context_id, ctx);
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

    // Simple bound: T: Protocol or T: not Copyable or Self: Protocol
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let param_name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let param_span: Span = Span::new(
        file_id,
        (text_range.start().into())..(text_range.end().into()),
    );

    // Check if this is a Self: Protocol constraint (for protocol extensions)
    if param_name == "Self" {
        // Check if this is a negative bound (Self: not Copyable) - not typical but handle it
        if find_child(syntax, SyntaxKind::NegativeConformance).is_some() {
            // For now, negative Self bounds are not supported - fall through to regular handling
            // which will create an unresolved constraint
        } else {
            let bounds =
                resolve_protocol_bounds_from_type_bound(syntax, source, file_id, context_id, ctx);
            if bounds.is_empty() {
                return None;
            }

            // Create SelfBound with empty associated type path (just Self: Protocol)
            return Some(Constraint::self_bound(Vec::new(), param_span, bounds));
        }
    }

    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == param_name)
        .map(|p| p.metadata().id());

    // Check if this is a negative bound (T: not Copyable)
    if let Some(neg_conformance) = find_child(syntax, SyntaxKind::NegativeConformance) {
        // Resolve the negated protocol
        let bound = resolve_negative_bound(&neg_conformance, source, file_id, context_id, ctx);

        return Some(match param_id {
            Some(id) => Constraint::negative_bound(id, param_name, param_span, bound),
            None => Constraint::unresolved_negative_bound(param_name, param_span, bound),
        });
    }

    // Positive bound: T: Protocol
    let bounds = resolve_protocol_bounds_from_type_bound(syntax, source, file_id, context_id, ctx);
    if bounds.is_empty() {
        return None;
    }

    Some(match param_id {
        Some(id) => Constraint::type_bound(id, param_name, param_span, bounds),
        None => Constraint::unresolved_type_bound(param_name, param_span, bounds),
    })
}

/// Resolve a negative bound (the protocol after `not`)
fn resolve_negative_bound(
    syntax: &SyntaxNode,
    _source: &str,
    file_id: usize,
    context_id: SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    // Find the Path inside NegativeConformance
    if let Some(path_node) = find_child(syntax, SyntaxKind::Path) {
        let span = get_node_span(&path_node, file_id);
        let segments = extract_path_segments(&path_node);

        if segments.is_empty() {
            return Ty::error(span);
        }

        return resolve_protocol_bound_path(&segments, span, context_id, ctx);
    }

    let span = get_node_span(syntax, file_id);
    Ty::error(span)
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

    let (left_path, left_span) =
        if let Some(left_target) = find_child(syntax, SyntaxKind::AssociatedTypeTarget) {
            (
                extract_path_from_node(&left_target),
                get_node_span(&left_target, file_id),
            )
        } else if let Some(name_node) = find_child(syntax, SyntaxKind::Name) {
            let name_token = name_node
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)?;
            let text_range = name_token.text_range();
            let name_span: Span = Span::new(
                file_id,
                (text_range.start().into())..(text_range.end().into()),
            );
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

    let right_ty = if let Some(ty_path_node) =
        ty_node.children().find(|c| c.kind() == SyntaxKind::TyPath)
    {
        if ty_path_node
            .children()
            .any(|c| c.kind() == SyntaxKind::TypeArgumentList)
        {
            let mut type_ctx =
                TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
            resolve_type_from_ty_node(&ty_node, &mut type_ctx)
        } else if let Some(path_node) = ty_path_node
            .children()
            .find(|c| c.kind() == SyntaxKind::Path)
        {
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
    if let Some(type_param) = type_params
        .iter()
        .find(|p| p.metadata().name().value == path[0])
    {
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
                        },
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
                        && let Ok(assoc_sym) = child
                            .clone()
                            .into_any_arc()
                            .downcast::<kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol>()
                        {
                            let container = Ty::type_parameter(type_param.clone(), span.clone());
                            return Ty::qualified_associated_type(assoc_sym, container, span.clone());
                        }
                }

                if let Some(flattened) = symbol
                    .metadata()
                    .get_behavior::<FlattenedProtocolBehavior>()
                    && let Some(flattened_assoc) = flattened.associated_types().get(assoc_type_name)
                {
                    let container = Ty::type_parameter(type_param.clone(), span.clone());
                    return Ty::qualified_associated_type(
                        flattened_assoc.symbol.clone(),
                        container,
                        span.clone(),
                    );
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
    source: &str,
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

        // If the next sibling is TypeArgumentList, parse the type arguments and apply them
        if i + 1 < children.len() && children[i + 1].kind() == SyntaxKind::TypeArgumentList {
            let type_arg_list = &children[i + 1];

            // First resolve the base protocol
            let base_ty = resolve_protocol_bound_path(&segments, span.clone(), context_id, ctx);

            // If it's a valid protocol, apply type arguments
            if let TyKind::Protocol { symbol, .. } = base_ty.kind() {
                // Resolve type arguments
                let mut type_args: Vec<Ty> = Vec::new();
                for ty_node in type_arg_list
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::Ty)
                {
                    let mut type_ctx = TypeSyntaxContext::new(
                        ctx.model,
                        ctx.diagnostics,
                        source,
                        file_id,
                        context_id,
                    );
                    let resolved_ty = resolve_type_from_ty_node(&ty_node, &mut type_ctx);
                    type_args.push(resolved_ty);
                }

                // Build substitutions from type parameters to type arguments
                let type_params = symbol.type_parameters();
                let mut substitutions = kestrel_semantic_tree::ty::Substitutions::new();
                for (i, param) in type_params.iter().enumerate() {
                    if i < type_args.len() {
                        substitutions.insert(param.metadata().id(), type_args[i].clone());
                    }
                }

                // Create protocol type with substitutions
                let protocol_ty = Ty::generic_protocol(symbol.clone(), substitutions, span);
                bounds.push(protocol_ty);
            } else {
                bounds.push(base_ty);
            }
            i += 2;
            continue;
        }

        bounds.push(resolve_protocol_bound_path(
            &segments, span, context_id, ctx,
        ));
        i += 1;
    }

    bounds
}

fn extract_path_from_node(node: &SyntaxNode) -> Vec<String> {
    let mut segments = Vec::new();

    if let Some(path_node) = find_child(node, SyntaxKind::Path) {
        for child in path_node.children() {
            if child.kind() == SyntaxKind::PathElement {
                for elem in child.children_with_tokens() {
                    if let Some(token) = elem.into_token()
                        && token.kind() == SyntaxKind::Identifier
                    {
                        segments.push(token.text().to_string());
                    }
                }
            }
        }
    }

    segments
}

/// Check for duplicate type parameter names within the same list,
/// and for shadowing of type parameters from outer scopes.
fn check_duplicate_type_parameters(
    type_params: &[Arc<TypeParameterSymbol>],
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ctx: &mut BindingContext,
) {
    use std::collections::HashMap;

    // Check for duplicates within the same parameter list
    let mut seen: HashMap<String, usize> = HashMap::new();

    for (i, param) in type_params.iter().enumerate() {
        let name = param.metadata().name().value.clone();
        if let Some(&first_idx) = seen.get(&name) {
            let first = &type_params[first_idx];
            ctx.diagnostics.throw(DuplicateTypeParameterError {
                name,
                first_span: first.metadata().name().span.clone(),
                duplicate_span: param.metadata().name().span.clone(),
            });
        } else {
            seen.insert(name, i);
        }
    }

    // Check for shadowing from outer scopes
    let outer_type_params = collect_outer_type_parameters(symbol);

    // For static methods, we allow shadowing of type parameters from the containing struct/enum
    // because static methods don't have access to the instance's type parameters.
    let allow_parent_shadowing = is_static_method(symbol);

    for param in type_params {
        let name = &param.metadata().name().value;
        if let Some(outer_param) = outer_type_params
            .iter()
            .find(|p| &p.metadata().name().value == name)
        {
            // If this is a static method and the outer param is from the immediate parent (struct/enum),
            // skip the shadowing error
            if allow_parent_shadowing && is_from_immediate_parent(outer_param, symbol) {
                continue;
            }

            ctx.diagnostics.throw(ShadowedTypeParameterError {
                name: name.clone(),
                outer_span: outer_param.metadata().name().span.clone(),
                inner_span: param.metadata().name().span.clone(),
            });
        }
    }
}

/// Collect type parameters from all ancestor scopes.
fn collect_outer_type_parameters(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Vec<Arc<TypeParameterSymbol>> {
    let mut result = Vec::new();
    let mut current = symbol.metadata().parent();

    while let Some(parent) = current {
        // Collect type parameters from this ancestor
        let children: Vec<Arc<dyn Symbol<KestrelLanguage>>> = parent.metadata().children();
        for child in children {
            if child.metadata().kind() == KestrelSymbolKind::TypeParameter
                && let Ok(type_param) = child.downcast_arc::<TypeParameterSymbol>()
            {
                result.push(type_param);
            }
        }
        current = parent.metadata().parent();
    }

    result
}

/// Check if the symbol is a static method.
fn is_static_method(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> bool {
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    if symbol.metadata().kind() != KestrelSymbolKind::Function {
        return false;
    }

    if let Ok(func) = symbol.clone().downcast_arc::<FunctionSymbol>() {
        func.is_static()
    } else {
        false
    }
}

/// Check if a type parameter is from the immediate parent of the symbol.
fn is_from_immediate_parent(
    type_param: &Arc<TypeParameterSymbol>,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> bool {
    let Some(parent) = symbol.metadata().parent() else {
        return false;
    };

    // Check if the type parameter's parent is the same as symbol's parent
    if let Some(tp_parent) = type_param.metadata().parent() {
        tp_parent.metadata().id() == parent.metadata().id()
    } else {
        false
    }
}
