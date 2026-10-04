//! Generic parameter lists, conformance lists, and where clauses.
//!
//! ```text
//! TypeParameterList = '[' (TypeParameter (',' TypeParameter)* ','?)? ']'
//! TypeParameter     = Name DefaultType?
//! DefaultType       = '=' Ty               // a bare path, no type arguments
//! ConformanceList   = ':' ConformanceItem (',' ConformanceItem)* ','?
//! ConformanceItem   = NegativeConformance | Ty
//! NegativeConformance = 'not' Ty
//! WhereClause       = 'where' constraint (',' constraint)*
//! constraint        = TypeEquality | TypeBound
//! TypeEquality      = AssociatedTypeTarget '=' Ty
//! TypeBound         = (Name | AssociatedTypeTarget) ':'
//!                     (NegativeConformance | bound ('and' bound)*)
//! bound             = Path TypeArgumentList?
//! ```
//!
//! Where-clause paths and bounds use the restricted `wpath` grammar
//! (identifiers joined by *adjacent* dots, optional `[args]` of the same
//! shape) rather than full types — that is what the language accepts there.

use kestrel_syntax_tree::SyntaxKind as K;

use super::types::{adjacent, ty};
use super::{delimited, name};
use crate::core::Parser;

/// `[T, U = Default]`, if present.
pub(super) fn opt_type_parameter_list(p: &mut Parser<'_>) {
    if !p.at(K::LBracket) {
        return;
    }
    let m = p.start();
    delimited(p, K::LBracket, K::RBracket, true, |p| {
        if !p.at(K::Identifier) {
            p.error_expected(&[K::Identifier]);
            return false;
        }
        let tp = p.start();
        name(p);
        if p.at(K::Equals) {
            let d = p.start();
            p.bump(K::Equals);
            // Wrapped as Ty > TyPath > Path so it reads like any type.
            let t = p.start();
            let tp_path = p.start();
            wpath(p);
            tp_path.complete(p, K::TyPath);
            t.complete(p, K::Ty);
            d.complete(p, K::DefaultType);
        }
        tp.complete(p, K::TypeParameter);
        true
    });
    m.complete(p, K::TypeParameterList);
}

/// `: P, Q, not Copyable`, if present.
pub(super) fn opt_conformance_list(p: &mut Parser<'_>) {
    if !p.at(K::Colon) {
        return;
    }
    let m = p.start();
    p.bump(K::Colon);
    loop {
        let item = p.start();
        if p.at(K::Not) {
            let n = p.start();
            p.bump(K::Not);
            ty(p);
            n.complete(p, K::NegativeConformance);
        } else {
            ty(p);
        }
        item.complete(p, K::ConformanceItem);
        if !p.eat(K::Comma) {
            break;
        }
        // Trailing comma: the list ends at whatever cannot start a type.
        if !p.at_any(&[K::Not, K::Identifier, K::LParen, K::LBracket, K::Bang, K::Underscore])
            && !p.at_any(&[K::Some, K::Ampersand, K::Mutating, K::Consuming])
        {
            break;
        }
    }
    m.complete(p, K::ConformanceList);
}

/// `where T: P and Q, U.Item = Int`, if present.
pub(super) fn opt_where_clause(p: &mut Parser<'_>) {
    if !p.at(K::Where) {
        return;
    }
    let m = p.start();
    p.bump(K::Where);
    loop {
        where_constraint(p);
        if !p.eat(K::Comma) {
            break;
        }
    }
    m.complete(p, K::WhereClause);
}

fn where_constraint(p: &mut Parser<'_>) {
    if !p.at(K::Identifier) {
        p.error_expected(&[K::Identifier]);
        return;
    }
    // Count the subject path's segments without consuming: `T` becomes a
    // `Name`, `T.Item` an `AssociatedTypeTarget`.
    let segments = wpath_len(p);
    let after = p.nth(segments * 2 - 1);
    if after == Some(K::Equals) {
        let m = p.start();
        let target = p.start();
        wpath(p);
        target.complete(p, K::AssociatedTypeTarget);
        p.bump(K::Equals);
        ty(p);
        m.complete(p, K::TypeEquality);
        return;
    }
    let m = p.start();
    if segments == 1 {
        name(p);
    } else {
        let target = p.start();
        wpath(p);
        target.complete(p, K::AssociatedTypeTarget);
    }
    p.expect(K::Colon);
    if p.at(K::Not) {
        let n = p.start();
        p.bump(K::Not);
        wbound(p);
        n.complete(p, K::NegativeConformance);
    } else {
        wbound(p);
        while p.at(K::And) {
            p.bump(K::And);
            wbound(p);
        }
    }
    m.complete(p, K::TypeBound);
}

/// Number of segments in the adjacent-dot path at the cursor.
fn wpath_len(p: &Parser<'_>) -> usize {
    let start = p.token_pos();
    let mut n = 1;
    while p.kind_at(start + 2 * n - 1) == Some(K::Dot)
        && p.kind_at(start + 2 * n) == Some(K::Identifier)
        && adjacent(p, start + 2 * n - 1)
        && adjacent(p, start + 2 * n)
    {
        n += 1;
    }
    n
}

/// `Path` node for an adjacent-dot identifier path (`T.Item`).
pub(super) fn wpath(p: &mut Parser<'_>) {
    let m = p.start();
    let n = if p.at(K::Identifier) { wpath_len(p) } else { 0 };
    if n == 0 {
        p.error_expected(&[K::Identifier]);
    }
    for i in 0..n {
        if i > 0 {
            p.bump(K::Dot);
        }
        let e = p.start();
        p.bump(K::Identifier);
        e.complete(p, K::PathElement);
    }
    m.complete(p, K::Path);
}

/// A where-clause bound: `Path` followed by optional `[args]`.
fn wbound(p: &mut Parser<'_>) {
    wpath(p);
    if p.at(K::LBracket) {
        wtype_args(p);
    }
}

/// `TypeArgumentList` of restricted path arguments: `[A, B[C]]`.
fn wtype_args(p: &mut Parser<'_>) {
    let m = p.start();
    delimited(p, K::LBracket, K::RBracket, true, |p| {
        if !p.at(K::Identifier) {
            p.error_expected_what("type");
            return false;
        }
        let t = p.start();
        let tp = p.start();
        wbound(p);
        tp.complete(p, K::TyPath);
        t.complete(p, K::Ty);
        true
    });
    m.complete(p, K::TypeArgumentList);
}
