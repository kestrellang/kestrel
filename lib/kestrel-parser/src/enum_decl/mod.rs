//! Enum declaration parsing
//!
//! This module is the single source of truth for enum declaration parsing.
//! Enum bodies contain cases with optional associated values.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{
    EnumCaseData, EnumCaseParameterData, EnumDeclarationData, emit_enum_declaration, identifier,
    token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::ty::ty_parser;
use crate::type_param::{type_parameter_list_parser, where_clause_parser};

/// Represents an enum declaration: (visibility)? (indirect)? enum Name[T]? (where ...)? { ... }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl EnumDeclaration {
    /// Create a new EnumDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the enum name from this declaration
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

    /// Check if this enum has the indirect modifier
    pub fn is_indirect(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::IndirectModifier)
    }

    /// Get child case declarations
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::EnumBody)
            .map(|body| {
                body.children()
                    .filter(|child| child.kind() == SyntaxKind::EnumCaseDeclaration)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Internal parser for enum case parameter (associated value)
///
/// Parses: label: Type
fn enum_case_parameter_parser_internal(
) -> impl Parser<Token, EnumCaseParameterData, Error = Simple<Token>> + Clone {
    identifier()
        .then(token(Token::Colon))
        .then(ty_parser())
        .map(|((label, colon), ty)| EnumCaseParameterData { label, colon, ty })
}

/// Internal parser for enum case declaration
///
/// Parses: case Name or case Name(label: Type, ...)
fn enum_case_parser_internal(
) -> impl Parser<Token, EnumCaseData, Error = Simple<Token>> + Clone {
    token(Token::Case)
        .then(identifier())
        .then(
            token(Token::LParen)
                .then(
                    enum_case_parameter_parser_internal()
                        .separated_by(token(Token::Comma))
                        .allow_trailing(),
                )
                .then(token(Token::RParen))
                .map(|((lparen, params), rparen)| (lparen, params, rparen))
                .or_not(),
        )
        .map(|((case_span, name_span), parameters)| EnumCaseData {
            case_span,
            name_span,
            parameters,
        })
}

/// Internal Chumsky parser for enum declaration
///
/// This is the single source of truth for enum declaration parsing.
pub fn enum_declaration_parser_internal(
) -> impl Parser<Token, EnumDeclarationData, Error = Simple<Token>> + Clone {
    // Check for optional "indirect" identifier modifier
    let indirect_parser = identifier()
        .try_map(|span, _| {
            // We need to check if the identifier text is "indirect"
            // For now, we'll accept any identifier and validate later
            // A proper implementation would check the source text
            Ok(span)
        })
        .or_not();

    visibility_parser_internal()
        .then(indirect_parser)
        .then(token(Token::Enum))
        .then(identifier())
        .then(type_parameter_list_parser().or_not())
        .then(where_clause_parser().or_not())
        .then(token(Token::LBrace))
        .then(enum_case_parser_internal().repeated())
        .then(token(Token::RBrace))
        .map(
            |(
                (
                    (
                        (
                            (
                                (((visibility, is_indirect), enum_span), name_span),
                                type_params,
                            ),
                            where_clause,
                        ),
                        lbrace_span,
                    ),
                    cases,
                ),
                rbrace_span,
            )| {
                EnumDeclarationData {
                    visibility,
                    is_indirect,
                    enum_span,
                    name_span,
                    type_params,
                    where_clause,
                    lbrace_span,
                    cases,
                    rbrace_span,
                }
            },
        )
}

/// Parse an enum declaration and emit events
///
/// This is the primary event-driven parser function for enum declarations.
pub fn parse_enum_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let end_pos = source.len();
    let tokens_with_range = tokens.map(|(tok, span)| (tok, span.range()));
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens_with_range);

    match enum_declaration_parser_internal().parse(stream) {
        Ok(data) => {
            emit_enum_declaration(sink, data);
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

    /// Helper to parse source code and return an EnumDeclaration
    fn parse(source: &str) -> EnumDeclaration {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new();
        parse_enum_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        EnumDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    /// Helper to check if a syntax node exists as a child
    fn has_child(decl: &EnumDeclaration, kind: SyntaxKind) -> bool {
        decl.syntax.children().any(|child| child.kind() == kind)
    }

    #[test]
    fn test_enum_declaration_basic() {
        let decl = parse("enum Color { }");
        assert_eq!(decl.name(), Some("Color".to_string()));
        assert_eq!(decl.visibility(), None);
        assert_eq!(decl.syntax.kind(), SyntaxKind::EnumDeclaration);
    }

    #[test]
    fn test_enum_declaration_with_visibility() {
        let decl = parse("public enum Status { }");
        assert_eq!(decl.name(), Some("Status".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_enum_declaration_with_cases() {
        let decl = parse("enum Color { case Red case Green case Blue }");
        assert_eq!(decl.name(), Some("Color".to_string()));
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert!(children
            .iter()
            .all(|c| c.kind() == SyntaxKind::EnumCaseDeclaration));
    }

    #[test]
    fn test_enum_declaration_with_type_params() {
        let decl = parse("enum Optional[T] { }");
        assert_eq!(decl.name(), Some("Optional".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
    }

    #[test]
    fn test_enum_declaration_with_where_clause() {
        let decl = parse("enum Container[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Container".to_string()));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }
}
