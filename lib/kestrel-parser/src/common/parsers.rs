//! Common parser combinators shared across multiple parsers
//!
//! This module provides reusable Chumsky parser combinators that are used
//! by multiple parser modules to avoid code duplication.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use super::data::{
    FieldDeclarationData, FunctionDeclarationData, InitializerDeclarationData, ParameterData,
    ReceiverModifier,
};
use crate::block::{CodeBlockData, code_block_parser};
use crate::ty::{TyVariant, ty_parser};
use crate::type_param::{type_parameter_list_parser, where_clause_parser};

/// Check if a token is trivia (whitespace or comment)
pub fn is_trivia(token: &Token) -> bool {
    matches!(
        token,
        Token::Whitespace | Token::LineComment | Token::BlockComment
    )
}

/// Parser that skips trivia tokens
pub fn skip_trivia() -> impl Parser<Token, (), Error = Simple<Token>> + Clone {
    filter(|token: &Token| is_trivia(token))
        .repeated()
        .ignored()
}

/// Wrap a parser to skip leading trivia
pub fn trivia<P, O>(parser: P) -> impl Parser<Token, O, Error = Simple<Token>> + Clone
where
    P: Parser<Token, O, Error = Simple<Token>> + Clone,
{
    skip_trivia().ignore_then(parser)
}

/// Match a specific token, skipping leading trivia
pub fn token(t: Token) -> impl Parser<Token, Span, Error = Simple<Token>> + Clone {
    trivia(just(t).map_with_span(|_, span| Span::from(span)))
}

/// Parse an identifier, skipping leading trivia
pub fn identifier() -> impl Parser<Token, Span, Error = Simple<Token>> + Clone {
    trivia(filter_map(|span, token| match token {
        Token::Identifier => Ok(Span::from(span)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    }))
}

/// Internal Chumsky parser for module path segments
///
/// Parses identifier sequences separated by dots: A.B.C
/// Returns a vector of spans for each identifier segment.
///
/// # Examples
/// - `A` → `[span(A)]`
/// - `A.B.C` → `[span(A), span(B), span(C)]`
pub fn module_path_parser_internal() -> impl Parser<Token, Vec<Span>, Error = Simple<Token>> + Clone
{
    identifier().separated_by(token(Token::Dot)).at_least(1)
}

/// Internal Chumsky parser for optional visibility modifier
///
/// Parses an optional visibility keyword: public, private, internal, or fileprivate
/// Returns `Some((token, span))` if a visibility modifier is present, `None` otherwise.
///
/// # Examples
/// - `public class Foo` → `Some((Token::Public, span))`
/// - `class Foo` → `None`
pub fn visibility_parser_internal()
-> impl Parser<Token, Option<(Token, Span)>, Error = Simple<Token>> + Clone {
    trivia(filter_map(|span, token| match token {
        Token::Public | Token::Private | Token::Internal | Token::Fileprivate => {
            Ok((token, Span::from(span)))
        }
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    }))
    .or_not()
}

/// Internal Chumsky parser for module declaration
///
/// Parses: `module A.B.C`
/// Returns: `(module_keyword_span, path_segments)`
///
/// # Examples
/// - `module A` → `(span(module), [span(A)])`
/// - `module A.B.C` → `(span(module), [span(A), span(B), span(C)])`
pub fn module_declaration_parser_internal()
-> impl Parser<Token, (Span, Vec<Span>), Error = Simple<Token>> + Clone {
    token(Token::Module).then(module_path_parser_internal())
}

/// Internal parser for import item (identifier or identifier as alias)
///
/// Parses a single import item, optionally with an alias:
/// - `D` → `(span(D), None)`
/// - `D as E` → `(span(D), Some(span(E)))`
///
/// # Examples
/// - `Foo` → `(span(Foo), None)`
/// - `Foo as Bar` → `(span(Foo), Some(span(Bar)))`
pub fn import_item_parser_internal()
-> impl Parser<Token, (Span, Option<Span>), Error = Simple<Token>> + Clone {
    identifier().then(token(Token::As).ignore_then(identifier()).or_not())
}

/// Internal parser for import items list
///
/// Parses a parenthesized list of import items: `(D, E)` or `(D as E, F as G)`
/// Returns a vector of tuples: `(name_span, optional_alias_span)`
///
/// # Examples
/// - `(D, E)` → `[(span(D), None), (span(E), None)]`
/// - `(D as E, F)` → `[(span(D), Some(span(E))), (span(F), None)]`
pub fn import_items_parser_internal()
-> impl Parser<Token, Vec<(Span, Option<Span>)>, Error = Simple<Token>> + Clone {
    token(Token::LParen)
        .ignore_then(
            import_item_parser_internal()
                .separated_by(token(Token::Comma))
                .at_least(1),
        )
        .then_ignore(token(Token::RParen))
}

/// Internal parser for import declaration
///
/// Parses all forms of import declarations:
/// - `import A.B.C` (import all)
/// - `import A.B.C as D` (import with alias)
/// - `import A.B.C.(D, E)` (import specific items)
/// - `import A.B.C.(D as E, F as G)` (import items with aliases)
///
/// Returns: `(import_keyword_span, path_segments, optional_alias, optional_items_list)`
///
/// # Examples
/// - `import A.B.C` → `(span(import), [span(A), span(B), span(C)], None, None)`
/// - `import A.B.C as D` → `(span(import), [span(A), span(B), span(C)], Some(span(D)), None)`
/// - `import A.B.C.(D, E)` → `(span(import), [span(A), span(B), span(C)], None, Some([...]))`
pub fn import_declaration_parser_internal() -> impl Parser<
    Token,
    (
        Span,
        Vec<Span>,
        Option<Span>,
        Option<Vec<(Span, Option<Span>)>>,
    ),
    Error = Simple<Token>,
> + Clone {
    token(Token::Import)
        .then(module_path_parser_internal())
        .then(
            // Optional: either "as Alias" or ".(items)"
            token(Token::As)
                .ignore_then(identifier())
                .map(|alias| (Some(alias), None))
                .or(token(Token::Dot)
                    .ignore_then(import_items_parser_internal())
                    .map(|items| (None, Some(items))))
                .or_not(),
        )
        .map(|((import_span, path_segments), alias_or_items)| {
            let (alias, items) = match alias_or_items {
                Some((alias, items)) => (alias, items),
                None => (None, None),
            };
            (import_span, path_segments, alias, items)
        })
}

// =============================================================================
// Shared Modifier Parsers
// =============================================================================

/// Parser for optional static modifier
///
/// Parses an optional `static` keyword and returns its span if present.
///
/// # Examples
/// - `static func foo()` → `Some(span(static))`
/// - `func foo()` → `None`
pub fn static_parser() -> impl Parser<Token, Option<Span>, Error = Simple<Token>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Static).map_with_span(|_, span| Some(Span::from(span))))
        .or(empty().map(|_| None))
}

/// Parser for optional receiver modifier (mutating/consuming)
///
/// Parses an optional `mutating` or `consuming` keyword and returns the modifier with its span.
///
/// # Examples
/// - `mutating func foo()` → `Some((ReceiverModifier::Mutating, span))`
/// - `consuming func foo()` → `Some((ReceiverModifier::Consuming, span))`
/// - `func foo()` → `None`
pub fn receiver_modifier_parser()
-> impl Parser<Token, Option<(ReceiverModifier, Span)>, Error = Simple<Token>> + Clone {
    skip_trivia()
        .ignore_then(
            just(Token::Mutating)
                .map_with_span(|_, span| Some((ReceiverModifier::Mutating, Span::from(span))))
                .or(just(Token::Consuming).map_with_span(|_, span| {
                    Some((ReceiverModifier::Consuming, Span::from(span)))
                })),
        )
        .or(empty().map(|_| None))
}

/// Parser for let/var mutability keyword
///
/// Parses either `let` or `var` and returns the span and mutability flag.
///
/// # Returns
/// - `(span, false)` for `let`
/// - `(span, true)` for `var`
pub fn let_var_parser() -> impl Parser<Token, (Span, bool), Error = Simple<Token>> + Clone {
    skip_trivia().ignore_then(
        just(Token::Let)
            .map_with_span(|_, span| (Span::from(span), false))
            .or(just(Token::Var).map_with_span(|_, span| (Span::from(span), true))),
    )
}

// =============================================================================
// Parameter Parsers
// =============================================================================

/// Parser for a single parameter: `(label)? bind_name: Type`
///
/// # Examples
/// - `x: Int` → label=None, bind_name=x
/// - `with x: Int` → label="with", bind_name=x
pub(crate) fn parameter_parser() -> impl Parser<Token, ParameterData, Error = Simple<Token>> + Clone
{
    // Try to parse: identifier identifier : type (labeled parameter)
    // Or: identifier : type (unlabeled parameter)
    skip_trivia()
        .ignore_then(filter_map(|span, token| match token {
            Token::Identifier => Ok(Span::from(span)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        }))
        .then(
            // Optional second identifier (if present, first was label)
            skip_trivia()
                .ignore_then(filter_map(|span, token| match token {
                    Token::Identifier => Ok(Span::from(span)),
                    _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
                }))
                .or_not(),
        )
        .then(
            skip_trivia().ignore_then(just(Token::Colon).map_with_span(|_, span| Span::from(span))),
        )
        .then(ty_parser())
        .map(|(((first_ident, second_ident_opt), colon), ty)| {
            match second_ident_opt {
                Some(second_ident) => {
                    // Two identifiers: first is label, second is bind_name
                    ParameterData {
                        label: Some(first_ident),
                        bind_name: second_ident,
                        colon,
                        ty,
                    }
                }
                None => {
                    // One identifier: it's the bind_name, no label
                    ParameterData {
                        label: None,
                        bind_name: first_ident,
                        colon,
                        ty,
                    }
                }
            }
        })
}

/// Parser for parameter list (zero or more parameters separated by commas)
pub(crate) fn parameter_list_parser()
-> impl Parser<Token, Vec<ParameterData>, Error = Simple<Token>> + Clone {
    skip_trivia().ignore_then(
        parameter_parser()
            .separated_by(just(Token::Comma).map_with_span(|_, span| Span::from(span)))
            .allow_trailing(),
    )
}

/// Parser for optional return type: `-> Type`
///
/// # Returns
/// - `Some((arrow_span, type))` if return type is present
/// - `None` if no return type
pub(crate) fn return_type_parser()
-> impl Parser<Token, Option<(Span, TyVariant)>, Error = Simple<Token>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Arrow).map_with_span(|_, span| Span::from(span)))
        .then(ty_parser())
        .map(|(arrow, ty)| (arrow, ty))
        .or_not()
}

/// Parser for optional function body (code block)
///
/// # Returns
/// - `Some(CodeBlockData)` if body is present
/// - `None` if no body (e.g., protocol method declarations)
pub fn function_body_parser()
-> impl Parser<Token, Option<CodeBlockData>, Error = Simple<Token>> + Clone {
    code_block_parser().map(Some).or(empty().map(|_| None))
}

// =============================================================================
// Declaration Parsers - Single Source of Truth
// =============================================================================

/// Parser for a function declaration
///
/// Syntax: `(visibility)? (static)? (mutating|consuming)? func name[T, U]?(params) (-> Type)? (where ...)? ({ })?`
///
/// This is the single source of truth for function declaration parsing.
pub fn function_declaration_parser_internal()
-> impl Parser<Token, FunctionDeclarationData, Error = Simple<Token>> + Clone {
    visibility_parser_internal()
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
                                            (((visibility, is_static), receiver_modifier), fn_span),
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
}

/// Parser for a field declaration
///
/// Syntax: `(visibility)? (static)? let/var name: Type (;)?`
///
/// This is the single source of truth for field declaration parsing.
/// An optional trailing semicolon is allowed for inline field declarations.
pub fn field_declaration_parser_internal()
-> impl Parser<Token, FieldDeclarationData, Error = Simple<Token>> + Clone {
    visibility_parser_internal()
        .then(static_parser())
        .then(let_var_parser())
        .then(identifier())
        .then(token(Token::Colon))
        .then(ty_parser())
        .then(token(Token::Semicolon).or_not())
        .map(
            |(
                (
                    (
                        (((visibility, is_static), (mutability_span, is_mutable)), name_span),
                        colon_span,
                    ),
                    ty,
                ),
                semicolon,
            )| {
                FieldDeclarationData {
                    visibility,
                    is_static,
                    mutability_span,
                    is_mutable,
                    name_span,
                    colon_span,
                    ty,
                    semicolon,
                }
            },
        )
}

/// Parser for an initializer declaration
///
/// Syntax: `(visibility)? init(params) { body }?`
/// Body is optional for protocol initializer declarations.
///
/// This is the single source of truth for initializer declaration parsing.
pub fn initializer_declaration_parser_internal()
-> impl Parser<Token, InitializerDeclarationData, Error = Simple<Token>> + Clone {
    visibility_parser_internal()
        .then(token(Token::Init))
        .then(token(Token::LParen))
        .then(parameter_list_parser())
        .then(token(Token::RParen))
        .then(function_body_parser())
        .map(
            |(((((visibility, init_span), lparen), parameters), rparen), body)| {
                InitializerDeclarationData {
                    visibility,
                    init_span,
                    lparen,
                    parameters,
                    rparen,
                    body,
                }
            },
        )
}
