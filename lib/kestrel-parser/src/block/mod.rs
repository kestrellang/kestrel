//! Code block parsing
//!
//! This module provides parsing for Kestrel code blocks.
//! A code block has the form: { statement; statement; expression }
//!
//! The trailing expression (without semicolon) determines the block's value and type.
//! If the last item has a semicolon, the block evaluates to unit ().

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::skip_trivia;
use crate::event::{EventSink, TreeBuilder};
use crate::expr::{ExprVariant, emit_expr_variant, expr_parser};
use crate::stmt::{StmtVariant, emit_stmt_variant, stmt_parser};

/// Represents a code block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl CodeBlock {
    /// Create a new CodeBlock from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Check if the code block is empty (just braces)
    pub fn is_empty(&self) -> bool {
        // Check if there are no statement or expression children
        self.syntax
            .children()
            .filter(|child| matches!(child.kind(), SyntaxKind::Statement | SyntaxKind::Expression))
            .count()
            == 0
    }

    /// Check if the block has a trailing expression (no semicolon)
    pub fn has_trailing_expression(&self) -> bool {
        // If the last child (excluding closing brace) is an Expression, it's trailing
        self.syntax
            .children()
            .filter(|child| matches!(child.kind(), SyntaxKind::Statement | SyntaxKind::Expression))
            .last()
            .map(|child| child.kind() == SyntaxKind::Expression)
            .unwrap_or(false)
    }
}

/// An item in a code block - either a statement or a trailing expression
#[derive(Debug, Clone)]
pub enum BlockItem {
    /// A statement (has semicolon)
    Statement(StmtVariant),
    /// A statement-like expression (if, while, etc. - no semicolon required)
    StatementExpr(ExprVariant),
    /// A trailing expression (no semicolon, determines block value)
    TrailingExpression(ExprVariant),
}

/// Raw parsed data for a code block
#[derive(Debug, Clone)]
pub struct CodeBlockData {
    /// Left brace span
    pub lbrace: Span,
    /// Items in the block (statements and optional trailing expression)
    pub items: Vec<BlockItem>,
    /// Right brace span
    pub rbrace: Span,
}

/// Parser for a code block
///
/// Syntax: { statement* expression? }
///
/// The parser handles:
/// - Empty blocks: { }
/// - Statement-only blocks: { stmt; stmt; }
/// - Trailing expression blocks: { stmt; expr }
pub fn code_block_parser() -> impl Parser<Token, CodeBlockData, Error = Simple<Token>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBrace).map_with_span(|_, span| Span::from(span)))
        .then(code_block_items_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with_span(|_, span| Span::from(span))),
        )
        .map(|((lbrace, items), rbrace)| CodeBlockData {
            lbrace,
            items,
            rbrace,
        })
}

/// Check if an expression variant is "statement-like" (doesn't require semicolon)
fn is_statement_like_expr(expr: &ExprVariant) -> bool {
    matches!(
        expr,
        ExprVariant::If { .. } | ExprVariant::While { .. } | ExprVariant::Loop { .. }
    )
}

/// Parser for the items inside a code block
fn code_block_items_parser() -> impl Parser<Token, Vec<BlockItem>, Error = Simple<Token>> + Clone {
    // We need to handle:
    // 1. Regular statements (let/var declarations, or expressions with semicolons)
    // 2. Statement-like expressions (if, while, etc.) that don't need semicolons
    // 3. A final trailing expression (determines the block's value)
    //
    // Strategy:
    // - Repeatedly parse items that are either statements OR statement-like expressions
    // - Then optionally parse a trailing expression
    //
    // The tricky part: we need to distinguish between:
    // - `if cond { } else { }` followed by more items (statement-like, continue parsing)
    // - `if cond { } else { }` at the end (trailing expression)
    // - `if cond { }` followed by more items (statement-like, continue parsing)
    // - `if cond { }` at the end (trailing expression with unit type)

    // An "item" is either:
    // 1. A regular statement (with semicolon)
    // 2. A statement-like expression followed by more content (not at end)

    // Parse a single block item (not the trailing expression)
    let block_item = stmt_parser().map(BlockItem::Statement).or(
        // Try to parse a statement-like expression
        // We need to look ahead to see if there's more content after it
        expr_parser()
            .then(
                // Check if there's a semicolon (making it a regular statement)
                skip_trivia()
                    .ignore_then(just(Token::Semicolon).map_with_span(|_, span| Span::from(span)))
                    .map(Some)
                    .or(empty().map(|_| None)),
            )
            .try_map(|(expr, maybe_semi), span| {
                if let Some(semi) = maybe_semi {
                    // Has semicolon - it's a regular expression statement
                    Ok(BlockItem::Statement(StmtVariant::Expression(expr, semi)))
                } else if is_statement_like_expr(&expr) {
                    // No semicolon but it's statement-like - OK
                    Ok(BlockItem::StatementExpr(expr))
                } else {
                    // No semicolon and not statement-like - fail, let it be parsed as trailing
                    Err(Simple::custom(span, "expected semicolon"))
                }
            }),
    );

    block_item
        .repeated()
        .then(
            // Optional trailing expression (any expression without semicolon at the end)
            expr_parser().map(BlockItem::TrailingExpression).or_not(),
        )
        .map(|(mut items, trailing)| {
            if let Some(expr) = trailing {
                items.push(expr);
            }
            items
        })
}

/// Emit events for a code block
pub fn emit_code_block(sink: &mut EventSink, data: &CodeBlockData) {
    sink.start_node(SyntaxKind::CodeBlock);
    sink.add_token(SyntaxKind::LBrace, data.lbrace.clone());

    for item in &data.items {
        match item {
            BlockItem::Statement(stmt) => {
                emit_stmt_variant(sink, stmt);
            }
            BlockItem::StatementExpr(expr) => {
                // Statement-like expressions are wrapped in Statement node
                // but don't have a semicolon
                sink.start_node(SyntaxKind::Statement);
                sink.start_node(SyntaxKind::ExpressionStatement);
                emit_expr_variant(sink, expr);
                sink.finish_node(); // ExpressionStatement
                sink.finish_node(); // Statement
            }
            BlockItem::TrailingExpression(expr) => {
                emit_expr_variant(sink, expr);
            }
        }
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace.clone());
    sink.finish_node();
}

/// Parse a code block and emit events
pub fn parse_code_block<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let end_pos = source.len();
    let tokens_with_range = tokens.map(|(tok, span)| (tok, span.range()));
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens_with_range);

    match code_block_parser().parse(stream) {
        Ok(data) => {
            emit_code_block(sink, &data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), Span::from(span));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    fn parse_block_from_source(source: &str) -> CodeBlock {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let mut sink = EventSink::new();
        parse_code_block(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        CodeBlock {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    #[test]
    fn test_empty_block() {
        let source = "{ }";
        let block = parse_block_from_source(source);

        assert!(block.is_empty());
        assert!(!block.has_trailing_expression());
    }

    #[test]
    fn test_block_with_trailing_expression() {
        let source = "{ () }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(block.has_trailing_expression());
    }

    #[test]
    fn test_block_with_statement() {
        let source = "{ (); }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(!block.has_trailing_expression());
    }

    #[test]
    fn test_block_with_variable_declaration() {
        let source = "{ let x: Int = (); }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(!block.has_trailing_expression());
    }

    #[test]
    fn test_block_with_statements_and_trailing_expr() {
        let source = "{ let x: Int = (); () }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(block.has_trailing_expression());
    }
}
