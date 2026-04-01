use kestrel_span2::Span;

use crate::{SyntaxElement, SyntaxKind, SyntaxNode};

/// Find a direct child node with the specified kind.
pub fn find_child(syntax: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    syntax.children().find(|n| n.kind() == kind)
}

/// Extract name from a `Name` node.
pub fn extract_name(syntax: &SyntaxNode) -> Option<String> {
    let name_node = find_child(syntax, SyntaxKind::Name)?;

    name_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|tok| tok.kind() == SyntaxKind::Identifier)
        .map(|tok| tok.text().to_string())
}

/// Extract identifier text from a `Name` syntax node.
pub fn extract_identifier_from_name(name_node: &SyntaxNode) -> Option<String> {
    name_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|tok| tok.kind() == SyntaxKind::Identifier)
        .map(|tok| tok.text().to_string())
}

/// Check if a `SyntaxKind` is trivia (whitespace or comment).
pub fn is_trivia(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Whitespace | SyntaxKind::LineComment | SyntaxKind::BlockComment
    )
}

/// Extract visibility modifier from a node with a `Visibility` child.
///
/// Returns the keyword as a string (`public`, `private`, `internal`, `fileprivate`).
pub fn extract_visibility(syntax: &SyntaxNode) -> Option<String> {
    let visibility_node = find_child(syntax, SyntaxKind::Visibility)?;

    let visibility_token = visibility_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|tok| !is_trivia(tok.kind()))?;

    let vis_text = match visibility_token.kind() {
        SyntaxKind::Public => "public",
        SyntaxKind::Private => "private",
        SyntaxKind::Internal => "internal",
        SyntaxKind::Fileprivate => "fileprivate",
        _ => return None,
    };

    Some(vis_text.to_string())
}

/// Get the span of a syntax node, excluding leading trivia.
pub fn get_node_span(node: &SyntaxNode, file_id: usize) -> Span {
    let text_range = node.text_range();
    let end: usize = text_range.end().into();

    let start = find_first_non_trivia_start(node).unwrap_or_else(|| text_range.start().into());

    Span::new(file_id, start..end)
}

/// Get the declaration span of a syntax node, excluding leading attributes and trivia.
/// Use this for DeclSpan so diagnostics point at the `func`/`struct`/etc keyword
/// rather than at a leading `@attribute`.
pub fn get_decl_span(node: &SyntaxNode, file_id: usize) -> Span {
    let text_range = node.text_range();
    let end: usize = text_range.end().into();

    // Find the first non-trivia, non-attribute child
    let start = node.children_with_tokens()
        .find_map(|child| {
            match child {
                SyntaxElement::Token(t) if !is_trivia(t.kind()) => {
                    Some(t.text_range().start().into())
                }
                SyntaxElement::Node(n) if n.kind() != SyntaxKind::AttributeList => {
                    find_first_non_trivia_start(&n)
                }
                _ => None,
            }
        })
        .unwrap_or_else(|| text_range.start().into());

    Span::new(file_id, start..end)
}

/// Get the span of the visibility node.
pub fn get_visibility_span(syntax: &SyntaxNode, file_id: usize) -> Option<Span> {
    let visibility_node = find_child(syntax, SyntaxKind::Visibility)?;
    Some(get_node_span(&visibility_node, file_id))
}

/// Extract path segments from a `Path` syntax node.
pub fn extract_path_segments(path_node: &SyntaxNode) -> Vec<String> {
    path_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::PathElement)
        .filter_map(|path_elem| {
            path_elem
                .children_with_tokens()
                .filter_map(|elem| elem.into_token())
                .find(|tok| tok.kind() == SyntaxKind::Identifier)
                .map(|tok| tok.text().to_string())
        })
        .collect()
}

fn find_first_non_trivia_start(node: &SyntaxNode) -> Option<usize> {
    for child in node.children_with_tokens() {
        match child {
            SyntaxElement::Token(t) => {
                if !is_trivia(t.kind()) {
                    return Some(t.text_range().start().into());
                }
            },
            SyntaxElement::Node(n) => {
                if let Some(start) = find_first_non_trivia_start(&n) {
                    return Some(start);
                }
            },
        }
    }
    None
}
