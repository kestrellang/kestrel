use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::event::{EventSink, TreeBuilder};

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

/// Internal parser for unit type: ()
/// Note: This is now handled by ty_parser directly, but kept for reference
#[allow(dead_code)]
fn unit_type_parser() -> impl Parser<Token, (Span, Span), Error = Simple<Token>> + Clone {
    just(Token::LParen)
        .map_with_span(|_, span| Span::from(span))
        .then(just(Token::RParen).map_with_span(|_, span| Span::from(span)))
}

/// Internal parser for never type: !
/// Skips leading whitespace
fn never_type_parser() -> impl Parser<Token, Span, Error = Simple<Token>> + Clone {
    use crate::common::skip_trivia;

    skip_trivia().ignore_then(just(Token::Bang).map_with_span(|_, span| Span::from(span)))
}

/// Internal parser for path segments: Ident or Ident.Ident.Ident
/// Skips leading whitespace before the first identifier
fn path_segments_parser() -> impl Parser<Token, Vec<Span>, Error = Simple<Token>> + Clone {
    use crate::common::skip_trivia;

    skip_trivia().ignore_then(
        filter_map(|span, token| match token {
            Token::Identifier => Ok(Span::from(span)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
        .separated_by(just(Token::Dot))
        .at_least(1),
    )
}

/// Combined type parser that returns a variant
/// Supports: !, (), (T1, T2), (T1) -> T2, Path, Path[Args]
pub(crate) fn ty_parser() -> impl Parser<Token, TyVariant, Error = Simple<Token>> + Clone {
    recursive(|ty| {
        use crate::common::skip_trivia;

        // Never type: !
        let never = never_type_parser().map(TyVariant::Never);

        // Inferred type: _
        let inferred = skip_trivia()
            .ignore_then(just(Token::Underscore).map_with_span(|_, span| Span::from(span)))
            .map(TyVariant::Inferred);

        // Unit type or tuple/function type
        let paren_types = {
            skip_trivia()
                .ignore_then(just(Token::LParen).map_with_span(|_, span| Span::from(span)))
                .then(ty.clone().separated_by(just(Token::Comma)).allow_trailing())
                .then(just(Token::RParen).map_with_span(|_, span| Span::from(span)))
                .then(
                    // Optional arrow and return type for function types
                    skip_trivia()
                        .ignore_then(just(Token::Arrow))
                        .map_with_span(|_, span| Span::from(span))
                        .then(ty.clone())
                        .or_not(),
                )
                .map(|(((lparen, types), rparen), arrow_and_return)| {
                    if let Some((arrow_span, return_ty)) = arrow_and_return {
                        TyVariant::Function(lparen, types, rparen, arrow_span, Box::new(return_ty))
                    } else if types.is_empty() {
                        TyVariant::Unit(lparen, rparen)
                    } else {
                        TyVariant::Tuple(lparen, types, rparen)
                    }
                })
        };

        // Path type with optional type arguments: Foo or Foo[Int, String]
        let path = path_segments_parser()
            .then(
                // Optional type arguments: [T1, T2]
                skip_trivia()
                    .ignore_then(just(Token::LBracket))
                    .ignore_then(ty.clone().separated_by(just(Token::Comma)).allow_trailing())
                    .then_ignore(skip_trivia())
                    .then_ignore(just(Token::RBracket))
                    .or_not(),
            )
            .map(|(segments, args)| TyVariant::Path { segments, args });

        // Array type: [T]
        let array = skip_trivia()
            .ignore_then(just(Token::LBracket).map_with_span(|_, span| Span::from(span)))
            .then(ty.clone())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::RBracket).map_with_span(|_, span| Span::from(span))),
            )
            .map(|((lbracket, element_ty), rbracket)| {
                TyVariant::Array(lbracket, Box::new(element_ty), rbracket)
            });

        // Try never first, then inferred, then paren types, then array, then path
        never.or(inferred).or(paren_types).or(array).or(path)
    })
}

/// Parse a type expression and emit events
/// This is the primary event-driven parser function
pub fn parse_ty<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let end_pos = source.len();
    let tokens_with_range = tokens.map(|(tok, span)| (tok, span.range()));
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens_with_range);

    match ty_parser().parse(stream) {
        Ok(variant) => {
            emit_ty_variant(sink, &variant);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), Span::from(span));
            }
        }
    }
}

/// Emit events for any type variant
pub(crate) fn emit_ty_variant(sink: &mut EventSink, variant: &TyVariant) {
    match variant {
        TyVariant::Unit(lparen_span, rparen_span) => {
            emit_unit_type(sink, lparen_span.clone(), rparen_span.clone());
        }
        TyVariant::Never(bang_span) => {
            emit_never_type(sink, bang_span.clone());
        }
        TyVariant::Inferred(underscore_span) => {
            emit_inferred_type(sink, underscore_span.clone());
        }
        TyVariant::Tuple(lparen, types, rparen) => {
            emit_tuple_type(sink, lparen.clone(), types, rparen.clone());
        }
        TyVariant::Function(lparen, params, rparen, arrow, return_ty) => {
            emit_function_type(
                sink,
                lparen.clone(),
                params,
                rparen.clone(),
                arrow.clone(),
                return_ty,
            );
        }
        TyVariant::Path { segments, args } => {
            emit_path_type(sink, segments, args.as_ref());
        }
        TyVariant::Array(lbracket, element_ty, rbracket) => {
            emit_array_type(sink, lbracket.clone(), element_ty, rbracket.clone());
        }
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
            sink.add_token(SyntaxKind::Dot, Span::from(span.start - 1..span.start));
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

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    fn parse_ty_from_source(source: &str) -> TyExpression {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let mut sink = EventSink::new();
        parse_ty(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        TyExpression {
            syntax: tree,
            span: Span::from(0..source.len()),
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
