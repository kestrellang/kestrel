//! Patterns.
//!
//! ```text
//! Pattern  = pat                                   // wrapper, top level only
//! pat      = base ('or' base)*                     // OrPattern when 2+
//! base     = '_'                                   // WildcardPattern
//!          | '..'                                  // RestPattern
//!          | lit ('..=' | '..<') lit | lit '..'    // RangePattern (Integer/Char)
//!          | ('..=' | '..<') lit                   // RangePattern
//!          | literal                               // LiteralPattern
//!          | 'null'                                // NullPattern
//!          | 'some' base                           // SomePattern
//!          | '.' Identifier ('(' args ')')?        // EnumPattern
//!          | Identifier '{' fields '}'             // StructPattern
//!          | '[' elems ']'                         // ArrayPattern
//!          | '&' 'mutating'? Identifier            // RefBindingPattern
//!          | 'var'? Identifier ('@' pat)?          // BindingPattern / AtPattern
//!          | '(' pat,* ')'                         // TuplePattern
//! ```
//!
//! Only the outermost pattern gets a `Pattern` wrapper; nested patterns are
//! bare. Function and closure parameters use the irrefutable subset
//! ([`param_pattern`]).

use kestrel_syntax_tree::SyntaxKind as K;

use crate::core::Parser;

/// A pattern with its `Pattern` wrapper.
pub(super) fn pattern(p: &mut Parser<'_>) {
    let m = p.start();
    pat(p);
    m.complete(p, K::Pattern);
}

/// Tokens that can begin a pattern.
pub(super) fn at_pattern_start(p: &Parser<'_>) -> bool {
    p.at_any(&[
        K::Underscore,
        K::DotDot,
        K::DotDotEquals,
        K::DotDotLess,
        K::Integer,
        K::Float,
        K::String,
        K::Boolean,
        K::Char,
        K::Null,
        K::Some,
        K::Dot,
        K::Identifier,
        K::LBracket,
        K::Ampersand,
        K::Var,
        K::LParen,
    ])
}

/// Or-level pattern without a wrapper.
fn pat(p: &mut Parser<'_>) {
    let Some(first) = base(p) else {
        return;
    };
    if !p.at(K::Or) {
        return;
    }
    let m = first.precede(p);
    while p.eat(K::Or) {
        base(p);
    }
    m.complete(p, K::OrPattern);
}

fn base(p: &mut Parser<'_>) -> Option<crate::core::CompletedMarker> {
    let m = p.start();
    let kind = match p.current() {
        Some(K::Underscore) => {
            p.bump(K::Underscore);
            K::WildcardPattern
        },
        Some(K::DotDot) => {
            p.bump(K::DotDot);
            K::RestPattern
        },
        Some(K::Integer | K::Char)
            if p.nth_at(1, K::DotDot)
                || (p.at_any_nth(1, &[K::DotDotEquals, K::DotDotLess])
                    && p.at_any_nth(2, &[K::Integer, K::Char])) =>
        {
            p.bump_any();
            if p.at(K::DotDot) {
                p.bump(K::DotDot);
            } else {
                p.bump_any();
                p.bump_any();
            }
            K::RangePattern
        },
        Some(K::DotDotEquals | K::DotDotLess) => {
            p.bump_any();
            if p.at_any(&[K::Integer, K::Char]) {
                p.bump_any();
            } else {
                p.error_expected_what("range bound");
            }
            K::RangePattern
        },
        Some(K::Float | K::Integer | K::String | K::Boolean | K::Char) => {
            p.bump_any();
            K::LiteralPattern
        },
        Some(K::Null) => {
            p.bump(K::Null);
            K::NullPattern
        },
        Some(K::Some) => {
            p.bump(K::Some);
            base(p);
            K::SomePattern
        },
        Some(K::Dot) => {
            enum_pattern(p);
            K::EnumPattern
        },
        Some(K::Identifier) if p.nth_at(1, K::LBrace) => {
            struct_pattern(p, pat);
            K::StructPattern
        },
        Some(K::LBracket) => {
            array_pattern(p);
            K::ArrayPattern
        },
        Some(K::Ampersand) => {
            p.bump(K::Ampersand);
            p.eat(K::Mutating);
            p.expect(K::Identifier);
            K::RefBindingPattern
        },
        Some(K::Var | K::Identifier) => binding(p),
        Some(K::LParen) => {
            tuple_pattern(p, pat);
            K::TuplePattern
        },
        _ => {
            m.abandon(p);
            p.error_expected_what("pattern");
            return None;
        },
    };
    Some(m.complete(p, kind))
}

/// `var? name` optionally followed by `@ pat`. Returns the node kind.
fn binding(p: &mut Parser<'_>) -> K {
    p.eat(K::Var);
    p.expect(K::Identifier);
    if p.eat(K::At) {
        pat(p);
        return K::AtPattern;
    }
    K::BindingPattern
}

/// `.Case` or `.Case(label, label: pat, pat)`.
fn enum_pattern(p: &mut Parser<'_>) {
    p.bump(K::Dot);
    p.expect(K::Identifier);
    if !p.at(K::LParen) {
        return;
    }
    super::delimited(p, K::LParen, K::RParen, true, |p| {
        let arg = p.start();
        if p.at(K::Identifier) {
            // A bare identifier is always a label (shorthand binding); a
            // pattern may follow its colon.
            p.bump(K::Identifier);
            if p.eat(K::Colon) {
                pat(p);
            }
        } else if at_pattern_start(p) {
            pat(p);
        } else {
            arg.abandon(p);
            p.error_expected_what("pattern");
            return false;
        }
        arg.complete(p, K::EnumPatternArg);
        true
    });
}

/// `Name { field, field: pat, .. }`, with `sub` parsing nested patterns.
fn struct_pattern(p: &mut Parser<'_>, sub: fn(&mut Parser<'_>)) {
    p.bump(K::Identifier);
    super::delimited(p, K::LBrace, K::RBrace, true, |p| {
        if p.at(K::DotDot) {
            let r = p.start();
            p.bump(K::DotDot);
            r.complete(p, K::StructPatternRest);
            return true;
        }
        if !p.at(K::Identifier) {
            p.error_expected(&[K::Identifier]);
            return false;
        }
        let f = p.start();
        p.bump(K::Identifier);
        if p.eat(K::Colon) {
            sub(p);
        }
        f.complete(p, K::StructPatternField);
        true
    });
}

/// `(pat, pat, …)` with each element in a `TuplePatternElement`.
fn tuple_pattern(p: &mut Parser<'_>, sub: fn(&mut Parser<'_>)) {
    super::delimited(p, K::LParen, K::RParen, true, |p| {
        let e = p.start();
        let before = p.token_pos();
        sub(p);
        if p.token_pos() == before {
            e.abandon(p);
            return false;
        }
        e.complete(p, K::TuplePatternElement);
        true
    });
}

/// `[a, b, ..rest, c]`.
fn array_pattern(p: &mut Parser<'_>) {
    super::delimited(p, K::LBracket, K::RBracket, true, |p| {
        if p.at(K::DotDot) {
            let r = p.start();
            p.bump(K::DotDot);
            p.eat(K::Identifier);
            r.complete(p, K::ArrayPatternRest);
            return true;
        }
        let e = p.start();
        let before = p.token_pos();
        pat(p);
        if p.token_pos() == before {
            e.abandon(p);
            return false;
        }
        e.complete(p, K::ArrayPatternElement);
        true
    });
}

/// An irrefutable parameter pattern with its `Pattern` wrapper: binding,
/// tuple, struct, or wildcard.
pub(super) fn param_pattern(p: &mut Parser<'_>) {
    let m = p.start();
    param_pat(p);
    m.complete(p, K::Pattern);
}

/// Tokens that can begin a parameter pattern.
pub(super) fn at_param_pattern_start(p: &Parser<'_>) -> bool {
    p.at_any(&[K::LParen, K::Identifier, K::Underscore, K::Var])
}

fn param_pat(p: &mut Parser<'_>) {
    let m = p.start();
    let kind = match p.current() {
        Some(K::LParen) => {
            tuple_pattern(p, param_pat);
            K::TuplePattern
        },
        Some(K::Identifier) if p.nth_at(1, K::LBrace) => {
            struct_pattern(p, param_pat);
            K::StructPattern
        },
        Some(K::Underscore) => {
            p.bump(K::Underscore);
            K::WildcardPattern
        },
        Some(K::Var | K::Identifier) => {
            p.eat(K::Var);
            p.expect(K::Identifier);
            K::BindingPattern
        },
        _ => {
            m.abandon(p);
            p.error_expected_what("pattern");
            return;
        },
    };
    m.complete(p, kind);
}
