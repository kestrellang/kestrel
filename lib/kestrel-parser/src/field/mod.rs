//! Field declaration parsing
//!
//! This module is the single source of truth for field declaration parsing.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{emit_field_declaration, field_declaration_parser_internal};
use crate::event::{EventSink, TreeBuilder};

/// Represents a field declaration: (visibility)? (static)? let/var name: Type
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl FieldDeclaration {
    /// Create a new FieldDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the field name from this declaration
    pub fn name(&self) -> Option<String> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Name)?
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string())
    }

    /// Get the visibility modifier if present
    pub fn visibility(&self) -> Option<SyntaxKind> {
        let visibility_node = self
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Visibility)?;

        visibility_node
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| {
                matches!(
                    tok.kind(),
                    SyntaxKind::Public
                        | SyntaxKind::Private
                        | SyntaxKind::Internal
                        | SyntaxKind::Fileprivate
                )
            })
            .map(|tok| tok.kind())
    }

    /// Check if this field has the static modifier
    pub fn is_static(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier)
    }

    /// Check if this field is mutable (var vs let)
    pub fn is_mutable(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Var)
    }

    /// Get the type expression
    pub fn ty(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Ty)
    }
}

/// Parse a field declaration and emit events
///
/// This is the primary event-driven parser function for field declarations.
pub fn parse_field_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::Parser;

    let end_pos = source.len();
    let tokens_with_range = tokens.map(|(tok, span)| (tok, span.range()));
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens_with_range);

    match field_declaration_parser_internal().parse(stream) {
        Ok(data) => {
            emit_field_declaration(sink, data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), Span::from(span));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_field_declaration_basic() {
        let source = "let x: Int";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new();
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.name(), Some("x".to_string()));
        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_static());
        assert!(!decl.is_mutable());
    }

    #[test]
    fn test_field_declaration_var() {
        let source = "var count: Int";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new();
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.name(), Some("count".to_string()));
        assert!(decl.is_mutable());
    }

    #[test]
    fn test_field_declaration_static() {
        let source = "static let instance: Self";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new();
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.name(), Some("instance".to_string()));
        assert!(decl.is_static());
        assert!(!decl.is_mutable());
    }

    #[test]
    fn test_field_declaration_with_visibility() {
        let source = "public let name: String";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new();
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.name(), Some("name".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_field_declaration_full() {
        let source = "public static var counter: Int";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new();
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };

        assert_eq!(decl.name(), Some("counter".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_static());
        assert!(decl.is_mutable());
    }
}
