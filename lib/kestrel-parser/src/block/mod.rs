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
use crate::expr::{ExprVariant, IfCondition, emit_expr_variant, emit_if_condition, expr_parser};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::parse_and_emit;
use crate::pattern::pattern_parser;
use crate::stmt::{StmtVariant, emit_stmt_variant};

/// Parser that skips trivia tokens (whitespace and comments)
fn skip_trivia<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| {
            matches!(
                token,
                Token::Whitespace | Token::Newline | Token::LineComment | Token::BlockComment
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
/// Supports chains: guard let .Some(x) = a, let .Some(y) = b, x > 0 else { ... }
#[derive(Debug, Clone)]
pub struct GuardLetData {
    /// Span of 'guard' keyword
    pub guard_span: Span,
    /// List of conditions (at least one let-binding, possibly followed by more let-bindings or bool conditions)
    pub conditions: Vec<IfCondition>,
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

/// Parser for items inside a guard-let `else { ... }` block, parameterized
/// by the recursive `expr` and inline `let`/`var` parsers.
///
/// Yields `Vec<ElseBlockItem>`. The items are: variable declarations, regular
/// expression statements (with semicolon), inline statement-like expressions
/// (no semicolon needed for `if`/`while`/`loop`/`for`/`match`/`return`/
/// `throw`/`try`), and an optional trailing expression.
///
/// Reused by inline guard-let inside `expr_parser` and inside closures so the
/// grammar lives in one place.
pub(crate) fn else_block_items_parser<'tokens, P, V>(
    expr: P,
    inline_var_decl: V,
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<ElseBlockItem>, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
    V: Parser<'tokens, ParserInput<'tokens>, StmtVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let else_item = inline_var_decl
        .map(ElseBlockItem::Statement)
        .or(expr
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                    .map(Some)
                    .or(empty().to(None)),
            )
            .try_map(|(e, maybe_semi), _extra| {
                if let Some(semi) = maybe_semi {
                    Ok(ElseBlockItem::Statement(StmtVariant::Expression(e, semi)))
                } else if crate::expr::is_inline_statement_like(&e) {
                    Ok(ElseBlockItem::StatementExpr(e))
                } else {
                    Err(Rich::custom(
                        chumsky::span::Span::new((), 0..0),
                        "expected semicolon",
                    ))
                }
            }));

    else_item
        .repeated()
        .collect::<Vec<_>>()
        .then(expr.map(ElseBlockItem::TrailingExpression).or_not())
        .map(|(mut items, trailing)| {
            if let Some(e) = trailing {
                items.push(e);
            }
            items
        })
        .boxed()
}

/// Parser for an inline `guard let ... else { ... }` block item with chain
/// support. Yields `BlockItem::GuardLet(GuardLetData)`.
///
/// Conditions form a chain starting with at least one `let pattern = expr`,
/// followed by zero or more comma-separated `let` or boolean conditions.
pub(crate) fn guard_let_block_item_parser<'tokens, P, V>(
    expr: P,
    inline_var_decl: V,
) -> impl Parser<'tokens, ParserInput<'tokens>, BlockItem, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
    V: Parser<'tokens, ParserInput<'tokens>, StmtVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let let_condition = skip_trivia()
        .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
        .then(pattern_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(expr.clone())
        .map(
            |(((let_span, pattern), equals_span), value)| IfCondition::Let {
                let_span,
                pattern,
                equals_span,
                value,
            },
        );

    let single_condition = let_condition
        .clone()
        .or(expr.clone().map(IfCondition::Expr));

    let conditions = let_condition
        .then(
            skip_trivia()
                .ignore_then(just(Token::Comma))
                .ignore_then(single_condition)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut conditions = vec![first];
            conditions.extend(rest);
            conditions
        });

    let else_items = else_block_items_parser(expr, inline_var_decl);

    skip_trivia()
        .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
        .then(conditions)
        .then(
            skip_trivia()
                .ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(else_items)
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |(((((guard_span, conditions), else_span), else_lbrace), else_items), else_rbrace)| {
                BlockItem::GuardLet(GuardLetData {
                    guard_span,
                    conditions,
                    else_span,
                    else_lbrace,
                    else_items,
                    else_rbrace,
                })
            },
        )
        .boxed()
}

/// Parser for the items inside an inline code block. Yields `Vec<BlockItem>`
/// with optional trailing expression already attached.
///
/// Item kinds: guard-let, variable declaration (via `inline_var_decl`),
/// expression statement (with semicolon), statement-like expression (no
/// semicolon needed), trailing expression. Does NOT include `deinit
/// identifier;` — that is only valid in top-level blocks parsed via
/// [`code_block_parser`].
pub(crate) fn block_items_parser<'tokens, P, V>(
    expr: P,
    inline_var_decl: V,
) -> impl Parser<'tokens, ParserInput<'tokens>, Vec<BlockItem>, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
    V: Parser<'tokens, ParserInput<'tokens>, StmtVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    let guard_let = guard_let_block_item_parser(expr.clone(), inline_var_decl.clone());
    let stmt_decl = inline_var_decl.map(BlockItem::Statement);
    let expr_item = expr
        .clone()
        .then(
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                .or_not(),
        )
        .try_map(|(e, maybe_semi), _extra| {
            if let Some(semi) = maybe_semi {
                Ok(BlockItem::Statement(StmtVariant::Expression(e, semi)))
            } else if crate::expr::is_inline_statement_like(&e) {
                Ok(BlockItem::StatementExpr(e))
            } else {
                Err(Rich::custom(
                    chumsky::span::Span::new((), 0..0),
                    "expected semicolon",
                ))
            }
        });

    let block_item = guard_let.or(stmt_decl).or(expr_item);

    block_item
        .repeated()
        .collect::<Vec<_>>()
        .then(expr.map(BlockItem::TrailingExpression).or_not())
        .map(|(mut items, trailing)| {
            if let Some(e) = trailing {
                items.push(e);
            }
            items
        })
        .boxed()
}

/// Parser for an inline `{ ... }` code block, parameterized by the `lbrace`
/// parser, the recursive `expr` handle, and the inline `let`/`var` parser.
///
/// Returns `CodeBlockData`. Used as the body of `if`/`while`/`loop`/`for`
/// expressions and `match` arms inside `expr_parser`.
pub(crate) fn inline_code_block_parser<'tokens, L, P, V>(
    lbrace_parser: L,
    expr: P,
    inline_var_decl: V,
) -> impl Parser<'tokens, ParserInput<'tokens>, CodeBlockData, ParserExtra<'tokens>> + Clone
where
    L: Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone + 'tokens,
    P: Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone + 'tokens,
    V: Parser<'tokens, ParserInput<'tokens>, StmtVariant, ParserExtra<'tokens>> + Clone + 'tokens,
{
    lbrace_parser
        .then(block_items_parser(expr, inline_var_decl))
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(|((lbrace, items), rbrace)| CodeBlockData {
            lbrace,
            items,
            rbrace,
        })
        .boxed()
}

/// Parser for a code block
///
/// Syntax: { statement* expression? }
///
/// The parser handles:
/// - Empty blocks: { }
/// - Statement-only blocks: { stmt; stmt; }
/// - Trailing expression blocks: { stmt; expr }
pub fn code_block_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, CodeBlockData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
        .then(code_block_items_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(|((lbrace, items), rbrace)| CodeBlockData {
            lbrace,
            items,
            rbrace,
        })
        .boxed()
}

/// Check if an expression variant is "statement-like" (doesn't require semicolon)
fn is_statement_like_expr(expr: &ExprVariant) -> bool {
    matches!(
        expr,
        ExprVariant::If { .. }
            | ExprVariant::While { .. }
            | ExprVariant::WhileLet { .. }
            | ExprVariant::Loop { .. }
            | ExprVariant::For { .. }
            | ExprVariant::Match { .. }
    )
}

/// Parser for items inside a guard-let else block.
/// This is a simplified version that doesn't allow nested guard-let statements
/// to avoid recursive parser types.
fn guard_let_else_items_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<ElseBlockItem>, ParserExtra<'tokens>> + Clone {
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
            |(
                ((((mutability_span, is_mutable), pattern), type_annotation), initializer),
                semicolon,
            )| {
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
                Ok(ElseBlockItem::Statement(StmtVariant::Expression(
                    expr, semi,
                )))
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
            expr_parser()
                .map(ElseBlockItem::TrailingExpression)
                .or_not(),
        )
        .map(|(mut items, trailing)| {
            if let Some(expr) = trailing {
                items.push(expr);
            }
            items
        })
        .boxed()
}

/// Parser for the items inside a code block
fn code_block_items_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<BlockItem>, ParserExtra<'tokens>> + Clone {
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

    // Guard-let statement: guard let pattern = expr, ... else { block }
    // Supports chains: guard let .Some(x) = a, let .Some(y) = b, x > 0 else { ... }
    // The else block is parsed inline to avoid recursive parser types

    // Single let condition: let pattern = expr
    let guard_let_condition = skip_trivia()
        .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
        .then(pattern_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(expr_parser())
        .map(
            |(((let_span, pattern), equals_span), value)| IfCondition::Let {
                let_span,
                pattern,
                equals_span,
                value,
            },
        );

    // Single condition: either let-binding or boolean expression
    let guard_single_condition = guard_let_condition
        .clone()
        .or(expr_parser().map(IfCondition::Expr));

    // Condition list: first must be let, followed by comma-separated conditions
    let guard_conditions = guard_let_condition
        .then(
            skip_trivia()
                .ignore_then(just(Token::Comma))
                .ignore_then(guard_single_condition)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut conditions = vec![first];
            conditions.extend(rest);
            conditions
        });

    let guard_let = skip_trivia()
        .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
        .then(guard_conditions)
        .then(
            skip_trivia()
                .ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(guard_let_else_items_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |(((((guard_span, conditions), else_span), else_lbrace), else_items), else_rbrace)| {
                BlockItem::GuardLet(GuardLetData {
                    guard_span,
                    conditions,
                    else_span,
                    else_lbrace,
                    else_items,
                    else_rbrace,
                })
            },
        );

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
            |(
                ((((mutability_span, is_mutable), pattern), type_annotation), initializer),
                semicolon,
            )| {
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

    // Deinit statement: deinit identifier;
    let deinit_stmt = skip_trivia()
        .ignore_then(just(Token::Deinit).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            skip_trivia()
                .ignore_then(just(Token::Identifier).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(|((deinit_span, identifier_span), semicolon)| {
            BlockItem::Statement(StmtVariant::Deinit(crate::stmt::DeinitStatementData {
                deinit_span,
                identifier_span,
                semicolon,
            }))
        });

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

    // A block item is a guard-let, deinit statement, variable declaration, or expression-based item
    // Guard-let and deinit must come first since they start with keywords
    let block_item = guard_let.or(deinit_stmt).or(var_decl).or(expr_item);

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
        .boxed()
}

/// Emit events for a code block
pub fn emit_code_block(sink: &mut EventSink, data: &CodeBlockData) {
    sink.start_node(SyntaxKind::CodeBlock);
    sink.add_token(SyntaxKind::LBrace, data.lbrace.clone());

    for item in &data.items {
        match item {
            BlockItem::Statement(stmt) => {
                emit_stmt_variant(sink, stmt);
            },
            BlockItem::StatementExpr(expr) => {
                // Statement-like expressions are wrapped in Statement node
                // but don't have a semicolon
                sink.start_node(SyntaxKind::Statement);
                sink.start_node(SyntaxKind::ExpressionStatement);
                emit_expr_variant(sink, expr);
                sink.finish_node(); // ExpressionStatement
                sink.finish_node(); // Statement
            },
            BlockItem::TrailingExpression(expr) => {
                emit_expr_variant(sink, expr);
            },
            BlockItem::GuardLet(guard_data) => {
                emit_guard_let(sink, guard_data);
            },
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
    // Emit each condition in the chain
    for condition in &data.conditions {
        emit_if_condition(sink, condition, SyntaxKind::GuardLetCondition);
    }
    // else keyword
    sink.add_token(SyntaxKind::Else, data.else_span.clone());
    // Else block - emit inline
    sink.start_node(SyntaxKind::CodeBlock);
    sink.add_token(SyntaxKind::LBrace, data.else_lbrace.clone());
    for item in &data.else_items {
        match item {
            ElseBlockItem::Statement(stmt) => {
                emit_stmt_variant(sink, stmt);
            },
            ElseBlockItem::StatementExpr(expr) => {
                sink.start_node(SyntaxKind::Statement);
                sink.start_node(SyntaxKind::ExpressionStatement);
                emit_expr_variant(sink, expr);
                sink.finish_node();
                sink.finish_node();
            },
            ElseBlockItem::TrailingExpression(expr) => {
                emit_expr_variant(sink, expr);
            },
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
    parse_and_emit!(
        source,
        tokens,
        sink,
        code_block_parser(),
        |sink, data: CodeBlockData| emit_code_block(sink, &data)
    );
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

        let mut sink = EventSink::new(0);
        parse_code_block(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        CodeBlock {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
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

    #[test]
    fn test_block_with_deinit_statement() {
        let source = "{ deinit x; }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(!block.has_trailing_expression());

        // Check that we have a DeinitStatement - look inside Statement nodes
        let has_deinit = block
            .syntax
            .children()
            .filter(|child| child.kind() == SyntaxKind::Statement)
            .any(|stmt| {
                stmt.children()
                    .any(|c| c.kind() == SyntaxKind::DeinitStatement)
            });
        assert!(has_deinit, "Expected DeinitStatement in block");
    }

    #[test]
    fn test_block_with_deinit_and_other_statements() {
        let source = "{ let x: Int = 0; deinit x; }";
        let block = parse_block_from_source(source);

        assert!(!block.is_empty());
        assert!(!block.has_trailing_expression());

        // Check that we have both a variable declaration and a deinit statement
        let statements: Vec<_> = block
            .syntax
            .children()
            .filter(|c| c.kind() == SyntaxKind::Statement)
            .collect();
        assert_eq!(statements.len(), 2, "Expected 2 statements");
    }
}
