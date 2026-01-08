//! Statement parsing
//!
//! This module provides parsing for Kestrel statements.
//! Currently supports:
//! - Variable declarations: let/var pattern: Type = expr;
//! - Expression statements: expr;

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::event::{EventSink, TreeBuilder};
use crate::expr::{ExprVariant, emit_expr_variant, expr_parser};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens, to_kestrel_span};
use crate::pattern::{PatternVariant, emit_pattern_variant, pattern_parser};
use crate::ty::{TyVariant, emit_ty_variant, ty_parser};

/// Represents a statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl Statement {
    /// Create a new Statement from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the kind of this statement
    pub fn kind(&self) -> SyntaxKind {
        self.syntax
            .children()
            .next()
            .map(|child| child.kind())
            .unwrap_or(SyntaxKind::Error)
    }

    /// Check if this is a variable declaration
    pub fn is_variable_declaration(&self) -> bool {
        self.kind() == SyntaxKind::VariableDeclaration
    }

    /// Check if this is an expression statement
    pub fn is_expression_statement(&self) -> bool {
        self.kind() == SyntaxKind::ExpressionStatement
    }
}

/// Raw parsed data for a variable declaration
#[derive(Debug, Clone)]
pub struct VariableDeclarationData {
    /// Span of let/var keyword
    pub mutability_span: Span,
    /// Whether this is mutable (var) or not (let)
    pub is_mutable: bool,
    /// The pattern being bound
    pub pattern: PatternVariant,
    /// Optional type annotation: (colon_span, type)
    pub type_annotation: Option<(Span, TyVariant)>,
    /// Optional initializer: (equals_span, expression)
    pub initializer: Option<(Span, ExprVariant)>,
    /// Semicolon span
    pub semicolon: Span,
}

/// Raw parsed data for a deinit statement: deinit identifier;
#[derive(Debug, Clone)]
pub struct DeinitStatementData {
    /// Span of 'deinit' keyword
    pub deinit_span: Span,
    /// Span of the identifier being deinited
    pub identifier_span: Span,
    /// Semicolon span
    pub semicolon: Span,
}

/// Internal enum to distinguish between statement variants during parsing
#[derive(Debug, Clone)]
pub enum StmtVariant {
    /// Variable declaration: let/var name: Type = expr;
    VariableDeclaration(VariableDeclarationData),
    /// Expression statement: expr;
    Expression(ExprVariant, Span), // (expression, semicolon_span)
    /// Deinit statement: deinit identifier;
    Deinit(DeinitStatementData),
}

/// Parser that skips trivia tokens
fn skip_trivia<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
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

/// Parser for variable declaration
///
/// Syntax: let/var pattern (: Type)? (= expr)? ;
fn variable_declaration_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, VariableDeclarationData, ParserExtra<'tokens>> + Clone
{
    skip_trivia()
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
                .then(ty_parser())
                .map(|(colon, ty)| (colon, ty))
                .or_not(),
        )
        .then(
            // Optional initializer: = expr
            skip_trivia()
                .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
                .then(expr_parser())
                .map(|(eq, expr)| (eq, expr))
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
                VariableDeclarationData {
                    mutability_span,
                    is_mutable,
                    pattern,
                    type_annotation,
                    initializer,
                    semicolon,
                }
            },
        )
}

/// Parser for expression statement
///
/// Syntax: expr ;
fn expression_statement_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (ExprVariant, Span), ParserExtra<'tokens>> + Clone {
    expr_parser().then(
        skip_trivia()
            .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span()))),
    )
}

/// Parser for deinit statement
///
/// Syntax: deinit identifier ;
fn deinit_statement_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, DeinitStatementData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Deinit).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            skip_trivia()
                .ignore_then(just(Token::Identifier).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .then(
            skip_trivia()
                .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span()))),
        )
        .map(
            |((deinit_span, identifier_span), semicolon)| DeinitStatementData {
                deinit_span,
                identifier_span,
                semicolon,
            },
        )
}

/// Parser for statements
///
/// Currently supports:
/// - Variable declarations: let/var name: Type = expr;
/// - Expression statements: expr;
/// - Deinit statements: deinit identifier;
pub fn stmt_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, StmtVariant, ParserExtra<'tokens>> + Clone {
    // Variable declaration starts with let or var
    let var_decl = variable_declaration_parser().map(StmtVariant::VariableDeclaration);

    // Deinit statement: deinit identifier;
    let deinit_stmt = deinit_statement_parser().map(StmtVariant::Deinit);

    // Expression statement is any expression followed by semicolon
    let expr_stmt =
        expression_statement_parser().map(|(expr, semi)| StmtVariant::Expression(expr, semi));

    // Try variable declaration first, then deinit statement, then expression statement
    var_decl.or(deinit_stmt).or(expr_stmt)
}

/// Emit events for any statement variant
pub fn emit_stmt_variant(sink: &mut EventSink, variant: &StmtVariant) {
    match variant {
        StmtVariant::VariableDeclaration(data) => {
            emit_variable_declaration(sink, data);
        }
        StmtVariant::Expression(expr, semicolon) => {
            emit_expression_statement(sink, expr, semicolon.clone());
        }
        StmtVariant::Deinit(data) => {
            emit_deinit_statement(sink, data);
        }
    }
}

/// Emit events for a variable declaration
fn emit_variable_declaration(sink: &mut EventSink, data: &VariableDeclarationData) {
    sink.start_node(SyntaxKind::Statement);
    sink.start_node(SyntaxKind::VariableDeclaration);

    // let/var keyword
    if data.is_mutable {
        sink.add_token(SyntaxKind::Var, data.mutability_span.clone());
    } else {
        sink.add_token(SyntaxKind::Let, data.mutability_span.clone());
    }

    // Pattern
    emit_pattern_variant(sink, &data.pattern);

    // Optional type annotation
    if let Some((colon_span, ty)) = &data.type_annotation {
        sink.add_token(SyntaxKind::Colon, colon_span.clone());
        emit_ty_variant(sink, ty);
    }

    // Optional initializer
    if let Some((eq_span, expr)) = &data.initializer {
        sink.add_token(SyntaxKind::Equals, eq_span.clone());
        emit_expr_variant(sink, expr);
    }

    // Semicolon
    sink.add_token(SyntaxKind::Semicolon, data.semicolon.clone());

    sink.finish_node(); // Finish VariableDeclaration
    sink.finish_node(); // Finish Statement
}

/// Emit events for an expression statement
fn emit_expression_statement(sink: &mut EventSink, expr: &ExprVariant, semicolon: Span) {
    sink.start_node(SyntaxKind::Statement);
    sink.start_node(SyntaxKind::ExpressionStatement);

    emit_expr_variant(sink, expr);
    sink.add_token(SyntaxKind::Semicolon, semicolon);

    sink.finish_node(); // Finish ExpressionStatement
    sink.finish_node(); // Finish Statement
}

/// Emit events for a deinit statement
fn emit_deinit_statement(sink: &mut EventSink, data: &DeinitStatementData) {
    sink.start_node(SyntaxKind::Statement);
    sink.start_node(SyntaxKind::DeinitStatement);

    sink.add_token(SyntaxKind::Deinit, data.deinit_span.clone());
    sink.add_token(SyntaxKind::Identifier, data.identifier_span.clone());
    sink.add_token(SyntaxKind::Semicolon, data.semicolon.clone());

    sink.finish_node(); // Finish DeinitStatement
    sink.finish_node(); // Finish Statement
}

/// Parse a statement and emit events
pub fn parse_stmt<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match stmt_parser().parse(input).into_result() {
        Ok(variant) => {
            emit_stmt_variant(sink, &variant);
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

    fn parse_stmt_from_source(source: &str) -> Statement {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let mut sink = EventSink::new();
        parse_stmt(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        Statement {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    #[test]
    fn test_let_declaration_simple() {
        let source = "let x: Int = ();";
        let stmt = parse_stmt_from_source(source);

        assert!(stmt.is_variable_declaration());
    }

    #[test]
    fn test_var_declaration_simple() {
        let source = "var y: Bool = ();";
        let stmt = parse_stmt_from_source(source);

        assert!(stmt.is_variable_declaration());
    }

    #[test]
    fn test_let_with_array_type() {
        let source = "let x: [Int] = [];";
        let stmt = parse_stmt_from_source(source);
        assert!(stmt.is_variable_declaration());
    }

    #[test]
    fn test_expression_statement() {
        let source = "();";
        let stmt = parse_stmt_from_source(source);

        assert!(stmt.is_expression_statement());
    }
}
