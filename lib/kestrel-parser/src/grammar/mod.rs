//! The Kestrel grammar, written as a handwritten recursive-descent parser.
//!
//! Each function parses one construct at the cursor and emits its CST node
//! through the [`Parser`] marker API. Choices are made with bounded
//! lookahead (a fixed number of tokens, or one bracket-matched group);
//! the only speculative parse is a type-argument list after an expression
//! path segment (`foo[Int]` vs. a following array literal), which is
//! bounded by its brackets and never contains an expression.
//!
//! | Module | Constructs |
//! |--------|------------|
//! | `items` | declarations, type bodies, parameters, accessors |
//! | `attrs` | `@attribute(args)` lists |
//! | `generics` | type parameters, conformance lists, where clauses |
//! | `types` | type expressions |
//! | `patterns` | patterns |
//! | `exprs` | expressions |
//! | `blocks` | code blocks, statements, `guard` |

mod attrs;
mod blocks;
mod exprs;
mod generics;
mod items;
mod patterns;
mod types;

use kestrel_syntax_tree::SyntaxKind as K;

use crate::core::Parser;

/// `SourceFile` — the whole file: declarations until end of input.
pub(crate) fn source_file(p: &mut Parser<'_>) {
    let m = p.start();
    items::item_list(p);
    m.complete(p, K::SourceFile);
}

/// Tokens a parameter label or argument label may be spelled with:
/// identifiers and every keyword except the access modes.
pub(crate) fn is_label_keyword(kind: K) -> bool {
    matches!(
        kind,
        K::As
            | K::And
            | K::Break
            | K::Case
            | K::Continue
            | K::Deinit
            | K::Else
            | K::Enum
            | K::Extend
            | K::Fileprivate
            | K::For
            | K::Func
            | K::Get
            | K::Guard
            | K::If
            | K::Import
            | K::In
            | K::Indirect
            | K::Init
            | K::Internal
            | K::Let
            | K::Loop
            | K::Match
            | K::Module
            | K::Not
            | K::Or
            | K::Private
            | K::Protocol
            | K::Public
            | K::Return
            | K::Set
            | K::Static
            | K::Struct
            | K::Subscript
            | K::Throw
            | K::Throws
            | K::Try
            | K::Type
            | K::Var
            | K::Where
            | K::While
    )
}

/// `Name` — a single identifier wrapped in a `Name` node.
pub(crate) fn name(p: &mut Parser<'_>) {
    if p.at(K::Identifier) {
        let m = p.start();
        p.bump(K::Identifier);
        m.complete(p, K::Name);
    } else {
        p.error_expected(&[K::Identifier]);
        // A keyword where a name belongs (`func case()`) is the name the
        // user meant: consume it so the rest of the header still parses.
        if p.current().is_some_and(is_label_keyword) {
            p.err_bump();
        }
    }
}

/// Parse a delimited, comma-separated list: `open item (, item)* [,] close`.
/// The caller has checked `open`. `item` must consume at least one token or
/// report an error; the list stops (and reports) when an item is followed by
/// neither `,` nor `close`.
pub(crate) fn delimited(
    p: &mut Parser<'_>,
    open: K,
    close: K,
    allow_trailing: bool,
    mut item: impl FnMut(&mut Parser<'_>) -> bool,
) {
    p.bump(open);
    if p.eat(close) {
        return;
    }
    // A hard stop ends the list even mid-recovery: the enclosing construct
    // owns it (`}` / `;` outside a brace list).
    let hard_stop =
        |p: &Parser<'_>| p.at_eof() || (close != K::RBrace && p.at_any(&[K::RBrace, K::Semicolon]));
    loop {
        let ok = item(p);
        if p.at(close) || hard_stop(p) {
            break;
        }
        if !p.at(K::Comma) {
            if ok {
                p.error_expected(&[K::Comma, close]);
            }
            // Skip the rest of this item, up to the next separator.
            p.err_recover_balanced(|p| p.at_any(&[K::Comma, close]) || hard_stop(p));
            if !p.at(K::Comma) {
                break;
            }
        }
        p.bump(K::Comma);
        if allow_trailing && p.at(close) {
            break;
        }
    }
    blocks::expect_closer(p, close);
}

/// Index of the token that closes the bracket at absolute index `open`, or
/// `None` if it is never closed.
pub(crate) fn matching_close(p: &Parser<'_>, open: usize) -> Option<usize> {
    p.closer_of(open)
}
