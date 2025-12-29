//! Enum declaration parsing
//!
//! This module is the single source of truth for enum declaration parsing.
//! Enum bodies can contain: cases, functions, initializers, nested structs/enums,
//! type aliases, modules, and imports.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::ConformanceListData;
use crate::common::{
    emit_enum_declaration, field_declaration_parser_internal, function_declaration_parser_internal,
    identifier, import_declaration_parser_internal, initializer_declaration_parser_internal,
    module_declaration_parser_internal, token, visibility_parser_internal, EnumCaseDeclarationData,
    EnumCaseParameterData, EnumDeclarationData, TypeDeclarationBodyItem,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens, to_kestrel_span, ParserExtra, ParserInput};
use crate::ty::ty_parser;
use crate::type_alias::type_alias_declaration_parser_internal;
use crate::type_param::{conformance_list_parser, type_parameter_list_parser, where_clause_parser};

/// Represents an enum declaration: (visibility)? (indirect)? enum Name[T]? (: Conformances)? (where ...)? { ... }
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

    /// Check if this enum has the `indirect` modifier
    pub fn is_indirect(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::IndirectModifier)
    }

    /// Get child declaration items (cases, nested structs, imports, modules, functions, initializers)
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::EnumBody)
            .map(|body| {
                body.children()
                    .filter(|child| {
                        matches!(
                            child.kind(),
                            SyntaxKind::EnumCaseDeclaration
                                | SyntaxKind::StructDeclaration
                                | SyntaxKind::EnumDeclaration
                                | SyntaxKind::ImportDeclaration
                                | SyntaxKind::ModuleDeclaration
                                | SyntaxKind::FieldDeclaration
                                | SyntaxKind::FunctionDeclaration
                                | SyntaxKind::InitializerDeclaration
                                | SyntaxKind::TypeAliasDeclaration
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all enum cases in this declaration
    pub fn cases(&self) -> Vec<SyntaxNode> {
        self.children()
            .into_iter()
            .filter(|child| child.kind() == SyntaxKind::EnumCaseDeclaration)
            .collect()
    }
}

/// Parser that skips trivia tokens
fn skip_trivia<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| {
            matches!(
                token,
                Token::Whitespace | Token::LineComment | Token::BlockComment
            )
        })
        .repeated()
        .ignored()
}

/// Parser for enum case parameter: `label: Type`
fn enum_case_parameter_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, EnumCaseParameterData, ParserExtra<'tokens>> + Clone
{
    identifier()
        .then(token(Token::Colon))
        .then(ty_parser())
        .map(|((label, colon), ty)| EnumCaseParameterData { label, colon, ty })
}

/// Parser for enum case declaration: `case Name` or `case Name(label: Type, ...)`
fn enum_case_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, EnumCaseDeclarationData, ParserExtra<'tokens>> + Clone
{
    token(Token::Case)
        .then(identifier())
        .then(
            // Optional parameter list: (label: Type, label: Type, ...)
            token(Token::LParen)
                .then(
                    enum_case_parameter_parser()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then(token(Token::RParen))
                .map(|((lparen, params), rparen)| Some((lparen, params, rparen)))
                .or(empty().map(|_| None)),
        )
        .map(
            |((case_span, name_span), parameters)| EnumCaseDeclarationData {
                case_span,
                name_span,
                parameters,
            },
        )
}

/// Internal parser for enum body items
///
/// Enum bodies can contain: cases, functions, initializers, nested structs/enums,
/// type aliases, modules, and imports.
/// Note: Fields are technically parsed but should be rejected at semantic analysis.
fn enum_body_item_parser_internal<'tokens>(
    enum_parser: impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>>
        + Clone,
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeDeclarationBodyItem, ParserExtra<'tokens>> + Clone
{
    let module_parser = module_declaration_parser_internal()
        .map(|(module_span, path)| TypeDeclarationBodyItem::Module(module_span, path));

    let import_parser =
        import_declaration_parser_internal().map(|(import_span, path, alias, items)| {
            TypeDeclarationBodyItem::Import(import_span, path, alias, items)
        });

    // Nested enums are boxed to avoid infinite size
    let nested_enum_parser = enum_parser.map(|data| TypeDeclarationBodyItem::Enum(Box::new(data)));

    // Enum cases
    let case_parser = enum_case_parser().map(TypeDeclarationBodyItem::EnumCase);

    let initializer_parser =
        initializer_declaration_parser_internal().map(TypeDeclarationBodyItem::Initializer);

    let function_parser =
        function_declaration_parser_internal().map(TypeDeclarationBodyItem::Function);

    let type_alias_parser =
        type_alias_declaration_parser_internal().map(TypeDeclarationBodyItem::TypeAlias);

    // Fields are parsed but should be rejected semantically (enums don't have stored properties)
    let field_parser = field_declaration_parser_internal().map(TypeDeclarationBodyItem::Field);

    // Order matters: try case first (unique to enums), then shared items
    module_parser
        .or(import_parser)
        .or(case_parser)
        .or(nested_enum_parser)
        .or(initializer_parser)
        .or(type_alias_parser) // Check type alias before function (both can have visibility)
        .or(function_parser)
        .or(field_parser)
}

/// Parser for the optional `indirect` modifier
fn indirect_modifier_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Option<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Indirect).map_with(|_, e| Some(to_kestrel_span(e.span()))))
        .or(empty().to(None))
}

/// Internal Chumsky parser for enum declaration
///
/// This is the single source of truth for enum declaration parsing.
pub fn enum_declaration_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>> + Clone {
    recursive(|enum_parser| {
        visibility_parser_internal()
            .then(indirect_modifier_parser())
            .then(token(Token::Enum))
            .then(identifier())
            .then(type_parameter_list_parser().or_not())
            .then(conformance_list_parser().or_not())
            .then(where_clause_parser().or_not())
            .then(token(Token::LBrace))
            .then(
                enum_body_item_parser_internal(enum_parser)
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .then(token(Token::RBrace))
            .map(
                |(
                    (
                        (
                            (
                                (
                                    ((((visibility, indirect), enum_span), name_span), type_params),
                                    conformances,
                                ),
                                where_clause,
                            ),
                            lbrace_span,
                        ),
                        body,
                    ),
                    rbrace_span,
                )| {
                    EnumDeclarationData {
                        visibility,
                        indirect,
                        enum_span,
                        name_span,
                        type_params,
                        conformances: conformances.map(|(colon_span, types)| ConformanceListData {
                            colon_span,
                            conformances: types,
                        }),
                        where_clause,
                        lbrace_span,
                        body,
                        rbrace_span,
                    }
                },
            )
    })
}

/// Parse an enum declaration and emit events
///
/// This is the primary event-driven parser function for enum declarations.
pub fn parse_enum_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match enum_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok(data) => {
            emit_enum_declaration(sink, data);
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
        assert!(!decl.is_indirect());
        assert_eq!(decl.syntax.kind(), SyntaxKind::EnumDeclaration);
    }

    #[test]
    fn test_enum_declaration_with_visibility() {
        let decl = parse("public enum Direction { }");
        assert_eq!(decl.name(), Some("Direction".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_enum_declaration_with_indirect() {
        let decl = parse("indirect enum Tree { }");
        assert_eq!(decl.name(), Some("Tree".to_string()));
        assert!(decl.is_indirect());
    }

    #[test]
    fn test_enum_declaration_with_visibility_and_indirect() {
        let decl = parse("public indirect enum LinkedList { }");
        assert_eq!(decl.name(), Some("LinkedList".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_indirect());
    }

    #[test]
    fn test_enum_with_simple_case() {
        let decl = parse("enum Color { case Red }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].kind(), SyntaxKind::EnumCaseDeclaration);
    }

    #[test]
    fn test_enum_with_multiple_cases() {
        let decl = parse("enum Color { case Red case Green case Blue }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 3);
    }

    #[test]
    fn test_enum_case_with_associated_values() {
        let decl = parse("enum Result { case Success(value: Int) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        // Check that the case has parameters
        let case_node = &cases[0];
        let has_param_list = case_node
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(has_param_list);
    }

    #[test]
    fn test_enum_case_with_multiple_associated_values() {
        let decl = parse("enum Event { case Click(x: Int, y: Int) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        // Count parameters in the case
        let case_node = &cases[0];
        let param_list = case_node
            .children()
            .find(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(param_list.is_some());

        let param_count = param_list
            .unwrap()
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumCaseParameter)
            .count();
        assert_eq!(param_count, 2);
    }

    #[test]
    fn test_enum_with_type_params() {
        let decl = parse("enum Option[T] { }");
        assert_eq!(decl.name(), Some("Option".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
    }

    #[test]
    fn test_enum_with_conformance() {
        let decl = parse("enum Status: Equatable { }");
        assert_eq!(decl.name(), Some("Status".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_enum_with_where_clause() {
        let decl = parse("enum Container[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Container".to_string()));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_enum_full_syntax() {
        let decl = parse("public indirect enum Result[T, E]: Equatable where E: Error { case Success(value: T) case Failure(error: E) }");
        assert_eq!(decl.name(), Some("Result".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_indirect());
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
        assert_eq!(decl.cases().len(), 2);
    }

    #[test]
    fn test_enum_with_function() {
        let decl = parse("enum Color { case Red func describe() -> String { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_enum_with_initializer() {
        let decl = parse("enum Direction { case North init() { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::InitializerDeclaration);
    }
}
