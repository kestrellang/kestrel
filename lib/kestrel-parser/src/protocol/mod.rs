//! Protocol declaration parsing
//!
//! This module is the single source of truth for protocol declaration parsing.
//! Protocol bodies can contain:
//! - Function declarations (methods)
//! - Associated type declarations
//! - Initializer declarations

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::common::ConformanceListData;
use crate::common::{
    ProtocolBodyItem, ProtocolDeclarationData, emit_protocol_declaration,
    function_declaration_parser_internal, identifier, initializer_declaration_parser_internal,
    token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens};
use crate::type_alias::type_alias_declaration_parser_internal;
use crate::type_param::{conformance_list_parser, type_parameter_list_parser, where_clause_parser};

/// Represents a protocol declaration: (visibility)? protocol Name[T]? (where ...)? { ... }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl ProtocolDeclaration {
    /// Create a new ProtocolDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the protocol name from this declaration
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

    /// Get the protocol body node
    pub fn body(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ProtocolBody)
    }

    /// Get child function declarations (protocol methods)
    pub fn methods(&self) -> Vec<SyntaxNode> {
        self.body()
            .map(|body| {
                body.children()
                    .filter(|child| child.kind() == SyntaxKind::FunctionDeclaration)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Parser for protocol body items (functions, associated types, or initializers)
fn protocol_body_item_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ProtocolBodyItem, ParserExtra<'tokens>> + Clone {
    let function = function_declaration_parser_internal().map(ProtocolBodyItem::Function);

    let associated_type =
        type_alias_declaration_parser_internal().map(ProtocolBodyItem::AssociatedType);

    let initializer = initializer_declaration_parser_internal().map(ProtocolBodyItem::Initializer);

    // Try function first, then associated type, then initializer
    // This works because:
    // - function starts with visibility? followed by (static)? (mutating/consuming)? 'func'
    // - associated type starts with visibility? followed by 'type'
    // - initializer starts with visibility? followed by 'init'
    // Chumsky will backtrack correctly when the keyword doesn't match
    function.or(associated_type).or(initializer)
}

/// Internal Chumsky parser for protocol declaration
///
/// This is the single source of truth for protocol declaration parsing.
pub fn protocol_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ProtocolDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(token(Token::Protocol))
        .then(identifier())
        .then(type_parameter_list_parser().or_not())
        .then(conformance_list_parser().or_not())
        .then(where_clause_parser().or_not())
        .then(token(Token::LBrace))
        .then(protocol_body_item_parser().repeated().collect::<Vec<_>>())
        .then(token(Token::RBrace))
        .map(
            |(
                (
                    (
                        (
                            (
                                (
                                    (((attributes, visibility), protocol_span), name_span),
                                    type_params,
                                ),
                                inherited,
                            ),
                            where_clause,
                        ),
                        lbrace_span,
                    ),
                    body,
                ),
                rbrace_span,
            )| {
                ProtocolDeclarationData {
                    attributes,
                    visibility,
                    protocol_span,
                    name_span,
                    type_params,
                    inherited: inherited.map(|(colon_span, items)| ConformanceListData {
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

/// Parse a protocol declaration and emit events
///
/// This is the primary event-driven parser function for protocol declarations.
pub fn parse_protocol_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match protocol_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_protocol_declaration(sink, data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), *span);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    /// Helper to parse source code and return a ProtocolDeclaration
    fn parse(source: &str) -> ProtocolDeclaration {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new(0);
        parse_protocol_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        ProtocolDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    /// Helper to check if a syntax node exists as a child
    fn has_child(decl: &ProtocolDeclaration, kind: SyntaxKind) -> bool {
        decl.syntax.children().any(|child| child.kind() == kind)
    }

    #[test]
    fn test_protocol_declaration_basic() {
        let decl = parse("protocol Drawable { }");
        assert_eq!(decl.name(), Some("Drawable".to_string()));
        assert_eq!(decl.visibility(), None);
        assert_eq!(decl.syntax.kind(), SyntaxKind::ProtocolDeclaration);
    }

    #[test]
    fn test_protocol_declaration_with_visibility() {
        let decl = parse("public protocol Serializable { }");
        assert_eq!(decl.name(), Some("Serializable".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_protocol_with_type_params() {
        let decl = parse("protocol Collection[T] { }");
        assert_eq!(decl.name(), Some("Collection".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
    }

    #[test]
    fn test_protocol_with_where_clause() {
        let decl = parse("protocol Comparable[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Comparable".to_string()));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_protocol_with_method() {
        let decl = parse("protocol Drawable { func draw() }");
        assert_eq!(decl.name(), Some("Drawable".to_string()));
        assert_eq!(decl.methods().len(), 1);
    }

    #[test]
    fn test_protocol_with_method_and_return_type() {
        let decl = parse("protocol Serializable { func serialize() -> String }");
        assert_eq!(decl.name(), Some("Serializable".to_string()));
        assert_eq!(decl.methods().len(), 1);
    }

    #[test]
    fn test_protocol_with_multiple_methods() {
        let decl = parse("protocol Collection { func count() -> Int func isEmpty() -> Bool }");
        assert_eq!(decl.name(), Some("Collection".to_string()));
        assert_eq!(decl.methods().len(), 2);
    }

    #[test]
    fn test_protocol_method_with_parameters() {
        let decl = parse("protocol NetworkClient { func fetch(from url: String) -> Data }");
        assert_eq!(decl.name(), Some("NetworkClient".to_string()));
        assert_eq!(decl.methods().len(), 1);
    }

    #[test]
    fn test_protocol_method_with_generics() {
        let decl = parse("protocol Container { func get[T](index: Int) -> T }");
        assert_eq!(decl.name(), Some("Container".to_string()));
        let methods = decl.methods();
        assert_eq!(methods.len(), 1);
        let has_type_params = methods[0]
            .children()
            .any(|child| child.kind() == SyntaxKind::TypeParameterList);
        assert!(has_type_params, "Expected TypeParameterList on method");
    }

    #[test]
    fn test_protocol_with_associated_type() {
        let decl = parse("protocol Iterator { type Item; }");
        assert_eq!(decl.name(), Some("Iterator".to_string()));
        // Check that the body contains the TypeAliasDeclaration
        let body = decl.body().expect("Protocol should have body");
        let has_type_alias = body
            .children()
            .any(|c| c.kind() == SyntaxKind::TypeAliasDeclaration);
        assert!(
            has_type_alias,
            "Protocol body should contain TypeAliasDeclaration for associated type"
        );
    }

    #[test]
    fn test_protocol_method_with_body_parses() {
        // This should parse successfully - the error about having a body
        // should be detected at semantic analysis, not parsing
        let decl = parse("protocol BadProtocol { func doSomething() { } }");
        assert_eq!(decl.name(), Some("BadProtocol".to_string()));
        let methods = decl.methods();
        assert_eq!(methods.len(), 1);
        let has_body = methods[0]
            .children()
            .any(|c| c.kind() == SyntaxKind::FunctionBody);
        assert!(
            has_body,
            "Protocol method with body should parse and include FunctionBody node"
        );
    }

    #[test]
    fn test_protocol_inheritance() {
        let decl = parse("protocol Shape: Drawable { }");
        assert_eq!(decl.name(), Some("Shape".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_protocol_multiple_inheritance() {
        let decl = parse("protocol Widget: Drawable, Clickable { }");
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
}
