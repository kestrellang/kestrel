//! Extension declaration parsing
//!
//! This module is the single source of truth for extension declaration parsing.
//! Extensions add methods and protocol conformances to existing types.
//!
//! Syntax: `extend Type: Protocol { func method() { ... } }`

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{
    ConformanceListData, ExtensionBodyItem, ExtensionDeclarationData, emit_extension_declaration,
    function_declaration_parser_internal, initializer_declaration_parser_internal, token,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens, to_kestrel_span};
use crate::ty::ty_parser;
use crate::type_param::{conformance_list_parser, where_clause_parser};

/// Represents an extension declaration: extend Type: Protocol { ... }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl ExtensionDeclaration {
    /// Create a new ExtensionDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the target type name from this declaration (simplified - just first identifier)
    pub fn target_type_name(&self) -> Option<String> {
        let ty = self.syntax.children().find(|child| {
            matches!(
                child.kind(),
                SyntaxKind::Ty
                    | SyntaxKind::TyUnit
                    | SyntaxKind::TyNever
                    | SyntaxKind::TyTuple
                    | SyntaxKind::TyFunction
                    | SyntaxKind::TyPath
                    | SyntaxKind::TyArray
                    | SyntaxKind::TyList
                    | SyntaxKind::TyInferred
            )
        })?;

        ty.descendants_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string())
    }

    /// Get child declaration items (functions, initializers)
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ExtensionBody)
            .map(|body| {
                body.children()
                    .filter(|child| {
                        matches!(
                            child.kind(),
                            SyntaxKind::FunctionDeclaration | SyntaxKind::InitializerDeclaration
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Internal parser for extension body items
///
/// Extension bodies can contain: functions and initializers
fn extension_body_item_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExtensionBodyItem, ParserExtra<'tokens>> + Clone {
    let initializer_parser =
        initializer_declaration_parser_internal().map(ExtensionBodyItem::Initializer);

    let function_parser = function_declaration_parser_internal().map(ExtensionBodyItem::Function);

    initializer_parser.or(function_parser)
}

/// Internal Chumsky parser for extension declaration
///
/// This is the single source of truth for extension declaration parsing.
/// Syntax: extend Type: Protocol where ... { ... }
pub fn extension_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExtensionDeclarationData, ParserExtra<'tokens>> + Clone
{
    token(Token::Extend)
        .then(ty_parser())
        .then(conformance_list_parser().or_not())
        .then(where_clause_parser().or_not())
        .then(token(Token::LBrace))
        .then(extension_body_item_parser_internal().repeated().collect())
        .then(token(Token::RBrace))
        .map(
            |(
                (((((extend_span, target_type), conformances), where_clause), lbrace_span), body),
                rbrace_span,
            )| {
                ExtensionDeclarationData {
                    extend_span,
                    target_type,
                    conformances: conformances.map(|(colon_span, items)| ConformanceListData {
                        colon_span,
                        conformances: items,
                    }),
                    where_clause,
                    lbrace_span,
                    body,
                    rbrace_span,
                }
            },
        )
}

/// Parse an extension declaration and emit events
///
/// This is the primary event-driven parser function for extension declarations.
pub fn parse_extension_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::Parser;

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match extension_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_extension_declaration(sink, data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), to_kestrel_span(*span));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    /// Helper to parse source code and return an ExtensionDeclaration
    fn parse(source: &str) -> ExtensionDeclaration {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new();
        parse_extension_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        ExtensionDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    /// Helper to check if a syntax node exists as a child
    fn has_child(decl: &ExtensionDeclaration, kind: SyntaxKind) -> bool {
        decl.syntax.children().any(|child| child.kind() == kind)
    }

    #[test]
    fn test_extension_basic() {
        let decl = parse("extend Point { }");
        assert_eq!(decl.target_type_name(), Some("Point".to_string()));
        assert_eq!(decl.syntax.kind(), SyntaxKind::ExtensionDeclaration);
    }

    #[test]
    fn test_extension_with_conformance() {
        let decl = parse("extend Point: Drawable { }");
        assert_eq!(decl.target_type_name(), Some("Point".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_extension_with_function() {
        let decl = parse("extend Point { func describe() -> String { } }");
        let children = decl.children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_extension_generic_type() {
        let decl = parse("extend Box[T] { }");
        assert_eq!(decl.target_type_name(), Some("Box".to_string()));
    }

    #[test]
    fn test_extension_specialized() {
        let decl = parse("extend Box[Int] { }");
        assert_eq!(decl.target_type_name(), Some("Box".to_string()));
    }

    #[test]
    fn test_extension_mixed_type_params() {
        let decl = parse("extend Pair[T, Int] { }");
        assert_eq!(decl.target_type_name(), Some("Pair".to_string()));
    }

    #[test]
    fn test_extension_with_where_clause() {
        let decl = parse("extend Box[T] where T: Equatable { }");
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_extension_with_multiple_methods() {
        let decl = parse("extend Point { func sum() -> Int { } func product() -> Int { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::FunctionDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_extension_with_multiple_conformances() {
        let decl = parse("extend Point: Drawable, Hashable { }");
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
        let conformance_list = decl
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ConformanceList)
            .expect("Expected ConformanceList node");
        let conformance_count = conformance_list
            .children()
            .filter(|c| c.kind() == SyntaxKind::ConformanceItem)
            .count();
        assert_eq!(conformance_count, 2);
    }

    #[test]
    fn test_extension_full_syntax() {
        let decl = parse("extend Box[T]: Hashable where T: Equatable { func hash() -> Int { } }");
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
        let children = decl.children();
        assert_eq!(children.len(), 1);
    }
}
