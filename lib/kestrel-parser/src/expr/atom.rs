//! Atomic expression parsers that don't depend on the recursive `expr` handle.
//!
//! Literals (integer/float/string/char/bool/null) and path expressions can be
//! parsed without referring to the top-level expression parser, so they live
//! here as standalone factories that `expr_parser` stitches into its
//! combinator chain.

use chumsky::prelude::*;
use kestrel_lexer::Token;

use crate::common::skip_trivia;
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::ty::ty_parser;

use super::data::{ExprVariant, PathSegmentData, TypeArgsData};

/// Parser for type arguments with full type support: `[T, (A, B), [Int], (X) -> Y]`.
pub(super) fn full_type_args_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, TypeArgsData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            ty_parser()
                .separated_by(skip_trivia().ignore_then(just(Token::Comma)))
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span())))
        .map(|((lbracket, args), rbracket)| TypeArgsData {
            lbracket,
            args,
            rbracket,
        })
        .boxed()
}

/// Combined literal parser: integer, float, string, raw string, char, bool, null.
///
/// Produces one `ExprVariant` per recognised literal.
pub(super) fn literal_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone {
    let integer = skip_trivia()
        .ignore_then(select! { Token::Integer = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::Integer);
    let float = skip_trivia()
        .ignore_then(select! { Token::Float = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::Float);
    let string = skip_trivia()
        .ignore_then(select! { Token::String = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::String);
    let raw_string = skip_trivia()
        .ignore_then(select! { Token::RawString = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::RawString);
    let char_literal = skip_trivia()
        .ignore_then(select! { Token::Char = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::Char);
    let boolean = skip_trivia()
        .ignore_then(select! { Token::Boolean = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::Bool);
    let null = skip_trivia()
        .ignore_then(select! { Token::Null = e => to_kestrel_span(e.span()) })
        .map(ExprVariant::Null);

    float
        .or(integer)
        .or(string)
        .or(raw_string)
        .or(char_literal)
        .or(boolean)
        .or(null)
        .boxed()
}

/// Parser for a single path segment: `identifier` or `identifier[T, U, ...]`.
pub(super) fn path_segment_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, PathSegmentData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
        .then(full_type_args_parser().or_not())
        .map(|(name, type_args)| PathSegmentData { name, type_args })
        .boxed()
}

/// Parser for a dotted path expression: `a.b.c` or `a[T].b[U].c`.
pub(super) fn path_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone {
    let segment = path_segment_parser();
    segment
        .clone()
        .then(
            skip_trivia()
                .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
                .then(segment)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut segments = vec![first];
            let mut dots = Vec::new();
            for (dot, segment) in rest {
                dots.push(dot);
                segments.push(segment);
            }
            ExprVariant::Path { segments, dots }
        })
        .boxed()
}

#[cfg(test)]
mod tests {
    use kestrel_lexer::lex;

    /// Regression: raw string nodes/tokens used to round-trip as `Error` because
    /// `ExprRawString` and the `RawString` token were missing from
    /// `KestrelLanguage::kind_from_raw`. The `_ => SyntaxKind::Error` fallback
    /// silently swallowed them when the green tree was viewed via SyntaxNode.
    ///
    /// Updated for the redesigned string syntax: raw strings now use
    /// `#"..."#` instead of `"""..."""` (which became multi-line cooked).
    #[test]
    fn raw_string_round_trips_through_full_pipeline() {
        let source = "module Test\n\nfunc main() -> lang.i64 {\n    #\"hello\"#\n}\n";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();
        let result = crate::parse_source_file_from_source(source, tokens.into_iter());
        let dbg = format!("{:#?}", result.tree);
        assert!(
            dbg.contains("ExprRawString"),
            "expected ExprRawString in tree, got:\n{dbg}"
        );
        assert!(
            result.errors.is_empty(),
            "unexpected errors: {:?}",
            result.errors
        );
    }
}
