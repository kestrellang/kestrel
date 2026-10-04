//! Expressions.
//!
//! ```text
//! expr      = binary (('=' | op'=') expr)?            // ExprAssignment / ExprCompoundAssignment
//! binary    = operand (binop operand)*                // ExprBinary, left-folded
//! operand   = 'try' postfix                           // ExprTry
//!           | unop+ postfix                           // ExprUnary
//!           | postfix
//! postfix   = stmt_like tight*                        // if/while/loop/for/match/return/throw
//!           | primary tight* trailing* ('.' member tight*)*
//! tight     = '(' args ')'                            // no line break before `(`
//!           | '.' member                              // `.` touching the operand
//!           | '!' | '..'
//! trailing  = ('Identifier' ':')? closure             // same line as the call
//! ```
//!
//! Every expression node is wrapped as `Expression > ExprX`. Member access
//! extends a path: `a.b().c.d` is `ExprPath[Expression(ExprCall(…)) . c . d]`.
//!
//! **Condition mode** (if/while/for/match heads, match guards, `if let`
//! values) is the same grammar with a restriction: no trailing closures, no
//! assignment, no block-like primaries (`if`, closures, …), and a single
//! prefix operator. Parenthesised and bracketed sub-expressions lift the
//! restriction.

use kestrel_syntax_tree::SyntaxKind as K;

use super::blocks::{self, BlockFlavor};
use super::patterns::{at_param_pattern_start, param_pattern, pattern};
use super::types::{try_type_argument_list, ty};
use super::{delimited, is_label_keyword, matching_close};
use crate::core::{CompletedMarker, Marker, Parser};
use crate::syntax_error::{SyntaxError, codes};

/// A parsed expression: its `Expression` node and the kind inside it.
#[derive(Clone, Copy)]
pub(super) struct ExprInfo {
    pub cm: CompletedMarker,
    pub kind: K,
}

/// Tokens that can begin an expression.
pub(super) fn at_expr_start(p: &Parser<'_>) -> bool {
    p.at_any(&[
        K::Integer,
        K::Float,
        K::String,
        K::RawString,
        K::Char,
        K::Boolean,
        K::Null,
        K::LBracket,
        K::LParen,
        K::LBrace,
        K::Dot,
        K::Identifier,
        K::Break,
        K::Continue,
        K::If,
        K::While,
        K::Loop,
        K::For,
        K::Match,
        K::Return,
        K::Throw,
        K::Try,
        K::Minus,
        K::Plus,
        K::Bang,
        K::Not,
        K::DotDotLess,
        K::DotDotEquals,
        K::Ampersand,
    ])
}

/// A full expression. Reports "expected expression" when none starts here.
pub(super) fn expr(p: &mut Parser<'_>) -> Option<ExprInfo> {
    let lhs = binary(p, false)?;
    let compound = is_compound_assign(p.current());
    if !(p.at(K::Equals) || compound) {
        return Some(lhs);
    }
    let kind = if compound {
        K::ExprCompoundAssignment
    } else {
        K::ExprAssignment
    };
    Some(wrap(p, lhs.cm, kind, |p| {
        p.bump_any();
        expr(p);
    }))
}

/// A condition expression (no assignment, no trailing closures).
pub(super) fn cond_expr(p: &mut Parser<'_>) -> Option<ExprInfo> {
    binary(p, true)
}

fn is_compound_assign(kind: Option<K>) -> bool {
    matches!(
        kind,
        Some(
            K::PlusEquals
                | K::MinusEquals
                | K::StarEquals
                | K::SlashEquals
                | K::PercentEquals
                | K::AmpersandEquals
                | K::PipeEquals
                | K::CaretEquals
                | K::LessLessEquals
                | K::GreaterGreaterEquals
        )
    )
}

fn is_binary_op(kind: Option<K>) -> bool {
    matches!(
        kind,
        Some(
            K::Plus
                | K::Minus
                | K::Star
                | K::Slash
                | K::Percent
                | K::Ampersand
                | K::Pipe
                | K::Caret
                | K::LessLess
                | K::GreaterGreater
                | K::Less
                | K::Greater
                | K::LessEquals
                | K::GreaterEquals
                | K::EqualsEquals
                | K::BangEquals
                | K::And
                | K::Or
                | K::QuestionQuestion
                | K::DotDotEquals
                | K::DotDotLess
        )
    )
}

fn is_unary_op(kind: Option<K>) -> bool {
    matches!(
        kind,
        Some(
            K::Minus
                | K::Plus
                | K::Bang
                | K::Not
                | K::DotDotLess
                | K::DotDotEquals
                | K::Ampersand
        )
    )
}

/// Wrap a completed `Expression` as `Expression > kind > (Expression, …)`.
fn wrap(
    p: &mut Parser<'_>,
    inner: CompletedMarker,
    kind: K,
    rest: impl FnOnce(&mut Parser<'_>),
) -> ExprInfo {
    let m = inner.precede(p);
    rest(p);
    let c = m.complete(p, kind);
    ExprInfo {
        cm: c.precede(p).complete(p, K::Expression),
        kind,
    }
}

/// Start `Expression > kind`; returns both markers (outer first).
fn start2(p: &mut Parser<'_>) -> (Marker, Marker) {
    let e = p.start();
    let k = p.start();
    (e, k)
}

fn finish2(p: &mut Parser<'_>, (e, k): (Marker, Marker), kind: K) -> ExprInfo {
    k.complete(p, kind);
    ExprInfo {
        cm: e.complete(p, K::Expression),
        kind,
    }
}

fn binary(p: &mut Parser<'_>, cond: bool) -> Option<ExprInfo> {
    let mut lhs = operand(p, cond)?;
    while is_binary_op(p.current()) {
        lhs = wrap(p, lhs.cm, K::ExprBinary, |p| {
            p.bump_any();
            if operand(p, cond).is_none() {
                // operand() already reported
            }
        });
    }
    Some(lhs)
}

fn operand(p: &mut Parser<'_>, cond: bool) -> Option<ExprInfo> {
    if p.at(K::Try) {
        let m = start2(p);
        p.bump(K::Try);
        postfix(p, cond);
        return Some(finish2(p, m, K::ExprTry));
    }
    if is_unary_op(p.current()) {
        return Some(unary(p, cond));
    }
    postfix(p, cond)
}

/// One prefix operator applied to its operand. Outside conditions operators
/// stack (`--x`); in a condition exactly one is allowed.
fn unary(p: &mut Parser<'_>, cond: bool) -> ExprInfo {
    let m = start2(p);
    let is_amp = p.at(K::Ampersand);
    p.bump_any();
    if is_amp {
        p.eat(K::Mutating);
    }
    if !cond && is_unary_op(p.current()) {
        unary(p, cond);
    } else {
        postfix(p, cond);
    }
    finish2(p, m, K::ExprUnary)
}

// ----- postfix chains ----------------------------------------------------------

/// The operand being extended by postfix operators.
enum Chain {
    /// A finished `Expression` whose inner node is `kind`.
    Done(CompletedMarker, K),
    /// An open `ExprPath` (path or member access); `outer` is the
    /// `Expression` started before it, if any.
    Path { path: Marker, outer: Option<Marker> },
    /// An open `ExprCall` whose `ArgumentList` is still open, so trailing
    /// closures can join it.
    Call { call: Marker, args: Marker },
}

fn close(p: &mut Parser<'_>, chain: Chain) -> ExprInfo {
    match chain {
        Chain::Done(cm, kind) => ExprInfo { cm, kind },
        Chain::Path { path, outer } => {
            let c = path.complete(p, K::ExprPath);
            let cm = match outer {
                Some(o) => o.complete(p, K::Expression),
                None => c.precede(p).complete(p, K::Expression),
            };
            ExprInfo {
                cm,
                kind: K::ExprPath,
            }
        },
        Chain::Call { call, args } => {
            args.complete(p, K::ArgumentList);
            let c = call.complete(p, K::ExprCall);
            ExprInfo {
                cm: c.precede(p).complete(p, K::Expression),
                kind: K::ExprCall,
            }
        },
    }
}

fn postfix(p: &mut Parser<'_>, cond: bool) -> Option<ExprInfo> {
    if cond {
        let chain = cond_primary(p)?;
        let chain = tight_ops(p, chain, true);
        return Some(close(p, chain));
    }
    if at_stmt_like(p) {
        let info = stmt_like(p);
        let chain = tight_ops(p, Chain::Done(info.cm, info.kind), false);
        return Some(close(p, chain));
    }
    let chain = primary(p)?;
    let mut chain = tight_ops(p, chain, false);
    chain = trailing_closures(p, chain);
    // Continuation: `.member` on a later line, followed by tight operators.
    while p.at(K::Dot) {
        chain = member(p, chain, true);
        chain = tight_ops(p, chain, false);
    }
    Some(close(p, chain))
}

/// Calls, adjacent member accesses, `!` and `..`. In a condition, `.`
/// may also follow a line break and `init` is not a member name.
fn tight_ops(p: &mut Parser<'_>, mut chain: Chain, cond: bool) -> Chain {
    loop {
        match p.current() {
            Some(K::LParen) if !p.nl_before() => chain = call(p, chain),
            Some(K::Dot) if cond || p.joined_at(p.token_pos()) => {
                chain = member(p, chain, !cond);
            },
            Some(K::Bang | K::DotDot) => {
                let info = close(p, chain);
                let w = wrap(p, info.cm, K::ExprPostfix, |p| p.bump_any());
                chain = Chain::Done(w.cm, w.kind);
            },
            _ => return chain,
        }
    }
}

/// `.name[args]?` (extends a path), `.0` (tuple index), or a bare `.`.
fn member(p: &mut Parser<'_>, chain: Chain, allow_init: bool) -> Chain {
    if p.nth_at(1, K::Integer) {
        let info = close(p, chain);
        let w = wrap(p, info.cm, K::ExprTupleIndex, |p| {
            p.bump(K::Dot);
            p.bump(K::Integer);
        });
        return Chain::Done(w.cm, w.kind);
    }
    let chain = match chain {
        Chain::Path { .. } => chain,
        other => {
            let info = close(p, other);
            Chain::Path {
                path: info.cm.precede(p),
                outer: None,
            }
        },
    };
    p.bump(K::Dot);
    match p.current() {
        Some(K::Identifier) => p.bump(K::Identifier),
        Some(K::Init) if allow_init => p.bump_as(K::Identifier),
        _ => {
            let range = p.prev_range().unwrap_or_else(|| p.error_range());
            p.push_error(SyntaxError::new(
                codes::EXPECTED_MEMBER_NAME,
                "expected identifier after `.`",
                range,
            ));
            return chain;
        },
    }
    if p.at(K::LBracket) {
        try_type_argument_list(p);
    }
    chain
}

/// `(args)` applied to the chain.
fn call(p: &mut Parser<'_>, chain: Chain) -> Chain {
    let info = close(p, chain);
    let call = info.cm.precede(p);
    let args = p.start();
    argument_list_body(p);
    Chain::Call { call, args }
}

/// `( arg, label: arg, … )` — the caller has opened the `ArgumentList`.
fn argument_list_body(p: &mut Parser<'_>) {
    delimited(p, K::LParen, K::RParen, true, |p| {
        if !at_expr_start(p) && !at_label(p) {
            p.error_expected_what("expression");
            return false;
        }
        argument(p);
        true
    });
}

/// `label:` at the cursor (an identifier or keyword followed by `:`).
fn at_label(p: &Parser<'_>) -> bool {
    p.nth(0)
        .is_some_and(|k| k == K::Identifier || is_label_keyword(k))
        && p.nth_at(1, K::Colon)
}

fn argument(p: &mut Parser<'_>) {
    let m = p.start();
    if at_label(p) {
        p.bump_as(K::Identifier);
        p.bump(K::Colon);
    }
    expr(p);
    m.complete(p, K::Argument);
}

/// A trailing closure follows on the same line: `{ … }` or `label: { … }`.
fn at_trailing_closure(p: &Parser<'_>) -> bool {
    if p.at(K::LBrace) {
        return !p.nl_before();
    }
    p.at(K::Identifier)
        && !p.nl_before()
        && p.nth_at(1, K::Colon)
        && !p.nth_nl_before(1)
        && p.nth_at(2, K::LBrace)
        && !p.nth_nl_before(2)
}

/// Attach trailing closures. They join an open call's argument list (after
/// its `)`, in source order) or turn a path into a call; other operands take
/// none.
fn trailing_closures(p: &mut Parser<'_>, chain: Chain) -> Chain {
    if !at_trailing_closure(p) {
        return chain;
    }
    let (call, args) = match chain {
        Chain::Done(..) => return chain,
        Chain::Call { call, args } => (call, args),
        path @ Chain::Path { .. } => {
            let info = close(p, path);
            (info.cm.precede(p), p.start())
        },
    };
    while at_trailing_closure(p) {
        let a = p.start();
        if p.at(K::Identifier) {
            p.bump(K::Identifier);
            p.bump(K::Colon);
        }
        closure(p);
        a.complete(p, K::Argument);
    }
    Chain::Call { call, args }
}

// ----- primaries -------------------------------------------------------------

/// `label: while|loop|for` or a block-like keyword at the cursor.
fn at_stmt_like(p: &Parser<'_>) -> bool {
    match p.current() {
        Some(K::If | K::While | K::Loop | K::For | K::Match | K::Return | K::Throw) => true,
        Some(K::Identifier) => {
            p.nth_at(1, K::Colon) && p.at_any_nth(2, &[K::While, K::Loop, K::For])
        },
        _ => false,
    }
}

fn stmt_like(p: &mut Parser<'_>) -> ExprInfo {
    match p.current() {
        Some(K::If) => if_expr(p),
        Some(K::Match) => match_expr(p),
        Some(K::Return | K::Throw) => return_or_throw(p),
        _ => loop_expr(p),
    }
}

fn primary(p: &mut Parser<'_>) -> Option<Chain> {
    let info = match p.current() {
        Some(
            K::Integer | K::Float | K::String | K::RawString | K::Char | K::Boolean | K::Null,
        ) => literal(p),
        Some(K::LBracket) => array_or_dict(p),
        Some(K::LParen) => paren(p),
        Some(K::Break | K::Continue) => jump(p),
        Some(K::LBrace) => closure(p),
        Some(K::Dot) => implicit_member(p),
        Some(K::Identifier) => return Some(path(p)),
        _ => {
            p.error_expected_what("expression");
            return None;
        },
    };
    Some(Chain::Done(info.cm, info.kind))
}

fn cond_primary(p: &mut Parser<'_>) -> Option<Chain> {
    let info = match p.current() {
        Some(
            K::Integer | K::Float | K::String | K::RawString | K::Char | K::Boolean | K::Null,
        ) => literal(p),
        Some(K::LBracket) => array_or_dict(p),
        Some(K::LParen) => paren(p),
        Some(K::Dot) => implicit_member(p),
        Some(K::Identifier) => return Some(path(p)),
        _ => {
            p.error_expected_what("expression");
            return None;
        },
    };
    Some(Chain::Done(info.cm, info.kind))
}

fn literal(p: &mut Parser<'_>) -> ExprInfo {
    let kind = match p.current() {
        Some(K::Integer) => K::ExprInteger,
        Some(K::Float) => K::ExprFloat,
        Some(K::String) => K::ExprString,
        Some(K::RawString) => K::ExprRawString,
        Some(K::Char) => K::ExprChar,
        Some(K::Boolean) => K::ExprBool,
        _ => K::ExprNull,
    };
    let m = start2(p);
    p.bump_any();
    finish2(p, m, kind)
}

/// `name[args]? (. name[args]?)*` — dots may follow line breaks.
fn path(p: &mut Parser<'_>) -> Chain {
    let outer = p.start();
    let path = p.start();
    p.bump(K::Identifier);
    if p.at(K::LBracket) {
        try_type_argument_list(p);
    }
    while p.at(K::Dot) && p.nth_at(1, K::Identifier) {
        p.bump(K::Dot);
        p.bump(K::Identifier);
        if p.at(K::LBracket) {
            try_type_argument_list(p);
        }
    }
    Chain::Path {
        path,
        outer: Some(outer),
    }
}

/// `[]`, `[:]`, `[a, b]`, `[k: v, …]`.
fn array_or_dict(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    p.bump(K::LBracket);
    if p.at(K::Colon) && p.nth_at(1, K::RBracket) {
        p.bump(K::Colon);
        p.bump(K::RBracket);
        return finish2(p, m, K::ExprDictionary);
    }
    if p.eat(K::RBracket) {
        return finish2(p, m, K::ExprArray);
    }
    let Some(first) = expr(p) else {
        p.err_recover_balanced(|p| p.at(K::RBracket));
        p.eat(K::RBracket);
        return finish2(p, m, K::ExprArray);
    };
    if p.at(K::Colon) {
        let entry = first.cm.precede(p);
        p.bump(K::Colon);
        expr(p);
        entry.complete(p, K::DictionaryEntry);
        while p.eat(K::Comma) {
            if p.at(K::RBracket) {
                break;
            }
            let entry = p.start();
            expr(p);
            p.expect(K::Colon);
            expr(p);
            entry.complete(p, K::DictionaryEntry);
        }
        p.expect(K::RBracket);
        return finish2(p, m, K::ExprDictionary);
    }
    while p.eat(K::Comma) {
        if p.at(K::RBracket) {
            break;
        }
        if expr(p).is_none() {
            break;
        }
    }
    p.expect(K::RBracket);
    finish2(p, m, K::ExprArray)
}

/// `()`, `(e)`, `(e,)`, `(e, f)`.
fn paren(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    p.bump(K::LParen);
    if p.eat(K::RParen) {
        return finish2(p, m, K::ExprUnit);
    }
    expr(p);
    if !p.at(K::Comma) {
        p.expect(K::RParen);
        return finish2(p, m, K::ExprGrouping);
    }
    while p.eat(K::Comma) {
        if p.at(K::RParen) {
            break;
        }
        if expr(p).is_none() {
            break;
        }
    }
    p.expect(K::RParen);
    finish2(p, m, K::ExprTuple)
}

/// `break label?` / `continue label?`.
fn jump(p: &mut Parser<'_>) -> ExprInfo {
    let kind = if p.at(K::Break) {
        K::ExprBreak
    } else {
        K::ExprContinue
    };
    let m = start2(p);
    p.bump_any();
    p.eat(K::Identifier);
    finish2(p, m, kind)
}

/// `return expr?` / `throw expr`.
fn return_or_throw(p: &mut Parser<'_>) -> ExprInfo {
    let is_throw = p.at(K::Throw);
    let m = start2(p);
    p.bump_any();
    if at_expr_start(p) {
        expr(p);
    } else if is_throw {
        let range = p.error_range();
        p.push_error(SyntaxError::new(
            codes::THROW_WITHOUT_VALUE,
            "expected expression after `throw`",
            range,
        ));
    }
    finish2(p, m, if is_throw { K::ExprThrow } else { K::ExprReturn })
}

/// `.Case` or `.Case(args)`.
fn implicit_member(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    p.bump(K::Dot);
    if p.at(K::Identifier) {
        let n = p.start();
        p.bump(K::Identifier);
        n.complete(p, K::Name);
    } else {
        let range = p.prev_range().unwrap_or_else(|| p.error_range());
        p.push_error(SyntaxError::new(
            codes::EXPECTED_MEMBER_NAME,
            "expected identifier after `.`",
            range,
        ));
    }
    if p.at(K::LParen) && !p.nl_before() {
        let args = p.start();
        argument_list_body(p);
        args.complete(p, K::ArgumentList);
    }
    finish2(p, m, K::ExprImplicitMemberAccess)
}

/// `{ (params) in body }` or `{ body }`.
pub(super) fn closure(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    p.bump(K::LBrace);
    if p.at(K::LParen) && at_closure_params(p) {
        closure_params(p);
        p.expect(K::In);
    }
    blocks::block_items(p, BlockFlavor::Closure);
    p.expect(K::RBrace);
    finish2(p, m, K::ExprClosure)
}

/// The `(` at the cursor closes on a `)` that is followed by `in`.
fn at_closure_params(p: &Parser<'_>) -> bool {
    matching_close(p, p.token_pos()).is_some_and(|close| p.kind_at(close + 1) == Some(K::In))
}

fn closure_params(p: &mut Parser<'_>) {
    let m = p.start();
    delimited(p, K::LParen, K::RParen, true, |p| {
        let param = p.start();
        p.eat(K::Mutating);
        if !at_param_pattern_start(p) {
            p.error_expected_what("pattern");
            param.complete(p, K::ClosureParam);
            return false;
        }
        param_pattern(p);
        if p.eat(K::Colon) {
            ty(p);
        }
        param.complete(p, K::ClosureParam);
        true
    });
    m.complete(p, K::ClosureParams);
}

// ----- control flow ------------------------------------------------------------

/// `if cond, let p = v, … { } (else (if … | { }))?`
fn if_expr(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    p.bump(K::If);
    condition_list(p, K::IfLetCondition);
    blocks::inline_block(p);
    if p.at(K::Else) {
        let e = p.start();
        p.bump(K::Else);
        if p.at(K::If) {
            expr(p);
        } else {
            blocks::inline_block(p);
        }
        e.complete(p, K::ElseClause);
    }
    finish2(p, m, K::ExprIf)
}

/// Comma-separated conditions; `let` conditions get `let_kind` nodes.
pub(super) fn condition_list(p: &mut Parser<'_>, let_kind: K) {
    loop {
        if p.at(K::Let) {
            let_condition(p, let_kind, false);
        } else {
            cond_expr(p);
        }
        if !p.eat(K::Comma) {
            break;
        }
    }
}

/// `let pattern = value` as a `kind` node. The value is a condition
/// expression unless `full` (guard conditions take full expressions).
pub(super) fn let_condition(p: &mut Parser<'_>, kind: K, full: bool) {
    let m = p.start();
    p.bump(K::Let);
    pattern(p);
    p.expect(K::Equals);
    if full {
        expr(p);
    } else {
        cond_expr(p);
    }
    m.complete(p, kind);
}

/// `label:? while …`, `label:? loop …`, `label:? for …`.
fn loop_expr(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    if p.at(K::Identifier) {
        let l = p.start();
        p.bump(K::Identifier);
        p.bump(K::Colon);
        l.complete(p, K::LoopLabel);
    }
    let kind = match p.current() {
        Some(K::While) => {
            p.bump(K::While);
            if p.at(K::Let) {
                condition_list(p, K::WhileLetCondition);
            } else {
                cond_expr(p);
            }
            K::ExprWhile
        },
        Some(K::For) => {
            p.bump(K::For);
            let fp = p.start();
            pattern(p);
            fp.complete(p, K::ForPattern);
            p.expect(K::In);
            let it = p.start();
            cond_expr(p);
            it.complete(p, K::ForIterable);
            K::ExprFor
        },
        _ => {
            p.bump(K::Loop);
            K::ExprLoop
        },
    };
    blocks::inline_block(p);
    finish2(p, m, kind)
}

/// `match scrutinee { pattern (if guard)? => expr, … }`.
fn match_expr(p: &mut Parser<'_>) -> ExprInfo {
    let m = start2(p);
    p.bump(K::Match);
    cond_expr(p);
    if p.expect(K::LBrace) {
        while !p.at(K::RBrace) && !p.at_eof() {
            match_arm(p);
            if p.eat(K::Comma) {
                continue;
            }
            if !p.at(K::RBrace) && !p.at_eof() {
                p.error_expected(&[K::Comma, K::RBrace]);
                p.err_recover_balanced(|p| p.at_any(&[K::Comma, K::RBrace]));
                p.eat(K::Comma);
            }
        }
        p.expect(K::RBrace);
    }
    finish2(p, m, K::ExprMatch)
}

fn match_arm(p: &mut Parser<'_>) {
    if !super::patterns::at_pattern_start(p) {
        p.error_expected_what("pattern");
        p.err_recover_balanced(|p| p.at_any(&[K::Comma, K::RBrace]));
        return;
    }
    let m = p.start();
    pattern(p);
    if p.at(K::If) {
        let g = p.start();
        p.bump(K::If);
        cond_expr(p);
        g.complete(p, K::MatchArmGuard);
    }
    if p.expect(K::FatArrow) {
        expr(p);
    }
    m.complete(p, K::MatchArm);
}
