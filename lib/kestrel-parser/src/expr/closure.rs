//! Closure expression parsing plus trailing-closure argument handling.
//!
//! Exposes two factories that `expr_parser` composes into its recursive
//! chain:
//!
//! - [`closure_parser`] — builds a `{ params in body }` closure parser.
//!   Accepts the `{` parser as an argument so the caller can choose between
//!   trivia-skipping and inline-trivia (no newline) variants — the inline
//!   variant is what lets trailing closures chain without being ambiguous
//!   with a new block on the next line.
//! - [`trailing_closure_arg_parser`] — builds a parser for a trailing
//!   closure argument (optionally with a label).
//!
//! Both factories take the recursive `expr` handle and the inline
//! variable-declaration parser as arguments, since the closure body can
//! contain either full expressions or inline `let`/`var` statements.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::block::{BlockItem, ElseBlockItem, GuardLetData};
use crate::common::{skip_inline_trivia, skip_trivia};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::stmt::StmtVariant;
use crate::ty::ty_parser;

use super::data::{CallArg, ClosureParamData, ClosureParamsData, ExprVariant, IfCondition};
use super::is_inline_statement_like;

/// Build a closure-expression parser given:
///
/// - `lbrace_parser`: recognises the opening `{` (typically either a normal
///   skip-trivia variant for primary closures or an inline-trivia variant
///   for trailing closures, so that a brace on a new line after a callee is
///   NOT parsed as a trailing closure).
/// - `expr`: the recursive expression parser.
/// - `inline_var_decl`: inline `let`/`var` statement parser, used for
///   statements inside the closure body.
pub(super) fn closure_parser<'tokens, L, P, V>(
    lbrace_parser: L,
    expr: P,
    inline_var_decl: V,
) -> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone
where
    L: Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
    V: Parser<'tokens, ParserInput<'tokens>, StmtVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let closure_param = skip_trivia()
        .ignore_then(crate::common::parsers::parameter_pattern_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                .then(ty_parser())
                .or_not(),
        )
        .map(|(pattern, ty_opt)| {
            let (colon, ty) = match ty_opt {
                Some((c, t)) => (Some(c), Some(t)),
                None => (None, None),
            };
            ClosureParamData { pattern, colon, ty }
        });

    let closure_params = skip_trivia()
        .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            closure_param
                .separated_by(
                    skip_trivia()
                        .ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))),
                )
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())))
        .then_ignore(skip_trivia())
        .then(just(Token::In).map_with(|_, e| to_kestrel_span(e.span())))
        .map(|(((lparen, params), rparen), in_span)| {
            (
                Some(ClosureParamsData {
                    lparen,
                    params,
                    commas: vec![],
                    rparen,
                }),
                Some(in_span),
            )
        });

    let expr_for_closure = expr.clone();
    let expr_for_closure_guard = expr.clone();
    let expr_for_closure_else = expr;

    // Inline else block items parser for guard-let in closures
    let closure_else_item = inline_var_decl
        .clone()
        .map(ElseBlockItem::Statement)
        .or(expr_for_closure_else
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                    .map(Some)
                    .or(empty().to(None)),
            )
            .try_map(|(e, maybe_semi), _extra| {
                if let Some(semi) = maybe_semi {
                    Ok(ElseBlockItem::Statement(StmtVariant::Expression(e, semi)))
                } else if is_inline_statement_like(&e) {
                    Ok(ElseBlockItem::StatementExpr(e))
                } else {
                    Err(Rich::custom(
                        chumsky::span::Span::new((), 0..0),
                        "expected semicolon",
                    ))
                }
            }));

    let closure_else_items = closure_else_item
        .repeated()
        .collect::<Vec<_>>()
        .then(
            expr_for_closure_else
                .map(ElseBlockItem::TrailingExpression)
                .or_not(),
        )
        .map(|(mut items, trailing)| {
            if let Some(e) = trailing {
                items.push(e);
            }
            items
        });

    // Inline guard-let parser for closures with chain support.
    // Single let condition: `let pattern = expr`.
    let closure_guard_let_condition = skip_trivia()
        .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
        .then(crate::pattern::pattern_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(expr_for_closure_guard.clone())
        .map(
            |(((let_span, pattern), equals_span), value)| IfCondition::Let {
                let_span,
                pattern,
                equals_span,
                value,
            },
        );

    // Single condition: either let-binding or boolean expression.
    let closure_guard_single_condition = closure_guard_let_condition
        .clone()
        .or(expr_for_closure_guard.clone().map(IfCondition::Expr));

    // Condition list: first must be let, followed by comma-separated conditions.
    let closure_guard_conditions = closure_guard_let_condition
        .then(
            skip_trivia()
                .ignore_then(just(Token::Comma))
                .ignore_then(closure_guard_single_condition)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut conditions = vec![first];
            conditions.extend(rest);
            conditions
        });

    let closure_guard_let = skip_trivia()
        .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
        .then(closure_guard_conditions)
        .then(
            skip_trivia().ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(closure_else_items)
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |(
                ((((guard_span, conditions), else_span), else_lbrace), else_items),
                else_rbrace,
            )| {
                BlockItem::GuardLet(GuardLetData {
                    guard_span,
                    conditions,
                    else_span,
                    else_lbrace,
                    else_items,
                    else_rbrace,
                })
            },
        );

    let closure_block_item = closure_guard_let
        .or(inline_var_decl.map(BlockItem::Statement))
        .or(expr_for_closure
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                    .map(Some)
                    .or(empty().to(None)),
            )
            .try_map(|(e, maybe_semi), _extra| {
                if let Some(semi) = maybe_semi {
                    Ok(BlockItem::Statement(StmtVariant::Expression(e, semi)))
                } else if is_inline_statement_like(&e) {
                    Ok(BlockItem::StatementExpr(e))
                } else {
                    Err(Rich::custom(
                        chumsky::span::Span::new((), 0..0),
                        "expected semicolon",
                    ))
                }
            }));

    lbrace_parser
        .then(
            closure_params
                .or_not()
                .map(|opt| opt.unwrap_or((None, None))),
        )
        .then(
            closure_block_item
                .repeated()
                .collect::<Vec<_>>()
                .then(expr_for_closure.map(BlockItem::TrailingExpression).or_not())
                .map(|(mut statements, trailing)| {
                    if let Some(expr) = trailing {
                        statements.push(expr);
                    }
                    statements
                }),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |(((lbrace, (params, in_span)), body), rbrace)| ExprVariant::Closure {
                lbrace,
                params,
                in_span,
                body,
                rbrace,
            },
        )
        .boxed()
}

/// Build a parser for a trailing-closure argument: either `{ ... }` or
/// `label: { ... }`. Takes the inline-trivia closure parser so the caller
/// passes in whichever variant matches its context.
pub(super) fn trailing_closure_arg_parser<'tokens, C>(
    closure_inline: C,
) -> impl Parser<'tokens, ParserInput<'tokens>, CallArg, ParserExtra<'tokens>> + Clone
where
    C: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let labeled = skip_inline_trivia()
        .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
        .then(
            skip_inline_trivia()
                .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(closure_inline.clone())
        .map(|((label, colon), closure)| CallArg {
            label: Some(label),
            colon: Some(colon),
            value: closure,
        });

    let unlabeled = closure_inline.map(|closure| CallArg {
        label: None,
        colon: None,
        value: closure,
    });

    labeled.or(unlabeled).boxed()
}
