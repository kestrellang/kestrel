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

use event::{EventSink, TreeBuilder};
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;

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

/// Parse a lone expression into `sink`. Trailing tokens are reported and
/// kept in an `Error` node.
pub fn parse_expr<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    run(source, tokens, sink, grammar::expression_only);
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

/// A standalone expression's syntax tree (rooted at `Expression`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub syntax: SyntaxNode,
    pub span: Span,
}

/// Parse a lone expression. On any syntax error the tree is
/// `Expression > Error` covering the input, so callers can treat it as
/// unparseable without inspecting diagnostics.
pub fn parse_expr_from_source<I>(source: &str, tokens: I) -> Expression
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    parse_expr(source, tokens, &mut sink);
    let failed = sink
        .events()
        .iter()
        .any(|e| matches!(e, event::Event::Error { .. }));
    let events = if failed {
        let mut sink = EventSink::new(file_id);
        sink.start_node(kestrel_syntax_tree::SyntaxKind::Expression);
        sink.start_node(kestrel_syntax_tree::SyntaxKind::Error);
        sink.finish_node();
        sink.finish_node();
        sink.into_events()
    } else {
        sink.into_events()
    };
    Expression {
        syntax: TreeBuilder::new(source, events).build(),
        span: Span::new(file_id, 0..source.len()),
    }
}
