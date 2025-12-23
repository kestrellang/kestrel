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

    /// Check if this is a struct pattern
    pub fn is_struct(&self) -> bool {
        self.kind() == SyntaxKind::StructPattern
    }
}

/// Data for a single enum pattern argument
#[derive(Debug, Clone)]
pub enum EnumPatternArgData {
    /// Labeled argument: `label` or `label: pattern`
    Labeled {
        /// Label name (identifier)
        label: Span,
        /// Optional colon followed by pattern
        binding: Option<(Span, PatternVariant)>,
    },
    /// Unlabeled pattern argument: `_`, `(a, b)`, `.Nested(x)`, etc.
    Unlabeled(PatternVariant),
}

/// Data for a single struct pattern field
#[derive(Debug, Clone)]
pub struct StructPatternFieldData {
    /// Field name (identifier)
    pub field_name: Span,
    /// Optional colon followed by pattern (None = shorthand binding)
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
    /// Range pattern: `0..=9` or `0..<10`
    Range {
        start: LiteralPatternKind,
        operator: Span,
        inclusive: bool,
        end: LiteralPatternKind,
    },
    /// Enum pattern: `.Case` or `.Case(args)`
    Enum {
        dot: Span,
        case_name: Span,
        /// Optional argument list
        arguments: Option<(Span, Vec<EnumPatternArgData>, Span)>, // (lparen, args, rparen)
    },
    /// Array pattern: `[a, b, ..rest]`
    Array {
        lbracket: Span,
        /// Elements before the rest pattern
        prefix: Vec<PatternVariant>,
        /// Rest pattern: None = no rest, Some((dotdot_span, None)) = `..`, Some((dotdot_span, Some(name_span))) = `..name`
        rest: Option<(Span, Option<Span>)>,
        /// Elements after the rest pattern
        suffix: Vec<PatternVariant>,
        rbracket: Span,
    },
    /// Struct pattern: `Point { x, y }` or `Point { x: a, y: b }`
    Struct {
        struct_name: Span,
        lbrace: Span,
        fields: Vec<StructPatternFieldData>,
        /// Rest pattern: Some(span) if `..` is present
        rest: Option<Span>,
        rbrace: Span,
    },
    /// Or-pattern: `p1 or p2 or p3`
    Or {
        /// The alternative patterns (at least 2)
        alternatives: Vec<PatternVariant>,
        /// The `or` keyword spans
        or_spans: Vec<Span>,
    },
    /// @-pattern: `name @ subpattern` or `var name @ subpattern`
    At {
        /// Optional `var` keyword span (if mutable)
        var_span: Option<Span>,
        /// The binding name span
        name_span: Span,
        /// The `@` token span
        at_span: Span,
        /// The subpattern
        subpattern: Box<PatternVariant>,
    },
    /// Rest pattern: `..` (used in tuples)
    Rest(Span),
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

        // Range pattern: `0..=9` or `0..<10` or `'a'..='z'`
        // Must parse before standalone literals
        let range_start = skip_trivia()
            .ignore_then(select! { Token::Integer = e => LiteralPatternKind::Integer(to_kestrel_span(e.span())) })
            .or(skip_trivia().ignore_then(select! { Token::String = e => {
                // Check if it's a char literal (single char in quotes)
                let span = to_kestrel_span(e.span());
                LiteralPatternKind::String(span)
            }}));

        let range_end = skip_trivia()
            .ignore_then(select! { Token::Integer = e => LiteralPatternKind::Integer(to_kestrel_span(e.span())) })
            .or(skip_trivia().ignore_then(select! { Token::String = e => {
                let span = to_kestrel_span(e.span());
                LiteralPatternKind::String(span)
            }}));

        let range_pattern = range_start.clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::DotDotEquals).map_with(|_, e| (to_kestrel_span(e.span()), true)))
                    .or(skip_trivia().ignore_then(just(Token::DotDotLess).map_with(|_, e| (to_kestrel_span(e.span()), false))))
            )
            .then(range_end)
            .map(|((start, (operator, inclusive)), end)| PatternVariant::Range {
                start,
                operator,
                inclusive,
                end,
            });

        let literal = float_literal
            .or(integer_literal)
            .or(string_literal)
            .or(bool_literal);

        // Rest pattern: `..` (used in tuples)
        let rest_pattern = skip_trivia()
            .ignore_then(just(Token::DotDot).map_with(|_, e| to_kestrel_span(e.span())))
            .map(PatternVariant::Rest);

        // Binding pattern: `var name` (mutable) or `name` (immutable)
        // Can optionally be followed by `@ subpattern` to make it an @-pattern
        // Need to be careful to distinguish from wildcards and literals
        let mutable_binding = skip_trivia()
            .ignore_then(just(Token::Var).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                skip_trivia()
                    .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) }),
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::At).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(pattern.clone())
                    .or_not()
            )
            .map(|((var_span, name_span), at_opt)| {
                match at_opt {
                    Some((at_span, subpattern)) => PatternVariant::At {
                        var_span: Some(var_span),
                        name_span,
                        at_span,
                        subpattern: Box::new(subpattern),
                    },
                    None => PatternVariant::Binding {
                        var_span: Some(var_span),
                        name_span,
                    },
                }
            });

        let immutable_binding = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(
                skip_trivia()
                    .ignore_then(just(Token::At).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(pattern.clone())
                    .or_not()
            )
            .map(|(name_span, at_opt)| {
                match at_opt {
                    Some((at_span, subpattern)) => PatternVariant::At {
                        var_span: None,
                        name_span,
                        at_span,
                        subpattern: Box::new(subpattern),
                    },
                    None => PatternVariant::Binding {
                        var_span: None,
                        name_span,
                    },
                }
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

        // Enum pattern argument: `label` or `label: pattern` or just `pattern`
        // Labeled form: identifier optionally followed by `: pattern`
        let labeled_arg = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(pattern.clone())
                    .or_not(),
            )
            .map(|(label, binding)| EnumPatternArgData::Labeled { label, binding });

        // Unlabeled form: any pattern that isn't a plain identifier
        // (plain identifiers are handled by labeled_arg as shorthand labels)
        let unlabeled_arg = pattern.clone().try_map(|p, span| {
            match &p {
                // Plain immutable binding looks like a label, so reject it here
                // to let labeled_arg handle it
                PatternVariant::Binding { var_span: None, .. } => {
                    Err(chumsky::error::Rich::custom(span, "expected labeled argument"))
                }
                // Everything else is an unlabeled pattern
                _ => Ok(EnumPatternArgData::Unlabeled(p)),
            }
        });

        // Try labeled first (handles `x` and `x: pattern`), then unlabeled (handles `_`, `(a,b)`, etc.)
        let enum_arg = labeled_arg.or(unlabeled_arg);

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

        // Struct pattern field: `x` or `x: pattern` or `..`
        let struct_field = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(pattern.clone())
                    .or_not(),
            )
            .map(|(field_name, binding)| StructPatternFieldData { field_name, binding });

        // Rest pattern in struct: `..`
        let struct_rest = skip_trivia()
            .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
            .then(skip_trivia().ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span()))))
            .map(|(dot1, dot2)| Span::new(dot1.file_id, dot1.start..dot2.end));

        // Either a field or rest pattern
        let struct_field_or_rest = struct_rest.clone().map(|span| (None, Some(span)))
            .or(struct_field.map(|f| (Some(f), None)));

        // Struct pattern: `TypeName { field1, field2: binding, .. }`
        let struct_pattern = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(
                skip_trivia()
                    .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(
                struct_field_or_rest
                    .separated_by(
                        skip_trivia().ignore_then(
                            just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())),
                        ),
                    )
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .map(|(((struct_name, lbrace), field_or_rests), rbrace)| {
                let mut fields = Vec::new();
                let mut rest = None;
                for (field_opt, rest_opt) in field_or_rests {
                    if let Some(field) = field_opt {
                        fields.push(field);
                    }
                    if let Some(rest_span) = rest_opt {
                        rest = Some(rest_span);
                    }
                }
                PatternVariant::Struct {
                    struct_name,
                    lbrace,
                    fields,
                    rest,
                    rbrace,
                }
            })
            .boxed();

        // Array pattern element: either a rest pattern (..) or (..name) or a regular pattern
        let array_rest = skip_trivia()
            .ignore_then(just(Token::DotDot).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                skip_trivia()
                    .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
                    .or_not()
            )
            .map(|(dotdot_span, name_span)| (dotdot_span, name_span));

        // Array pattern: [p1, p2, ..rest, p3]
        let array_pattern = skip_trivia()
            .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                // Parse elements: either rest patterns or regular patterns
                array_rest.clone().map(|(dotdot, name)| (None, Some((dotdot, name))))
                    .or(pattern.clone().map(|p| (Some(p), None)))
                    .separated_by(
                        skip_trivia().ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())))
                    )
                    .allow_trailing()
                    .collect::<Vec<_>>()
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span())))
            )
            .map(|((lbracket, elements), rbracket)| {
                // Split elements into prefix, rest, suffix
                let mut prefix = Vec::new();
                let mut rest: Option<(Span, Option<Span>)> = None;
                let mut suffix = Vec::new();

                for (pattern_opt, rest_opt) in elements {
                    if let Some((dotdot_span, name_span)) = rest_opt {
                        // This is a rest pattern
                        rest = Some((dotdot_span, name_span));
                    } else if let Some(p) = pattern_opt {
                        // This is a regular pattern
                        if rest.is_some() {
                            suffix.push(p);
                        } else {
                            prefix.push(p);
                        }
                    }
                }

                PatternVariant::Array {
                    lbracket,
                    prefix,
                    rest,
                    suffix,
                    rbracket,
                }
            })
            .boxed();

        // Order matters: try more specific patterns first
        // - wildcard (single underscore)
        // - range pattern (literal followed by ..= or ..<)
        // - rest pattern (.. - for tuples)
        // - literal (numbers, strings, bools)
        // - enum pattern (starts with dot)
        // - struct pattern (identifier followed by braces)
        // - array pattern (starts with lbracket)
        // - mutable binding (starts with var)
        // - tuple pattern (starts with lparen)
        // - immutable binding (identifier - least specific)
        let base_pattern = wildcard
            .or(rest_pattern)
            .or(range_pattern)
            .or(literal)
            .or(enum_pattern)
            .or(struct_pattern)
            .or(array_pattern)
            .or(mutable_binding)
            .or(tuple_pattern)
            .or(immutable_binding);

        // Or-pattern: base_pattern (or base_pattern)*
        // The `or` keyword has lowest precedence in pattern context
        let or_continuation = skip_trivia()
            .ignore_then(just(Token::Or).map_with(|_, e| to_kestrel_span(e.span())))
            .then(base_pattern.clone())
            .map(|(or_span, pat)| (or_span, pat));

        base_pattern
            .then(or_continuation.repeated().collect::<Vec<_>>())
            .map(|(first, rest)| {
                if rest.is_empty() {
                    first
                } else {
                    let mut alternatives = vec![first];
                    let mut or_spans = Vec::new();
                    for (or_span, pat) in rest {
                        or_spans.push(or_span);
                        alternatives.push(pat);
                    }
                    PatternVariant::Or {
                        alternatives,
                        or_spans,
                    }
                }
            })
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
        PatternVariant::Range {
            start,
            operator,
            inclusive,
            end,
        } => {
            sink.start_node(SyntaxKind::RangePattern);
            // Emit start literal
            match start {
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
            // Emit range operator
            if *inclusive {
                sink.add_token(SyntaxKind::DotDotEquals, operator.clone());
            } else {
                sink.add_token(SyntaxKind::DotDotLess, operator.clone());
            }
            // Emit end literal
            match end {
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
                    match arg {
                        EnumPatternArgData::Labeled { label, binding } => {
                            sink.add_token(SyntaxKind::Identifier, label.clone());
                            if let Some((colon, pattern)) = binding {
                                sink.add_token(SyntaxKind::Colon, colon.clone());
                                emit_pattern_variant_inner(sink, pattern);
                            }
                        }
                        EnumPatternArgData::Unlabeled(pattern) => {
                            emit_pattern_variant_inner(sink, pattern);
                        }
                    }
                    sink.finish_node();
                }
                sink.add_token(SyntaxKind::RParen, rparen.clone());
            }
            sink.finish_node();
        }
        PatternVariant::Or {
            alternatives,
            or_spans,
        } => {
            sink.start_node(SyntaxKind::OrPattern);
            for (i, alt) in alternatives.iter().enumerate() {
                emit_pattern_variant_inner(sink, alt);
                if i < or_spans.len() {
                    sink.add_token(SyntaxKind::Or, or_spans[i].clone());
                }
            }
            sink.finish_node();
        }
        PatternVariant::Struct {
            struct_name,
            lbrace,
            fields,
            rest,
            rbrace,
        } => {
            sink.start_node(SyntaxKind::StructPattern);
            sink.add_token(SyntaxKind::Identifier, struct_name.clone());
            sink.add_token(SyntaxKind::LBrace, lbrace.clone());
            for field in fields {
                sink.start_node(SyntaxKind::StructPatternField);
                sink.add_token(SyntaxKind::Identifier, field.field_name.clone());
                if let Some((colon, pattern)) = &field.binding {
                    sink.add_token(SyntaxKind::Colon, colon.clone());
                    emit_pattern_variant_inner(sink, pattern);
                }
                sink.finish_node();
            }
            if let Some(rest_span) = rest {
                sink.start_node(SyntaxKind::StructPatternRest);
                sink.add_token(SyntaxKind::Dot, Span::new(rest_span.file_id, rest_span.start..rest_span.start + 1));
                sink.add_token(SyntaxKind::Dot, Span::new(rest_span.file_id, rest_span.start + 1..rest_span.end));
                sink.finish_node();
            }
            sink.add_token(SyntaxKind::RBrace, rbrace.clone());
            sink.finish_node();
        }
        PatternVariant::Array {
            lbracket,
            prefix,
            rest,
            suffix,
            rbracket,
        } => {
            sink.start_node(SyntaxKind::ArrayPattern);
            sink.add_token(SyntaxKind::LBracket, lbracket.clone());
            // Emit prefix elements
            for elem in prefix {
                sink.start_node(SyntaxKind::ArrayPatternElement);
                emit_pattern_variant_inner(sink, elem);
                sink.finish_node();
            }
            // Emit rest pattern if present
            if let Some((dotdot_span, name_span)) = rest {
                sink.start_node(SyntaxKind::ArrayPatternRest);
                sink.add_token(SyntaxKind::DotDot, dotdot_span.clone());
                if let Some(name) = name_span {
                    sink.add_token(SyntaxKind::Identifier, name.clone());
                }
                sink.finish_node();
            }
            // Emit suffix elements
            for elem in suffix {
                sink.start_node(SyntaxKind::ArrayPatternElement);
                emit_pattern_variant_inner(sink, elem);
                sink.finish_node();
            }
            sink.add_token(SyntaxKind::RBracket, rbracket.clone());
            sink.finish_node();
        }
        PatternVariant::At {
            var_span,
            name_span,
            at_span,
            subpattern,
        } => {
            sink.start_node(SyntaxKind::AtPattern);
            if let Some(var) = var_span {
                sink.add_token(SyntaxKind::Var, var.clone());
            }
            sink.add_token(SyntaxKind::Identifier, name_span.clone());
            sink.add_token(SyntaxKind::At, at_span.clone());
            emit_pattern_variant_inner(sink, subpattern);
            sink.finish_node();
        }
        PatternVariant::Rest(span) => {
            sink.start_node(SyntaxKind::RestPattern);
            sink.add_token(SyntaxKind::DotDot, span.clone());
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
        PatternVariant::Range {
            start,
            operator,
            inclusive,
            end,
        } => {
            sink.start_node(SyntaxKind::RangePattern);
            // Emit start literal
            match start {
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
            // Emit range operator
            if *inclusive {
                sink.add_token(SyntaxKind::DotDotEquals, operator.clone());
            } else {
                sink.add_token(SyntaxKind::DotDotLess, operator.clone());
            }
            // Emit end literal
            match end {
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
                    match arg {
                        EnumPatternArgData::Labeled { label, binding } => {
                            sink.add_token(SyntaxKind::Identifier, label.clone());
                            if let Some((colon, pattern)) = binding {
                                sink.add_token(SyntaxKind::Colon, colon.clone());
                                emit_pattern_variant_inner(sink, pattern);
                            }
                        }
                        EnumPatternArgData::Unlabeled(pattern) => {
                            emit_pattern_variant_inner(sink, pattern);
                        }
                    }
                    sink.finish_node();
                }
                sink.add_token(SyntaxKind::RParen, rparen.clone());
            }
            sink.finish_node();
        }
        PatternVariant::Or {
            alternatives,
            or_spans,
        } => {
            sink.start_node(SyntaxKind::OrPattern);
            for (i, alt) in alternatives.iter().enumerate() {
                emit_pattern_variant_inner(sink, alt);
                if i < or_spans.len() {
                    sink.add_token(SyntaxKind::Or, or_spans[i].clone());
                }
            }
            sink.finish_node();
        }
        PatternVariant::Struct {
            struct_name,
            lbrace,
            fields,
            rest,
            rbrace,
        } => {
            sink.start_node(SyntaxKind::StructPattern);
            sink.add_token(SyntaxKind::Identifier, struct_name.clone());
            sink.add_token(SyntaxKind::LBrace, lbrace.clone());
            for field in fields {
                sink.start_node(SyntaxKind::StructPatternField);
                sink.add_token(SyntaxKind::Identifier, field.field_name.clone());
                if let Some((colon, pattern)) = &field.binding {
                    sink.add_token(SyntaxKind::Colon, colon.clone());
                    emit_pattern_variant_inner(sink, pattern);
                }
                sink.finish_node();
            }
            if let Some(rest_span) = rest {
                sink.start_node(SyntaxKind::StructPatternRest);
                sink.add_token(SyntaxKind::Dot, Span::new(rest_span.file_id, rest_span.start..rest_span.start + 1));
                sink.add_token(SyntaxKind::Dot, Span::new(rest_span.file_id, rest_span.start + 1..rest_span.end));
                sink.finish_node();
            }
            sink.add_token(SyntaxKind::RBrace, rbrace.clone());
            sink.finish_node();
        }
        PatternVariant::Array {
            lbracket,
            prefix,
            rest,
            suffix,
            rbracket,
        } => {
            sink.start_node(SyntaxKind::ArrayPattern);
            sink.add_token(SyntaxKind::LBracket, lbracket.clone());
            // Emit prefix elements
            for elem in prefix {
                sink.start_node(SyntaxKind::ArrayPatternElement);
                emit_pattern_variant_inner(sink, elem);
                sink.finish_node();
            }
            // Emit rest pattern if present
            if let Some((dotdot_span, name_span)) = rest {
                sink.start_node(SyntaxKind::ArrayPatternRest);
                sink.add_token(SyntaxKind::DotDot, dotdot_span.clone());
                if let Some(name) = name_span {
                    sink.add_token(SyntaxKind::Identifier, name.clone());
                }
                sink.finish_node();
            }
            // Emit suffix elements
            for elem in suffix {
                sink.start_node(SyntaxKind::ArrayPatternElement);
                emit_pattern_variant_inner(sink, elem);
                sink.finish_node();
            }
            sink.add_token(SyntaxKind::RBracket, rbracket.clone());
            sink.finish_node();
        }
        PatternVariant::At {
            var_span,
            name_span,
            at_span,
            subpattern,
        } => {
            sink.start_node(SyntaxKind::AtPattern);
            if let Some(var) = var_span {
                sink.add_token(SyntaxKind::Var, var.clone());
            }
            sink.add_token(SyntaxKind::Identifier, name_span.clone());
            sink.add_token(SyntaxKind::At, at_span.clone());
            emit_pattern_variant_inner(sink, subpattern);
            sink.finish_node();
        }
        PatternVariant::Rest(span) => {
            sink.start_node(SyntaxKind::RestPattern);
            sink.add_token(SyntaxKind::DotDot, span.clone());
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
    fn test_enum_pattern_with_wildcard() {
        let source = ".Some(_)";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_enum());
    }

    #[test]
    fn test_enum_pattern_with_tuple_arg() {
        let source = ".Pair((a, b))";
        let pattern = parse_pattern_from_source(source);
        assert!(pattern.is_enum());
    }

    #[test]
    fn test_enum_pattern_with_nested_enum() {
        let source = ".Outer(.Inner(x))";
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

    // Array pattern tests
    fn is_array_pattern(pattern: &Pattern) -> bool {
        pattern.kind() == SyntaxKind::ArrayPattern
    }

    #[test]
    fn test_empty_array_pattern() {
        let source = "[]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_single_element() {
        let source = "[x]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_multiple_elements() {
        let source = "[a, b, c]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_with_rest() {
        let source = "[first, ..]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_with_named_rest() {
        let source = "[first, ..rest]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_rest_at_beginning() {
        let source = "[.., last]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_rest_in_middle() {
        let source = "[first, .., last]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_with_wildcard() {
        let source = "[_, x]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }

    #[test]
    fn test_array_pattern_with_literals() {
        let source = "[1, 2, x]";
        let pattern = parse_pattern_from_source(source);
        assert!(is_array_pattern(&pattern), "Expected ArrayPattern, got {:?}", pattern.kind());
    }
}
