use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

/// Represents a type expression
///
/// The type is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyExpression {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl TyExpression {
    /// Create a new TyExpression from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the kind of this type expression
    pub fn kind(&self) -> SyntaxKind {
        // Find the first child node which represents the actual type variant
        self.syntax
            .children()
            .next()
            .map(|child| child.kind())
            .unwrap_or(SyntaxKind::Error)
    }

    /// Check if this is a unit type
    pub fn is_unit(&self) -> bool {
        self.kind() == SyntaxKind::TyUnit
    }

    /// Check if this is a never type
    pub fn is_never(&self) -> bool {
        self.kind() == SyntaxKind::TyNever
    }

    /// Check if this is a tuple type
    pub fn is_tuple(&self) -> bool {
        self.kind() == SyntaxKind::TyTuple
    }

    /// Check if this is a function type
    pub fn is_function(&self) -> bool {
        self.kind() == SyntaxKind::TyFunction
    }

    /// Check if this is a path type
    pub fn is_path(&self) -> bool {
        self.kind() == SyntaxKind::TyPath
    }

    /// Check if this is an array type
    pub fn is_array(&self) -> bool {
        self.kind() == SyntaxKind::TyArray
    }

    /// Check if this is an inferred type (_)
    pub fn is_inferred(&self) -> bool {
        self.kind() == SyntaxKind::TyInferred
    }

    /// Get the path segments if this is a path type
    /// Structure: Ty -> TyPath -> Path -> PathElement -> Identifier
    pub fn path_segments(&self) -> Option<Vec<String>> {
        if !self.is_path() {
            return None;
        }

        // Navigate: Ty -> TyPath -> Path
        let ty_path_node = self.syntax.children().next()?;
        let path_node = ty_path_node
            .children()
            .find(|child| child.kind() == SyntaxKind::Path)?;

        // Collect identifiers from PathElement nodes
        Some(
            path_node
                .children()
                .filter(|child| child.kind() == SyntaxKind::PathElement)
                .filter_map(|path_elem| {
                    path_elem
                        .children_with_tokens()
                        .filter_map(|elem| elem.into_token())
                        .find(|tok| tok.kind() == SyntaxKind::Identifier)
                        .map(|tok| tok.text().to_string())
                })
                .collect(),
        )
    }

    /// Get the tuple element types if this is a tuple type
    /// Returns the number of elements (we don't recursively parse nested types yet)
    pub fn tuple_element_count(&self) -> Option<usize> {
        if !self.is_tuple() {
            return None;
        }

        let tuple_node = self.syntax.children().next()?;

        // Count the number of Ty child nodes
        Some(
            tuple_node
                .children()
                .filter(|child| child.kind() == SyntaxKind::Ty)
                .count(),
        )
    }
}

/// Check if a token is trivia (whitespace or comment)
fn is_trivia(token: &Token) -> bool {
    matches!(
        token,
        Token::Whitespace | Token::Newline | Token::LineComment | Token::BlockComment
    )
}

/// Parser that skips trivia tokens
fn skip_trivia<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| is_trivia(token))
        .repeated()
        .ignored()
}

/// Internal parser for never type: !
/// Skips leading whitespace
fn never_type_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(just(Token::Bang).map_with(|_, e| to_kestrel_span(e.span())))
}

/// Internal parser for path segments: Ident or Ident.Ident.Ident
/// Skips leading whitespace before the first identifier
fn path_segments_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        select! {
            Token::Identifier = e => to_kestrel_span(e.span()),
        }
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect(),
    )
}

/// Combined type parser that returns a variant
/// Supports: !, (), (T1, T2), (T1) -> T2, Path, Path[Args]
pub(crate) fn ty_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, TyVariant, ParserExtra<'tokens>> + Clone {
    recursive(|ty| {
        // Never type: !
        let never = never_type_parser().map(TyVariant::Never);

        // Inferred type: _
        let inferred = skip_trivia()
            .ignore_then(just(Token::Underscore).map_with(|_, e| to_kestrel_span(e.span())))
            .map(TyVariant::Inferred);

        // Unit type, grouping (T), tuple (T, U) or (T,), or function type
        // We need to distinguish:
        // - () -> Unit
        // - (T) -> Grouping (just returns T, for precedence)
        // - (T,) -> Single-element Tuple
        // - (T, U, ...) -> Tuple
        // - (...) -> T -> Function
        let paren_types = {
            skip_trivia()
                .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
                .then(
                    // Empty parens case
                    skip_trivia()
                        .ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())))
                        .map(|rparen| (Vec::new(), false, rparen))
                        .or(
                            // At least one type
                            ty.clone()
                                .then(
                                    // Check for comma after first element
                                    skip_trivia()
                                        .ignore_then(
                                            just(Token::Comma)
                                                .map_with(|_, e| to_kestrel_span(e.span())),
                                        )
                                        .then(
                                            // After comma: more types separated by comma
                                            ty.clone()
                                                .separated_by(
                                                    skip_trivia().ignore_then(just(Token::Comma)),
                                                )
                                                .allow_trailing()
                                                .collect::<Vec<_>>(),
                                        )
                                        .map(|(_comma, more)| (true, more))
                                        .or(empty().to((false, Vec::new()))),
                                )
                                .then(skip_trivia().ignore_then(
                                    just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())),
                                ))
                                .map(|((first, (has_comma, more)), rparen)| {
                                    let mut types = vec![first];
                                    types.extend(more);
                                    (types, has_comma, rparen)
                                }),
                        ),
                )
                .then(
                    // Optional arrow and return type for function types
                    skip_trivia()
                        .ignore_then(just(Token::Arrow))
                        .map_with(|_, e| to_kestrel_span(e.span()))
                        .then(ty.clone())
                        .or_not(),
                )
                .map(|((lparen, (types, has_comma, rparen)), arrow_and_return)| {
                    if let Some((arrow_span, return_ty)) = arrow_and_return {
                        TyVariant::Function(lparen, types, rparen, arrow_span, Box::new(return_ty))
                    } else if types.is_empty() {
                        TyVariant::Unit(lparen, rparen)
                    } else if types.len() == 1 && !has_comma {
                        // (T) - grouping, just return the inner type
                        types.into_iter().next().unwrap()
                    } else {
                        // (T,) or (T, U, ...) - tuple
                        TyVariant::Tuple(lparen, types, rparen)
                    }
                })
                .boxed()
        };

        // Path type with optional type arguments: Foo or Foo[Int, String]
        let path = path_segments_parser()
            .then(
                // Optional type arguments: [T1, T2]
                skip_trivia()
                    .ignore_then(just(Token::LBracket))
                    .ignore_then(
                        ty.clone()
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>(),
                    )
                    .then_ignore(skip_trivia())
                    .then_ignore(just(Token::RBracket))
                    .or_not(),
            )
            .map(|(segments, args)| TyVariant::Path { segments, args })
            .boxed();

        // Array type [T] or Dictionary type [K: V]
        let array_or_dict = skip_trivia()
            .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
            .then(ty.clone())
            .then(
                // Check for colon - if present, this is a dictionary [K: V]
                skip_trivia()
                    .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(ty.clone())
                    .or_not(),
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .map(
                |(((lbracket, first_ty), maybe_colon_and_value), rbracket)| {
                    if let Some((colon_span, value_ty)) = maybe_colon_and_value {
                        // Dictionary: [K: V]
                        TyVariant::Dictionary(
                            lbracket,
                            Box::new(first_ty),
                            colon_span,
                            Box::new(value_ty),
                            rbracket,
                        )
                    } else {
                        // Array: [T]
                        TyVariant::Array(lbracket, Box::new(first_ty), rbracket)
                    }
                },
            )
            .boxed();

        // Try never first, then inferred, then paren types, then array/dict, then path
        let base_ty = never
            .or(inferred)
            .or(paren_types)
            .or(array_or_dict)
            .or(path)
            .boxed();

        // Type operators: ? (Optional) and throws E (Result)
        // Both operators can appear in any order and can be chained:
        // - T? -> Optional[T]
        // - T throws E -> Result[T, E]
        // - T? throws E -> Result[Optional[T], E]
        // - T throws E? -> Optional[Result[T, E]]
        // - T throws E1 throws E2 -> Result[Result[T, E1], E2]
        //
        // We use a helper enum to track which operator we found
        #[derive(Clone)]
        enum TypeOperator {
            Optional(Span),
            Throws(Span, TyVariant),
        }

        let type_operator = skip_trivia()
            .ignore_then(
                just(Token::Question)
                    .map_with(|_, e| TypeOperator::Optional(to_kestrel_span(e.span())))
                    .or(just(Token::Throws)
                        .map_with(|_, e| to_kestrel_span(e.span()))
                        .then(ty.clone())
                        .map(|(throws_span, error_ty)| {
                            TypeOperator::Throws(throws_span, error_ty)
                        })),
            )
            .boxed();

        // Parse base type, then zero or more type operators
        base_ty
            .then(type_operator.repeated().collect::<Vec<_>>())
            .map(|(base, operators)| {
                // Apply operators left-to-right
                let mut result = base;
                for op in operators {
                    match op {
                        TypeOperator::Optional(question_span) => {
                            result = TyVariant::Optional(Box::new(result), question_span);
                        },
                        TypeOperator::Throws(throws_span, error_ty) => {
                            result = TyVariant::Result(
                                Box::new(result),
                                throws_span,
                                Box::new(error_ty),
                            );
                        },
                    }
                }
                result
            })
            .boxed()
    })
}

/// Parse a type expression and emit events
/// This is the primary event-driven parser function
pub fn parse_ty<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    use crate::input::{create_input, prepare_tokens};

    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match ty_parser().parse(input).into_result() {
        Ok(variant) => {
            emit_ty_variant(sink, &variant);
        },
        Err(errors) => {
            for error in errors {
                sink.error_from_rich(&error);
            }
        },
    }
}

/// Emit events for any type variant
pub(crate) fn emit_ty_variant(sink: &mut EventSink, variant: &TyVariant) {
    match variant {
        TyVariant::Unit(lparen_span, rparen_span) => {
            emit_unit_type(sink, lparen_span.clone(), rparen_span.clone());
        },
        TyVariant::Never(bang_span) => {
            emit_never_type(sink, bang_span.clone());
        },
        TyVariant::Inferred(underscore_span) => {
            emit_inferred_type(sink, underscore_span.clone());
        },
        TyVariant::Tuple(lparen, types, rparen) => {
            emit_tuple_type(sink, lparen.clone(), types, rparen.clone());
        },
        TyVariant::Function(lparen, params, rparen, arrow, return_ty) => {
            emit_function_type(
                sink,
                lparen.clone(),
                params,
                rparen.clone(),
                arrow.clone(),
                return_ty,
            );
        },
        TyVariant::Path { segments, args } => {
            emit_path_type(sink, segments, args.as_ref());
        },
        TyVariant::Array(lbracket, element_ty, rbracket) => {
            emit_array_type(sink, lbracket.clone(), element_ty, rbracket.clone());
        },
        TyVariant::Dictionary(lbracket, key_ty, colon, value_ty, rbracket) => {
            emit_dictionary_type(
                sink,
                lbracket.clone(),
                key_ty,
                colon.clone(),
                value_ty,
                rbracket.clone(),
            );
        },
        TyVariant::Optional(base_ty, question_span) => {
            emit_optional_type(sink, base_ty, question_span.clone());
        },
        TyVariant::Result(success_ty, throws_span, error_ty) => {
            emit_result_type(sink, success_ty, throws_span.clone(), error_ty);
        },
    }
}

/// Internal enum to distinguish between type variants during parsing
#[derive(Debug, Clone)]
pub enum TyVariant {
    Unit(Span, Span),
    Never(Span),
    Inferred(Span), // _ type
    Tuple(Span, Vec<TyVariant>, Span),
    Function(Span, Vec<TyVariant>, Span, Span, Box<TyVariant>),
    /// Path with optional type arguments: Foo or Foo[Int, String]
    Path {
        segments: Vec<Span>,
        args: Option<Vec<TyVariant>>,
    },
    /// Array type: [T]
    Array(Span, Box<TyVariant>, Span), // (lbracket, element_type, rbracket)
    /// Dictionary type: [K: V]
    Dictionary(Span, Box<TyVariant>, Span, Box<TyVariant>, Span), // (lbracket, key_type, colon, value_type, rbracket)
    /// Optional type: T?
    Optional(Box<TyVariant>, Span), // (base_type, question_span)
    /// Result type: T throws E
    Result(Box<TyVariant>, Span, Box<TyVariant>), // (success_type, throws_span, error_type)
}

/// Emit events for an inferred type: _
pub(crate) fn emit_inferred_type(sink: &mut EventSink, underscore_span: Span) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyInferred);
    sink.add_token(SyntaxKind::Underscore, underscore_span);
    sink.finish_node(); // Finish TyInferred
    sink.finish_node(); // Finish Ty
}

/// Emit events for a unit type
pub(crate) fn emit_unit_type(sink: &mut EventSink, lparen_span: Span, rparen_span: Span) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyUnit);
    sink.add_token(SyntaxKind::LParen, lparen_span);
    sink.add_token(SyntaxKind::RParen, rparen_span);
    sink.finish_node(); // Finish TyUnit
    sink.finish_node(); // Finish Ty
}

/// Emit events for a never type
pub(crate) fn emit_never_type(sink: &mut EventSink, bang_span: Span) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyNever);
    sink.add_token(SyntaxKind::Bang, bang_span);
    sink.finish_node(); // Finish TyNever
    sink.finish_node(); // Finish Ty
}

/// Helper function to emit a path structure: Path -> PathElement -> Identifier
/// This is used within TyPath nodes
fn emit_path(sink: &mut EventSink, segments: &[Span]) {
    sink.start_node(SyntaxKind::Path);

    for (i, span) in segments.iter().enumerate() {
        if i > 0 {
            // Add the dot separator (between path elements)
            let dot_start = span.start.saturating_sub(1);
            sink.add_token(
                SyntaxKind::Dot,
                Span::new(span.file_id, dot_start..span.start),
            );
        }

        // Wrap each identifier in a PathElement node
        sink.start_node(SyntaxKind::PathElement);
        sink.add_token(SyntaxKind::Identifier, span.clone());
        sink.finish_node(); // Finish PathElement
    }

    sink.finish_node(); // Finish Path
}

/// Emit events for a path type with optional type arguments
/// Structure: Ty -> TyPath -> Path -> PathElement -> Identifier
///            (optional) TypeArgumentList -> Ty...
pub(crate) fn emit_path_type(
    sink: &mut EventSink,
    segments: &[Span],
    args: Option<&Vec<TyVariant>>,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyPath);
    emit_path(sink, segments);

    // Emit type arguments if present: [Int, String]
    if let Some(type_args) = args {
        sink.start_node(SyntaxKind::TypeArgumentList);
        for arg in type_args {
            emit_ty_variant(sink, arg);
        }
        sink.finish_node(); // Finish TypeArgumentList
    }

    sink.finish_node(); // Finish TyPath
    sink.finish_node(); // Finish Ty
}

/// Emit events for a tuple type
pub(crate) fn emit_tuple_type(
    sink: &mut EventSink,
    lparen: Span,
    types: &[TyVariant],
    rparen: Span,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyTuple);

    sink.add_token(SyntaxKind::LParen, lparen);

    // Emit each type in the tuple
    for ty in types {
        emit_ty_variant(sink, ty);
    }

    sink.add_token(SyntaxKind::RParen, rparen);

    sink.finish_node(); // Finish TyTuple
    sink.finish_node(); // Finish Ty
}

/// Emit events for a function type
pub(crate) fn emit_function_type(
    sink: &mut EventSink,
    lparen: Span,
    params: &[TyVariant],
    rparen: Span,
    arrow: Span,
    return_ty: &TyVariant,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyFunction);

    // Parameter list
    sink.start_node(SyntaxKind::TyList);
    sink.add_token(SyntaxKind::LParen, lparen);

    for param in params {
        emit_ty_variant(sink, param);
    }

    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node(); // Finish TyList

    // Arrow
    sink.add_token(SyntaxKind::Arrow, arrow);

    // Return type
    emit_ty_variant(sink, return_ty);

    sink.finish_node(); // Finish TyFunction
    sink.finish_node(); // Finish Ty
}

/// Emit events for an array type
pub(crate) fn emit_array_type(
    sink: &mut EventSink,
    lbracket: Span,
    element_ty: &TyVariant,
    rbracket: Span,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyArray);

    sink.add_token(SyntaxKind::LBracket, lbracket);
    emit_ty_variant(sink, element_ty);
    sink.add_token(SyntaxKind::RBracket, rbracket);

    sink.finish_node(); // Finish TyArray
    sink.finish_node(); // Finish Ty
}

/// Emit events for an optional type: T?
pub(crate) fn emit_optional_type(sink: &mut EventSink, base_ty: &TyVariant, question_span: Span) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyOptional);

    emit_ty_variant(sink, base_ty);
    sink.add_token(SyntaxKind::Question, question_span);

    sink.finish_node(); // Finish TyOptional
    sink.finish_node(); // Finish Ty
}

/// Emit events for a dictionary type: [K: V]
pub(crate) fn emit_dictionary_type(
    sink: &mut EventSink,
    lbracket: Span,
    key_ty: &TyVariant,
    colon: Span,
    value_ty: &TyVariant,
    rbracket: Span,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyDictionary);

    sink.add_token(SyntaxKind::LBracket, lbracket);
    emit_ty_variant(sink, key_ty);
    sink.add_token(SyntaxKind::Colon, colon);
    emit_ty_variant(sink, value_ty);
    sink.add_token(SyntaxKind::RBracket, rbracket);

    sink.finish_node(); // Finish TyDictionary
    sink.finish_node(); // Finish Ty
}

/// Emit events for a result type: T throws E
pub(crate) fn emit_result_type(
    sink: &mut EventSink,
    success_ty: &TyVariant,
    throws_span: Span,
    error_ty: &TyVariant,
) {
    sink.start_node(SyntaxKind::Ty);
    sink.start_node(SyntaxKind::TyResult);

    emit_ty_variant(sink, success_ty);
    sink.add_token(SyntaxKind::Throws, throws_span);
    emit_ty_variant(sink, error_ty);

    sink.finish_node(); // Finish TyResult
    sink.finish_node(); // Finish Ty
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    fn parse_ty_from_source(source: &str) -> TyExpression {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let mut sink = EventSink::new(0);
        parse_ty(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        TyExpression {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        }
    }

    #[test]
    fn test_unit_type() {
        let source = "()";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_unit());
        assert!(!ty.is_never());
        assert!(!ty.is_tuple());
        assert!(!ty.is_function());
    }

    #[test]
    fn test_never_type() {
        let source = "!";
        let ty = parse_ty_from_source(source);

        assert!(!ty.is_unit());
        assert!(ty.is_never());
        assert!(!ty.is_tuple());
        assert!(!ty.is_function());
    }

    #[test]
    fn test_inferred_type() {
        let source = "_";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_inferred());
        assert!(!ty.is_unit());
        assert!(!ty.is_never());
        assert!(!ty.is_path());
    }

    #[test]
    fn test_path_type_simple() {
        let source = "Int";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_path());
        assert_eq!(ty.path_segments(), Some(vec!["Int".to_string()]));
    }

    #[test]
    fn test_path_type_qualified() {
        let source = "A.B.C";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_path());
        assert_eq!(
            ty.path_segments(),
            Some(vec!["A".to_string(), "B".to_string(), "C".to_string()])
        );
    }

    #[test]
    fn test_tuple_type_simple() {
        let source = "(Int, String)";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_tuple());
        assert_eq!(ty.tuple_element_count(), Some(2));
    }

    #[test]
    fn test_tuple_type_single() {
        let source = "(Int,)";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_tuple());
        assert_eq!(ty.tuple_element_count(), Some(1));
    }

    #[test]
    fn test_function_type_simple() {
        let source = "() -> Int";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_function());
    }

    #[test]
    fn test_function_type_with_params() {
        let source = "(Int, String) -> Bool";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_function());
    }

    #[test]
    fn test_generic_type_simple() {
        let source = "List[Int]";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_path());
        // Check that it parsed the base type
        assert_eq!(ty.path_segments(), Some(vec!["List".to_string()]));
        // The type arguments are part of the TyPath node
    }

    #[test]
    fn test_generic_type_multiple_args() {
        let source = "Map[String, Int]";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_path());
        assert_eq!(ty.path_segments(), Some(vec!["Map".to_string()]));
    }

    #[test]
    fn test_generic_type_nested() {
        let source = "List[Option[Int]]";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_path());
        assert_eq!(ty.path_segments(), Some(vec!["List".to_string()]));
    }

    #[test]
    fn test_function_type_qualified() {
        let source = "(A.B, C.D) -> E.F";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_function());
    }

    #[test]
    fn test_array_type_simple() {
        let source = "[Int]";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_array());
    }

    #[test]
    fn test_array_type_nested() {
        let source = "[[Int]]";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_array());
    }

    #[test]
    fn test_array_type_of_tuple() {
        let source = "[(Int, String)]";
        let ty = parse_ty_from_source(source);

        assert!(ty.is_array());
    }
}
