mod path;

pub use path::{ModulePath, parse_module_path};

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::{emit_module_path, module_declaration_parser_internal};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens};

/// Represents a module declaration: module A.B.C
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl ModuleDeclaration {
    /// Create a new ModuleDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Create a new ModuleDeclaration from spans, building the syntax tree
    /// This is a convenience function that emits events and builds the tree
    pub fn new(source: &str, module_span: Span, path_segments: Vec<Span>) -> Self {
        let full_span = Span::from(module_span.start..path_segments.last().unwrap().end);
        let mut sink = EventSink::new(0);
        emit_module_declaration(&mut sink, module_span, &path_segments);
        Self::from_events(source, sink.into_events(), full_span)
    }

    /// Get the module path from this declaration
    pub fn path(&self) -> ModulePath {
        self.syntax
            .children()
            .find(|node| node.kind() == SyntaxKind::ModulePath)
            .map(|node| ModulePath { syntax: node })
            .expect("ModuleDeclaration must have a ModulePath child")
    }
}

/// Parse a module declaration and emit events
/// This is the primary event-driven parser function
pub fn parse_module_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use chumsky::prelude::*;

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match module_declaration_parser_internal()
        .parse(input)
        .into_result()
    {
        Ok((module_span, path_segments)) => {
            emit_module_declaration(sink, module_span, &path_segments);
        }
        Err(errors) => {
            // Emit error events for each parse error
            for error in errors {
                // Chumsky errors have span information
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), *span);
            }
        }
    }
}

/// Emit events for a module declaration
/// Internal helper function
fn emit_module_declaration(sink: &mut EventSink, module_span: Span, path_segments: &[Span]) {
    sink.start_node(SyntaxKind::ModuleDeclaration);
    sink.add_token(SyntaxKind::Module, module_span);
    emit_module_path(sink, path_segments);
    sink.finish_node();
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_module_path_single_segment() {
        let source = "A";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_module_path(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let path = ModulePath { syntax: tree };

        assert_eq!(path.segment_count(), 1);
        assert_eq!(path.segment_names(), vec!["A".to_string()]);

        // Verify syntax tree
        assert_eq!(path.syntax.kind(), SyntaxKind::ModulePath);
        assert_eq!(path.syntax.children().count(), 0); // All children are tokens
        assert_eq!(path.syntax.children_with_tokens().count(), 1); // One identifier token
    }

    #[test]
    fn test_module_path_multiple_segments() {
        let source = "A.B.C";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_module_path(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let path = ModulePath { syntax: tree };

        assert_eq!(path.segment_count(), 3);
        assert_eq!(
            path.segment_names(),
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );

        // Verify syntax tree: A.B.C = 3 identifiers + 2 dots = 5 tokens
        assert_eq!(path.syntax.kind(), SyntaxKind::ModulePath);
        assert_eq!(path.syntax.children_with_tokens().count(), 5);
    }

    #[test]
    fn test_module_declaration() {
        let source = "module A.B.C";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_module_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = ModuleDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };
        let path = decl.path();

        assert_eq!(path.segment_count(), 3);
        assert_eq!(
            path.segment_names(),
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );

        // Verify syntax tree structure
        assert_eq!(decl.syntax.kind(), SyntaxKind::ModuleDeclaration);
        // Should have: 1 Module token + 1 ModulePath node = 2 children with tokens
        assert_eq!(decl.syntax.children_with_tokens().count(), 2);

        // Verify the module path child node
        let path_node = decl.syntax.children().next().unwrap();
        assert_eq!(path_node.kind(), SyntaxKind::ModulePath);
    }

    #[test]
    fn test_module_declaration_single_segment() {
        let source = "module Main";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_module_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = ModuleDeclaration {
            syntax: tree,
            span: Span::from(0..source.len()),
        };
        let path = decl.path();

        assert_eq!(path.segment_count(), 1);
        assert_eq!(path.segment_names(), vec!["Main".to_string()]);

        // Verify syntax tree
        assert_eq!(decl.syntax.kind(), SyntaxKind::ModuleDeclaration);
    }
}
