//! Function declaration parsing
//!
//! This module is the single source of truth for function declaration parsing.
//! Functions support generics with type parameters and where clauses.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::common::{
    AttributeData, FunctionBodyData, ParameterData, emit_attribute_list, emit_function_body,
    emit_name, emit_parameter_list, emit_return_type, emit_static_modifier, emit_visibility,
    function_body_parser, identifier, parameter_list_parser, return_type_parser, skip_trivia,
    static_parser, token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::parse_and_emit;
use crate::ty::TyVariant;
use crate::type_param::{
    TypeParameterData, WhereClauseData, emit_type_parameter_list, emit_where_clause,
    type_parameter_list_parser, where_clause_parser,
};

/// Represents a function declaration: (visibility)? (static)? fn name[T]?(params) (-> return_type)? (where ...)? { }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl FunctionDeclaration {
    /// Create a new FunctionDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the function name from this declaration
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

    /// Check if this function has the static modifier
    pub fn is_static(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier)
    }

    /// Get the parameter list node
    pub fn parameter_list(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ParameterList)
    }

    /// Get the return type node if present
    pub fn return_type(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ReturnType)
    }

    /// Get the function body node
    pub fn body(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::FunctionBody)
    }

    /// Check if this function has type parameters
    pub fn has_type_parameters(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::TypeParameterList)
    }

    /// Check if this function has a where clause
    pub fn has_where_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::WhereClause)
    }
}

/// Receiver modifier for instance methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverModifier {
    /// `mutating func` - method can mutate self
    Mutating,
    /// `consuming func` - method takes ownership of self
    Consuming,
}

/// Raw parsed data for function declaration internals
///
/// Used by both function declarations and protocol method declarations.
#[derive(Debug, Clone)]
pub struct FunctionDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub is_static: Option<Span>,
    /// Receiver modifier (mutating/consuming) with its span
    pub receiver_modifier: Option<(ReceiverModifier, Span)>,
    pub fn_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub lparen: Span,
    pub parameters: Vec<ParameterData>,
    pub rparen: Span,
    pub return_type: Option<(Span, TyVariant)>, // (arrow_span, return_ty)
    pub where_clause: Option<WhereClauseData>,
    pub body: Option<FunctionBodyData>, // Optional body - None for protocol methods
}

/// Parser for optional receiver modifier (mutating/consuming)
///
/// Parses an optional `mutating` or `consuming` keyword and returns the modifier with its span.
fn receiver_modifier_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<(ReceiverModifier, Span)>, ParserExtra<'tokens>>
+ Clone {
    skip_trivia()
        .ignore_then(
            just(Token::Mutating)
                .map_with(|_, e| Some((ReceiverModifier::Mutating, to_kestrel_span(e.span()))))
                .or(just(Token::Consuming).map_with(|_, e| {
                    Some((ReceiverModifier::Consuming, to_kestrel_span(e.span())))
                })),
        )
        .or(empty().to(None))
}

/// Parser for a function declaration
///
/// Syntax: `(@attr)* (visibility)? (static)? (mutating|consuming)? func name[T, U]?(params) (-> Type)? (where ...)? ({ } | = expr)?`
///
/// This is the single source of truth for function declaration parsing.
pub fn function_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, FunctionDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(static_parser())
        .then(receiver_modifier_parser())
        .then(token(Token::Func))
        .then(identifier())
        .then(type_parameter_list_parser().or_not())
        .then(token(Token::LParen))
        .then(parameter_list_parser())
        .then(token(Token::RParen))
        .then(return_type_parser())
        .then(where_clause_parser().or_not())
        .then(function_body_parser())
        .map(
            |(
                (
                    (
                        (
                            (
                                (
                                    (
                                        (
                                            (
                                                (
                                                    ((attributes, visibility), is_static),
                                                    receiver_modifier,
                                                ),
                                                fn_span,
                                            ),
                                            name_span,
                                        ),
                                        type_params,
                                    ),
                                    lparen,
                                ),
                                parameters,
                            ),
                            rparen,
                        ),
                        return_type,
                    ),
                    where_clause,
                ),
                body,
            )| {
                FunctionDeclarationData {
                    attributes,
                    visibility,
                    is_static,
                    receiver_modifier,
                    fn_span,
                    name_span,
                    type_params,
                    lparen,
                    parameters,
                    rparen,
                    return_type,
                    where_clause,
                    body,
                }
            },
        )
        .boxed()
}

/// Emit events for a function declaration
///
/// This is the single source of truth for function declaration emission.
pub fn emit_function_declaration(sink: &mut EventSink, data: FunctionDeclarationData) {
    sink.start_node(SyntaxKind::FunctionDeclaration);

    emit_attribute_list(sink, &data.attributes);
    emit_visibility(sink, data.visibility);
    emit_static_modifier(sink, data.is_static);

    // Emit receiver modifier (mutating/consuming) if present
    if let Some((modifier, span)) = data.receiver_modifier {
        let kind = match modifier {
            ReceiverModifier::Mutating => SyntaxKind::Mutating,
            ReceiverModifier::Consuming => SyntaxKind::Consuming,
        };
        sink.add_token(kind, span);
    }

    sink.add_token(SyntaxKind::Func, data.fn_span);
    emit_name(sink, data.name_span);

    if let Some((lbracket, params, rbracket)) = data.type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    emit_parameter_list(sink, data.lparen, data.parameters, data.rparen);

    if let Some((arrow_span, return_ty)) = data.return_type {
        emit_return_type(sink, arrow_span, return_ty);
    }

    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    if let Some(ref body) = data.body {
        emit_function_body(sink, body);
    }

    sink.finish_node();
}

/// Parse a function declaration and emit events
///
/// This is the primary event-driven parser function for function declarations.
pub fn parse_function_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        function_declaration_parser_internal(),
        emit_function_declaration
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_function_declaration_basic() {
        let source = "func test() { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("test".to_string()));
        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_static());
    }

    #[test]
    fn test_function_declaration_with_visibility() {
        let source = "public func greet() { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("greet".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_function_declaration_static() {
        let source = "static func create() { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("create".to_string()));
        assert!(decl.is_static());
    }

    #[test]
    fn test_function_declaration_with_params() {
        let source = "func add(a: Int, b: Int) { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("add".to_string()));
        assert!(decl.parameter_list().is_some());
    }

    #[test]
    fn test_function_declaration_with_labeled_param() {
        let source = "func greet(with name: String) { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("greet".to_string()));
        assert!(decl.parameter_list().is_some());
    }

    #[test]
    fn test_function_declaration_with_return_type() {
        let source = "func multiply(x: Int, y: Int) -> Int { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("multiply".to_string()));
        assert!(decl.return_type().is_some());
    }

    #[test]
    fn test_function_declaration_with_generics() {
        let source = "func identity[T](value: T) -> T { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("identity".to_string()));
        assert!(decl.has_type_parameters());
    }

    #[test]
    fn test_function_declaration_with_where_clause() {
        let source = "func compare[T](a: T, b: T) -> Bool where T: Equatable { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("compare".to_string()));
        assert!(decl.has_type_parameters());
        assert!(decl.has_where_clause());
    }

    #[test]
    fn test_function_declaration_full() {
        let source = "public static func calculate(value: Float, multiplier: Float) -> Float { }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("calculate".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_static());
        assert!(decl.parameter_list().is_some());
        assert!(decl.return_type().is_some());
    }

    #[test]
    fn test_function_with_deinit_statement() {
        let source = "func example() { let x: Int = 0; deinit x; }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_function_declaration(source, tokens.into_iter(), &mut sink);

        let events = sink.into_events();

        // Check for parse errors
        let errors: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                crate::event::Event::Error { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect();

        assert!(errors.is_empty(), "Got parse errors: {:?}", errors);

        let tree = TreeBuilder::new(source, events).build();
        let decl = FunctionDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("example".to_string()));
        assert!(decl.body().is_some(), "Function should have a body");
    }
}
