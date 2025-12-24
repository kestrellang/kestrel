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

use crate::event::{EventSink, TreeBuilder};
use crate::expr::{ExprVariant, emit_expr_variant, expr_parser};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens, to_kestrel_span};
use crate::pattern::{PatternVariant, emit_pattern_variant, pattern_parser};
use crate::stmt::{StmtVariant, emit_stmt_variant};

/// Parser that skips trivia tokens (whitespace and comments)
fn skip_trivia<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| {
            matches!(
                token,
                Token::Whitespace | Token::LineComment | Token::BlockComment
            )
        })
        .repeated()
        .ignored()
}

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

/// Raw parsed data for a guard-let statement
#[derive(Debug, Clone)]
pub struct GuardLetData {
    /// Span of 'guard' keyword
    pub guard_span: Span,
    /// Span of 'let' keyword
    pub let_span: Span,
    /// The pattern being matched
    pub pattern: PatternVariant,
    /// Span of '=' token
    pub equals_span: Span,
    /// The expression being matched against
    pub value: ExprVariant,
    /// Span of 'else' keyword
    pub else_span: Span,
    /// The else block braces and items (must diverge)
    pub else_lbrace: Span,
    /// Items in the else block
    pub else_items: Vec<ElseBlockItem>,
    /// Right brace of the else block
    pub else_rbrace: Span,
}

/// An item in a guard-let else block (simplified - no nested guard-let allowed)
#[derive(Debug, Clone)]
pub enum ElseBlockItem {
    /// A statement (variable declaration or expression with semicolon)
    Statement(StmtVariant),
    /// A statement-like expression (if, while, etc. - no semicolon required)
    StatementExpr(ExprVariant),
    /// A trailing expression
    TrailingExpression(ExprVariant),
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
    /// A guard-let statement (no semicolon required)
    GuardLet(GuardLetData),
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
pub fn code_block_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, CodeBlockData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
        .then(code_block_items_parser())
        .then(
            skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
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
        ExprVariant::If { .. } | ExprVariant::While { .. } | ExprVariant::WhileLet { .. } | ExprVariant::Loop { .. }
    )
}

/// Parser for items inside a guard-let else block.
/// This is a simplified version that doesn't allow nested guard-let statements
/// to avoid recursive parser types.
fn guard_let_else_items_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<ElseBlockItem>, ParserExtra<'tokens>> + Clone {
    // Variable declaration: let/var pattern: Type = expr;
    let var_decl = skip_trivia()
        .ignore_then(
            just(Token::Let)
                .map_with(|_, e| (to_kestrel_span(e.span()), false))
                .or(just(Token::Var).map_with(|_, e| (to_kestrel_span(e.span()), true))),
        )
        .then(pattern_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                .then(crate::ty::ty_parser())
                .or_not(),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
                .then(expr_parser())
                .or_not(),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |(((((mutability_span, is_mutable), pattern), type_annotation), initializer), semicolon)| {
                ElseBlockItem::Statement(StmtVariant::VariableDeclaration(
                    crate::stmt::VariableDeclarationData {
                        mutability_span,
                        is_mutable,
                        pattern,
                        type_annotation,
                        initializer,
                        semicolon,
                    },
                ))
            },
        );

    // Expression-based item: parse expression first, then check for semicolon
    let expr_item = expr_parser()
        .then(
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                .map(Some)
                .or(empty().to(None)),
        )
        .try_map(|(expr, maybe_semi), span| {
            if let Some(semi) = maybe_semi {
                Ok(ElseBlockItem::Statement(StmtVariant::Expression(expr, semi)))
            } else if is_statement_like_expr(&expr) {
                Ok(ElseBlockItem::StatementExpr(expr))
            } else {
                Err(Rich::custom(span, "expected semicolon"))
            }
        });

    let block_item = var_decl.or(expr_item);

    block_item
        .repeated()
        .collect::<Vec<_>>()
        .then(
            expr_parser().map(ElseBlockItem::TrailingExpression).or_not(),
        )
        .map(|(mut items, trailing)| {
            if let Some(expr) = trailing {
                items.push(expr);
            }
            items
        })
}

/// Parser for the items inside a code block
fn code_block_items_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<BlockItem>, ParserExtra<'tokens>> + Clone {
    // We need to handle:
    // 1. Guard-let statements (guard let pattern = expr else { block })
    // 2. Variable declarations (let/var name: Type = expr;)
    // 3. Expression statements (expr;)
    // 4. Statement-like expressions (if, while, loop - no semicolon needed)
    // 5. A final trailing expression (determines the block's value)
    //
    // Strategy:
    // - First try guard-let (starts with 'guard')
    // - Then try variable declarations (starts with let/var)
    // - Otherwise parse an expression, then decide based on what follows:
    //   - If semicolon: expression statement
    //   - If statement-like: statement expression (no semicolon needed)
    //   - Otherwise: fail (will be retried as trailing expression)

    // Guard-let statement: guard let pattern = expr else { block }
    // The else block is parsed inline to avoid recursive parser types
    let guard_let = skip_trivia()
        .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
        .then(skip_trivia().ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span()))))
        .then(pattern_parser())
        .then(skip_trivia().ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))))
        .then(expr_parser())
        .then(skip_trivia().ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span()))))
        .then(skip_trivia().ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))))
        .then(guard_let_else_items_parser())
        .then(skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))))
        .map(|((((((((guard_span, let_span), pattern), equals_span), value), else_span), else_lbrace), else_items), else_rbrace)| {
            BlockItem::GuardLet(GuardLetData {
                guard_span,
                let_span,
                pattern,
                equals_span,
                value,
                else_span,
                else_lbrace,
                else_items,
                else_rbrace,
            })
        });

    // Variable declaration: let/var pattern: Type = expr;
    let var_decl = skip_trivia()
        .ignore_then(
            just(Token::Let)
                .map_with(|_, e| (to_kestrel_span(e.span()), false))
                .or(just(Token::Var).map_with(|_, e| (to_kestrel_span(e.span()), true))),
        )
        .then(pattern_parser())
        .then(
            // Optional type annotation: : Type
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                .then(crate::ty::ty_parser())
                .or_not(),
        )
        .then(
            // Optional initializer: = expr
            skip_trivia()
                .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
                .then(expr_parser())
                .or_not(),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |(((((mutability_span, is_mutable), pattern), type_annotation), initializer), semicolon)| {
                BlockItem::Statement(StmtVariant::VariableDeclaration(
                    crate::stmt::VariableDeclarationData {
                        mutability_span,
                        is_mutable,
                        pattern,
                        type_annotation,
                        initializer,
                        semicolon,
                    },
                ))
            },
        );

    // Expression-based item: parse expression first, then check for semicolon
    let expr_item = expr_parser()
        .then(
            // Check if there's a semicolon (making it a regular statement)
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                .map(Some)
                .or(empty().to(None)),
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
                Err(Rich::custom(span, "expected semicolon"))
            }
        });

    // A block item is a guard-let, variable declaration, or expression-based item
    // Guard-let must come first since it starts with a keyword (guard) that's not
    // valid as the start of a variable declaration or expression
    let block_item = guard_let.or(var_decl).or(expr_item);

    block_item
        .repeated()
        .collect::<Vec<_>>()
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
            BlockItem::GuardLet(guard_data) => {
                emit_guard_let(sink, guard_data);
            }
        }
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace.clone());
    sink.finish_node();
}

/// Emit events for a guard-let statement
fn emit_guard_let(sink: &mut EventSink, data: &GuardLetData) {
    sink.start_node(SyntaxKind::Statement);
    sink.start_node(SyntaxKind::GuardLetStatement);

    // guard keyword
    sink.add_token(SyntaxKind::Guard, data.guard_span.clone());
    // let keyword
    sink.add_token(SyntaxKind::Let, data.let_span.clone());
    // Pattern
    emit_pattern_variant(sink, &data.pattern);
    // = token
    sink.add_token(SyntaxKind::Equals, data.equals_span.clone());
    // Value expression
    emit_expr_variant(sink, &data.value);
    // else keyword
    sink.add_token(SyntaxKind::Else, data.else_span.clone());
    // Else block - emit inline
    sink.start_node(SyntaxKind::CodeBlock);
    sink.add_token(SyntaxKind::LBrace, data.else_lbrace.clone());
    for item in &data.else_items {
        match item {
            ElseBlockItem::Statement(stmt) => {
                emit_stmt_variant(sink, stmt);
            }
            ElseBlockItem::StatementExpr(expr) => {
                sink.start_node(SyntaxKind::Statement);
                sink.start_node(SyntaxKind::ExpressionStatement);
                emit_expr_variant(sink, expr);
                sink.finish_node();
                sink.finish_node();
            }
            ElseBlockItem::TrailingExpression(expr) => {
                emit_expr_variant(sink, expr);
            }
        }
    }
    sink.add_token(SyntaxKind::RBrace, data.else_rbrace.clone());
    sink.finish_node(); // CodeBlock

    sink.finish_node(); // GuardLetStatement
    sink.finish_node(); // Statement
}

/// Parse a code block and emit events
pub fn parse_code_block<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match code_block_parser().parse(input).into_result() {
        Ok(data) => {
            emit_code_block(sink, &data);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), to_kestrel_span(*span));
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

    #[test]
    fn test_block_with_assignment() {
        let source = "{ self.x = x; }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(!block.has_trailing_expression());
    }

    #[test]
    fn test_block_with_multiple_assignments() {
        let source = "{ self.x = x; self.y = y; }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(!block.has_trailing_expression());
    }

}
