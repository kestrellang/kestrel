//! Attributes: `@name` and `@name(arg, label: arg, ...)`.
//!
//! ```text
//! AttributeList = Attribute+
//! Attribute     = '@' Identifier AttributeArgs?
//! AttributeArgs = '(' (AttributeArg (',' AttributeArg)* ','?)? ')'
//! AttributeArg  = (Identifier ':')? value
//! value         = String | Float | Integer | Boolean | '.' Identifier
//!               | Identifier ('.' Identifier)*
//! ```

use kestrel_syntax_tree::SyntaxKind as K;

use super::delimited;
use crate::core::Parser;

/// Zero or more attributes, wrapped in an `AttributeList` node when present.
pub(super) fn attribute_list(p: &mut Parser<'_>) {
    if !p.at(K::At) {
        return;
    }
    let m = p.start();
    while p.at(K::At) {
        attribute(p);
    }
    m.complete(p, K::AttributeList);
}

fn attribute(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(K::At);
    p.expect(K::Identifier);
    if p.at(K::LParen) {
        let args = p.start();
        delimited(p, K::LParen, K::RParen, true, attribute_arg);
        args.complete(p, K::AttributeArgs);
    }
    m.complete(p, K::Attribute);
}

fn attribute_arg(p: &mut Parser<'_>) -> bool {
    let m = p.start();
    if p.at(K::Identifier) && p.nth_at(1, K::Colon) {
        p.bump(K::Identifier);
        p.bump(K::Colon);
    }
    let ok = attribute_value(p);
    m.complete(p, K::AttributeArg);
    ok
}

fn attribute_value(p: &mut Parser<'_>) -> bool {
    match p.current() {
        Some(K::String | K::Float | K::Integer | K::Boolean) => {
            p.bump_any();
            true
        },
        Some(K::Dot) => {
            p.bump(K::Dot);
            p.expect(K::Identifier)
        },
        Some(K::Identifier) => {
            p.bump(K::Identifier);
            while p.at(K::Dot) && p.nth_at(1, K::Identifier) {
                p.bump(K::Dot);
                p.bump(K::Identifier);
            }
            true
        },
        _ => {
            p.error_expected_what("attribute argument");
            false
        },
    }
}
