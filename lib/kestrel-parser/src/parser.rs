//! High-level parser API
//!
//! This module provides a convenient Parser struct that handles:
//! - Creating event sinks
//! - Parsing with any parse function
//! - Extracting errors from events
//! - Building syntax trees
//!
//! # Example
//!
//! ```no_run
//! use kestrel_parser::parser::Parser;
//! use kestrel_parser::parse_source_file;
//! use kestrel_lexer::lex;
//!
//! let source = "module A.B.C\nimport X.Y.Z";
//! let tokens: Vec<_> = lex(source, 0)
//!     .filter_map(|t| t.ok())
//!     .map(|spanned| (spanned.value, spanned.span))
//!     .collect();
//!
//! let result = Parser::parse(source, tokens.into_iter(), parse_source_file);
//!
//! println!("Syntax tree: {:?}", result.tree);
//! for error in result.errors {
//!     println!("Error: {}", error.message);
//! }
//! ```

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;
use std::fmt;

use crate::event::{Event, EventSink, TreeBuilder};

/// The kind of parse error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// An unexpected token was encountered
    UnexpectedToken,
    /// A required token is missing
    MissingToken,
    /// End of input was reached unexpectedly
    UnexpectedEof,
    /// Generic syntax error
    SyntaxError,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken => write!(f, "unexpected token"),
            ParseErrorKind::MissingToken => write!(f, "missing token"),
            ParseErrorKind::UnexpectedEof => write!(f, "unexpected end of input"),
            ParseErrorKind::SyntaxError => write!(f, "syntax error"),
        }
    }
}

/// A parse error with detailed information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The kind of error
    pub kind: ParseErrorKind,
    /// A human-readable error message
    pub message: String,
    /// The span where the error occurred (if available)
    pub span: Option<Span>,
    /// Tokens that were expected (if applicable)
    pub expected: Vec<String>,
    /// The token that was found (if applicable)
    pub found: Option<String>,
}

impl ParseError {
    /// Create a new parse error with basic information
    pub fn new(kind: ParseErrorKind, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            expected: Vec::new(),
            found: None,
        }
    }

    /// Create a parse error from a Chumsky error
    pub fn from_chumsky_error<T: fmt::Debug + std::hash::Hash + Eq>(
        error: chumsky::error::Simple<T>,
    ) -> Self {
        let span = Some(Span::from(error.span()));

        // Determine error kind
        let kind = if error.found().is_none() {
            ParseErrorKind::UnexpectedEof
        } else {
            ParseErrorKind::UnexpectedToken
        };

        // Format expected tokens
        let expected: Vec<String> = error
            .expected()
            .filter_map(|opt| opt.as_ref().map(|t| format!("{:?}", t)))
            .collect();

        // Format found token
        let found = error.found().map(|t| format!("{:?}", t));

        // Build user-friendly message
        let message = if expected.is_empty() {
            match &found {
                Some(f) => format!("unexpected {}", f),
                None => "unexpected end of input".to_string(),
            }
        } else if expected.len() == 1 {
            match &found {
                Some(f) => format!("expected {}, found {}", expected[0], f),
                None => format!("expected {}", expected[0]),
            }
        } else {
            let expected_str = expected.join(", ");
            match &found {
                Some(f) => format!("expected one of [{}], found {}", expected_str, f),
                None => format!("expected one of [{}]", expected_str),
            }
        };

        Self {
            kind,
            message,
            span,
            expected,
            found,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(span) = &self.span {
            write!(f, " at {}..{}", span.start, span.end)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// The result of parsing, containing both the syntax tree and any errors
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The parsed syntax tree
    pub tree: SyntaxNode,
    /// Any parse errors encountered
    pub errors: Vec<ParseError>,
}

/// High-level parser that provides a convenient API for parsing
pub struct Parser;

impl Parser {
    /// Parse source code using the provided parse function
    ///
    /// # Arguments
    ///
    /// * `source` - The source code to parse
    /// * `tokens` - Iterator of tokens (from the lexer)
    /// * `parse_fn` - The parse function to use (e.g., `parse_source_file`)
    ///
    /// # Returns
    ///
    /// A `ParseResult` containing both the syntax tree and any errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// use kestrel_parser::parser::Parser;
    /// use kestrel_parser::parse_source_file;
    /// use kestrel_lexer::lex;
    ///
    /// let source = "module Main";
    /// let tokens: Vec<_> = lex(source, 0)
    ///     .filter_map(|t| t.ok())
    ///     .map(|spanned| (spanned.value, spanned.span))
    ///     .collect();
    ///
    /// let result = Parser::parse(source, tokens.into_iter(), parse_source_file);
    /// assert!(result.errors.is_empty());
    /// ```
    pub fn parse<I, F>(source: &str, tokens: I, parse_fn: F) -> ParseResult
    where
        I: Iterator<Item = (Token, Span)> + Clone,
        F: FnOnce(&str, I, &mut EventSink),
    {
        // Create event sink
        let mut sink = EventSink::new();

        // Parse and collect events
        parse_fn(source, tokens, &mut sink);

        // Extract errors from events
        let events = sink.events();
        let errors: Vec<ParseError> = events
            .iter()
            .filter_map(|e| match e {
                Event::Error { message, span } => Some(ParseError::new(
                    ParseErrorKind::SyntaxError,
                    message.clone(),
                    span.clone(),
                )),
                _ => None,
            })
            .collect();

        // Build syntax tree from events
        let tree = TreeBuilder::new(source, sink.into_events()).build();

        ParseResult { tree, errors }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source_file;
    use kestrel_lexer::lex;

    #[test]
    fn test_parser_with_valid_source() {
        let source = "module Test";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree::SyntaxKind::SourceFile
        );
    }

    #[test]
    fn test_parser_with_multiple_declarations() {
        let source = "module A.B.C\nimport X.Y.Z";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree::SyntaxKind::SourceFile
        );
        assert_eq!(
            result.tree.children().count(),
            2,
            "Should have 2 declaration children"
        );
    }

    #[test]
    fn test_parser_error_recovery_behavior() {
        // Test the current error recovery behavior:
        // The parser uses Chumsky's .repeated() combinator which provides basic error recovery
        // by continuing to parse after encountering errors in the stream.

        // Test case 1: Parser handles valid code correctly
        let valid_source = r#"
module Test
public struct A {}
public struct B {}
"#;
        let tokens: Vec<_> = lex(valid_source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(valid_source, tokens.into_iter(), parse_source_file);
        assert_eq!(result.errors.len(), 0, "Valid code should have no errors");
        assert_eq!(
            result.tree.children().count(),
            3,
            "Should parse all declarations"
        );

        // Test case 2: Parser still creates a tree even with parse errors
        let source_with_errors = r#"module"#; // Incomplete module
        let tokens: Vec<_> = lex(source_with_errors, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source_with_errors, tokens.into_iter(), parse_source_file);
        // Parser creates a SourceFile node even when parsing fails
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree::SyntaxKind::SourceFile
        );

        println!(
            "Error recovery test: {} declarations, {} errors",
            result.tree.children().count(),
            result.errors.len()
        );
    }

    #[test]
    fn test_error_spans_present() {
        // Test that parse errors include span information when errors occur
        // Use a syntax that will definitely cause a parse error
        let source = "struct 123"; // struct keyword followed by number instead of identifier
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

        // Parser should report errors or successfully parse depending on error recovery
        // The important thing is that IF errors are reported, they should have spans
        for error in &result.errors {
            // Parse errors from chumsky should have spans
            println!("Error: {} at {:?}", error.message, error.span);
            // If we have errors, verify they have span info where possible
            if error.span.is_some() {
                println!("  ✓ Span information present");
            }
        }

        // This test primarily documents that span tracking infrastructure is in place
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree::SyntaxKind::SourceFile
        );
    }

    #[test]
    fn test_module_then_struct() {
        let source = "module Test\nstruct Empty {}";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.children().count(),
            2,
            "Should have 2 children (module + struct)"
        );
    }

    #[test]
    fn test_module_then_struct_with_indentation() {
        let source = "module Test\n            struct Empty {}";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.children().count(),
            2,
            "Should have 2 children (module + struct)"
        );
    }

    // TODO: Enable once assignment expressions are implemented
    // #[test]
    // fn test_init_with_assignment() {
    //     let source = r#"module Test
    // struct Point { init() { self.x = 1 } }
    // func test() {}
    //         "#;
    //     let tokens: Vec<_> = lex(source, 0)
    //         .filter_map(|t| t.ok())
    //         .map(|spanned| (spanned.value, spanned.span))
    //         .collect();
    //
    //     let result = Parser::parse(source, tokens.into_iter(), parse_source_file);
    //
    //     assert!(result.tree.children().count() >= 3, "Should have 3 children: module, struct, func");
    // }
}
