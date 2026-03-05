use kestrel_lexer2::Token;
use kestrel_span2::Span;
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode, SyntaxToken};

use crate::common::module_path_parser_internal;
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens};

/// Represents a module path like A.B.C
///
/// The module path is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModulePath {
    pub syntax: SyntaxNode,
}

impl ModulePath {
    /// Create a new ModulePath from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax }
    }

    /// Create a new ModulePath from segments, building the syntax tree
    /// This is a convenience function that emits events and builds the tree
    pub fn new(source: &str, segments: Vec<Span>) -> Self {
        let mut sink = EventSink::new(0);
        crate::common::emit_module_path(&mut sink, &segments);
        Self::from_events(source, sink.into_events())
    }

    /// Get all identifier tokens in the path
    pub fn identifier_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .filter(|tok| tok.kind() == SyntaxKind::Identifier)
    }

    /// Extract segment names from the syntax tree
    /// Returns owned strings since the text is borrowed from the syntax tree
    pub fn segment_names(&self) -> Vec<String> {
        self.identifier_tokens()
            .map(|tok| tok.text().to_string())
            .collect()
    }

    /// Extract segments with their spans from the syntax tree
    /// Returns (name, span) pairs for each segment
    pub fn segments_with_spans(&self) -> Vec<(String, Span)> {
        self.identifier_tokens()
            .map(|tok| {
                let range = tok.text_range();
                let start: usize = range.start().into();
                let end: usize = range.end().into();
                (tok.text().to_string(), Span::new(0, start..end))
            })
            .collect()
    }

    /// Get the span of the entire module path
    pub fn span(&self) -> Span {
        let range = self.syntax.text_range();
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        Span::new(0, start..end)
    }

    /// Get the number of segments in the path
    pub fn segment_count(&self) -> usize {
        self.identifier_tokens().count()
    }
}

/// Parse a module path and emit events
/// This is the primary event-driven parser function
pub fn parse_module_path<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::prelude::*;

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match module_path_parser_internal().parse(input).into_result() {
        Ok(segments) => {
            crate::common::emit_module_path(sink, &segments);
        },
        Err(errors) => {
            for error in errors {
                sink.error_from_rich(&error);
            }
        },
    }
}
