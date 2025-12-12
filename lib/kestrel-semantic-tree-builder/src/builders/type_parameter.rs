use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, WhereClause};
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use kestrel_syntax_tree::utils::{extract_path_segments, find_child, get_node_span};

pub fn extract_type_parameters(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
) -> Vec<Arc<TypeParameterSymbol>> {
    let type_param_list = match find_child(syntax, SyntaxKind::TypeParameterList) {
        Some(node) => node,
        None => return Vec::new(),
    };

    let mut type_params = Vec::new();

    for child in type_param_list.children() {
        if child.kind() == SyntaxKind::TypeParameter {
            if let Some(param) = parse_type_parameter(&child, source, file_id, parent.clone()) {
                type_params.push(Arc::new(param));
            }
        }
    }

    type_params
}

pub fn add_type_params_as_children(
    type_params: &[Arc<TypeParameterSymbol>],
    owner: &Arc<dyn Symbol<KestrelLanguage>>,
) {
    for type_param in type_params {
        owner
            .metadata()
            .add_child(&(type_param.clone() as Arc<dyn Symbol<KestrelLanguage>>));
    }
}

fn parse_type_parameter(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
) -> Option<TypeParameterSymbol> {
    let (name_text, name_span) = extract_type_param_name(syntax, file_id)?;
    let full_span = get_node_span(syntax, file_id);
    let name = Spanned::new(name_text, name_span);

    let default_ty = extract_default_type(syntax, source, file_id);

    Some(if let Some(default) = default_ty {
        TypeParameterSymbol::with_default(name, full_span, default, parent)
    } else {
        TypeParameterSymbol::new(name, full_span, parent)
    })
}

fn extract_type_param_name(
    syntax: &SyntaxNode,
    file_id: usize,
) -> Option<(String, kestrel_span::Span)> {
    for child in syntax.children_with_tokens() {
        if let Some(token) = child.into_token() {
            if token.kind() == SyntaxKind::Identifier {
                let text_range = token.text_range();
                let span: kestrel_span::Span = Span::new(
                    file_id,
                    (text_range.start().into())..(text_range.end().into()),
                );
                return Some((token.text().to_string(), span));
            }
        }
    }

    if let Some(name_node) = find_child(syntax, SyntaxKind::Name) {
        for child in name_node.children_with_tokens() {
            if let Some(token) = child.into_token() {
                if token.kind() == SyntaxKind::Identifier {
                    let text_range = token.text_range();
                    let span: kestrel_span::Span = Span::new(
                        file_id,
                        (text_range.start().into())..(text_range.end().into()),
                    );
                    return Some((token.text().to_string(), span));
                }
            }
        }
    }

    None
}

fn extract_default_type(syntax: &SyntaxNode, source: &str, file_id: usize) -> Option<Ty> {
    let default_node = find_child(syntax, SyntaxKind::DefaultType)?;
    let ty_node = find_child(&default_node, SyntaxKind::Ty)?;
    extract_ty_from_node(&ty_node, source, file_id)
}

fn extract_ty_from_node(ty_node: &SyntaxNode, source: &str, file_id: usize) -> Option<Ty> {
    let span = get_node_span(ty_node, file_id);
    let variant_node = ty_node.children().next()?;

    match variant_node.kind() {
        SyntaxKind::TyUnit => Some(Ty::unit(span)),
        SyntaxKind::TyNever => Some(Ty::never(span)),
        SyntaxKind::TyPath => {
            let path_node = find_child(&variant_node, SyntaxKind::Path)?;
            let segments = extract_path_segments(&path_node);
            if segments.is_empty() {
                None
            } else {
                Some(Ty::error(span))
            }
        }
        SyntaxKind::TyTuple => {
            let elements: Vec<Ty> = variant_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::Ty)
                .filter_map(|c| extract_ty_from_node(&c, source, file_id))
                .collect();
            Some(Ty::tuple(elements, span))
        }
        _ => None,
    }
}

pub fn extract_where_clause(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    type_params: &[Arc<TypeParameterSymbol>],
) -> WhereClause {
    let where_clause_node = match find_child(syntax, SyntaxKind::WhereClause) {
        Some(node) => node,
        None => return WhereClause::new(),
    };

    let mut constraints = Vec::new();

    for child in where_clause_node.children() {
        match child.kind() {
            SyntaxKind::TypeBound => {
                if let Some(constraint) = parse_type_bound(&child, source, file_id, type_params) {
                    constraints.push(constraint);
                }
            }
            SyntaxKind::TypeEquality => {
                if let Some(constraint) = parse_type_equality(&child, source, file_id) {
                    constraints.push(constraint);
                }
            }
            _ => {}
        }
    }

    WhereClause::with_constraints(constraints)
}

fn parse_type_bound(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    type_params: &[Arc<TypeParameterSymbol>],
) -> Option<Constraint> {
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    let name_token = name_node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;

    let param_name = name_token.text().to_string();
    let text_range = name_token.text_range();
    let param_span: kestrel_span::Span =
        Span::new(file_id, (text_range.start().into())..(text_range.end().into()));

    let param_id = type_params
        .iter()
        .find(|p| p.metadata().name().value == param_name)
        .map(|p| p.metadata().id());

    let bounds: Vec<Ty> = syntax
        .children()
        .filter(|c| c.kind() == SyntaxKind::Path)
        .map(|path_node| {
            let span = get_node_span(&path_node, file_id);
            Ty::error(span)
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

fn parse_type_equality(syntax: &SyntaxNode, _source: &str, file_id: usize) -> Option<Constraint> {
    let span = get_node_span(syntax, file_id);

    let left_target = find_child(syntax, SyntaxKind::AssociatedTypeTarget)?;
    let left_span = get_node_span(&left_target, file_id);
    let left = Ty::error(left_span);

    let right_node = find_child(syntax, SyntaxKind::Ty)?;
    let right_span = get_node_span(&right_node, file_id);
    let right = Ty::error(right_span);

    Some(Constraint::type_equality(left, right, span))
}
