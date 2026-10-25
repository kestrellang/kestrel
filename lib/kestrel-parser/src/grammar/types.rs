//! Type expressions.
//!
//! ```text
//! Ty        = '&' 'mutating'? Ty                      // TyRef / TyMutRef
//!           | base ('?' | '??' | 'throws' Ty)*        // TyOptional / TyResult
//! base      = 'some' bound ('and' bound)* ('and' 'not' bound)?   // TySome
//!           | '!'                                     // TyNever
//!           | '_'                                     // TyInferred
//!           | kind '(' elems ')' '->' Ty              // TyFunction with kind
//!           | '(' elems ')' ('->' Ty)?                // TyUnit / grouping / TyTuple / TyFunction
//!           | '[' Ty (':' Ty)? ']'                    // TyArray / TyDictionary
//!           | Path TypeArgumentList?                  // TyPath
//! kind      = 'mutating' | 'consuming' | 'escaping'   // `escaping` is contextual
//! elem      = 'mutating'? Ty                          // per-param convention
//! ```
//!
//! Every type is wrapped in a `Ty` node around its specific kind. A grouping
//! `(T)` produces just `T`'s node; its parens are tokens of the parent.
//! Path dots must be adjacent to both neighbours (`a.b`, not `a. b`).

use kestrel_syntax_tree::SyntaxKind as K;

use super::{delimited, matching_close};
use crate::core::{CompletedMarker, Parser};

/// Whether the token at absolute index `idx` touches its predecessor.
pub(super) fn adjacent(p: &Parser<'_>, idx: usize) -> bool {
    p.joined_at(idx)
}

/// Tokens that can begin a type.
pub(super) fn at_type_start(p: &Parser<'_>) -> bool {
    match p.current() {
        Some(
            K::Ampersand
            | K::Some
            | K::Bang
            | K::Underscore
            | K::LParen
            | K::LBracket
            | K::Identifier,
        ) => true,
        Some(K::Mutating | K::Consuming) => at_kinded_fn(p),
        _ => false,
    }
}

/// A full type. Reports "expected type" (and returns `None`) when nothing
/// type-like is at the cursor.
pub(super) fn ty(p: &mut Parser<'_>) -> Option<CompletedMarker> {
    if p.at(K::Ampersand) {
        let m = p.start();
        let r = p.start();
        p.bump(K::Ampersand);
        let kind = if p.eat(K::Mutating) {
            K::TyMutRef
        } else {
            K::TyRef
        };
        ty(p);
        r.complete(p, kind);
        return Some(m.complete(p, K::Ty));
    }
    let mut base = base_ty(p)?;
    loop {
        match p.current() {
            Some(K::Question) => {
                base = wrap(p, base, K::TyOptional, |p| p.bump(K::Question));
            },
            Some(K::QuestionQuestion) => {
                // `T??` is `(T?)?`: one `??` token closes both optionals.
                base = wrap(p, base, K::TyOptional, |_| {});
                base = wrap(p, base, K::TyOptional, |p| p.bump(K::QuestionQuestion));
            },
            Some(K::Throws) => {
                base = wrap(p, base, K::TyResult, |p| {
                    p.bump(K::Throws);
                    ty(p);
                });
            },
            _ => return Some(base),
        }
    }
}

/// Wrap a completed `Ty` as `Ty > kind > (Ty, …)`.
fn wrap(
    p: &mut Parser<'_>,
    inner: CompletedMarker,
    kind: K,
    rest: impl FnOnce(&mut Parser<'_>),
) -> CompletedMarker {
    let m = inner.precede(p);
    rest(p);
    let c = m.complete(p, kind);
    c.precede(p).complete(p, K::Ty)
}

fn base_ty(p: &mut Parser<'_>) -> Option<CompletedMarker> {
    match p.current() {
        Some(K::Some) => Some(some_type(p)),
        Some(K::Bang) => Some(leaf(p, K::TyNever, K::Bang)),
        Some(K::Underscore) => Some(leaf(p, K::TyInferred, K::Underscore)),
        Some(K::Mutating | K::Consuming) if at_kinded_fn(p) => Some(kinded_fn(p)),
        Some(K::Identifier) if p.nth_text(0) == "escaping" && at_kinded_fn(p) => Some(kinded_fn(p)),
        Some(K::LParen) => paren_type(p),
        Some(K::LBracket) => Some(array_or_dict(p)),
        Some(K::Identifier) => Some(path_type(p)),
        _ => {
            p.error_expected_what("type");
            None
        },
    }
}

fn leaf(p: &mut Parser<'_>, kind: K, token: K) -> CompletedMarker {
    let m = p.start();
    let inner = p.start();
    p.bump(token);
    inner.complete(p, kind);
    m.complete(p, K::Ty)
}

/// `kind ( … ) ->` at the cursor, where kind is the keyword at the cursor.
fn at_kinded_fn(p: &Parser<'_>) -> bool {
    if !p.nth_at(1, K::LParen) {
        return false;
    }
    let open = p.token_pos() + 1;
    matching_close(p, open).is_some_and(|close| p.kind_at(close + 1) == Some(K::Arrow))
}

/// `(` at the cursor opens a parameter list followed by `->`.
fn at_fn_parens(p: &Parser<'_>) -> bool {
    let open = p.token_pos();
    matching_close(p, open).is_some_and(|close| p.kind_at(close + 1) == Some(K::Arrow))
}

/// `mutating (T) -> R` / `consuming (…) -> R` / `escaping (…) -> R`.
fn kinded_fn(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    let f = p.start();
    if p.at(K::Identifier) {
        // contextual `escaping` stays an Identifier token
        p.bump(K::Identifier);
    } else {
        p.bump_any();
    }
    fn_params(p);
    p.expect(K::Arrow);
    ty(p);
    f.complete(p, K::TyFunction);
    m.complete(p, K::Ty)
}

/// `TyList` of a function type: `( 'mutating'? Ty, … )`.
fn fn_params(p: &mut Parser<'_>) {
    let list = p.start();
    if p.at(K::LParen) {
        delimited(p, K::LParen, K::RParen, true, fn_elem);
    } else {
        p.error_expected(&[K::LParen]);
    }
    list.complete(p, K::TyList);
}

/// One element of a parenthesised type list: a `mutating` convention marker
/// unless the `mutating` begins a kinded function type.
fn fn_elem(p: &mut Parser<'_>) -> bool {
    if p.at(K::Mutating) && !at_kinded_fn(p) {
        p.bump(K::Mutating);
    }
    if !at_type_start(p) {
        p.error_expected_what("type");
        return false;
    }
    ty(p).is_some()
}

/// `()`, `(T)`, `(T,)`, `(T, U)`, or `(T) -> R`.
fn paren_type(p: &mut Parser<'_>) -> Option<CompletedMarker> {
    if at_fn_parens(p) {
        let m = p.start();
        let f = p.start();
        fn_params(p);
        p.bump(K::Arrow);
        ty(p);
        f.complete(p, K::TyFunction);
        return Some(m.complete(p, K::Ty));
    }
    if p.nth_at(1, K::RParen) {
        let m = p.start();
        let u = p.start();
        p.bump(K::LParen);
        p.bump(K::RParen);
        u.complete(p, K::TyUnit);
        return Some(m.complete(p, K::Ty));
    }
    // Grouping or tuple: decided by whether a comma follows the first element.
    let m = p.start();
    let tuple = p.start();
    p.bump(K::LParen);
    let first = {
        if p.at(K::Mutating) && !at_kinded_fn(p) {
            p.bump(K::Mutating);
        }
        ty(p)
    };
    if !p.at(K::Comma) {
        // Grouping parens: `Ty > TyParen > ( Ty )`, so the parens belong to
        // a node rather than leaking into whatever holds the type.
        p.expect(K::RParen);
        if first.is_none() {
            tuple.abandon(p);
            m.abandon(p);
            return None;
        }
        tuple.complete(p, K::TyParen);
        return Some(m.complete(p, K::Ty));
    }
    while p.eat(K::Comma) {
        if p.at(K::RParen) {
            break;
        }
        if !fn_elem(p) {
            break;
        }
    }
    p.expect(K::RParen);
    tuple.complete(p, K::TyTuple);
    Some(m.complete(p, K::Ty))
}

/// `[T]` or `[K: V]`.
fn array_or_dict(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    let inner = p.start();
    p.bump(K::LBracket);
    ty(p);
    let kind = if p.eat(K::Colon) {
        ty(p);
        K::TyDictionary
    } else {
        K::TyArray
    };
    p.expect(K::RBracket);
    inner.complete(p, kind);
    m.complete(p, K::Ty)
}

/// `Ty > TyPath > Path TypeArgumentList?`.
fn path_type(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    let tp = p.start();
    type_path(p);
    if p.at(K::LBracket) {
        type_argument_list(p);
    }
    tp.complete(p, K::TyPath);
    m.complete(p, K::Ty)
}

/// `Path` of adjacent-dot identifiers.
fn type_path(p: &mut Parser<'_>) {
    let m = p.start();
    let e = p.start();
    p.expect(K::Identifier);
    e.complete(p, K::PathElement);
    loop {
        let dot = p.token_pos();
        if !(p.at(K::Dot) && p.nth_at(1, K::Identifier) && adjacent(p, dot) && adjacent(p, dot + 1))
        {
            break;
        }
        p.bump(K::Dot);
        let e = p.start();
        p.bump(K::Identifier);
        e.complete(p, K::PathElement);
    }
    m.complete(p, K::Path);
}

/// `TypeArgumentList` of full types: `[Int, [String: T]]`.
pub(super) fn type_argument_list(p: &mut Parser<'_>) {
    let m = p.start();
    delimited(p, K::LBracket, K::RBracket, true, |p| {
        if !at_type_start(p) {
            p.error_expected_what("type");
            return false;
        }
        ty(p).is_some()
    });
    m.complete(p, K::TypeArgumentList);
}

/// `some P and Q and not Copyable`.
fn some_type(p: &mut Parser<'_>) -> CompletedMarker {
    let m = p.start();
    let s = p.start();
    p.bump(K::Some);
    some_bound(p);
    while p.at(K::And) {
        if p.nth_at(1, K::Not) {
            p.bump(K::And);
            let n = p.start();
            p.bump(K::Not);
            some_bound(p);
            n.complete(p, K::NegativeConformance);
            break;
        }
        p.bump(K::And);
        some_bound(p);
    }
    s.complete(p, K::TySome);
    m.complete(p, K::Ty)
}

fn some_bound(p: &mut Parser<'_>) {
    if !p.at(K::Identifier) {
        p.error_expected_what("protocol");
        return;
    }
    path_type(p);
}

/// Speculatively parse `[types]` after an expression path segment. Returns
/// whether it parsed cleanly; on failure nothing is consumed or reported.
pub(super) fn try_type_argument_list(p: &mut Parser<'_>) -> bool {
    let cp = p.checkpoint();
    type_argument_list(p);
    if p.has_errors_since(&cp) {
        p.rollback(cp);
        return false;
    }
    true
}
