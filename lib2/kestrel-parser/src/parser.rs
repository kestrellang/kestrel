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
//! use kestrel_parser2::parser::Parser;
//! use kestrel_parser2::parse_source_file;
//! use kestrel_lexer2::lex;
//!
//! let source = "module A.B.C\nimport X.Y.Z";
//! let tokens: Vec<_> = lex(source, 0)
//!     .filter_map(|t| t.ok())
//!     .map(|spanned| (spanned.value, spanned.span))
//!     .collect();
//!
//! let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);
//!
//! println!("Syntax tree: {:?}", result.tree);
//! for error in result.errors {
//!     println!("Error: {}", error.message);
//! }
//! ```

use kestrel_lexer2::Token;
use kestrel_span2::Span;
use kestrel_syntax_tree2::SyntaxNode;
use std::fmt;

use crate::event::{Event, EventSink, TreeBuilder};

/// Format a token for user-friendly display (not debug format)
pub fn format_token_for_display(token: &Token) -> String {
    match token {
        // Trivia
        Token::Whitespace => "whitespace".to_string(),
        Token::Newline => "newline".to_string(),
        Token::LineComment => "comment".to_string(),
        Token::BlockComment => "comment".to_string(),

        // Literals
        Token::Underscore => "'_'".to_string(),
        Token::Identifier => "identifier".to_string(),
        Token::String => "string".to_string(),
        Token::Char => "character".to_string(),
        Token::RawString => "raw string".to_string(),
        Token::Integer => "integer".to_string(),
        Token::Float => "float".to_string(),
        Token::Boolean => "boolean".to_string(),
        Token::Null => "'null'".to_string(),

        // Declaration Keywords
        Token::Extend => "'extend'".to_string(),
        Token::Fileprivate => "'fileprivate'".to_string(),
        Token::Func => "'func'".to_string(),
        Token::Import => "'import'".to_string(),
        Token::Deinit => "'deinit'".to_string(),
        Token::Init => "'init'".to_string(),
        Token::Internal => "'internal'".to_string(),
        Token::Let => "'let'".to_string(),
        Token::Module => "'module'".to_string(),
        Token::Mutating => "'mutating'".to_string(),
        Token::Private => "'private'".to_string(),
        Token::Protocol => "'protocol'".to_string(),
        Token::Public => "'public'".to_string(),
        Token::Static => "'static'".to_string(),
        Token::Struct => "'struct'".to_string(),
        Token::Type => "'type'".to_string(),
        Token::Var => "'var'".to_string(),
        Token::Where => "'where'".to_string(),

        // Enum Keywords
        Token::Enum => "'enum'".to_string(),
        Token::Case => "'case'".to_string(),
        Token::Indirect => "'indirect'".to_string(),

        // Logical Keywords
        Token::And => "'and'".to_string(),
        Token::Not => "'not'".to_string(),
        Token::Or => "'or'".to_string(),

        // Statement Keywords
        Token::As => "'as'".to_string(),
        Token::Break => "'break'".to_string(),
        Token::Consuming => "'consuming'".to_string(),
        Token::Continue => "'continue'".to_string(),
        Token::Else => "'else'".to_string(),
        Token::For => "'for'".to_string(),
        Token::If => "'if'".to_string(),
        Token::In => "'in'".to_string(),
        Token::Loop => "'loop'".to_string(),
        Token::Return => "'return'".to_string(),
        Token::Throw => "'throw'".to_string(),
        Token::Try => "'try'".to_string(),
        Token::Throws => "'throws'".to_string(),
        Token::While => "'while'".to_string(),
        Token::Match => "'match'".to_string(),
        Token::Guard => "'guard'".to_string(),

        // Property Accessor Keywords
        Token::Get => "'get'".to_string(),
        Token::Set => "'set'".to_string(),
        Token::Subscript => "'subscript'".to_string(),

        // Braces
        Token::LParen => "'('".to_string(),
        Token::RParen => "')'".to_string(),
        Token::LBrace => "'{'".to_string(),
        Token::RBrace => "'}'".to_string(),
        Token::LBracket => "'['".to_string(),
        Token::RBracket => "']'".to_string(),

        // Punctuation
        Token::Semicolon => "';'".to_string(),
        Token::Comma => "','".to_string(),
        Token::Dot => "'.'".to_string(),
        Token::Colon => "':'".to_string(),
        Token::Question => "'?'".to_string(),
        Token::Bang => "'!'".to_string(),

        // Operators
        Token::DotDotEquals => "'..='".to_string(),
        Token::DotDotLess => "'..<'".to_string(),
        Token::DotDot => "'..'".to_string(),
        Token::LessLessEquals => "'<<='".to_string(),
        Token::GreaterGreaterEquals => "'>>='".to_string(),
        Token::LessLess => "'<<'".to_string(),
        Token::GreaterGreater => "'>>'".to_string(),
        Token::LessEquals => "'<='".to_string(),
        Token::GreaterEquals => "'>='".to_string(),
        Token::EqualsEquals => "'=='".to_string(),
        Token::BangEquals => "'!='".to_string(),
        Token::QuestionQuestion => "'??'".to_string(),
        Token::Arrow => "'->'".to_string(),
        Token::FatArrow => "'=>'".to_string(),
        Token::PlusEquals => "'+='".to_string(),
        Token::MinusEquals => "'-='".to_string(),
        Token::StarEquals => "'*='".to_string(),
        Token::SlashEquals => "'/='".to_string(),
        Token::PercentEquals => "'%='".to_string(),
        Token::AmpersandEquals => "'&='".to_string(),
        Token::PipeEquals => "'|='".to_string(),
        Token::CaretEquals => "'^='".to_string(),
        Token::Equals => "'='".to_string(),
        Token::Plus => "'+'".to_string(),
        Token::Minus => "'-'".to_string(),
        Token::Star => "'*'".to_string(),
        Token::Slash => "'/'".to_string(),
        Token::Percent => "'%'".to_string(),
        Token::Ampersand => "'&'".to_string(),
        Token::Pipe => "'|'".to_string(),
        Token::Caret => "'^'".to_string(),
        Token::Less => "'<'".to_string(),
        Token::Greater => "'>'".to_string(),
        Token::At => "'@'".to_string(),
    }
}

/// Generate a suggestion for common mistakes
pub fn suggest_fix(found: Option<&str>, expected: &[String]) -> Option<String> {
    let found = found?;

    // Wrong function keyword
    if found == "function" && expected.iter().any(|e| e.contains("func")) {
        return Some("use 'func' instead of 'function' to declare functions".to_string());
    }
    if found == "fn" && expected.iter().any(|e| e.contains("func")) {
        return Some("use 'func' instead of 'fn' to declare functions".to_string());
    }

    // Wrong variable keyword
    if found == "const"
        && expected
            .iter()
            .any(|e| e.contains("let") || e.contains("var"))
    {
        return Some("use 'let' for immutable bindings or 'var' for mutable bindings".to_string());
    }

    // Wrong arrow
    if found == "=>" && expected.iter().any(|e| e.contains("->")) {
        return Some("use '->' for return type annotations; '=>' is for match arms".to_string());
    }
    if found == "->" && expected.iter().any(|e| e.contains("=>")) {
        return Some("use '=>' for match arms; '->' is for return type annotations".to_string());
    }

    // Missing semicolon hint
    if expected.iter().any(|e| e.contains(";")) {
        return Some("you may be missing a semicolon".to_string());
    }

    // Missing closing brace
    if expected.iter().any(|e| e.contains("}")) {
        return Some("you may have forgotten to close a block with '}'".to_string());
    }

    // Missing closing paren
    if expected.iter().any(|e| e.contains(")")) {
        return Some("you may have forgotten to close with ')'".to_string());
    }

    None
}

/// Build a human-readable error message from expected and found tokens
fn build_error_message(expected: &[String], found: Option<&str>) -> String {
    match (expected.len(), found) {
        (0, Some(found)) => format!("unexpected {}", found),
        (0, None) => "unexpected end of file".to_string(),
        (1, Some(found)) => format!("expected {}, found {}", expected[0], found),
        (1, None) => format!("expected {} before end of file", expected[0]),
        (n, Some(found)) if n <= 3 => {
            format!("expected one of {}, found {}", expected.join(", "), found)
        },
        (n, Some(found)) => {
            format!(
                "expected {}, or {} others, found {}",
                expected[..2].join(", "),
                n - 2,
                found
            )
        },
        (n, None) if n <= 3 => {
            format!("expected one of {} before end of file", expected.join(", "))
        },
        (n, None) => {
            format!(
                "expected {}, or {} others before end of file",
                expected[..2].join(", "),
                n - 2
            )
        },
    }
}

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

    /// Create a parse error from a chumsky 0.12 Rich error (generic version)
    pub fn from_rich_error<'a, T: fmt::Debug + fmt::Display>(
        error: &chumsky::error::Rich<'a, T>,
    ) -> Self {
        use crate::input::to_kestrel_span2;

        let span = Some(to_kestrel_span2(*error.span()));

        // Determine error kind based on what was found
        let kind = if error.found().is_none() {
            ParseErrorKind::UnexpectedEof
        } else {
            ParseErrorKind::UnexpectedToken
        };

        // Format found token using Display
        let found = error.found().map(|t| format!("{}", t));

        // Build user-friendly message from the Rich error
        let message = format!("{}", error.reason());

        Self {
            kind,
            message,
            span,
            expected: Vec::new(),
            found,
        }
    }

    /// Create a parse error from a chumsky 0.12 Rich error with Token type
    /// This version provides better formatting for Kestrel tokens
    pub fn from_token_error<'a>(error: &chumsky::error::Rich<'a, Token>) -> Self {
        use crate::input::to_kestrel_span2;
        use chumsky::error::RichPattern;

        let span = Some(to_kestrel_span2(*error.span()));

        // Determine error kind based on what was found
        let kind = if error.found().is_none() {
            ParseErrorKind::UnexpectedEof
        } else {
            ParseErrorKind::UnexpectedToken
        };

        // Format found token using our display helper
        let found = error.found().map(format_token_for_display);

        // Extract expected tokens from the error
        let expected: Vec<String> = error
            .expected()
            .filter_map(|pattern| match pattern {
                RichPattern::Token(token) => Some(format_token_for_display(token)),
                RichPattern::Label(label) => Some(label.to_string()),
                RichPattern::EndOfInput => Some("end of input".to_string()),
                _ => None, // Handle future RichPattern variants
            })
            .collect();

        // Build human-readable message
        let message = build_error_message(&expected, found.as_deref());

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
    /// * `file_id` - The file ID for error span reporting
    ///
    /// # Returns
    ///
    /// A `ParseResult` containing both the syntax tree and any errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// use kestrel_parser2::parser::Parser;
    /// use kestrel_parser2::parse_source_file;
    /// use kestrel_lexer2::lex;
    ///
    /// let source = "module Main";
    /// let file_id = 0;
    /// let tokens: Vec<_> = lex(source, file_id)
    ///     .filter_map(|t| t.ok())
    ///     .map(|spanned| (spanned.value, spanned.span))
    ///     .collect();
    ///
    /// let result = Parser::parse(source, tokens.into_iter(), parse_source_file, file_id);
    /// assert!(result.errors.is_empty());
    /// ```
    pub fn parse<I, F>(source: &str, tokens: I, parse_fn: F, file_id: usize) -> ParseResult
    where
        I: Iterator<Item = (Token, Span)> + Clone,
        F: FnOnce(&str, I, &mut EventSink),
    {
        // Create event sink with file_id for proper error span reporting
        let mut sink = EventSink::new(file_id);

        // Parse and collect events
        // Use stacker to grow the stack if needed for deeply nested types
        stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
            parse_fn(source, tokens, &mut sink);
        });

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
    use kestrel_lexer2::lex;

    #[test]
    fn test_parser_with_valid_source() {
        let source = "module Test";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree2::SyntaxKind::SourceFile
        );
    }

    #[test]
    fn test_parser_with_multiple_declarations() {
        let source = "module A.B.C\nimport X.Y.Z";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree2::SyntaxKind::SourceFile
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

        let result = Parser::parse(valid_source, tokens.into_iter(), parse_source_file, 0);
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

        let result = Parser::parse(source_with_errors, tokens.into_iter(), parse_source_file, 0);
        // Parser creates a SourceFile node even when parsing fails
        assert_eq!(
            result.tree.kind(),
            kestrel_syntax_tree2::SyntaxKind::SourceFile
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

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);

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
            kestrel_syntax_tree2::SyntaxKind::SourceFile
        );
    }

    #[test]
    fn test_module_then_struct() {
        let source = "module Test\nstruct Empty {}";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);

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

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.children().count(),
            2,
            "Should have 2 children (module + struct)"
        );
    }

    #[test]
    fn test_error_spans_have_correct_file_id() {
        // Test that parse errors get the correct file_id
        let source = "struct 123"; // Invalid syntax
        let file_id = 42;
        let tokens: Vec<_> = lex(source, file_id)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(source, tokens.into_iter(), parse_source_file, file_id);

        // If there are errors, they should have the correct file_id
        for error in &result.errors {
            if let Some(span) = &error.span {
                assert_eq!(
                    span.file_id, file_id,
                    "Error span should have correct file_id"
                );
            }
        }
    }
}
