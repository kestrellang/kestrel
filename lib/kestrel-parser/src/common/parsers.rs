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
    DeinitDeclarationData, FunctionBodyData, InitEffect, InitializerDeclarationData,
    ParameterAccessMode, ParameterData,
};
use crate::attribute::attribute_list_parser;
use crate::block::{CodeBlockData, code_block_parser};
use crate::expr::expr_parser;
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::pattern::{PatternVariant, StructPatternFieldData};
use crate::ty::{TyVariant, ty_parser};
use crate::type_param::{type_parameter_list_parser, where_clause_parser};

/// Check if a token is trivia (whitespace or comment)
pub fn is_trivia(token: &Token) -> bool {
    matches!(
        token,
        Token::Whitespace | Token::Newline | Token::LineComment | Token::BlockComment
    )
}

/// Check if a token is inline trivia (whitespace/comments, excluding explicit newline tokens)
pub fn is_inline_trivia(token: &Token) -> bool {
    matches!(
        token,
        Token::Whitespace | Token::LineComment | Token::BlockComment
    )
}

/// Parser that skips trivia tokens
pub fn skip_trivia<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| is_trivia(token))
        .repeated()
        .ignored()
}

/// Parser that skips inline trivia tokens (no newlines)
pub fn skip_inline_trivia<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| is_inline_trivia(token))
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
pub fn identifier<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    trivia(select! {
        Token::Identifier = e => to_kestrel_span(e.span()),
    })
}

/// Parse an identifier or keyword token, skipping leading trivia.
/// Used in label position where keywords are allowed (e.g., `func insert(in list: Array[Int])`).
pub fn identifier_or_keyword<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    trivia(
        any()
            .filter(|t: &Token| matches!(t, Token::Identifier) || t.is_label_keyword())
            .map_with(|_, e| to_kestrel_span(e.span())),
    )
}

/// Internal Chumsky parser for module path segments
///
/// Parses identifier sequences separated by dots: A.B.C
/// Returns a vector of spans for each identifier segment.
///
/// # Examples
/// - `A` → `[span(A)]`
/// - `A.B.C` → `[span(A), span(B), span(C)]`
pub fn module_path_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<Span>, ParserExtra<'tokens>> + Clone {
    identifier()
        .separated_by(token(Token::Dot))
        .at_least(1)
        .collect()
        .boxed()
}

/// Internal Chumsky parser for optional visibility modifier
///
/// Parses an optional visibility keyword: public, private, internal, or fileprivate
/// Returns `Some((token, span))` if a visibility modifier is present, `None` otherwise.
///
/// # Examples
/// - `public class Foo` → `Some((Token::Public, span))`
/// - `class Foo` → `None`
pub fn visibility_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<(Token, Span)>, ParserExtra<'tokens>> + Clone {
    trivia(select! {
        Token::Public = e => (Token::Public, to_kestrel_span(e.span())),
        Token::Private = e => (Token::Private, to_kestrel_span(e.span())),
        Token::Internal = e => (Token::Internal, to_kestrel_span(e.span())),
        Token::Fileprivate = e => (Token::Fileprivate, to_kestrel_span(e.span())),
    })
    .or_not()
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
pub fn static_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Static).map_with(|_, e| Some(to_kestrel_span(e.span()))))
        .or(empty().to(None))
}

/// Parser for let/var mutability keyword
///
/// Parses either `let` or `var` and returns the span and mutability flag.
///
/// # Returns
/// - `(span, false)` for `let`
/// - `(span, true)` for `var`
pub fn let_var_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (Span, bool), ParserExtra<'tokens>> + Clone {
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
fn parameter_access_mode_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<(ParameterAccessMode, Span)>, ParserExtra<'tokens>>
+ Clone {
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

/// Parser for irrefutable patterns used in function parameters.
///
/// Only allows patterns that always match:
/// - Binding patterns: `x`, `var x`
/// - Tuple patterns: `(a, b)`, `(a, (b, c))`
/// - Struct patterns: `Point { x, y }`, `Point { x: a, .. }`
/// - Wildcard: `_`
///
/// Does NOT allow refutable patterns (enum, literal, range, or).
pub(crate) fn parameter_pattern_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone {
    recursive(|param_pattern| {
        // Wildcard pattern: _
        let wildcard = skip_trivia()
            .ignore_then(just(Token::Underscore).map_with(|_, e| to_kestrel_span(e.span())))
            .map(PatternVariant::Wildcard);

        // Binding pattern: name or var name
        let binding = skip_trivia()
            .ignore_then(
                just(Token::Var)
                    .map_with(|_, e| Some(to_kestrel_span(e.span())))
                    .or(empty().to(None)),
            )
            .then(trivia(select! {
                Token::Identifier = e => to_kestrel_span(e.span()),
            }))
            .map(|(var_span, name_span)| PatternVariant::Binding {
                var_span,
                name_span,
            });

        // Tuple pattern: (p1, p2, ...)
        let tuple = skip_trivia()
            .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                param_pattern
                    .clone()
                    .separated_by(trivia(just(Token::Comma)))
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then(trivia(
                just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())),
            ))
            .map(|((lparen, elements), rparen)| PatternVariant::Tuple {
                lparen,
                elements,
                rparen,
            });

        // Struct pattern: StructName { field, field: pattern, .. }
        let struct_field = trivia(select! {
            Token::Identifier = e => to_kestrel_span(e.span()),
        })
        .then(
            trivia(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                .then(param_pattern.clone())
                .map(|(colon, pattern)| Some((colon, pattern)))
                .or(empty().to(None)),
        )
        .map(|(field_name, binding)| StructPatternFieldData {
            field_name,
            binding,
        });

        let struct_rest = trivia(just(Token::DotDot).map_with(|_, e| to_kestrel_span(e.span())));

        let struct_pattern = trivia(select! {
            Token::Identifier = e => to_kestrel_span(e.span()),
        })
        .then(trivia(
            just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())),
        ))
        .then(
            struct_field
                .separated_by(trivia(just(Token::Comma)))
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then(
            trivia(just(Token::Comma))
                .or_not()
                .ignore_then(struct_rest.or_not()),
        )
        .then(trivia(
            just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span())),
        ))
        .map(
            |((((struct_name, lbrace), fields), rest), rbrace)| PatternVariant::Struct {
                struct_name,
                lbrace,
                fields,
                rest,
                rbrace,
            },
        );

        // Priority: tuple first (starts with `(`), then struct (identifier + `{`),
        // then wildcard (`_`), then binding (identifier without `{`)
        tuple.or(struct_pattern).or(wildcard).or(binding).boxed()
    })
}

/// Parser for optional default value: `= expression`
///
/// # Returns
/// - `Some((equals_span, expression))` if default value is present
/// - `None` if no default value
fn default_value_parser<'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens>,
    Option<(Span, crate::expr::ExprVariant)>,
    ParserExtra<'tokens>,
> + Clone {
    // Parse optional default value: = expression
    // The trivia combinator handles whitespace before =
    // .or_not() makes the entire thing optional
    trivia(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
        .then(expr_parser())
        .or_not()
        .boxed()
}

/// Parser for a single parameter: `(access_mode)? (label)? pattern: Type (= default)?`
///
/// # Examples
/// - `x: Int` → access_mode=None, label=None, pattern=Binding(x)
/// - `with x: Int` → access_mode=None, label="with", pattern=Binding(x)
/// - `mutating x: Int` → access_mode=Mutating, label=None, pattern=Binding(x)
/// - `(a, b): (Int, Int)` → access_mode=None, label=None, pattern=Tuple
/// - `point (x, y): Point` → access_mode=None, label="point", pattern=Tuple
/// - `Point { x, y }: Point` → access_mode=None, label=None, pattern=Struct
/// - `_: Int` → access_mode=None, label=None, pattern=Wildcard
/// - `x: Int = 0` → access_mode=None, label=None, pattern=Binding(x), default=Some(0)
pub(crate) fn parameter_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ParameterData, ParserExtra<'tokens>> + Clone {
    // Parse label: identifier or keyword (keywords allowed as parameter labels)
    let ident = identifier_or_keyword();

    let param_pattern = parameter_pattern_parser();

    // Labeled parameter: (access_mode)? label pattern: Type (= default)?
    // The label is always a simple identifier, followed by a pattern
    let labeled = parameter_access_mode_parser()
        .then(ident.clone())
        .then(param_pattern.clone())
        .then(trivia(
            just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())),
        ))
        .then(ty_parser())
        .then(default_value_parser())
        .map(
            |(((((access_mode, label), pattern), colon), ty), default)| ParameterData {
                access_mode,
                label: Some(label),
                pattern,
                colon,
                ty,
                default,
            },
        );

    // Unlabeled parameter: (access_mode)? pattern: Type (= default)?
    let unlabeled = parameter_access_mode_parser()
        .then(param_pattern)
        .then(trivia(
            just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())),
        ))
        .then(ty_parser())
        .then(default_value_parser())
        .map(
            |((((access_mode, pattern), colon), ty), default)| ParameterData {
                access_mode,
                label: None,
                pattern,
                colon,
                ty,
                default,
            },
        );

    // Try labeled first (more specific), then unlabeled
    labeled.or(unlabeled).boxed()
}

/// Parser for parameter list (zero or more parameters separated by commas)
pub(crate) fn parameter_list_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<ParameterData>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(
            parameter_parser()
                .separated_by(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())))
                .allow_trailing()
                .collect(),
        )
        .boxed()
}

/// Parser for optional return type: `-> Type`
///
/// # Returns
/// - `Some((arrow_span, type))` if return type is present
/// - `None` if no return type
pub(crate) fn return_type_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<(Span, TyVariant)>, ParserExtra<'tokens>> + Clone
{
    skip_trivia()
        .ignore_then(just(Token::Arrow).map_with(|_, e| to_kestrel_span(e.span())))
        .then(ty_parser())
        .map(|(arrow, ty)| (arrow, ty))
        .or_not()
        .boxed()
}

/// Parser for optional function body (block or expression)
///
/// # Syntax
/// - Block body: `{ statements; expr }`
/// - Expression body: `= expr`
/// - No body (protocol methods)
///
/// # Returns
/// - `Some(FunctionBodyData::Block(..))` if block body present
/// - `Some(FunctionBodyData::Expression(..))` if expression body present
/// - `None` if no body (e.g., protocol method declarations)
pub fn function_body_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<FunctionBodyData>, ParserExtra<'tokens>> + Clone
{
    // Expression body: `= expr`
    let expression_body = token(Token::Equals)
        .then(expr_parser())
        .map(|(eq_span, expr)| FunctionBodyData::Expression(eq_span, expr));

    // Block body: `{ ... }`
    let block_body = code_block_parser().map(FunctionBodyData::Block);

    // Try expression body first, then block body, then nothing
    expression_body
        .or(block_body)
        .map(Some)
        .or(empty().to(None))
        .boxed()
}

/// Parser for optional block-only body (code block)
///
/// Used by initializers which only support block bodies, not expression bodies.
///
/// # Returns
/// - `Some(CodeBlockData)` if body is present
/// - `None` if no body (e.g., protocol initializer declarations)
pub fn block_body_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<CodeBlockData>, ParserExtra<'tokens>> + Clone {
    code_block_parser().map(Some).or(empty().to(None)).boxed()
}

// =============================================================================
// Declaration Parsers - Single Source of Truth
// =============================================================================

/// Parser for an init effect modifier: `?` or `throws Type`
fn init_effect_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<InitEffect>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(
            just(Token::Question)
                .map_with(|_, e| InitEffect::Failable(to_kestrel_span(e.span())))
                .or(just(Token::Throws)
                    .map_with(|_, e| to_kestrel_span(e.span()))
                    .then(ty_parser())
                    .map(|(throws_span, err_ty)| InitEffect::Throwing(throws_span, err_ty))),
        )
        .or_not()
        .boxed()
}

/// Parser for an initializer declaration
///
/// Syntax: `(@attr)* (visibility)? init(params)(? | throws E)? { body }?`
/// Body is optional for protocol initializer declarations.
///
/// This is the single source of truth for initializer declaration parsing.
pub fn initializer_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, InitializerDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(token(Token::Init))
        .then(type_parameter_list_parser().or_not())
        .then(token(Token::LParen))
        .then(parameter_list_parser())
        .then(token(Token::RParen))
        .then(init_effect_parser())
        .then(where_clause_parser().or_not())
        .then(block_body_parser())
        .map(
            |(
                (
                    (
                        (
                            (
                                ((((attributes, visibility), init_span), type_params), lparen),
                                parameters,
                            ),
                            rparen,
                        ),
                        effect,
                    ),
                    where_clause,
                ),
                body,
            )| {
                InitializerDeclarationData {
                    attributes,
                    visibility,
                    init_span,
                    type_params,
                    lparen,
                    parameters,
                    rparen,
                    effect,
                    where_clause,
                    body,
                }
            },
        )
        .boxed()
}

/// Parser for a deinitializer declaration
///
/// Syntax: `deinit { body }`
/// Deinit blocks are used for RAII-style cleanup when a value goes out of scope.
/// They have no parameters, attributes, or visibility modifiers.
///
/// This is the single source of truth for deinit declaration parsing.
pub fn deinit_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, DeinitDeclarationData, ParserExtra<'tokens>> + Clone {
    token(Token::Deinit)
        .then(code_block_parser())
        .map(|(deinit_span, body)| DeinitDeclarationData { deinit_span, body })
        .boxed()
}
