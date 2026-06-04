//! Shared extraction helpers for building declaration entities.
//!
//! Extracts visibility, attributes, documentation, conformances, and
//! where clauses from CST nodes.

use kestrel_hecs::{Entity, World};
use kestrel_span::Span;
use kestrel_syntax_tree::utils::{
    extract_path_segments, extract_visibility, find_child, get_decl_span, is_trivia,
};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ast_type::{AstType, PathSegment, ast_type_from_cst};
use crate::components::*;
use crate::lower;

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
pub fn set_attributes(world: &mut World, entity: Entity, node: &SyntaxNode, file_id: usize) {
    let attr_list = find_child(node, SyntaxKind::AttributeList);
    let attrs: Vec<AstAttribute> = attr_list
        .iter()
        .flat_map(|list| list.children())
        .filter(|child| child.kind() == SyntaxKind::Attribute)
        .filter_map(|n| extract_attribute(&n, file_id))
        .collect();

    if !attrs.is_empty() {
        world.set(entity, Attributes(attrs));
    }
}

/// Extract a single attribute from an Attribute CST node.
fn extract_attribute(node: &SyntaxNode, file_id: usize) -> Option<AstAttribute> {
    // Attribute name is the identifier token after @
    let name_token = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)?;
    let name = name_token.text().to_string();

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

    // Use the identifier token's range — rowan Attribute nodes include leading
    // trivia (previous line's newline), which would map the span to the wrong
    // line for diagnostics.
    let range = name_token.text_range();
    let span = Span::new(file_id, (range.start().into())..(range.end().into()));

    Some(AstAttribute { name, args, span })
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
        let label = tokens
            .get(pos.wrapping_sub(1))
            .map(|t| t.text().to_string());
        let value = extract_value_from_tokens(&tokens[(pos + 1)..]).unwrap_or_default();
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
fn extract_value_from_tokens(tokens: &[kestrel_syntax_tree::SyntaxToken]) -> Option<String> {
    if tokens.len() >= 2
        && tokens[0].kind() == SyntaxKind::Dot
        && tokens[1].kind() == SyntaxKind::Identifier
    {
        Some(format!(".{}", tokens[1].text()))
    } else {
        tokens.first().map(|t| t.text().to_string())
    }
}

/// Extract and set documentation from leading `///` line comments and
/// `/** … */` block comments attached to a declaration.
///
/// The parser splices trivia into the tree right before the next AddToken
/// event, so the placement depends on whether the decl has a visibility
/// modifier or attributes:
///
/// - `public struct Foo` — trivia lands *inside* the `Visibility` node,
///   before the `public` keyword token.
/// - `@attr struct Foo` — trivia lands *inside* the `AttributeList`.
/// - `struct Foo` (no preamble) — trivia is a sibling token of the empty
///   `Visibility` node, just before the `struct` keyword.
///
/// To handle all three cases uniformly, we walk `descendants_with_tokens`
/// (a flat in-order token stream) and collect every doc comment we see
/// until we hit the first non-preamble token — i.e. the declaration's
/// own keyword or its name identifier. A non-doc comment between doc
/// blocks resets the accumulator (matches the rustdoc convention).
pub fn set_documentation(world: &mut World, entity: Entity, node: &SyntaxNode) {
    let mut chunks: Vec<String> = Vec::new();
    for elem in node.descendants_with_tokens() {
        let rowan::NodeOrToken::Token(tok) = elem else {
            continue;
        };
        match tok.kind() {
            SyntaxKind::Whitespace | SyntaxKind::Newline => continue,
            SyntaxKind::LineComment => {
                let text = tok.text();
                if is_doc_line(text) {
                    chunks.push(strip_doc_line(text));
                } else {
                    chunks.clear();
                }
            },
            SyntaxKind::BlockComment => {
                let text = tok.text();
                if is_doc_block(text) {
                    chunks.push(strip_doc_block(text));
                } else {
                    chunks.clear();
                }
            },
            // Preamble tokens that may legitimately appear before the
            // declaration keyword.
            SyntaxKind::Public
            | SyntaxKind::Private
            | SyntaxKind::Internal
            | SyntaxKind::Fileprivate
            | SyntaxKind::At => continue,
            // Anything else is the declaration's own content; stop.
            _ => break,
        }
    }
    let docs = chunks.join("\n").trim().to_string();
    if !docs.is_empty() {
        world.set(entity, Documentation(docs));
    }
}

/// `///` (but not `////` which is a section divider).
fn is_doc_line(text: &str) -> bool {
    let bytes = text.as_bytes();
    if !bytes.starts_with(b"///") {
        return false;
    }
    bytes.len() == 3 || bytes[3] != b'/'
}

/// `/** … */` (but not `/*** … */` which is decorative).
fn is_doc_block(text: &str) -> bool {
    text.starts_with("/**") && !text.starts_with("/***")
}

fn strip_doc_line(text: &str) -> String {
    let t = text.trim_end_matches(['\n', '\r']);
    let body = t.strip_prefix("///").unwrap_or(t);
    body.strip_prefix(' ').unwrap_or(body).to_string()
}

fn strip_doc_block(text: &str) -> String {
    let t = text
        .strip_prefix("/**")
        .and_then(|s| s.strip_suffix("*/"))
        .unwrap_or(text);
    // Drop a leading `*` from each line (and an optional space after it),
    // matching the canonical block-comment doc style.
    let mut out = String::with_capacity(t.len());
    for (i, line) in t.lines().enumerate() {
        let trimmed = line.trim_start();
        let body = trimmed
            .strip_prefix('*')
            .map(|s| s.strip_prefix(' ').unwrap_or(s))
            .unwrap_or(line);
        if i > 0 {
            out.push('\n');
        }
        out.push_str(body.trim_end());
    }
    out.trim().to_string()
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
                    // ConformanceItem wraps either a direct type (positive) or
                    // a nested `NegativeConformance > <type>` (for `not Proto`).
                    if let Some(neg) = child
                        .children()
                        .find(|c| c.kind() == SyntaxKind::NegativeConformance)
                    {
                        let ty = neg
                            .children()
                            .find(|c| is_type_kind(c.kind()))
                            .and_then(|c| ast_type_from_cst(&c, file_id))?;
                        Some(ConformanceItem::Negative(ty, child))
                    } else {
                        let ty = child
                            .children()
                            .find(|c| is_type_kind(c.kind()))
                            .and_then(|c| ast_type_from_cst(&c, file_id))?;
                        Some(ConformanceItem::Positive(ty, child))
                    }
                },
                SyntaxKind::NegativeConformance => {
                    // Legacy/alternate shape where NegativeConformance is a
                    // direct child of ConformanceList.
                    let ty = child
                        .children()
                        .find(|c| is_type_kind(c.kind()))
                        .and_then(|c| ast_type_from_cst(&c, file_id))?;
                    Some(ConformanceItem::Negative(ty, child))
                },
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

                    // Negative bound: `T: not Proto` emits
                    //   TypeBound > { Name, NegativeConformance > Path }
                    // — the Path lives inside NegativeConformance, not as a
                    // direct child of TypeBound, so handle that shape first.
                    if let Some(neg) = child
                        .children()
                        .find(|c| c.kind() == SyntaxKind::NegativeConformance)
                    {
                        let path_node = neg.children().find(|c| c.kind() == SyntaxKind::Path)?;
                        let mut ty = path_to_ast_type(&path_node, file_id)?;
                        if let Some(args_node) = neg
                            .children()
                            .find(|c| c.kind() == SyntaxKind::TypeArgumentList)
                            && let AstType::Named {
                                ref mut segments, ..
                            } = ty
                        {
                            let type_args: Vec<AstType> = args_node
                                .children()
                                .filter(|c| crate::ast_type::is_type_node(c.kind()))
                                .filter_map(|c| crate::ast_type::ast_type_from_cst(&c, file_id))
                                .collect();
                            if let Some(last) = segments.last_mut() {
                                last.type_args = type_args;
                            }
                        }
                        return Some(WhereConstraint::NegativeBound {
                            subject,
                            protocol: ty,
                            node: child,
                        });
                    }

                    // Positive bound: protocols come from Path children, with
                    // TypeArgumentList siblings carrying generic args.
                    let protocols: Vec<_> = {
                        let children: Vec<_> = child.children().collect();
                        let mut protos = Vec::new();
                        let mut i = 0;
                        while i < children.len() {
                            if children[i].kind() == SyntaxKind::Path {
                                let mut ty = path_to_ast_type(&children[i], file_id);
                                // Check for TypeArgumentList following the Path
                                if i + 1 < children.len()
                                    && children[i + 1].kind() == SyntaxKind::TypeArgumentList
                                {
                                    if let Some(AstType::Named {
                                        ref mut segments, ..
                                    }) = ty
                                    {
                                        let type_args: Vec<AstType> = children[i + 1]
                                            .children()
                                            .filter(|c| crate::ast_type::is_type_node(c.kind()))
                                            .filter_map(|c| {
                                                crate::ast_type::ast_type_from_cst(&c, file_id)
                                            })
                                            .collect();
                                        if let Some(last) = segments.last_mut() {
                                            last.type_args = type_args;
                                        }
                                    }
                                    i += 1; // skip the TypeArgumentList
                                }
                                if let Some(t) = ty {
                                    protos.push(t);
                                }
                            }
                            i += 1;
                        }
                        protos
                    };

                    if protocols.is_empty() {
                        return None;
                    }

                    Some(WhereConstraint::Bound {
                        subject,
                        protocols,
                        node: child,
                    })
                },
                SyntaxKind::TypeEquality => {
                    // Type equality uses Ty nodes, Name/Path, or AssociatedTypeTarget (wraps a Path)
                    let mut type_children = child.children().filter(|c| {
                        is_type_kind(c.kind())
                            || c.kind() == SyntaxKind::Name
                            || c.kind() == SyntaxKind::Path
                            || c.kind() == SyntaxKind::AssociatedTypeTarget
                    });

                    let lhs_node = type_children.next()?;
                    let lhs = node_to_ast_type(&lhs_node, file_id)?;
                    let rhs_node = type_children.next()?;
                    let rhs = node_to_ast_type(&rhs_node, file_id)?;

                    Some(WhereConstraint::Equality {
                        lhs,
                        rhs,
                        node: child,
                    })
                },
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
    if let Some(assoc) = find_child(parent, SyntaxKind::AssociatedTypeTarget)
        && let Some(path) = find_child(&assoc, SyntaxKind::Path)
    {
        return path_to_ast_type(&path, file_id);
    }
    // Fall back to simple Name (T)
    name_to_ast_type(parent, file_id)
}

/// Convert a Name node to AstType::Named.
fn name_to_ast_type(parent: &SyntaxNode, file_id: usize) -> Option<AstType> {
    let name_node = find_child(parent, SyntaxKind::Name)?;
    let ident = kestrel_syntax_tree::utils::extract_identifier_from_name(&name_node)?;
    let range = name_node.text_range();
    let span = Span::new(file_id, (range.start().into())..(range.end().into()));
    Some(AstType::Named {
        segments: vec![PathSegment {
            name: ident,
            type_args: vec![],
            span: span.clone(),
        }],
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

    // Extract type arguments (e.g., Factory[lang.i64] → [lang.i64])
    // Type args appear as a TypeArgumentList child of the path node
    let type_args: Vec<AstType> = find_child(path_node, SyntaxKind::TypeArgumentList)
        .map(|args_node| {
            args_node
                .children()
                .filter(|c| crate::ast_type::is_type_node(c.kind()))
                .filter_map(|c| crate::ast_type::ast_type_from_cst(&c, file_id))
                .collect()
        })
        .unwrap_or_default();

    // Type args go on the last segment
    let segments = names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let seg_args = if i == 0 && !type_args.is_empty() {
                // For single-segment paths, args go on the only segment
                type_args.clone()
            } else {
                vec![]
            };
            PathSegment {
                name,
                type_args: seg_args,
                span: span.clone(),
            }
        })
        .collect::<Vec<_>>();
    // If multi-segment, put type args on last segment
    let mut segments = segments;
    if segments.len() > 1
        && !type_args.is_empty()
        && let Some(last) = segments.last_mut()
    {
        last.type_args = type_args;
    }
    Some(AstType::Named { segments, span })
}

/// Convert various node kinds to AstType.
fn node_to_ast_type(node: &SyntaxNode, file_id: usize) -> Option<AstType> {
    match node.kind() {
        SyntaxKind::Name => {
            let ident = kestrel_syntax_tree::utils::extract_identifier_from_name(node)?;
            let range = node.text_range();
            let span = Span::new(file_id, (range.start().into())..(range.end().into()));
            Some(AstType::Named {
                segments: vec![PathSegment {
                    name: ident,
                    type_args: vec![],
                    span: span.clone(),
                }],
                span,
            })
        },
        SyntaxKind::Path => path_to_ast_type(node, file_id),
        // AssociatedTypeTarget wraps a Path (e.g., Item.Output in where clauses)
        SyntaxKind::AssociatedTypeTarget => {
            find_child(node, SyntaxKind::Path).and_then(|p| path_to_ast_type(&p, file_id))
        },
        _ if is_type_kind(node.kind()) => ast_type_from_cst(node, file_id),
        _ => None,
    }
}

/// Check if a SyntaxKind is a type-related node.
pub fn is_type_kind(kind: SyntaxKind) -> bool {
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
            | SyntaxKind::TySome
    )
}

/// Check if a declaration node has a StaticModifier child.
pub fn has_static_modifier(node: &SyntaxNode) -> bool {
    find_child(node, SyntaxKind::StaticModifier).is_some()
}

/// Spawn a `NodeKind::Setter` child entity under a Field or Subscript.
///
/// Caller supplies the full params list (for Field: `[newValue]`; for Subscript:
/// `[index_params..., newValue]`) and the receiver kind. No `Name`, `Vis`, or
/// `TypeAnnotation` component is set — setters are discovered by `NodeKind::
/// Setter` via `children_of(parent)`, access control flows through the parent
/// declaration's `Settable`, and setters return unit (no explicit return type).
pub fn spawn_setter(
    world: &mut World,
    parent: Entity,
    setter_clause: &SyntaxNode,
    setter_body: &SyntaxNode,
    params: Vec<AstParam>,
    receiver: Option<ReceiverKind>,
    file_entity: Entity,
    file_id: usize,
    is_static: bool,
) {
    let setter = world.spawn();
    world.set(setter, NodeKind::Setter);
    world.set(setter, FileId(file_entity));
    world.set(setter, DeclSpan(get_decl_span(setter_clause, file_id)));
    world.set(setter, CstNode(setter_clause.clone()));
    world.set_parent(setter, parent);
    // Setter → Subscript/Field → Container. Record the container so
    // downstream code can skip the intermediate hop without walking.
    if let Some(container) = world.parent_of(parent) {
        world.set(setter, EnclosingContainer(container));
    }
    world.set(setter, Callable { params, receiver });
    world.set(setter, Body(lower::lower_body(setter_body, file_id)));
    world.set(setter, Valued(setter_body.clone()));
    if is_static {
        world.set(setter, Static);
    }
    set_documentation(world, setter, setter_clause);
}
