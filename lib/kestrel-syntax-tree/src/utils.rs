//! Span helpers over untyped nodes, for callers that hold a declaration's
//! `SyntaxNode` without knowing its kind. Shape-specific reads go through
//! the typed views in [`crate::ast`].

use kestrel_span::Span;

use crate::ast::{self, AstNode};
use crate::{SyntaxElement, SyntaxKind, SyntaxNode};

/// The declaration span of a node: from its first non-trivia token outside
/// a leading `AttributeList`, so diagnostics point at the `func`/`struct`/…
/// keyword rather than at an `@attribute`.
pub fn get_decl_span(node: &SyntaxNode, file_id: usize) -> Span {
    let text_range = node.text_range();
    let end: usize = text_range.end().into();
    let start = node
        .children_with_tokens()
        .find_map(|child| match child {
            SyntaxElement::Token(t) if !t.kind().is_trivia() && t.kind() != SyntaxKind::Error => {
                Some(t.text_range().start().into())
            },
            SyntaxElement::Node(n) if n.kind() != SyntaxKind::AttributeList => {
                first_non_trivia_start(&n)
            },
            _ => None,
        })
        .unwrap_or_else(|| text_range.start().into());
    Span::new(file_id, start..end)
}

/// Span of a declaration's identifier token (`foo` in `func foo(...)`).
///
/// Returns `None` for declarations without a `Name` child (e.g. `Module`,
/// anonymous initializers) or when the name is missing — callers can fall
/// back to [`get_decl_span`] for those cases.
///
/// Used by the LSP for `textDocument/rename` (to compute the edit range) and
/// `documentSymbol.selectionRange` (to highlight just the name when an
/// outline item is selected).
pub fn get_name_span(node: &SyntaxNode, file_id: usize) -> Option<Span> {
    let name = node.children().find_map(ast::Name::cast)?;
    let range = name.identifier_token()?.text_range();
    Some(Span::new(file_id, range.start().into()..range.end().into()))
}

fn first_non_trivia_start(node: &SyntaxNode) -> Option<usize> {
    node.children_with_tokens().find_map(|child| match child {
        SyntaxElement::Token(t) if !t.kind().is_trivia() && t.kind() != SyntaxKind::Error => {
            Some(t.text_range().start().into())
        },
        SyntaxElement::Token(_) => None,
        SyntaxElement::Node(n) => first_non_trivia_start(&n),
    })
}

/// The first direct child node of `kind`.
pub fn find_child(syntax: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    syntax.children().find(|n| n.kind() == kind)
}

/// The span of a node from its first non-trivia token.
pub fn get_node_span(node: &SyntaxNode, file_id: usize) -> Span {
    let range = node.text_range();
    let start = first_non_trivia_start(node).unwrap_or_else(|| range.start().into());
    Span::new(file_id, start..range.end().into())
}
