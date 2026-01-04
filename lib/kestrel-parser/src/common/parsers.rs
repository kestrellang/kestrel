//! Common parser combinators shared across multiple parsers
//!
//! This module provides reusable Chumsky parser combinators that are used
//! by multiple parser modules to avoid code duplication.
//!
//! # Chumsky 0.12 Migration Notes
//!
//! All parsers use the new chumsky 0.12 API with:
//! - Lifetime parameters for zero-copy parsing
//! - `select!` macro instead of `filter_map`
//! - `map_with` instead of `map_with_span`
//! - `Rich` errors instead of `Simple`

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use super::data::{
    DeinitDeclarationData, FieldDeclarationData, FunctionDeclarationData,
    InitializerDeclarationData, ParameterAccessMode, ParameterData, ReceiverModifier,
};
use crate::attribute::attribute_list_parser;
use crate::block::{code_block_parser, CodeBlockData};
use crate::input::{to_kestrel_span, ParserExtra, ParserInput};
use crate::ty::{ty_parser, TyVariant};
use crate::type_param::{type_parameter_list_parser, where_clause_parser};

/// Check if a token is trivia (whitespace or comment)
pub fn is_trivia(token: &Token) -> bool {
    matches!(
        token,
        Token::Whitespace | Token::LineComment | Token::BlockComment
    )
}

/// Parser that skips trivia tokens
pub fn skip_trivia<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| is_trivia(token))
        .repeated()
        .ignored()
}

/// Wrap a parser to skip leading trivia
pub fn trivia<'tokens, 'src: 'tokens, P, O>(
    parser: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, O, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, O, ParserExtra<'tokens>> + Clone,
{
    skip_trivia().ignore_then(parser)
}

/// Match a specific token, skipping leading trivia
pub fn token<'tokens>(
    t: Token,
) -> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    trivia(just(t).map_with(|_, e| to_kestrel_span(e.span())))
}

/// Parse an identifier, skipping leading trivia
pub fn identifier<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    trivia(select! {
        Token::Identifier = e => to_kestrel_span(e.span()),
    })
}

/// Internal Chumsky parser for module path segments
///
/// Parses identifier sequences separated by dots: A.B.C
/// Returns a vector of spans for each identifier segment.
///
/// # Examples
/// - `A` → `[span(A)]`
/// - `A.B.C` → `[span(A), span(B), span(C)]`
pub fn module_path_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<Span>, ParserExtra<'tokens>> + Clone {
    identifier()
        .separated_by(token(Token::Dot))
        .at_least(1)
        .collect()
}

/// Internal Chumsky parser for optional visibility modifier
///
/// Parses an optional visibility keyword: public, private, internal, or fileprivate
/// Returns `Some((token, span))` if a visibility modifier is present, `None` otherwise.
///
/// # Examples
/// - `public class Foo` → `Some((Token::Public, span))`
/// - `class Foo` → `None`
pub fn visibility_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Option<(Token, Span)>, ParserExtra<'tokens>> + Clone
{
    trivia(select! {
        Token::Public = e => (Token::Public, to_kestrel_span(e.span())),
        Token::Private = e => (Token::Private, to_kestrel_span(e.span())),
        Token::Internal = e => (Token::Internal, to_kestrel_span(e.span())),
        Token::Fileprivate = e => (Token::Fileprivate, to_kestrel_span(e.span())),
    })
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
pub fn module_declaration_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, (Span, Vec<Span>), ParserExtra<'tokens>> + Clone {
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
pub fn import_item_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, (Span, Option<Span>), ParserExtra<'tokens>> + Clone
{
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
pub fn import_items_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<(Span, Option<Span>)>, ParserExtra<'tokens>> + Clone
{
    token(Token::LParen)
        .ignore_then(
            import_item_parser_internal()
                .separated_by(token(Token::Comma))
                .at_least(1)
                .collect(),
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
pub fn import_declaration_parser_internal<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    (
        Span,
        Vec<Span>,
        Option<Span>,
        Option<Vec<(Span, Option<Span>)>>,
    ),
    ParserExtra<'tokens>,
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
pub fn static_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Option<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Static).map_with(|_, e| Some(to_kestrel_span(e.span()))))
        .or(empty().to(None))
}

/// Parser for optional receiver modifier (mutating/consuming)
///
/// Parses an optional `mutating` or `consuming` keyword and returns the modifier with its span.
///
/// # Examples
/// - `mutating func foo()` → `Some((ReceiverModifier::Mutating, span))`
/// - `consuming func foo()` → `Some((ReceiverModifier::Consuming, span))`
/// - `func foo()` → `None`
pub fn receiver_modifier_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Option<(ReceiverModifier, Span)>, ParserExtra<'tokens>>
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

/// Parser for let/var mutability keyword
///
/// Parses either `let` or `var` and returns the span and mutability flag.
///
/// # Returns
/// - `(span, false)` for `let`
/// - `(span, true)` for `var`
pub fn let_var_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, (Span, bool), ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        just(Token::Let)
            .map_with(|_, e| (to_kestrel_span(e.span()), false))
            .or(just(Token::Var).map_with(|_, e| (to_kestrel_span(e.span()), true))),
    )
}

// =============================================================================
// Parameter Parsers
// =============================================================================

/// Parser for optional parameter access mode (mutating/consuming)
///
/// Parses an optional `mutating` or `consuming` keyword before a parameter.
///
/// # Examples
/// - `mutating x: Int` → `Some((ParameterAccessMode::Mutating, span))`
/// - `consuming x: Int` → `Some((ParameterAccessMode::Consuming, span))`
/// - `x: Int` → `None` (defaults to borrow)
fn parameter_access_mode_parser<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    Option<(ParameterAccessMode, Span)>,
    ParserExtra<'tokens>,
> + Clone {
    skip_trivia()
        .ignore_then(
            just(Token::Mutating)
                .map_with(|_, e| Some((ParameterAccessMode::Mutating, to_kestrel_span(e.span()))))
                .or(just(Token::Consuming).map_with(|_, e| {
                    Some((ParameterAccessMode::Consuming, to_kestrel_span(e.span())))
                })),
        )
        .or(empty().to(None))
}

/// Parser for a single parameter: `(access_mode)? (label)? bind_name: Type`
///
/// # Examples
/// - `x: Int` → access_mode=None, label=None, bind_name=x
/// - `with x: Int` → access_mode=None, label="with", bind_name=x
/// - `mutating x: Int` → access_mode=Mutating, label=None, bind_name=x
/// - `consuming point p: Point` → access_mode=Consuming, label="point", bind_name=p
pub(crate) fn parameter_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, ParameterData, ParserExtra<'tokens>> + Clone {
    // Parse identifier (with trivia skipping)
    let ident = trivia(select! {
        Token::Identifier = e => to_kestrel_span(e.span()),
    });

    // Labeled parameter: (access_mode)? label name: Type
    let labeled = parameter_access_mode_parser()
        .then(ident.clone())
        .then(ident.clone())
        .then(trivia(
            just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())),
        ))
        .then(ty_parser())
        .map(
            |((((access_mode, label), bind_name), colon), ty)| ParameterData {
                access_mode,
                label: Some(label),
                bind_name,
                colon,
                ty,
            },
        );

    // Unlabeled parameter: (access_mode)? name: Type
    let unlabeled = parameter_access_mode_parser()
        .then(ident)
        .then(trivia(
            just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())),
        ))
        .then(ty_parser())
        .map(|(((access_mode, bind_name), colon), ty)| ParameterData {
            access_mode,
            label: None,
            bind_name,
            colon,
            ty,
        });

    // Try labeled first (more specific), then unlabeled
    labeled.or(unlabeled)
}

/// Parser for parameter list (zero or more parameters separated by commas)
pub(crate) fn parameter_list_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<ParameterData>, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        parameter_parser()
            .separated_by(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())))
            .allow_trailing()
            .collect(),
    )
}

/// Parser for optional return type: `-> Type`
///
/// # Returns
/// - `Some((arrow_span, type))` if return type is present
/// - `None` if no return type
pub(crate) fn return_type_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Option<(Span, TyVariant)>, ParserExtra<'tokens>> + Clone
{
    skip_trivia()
        .ignore_then(just(Token::Arrow).map_with(|_, e| to_kestrel_span(e.span())))
        .then(ty_parser())
        .map(|(arrow, ty)| (arrow, ty))
        .or_not()
}

/// Parser for optional function body (code block)
///
/// # Returns
/// - `Some(CodeBlockData)` if body is present
/// - `None` if no body (e.g., protocol method declarations)
pub fn function_body_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Option<CodeBlockData>, ParserExtra<'tokens>> + Clone
{
    code_block_parser().map(Some).or(empty().to(None))
}

// =============================================================================
// Declaration Parsers - Single Source of Truth
// =============================================================================

/// Parser for a function declaration
///
/// Syntax: `(@attr)* (visibility)? (static)? (mutating|consuming)? func name[T, U]?(params) (-> Type)? (where ...)? ({ })?`
///
/// This is the single source of truth for function declaration parsing.
pub fn function_declaration_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, FunctionDeclarationData, ParserExtra<'tokens>> + Clone
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
}

/// Parser for a field declaration
///
/// Syntax: `(@attr)* (visibility)? (static)? let/var name: Type (;)?`
///
/// This is the single source of truth for field declaration parsing.
/// An optional trailing semicolon is allowed for inline field declarations.
pub fn field_declaration_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, FieldDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
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
                        (
                            (((attributes, visibility), is_static), (mutability_span, is_mutable)),
                            name_span,
                        ),
                        colon_span,
                    ),
                    ty,
                ),
                semicolon,
            )| {
                FieldDeclarationData {
                    attributes,
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
/// Syntax: `(@attr)* (visibility)? init(params) { body }?`
/// Body is optional for protocol initializer declarations.
///
/// This is the single source of truth for initializer declaration parsing.
pub fn initializer_declaration_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, InitializerDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(token(Token::Init))
        .then(token(Token::LParen))
        .then(parameter_list_parser())
        .then(token(Token::RParen))
        .then(function_body_parser())
        .map(
            |((((((attributes, visibility), init_span), lparen), parameters), rparen), body)| {
                InitializerDeclarationData {
                    attributes,
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

/// Parser for a deinitializer declaration
///
/// Syntax: `deinit { body }`
/// Deinit blocks are used for RAII-style cleanup when a value goes out of scope.
/// They have no parameters, attributes, or visibility modifiers.
///
/// This is the single source of truth for deinit declaration parsing.
pub fn deinit_declaration_parser_internal<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, DeinitDeclarationData, ParserExtra<'tokens>> + Clone
{
    token(Token::Deinit)
        .then(code_block_parser())
        .map(|(deinit_span, body)| DeinitDeclarationData { deinit_span, body })
}
