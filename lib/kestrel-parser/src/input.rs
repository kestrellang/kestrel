//! Input infrastructure for chumsky 0.12
//!
//! This module provides type aliases and helper functions for parsing
//! with chumsky 0.12's new API. It handles the integration between
//! kestrel-lexer's Token type and chumsky's parsing infrastructure.
//!
//! # Key Types
//!
//! - `ParserInput<'tokens>`: The input type for token-based parsing
//! - `ParserExtra<'tokens>`: Extra type for error handling
//!
//! # Migration from chumsky 0.9
//!
//! Old pattern:
//! ```ignore
//! fn parser() -> impl Parser<Token, Output, Error = Simple<Token>> + Clone
//! ```
//!
//! New pattern:
//! ```ignore
//! fn parser<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, ParserInput<'tokens, 'src>, Output, ParserExtra<'tokens, 'src>> + Clone
//! ```

use chumsky::input::MappedInput;
use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span as KestrelSpan;

/// The span type used by chumsky (byte range)
pub type ChumskySpan = SimpleSpan<usize>;

/// A spanned token tuple for chumsky input
pub type SpannedToken = (Token, ChumskySpan);

/// The input type for our parsers - a mapped input that splits (Token, Span) pairs
///
/// This is created by calling `.split_token_span(eoi_span)` on a slice of spanned tokens.
pub type ParserInput<'tokens> = MappedInput<'tokens, Token, ChumskySpan, &'tokens [SpannedToken]>;

/// Extra type alias for error handling
pub type ParserExtra<'tokens> = extra::Err<Rich<'tokens, Token, ChumskySpan>>;

/// Convert a Kestrel Span to a chumsky SimpleSpan
///
/// Note: This drops the file_id information since chumsky's SimpleSpan
/// doesn't support it. The file_id should be tracked separately.
#[inline]
pub fn to_chumsky_span(span: &KestrelSpan) -> ChumskySpan {
    SimpleSpan::new((), span.start..span.end)
}

/// Convert a chumsky SimpleSpan to a Kestrel Span
///
/// Uses file_id 0 since chumsky doesn't track file IDs.
/// Callers should set the correct file_id if needed.
#[inline]
pub fn to_kestrel_span(span: ChumskySpan) -> KestrelSpan {
    KestrelSpan::new(0, span.start..span.end)
}

/// Convert a chumsky SimpleSpan to a Kestrel Span with a specific file_id
#[inline]
pub fn to_kestrel_span_with_file(span: ChumskySpan, file_id: usize) -> KestrelSpan {
    KestrelSpan::new(file_id, span.start..span.end)
}

/// Prepare tokens for parsing by converting Kestrel spans to chumsky spans
///
/// Takes an iterator of (Token, KestrelSpan) and produces a Vec of (Token, ChumskySpan)
/// suitable for use with chumsky's input system.
pub fn prepare_tokens<I>(tokens: I) -> Vec<SpannedToken>
where
    I: Iterator<Item = (Token, KestrelSpan)>,
{
    tokens
        .map(|(token, span)| (token, to_chumsky_span(&span)))
        .collect()
}

/// Create parser input from a slice of spanned tokens
///
/// Uses the `split_token_span` method to create a MappedInput that chumsky can parse.
/// The `source_len` is used to create an EOF span for error reporting.
pub fn create_input(tokens: &[SpannedToken], source_len: usize) -> ParserInput<'_> {
    use chumsky::input::Input;
    let end_span = SimpleSpan::new((), source_len..source_len);
    tokens.split_token_span(end_span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_conversion() {
        let kestrel_span = KestrelSpan::new(42, 10..20);
        let chumsky_span = to_chumsky_span(&kestrel_span);

        assert_eq!(chumsky_span.start, 10);
        assert_eq!(chumsky_span.end, 20);

        let back = to_kestrel_span(chumsky_span);
        assert_eq!(back.start, 10);
        assert_eq!(back.end, 20);
        assert_eq!(back.file_id, 0); // Default file_id
    }

    #[test]
    fn test_span_with_file_id() {
        let chumsky_span = SimpleSpan::new((), 5..15);
        let kestrel_span = to_kestrel_span_with_file(chumsky_span, 99);

        assert_eq!(kestrel_span.start, 5);
        assert_eq!(kestrel_span.end, 15);
        assert_eq!(kestrel_span.file_id, 99);
    }

    #[test]
    fn test_prepare_tokens() {
        let tokens = vec![
            (Token::Let, KestrelSpan::new(0, 0..3)),
            (Token::Identifier, KestrelSpan::new(0, 4..5)),
        ];

        let prepared = prepare_tokens(tokens.into_iter());

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].0, Token::Let);
        assert_eq!(prepared[0].1.start, 0);
        assert_eq!(prepared[0].1.end, 3);
        assert_eq!(prepared[1].0, Token::Identifier);
        assert_eq!(prepared[1].1.start, 4);
        assert_eq!(prepared[1].1.end, 5);
    }
}
