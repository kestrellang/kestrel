//! Helpers shared by the parser's integration tests.
#![allow(dead_code)]

use kestrel_parser::{ParseResult, parse_source_file_from_source};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

pub fn parse(source: &str) -> ParseResult {
    let tokens: Vec<_> = kestrel_lexer::lex(source, 0)
        .filter_map(|t| t.ok())
        .map(|t| (t.value, t.span))
        .collect();
    parse_source_file_from_source(source, tokens.into_iter())
}

/// Parse valid source: no errors, no `Error` element, exact round trip.
pub fn parse_ok(source: &str) -> SyntaxNode {
    let result = parse(source);
    assert!(
        result.errors.is_empty(),
        "unexpected errors {:?} in:\n{source}",
        result.errors
    );
    assert_eq!(
        result.tree.text().to_string(),
        source,
        "tree must round-trip"
    );
    let errors = result
        .tree
        .descendants_with_tokens()
        .filter(|e| e.kind() == SyntaxKind::Error)
        .count();
    assert_eq!(
        errors, 0,
        "valid source produced Error elements:\n{:#?}",
        result.tree
    );
    result.tree
}

/// Compact rendering of the non-trivia tree for shape assertions.
pub fn shape(node: &SyntaxNode) -> String {
    let mut out = String::new();
    render(node, &mut out);
    out
}

fn render(node: &SyntaxNode, out: &mut String) {
    out.push_str(&format!("{:?}(", node.kind()));
    let mut first = true;
    for child in node.children_with_tokens() {
        if child.kind().is_trivia() {
            continue;
        }
        if !first {
            out.push(' ');
        }
        first = false;
        match child {
            rowan::NodeOrToken::Node(n) => render(&n, out),
            rowan::NodeOrToken::Token(t) => out.push_str(t.text()),
        }
    }
    out.push(')');
}

pub fn first(node: &SyntaxNode, kind: SyntaxKind) -> SyntaxNode {
    node.descendants()
        .find(|n| n.kind() == kind)
        .unwrap_or_else(|| panic!("no {kind:?} in {node:#?}"))
}
