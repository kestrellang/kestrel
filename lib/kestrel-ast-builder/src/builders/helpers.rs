//! Shared extraction helpers for building declaration entities.
//!
//! Read visibility, attributes, documentation, conformances, and where
//! clauses through the typed CST views (`kestrel_syntax_tree::ast`), so a
//! declaration's shape is named once, in `kestrel.ungram`.

use kestrel_hecs::{Entity, World};
use kestrel_span::Span;
use kestrel_syntax_tree::ast::{
    self, AstNode, HasAttributes, HasConformances, HasGenerics, HasVisibility, VisibilityKind,
};
use kestrel_syntax_tree::utils::get_decl_span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::ast_type::{AstType, PathSegment, lower_opt_type, lower_types};
use crate::components::*;
use crate::lower;

/// Set the `Vis` component from the declaration's visibility keyword.
pub fn set_visibility(world: &mut World, entity: Entity, node: &impl HasVisibility) {
    let Some(kind) = node.visibility().and_then(|v| v.kind()) else {
        return;
    };
    let vis = match kind {
        VisibilityKind::Public => Vis::Public,
        VisibilityKind::Private => Vis::Private,
        VisibilityKind::Internal => Vis::Internal,
        VisibilityKind::Fileprivate => Vis::Fileprivate,
    };
    world.set(entity, vis);
}

/// Set the `Attributes` component from the declaration's `@attributes`.
pub fn set_attributes(
    world: &mut World,
    entity: Entity,
    node: &impl HasAttributes,
    file_id: usize,
) {
    let attrs: Vec<AstAttribute> = node
        .attributes()
        .filter_map(|a| extract_attribute(&a, file_id))
        .collect();
    if !attrs.is_empty() {
        world.set(entity, Attributes(attrs));
    }
}

/// Extract a single attribute.
fn extract_attribute(attr: &ast::Attribute, file_id: usize) -> Option<AstAttribute> {
    let name_token = attr.identifier_token()?;
    let args = attr
        .attribute_args()
        .map(|args| {
            args.attribute_args()
                .filter_map(|a| extract_attribute_arg(&a))
                .collect()
        })
        .unwrap_or_default();
    // Span the name token: the node's range starts at its leading trivia
    // (the previous line's newline), which would put diagnostics a line early.
    let range = name_token.text_range();
    let span = Span::new(file_id, (range.start().into())..(range.end().into()));
    Some(AstAttribute {
        name: name_token.text().to_string(),
        args,
        span,
    })
}

/// Extract a single attribute argument: `label: value` or `value`.
fn extract_attribute_arg(arg: &ast::AttributeArg) -> Option<AstAttributeArg> {
    let tokens: Vec<_> = arg
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .collect();
    match tokens.iter().position(|t| t.kind() == SyntaxKind::Colon) {
        Some(pos) => Some(AstAttributeArg {
            label: pos.checked_sub(1).map(|i| tokens[i].text().to_string()),
            value: extract_value_from_tokens(&tokens[(pos + 1)..]).unwrap_or_default(),
        }),
        None => Some(AstAttributeArg {
            label: None,
            value: extract_value_from_tokens(&tokens)?,
        }),
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

/// Set the `Conformances` component from `: P, not Q`.
pub fn set_conformances(
    world: &mut World,
    entity: Entity,
    node: &impl HasConformances,
    file_id: usize,
) {
    let Some(list) = node.conformance_list() else {
        return;
    };
    let items: Vec<ConformanceItem> = list
        .conformance_items()
        .filter_map(|item| {
            let syntax = item.syntax().clone();
            match item.negative_conformance() {
                Some(neg) => Some(ConformanceItem::Negative(
                    lower_opt_type(neg.ty(), file_id)?,
                    syntax,
                )),
                None => Some(ConformanceItem::Positive(
                    lower_opt_type(item.ty(), file_id)?,
                    syntax,
                )),
            }
        })
        .collect();
    if !items.is_empty() {
        world.set(entity, Conformances(items));
    }
}

/// Set the `WhereClause` component from `where …`.
pub fn set_where_clause(
    world: &mut World,
    entity: Entity,
    node: &impl HasGenerics,
    file_id: usize,
) {
    let Some(clause) = node.where_clause() else {
        return;
    };
    let constraints: Vec<WhereConstraint> = clause
        .where_constraints()
        .filter_map(|c| match c {
            ast::WhereConstraint::TypeBound(b) => type_bound(&b, file_id),
            ast::WhereConstraint::TypeEquality(e) => Some(WhereConstraint::Equality {
                lhs: assoc_target_to_ast_type(&e.associated_type_target()?, file_id)?,
                rhs: lower_opt_type(e.ty(), file_id)?,
                node: e.syntax().clone(),
            }),
        })
        .collect();
    if !constraints.is_empty() {
        world.set(entity, WhereClause(constraints));
    }
}

/// `T: P and Q[A]` or `T: not P`. The subject is a `Name` (`T`) or an
/// `AssociatedTypeTarget` path (`T.Item`); each bound is a `Path` with an
/// optional `TypeArgumentList` after it, applied to its last segment.
fn type_bound(bound: &ast::TypeBound, file_id: usize) -> Option<WhereConstraint> {
    let subject = match bound.associated_type_target() {
        Some(target) => assoc_target_to_ast_type(&target, file_id)?,
        None => name_to_ast_type(&bound.name()?, file_id)?,
    };
    let node = bound.syntax().clone();
    if let Some(neg) = bound.negative_conformance() {
        let protocol = path_with_args(&neg.path()?, neg.type_argument_list(), file_id)?;
        return Some(WhereConstraint::NegativeBound {
            subject,
            protocol,
            node,
        });
    }
    // Pair each Path with the TypeArgumentList right after it, if any.
    let mut protocols = Vec::new();
    let mut children = bound.syntax().children().peekable();
    while let Some(child) = children.next() {
        let Some(path) = ast::Path::cast(child) else {
            continue;
        };
        let args = children
            .next_if(|c| c.kind() == SyntaxKind::TypeArgumentList)
            .and_then(ast::TypeArgumentList::cast);
        protocols.extend(path_with_args(&path, args, file_id));
    }
    if protocols.is_empty() {
        return None;
    }
    Some(WhereConstraint::Bound {
        subject,
        protocols,
        node,
    })
}

/// A path type whose last segment takes `args`.
fn path_with_args(
    path: &ast::Path,
    args: Option<ast::TypeArgumentList>,
    file_id: usize,
) -> Option<AstType> {
    let mut ty = path_to_ast_type(path, file_id)?;
    if let Some(args) = args
        && let AstType::Named { segments, .. } = &mut ty
        && let Some(last) = segments.last_mut()
    {
        last.type_args = lower_types(args.types(), file_id);
    }
    Some(ty)
}

/// The path of a where-clause `AssociatedTypeTarget` (`T.Item`).
fn assoc_target_to_ast_type(target: &ast::AssociatedTypeTarget, file_id: usize) -> Option<AstType> {
    path_to_ast_type(&target.path()?, file_id)
}

/// A one-segment named type for a `Name`.
fn name_to_ast_type(name: &ast::Name, file_id: usize) -> Option<AstType> {
    let ident = name.text()?;
    let range = name.syntax().text_range();
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

/// A named type for a `Path` (no type arguments: those follow the path).
fn path_to_ast_type(path: &ast::Path, file_id: usize) -> Option<AstType> {
    let names = path.segments();
    if names.is_empty() {
        return None;
    }
    // Start at the first identifier rather than the node, whose range begins
    // at leading trivia (for a clause on its own line, the line before it).
    let range = path.syntax().text_range();
    let start = path
        .segment_tokens()
        .next()
        .map_or(range.start(), |t| t.text_range().start());
    let span = Span::new(file_id, (start.into())..(range.end().into()));
    let segments = names
        .into_iter()
        .map(|name| PathSegment {
            name,
            type_args: vec![],
            span: span.clone(),
        })
        .collect();
    Some(AstType::Named { segments, span })
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

/// Spawn a `NodeKind::RefAccessor` child entity under a Field or Subscript
/// for a `ref { … }` / `mutating ref { … }` place-accessor clause.
///
/// Mirrors `spawn_setter` (discovered by NodeKind via `children_of(parent)`,
/// access control flows through the parent), with one difference: the
/// accessor has a RETURN type — the SYNTHESIZED `&T` / `&mutating T`
/// wrapping the parent's declared type. The user writes `-> T`; a ref type
/// never appears in source (declared `-> &T` subscripts stay E481). The
/// `MutatingAccessor` marker distinguishes the two kinds.
pub fn spawn_ref_accessor(
    world: &mut World,
    parent: Entity,
    clause: &SyntaxNode,
    clause_body: &SyntaxNode,
    params: Vec<AstParam>,
    receiver: Option<ReceiverKind>,
    mutating: bool,
    file_entity: Entity,
    file_id: usize,
    is_static: bool,
) {
    let acc = world.spawn();
    world.set(acc, NodeKind::RefAccessor);
    world.set(acc, FileId(file_entity));
    world.set(acc, DeclSpan(get_decl_span(clause, file_id)));
    world.set(acc, CstNode(clause.clone()));
    world.set_parent(acc, parent);
    // RefAccessor → Subscript/Field → Container (same hop-skip as Setter).
    if let Some(container) = world.parent_of(parent) {
        world.set(acc, EnclosingContainer(container));
    }
    world.set(acc, Callable { params, receiver });
    world.set(acc, Body(lower::lower_body(clause_body, file_id)));
    world.set(acc, Valued(clause_body.clone()));
    if mutating {
        world.set(acc, MutatingAccessor);
    }
    // Synthesized ref return around the parent's declared type. Spanned at
    // the clause so any ref-placement diagnostic lands on the accessor.
    if let Some(parent_ty) = world.get::<TypeAnnotation>(parent).map(|t| t.0.clone()) {
        world.set(
            acc,
            TypeAnnotation(AstType::Ref {
                inner: Box::new(parent_ty),
                mutating,
                span: get_decl_span(clause, file_id),
            }),
        );
    }
    if is_static {
        world.set(acc, Static);
    }
    set_documentation(world, acc, clause);
}
