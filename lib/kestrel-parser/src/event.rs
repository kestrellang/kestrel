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

use kestrel_span::Span;
use kestrel_syntax_tree::{GreenNodeBuilder, SyntaxKind, SyntaxNode};

/// The exact source text a fixed-lexeme `SyntaxKind` must have.
///
/// Only punctuation and keywords — kinds whose spelling is fixed. Identifiers,
/// literals and nodes have no expected text and are absent.
fn expected_lexeme(kind: SyntaxKind) -> Option<&'static str> {
    Some(match kind {
        SyntaxKind::Dot => ".",
        SyntaxKind::Comma => ",",
        SyntaxKind::Colon => ":",
        SyntaxKind::Semicolon => ";",
        SyntaxKind::LParen => "(",
        SyntaxKind::RParen => ")",
        SyntaxKind::LBrace => "{",
        SyntaxKind::RBrace => "}",
        SyntaxKind::LBracket => "[",
        SyntaxKind::RBracket => "]",
        SyntaxKind::As => "as",
        SyntaxKind::Import => "import",
        SyntaxKind::Module => "module",
        _ => return None,
    })
}

/// Reject a token whose emitted span does not actually contain that token.
///
/// Emitters used to *invent* punctuation spans by byte arithmetic off a
/// neighbouring identifier — `last_segment_end + 1 .. + 2` for a `(`, assuming
/// exactly one byte of separator and zero trivia. But the combinators skip
/// trivia, so `import A.B. (X, Y)` and multi-line imports are grammatical, and
/// the fabricated range then held whitespace. The tree still round-tripped
/// (`emit_trivia_until`'s safety net re-emitted the real punctuation as
/// `SyntaxKind::Error`), so nothing failed — it just silently marked
/// well-formed shipping stdlib source as a parse-recovery site (F25).
///
/// **Zero-width spans are exempt.** A parser recovery branch may synthesize an
/// absent token (`or(empty().map_with(...))`); `emit_expression_statement`
/// does exactly that for a missing `;`, which is why an empty-text `Semicolon`
/// reaches here on shipping stdlib source. That is not a fabricated span — it
/// is the documented synthesized-token idiom, and the fact that it is accepted
/// silently is a separate finding (F13). What this rejects is a **non-empty**
/// range that holds the wrong bytes.
///
/// Debug-only: the check is a string compare per fixed-lexeme token, and a
/// release build must not pay for it. The suite runs debug, so drift is caught.
fn debug_assert_token_text(kind: SyntaxKind, text: &str) {
    if text.is_empty() {
        return;
    }
    debug_assert!(
        expected_lexeme(kind).is_none_or(|expected| expected == text),
        "token {kind:?} was emitted over the text {text:?}, but that kind is \
         always spelled {:?} — the span was derived by arithmetic instead of \
         being captured by the parser (see F25)",
        expected_lexeme(kind).unwrap_or_default()
    );
}

/// Distinct trivia kinds that can appear between syntax tokens. Kept local to
/// the tree builder so callers don't need to know the enumeration.
fn is_trivia_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Whitespace
            | SyntaxKind::Newline
            | SyntaxKind::LineComment
            | SyntaxKind::BlockComment
    )
}

/// Events emitted during parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Start a new syntax node
    StartNode(SyntaxKind),
    /// Add a token to the current node
    AddToken(SyntaxKind, Span),
    /// Emit a synthesized zero-width token wrapped in a `Missing` node.
    /// Used by error recovery when a required token (e.g. the identifier
    /// after `foo.`) is absent. `at` carries the byte position where the
    /// token would have appeared (start == end for a zero-width span); the
    /// inner token's `kind` is what consumers pattern-match against.
    MissingToken { kind: SyntaxKind, at: Span },
    /// Finish the current syntax node
    FinishNode,
    /// A parse error occurred. `code` is its `E8xx` syntax-error code.
    Error {
        message: String,
        span: Option<Span>,
        code: Option<&'static str>,
    },
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

    /// Emit a synthesized zero-width token wrapped in a `Missing` node.
    pub fn missing_token(&mut self, kind: SyntaxKind, at: Span) {
        self.events.push(Event::MissingToken { kind, at });
    }

    /// Finish the current syntax node
    pub fn finish_node(&mut self) {
        self.events.push(Event::FinishNode);
    }

    /// Record a parse error with an optional span
    pub fn error(&mut self, message: String, span: Option<Span>) {
        self.events.push(Event::Error {
            message,
            span,
            code: None,
        });
    }

    /// Append already-built events (the parser's output).
    pub fn extend(&mut self, events: impl IntoIterator<Item = Event>) {
        self.events.extend(events);
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

    /// Emit trivia (whitespace, newlines, line comments, block comments) from
    /// `source_pos` up to `target_pos`, preserving each trivia token's distinct
    /// `SyntaxKind`. The trivia range is re-lexed so the tree reflects the same
    /// token kinds the lexer produced, rather than lumping everything into
    /// `Whitespace`.
    fn emit_trivia_until(&mut self, target_pos: usize, builder: &mut GreenNodeBuilder) {
        if target_pos <= self.source_pos || target_pos > self.source.len() {
            return;
        }

        let start = self.source_pos;
        let trivia = &self.source[start..target_pos];
        if trivia.is_empty() {
            self.source_pos = target_pos;
            return;
        }

        let mut cursor = 0usize;
        for result in kestrel_lexer::lex(trivia, 0) {
            let (kind, span) = match result {
                Ok(spanned) => (SyntaxKind::from(spanned.value), spanned.span.range()),
                Err(spanned) => (SyntaxKind::Error, spanned.span.range()),
            };

            // Safety net: if a non-trivia token slipped into a gap between
            // emitted syntax tokens, emit remaining bytes as Error and stop so
            // we never lose source bytes.
            if !is_trivia_kind(kind) {
                let remaining = &trivia[cursor..];
                if !remaining.is_empty() {
                    builder.token(SyntaxKind::Error.into(), remaining);
                }
                cursor = trivia.len();
                break;
            }

            let text = &trivia[span.clone()];
            builder.token(kind.into(), text);
            cursor = span.end;
        }

        // If re-lexing consumed less than the gap (unexpected), emit the tail
        // as Error rather than dropping bytes.
        if cursor < trivia.len() {
            builder.token(SyntaxKind::Error.into(), &trivia[cursor..]);
        }

        self.source_pos = target_pos;
    }

    /// Process all events and build the tree
    fn process_events(&mut self, builder: &mut GreenNodeBuilder) {
        // Track the span of the outermost node so trailing trivia lands
        // inside it rather than as a sibling at the tree root.
        let mut root_depth: usize = 0;
        let mut pending_trailing_trivia = false;

        while self.pos < self.events.len() {
            // Use match on reference to avoid cloning events
            match &self.events[self.pos] {
                Event::StartNode(kind) => {
                    builder.start_node((*kind).into());
                    root_depth += 1;
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
                    debug_assert_token_text(kind, text);
                    builder.token(kind.into(), text);
                    self.source_pos = span_end;
                    self.pos += 1;
                },
                Event::MissingToken { kind, at } => {
                    let kind = *kind;
                    let at_start = at.start;

                    // Flush trivia up to the recovery point so the missing
                    // marker lands in the right place in the tree.
                    self.emit_trivia_until(at_start, builder);

                    builder.start_node(SyntaxKind::Missing.into());
                    builder.token(kind.into(), "");
                    builder.finish_node();
                    self.pos += 1;
                },
                Event::FinishNode => {
                    // Emit any trailing trivia before closing the outermost
                    // node so the tree text round-trips with the source.
                    if root_depth == 1 && !pending_trailing_trivia {
                        self.emit_trivia_until(self.source.len(), builder);
                        pending_trailing_trivia = true;
                    }
                    builder.finish_node();
                    root_depth = root_depth.saturating_sub(1);
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
    fn tree_builder_classifies_inter_token_trivia_by_kind() {
        // Source has whitespace, a newline, a block comment, and a line comment
        // wedged between two syntax tokens. The builder should preserve each
        // trivia kind distinctly rather than folding everything into Whitespace.
        let source = "A /* b */ \n// c\nB";
        let mut sink = EventSink::new(0);

        sink.start_node(SyntaxKind::ModulePath);
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 0..1));
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 16..17));
        sink.finish_node();

        let tree = TreeBuilder::new(source, sink.into_events()).build();

        assert_eq!(tree.text().to_string(), source);

        let mut kinds: Vec<SyntaxKind> = tree
            .children_with_tokens()
            .filter_map(|el| el.into_token())
            .map(|t| t.kind())
            .collect();
        // Expected: Identifier, Whitespace, BlockComment, Whitespace,
        //           Newline, LineComment, Newline, Identifier
        kinds.retain(|k| is_trivia_kind(*k) || *k == SyntaxKind::Identifier);
        assert_eq!(
            kinds,
            vec![
                SyntaxKind::Identifier,
                SyntaxKind::Whitespace,
                SyntaxKind::BlockComment,
                SyntaxKind::Whitespace,
                SyntaxKind::Newline,
                SyntaxKind::LineComment,
                SyntaxKind::Newline,
                SyntaxKind::Identifier,
            ]
        );
    }

    #[test]
    fn tree_builder_emits_trailing_trivia_after_last_token() {
        let source = "A\n// tail\n";
        let mut sink = EventSink::new(0);

        sink.start_node(SyntaxKind::ModulePath);
        sink.add_token(SyntaxKind::Identifier, Span::new(0, 0..1));
        sink.finish_node();

        let tree = TreeBuilder::new(source, sink.into_events()).build();

        assert_eq!(tree.text().to_string(), source);
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
