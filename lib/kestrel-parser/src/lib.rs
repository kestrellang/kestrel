//! Kestrel Parser
//!
//! A handwritten recursive-descent parser producing a lossless rowan CST.
//!
//! ```text
//! tokens ─▶ core::Parser (token source + markers) ─▶ events ─▶ TreeBuilder ─▶ SyntaxNode
//!                 ▲
//!            grammar/*  (one function per construct)
//! ```
//!
//! - `core` — the engine: non-trivia token source with newline flags,
//!   `start / complete / precede` markers, bounded speculation, and the
//!   conversion of its buffer into [`event::Event`]s.
//! - `grammar` — the Kestrel grammar. Linear time: every choice is made
//!   with bounded lookahead, nothing is parsed twice.
//! - `event` — the event stream and the `TreeBuilder` that re-inserts
//!   trivia and builds the rowan tree.
//! - `syntax_error` — coded (`E8xx`) syntax diagnostics.
//!
//! # Example
//!
//! ```no_run
//! use kestrel_lexer::lex;
//!
//! let source = "module A.B.C\nimport X.Y.Z";
//! let tokens: Vec<_> = lex(source, 0)
//!     .filter_map(|t| t.ok())
//!     .map(|spanned| (spanned.value, spanned.span))
//!     .collect();
//! let result = kestrel_parser::parse_source_file_from_source(source, tokens.into_iter());
//! assert!(result.errors.is_empty());
//! ```

mod core;
pub mod event;
mod grammar;
pub mod parser;
pub mod syntax_error;

use event::EventSink;
use kestrel_lexer::Token;
use kestrel_span::Span;

pub use parser::{ParseError, ParseErrorKind, ParseResult, Parser};
pub use syntax_error::codes;

/// Run a grammar entry point over `tokens` and append its events to `sink`.
fn run<I>(source: &str, tokens: I, sink: &mut EventSink, entry: fn(&mut core::Parser<'_>))
where
    I: Iterator<Item = (Token, Span)>,
{
    let mut p = core::Parser::new(source, tokens, sink.file_id());
    entry(&mut p);
    sink.extend(p.finish());
}

/// Parse a whole source file (a `SourceFile` node) into `sink`.
pub fn parse_source_file<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    run(source, tokens, sink, grammar::source_file);
}

/// File id carried by the first token, or 0 when there are none.
fn extract_file_id<I>(tokens: &I) -> usize
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    tokens
        .clone()
        .next()
        .map(|(_, span)| span.file_id)
        .unwrap_or(0)
}

/// Parse a source file and build its tree.
pub fn parse_source_file_from_source<I>(source: &str, tokens: I) -> ParseResult
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    Parser::parse(source, tokens, parse_source_file, file_id)
}
