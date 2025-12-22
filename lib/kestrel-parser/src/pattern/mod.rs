//! Pattern parsing
//!
//! This module provides parsing for Kestrel patterns used in match expressions
//! and let bindings.
//!
//! Currently supports:
//! - Wildcard patterns: `_`
//! - Binding patterns: `name` or `var name`
//! - Tuple patterns: `(p1, p2, ...)`
//! - Literal patterns: `42`, `"hello"`, `'c'`, `true`
//! - Enum patterns: `.Case` or `.Case(label)` or `.Case(label: pattern)`

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::common::skip_trivia;
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens, to_kestrel_span, ParserExtra, ParserInput};

/// Represents a pattern syntax node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl Pattern {
    /// Create a new Pattern from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the kind of this pattern
    pub fn kind(&self) -> SyntaxKind {
        self.syntax
            .children()
            .next()
            .map(|child| child.kind())
            .unwrap_or(SyntaxKind::Error)
    }

    /// Check if this is a wildcard pattern
    pub fn is_wildcard(&self) -> bool {
        self.kind() == SyntaxKind::WildcardPattern
    }

    /// Check if this is a binding pattern
    pub fn is_binding(&self) -> bool {
        self.kind() == SyntaxKind::BindingPattern
    }

    /// Check if this is a tuple pattern
    pub fn is_tuple(&self) -> bool {
        self.kind() == SyntaxKind::TuplePattern
    }

    /// Check if this is a literal pattern
    pub fn is_literal(&self) -> bool {
        self.kind() == SyntaxKind::LiteralPattern
    }

    /// Check if this is an enum pattern
    pub fn is_enum(&self) -> bool {
        self.kind() == SyntaxKind::EnumPattern
    }
}

/// Data for a single enum pattern argument
#[derive(Debug, Clone)]
pub struct EnumPatternArgData {
    /// Label name (identifier)
    pub label: Span,
    /// Optional colon followed by pattern
    pub binding: Option<(Span, PatternVariant)>,
}

/// Internal enum to distinguish between pattern variants during parsing
#[derive(Debug, Clone)]
pub enum PatternVariant {
    /// Wildcard pattern: `_`
    Wildcard(Span),
    /// Binding pattern: `name` or `var name`
    Binding {
        /// Optional `var` keyword span (if mutable)
        var_span: Option<Span>,
        /// Name identifier span
        name_span: Span,
    },
    /// Tuple pattern: `(p1, p2, ...)`
    Tuple {
        lparen: Span,
        elements: Vec<PatternVariant>,
        rparen: Span,
    },
    /// Literal pattern: integer, float, string, bool, char
    Literal(LiteralPatternKind),
    /// Enum pattern: `.Case` or `.Case(args)`
    Enum {
        dot: Span,
        case_name: Span,
        /// Optional argument list
        arguments: Option<(Span, Vec<EnumPatternArgData>, Span)>, // (lparen, args, rparen)
    },
    /// Error pattern (for error recovery)
    Error(Span),
}

/// Kind of literal in a literal pattern
#[derive(Debug, Clone)]
pub enum LiteralPatternKind {
    Integer(Span),
    Float(Span),
    String(Span),
    Bool(Span),
}

/// Parser for patterns
///
/// Uses boxed() on recursive sub-parsers to manage compile time.
pub fn pattern_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone {
    recursive(|pattern| {
        // Wildcard pattern: _
        let wildcard = skip_trivia()
            .ignore_then(just(Token::Underscore).map_with(|_, e| to_kestrel_span(e.span())))
            .map(PatternVariant::Wildcard);

        // Literal patterns
        let integer_literal = skip_trivia()
            .ignore_then(select! { Token::Integer = e => to_kestrel_span(e.span()) })
            .map(|span| PatternVariant::Literal(LiteralPatternKind::Integer(span)));

        let float_literal = skip_trivia()
            .ignore_then(select! { Token::Float = e => to_kestrel_span(e.span()) })
            .map(|span| PatternVariant::Literal(LiteralPatternKind::Float(span)));

        let string_literal = skip_trivia()
            .ignore_then(select! { Token::String = e => to_kestrel_span(e.span()) })
            .map(|span| PatternVariant::Literal(LiteralPatternKind::String(span)));

        let bool_literal = skip_trivia()
            .ignore_then(select! { Token::Boolean = e => to_kestrel_span(e.span()) })
            .map(|span| PatternVariant::Literal(LiteralPatternKind::Bool(span)));

        let literal = float_literal
            .or(integer_literal)
            .or(string_literal)
            .or(bool_literal);

        // Binding pattern: `var name` (mutable) or `name` (immutable)
        // Need to be careful to distinguish from wildcards and literals
        let mutable_binding = skip_trivia()
            .ignore_then(just(Token::Var).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                skip_trivia()
                    .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) }),
            )
            .map(|(var_span, name_span)| PatternVariant::Binding {
                var_span: Some(var_span),
                name_span,
            });

        let immutable_binding = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .map(|name_span| PatternVariant::Binding {
                var_span: None,
                name_span,
            });

        // Tuple pattern: (p1, p2, ...)
        let tuple_pattern = skip_trivia()
            .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                pattern
                    .clone()
                    .separated_by(
                        skip_trivia()
                            .ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))),
                    )
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .map(|((lparen, elements), rparen)| PatternVariant::Tuple {
                lparen,
                elements,
                rparen,
            })
            .boxed();

        // Enum pattern argument: `label` or `label: pattern`
        let enum_arg = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(pattern.clone())
                    .or_not(),
            )
            .map(|(label, binding)| EnumPatternArgData { label, binding });

        // Enum pattern: `.Case` or `.Case(args)`
        let enum_pattern = skip_trivia()
            .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                skip_trivia()
                    .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) }),
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(
                        enum_arg
                            .separated_by(
                                skip_trivia().ignore_then(
                                    just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())),
                                ),
                            )
                            .allow_trailing()
                            .collect::<Vec<_>>(),
                    )
                    .then(
                        skip_trivia().ignore_then(
                            just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())),
                        ),
                    )
                    .map(|((lparen, args), rparen)| (lparen, args, rparen))
                    .or_not(),
            )
            .map(|((dot, case_name), arguments)| PatternVariant::Enum {
                dot,
                case_name,
                arguments,
            });

        // Order matters: try more specific patterns first
        // - wildcard (single underscore)
        // - literal (numbers, strings, bools)
        // - enum pattern (starts with dot)
        // - mutable binding (starts with var)
        // - tuple pattern (starts with lparen)
        // - immutable binding (identifier - least specific)
        wildcard
            .or(literal)
            .or(enum_pattern)
            .or(mutable_binding)
            .or(tuple_pattern)
            .or(immutable_binding)
    })
}

/// Emit events for a pattern variant
pub fn emit_pattern_variant(sink: &mut EventSink, variant: &PatternVariant) {
    sink.start_node(SyntaxKind::Pattern);
    match variant {
        PatternVariant::Wildcard(span) => {
            sink.start_node(SyntaxKind::WildcardPattern);
            sink.add_token(SyntaxKind::Underscore, span.clone());
            sink.finish_node();
        }
        PatternVariant::Binding { var_span, name_span } => {
            sink.start_node(SyntaxKind::BindingPattern);
            if let Some(var) = var_span {
                sink.add_token(SyntaxKind::Var, var.clone());
            }
            sink.add_token(SyntaxKind::Identifier, name_span.clone());
            sink.finish_node();
        }
        PatternVariant::Tuple {
            lparen,
            elements,
            rparen,
        } => {
            sink.start_node(SyntaxKind::TuplePattern);
            sink.add_token(SyntaxKind::LParen, lparen.clone());
            for (i, element) in elements.iter().enumerate() {
                sink.start_node(SyntaxKind::TuplePatternElement);
                emit_pattern_variant_inner(sink, element);
                sink.finish_node();
                // Add comma after each element except the last
                // Note: We don't track commas in the variant, but that's okay for now
                if i < elements.len() - 1 {
                    // Comma spans are not stored - tree builder will infer
                }
            }
            sink.add_token(SyntaxKind::RParen, rparen.clone());
            sink.finish_node();
        }
        PatternVariant::Literal(kind) => {
            sink.start_node(SyntaxKind::LiteralPattern);
            match kind {
                LiteralPatternKind::Integer(span) => {
                    sink.add_token(SyntaxKind::Integer, span.clone());
                }
                LiteralPatternKind::Float(span) => {
                    sink.add_token(SyntaxKind::Float, span.clone());
                }
                LiteralPatternKind::String(span) => {
                    sink.add_token(SyntaxKind::String, span.clone());
                }
                LiteralPatternKind::Bool(span) => {
                    sink.add_token(SyntaxKind::Boolean, span.clone());
                }
            }
            sink.finish_node();
        }
        PatternVariant::Enum {
            dot,
            case_name,
            arguments,
        } => {
            sink.start_node(SyntaxKind::EnumPattern);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::Identifier, case_name.clone());
            if let Some((lparen, args, rparen)) = arguments {
                sink.add_token(SyntaxKind::LParen, lparen.clone());
                for arg in args {
                    sink.start_node(SyntaxKind::EnumPatternArg);
                    sink.add_token(SyntaxKind::Identifier, arg.label.clone());
                    if let Some((colon, pattern)) = &arg.binding {
                        sink.add_token(SyntaxKind::Colon, colon.clone());
                        emit_pattern_variant_inner(sink, pattern);
                    }
                    sink.finish_node();
                }
                sink.add_token(SyntaxKind::RParen, rparen.clone());
            }
            sink.finish_node();
        }
        PatternVariant::Error(span) => {
            sink.start_node(SyntaxKind::ErrorPattern);
            sink.error_at("Invalid pattern".to_string(), span.clone());
            sink.finish_node();
        }
    }
    sink.finish_node(); // Finish Pattern wrapper
}

/// Emit events for a pattern variant without the Pattern wrapper
/// Used for nested patterns (e.g., in tuple elements)
fn emit_pattern_variant_inner(sink: &mut EventSink, variant: &PatternVariant) {
    match variant {
        PatternVariant::Wildcard(span) => {
            sink.start_node(SyntaxKind::WildcardPattern);
            sink.add_token(SyntaxKind::Underscore, span.clone());
            sink.finish_node();
        }
        PatternVariant::Binding { var_span, name_span } => {
            sink.start_node(SyntaxKind::BindingPattern);
            if let Some(var) = var_span {
                sink.add_token(SyntaxKind::Var, var.clone());
            }
            sink.add_token(SyntaxKind::Identifier, name_span.clone());
            sink.finish_node();
        }
        PatternVariant::Tuple {
            lparen,
            elements,
            rparen,
        } => {
            sink.start_node(SyntaxKind::TuplePattern);
            sink.add_token(SyntaxKind::LParen, lparen.clone());
            for element in elements {
                sink.start_node(SyntaxKind::TuplePatternElement);
                emit_pattern_variant_inner(sink, element);
                sink.finish_node();
            }
            sink.add_token(SyntaxKind::RParen, rparen.clone());
            sink.finish_node();
        }
        PatternVariant::Literal(kind) => {
            sink.start_node(SyntaxKind::LiteralPattern);
            match kind {
                LiteralPatternKind::Integer(span) => {
                    sink.add_token(SyntaxKind::Integer, span.clone());
                }
                LiteralPatternKind::Float(span) => {
                    sink.add_token(SyntaxKind::Float, span.clone());
                }
                LiteralPatternKind::String(span) => {
                    sink.add_token(SyntaxKind::String, span.clone());
                }
                LiteralPatternKind::Bool(span) => {
                    sink.add_token(SyntaxKind::Boolean, span.clone());
                }
            }
            sink.finish_node();
        }
        PatternVariant::Enum {
            dot,
            case_name,
            arguments,
        } => {
            sink.start_node(SyntaxKind::EnumPattern);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::Identifier, case_name.clone());
            if let Some((lparen, args, rparen)) = arguments {
                sink.add_token(SyntaxKind::LParen, lparen.clone());
                for arg in args {
                    sink.start_node(SyntaxKind::EnumPatternArg);
                    sink.add_token(SyntaxKind::Identifier, arg.label.clone());
                    if let Some((colon, pattern)) = &arg.binding {
                        sink.add_token(SyntaxKind::Colon, colon.clone());
                        emit_pattern_variant_inner(sink, pattern);
                    }
                    sink.finish_node();
                }
                sink.add_token(SyntaxKind::RParen, rparen.clone());
            }
            sink.finish_node();
        }
        PatternVariant::Error(span) => {
            sink.start_node(SyntaxKind::ErrorPattern);
            sink.error_at("Invalid pattern".to_string(), span.clone());
            sink.finish_node();
        }
    }
}

/// Parse a pattern and emit events
pub fn parse_pattern<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match pattern_parser().parse(input).into_result() {
        Ok(variant) => {
            emit_pattern_variant(sink, &variant);
        }
        Err(errors) => {
            // Even on error, we need to emit a valid tree structure
            sink.start_node(SyntaxKind::Pattern);
            sink.start_node(SyntaxKind::ErrorPattern);
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), to_kestrel_span(*span));
            }
            sink.finish_node(); // ErrorPattern
            sink.finish_node(); // Pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    fn parse_pattern_from_source(source: &str) -> Pattern {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let mut sink = EventSink::new();
        parse_pattern(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        Pattern {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    #[test]
    fn test_wildcard_pattern() {
        let source = "_";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_wildcard());
    }

    #[test]
    fn test_binding_pattern() {
        let source = "x";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_binding());
    }

    #[test]
    fn test_mutable_binding_pattern() {
        let source = "var x";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_binding());
    }

    #[test]
    fn test_tuple_pattern() {
        let source = "(a, b)";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_tuple());
    }

    #[test]
    fn test_integer_literal_pattern() {
        let source = "42";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_literal());
    }

    #[test]
    fn test_string_literal_pattern() {
        let source = "\"hello\"";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_literal());
    }

    #[test]
    fn test_bool_literal_pattern() {
        let source = "true";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_literal());
    }

    #[test]
    fn test_enum_pattern_simple() {
        let source = ".None";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_enum());
    }

    #[test]
    fn test_enum_pattern_with_args() {
        let source = ".Some(value)";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_enum());
    }

    #[test]
    fn test_enum_pattern_with_labeled_args() {
        let source = ".Point(x: a, y: b)";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_enum());
    }

    #[test]
    fn test_nested_tuple_pattern() {
        let source = "((a, b), c)";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_tuple());
    }

    #[test]
    fn test_tuple_with_wildcard() {
        let source = "(_, x)";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_tuple());
    }
}
