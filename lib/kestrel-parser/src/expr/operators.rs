//! Operator token parsers (unary, binary, compound assignment).
//!
//! These are token-level recognisers that don't capture the recursive `expr`
//! handle, so they live standalone. `expr_parser` composes them with its
//! operand parsers to build the flat operator tree the Pratt parser will
//! later restructure.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::common::skip_trivia;
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

/// Prefix unary operators: `-`, `+`, `!` (bitwise-not), `not` (logical-not).
pub(super) fn unary_op_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (Token, Span), ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(
            just(Token::Minus)
                .map_with(|tok, e| (tok, to_kestrel_span(e.span())))
                .or(just(Token::Plus).map_with(|tok, e| (tok, to_kestrel_span(e.span()))))
                .or(just(Token::Bang).map_with(|tok, e| (tok, to_kestrel_span(e.span()))))
                .or(just(Token::Not).map_with(|tok, e| (tok, to_kestrel_span(e.span())))),
        )
        .boxed()
}

/// Binary operators (arithmetic, bitwise, shift, comparison, logical, range, coalesce).
///
/// Operator precedence and associativity are intentionally NOT applied here —
/// the parser preserves source order and the later Pratt pass restructures.
pub(super) fn binary_op_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (Token, Span), ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(select! {
            Token::Plus = e => (Token::Plus, to_kestrel_span(e.span())),
            Token::Minus = e => (Token::Minus, to_kestrel_span(e.span())),
            Token::Star = e => (Token::Star, to_kestrel_span(e.span())),
            Token::Slash = e => (Token::Slash, to_kestrel_span(e.span())),
            Token::Percent = e => (Token::Percent, to_kestrel_span(e.span())),
            Token::Ampersand = e => (Token::Ampersand, to_kestrel_span(e.span())),
            Token::Pipe = e => (Token::Pipe, to_kestrel_span(e.span())),
            Token::Caret = e => (Token::Caret, to_kestrel_span(e.span())),
            Token::LessLess = e => (Token::LessLess, to_kestrel_span(e.span())),
            Token::GreaterGreater = e => (Token::GreaterGreater, to_kestrel_span(e.span())),
            Token::Less = e => (Token::Less, to_kestrel_span(e.span())),
            Token::Greater = e => (Token::Greater, to_kestrel_span(e.span())),
            Token::LessEquals = e => (Token::LessEquals, to_kestrel_span(e.span())),
            Token::GreaterEquals = e => (Token::GreaterEquals, to_kestrel_span(e.span())),
            Token::EqualsEquals = e => (Token::EqualsEquals, to_kestrel_span(e.span())),
            Token::BangEquals = e => (Token::BangEquals, to_kestrel_span(e.span())),
            Token::And = e => (Token::And, to_kestrel_span(e.span())),
            Token::Or = e => (Token::Or, to_kestrel_span(e.span())),
            Token::QuestionQuestion = e => (Token::QuestionQuestion, to_kestrel_span(e.span())),
            Token::DotDotEquals = e => (Token::DotDotEquals, to_kestrel_span(e.span())),
            Token::DotDotLess = e => (Token::DotDotLess, to_kestrel_span(e.span())),
        })
        .boxed()
}

/// Compound assignment operators: `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`.
pub(super) fn compound_assign_op_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (Token, Span), ParserExtra<'tokens>> + Clone {
    choice((
        just(Token::PlusEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::MinusEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::StarEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::SlashEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::PercentEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::AmpersandEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::PipeEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::CaretEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::LessLessEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
        just(Token::GreaterGreaterEquals).map_with(|t, e| (t, to_kestrel_span(e.span()))),
    ))
    .boxed()
}
