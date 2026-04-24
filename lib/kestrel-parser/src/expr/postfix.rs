//! Postfix expression parsing — argument lists, member access, tuple index,
//! and the postfix `!` unwrap operator.
//!
//! Postfix operators apply to a "primary" expression and chain:
//! `foo.bar(baz)[T].0!`. This module exposes:
//!
//! - [`PostfixOp`] — the result of a single postfix step, later folded into
//!   an `ExprVariant` (Call/MemberAccess/TupleIndex/Postfix) by
//!   [`fold_postfix_ops`].
//! - [`arg_list_parser`] / [`member_access_parser`] / [`postfix_bang_parser`]
//!   / [`postfix_op_parser`] — Chumsky factories that produce `PostfixOp`s.
//! - [`fold_postfix_ops`] — pure helper that collapses a base expression
//!   plus a sequence of `PostfixOp`s into the final `ExprVariant`.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::common::{skip_inline_trivia, skip_trivia};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

use super::atom::full_type_args_parser;
use super::data::{CallArg, ExprVariant, TypeArgsData};

/// A single postfix operation applied to a base expression. The parser
/// produces a flat sequence of these; [`fold_postfix_ops`] folds them into a
/// left-associative `ExprVariant` tree.
#[derive(Debug, Clone)]
pub(super) enum PostfixOp {
    /// Function call: `(args)` — always has parens since this is parsed from
    /// `(args)` syntax. Trailing-closure-only calls are synthesized later by
    /// `attach_trailing_closures`.
    Call {
        lparen: Option<Span>,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        rparen: Option<Span>,
    },
    /// Member access: `.identifier` or `.identifier[T]`.
    MemberAccess {
        dot: Span,
        member: Span,
        type_args: Option<TypeArgsData>,
    },
    /// Tuple index: `.0`, `.1`, ...
    TupleIndex { dot: Span, index: Span },
    /// Postfix operator: `expr!`.
    PostfixOperator {
        operator: Token,
        operator_span: Span,
    },
}

/// Parser for a single call argument — labeled (`name: value`) or
/// unlabeled. Reused by the call-postfix argument list AND the
/// implicit-member-access argument list.
///
/// Labeled is tried first so the `identifier : expr` shape wins over the
/// identifier-is-an-expression fallback.
pub(super) fn argument_parser<'tokens, P>(
    expr: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, CallArg, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let labeled = skip_trivia()
        .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
        .then(
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(skip_trivia().ignore_then(expr.clone()))
        .map(|((label, colon), value)| CallArg {
            label: Some(label),
            colon: Some(colon),
            value,
        });

    let unlabeled = skip_trivia().ignore_then(expr).map(|value| CallArg {
        label: None,
        colon: None,
        value,
    });

    labeled.or(unlabeled).boxed()
}

/// Parser for a call argument list: `(arg, arg, ...)`.
///
/// Accepts both labeled (`name: value`) and unlabeled arguments and allows a
/// trailing comma. Uses `skip_inline_trivia` before the opening paren so a
/// newline between the callee and `(` is NOT consumed — preventing a line
/// break from being absorbed as part of a call.
pub(super) fn arg_list_parser<'tokens, P>(
    expr: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, PostfixOp, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    skip_inline_trivia()
        .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            argument_parser(expr)
                .separated_by(
                    skip_trivia()
                        .ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))),
                )
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(|((lparen, arguments), rparen)| PostfixOp::Call {
            lparen: Some(lparen),
            arguments,
            commas: vec![],
            rparen: Some(rparen),
        })
        .boxed()
}

/// Parser for `.identifier` / `.identifier[T]` / `.0` / `.init`.
///
/// IMPORTANT: does NOT skip leading trivia before the dot. The dot must
/// immediately follow the previous token so that `.foo` on a new line is
/// NOT treated as a member access on the previous expression.
pub(super) fn member_access_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, PostfixOp, ParserExtra<'tokens>> + Clone {
    just(Token::Dot)
        .map_with(|_, e| to_kestrel_span(e.span()))
        .then(skip_trivia().ignore_then(select! {
            Token::Identifier = e => (Token::Identifier, to_kestrel_span(e.span())),
            Token::Integer = e => (Token::Integer, to_kestrel_span(e.span())),
            Token::Init = e => (Token::Init, to_kestrel_span(e.span())),
        }))
        .then(full_type_args_parser().or_not())
        .map(|((dot, (token, span)), type_args)| match token {
            Token::Integer => PostfixOp::TupleIndex { dot, index: span },
            _ => PostfixOp::MemberAccess {
                dot,
                member: span,
                type_args,
            },
        })
        .boxed()
}

/// Parser for the postfix `!` unwrap operator.
pub(super) fn postfix_bang_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, PostfixOp, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Bang).map_with(|tok, e| (tok, to_kestrel_span(e.span()))))
        .map(|(tok, span)| PostfixOp::PostfixOperator {
            operator: tok,
            operator_span: span,
        })
        .boxed()
}

/// Combined postfix operator parser: call | member-access | postfix-bang.
pub(super) fn postfix_op_parser<'tokens, P>(
    expr: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, PostfixOp, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    arg_list_parser(expr)
        .or(member_access_parser())
        .or(postfix_bang_parser())
        .boxed()
}

/// Fold a base expression and a sequence of postfix operations into a
/// left-associative `ExprVariant` tree.
pub(super) fn fold_postfix_ops(base: ExprVariant, ops: Vec<PostfixOp>) -> ExprVariant {
    ops.into_iter().fold(base, |acc, op| match op {
        PostfixOp::Call {
            lparen,
            arguments,
            commas,
            rparen,
        } => ExprVariant::Call {
            callee: Box::new(acc),
            lparen,
            arguments,
            commas,
            rparen,
        },
        PostfixOp::MemberAccess {
            dot,
            member,
            type_args,
        } => ExprVariant::MemberAccess {
            base: Box::new(acc),
            dot,
            member,
            type_args,
        },
        PostfixOp::TupleIndex { dot, index } => ExprVariant::TupleIndex {
            base: Box::new(acc),
            dot,
            index,
        },
        PostfixOp::PostfixOperator {
            operator,
            operator_span,
        } => ExprVariant::Postfix {
            operand: Box::new(acc),
            operator,
            operator_span,
        },
    })
}
