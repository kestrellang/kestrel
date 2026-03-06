//! Shared extraction helpers for building declaration entities.
//!
//! Extracts visibility, attributes, documentation, conformances, and
//! where clauses from CST nodes.

use kestrel_hecs::{Entity, World};
use kestrel_span2::Span;
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_path_segments, extract_visibility, find_child, is_trivia};

use crate::ast_type::{AstType, PathSegment, ast_type_from_cst};
use crate::components::*;

/// Extract and set visibility component from a declaration node.
pub fn set_visibility(world: &mut World, entity: Entity, node: &SyntaxNode) {
    if let Some(vis_str) = extract_visibility(node) {
        let vis = match vis_str.as_str() {
            "public" => Vis::Public,
            "private" => Vis::Private,
            "internal" => Vis::Internal,
            "fileprivate" => Vis::Fileprivate,
            _ => return,
        };
        world.set(entity, vis);
    }
}

/// Extract and set attributes from a declaration node.
pub fn set_attributes(world: &mut World, entity: Entity, node: &SyntaxNode) {
    let attr_list = find_child(node, SyntaxKind::AttributeList);
    let attrs: Vec<AstAttribute> = attr_list
        .iter()
        .flat_map(|list| list.children())
        .filter(|child| child.kind() == SyntaxKind::Attribute)
        .filter_map(|n| extract_attribute(&n))
        .collect();

    if !attrs.is_empty() {
        world.set(entity, Attributes(attrs));
    }
}

/// Extract a single attribute from an Attribute CST node.
fn extract_attribute(node: &SyntaxNode) -> Option<AstAttribute> {
    // Attribute name is the identifier token after @
    let name = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?
        .text()
        .to_string();

    // Extract args from AttributeArgs child if present
    let args = find_child(node, SyntaxKind::AttributeArgs)
        .map(|args_node| {
            args_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::AttributeArg)
                .filter_map(|n| extract_attribute_arg(&n))
                .collect()
        })
        .unwrap_or_default();

    Some(AstAttribute { name, args })
}

/// Extract a single attribute argument.
fn extract_attribute_arg(node: &SyntaxNode) -> Option<AstAttributeArg> {
    let tokens: Vec<_> = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !is_trivia(t.kind()))
        .collect();

    // Check for label: value pattern (has a Colon token)
    let colon_pos = tokens.iter().position(|t| t.kind() == SyntaxKind::Colon);

    if let Some(pos) = colon_pos {
        let label = tokens.get(pos.wrapping_sub(1))
            .map(|t| t.text().to_string());
        let value = extract_value_from_tokens(&tokens[(pos + 1)..])
            .unwrap_or_default();
        Some(AstAttributeArg { label, value })
    } else {
        // Just a value, no label
        let value = extract_value_from_tokens(&tokens)?;
        Some(AstAttributeArg { label: None, value })
    }
}

/// Extract a value from tokens, handling implicit member syntax (.Name).
///
/// For `@builtin(.Copyable)`, the tokens are [Dot, Identifier("Copyable")].
/// We combine them into ".Copyable" so consumers can recognize the pattern.
fn extract_value_from_tokens(tokens: &[kestrel_syntax_tree2::SyntaxToken]) -> Option<String> {
    if tokens.len() >= 2
        && tokens[0].kind() == SyntaxKind::Dot
        && tokens[1].kind() == SyntaxKind::Identifier
    {
        Some(format!(".{}", tokens[1].text()))
    } else {
        tokens.first().map(|t| t.text().to_string())
    }
}

/// Extract and set documentation from leading line comments.
pub fn set_documentation(world: &mut World, entity: Entity, node: &SyntaxNode) {
    let mut doc_lines = Vec::new();

    // Walk tokens before the first non-trivia element to find doc comments
    for elem in node.children_with_tokens() {
        match elem {
            rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::LineComment => {
                let text = t.text();
                // Strip comment prefix: "///" doc comment or "//" regular comment
                let stripped = if let Some(rest) = text.strip_prefix("///") {
                    rest.strip_prefix(' ').unwrap_or(rest)
                } else {
                    continue; // Skip non-doc comments
                };
                doc_lines.push(stripped.to_string());
            }
            rowan::NodeOrToken::Token(t) if is_trivia(t.kind()) => continue,
            _ => break, // Stop at first non-trivia element
        }
    }

    if !doc_lines.is_empty() {
        world.set(entity, Documentation(doc_lines.join("\n")));
    }
}

/// Extract and set conformances from a ConformanceList child.
pub fn set_conformances(world: &mut World, entity: Entity, node: &SyntaxNode, file_id: usize) {
    let conf_list = match find_child(node, SyntaxKind::ConformanceList) {
        Some(list) => list,
        None => return,
    };

    let items: Vec<ConformanceItem> = conf_list
        .children()
        .filter_map(|child| {
            match child.kind() {
                SyntaxKind::ConformanceItem => {
                    // Positive conformance — find the type inside
                    let ty = child
                        .children()
                        .find(|c| is_type_kind(c.kind()))
                        .and_then(|c| ast_type_from_cst(&c, file_id))?;
                    Some(ConformanceItem::Positive(ty, child))
                }
                SyntaxKind::NegativeConformance => {
                    // Negative conformance — `not Protocol`
                    let ty = child
                        .children()
                        .find(|c| is_type_kind(c.kind()))
                        .and_then(|c| ast_type_from_cst(&c, file_id))?;
                    Some(ConformanceItem::Negative(ty, child))
                }
                _ => None,
            }
        })
        .collect();

    if !items.is_empty() {
        world.set(entity, Conformances(items));
    }
}

/// Extract and set where clause from a WhereClause child.
///
/// CST structure for TypeBound:
/// ```text
/// TypeBound " T: Comparable"
///   Name " T"
///     Identifier "T"
///   Path ": Comparable"
///     PathElement ": Comparable"
///       Identifier "Comparable"
/// ```
/// Subject comes from Name, conformances come from Path children.
pub fn set_where_clause(world: &mut World, entity: Entity, node: &SyntaxNode, file_id: usize) {
    let where_node = match find_child(node, SyntaxKind::WhereClause) {
        Some(w) => w,
        None => return,
    };

    let constraints: Vec<WhereConstraint> = where_node
        .children()
        .filter_map(|child| {
            match child.kind() {
                SyntaxKind::TypeBound => {
                    // Subject is a Name (simple: T) or AssociatedTypeTarget (dotted: T.Item)
                    let subject = bound_subject_to_ast_type(&child, file_id)?;

                    // Protocol conformances come from Path children
                    let protocols: Vec<_> = child
                        .children()
                        .filter(|c| c.kind() == SyntaxKind::Path)
                        .filter_map(|path| path_to_ast_type(&path, file_id))
                        .collect();

                    if protocols.is_empty() {
                        return None;
                    }

                    // Check for negative bound (`not`)
                    let has_not = child
                        .children_with_tokens()
                        .any(|e| e.as_token().is_some_and(|t| t.kind() == SyntaxKind::Not));
                    if has_not && protocols.len() == 1 {
                        Some(WhereConstraint::NegativeBound {
                            subject,
                            protocol: protocols.into_iter().next().unwrap(),
                            node: child,
                        })
                    } else {
                        Some(WhereConstraint::Bound {
                            subject,
                            protocols,
                            node: child,
                        })
                    }
                }
                SyntaxKind::TypeEquality => {
                    // Type equality uses Ty nodes or Name/Path
                    let mut type_children = child
                        .children()
                        .filter(|c| is_type_kind(c.kind()) || c.kind() == SyntaxKind::Name || c.kind() == SyntaxKind::Path);

                    let lhs_node = type_children.next()?;
                    let lhs = node_to_ast_type(&lhs_node, file_id)?;
                    let rhs_node = type_children.next()?;
                    let rhs = node_to_ast_type(&rhs_node, file_id)?;

                    Some(WhereConstraint::Equality {
                        lhs,
                        rhs,
                        node: child,
                    })
                }
                _ => None,
            }
        })
        .collect();

    if !constraints.is_empty() {
        world.set(entity, WhereClause(constraints));
    }
}

/// Extract the subject of a TypeBound as AstType.
/// Handles both simple `Name` (T) and `AssociatedTypeTarget` (T.Item) nodes.
fn bound_subject_to_ast_type(parent: &SyntaxNode, file_id: usize) -> Option<AstType> {
    // Try AssociatedTypeTarget first (T.Item — contains a Path)
    if let Some(assoc) = find_child(parent, SyntaxKind::AssociatedTypeTarget) {
        if let Some(path) = find_child(&assoc, SyntaxKind::Path) {
            return path_to_ast_type(&path, file_id);
        }
    }
    // Fall back to simple Name (T)
    name_to_ast_type(parent, file_id)
}

/// Convert a Name node to AstType::Named.
fn name_to_ast_type(parent: &SyntaxNode, file_id: usize) -> Option<AstType> {
    let name_node = find_child(parent, SyntaxKind::Name)?;
    let ident = kestrel_syntax_tree2::utils::extract_identifier_from_name(&name_node)?;
    let range = name_node.text_range();
    let span = Span::new(file_id, (range.start().into())..(range.end().into()));
    Some(AstType::Named {
        segments: vec![PathSegment { name: ident, type_args: vec![], span: span.clone() }],
        span,
    })
}

/// Convert a Path node to AstType::Named.
fn path_to_ast_type(path_node: &SyntaxNode, file_id: usize) -> Option<AstType> {
    let names = extract_path_segments(path_node);
    if names.is_empty() {
        return None;
    }
    let range = path_node.text_range();
    let span = Span::new(file_id, (range.start().into())..(range.end().into()));
    let segments = names.into_iter()
        .map(|name| PathSegment { name, type_args: vec![], span: span.clone() })
        .collect();
    Some(AstType::Named { segments, span })
}

/// Convert various node kinds to AstType.
fn node_to_ast_type(node: &SyntaxNode, file_id: usize) -> Option<AstType> {
    match node.kind() {
        SyntaxKind::Name => {
            let ident = kestrel_syntax_tree2::utils::extract_identifier_from_name(node)?;
            let range = node.text_range();
            let span = Span::new(file_id, (range.start().into())..(range.end().into()));
            Some(AstType::Named {
                segments: vec![PathSegment { name: ident, type_args: vec![], span: span.clone() }],
                span,
            })
        }
        SyntaxKind::Path => path_to_ast_type(node, file_id),
        _ if is_type_kind(node.kind()) => ast_type_from_cst(node, file_id),
        _ => None,
    }
}

/// Check if a SyntaxKind is a type-related node.
fn is_type_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Ty
            | SyntaxKind::TyPath
            | SyntaxKind::TyTuple
            | SyntaxKind::TyFunction
            | SyntaxKind::TyArray
            | SyntaxKind::TyDictionary
            | SyntaxKind::TyOptional
            | SyntaxKind::TyResult
            | SyntaxKind::TyUnit
            | SyntaxKind::TyNever
            | SyntaxKind::TyInferred
    )
}

/// Check if a declaration node has a StaticModifier child.
pub fn has_static_modifier(node: &SyntaxNode) -> bool {
    find_child(node, SyntaxKind::StaticModifier).is_some()
}
