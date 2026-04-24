//! Event-based parsing infrastructure
//!
//! This module provides the event-based parsing architecture inspired by rust-analyzer.
//! Instead of directly building syntax trees during parsing, parsers emit events that
//! are later converted into syntax trees by a TreeBuilder.
//!
//! # Architecture
//!
//! 1. Parser emits events (StartNode, AddToken, FinishNode) to an EventSink
//! 2. EventSink collects events in a Vec
//! 3. TreeBuilder consumes events and source text to build the final syntax tree
//!
//! # Benefits
//!
//! - Decouples parsing logic from tree building
//! - Easier to implement error recovery
//! - More testable (can inspect events)
//! - Follows proven rust-analyzer architecture

use crate::input::ChumskySpan;
use crate::parser::ParseError;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{GreenNodeBuilder, SyntaxKind, SyntaxNode};

/// Events emitted during parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Start a new syntax node
    StartNode(SyntaxKind),
    /// Add a token to the current node
    AddToken(SyntaxKind, Span),
    /// Finish the current syntax node
    FinishNode,
    /// A parse error occurred
    Error { message: String, span: Option<Span> },
}

/// Collects events during parsing
#[derive(Debug, Clone)]
pub struct EventSink {
    events: Vec<Event>,
    file_id: usize,
}

impl EventSink {
    /// Create a new event sink for the given file
    pub fn new(file_id: usize) -> Self {
        Self {
            events: Vec::new(),
            file_id,
        }
    }

    /// Get the file ID associated with this sink
    pub fn file_id(&self) -> usize {
        self.file_id
    }

    /// Start a new syntax node
    pub fn start_node(&mut self, kind: SyntaxKind) {
        self.events.push(Event::StartNode(kind));
    }

    /// Add a token to the current node
    pub fn add_token(&mut self, kind: SyntaxKind, span: Span) {
        self.events.push(Event::AddToken(kind, span));
    }

    /// Finish the current syntax node
    pub fn finish_node(&mut self) {
        self.events.push(Event::FinishNode);
    }

    /// Record a parse error with an optional span
    pub fn error(&mut self, message: String, span: Option<Span>) {
        self.events.push(Event::Error { message, span });
    }

    /// Record a parse error at a chumsky span (uses stored file_id)
    ///
    /// This is the primary method for recording errors from chumsky parsers,
    /// which use SimpleSpan without file ID information.
    pub fn error_at(&mut self, message: String, span: ChumskySpan) {
        self.events.push(Event::Error {
            message,
            span: Some(Span::new(self.file_id, span.start..span.end)),
        });
    }

    /// Record a parse error with a pre-built Span (uses span's file_id)
    ///
    /// Use this when you already have a complete Span with the correct file_id.
    pub fn error_at_span(&mut self, message: String, span: Span) {
        self.events.push(Event::Error {
            message,
            span: Some(span),
        });
    }

    /// Record a parse error without a specific span
    pub fn error_no_span(&mut self, message: String) {
        self.events.push(Event::Error {
            message,
            span: None,
        });
    }

    /// Record a parse error from a chumsky Rich<Token> error
    ///
    /// This is the preferred method for recording errors from chumsky parsers
    /// as it properly formats the token and extracts expected tokens.
    pub fn error_from_rich(&mut self, error: &chumsky::error::Rich<'_, Token>) {
        let parse_error = ParseError::from_token_error(error);
        // Fix the file_id in the span (chumsky spans don't carry file_id)
        let span = parse_error
            .span
            .map(|s| Span::new(self.file_id, s.start..s.end));
        self.events.push(Event::Error {
            message: parse_error.message,
            span,
        });
    }

    /// Get the collected events
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Consume the sink and return the events
    pub fn into_events(self) -> Vec<Event> {
        self.events
    }
}

/// Builds a syntax tree from events and source text
pub struct TreeBuilder<'src> {
    source: &'src str,
    events: Vec<Event>,
    pos: usize,
    /// Current position in the source, used to emit trivia before tokens
    source_pos: usize,
}

impl<'src> TreeBuilder<'src> {
    /// Create a new tree builder
    pub fn new(source: &'src str, events: Vec<Event>) -> Self {
        Self {
            source,
            events,
            pos: 0,
            source_pos: 0,
        }
    }

    /// Build the syntax tree from events
    pub fn build(mut self) -> SyntaxNode {
        let mut builder = GreenNodeBuilder::new();
        self.process_events(&mut builder);
        let green = builder.finish();
        SyntaxNode::new_root(green)
    }

    /// Emit trivia (whitespace and comments) from source_pos to the given position
    fn emit_trivia_until(&mut self, target_pos: usize, builder: &mut GreenNodeBuilder) {
        if target_pos <= self.source_pos || target_pos > self.source.len() {
            return;
        }

        let trivia = &self.source[self.source_pos..target_pos];
        if !trivia.is_empty() {
            // Emit as whitespace token (could be more granular)
            builder.token(SyntaxKind::Whitespace.into(), trivia);
        }
        self.source_pos = target_pos;
    }

    /// Process all events and build the tree
    fn process_events(&mut self, builder: &mut GreenNodeBuilder) {
        while self.pos < self.events.len() {
            // Use match on reference to avoid cloning events
            match &self.events[self.pos] {
                Event::StartNode(kind) => {
                    builder.start_node((*kind).into());
                    self.pos += 1;
                },
                Event::AddToken(kind, span) => {
                    // Extract values before modifying self
                    let kind = *kind;
                    let span_start = span.start;
                    let span_end = span.end;
                    let span_range = span.range();

                    // Emit any trivia before this token
                    self.emit_trivia_until(span_start, builder);

                    let text = &self.source[span_range];
                    builder.token(kind.into(), text);
                    self.source_pos = span_end;
                    self.pos += 1;
                },
                Event::FinishNode => {
                    builder.finish_node();
                    self.pos += 1;
                },
                Event::Error { .. } => {
                    // Skip error events when building the tree
                    // Errors can be extracted from the event list separately
                    self.pos += 1;
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_sink() {
        let mut sink = EventSink::new(0);
        sink.start_node(SyntaxKind::ModulePath);
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 0..1));
        sink.finish_node();

        let events = sink.events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], Event::StartNode(SyntaxKind::ModulePath));
        assert_eq!(
            events[1],
            Event::AddToken(SyntaxKind::Identifier, Span::new(0, 0..1))
        );
        assert_eq!(events[2], Event::FinishNode);
    }

    #[test]
    fn test_event_sink_file_id() {
        let sink = EventSink::new(42);
        assert_eq!(sink.file_id(), 42);
    }

    #[test]
    fn test_tree_builder_simple() {
        let source = "A";
        let mut sink = EventSink::new(0);

        sink.start_node(SyntaxKind::ModulePath);
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 0..1));
        sink.finish_node();

        let builder = TreeBuilder::new(source, sink.into_events());
        let tree = builder.build();

        assert_eq!(tree.kind(), SyntaxKind::ModulePath);
        assert_eq!(tree.children_with_tokens().count(), 1);
    }

    #[test]
    fn test_tree_builder_nested() {
        let source = "A.B";
        let mut sink = EventSink::new(0);

        sink.start_node(SyntaxKind::ModulePath);
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 0..1));
        sink.add_token(SyntaxKind::Dot, Span::new(0, 1..2));
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 2..3));
        sink.finish_node();

        let builder = TreeBuilder::new(source, sink.into_events());
        let tree = builder.build();

        assert_eq!(tree.kind(), SyntaxKind::ModulePath);
        assert_eq!(tree.children_with_tokens().count(), 3);
    }

    #[test]
    fn test_tree_builder_with_child_nodes() {
        let source = "module A";
        let mut sink = EventSink::new(0);

        // ModuleDeclaration
        sink.start_node(SyntaxKind::ModuleDeclaration);
        sink.add_token(SyntaxKind::Module, Span::new(0, 0..6));

        // ModulePath (child node)
        sink.start_node(SyntaxKind::ModulePath);
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 7..8));
        sink.finish_node();

        sink.finish_node();

        let builder = TreeBuilder::new(source, sink.into_events());
        let tree = builder.build();

        assert_eq!(tree.kind(), SyntaxKind::ModuleDeclaration);
        assert_eq!(tree.children().count(), 1);

        let path_node = tree.children().next().unwrap();
        assert_eq!(path_node.kind(), SyntaxKind::ModulePath);
    }
}
