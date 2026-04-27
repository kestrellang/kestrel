//! Simple control-flow expressions that only need access to the recursive
//! `expr` handle (no shared sub-parsers like code blocks or conditions).
//!
//! - `break [label]` / `continue [label]` — label-only, no `expr` needed.
//! - `return [expr]` / `throw expr` — take the recursive `expr` handle.
//! - `try` keyword — standalone span; postfix connection happens in
//!   `expr_parser`.
//!
//! `if` / `while` / `loop` / `for` / `match` live in `expr_parser` because
//! they also need the code-block / condition / pattern plumbing that is
//! built up inside the `recursive(|expr| ...)` closure.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::common::skip_trivia;
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

use super::data::ExprVariant;

/// Parser for `break` or `break label`.
pub(super) fn break_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Break).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            skip_trivia()
                .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
                .or_not(),
        )
        .map(|(break_span, label)| ExprVariant::Break { break_span, label })
        .boxed()
}

/// Parser for `continue` or `continue label`.
pub(super) fn continue_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Continue).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            skip_trivia()
                .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
                .or_not(),
        )
        .map(|(continue_span, label)| ExprVariant::Continue {
            continue_span,
            label,
        })
        .boxed()
}

/// Parser for `return` or `return expr`.
pub(super) fn return_parser<'tokens, P>(
    expr: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    skip_trivia()
        .ignore_then(just(Token::Return).map_with(|_, e| to_kestrel_span(e.span())))
        .then(expr.map(Box::new).or_not())
        .map(|(return_span, value)| ExprVariant::Return { return_span, value })
        .boxed()
}

/// Parser for `throw expr`.
///
/// The expression after `throw` is parsed via `.or_not()` so a half-typed
/// `throw` (no expression yet) doesn't poison the enclosing block — without
/// this, a function body like `{ throw\n}` couldn't reach its closing `}`
/// and the editor saw a phantom "expected `}`" on the opening brace. A
/// peek at the next non-trivia token gives the diagnostic an anchor at the
/// position where the expression *should* have appeared (matching where
/// chumsky's old "found `<token>`" error used to land), instead of pinning
/// the squiggle on the rule's start.
pub(super) fn throw_parser<'tokens, P>(
    expr: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let next_token_span =
        skip_trivia().ignore_then(any().rewind().map_with(|_, e| to_kestrel_span(e.span())));
    skip_trivia()
        .ignore_then(just(Token::Throw).map_with(|_, e| to_kestrel_span(e.span())))
        .then(expr.map(Box::new).or_not())
        .then(next_token_span.or_not())
        .validate(|((throw_span, value), next_span), e, emitter| {
            if value.is_none() {
                let span = next_span
                    .map(|s| chumsky::span::SimpleSpan::new((), s.start..s.end))
                    .unwrap_or_else(|| e.span());
                emitter.emit(chumsky::error::Rich::custom(
                    span,
                    "expected expression after `throw`",
                ));
            }
            (throw_span, value)
        })
        .map(|(throw_span, value)| ExprVariant::Throw { throw_span, value })
        .boxed()
}

/// Parser for the `try` keyword alone. `expr_parser` wraps this around a
/// postfix expression so `try` binds with high precedence.
pub(super) fn try_keyword_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Try).map_with(|_, e| to_kestrel_span(e.span())))
        .boxed()
}

/// Parser for a loop label: `name:`.
pub(super) fn label_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, super::data::LabelData, ParserExtra<'tokens>> + Clone
{
    skip_trivia()
        .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
        .then(
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(|(name, colon)| super::data::LabelData { name, colon })
        .boxed()
}
