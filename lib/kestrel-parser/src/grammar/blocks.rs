//! Code blocks and statements.
//!
//! ```text
//! CodeBlock  = '{' item* expr? '}'
//! item       = Statement                     // let/var, `expr;`, guard, deinit x;
//!            | stmt_like_expr                // `if`/`while`/… need no `;`
//! Statement  = VariableDeclaration | ExpressionStatement | GuardStatement | DeinitStatement
//! ```
//!
//! A block's last expression without `;` is its value (a bare `Expression`
//! child). Which expressions may stand as statements without `;` depends on
//! where the block is — see [`BlockFlavor`]. The decision is made once,
//! after parsing the expression, from its kind and the token that follows:
//! nothing is ever parsed twice.

use kestrel_syntax_tree::SyntaxKind as K;

use super::exprs::{self, ExprInfo, at_expr_start};
use super::patterns::pattern;
use super::types::ty;
use crate::core::Parser;

/// Where a block sits. It decides which statements are allowed and which
/// expressions count as statements without a `;`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockFlavor {
    /// A declaration body (function, accessor, init, deinit). Allows
    /// `deinit x;`. `if`/`while`/`loop`/`for`/`match` stand alone.
    Top,
    /// The body of `if`/`while`/`loop`/`for`. Additionally `return`,
    /// `throw` and `try` stand alone.
    Inline,
    /// A closure body: as `Inline`, but a standalone statement-like
    /// expression is not wrapped in `Statement`.
    Closure,
    /// The `else` block of a `guard` in a declaration body.
    GuardElseTop,
    /// The `else` block of a `guard` in an inline block or closure.
    GuardElseInline,
}

impl BlockFlavor {
    fn allows_guard(self) -> bool {
        matches!(self, Self::Top | Self::Inline | Self::Closure)
    }

    fn guard_else(self) -> Self {
        if self == Self::Top {
            Self::GuardElseTop
        } else {
            Self::GuardElseInline
        }
    }

    /// Whether an expression of `kind` is a statement on its own.
    fn is_stmt_like(self, kind: K) -> bool {
        let top = matches!(
            kind,
            K::ExprIf | K::ExprWhile | K::ExprLoop | K::ExprFor | K::ExprMatch
        );
        match self {
            Self::Top | Self::GuardElseTop => top,
            Self::Inline | Self::Closure | Self::GuardElseInline => {
                top || matches!(kind, K::ExprReturn | K::ExprThrow | K::ExprTry)
            },
        }
    }
}

/// A declaration body: `CodeBlock` with [`BlockFlavor::Top`].
pub(super) fn top_block(p: &mut Parser<'_>) {
    block(p, BlockFlavor::Top);
}

/// The body of `if`/`while`/`loop`/`for`.
pub(super) fn inline_block(p: &mut Parser<'_>) {
    block(p, BlockFlavor::Inline);
}

fn block(p: &mut Parser<'_>, flavor: BlockFlavor) {
    if !p.at(K::LBrace) {
        p.error_expected(&[K::LBrace]);
        return;
    }
    let m = p.start();
    p.bump(K::LBrace);
    block_items(p, flavor);
    expect_closer(p, K::RBrace);
    m.complete(p, K::CodeBlock);
}

/// Report a missing `;`/`)`/`}`/`]` at the end of the previous token, where
/// it belongs, rather than on whatever starts the next line.
pub(crate) fn expect_closer(p: &mut Parser<'_>, kind: K) -> bool {
    if p.eat(kind) {
        return true;
    }
    let found = p.current();
    let range = p.prev_range().unwrap_or_else(|| p.error_range());
    p.push_error(crate::syntax_error::SyntaxError::expected_tokens(
        &[kind],
        found,
        range,
    ));
    false
}

/// Tokens that start a new statement: block recovery stops here.
fn at_stmt_boundary(p: &Parser<'_>) -> bool {
    p.at_any(&[
        K::RBrace,
        K::Let,
        K::Var,
        K::Guard,
        K::Deinit,
        K::If,
        K::While,
        K::For,
        K::Loop,
        K::Match,
        K::Return,
        K::Break,
        K::Continue,
        K::Throw,
        K::Try,
    ])
}

/// The items of a block, up to (not including) its `}`.
pub(super) fn block_items(p: &mut Parser<'_>, flavor: BlockFlavor) {
    while !p.at(K::RBrace) && !p.at_eof() {
        match p.current() {
            Some(K::Guard) if flavor.allows_guard() => guard_stmt(p, flavor.guard_else()),
            Some(K::Deinit) if flavor == BlockFlavor::Top => deinit_stmt(p),
            Some(K::Let | K::Var) => var_decl(p),
            _ if at_expr_start(p) => expr_item(p, flavor),
            _ => {
                p.error_expected_what("expression");
                // Skip to the next statement, swallowing one `;` with it.
                let m = p.start();
                loop {
                    let semi = p.at(K::Semicolon);
                    p.bump_balanced();
                    if semi || p.at_eof() || at_stmt_boundary(p) || at_expr_start(p) {
                        break;
                    }
                }
                m.complete(p, K::Error);
            },
        }
    }
}

/// An expression in statement position: a statement (`expr;`), a
/// statement-like expression standing alone, or the block's value.
fn expr_item(p: &mut Parser<'_>, flavor: BlockFlavor) {
    let Some(info) = exprs::expr(p) else {
        return;
    };
    if p.at(K::Semicolon) {
        expression_statement(p, info, true);
        return;
    }
    if flavor.is_stmt_like(info.kind) {
        if flavor != BlockFlavor::Closure {
            expression_statement(p, info, false);
        }
        return;
    }
    if p.at(K::RBrace) || p.at_eof() {
        // The block's value.
        return;
    }
    // Something else follows: this was meant to be a statement.
    expression_statement(p, info, true);
}

/// `Statement > ExpressionStatement > expr ;?` around an already parsed
/// expression. With `semi`, the `;` is required.
fn expression_statement(p: &mut Parser<'_>, info: ExprInfo, semi: bool) {
    let es = info.cm.precede(p);
    if semi {
        expect_closer(p, K::Semicolon);
    }
    let c = es.complete(p, K::ExpressionStatement);
    c.precede(p).complete(p, K::Statement);
}

/// `let|var pattern (: Ty)? (= expr)? ;`
fn var_decl(p: &mut Parser<'_>) {
    let s = p.start();
    let m = p.start();
    p.bump_any();
    pattern(p);
    if p.eat(K::Colon) {
        ty(p);
    }
    if p.eat(K::Equals) {
        exprs::expr(p);
    }
    expect_closer(p, K::Semicolon);
    m.complete(p, K::VariableDeclaration);
    s.complete(p, K::Statement);
}

/// `deinit name;`
fn deinit_stmt(p: &mut Parser<'_>) {
    let s = p.start();
    let m = p.start();
    p.bump(K::Deinit);
    p.expect(K::Identifier);
    expect_closer(p, K::Semicolon);
    m.complete(p, K::DeinitStatement);
    s.complete(p, K::Statement);
}

/// `guard cond, let p = v, … else { … }`. Conditions are full expressions.
fn guard_stmt(p: &mut Parser<'_>, else_flavor: BlockFlavor) {
    let s = p.start();
    let m = p.start();
    p.bump(K::Guard);
    loop {
        if p.at(K::Let) {
            exprs::let_condition(p, K::GuardCondition, true);
        } else {
            exprs::expr(p);
        }
        if !p.eat(K::Comma) {
            break;
        }
    }
    if p.expect(K::Else) {
        block(p, else_flavor);
    }
    m.complete(p, K::GuardStatement);
    s.complete(p, K::Statement);
}
