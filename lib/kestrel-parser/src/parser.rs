//! High-level parser API
//!
//! [`Parser::parse`] runs a parse function into an [`EventSink`], collects
//! the coded syntax errors, and builds the lossless tree.
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
//! let result = Parser::parse(source, tokens.into_iter(), parse_source_file, 0);
//! for error in result.errors {
//!     println!("{}: {}", error.code.unwrap_or(""), error.message);
//! }
//! ```

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;
use std::fmt;

use crate::event::{Event, EventSink, TreeBuilder};

/// The kind of parse error
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// A parse error with a stable `E8xx` code and a handwritten message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    /// The kind of error
    pub kind: ParseErrorKind,
    /// A human-readable error message
    pub message: String,
    /// The span where the error occurred (if available)
    pub span: Option<Span>,
    /// The syntax-error code (`E800`…); see `syntax_error::codes`.
    pub code: Option<&'static str>,
}

impl ParseError {
    /// Create a new parse error with basic information
    pub fn new(kind: ParseErrorKind, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            code: None,
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
    /// Any parse errors encountered, sorted by position
    pub errors: Vec<ParseError>,
}

impl std::hash::Hash for ParseResult {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tree.text().to_string().hash(state);
        self.errors.hash(state);
    }
}

/// Runs a parse function and builds its tree.
pub struct Parser;

impl Parser {
    /// Run `parse_fn` over `tokens` and build the tree.
    pub fn parse<I, F>(source: &str, tokens: I, parse_fn: F, file_id: usize) -> ParseResult
    where
        I: Iterator<Item = (Token, Span)> + Clone,
        F: FnOnce(&str, I, &mut EventSink),
    {
        let mut sink = EventSink::new(file_id);
        // Deeply nested source recurses deeply; grow the stack on demand.
        stacker::maybe_grow(64 * 1024, 4 * 1024 * 1024, || {
            parse_fn(source, tokens, &mut sink);
        });

        let errors: Vec<ParseError> = sink
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::Error {
                    message,
                    span,
                    code,
                } => Some(ParseError {
                    kind: ParseErrorKind::SyntaxError,
                    message: message.clone(),
                    span: span.clone(),
                    code: *code,
                }),
                _ => None,
            })
            .collect();

        let tree = TreeBuilder::new(source, sink.into_events()).build();

        // Contract: an error-free tree has the shape `kestrel.ungram` names,
        // which is what the generated typed views read. Checked in debug
        // builds; `tests/conformance.rs` checks the corpus in any build.
        #[cfg(debug_assertions)]
        if errors.is_empty() {
            let violations = kestrel_syntax_tree::validate::validate(&tree);
            debug_assert!(
                violations.is_empty(),
                "parser built a tree that does not conform to kestrel.ungram: {}",
                violations
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }

        ParseResult { tree, errors }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source_file;
    use kestrel_lexer::lex;
    use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

    fn parse_source(source: &str, file_id: usize) -> ParseResult {
        let tokens: Vec<_> = lex(source, file_id)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        Parser::parse(source, tokens.into_iter(), parse_source_file, file_id)
    }

    fn token_texts(node: &SyntaxNode, kinds: &[SyntaxKind]) -> Vec<String> {
        let mut texts = Vec::new();
        collect_token_texts(node, kinds, &mut texts);
        texts
    }

    fn collect_token_texts(node: &SyntaxNode, kinds: &[SyntaxKind], texts: &mut Vec<String>) {
        for element in node.children_with_tokens() {
            if let Some(child) = element.clone().into_node() {
                collect_token_texts(&child, kinds, texts);
            } else if let Some(token) = element.into_token()
                && kinds.contains(&token.kind())
            {
                texts.push(token.text().to_string());
            }
        }
    }

    fn count_nodes(node: &SyntaxNode, kind: SyntaxKind) -> usize {
        let here = usize::from(node.kind() == kind);
        here + node
            .children()
            .map(|child| count_nodes(&child, kind))
            .sum::<usize>()
    }

    #[test]
    fn test_parser_with_valid_source() {
        let source = "module Test";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);
    }

    #[test]
    fn test_parser_with_multiple_declarations() {
        let source = "module A.B.C\nimport X.Y.Z";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);
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
        let result = parse_source(valid_source, 0);
        assert_eq!(result.errors.len(), 0, "Valid code should have no errors");
        assert_eq!(
            result.tree.children().count(),
            3,
            "Should parse all declarations"
        );

        // Test case 2: Parser still creates a tree even with parse errors
        let source_with_errors = r#"module"#; // Incomplete module
        let result = parse_source(source_with_errors, 0);
        // Parser creates a SourceFile node even when parsing fails
        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);

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
        let result = parse_source(source, 0);

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
        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);
    }

    #[test]
    fn test_module_then_struct() {
        let source = "module Test\nstruct Empty {}";
        let result = parse_source(source, 0);

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
        let result = parse_source(source, 0);

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

    #[test]
    fn trivia_kinds_are_distinct_between_declarations() {
        let source = "module Test\n// keep this comment\nimport Std.IO";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(result.tree.text().to_string(), source);

        let line_comments = token_texts(&result.tree, &[SyntaxKind::LineComment]);
        assert_eq!(line_comments, vec!["// keep this comment"]);

        let newlines = token_texts(&result.tree, &[SyntaxKind::Newline]);
        assert_eq!(
            newlines.len(),
            2,
            "two \\n separators between the three tokens"
        );

        let whitespace = token_texts(&result.tree, &[SyntaxKind::Whitespace]);
        assert!(
            whitespace
                .iter()
                .all(|t| !t.contains('\n') && !t.contains("//")),
            "Whitespace kind holds only spaces/tabs, not newlines or comments"
        );
    }

    #[test]
    fn trivia_round_trips_block_and_line_comments() {
        let source = "module Test\n/* block */ struct Foo {}\n// trailing\n";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.text().to_string(),
            source,
            "tree text must round-trip the source verbatim"
        );

        let block_comments = token_texts(&result.tree, &[SyntaxKind::BlockComment]);
        assert_eq!(block_comments, vec!["/* block */"]);

        let line_comments = token_texts(&result.tree, &[SyntaxKind::LineComment]);
        assert_eq!(line_comments, vec!["// trailing"]);
    }

    #[test]
    fn trailing_trivia_is_preserved_in_tree() {
        let source = "module Test\n// tail comment\n   \n";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            result.tree.text().to_string(),
            source,
            "trailing trivia after the last syntax token must appear in the tree"
        );
    }

    #[test]
    fn characterization_nested_struct_enum_declarations() {
        let source = "struct Outer { enum Inner { case Value struct Nested {} } }";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(count_nodes(&result.tree, SyntaxKind::StructDeclaration), 2);
        assert_eq!(count_nodes(&result.tree, SyntaxKind::EnumDeclaration), 1);
        assert_eq!(
            count_nodes(&result.tree, SyntaxKind::EnumCaseDeclaration),
            1
        );
    }

    #[test]
    fn recovery_preserves_declarations_around_garbage_region() {
        // A malformed token run between two valid declarations should not
        // swallow the surrounding declarations — both should still parse.
        let source = "module A\nxyz bad stuff\nimport Std.IO";
        let result = parse_source(source, 0);

        assert_eq!(
            result.tree.text().to_string(),
            source,
            "tree must still round-trip even when recovering"
        );
        assert!(!result.errors.is_empty(), "recovery should report an error");

        // Both the module and import declarations should be in the tree.
        assert_eq!(count_nodes(&result.tree, SyntaxKind::ModuleDeclaration), 1);
        assert_eq!(count_nodes(&result.tree, SyntaxKind::ImportDeclaration), 1);
        // The recovered garbage is wrapped in an Error node.
        assert!(
            count_nodes(&result.tree, SyntaxKind::Error) >= 1,
            "recovered region should become an Error node"
        );
    }

    #[test]
    fn recovery_error_span_covers_skipped_garbage() {
        let source = "module A\nxyz bad\nimport B";
        let result = parse_source(source, 0);

        assert!(!result.errors.is_empty());
        let err = result.errors.iter().find(|e| e.span.is_some()).unwrap();
        let span = err.span.as_ref().unwrap();
        let covered = &source[span.start..span.end];
        assert!(
            covered.contains("xyz"),
            "recovery span should include the skipped token, got {covered:?}"
        );
    }

    #[test]
    fn recovery_does_not_fire_on_trailing_trivia() {
        // Trailing whitespace/comments after the final declaration must not
        // trigger recovery — otherwise every file with a newline at the end
        // would report a phantom error.
        let source = "module A\n// trailing\n";
        let result = parse_source(source, 0);

        assert!(
            result.errors.is_empty(),
            "trailing trivia should not trigger recovery, got {:?}",
            result.errors
        );
        assert_eq!(result.tree.text().to_string(), source);
    }

    #[test]
    fn characterization_operator_tokens_are_preserved_for_later_pratt_parser() {
        let source = "func calc() { let value = a + b * c ?? d; }";
        let result = parse_source(source, 0);

        assert!(result.errors.is_empty(), "Should have no errors");
        assert_eq!(
            token_texts(
                &result.tree,
                &[
                    SyntaxKind::Plus,
                    SyntaxKind::Star,
                    SyntaxKind::QuestionQuestion
                ],
            ),
            vec!["+", "*", "??"]
        );
    }

    #[test]
    fn missing_member_after_dot_recovers_without_a_node() {
        // Cursor mid-edit: `foo.` with nothing after. The member name is
        // simply absent from the tree (no synthesized token) and exactly
        // one parse error is recorded.
        let source = "func f() { foo. }";
        let result = parse_source(source, 0);

        let missing_count = count_nodes(&result.tree, SyntaxKind::Missing);
        assert_eq!(
            missing_count, 0,
            "missing tokens are absent, not synthesized"
        );

        let recovery_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.message.contains("expected identifier after `.`"))
            .collect();
        assert_eq!(
            recovery_errors.len(),
            1,
            "expected exactly one recovery diagnostic, got {:?}",
            result.errors
        );

        // The synthesized identifier inside Missing should have empty text,
        // so the source round-trips without garbage.
        assert_eq!(result.tree.text().to_string(), source);
    }

    #[test]
    fn well_formed_member_access_does_not_emit_missing() {
        // Sanity: real `foo.bar` must still produce zero Missing nodes and
        // zero parse errors. Catches regressions where the recovery path
        // fires on the happy case.
        let source = "func f() { foo.bar }";
        let result = parse_source(source, 0);

        assert_eq!(count_nodes(&result.tree, SyntaxKind::Missing), 0);
        let recovery_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.message.contains("expected identifier after `.`"))
            .collect();
        assert!(recovery_errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn block_recovery_skips_garbage_to_next_statement_boundary() {
        // A garbage line (`@@@`) between two well-formed statements must
        // not poison the rest of the body. Phase 6 wraps the broken stretch
        // in an Error node and the second `let` still parses.
        let source = "func f() { let x = 1; @@@ let y = 2; }";
        let result = parse_source(source, 0);

        // The body should contain at least one Error wrapper from recovery
        // plus two Let statements (one before, one after the garbage).
        let error_nodes = count_nodes(&result.tree, SyntaxKind::Error);
        assert!(
            error_nodes >= 1,
            "expected at least one recovered Error node, tree:\n{:#?}",
            result.tree
        );
        let lets = count_nodes(&result.tree, SyntaxKind::VariableDeclaration);
        assert_eq!(
            lets, 2,
            "both `let` statements must still parse around the garbage; tree:\n{:#?}",
            result.tree
        );

        // Source text must round-trip.
        assert_eq!(result.tree.text().to_string(), source);
    }

    #[test]
    fn block_recovery_preserves_following_statements_for_completion() {
        // Mid-edit shape: a stray punctuation token between two real
        // statements shouldn't wipe out hover/completion on the next
        // line. The second `let` needs to land in the tree so an LSP
        // query at its position can still find a valid declaration.
        //
        // Note: recovery deliberately refuses to consume tokens that
        // could begin an expression (identifiers, literals, `(`, …) so
        // tail expressions like `{ () }` aren't swallowed. Garbage that
        // starts with an expression-starter is a follow-up — see the
        // CHECKLIST under "Parser recovery".
        let source = "func f() { let x = 1; ?? let y = 7; }";
        let result = parse_source(source, 0);

        let lets = count_nodes(&result.tree, SyntaxKind::VariableDeclaration);
        assert_eq!(
            lets, 2,
            "both `let` statements must survive the stray `??`; tree:\n{:#?}",
            result.tree
        );
        assert_eq!(result.tree.text().to_string(), source);
    }

    #[test]
    fn missing_close_paren_recovers_as_a_call() {
        // Phase-4 recovery: cursor mid-edit `foo(1, 2` (no closing paren).
        // The call should still parse (so inference can type the args and
        // completion works on the receiver / next dot) and the source
        // text must round-trip. The `)` is absent from the tree.
        let source = "func f() { foo(1, 2 }";
        let result = parse_source(source, 0);

        assert_eq!(
            count_nodes(&result.tree, SyntaxKind::ExprCall),
            1,
            "the call survives without its `)`, tree:\n{:#?}",
            result.tree
        );
        let recovery_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.message.contains("expected `)`"))
            .collect();
        assert!(
            !recovery_errors.is_empty(),
            "expected an `expected \\`)\\`` diagnostic, got {:?}",
            result.errors
        );
        assert_eq!(result.tree.text().to_string(), source);
    }

    #[test]
    fn missing_member_before_semicolon_recovers() {
        // Cursor mid-edit shape: `foo.;`. The `;` is not a valid member token,
        // so recovery emits Missing and the rest of the body still parses.
        let source = "func f() { foo.; }";
        let result = parse_source(source, 0);

        assert_eq!(count_nodes(&result.tree, SyntaxKind::Missing), 0);
        let recovery_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.message.contains("expected identifier after `.`"))
            .collect();
        assert_eq!(recovery_errors.len(), 1);
        assert_eq!(result.tree.text().to_string(), source);
    }

    /// Count the `expected `;`` diagnostics in a parse result.
    fn missing_semi_errors(result: &ParseResult) -> usize {
        result
            .errors
            .iter()
            .filter(|e| e.message.contains("expected `;`"))
            .count()
    }

    #[test]
    fn missing_semicolon_on_expression_statement_is_reported() {
        // F13: a non-statement-like expression statement without its `;` used
        // to parse to a zero-width `Semicolon` token with no `Missing` node and
        // no diagnostic — the whole body compiled silently. It must now behave
        // exactly like the sibling var-decl path.
        let source = "func f() { foo() bar(); }";
        let result = parse_source(source, 0);

        assert_eq!(
            missing_semi_errors(&result),
            1,
            "expected exactly one `expected `;`` diagnostic, got {:?}",
            result.errors
        );
        assert_eq!(
            count_nodes(&result.tree, SyntaxKind::ExpressionStatement),
            2,
            "both calls are statements; the absent `;` is not synthesized, tree:\n{:#?}",
            result.tree
        );
        assert_eq!(result.tree.text().to_string(), source);
    }

    #[test]
    fn mid_block_statement_like_expr_needs_no_semicolon() {
        // The other half of F13: `if` / `while` / `for` / `match` / `loop`
        // never need a `;`, mid-block included. The block-end lookahead only
        // fires at block end, so mid-block forms reach the synth branch — they
        // must be routed to `BlockItem::StatementExpr`, not reported. `lang/`
        // has ~1439 of these; a false positive here breaks the whole stdlib.
        for body in [
            "if a { b(); } c();",
            "while a { b(); } c();",
            "for x in a { b(); } c();",
            "loop { b(); } c();",
            "match a { _ => { b(); } } c();",
        ] {
            let source = format!("func f() {{ {body} }}");
            let result = parse_source(&source, 0);

            assert_eq!(
                missing_semi_errors(&result),
                0,
                "statement-like `{body}` must not require a `;`, got {:?}",
                result.errors
            );
            assert_eq!(
                count_nodes(&result.tree, SyntaxKind::Missing),
                0,
                "no Missing node expected for `{body}`, tree:\n{:#?}",
                result.tree
            );
            assert_eq!(result.tree.text().to_string(), source);
        }
    }
}
