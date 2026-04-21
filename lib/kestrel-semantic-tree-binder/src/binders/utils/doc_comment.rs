//! Doc comment extraction utility for the binder.

use kestrel_semantic_tree::behavior::doc_comment::DocCommentBehavior;
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind, SyntaxNode};

/// Extract doc comments from the trivia preceding a declaration node.
///
/// Looks for `///` line comments (but not `////`) and `/** */` block comments
/// (but not `/***`) in the leading trivia of the syntax node. Multiple consecutive
/// `///` lines are joined with newlines. Returns `None` if no doc comment is found.
pub fn extract_doc_comment(syntax: &SyntaxNode) -> Option<DocCommentBehavior> {
    let mut doc_lines: Vec<String> = Vec::new();

    for child in syntax.children_with_tokens() {
        match child {
            SyntaxElement::Token(tok) => match tok.kind() {
                SyntaxKind::LineComment => {
                    let text = tok.text();
                    if text.starts_with("///") && !text.starts_with("////") {
                        let content = text.strip_prefix("/// ").unwrap_or_else(|| &text[3..]);
                        doc_lines.push(content.to_string());
                    }
                },
                SyntaxKind::BlockComment => {
                    let text = tok.text();
                    if text.starts_with("/**") && !text.starts_with("/***") {
                        let inner = &text[3..];
                        let inner = inner.strip_suffix("*/").unwrap_or(inner);
                        doc_lines.push(process_block_doc_comment(inner));
                    }
                },
                SyntaxKind::Whitespace => {
                    // Whitespace between doc comment lines — continue
                },
                _ => break,
            },
            SyntaxElement::Node(_) => break,
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(DocCommentBehavior::new(doc_lines.join("\n")))
    }
}

/// Process a block doc comment's inner text.
///
/// Strips optional leading `*` on each line and trims the result.
fn process_block_doc_comment(inner: &str) -> String {
    inner
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("* ") {
                rest.to_string()
            } else if trimmed == "*" {
                String::new()
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
